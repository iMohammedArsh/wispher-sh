//! Microphone capture for the desktop (cpal). Everything after the callback
//! (buffering, the level meter, the tail after release, the length cap,
//! previews and note chunks) is `hams_core::audio::PushRecorder`, the same code
//! the phones use. This file only opens and closes the stream and feeds it mono
//! frames, so an audio change in hams-core reaches the Mac, Windows and Android
//! together.
//!
//! cpal streams are not `Send` on every platform, so one dedicated audio thread
//! owns the stream and is driven over a channel. The real-time callback only
//! downmixes into a reused buffer and hands it over; it never allocates on the
//! hot path after warm-up.
//!
//! The microphone is open only while a recording runs: the stream is created
//! on `start` and dropped on `stop`/`cancel`, so macOS shows its mic indicator
//! only during a dictation. A `start` that times out (for example while macOS
//! is still asking for microphone permission) queues a stop behind itself, so
//! a stream that opens late is closed again instead of staying on.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, OnceLock, Weak};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hams_core::audio::push::MicControl;
use hams_core::audio::PushRecorder;
use parking_lot::Mutex;

use crate::error::{AppError, AppResult};

/// How long `start` waits for the microphone to open.
const START_TIMEOUT: Duration = Duration::from_secs(3);
/// How long `stop` waits for the stream to close.
const STOP_TIMEOUT: Duration = Duration::from_secs(2);

/// Where the stream callback sends its frames. It is filled in right after the
/// recorder is built (the recorder owns the mic and the mic needs the recorder).
type Sink = Arc<OnceLock<Weak<PushRecorder>>>;

/// The desktop recorder: hams-core's [`PushRecorder`] fed by cpal.
pub fn new_recorder() -> Arc<PushRecorder> {
    let sink = Sink::default();
    let recorder = Arc::new(PushRecorder::new(Arc::new(CpalMic::new(Arc::clone(&sink)))));
    let _ = sink.set(Arc::downgrade(&recorder));
    recorder
}

pub fn list_microphones() -> Vec<String> {
    cpal::default_host()
        .input_devices()
        .map(|devices| devices.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

enum Request {
    Start { device: Option<String>, reply: Sender<AppResult<()>> },
    Stop { reply: Sender<()> },
}

/// Opens and closes the cpal input stream on its own thread.
pub struct CpalMic {
    requests: Mutex<Sender<Request>>,
    start_timeout: Duration,
}

impl CpalMic {
    fn new(sink: Sink) -> Self {
        Self::spawn(open_stream, sink, START_TIMEOUT)
    }

    /// Run the audio thread with `open` creating the input stream; whatever it
    /// returns holds the microphone open until it's dropped.
    fn spawn<S: 'static>(open: fn(Option<&str>, &Sink) -> AppResult<S>, sink: Sink, start_timeout: Duration) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("hams-audio".into())
            .spawn(move || audio_thread(rx, sink, open))
            .expect("spawn audio thread");
        Self { requests: Mutex::new(tx), start_timeout }
    }
}

impl MicControl for CpalMic {
    fn start(&self, device: Option<&str>) -> AppResult<()> {
        let (reply, response) = mpsc::channel();
        self.requests
            .lock()
            .send(Request::Start { device: device.map(str::to_string), reply })
            .map_err(|_| AppError::Audio("audio thread stopped".into()))?;
        match response.recv_timeout(self.start_timeout) {
            Ok(result) => result,
            Err(_) => {
                // The audio thread may still open the stream after we give up.
                // Requests run in order, so this stop closes it right after.
                let (reply, _) = mpsc::channel();
                let _ = self.requests.lock().send(Request::Stop { reply });
                Err(AppError::Audio("the microphone took too long to start".into()))
            }
        }
    }

    fn stop(&self) {
        let (reply, response) = mpsc::channel();
        if self.requests.lock().send(Request::Stop { reply }).is_ok() {
            let _ = response.recv_timeout(STOP_TIMEOUT);
        }
    }
}

fn audio_thread<S: 'static>(requests: Receiver<Request>, sink: Sink, open: fn(Option<&str>, &Sink) -> AppResult<S>) {
    // The microphone is open exactly while this holds a stream.
    let mut stream: Option<S> = None;
    while let Ok(request) = requests.recv() {
        match request {
            Request::Start { device, reply } => {
                // Close any earlier stream before opening the next one.
                drop(stream.take());
                let result = open(device.as_deref(), &sink).map(|opened| {
                    stream = Some(opened);
                });
                let _ = reply.send(result);
            }
            Request::Stop { reply } => {
                drop(stream.take());
                let _ = reply.send(());
            }
        }
    }
}

