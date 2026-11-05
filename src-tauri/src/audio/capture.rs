//! Microphone capture. cpal streams are not `Send` on every platform, so one
//! dedicated audio thread owns the stream and is driven over a channel. The
//! real-time callback only downmixes into a pre-allocated buffer and updates the
//! level meter; it never allocates on the hot path after warm-up.

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

struct Shared {
    samples: Mutex<Vec<f32>>,
    rate: AtomicU32,
    level: AtomicU32,
    capturing: AtomicBool,
}

enum Request {
    Start { device: Option<String>, reply: Sender<AppResult<()>> },
    Stop { reply: Sender<()> },
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
        let shared = Arc::new(Shared {
            samples: Mutex::new(Vec::with_capacity(48_000 * 30)),
            rate: AtomicU32::new(48_000),
            level: AtomicU32::new(0),
            capturing: AtomicBool::new(false),
        });
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

    /// Stop and return Whisper-ready audio.
    pub fn stop(&self) -> AppResult<Captured> {
        let raw = self.stop_raw()?;
        let rate = self.shared.rate.load(Ordering::Relaxed);
        Ok(dsp::prepare(&raw, rate))
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

fn audio_thread(requests: Receiver<Request>, shared: Arc<Shared>) {
    let mut stream: Option<cpal::Stream> = None;
    while let Ok(request) = requests.recv() {
        match request {
            Request::Start { device, reply } => {
                stream = None;
                shared.samples.lock().clear();
                let result = open_stream(device.as_deref(), &shared).map(|s| {
                    shared.capturing.store(true, Ordering::Release);
                    stream = Some(s);
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

    let on_error = |error| tracing::error!("Microphone stream error: {error}");
    macro_rules! build {
        ($t:ty, $convert:expr) => {{
            let shared = Arc::clone(shared);
            device.build_input_stream(
                &config,
                move |data: &[$t], _: &cpal::InputCallbackInfo| {
                    push_frames(&shared, data, channels, max_samples, $convert)
                },
                on_error,
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

fn push_frames<T: Copy>(shared: &Shared, data: &[T], channels: usize, max: usize, convert: impl Fn(T) -> f32) {
    if channels == 0 || !shared.capturing.load(Ordering::Acquire) {
        return;
    }
    let mut samples = shared.samples.lock();
    let start = samples.len();
    if start >= max {
        return;
    }
    samples.extend(
        data.chunks_exact(channels)
            .map(|frame| frame.iter().map(|&s| convert(s)).sum::<f32>() / channels as f32),
    );
    let level = dsp::meter_level(&samples[start..]);
    drop(samples);
    // Fast attack, slower release, so the meter looks alive but not jittery.
    let previous = f32::from_bits(shared.level.load(Ordering::Relaxed));
    let smoothed = if level > previous { level } else { previous * 0.8 + level * 0.2 };
    shared.level.store(smoothed.to_bits(), Ordering::Relaxed);
}
