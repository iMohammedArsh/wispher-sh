//! Microphone capture. cpal streams are not `Send` on every platform, so one
//! dedicated audio thread owns the stream and is driven over a channel. The
//! real-time callback only downmixes into a pre-allocated buffer and updates the
//! level meter; it never allocates on the hot path after warm-up.
//!
//! With "keep the mic warm" on, the stream stays open between dictations and
//! the callback keeps only the last `PREROLL` of audio in a small ring. A new
//! recording starts instantly and begins with that audio, so a first word
//! spoken while pressing the key isn't lost. Nothing in the ring is kept longer
//! than `PREROLL` or ever leaves memory.

use std::collections::VecDeque;
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
/// Warm mic: how much audio from just before the key press starts a recording.
const PREROLL: Duration = Duration::from_millis(700);

struct Shared {
    samples: Mutex<Vec<f32>>,
    /// Warm mic only: the last `PREROLL` of audio while not recording.
    preroll: Mutex<VecDeque<f32>>,
    rate: AtomicU32,
    level: AtomicU32,
    capturing: AtomicBool,
    warm: AtomicBool,
    /// The stream reported an error (device unplugged, etc.): reopen it next time.
    failed: AtomicBool,
}

impl Shared {
    fn new() -> Self {
        Self {
            samples: Mutex::new(Vec::with_capacity(48_000 * 30)),
            preroll: Mutex::new(VecDeque::with_capacity(48_000)),
            rate: AtomicU32::new(48_000),
            level: AtomicU32::new(0),
            capturing: AtomicBool::new(false),
            warm: AtomicBool::new(false),
            failed: AtomicBool::new(false),
        }
    }
}

enum Request {
    Start { device: Option<String>, reply: Sender<AppResult<()>> },
    Stop { reply: Sender<()> },
    Warm { device: Option<String>, on: bool },
}