fn open_stream(device_name: Option<&str>, sink: &Sink) -> AppResult<cpal::Stream> {
    let host = cpal::default_host();
    let device = match device_name {
        Some(name) => host
            .input_devices()
            .ok()
            .and_then(|mut devices| devices.find(|d| d.name().map(|n| n == name).unwrap_or(false)))
            .or_else(|| {
                tracing::warn!("Microphone \"{name}\" not found; using the default input");
                host.default_input_device()
            }),
        None => host.default_input_device(),
    }
    .ok_or_else(|| AppError::Audio("no microphone found".into()))?;

    let supported = device
        .default_input_config()
        .map_err(|e| AppError::Audio(format!("could not read microphone format: {e}")))?;
    let channels = supported.channels() as usize;
    let rate = supported.sample_rate().0;
    let config: cpal::StreamConfig = supported.config();
    tracing::debug!(
        "Opening microphone {:?} at {rate} Hz, {channels} ch, {:?}",
        device.name().unwrap_or_default(),
        supported.sample_format()
    );

    macro_rules! build {
        ($t:ty, $convert:expr) => {{
            let sink = Arc::clone(sink);
            // Reused by every callback, so the audio thread doesn't allocate.
            let mut mono: Vec<f32> = Vec::with_capacity(rate as usize / 5);
            device.build_input_stream(
                &config,
                move |data: &[$t], _: &cpal::InputCallbackInfo| {
                    push_frames(&sink, &mut mono, data, channels, rate, $convert)
                },
                move |error| tracing::error!("Microphone stream error: {error}"),
                None,
            )
        }};
    }
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => build!(f32, |s: f32| s),
        cpal::SampleFormat::I16 => build!(i16, |s: i16| s as f32 / 32_768.0),
        cpal::SampleFormat::I32 => build!(i32, |s: i32| s as f32 / 2_147_483_648.0),
        cpal::SampleFormat::U16 => build!(u16, |s: u16| (s as f32 - 32_768.0) / 32_768.0),
        other => return Err(AppError::Audio(format!("unsupported microphone format {other:?}"))),
    }
    .map_err(|e| AppError::Audio(format!("could not open the microphone: {e}")))?;
    stream
        .play()
        .map_err(|e| AppError::Audio(format!("could not start the microphone: {e}")))?;
    Ok(stream)
}

/// One callback's interleaved frames, averaged to mono and handed to the recorder.
fn push_frames<T: Copy>(
    sink: &Sink,
    mono: &mut Vec<f32>,
    data: &[T],
    channels: usize,
    rate: u32,
    convert: impl Fn(T) -> f32,
) {
    if channels == 0 {
        return;
    }
    let Some(recorder) = sink.get().and_then(Weak::upgrade) else { return };
    mono.clear();
    mono.extend(data.chunks_exact(channels).map(|frame| frame.iter().map(|&s| convert(s)).sum::<f32>() / channels as f32));
    recorder.push(mono, rate);
}

#[cfg(test)]
mod tests {
    use super::*;
    use hams_core::controller::Recorder;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Stands in for a cpal stream: counts how many are open.
    static OPEN: AtomicUsize = AtomicUsize::new(0);
    /// How many have ever been opened.
    static OPENED: AtomicUsize = AtomicUsize::new(0);
    struct FakeStream;
    impl Drop for FakeStream {
        fn drop(&mut self) {
            OPEN.fetch_sub(1, Ordering::SeqCst);
        }
    }
    fn open_fast(_: Option<&str>, _: &Sink) -> AppResult<FakeStream> {
        OPEN.fetch_add(1, Ordering::SeqCst);
        OPENED.fetch_add(1, Ordering::SeqCst);
        Ok(FakeStream)
    }
    /// Like macOS while its microphone permission prompt is up.
    fn open_slow(device: Option<&str>, sink: &Sink) -> AppResult<FakeStream> {
        std::thread::sleep(Duration::from_millis(150));
        open_fast(device, sink)
    }
    /// The tests share `OPEN`, so they take turns.
    static SERIAL: Mutex<()> = parking_lot::const_mutex(());

