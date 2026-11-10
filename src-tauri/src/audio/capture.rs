//! Microphone capture. cpal streams are not `Send` on every platform, so one
//! dedicated audio thread owns the stream and is driven over a channel. The
//! real-time callback only downmixes into a pre-allocated buffer and updates the
//! level meter; it never allocates on the hot path after warm-up.
//!
//! The microphone is open only while a recording runs: the stream is created
//! on `start` and dropped on `stop`/`cancel`, so macOS shows its mic indicator
//! only during a dictation. A `start` that times out (for example while macOS
//! is still asking for microphone permission) queues a stop behind itself, so
//! a stream that opens late is closed again instead of staying on.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;

use super::{dsp, Captured};
use crate::error::{AppError, AppResult};

/// Extra audio kept after the key is released so the last word isn't clipped.
const TAIL: Duration = Duration::from_millis(150);
/// Hard cap so a forgotten hands-free session can't eat all memory (10 minutes).
const MAX_SECONDS: usize = 600;
/// How long `start` waits for the microphone to open.
const START_TIMEOUT: Duration = Duration::from_secs(3);

struct Shared {
    samples: Mutex<Vec<f32>>,
    rate: AtomicU32,
    level: AtomicU32,
    capturing: AtomicBool,
}

impl Shared {
    fn new() -> Self {
        Self {
            samples: Mutex::new(Vec::with_capacity(48_000 * 30)),
            rate: AtomicU32::new(48_000),
            level: AtomicU32::new(0),
            capturing: AtomicBool::new(false),
        }
    }
}

enum Request {
    Start { device: Option<String>, reply: Sender<AppResult<()>> },
    Stop { reply: Sender<()> },
}

pub struct AudioRecorder {
    requests: Mutex<Sender<Request>>,
    shared: Arc<Shared>,
    start_timeout: Duration,
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self::spawn(open_stream, START_TIMEOUT)
    }

    /// Run the audio thread with `open` creating the input stream; whatever it
    /// returns holds the microphone open until it's dropped.
    fn spawn<S: 'static>(open: fn(Option<&str>, &Arc<Shared>) -> AppResult<S>, start_timeout: Duration) -> Self {
        let shared = Arc::new(Shared::new());
        let (tx, rx) = mpsc::channel();
        let thread_shared = Arc::clone(&shared);
        std::thread::Builder::new()
            .name("hams-audio".into())
            .spawn(move || audio_thread(rx, thread_shared, open))
            .expect("spawn audio thread");
        Self { requests: Mutex::new(tx), shared, start_timeout }
    }

    pub fn start(&self, device: Option<&str>) -> AppResult<()> {
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

    /// Stop and return Whisper-ready audio.
    pub fn stop(&self) -> AppResult<Captured> {
        let raw = self.stop_raw()?;
        let rate = self.shared.rate.load(Ordering::Relaxed);
        Ok(dsp::prepare(&raw, rate))
    }

    /// Whisper-ready copy of the last `seconds` of audio, without stopping.
    pub fn snapshot(&self, seconds: u32) -> Captured {
        let rate = self.shared.rate.load(Ordering::Relaxed);
        let tail = {
            let samples = self.shared.samples.lock();
            let keep = (rate as usize * seconds as usize).min(samples.len());
            samples[samples.len() - keep..].to_vec()
        };
        dsp::prepare(&tail, rate)
    }

    /// Notes mode: once at least `min_seconds` are buffered, take them (cut at a
    /// pause near the end) and keep recording into an emptied buffer, so a long
    /// note never holds more than one chunk of raw audio in memory.
    pub fn drain(&self, min_seconds: u32) -> Option<Captured> {
        let rate = self.shared.rate.load(Ordering::Relaxed);
        let chunk = dsp::take_chunk(&mut self.shared.samples.lock(), rate, min_seconds)?;
        Some(dsp::prepare(&chunk, rate))
    }

    /// Stop and throw the audio away.
    pub fn cancel(&self) {
        let _ = self.stop_raw();
    }

    fn stop_raw(&self) -> AppResult<Vec<f32>> {
        let (reply, response) = mpsc::channel();
        self.requests
            .lock()
            .send(Request::Stop { reply })
            .map_err(|_| AppError::Audio("audio thread stopped".into()))?;
        let _ = response.recv_timeout(Duration::from_secs(2));
        self.shared.level.store(0, Ordering::Relaxed);
        Ok(std::mem::take(&mut *self.shared.samples.lock()))
    }

    /// Current input level in 0..1 (perceptual scale), for the live meter.
    pub fn level(&self) -> f32 {
        f32::from_bits(self.shared.level.load(Ordering::Relaxed))
    }
}