pub struct AudioRecorder {
    requests: Mutex<Sender<Request>>,
    shared: Arc<Shared>,
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioRecorder {
    pub fn new() -> Self {
        let shared = Arc::new(Shared::new());
        let (tx, rx) = mpsc::channel();
        let thread_shared = Arc::clone(&shared);
        std::thread::Builder::new()
            .name("hams-audio".into())
            .spawn(move || audio_thread(rx, thread_shared))
            .expect("spawn audio thread");
        Self { requests: Mutex::new(tx), shared }
    }

    pub fn start(&self, device: Option<&str>) -> AppResult<()> {
        let (reply, response) = mpsc::channel();
        self.requests
            .lock()
            .send(Request::Start { device: device.map(str::to_string), reply })
            .map_err(|_| AppError::Audio("audio thread stopped".into()))?;
        response
            .recv_timeout(Duration::from_secs(3))
            .map_err(|_| AppError::Audio("the microphone took too long to start".into()))?
    }

    /// Keep the microphone open between dictations (or close it again).
    pub fn set_warm(&self, device: Option<&str>, on: bool) {
        let _ = self.requests.lock().send(Request::Warm { device: device.map(str::to_string), on });
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

/// An open input stream and what it was opened for.
struct Open {
    /// Kept only to hold the stream open.
    _stream: cpal::Stream,
    /// The device the user asked for (None = system default).
    wanted: Option<String>,
    /// The device actually opened, to notice when the system default changes.
    name: String,
}

impl Open {
    /// Still the right stream for a recording on `wanted`?
    fn reusable(&self, wanted: &Option<String>, shared: &Shared) -> bool {
        if shared.failed.load(Ordering::Acquire) || &self.wanted != wanted {
            return false;
        }
        match wanted {
            Some(_) => true,
            None => default_input_name().is_some_and(|name| name == self.name),
        }
    }
}

fn audio_thread(requests: Receiver<Request>, shared: Arc<Shared>) {
    let mut open: Option<Open> = None;
    let reopen = |open: &mut Option<Open>, wanted: Option<String>| -> AppResult<()> {
        *open = None;
        shared.preroll.lock().clear();
        shared.failed.store(false, Ordering::Release);
        let (stream, name) = open_stream(wanted.as_deref(), &shared)?;
        *open = Some(Open { _stream: stream, wanted, name });
        Ok(())
    };
    while let Ok(request) = requests.recv() {
        match request {
            Request::Start { device, reply } => {
                let warm = shared.warm.load(Ordering::Acquire);
                let result = if warm && open.as_ref().is_some_and(|o| o.reusable(&device, &shared)) {
                    Ok(())
                } else {
                    reopen(&mut open, device)
                };
                if result.is_ok() {
                    let mut samples = shared.samples.lock();
                    samples.clear();
                    samples.extend(shared.preroll.lock().drain(..));
                    shared.capturing.store(true, Ordering::Release);
                }
                let _ = reply.send(result);
            }
            Request::Stop { reply } => {
                if open.is_some() {
                    std::thread::sleep(TAIL);
                }
                shared.capturing.store(false, Ordering::Release);
                if !shared.warm.load(Ordering::Acquire) {
                    open = None;
                }
                let _ = reply.send(());
            }
            Request::Warm { device, on } => {
                shared.warm.store(on, Ordering::Release);
                let capturing = shared.capturing.load(Ordering::Acquire);
                if capturing {
                    // Applied when this recording stops (or at the next start).
                } else if !on {
                    open = None;
                    shared.preroll.lock().clear();
                } else if !open.as_ref().is_some_and(|o| o.reusable(&device, &shared)) {
                    if let Err(error) = reopen(&mut open, device) {
                        tracing::warn!("Couldn't keep the microphone warm: {error}");
                    }
                }
            }
        }
    }
}

fn default_input_name() -> Option<String> {
    cpal::default_host().default_input_device().and_then(|d| d.name().ok())
}

fn open_stream(device_name: Option<&str>, shared: &Arc<Shared>) -> AppResult<(cpal::Stream, String)> {
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
    let name = device.name().unwrap_or_default();
    tracing::debug!(
        "Opening microphone {:?} at {rate} Hz, {channels} ch, {:?}",
        device.name().unwrap_or_default(),
        supported.sample_format()
    );

    let preroll_samples = (rate as f32 * PREROLL.as_secs_f32()) as usize;
    macro_rules! build {
        ($t:ty, $convert:expr) => {{
            let shared = Arc::clone(shared);
            let errors = Arc::clone(&shared);
            device.build_input_stream(
                &config,
                move |data: &[$t], _: &cpal::InputCallbackInfo| {
                    push_frames(&shared, data, channels, max_samples, preroll_samples, $convert)
                },
                move |error| {
                    tracing::error!("Microphone stream error: {error}");
                    errors.failed.store(true, Ordering::Release);
                },
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
    Ok((stream, name))
}

fn push_frames<T: Copy>(
    shared: &Shared,
    data: &[T],
    channels: usize,
    max: usize,
    preroll: usize,
    convert: impl Fn(T) -> f32,
) {
    if channels == 0 {
        return;
    }
    let mono = |frame: &[T]| frame.iter().map(|&s| convert(s)).sum::<f32>() / channels as f32;
    if !shared.capturing.load(Ordering::Acquire) {
        if shared.warm.load(Ordering::Acquire) {
            let mut ring = shared.preroll.lock();
            ring.extend(data.chunks_exact(channels).map(mono));
            let excess = ring.len().saturating_sub(preroll);
            ring.drain(..excess);
        }
        return;
    }
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

    fn push(shared: &Shared, frames: std::ops::Range<i32>) {
        // Stereo frames whose channels average to the frame number.
        let data: Vec<f32> = frames.flat_map(|i| [i as f32 - 1.0, i as f32 + 1.0]).collect();
        push_frames(shared, &data, 2, 1_000, 5, |s| s);
    }

    #[test]
    fn a_cold_mic_records_nothing_between_dictations() {
        let shared = Shared::new();
        push(&shared, 0..20);
        assert!(shared.samples.lock().is_empty());
        assert!(shared.preroll.lock().is_empty());
    }

    #[test]
    fn a_warm_mic_keeps_only_the_last_moment_and_recordings_begin_with_it() {
        let shared = Shared::new();
        shared.warm.store(true, Ordering::Release);
        push(&shared, 0..20);
        assert_eq!(shared.preroll.lock().iter().copied().collect::<Vec<_>>(), vec![15.0, 16.0, 17.0, 18.0, 19.0]);
        assert!(shared.samples.lock().is_empty());

        // What the audio thread does on Start.
        shared.samples.lock().extend(shared.preroll.lock().drain(..));
        shared.capturing.store(true, Ordering::Release);
        push(&shared, 20..23);
        assert_eq!(*shared.samples.lock(), vec![15.0, 16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0]);
        assert!(shared.preroll.lock().is_empty());
    }
}