    /// A recorder wired like `new_recorder`, but with a fake stream.
    fn recorder_with(open: fn(Option<&str>, &Sink) -> AppResult<FakeStream>, start_timeout: Duration) -> (Sink, Arc<PushRecorder>) {
        let sink = Sink::default();
        let mic = Arc::new(CpalMic::spawn(open, Arc::clone(&sink), start_timeout));
        let recorder = Arc::new(PushRecorder::new(mic));
        let _ = sink.set(Arc::downgrade(&recorder));
        (sink, recorder)
    }

    #[test]
    fn the_mic_is_open_only_while_recording() {
        let _turn = SERIAL.lock();
        let (_sink, recorder) = recorder_with(open_fast, START_TIMEOUT);
        assert_eq!(OPEN.load(Ordering::SeqCst), 0);
        recorder.start(None).unwrap();
        assert_eq!(OPEN.load(Ordering::SeqCst), 1);
        recorder.stop().unwrap();
        assert_eq!(OPEN.load(Ordering::SeqCst), 0);
        recorder.start(None).unwrap();
        recorder.cancel();
        assert_eq!(OPEN.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_start_that_times_out_does_not_leave_the_mic_on() {
        let _turn = SERIAL.lock();
        let (_sink, recorder) = recorder_with(open_slow, Duration::from_millis(20));
        let opened = OPENED.load(Ordering::SeqCst);
        assert!(recorder.start(None).is_err());
        // Wait for the stream to open late, then for the queued stop.
        let closed_after_opening = (0..200).any(|_| {
            std::thread::sleep(Duration::from_millis(10));
            OPENED.load(Ordering::SeqCst) > opened && OPEN.load(Ordering::SeqCst) == 0
        });
        assert!(closed_after_opening, "the late stream was left open");
        assert!(!recorder.is_capturing());
    }

    #[test]
    fn stereo_frames_are_averaged_to_mono_before_the_recorder_sees_them() {
        let _turn = SERIAL.lock();
        let (sink, recorder) = recorder_with(open_fast, START_TIMEOUT);
        recorder.start(None).unwrap();
        // One second of stereo at 16 kHz is 32 000 values but 16 000 frames.
        let stereo: Vec<f32> = (0..32_000).map(|i| ((i / 2) as f32 * 0.05).sin() * 0.3).collect();
        let mut mono = Vec::new();
        push_frames(&sink, &mut mono, &stereo, 2, 16_000, |s| s);
        assert_eq!(mono.len(), 16_000);
        assert!(recorder.level() > 0.0);
        let captured = recorder.stop().unwrap();
        assert_eq!(captured.duration_ms, 1_000);
    }

    /// The real microphone through the real recorder, for one second. Run it by
    /// hand (it opens the mic, so macOS may ask the terminal for permission):
    ///   cargo test -p hams real_mic -- --ignored --nocapture
    #[test]
    #[ignore]
    fn real_mic_records_for_one_second() {
        let recorder = new_recorder();
        recorder.start(None).expect("open the default microphone");
        std::thread::sleep(Duration::from_secs(1));
        let captured = recorder.stop().unwrap();
        println!("{} ms recorded, speech: {}, quiet: {}", captured.duration_ms, captured.has_speech, captured.quiet);
        // One second, plus the 150 ms tail kept after stop, plus start-up slack.
        assert!((900..1_700).contains(&captured.duration_ms), "recorded {} ms", captured.duration_ms);
    }

    #[test]
    fn frames_are_dropped_once_the_recorder_is_gone_or_not_recording() {
        let _turn = SERIAL.lock();
        let (sink, recorder) = recorder_with(open_fast, START_TIMEOUT);
        let mut mono = Vec::new();
        // Not recording yet: the recorder ignores them.
        push_frames(&sink, &mut mono, &[0.5_f32; 1_000], 1, 16_000, |s| s);
        assert_eq!(recorder.level(), 0.0);
        // No channels is a broken device, not a reason to panic.
        push_frames(&sink, &mut mono, &[0.5_f32; 10], 0, 16_000, |s| s);
        // The recorder is gone (the app is shutting down): the callback just returns.
        drop(recorder);
        push_frames(&sink, &mut mono, &[0.5_f32; 1_000], 1, 16_000, |s| s);
    }
}