pub fn list_microphones() -> Vec<String> {
    cpal::default_host()
        .input_devices()
        .map(|devices| devices.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

fn audio_thread<S: 'static>(requests: Receiver<Request>, shared: Arc<Shared>, open: fn(Option<&str>, &Arc<Shared>) -> AppResult<S>) {
    // The microphone is open exactly while this holds a stream.
    let mut stream: Option<S> = None;
    while let Ok(request) = requests.recv() {
        match request {
            Request::Start { device, reply } => {
                stream = None;
                shared.capturing.store(false, Ordering::Release);
                shared.samples.lock().clear();
                let result = open(device.as_deref(), &shared).map(|s| {
                    stream = Some(s);
                    shared.capturing.store(true, Ordering::Release);
                });
                let _ = reply.send(result);
            }
            Request::Stop { reply } => {
                if stream.is_some() {
                    std::thread::sleep(TAIL);
                }
                shared.capturing.store(false, Ordering::Release);
                stream = None;
                let _ = reply.send(());
            }
        }
    }
}

fn open_stream(device_name: Option<&str>, shared: &Arc<Shared>) -> AppResult<cpal::Stream> {
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
    shared.rate.store(rate, Ordering::Relaxed);
    let config: cpal::StreamConfig = supported.config();
    let max_samples = rate as usize * MAX_SECONDS;
    tracing::debug!(
        "Opening microphone {:?} at {rate} Hz, {channels} ch, {:?}",
        device.name().unwrap_or_default(),
        supported.sample_format()
    );

    macro_rules! build {
        ($t:ty, $convert:expr) => {{
            let shared = Arc::clone(shared);
            device.build_input_stream(
                &config,
                move |data: &[$t], _: &cpal::InputCallbackInfo| {
                    push_frames(&shared, data, channels, max_samples, $convert)
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

fn push_frames<T: Copy>(
    shared: &Shared,
    data: &[T],
    channels: usize,
    max: usize,
    convert: impl Fn(T) -> f32,
) {
    if channels == 0 || !shared.capturing.load(Ordering::Acquire) {
        return;
    }
    let mono = |frame: &[T]| frame.iter().map(|&s| convert(s)).sum::<f32>() / channels as f32;
    let mut samples = shared.samples.lock();
    let start = samples.len();
    if start >= max {
        return;
    }
    samples.extend(data.chunks_exact(channels).map(mono));
    let level = dsp::meter_level(&samples[start..]);
    drop(samples);
    // Fast attack, slower release, so the meter looks alive but not jittery.
    let previous = f32::from_bits(shared.level.load(Ordering::Relaxed));
    let smoothed = if level > previous { level } else { previous * 0.8 + level * 0.2 };
    shared.level.store(smoothed.to_bits(), Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

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
    fn open_fast(_: Option<&str>, _: &Arc<Shared>) -> AppResult<FakeStream> {
        OPEN.fetch_add(1, Ordering::SeqCst);
        OPENED.fetch_add(1, Ordering::SeqCst);
        Ok(FakeStream)
    }
    /// Like macOS while its microphone permission prompt is up.
    fn open_slow(device: Option<&str>, shared: &Arc<Shared>) -> AppResult<FakeStream> {
        std::thread::sleep(Duration::from_millis(150));
        open_fast(device, shared)
    }
    /// Both tests share `OPEN`, so they take turns.
    static SERIAL: Mutex<()> = parking_lot::const_mutex(());


    #[test]
    fn the_mic_is_open_only_while_recording() {
        let _turn = SERIAL.lock();
        let recorder = AudioRecorder::spawn(open_fast, START_TIMEOUT);
        assert_eq!(OPEN.load(Ordering::SeqCst), 0);
        recorder.start(None).unwrap();
        assert_eq!(OPEN.load(Ordering::SeqCst), 1);
        recorder.stop_raw().unwrap();
        assert_eq!(OPEN.load(Ordering::SeqCst), 0);
        recorder.start(None).unwrap();
        recorder.cancel();
        assert_eq!(OPEN.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_start_that_times_out_does_not_leave_the_mic_on() {
        let _turn = SERIAL.lock();
        let recorder = AudioRecorder::spawn(open_slow, Duration::from_millis(20));
        let opened = OPENED.load(Ordering::SeqCst);
        assert!(recorder.start(None).is_err());
        // Wait for the stream to open late, then for the queued stop.
        let closed_after_opening = (0..200).any(|_| {
            std::thread::sleep(Duration::from_millis(10));
            OPENED.load(Ordering::SeqCst) > opened && OPEN.load(Ordering::SeqCst) == 0
        });
        assert!(closed_after_opening, "the late stream was left open");
        assert!(!recorder.shared.capturing.load(Ordering::SeqCst));
    }

    #[test]
    fn nothing_is_kept_while_not_recording() {
        let shared = Shared::new();
        let data: Vec<f32> = (0..40).map(|i| i as f32).collect();
        push_frames(&shared, &data, 2, 1_000, |s| s);
        assert!(shared.samples.lock().is_empty());
    }
}
