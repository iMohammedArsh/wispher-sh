//! A [`Recorder`] for platforms that capture audio themselves and hand it over
//! (Android `AudioRecord`, iOS `AVAudioEngine`). The engine tells the platform
//! when to open and close the microphone through [`MicControl`]; the platform
//! pushes mono frames with [`PushRecorder::push`] in between.
//!
//! As on desktop, the microphone is open only while a recording runs.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;

use super::{dsp, Captured};
use crate::controller::{Recorder, PREVIEW_WINDOW_SECONDS};
use crate::error::AppResult;

/// Extra audio kept after release so the last word isn't clipped.
const TAIL: Duration = Duration::from_millis(150);
/// Hard cap so a forgotten hands-free session can't eat all memory (10 minutes).
const MAX_SECONDS: usize = 600;

/// Opens and closes the platform microphone.
pub trait MicControl: Send + Sync {
    /// Start capturing and pushing frames. Errors (no permission, busy mic) are
    /// shown to the user.
    fn start(&self) -> AppResult<()>;
    /// Stop capturing. Frames pushed after this are ignored.
    fn stop(&self);
}

pub struct PushRecorder {
    mic: Arc<dyn MicControl>,
    samples: Mutex<Vec<f32>>,
    rate: AtomicU32,
    level: AtomicU32,
    capturing: AtomicBool,
    tail: Duration,
}

impl PushRecorder {
    pub fn new(mic: Arc<dyn MicControl>) -> Self {
        Self {
            mic,
            samples: Mutex::new(Vec::new()),
            rate: AtomicU32::new(super::TARGET_RATE),
            level: AtomicU32::new(0),
            capturing: AtomicBool::new(false),
            tail: TAIL,
        }
    }

    /// Mono frames in -1..1 at `rate` Hz, from the platform's capture thread.
    pub fn push(&self, frames: &[f32], rate: u32) {
        if frames.is_empty() || rate == 0 || !self.capturing.load(Ordering::Acquire) {
            return;
        }
        let mut samples = self.samples.lock();
        let previous_rate = self.rate.swap(rate, Ordering::Relaxed);
        if previous_rate != rate && !samples.is_empty() {
            // The platform changed rate mid-recording (rare: a headset plugged in).
            // Resample what we have so the buffer stays at one rate.
            let converted = dsp::resample(&samples, previous_rate, rate);
            *samples = converted;
        }
        if samples.len() >= rate as usize * MAX_SECONDS {
            return;
        }
        let start = samples.len();
        samples.extend_from_slice(frames);
        let level = dsp::meter_level(&samples[start..]);
        drop(samples);
        // Fast attack, slower release, so the meter looks alive but not jittery.
        let previous = f32::from_bits(self.level.load(Ordering::Relaxed));
        let smoothed = if level > previous { level } else { previous * 0.8 + level * 0.2 };
        self.level.store(smoothed.to_bits(), Ordering::Relaxed);
    }

    /// 16-bit PCM convenience for platforms that capture `short`s.
    pub fn push_i16(&self, frames: &[i16], rate: u32) {
        let converted: Vec<f32> = frames.iter().map(|&s| s as f32 / 32_768.0).collect();
        self.push(&converted, rate);
    }

    pub fn is_capturing(&self) -> bool {
        self.capturing.load(Ordering::Acquire)
    }

    fn stop_raw(&self, keep_tail: bool) -> Vec<f32> {
        if keep_tail && self.capturing.load(Ordering::Acquire) {
            std::thread::sleep(self.tail);
        }
        self.capturing.store(false, Ordering::Release);
        self.mic.stop();
        self.level.store(0, Ordering::Relaxed);
        std::mem::take(&mut *self.samples.lock())
    }
}

impl Recorder for PushRecorder {
    fn start(&self, _device: Option<&str>) -> AppResult<()> {
        self.samples.lock().clear();
        self.capturing.store(true, Ordering::Release);
        if let Err(error) = self.mic.start() {
            self.capturing.store(false, Ordering::Release);
            return Err(error);
        }
        Ok(())
    }

    fn stop(&self) -> AppResult<Captured> {
        let raw = self.stop_raw(true);
        Ok(dsp::prepare(&raw, self.rate.load(Ordering::Relaxed)))
    }

    fn cancel(&self) {
        let _ = self.stop_raw(false);
    }

    fn level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }

    fn snapshot(&self) -> Option<Captured> {
        let rate = self.rate.load(Ordering::Relaxed);
        let tail = {
            let samples = self.samples.lock();
            let keep = (rate as usize * PREVIEW_WINDOW_SECONDS as usize).min(samples.len());
            samples[samples.len() - keep..].to_vec()
        };
        Some(dsp::prepare(&tail, rate))
    }

    fn drain(&self, min_seconds: u32) -> Option<Captured> {
        let rate = self.rate.load(Ordering::Relaxed);
        let chunk = dsp::take_chunk(&mut self.samples.lock(), rate, min_seconds)?;
        Some(dsp::prepare(&chunk, rate))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[derive(Default)]
    struct FakeMic {
        open: AtomicUsize,
        fail: AtomicBool,
    }
    impl MicControl for FakeMic {
        fn start(&self) -> AppResult<()> {
            if self.fail.load(Ordering::SeqCst) {
                return Err(crate::error::AppError::Audio("no permission".into()));
            }
            self.open.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        fn stop(&self) {
            let _ = self.open.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1));
        }
    }

    fn recorder() -> (Arc<FakeMic>, PushRecorder) {
        let mic = Arc::new(FakeMic::default());
        let mut recorder = PushRecorder::new(mic.clone());
        recorder.tail = Duration::ZERO;
        (mic, recorder)
    }

    fn tone(seconds: f32, rate: u32) -> Vec<f32> {
        (0..(seconds * rate as f32) as usize).map(|i| (i as f32 * 0.05).sin() * 0.3).collect()
    }

    #[test]
    fn frames_are_kept_only_while_recording() {
        let (mic, recorder) = recorder();
        recorder.push(&tone(0.5, 16_000), 16_000);
        assert!(recorder.samples.lock().is_empty());
        recorder.start(None).unwrap();
        assert_eq!(mic.open.load(Ordering::SeqCst), 1);
        recorder.push(&tone(2.0, 16_000), 16_000);
        assert!(recorder.level() > 0.0);
        let captured = recorder.stop().unwrap();
        assert_eq!(mic.open.load(Ordering::SeqCst), 0);
        assert_eq!(captured.duration_ms, 2_000);
        assert!(captured.has_speech);
        assert_eq!(recorder.level(), 0.0);
    }

    #[test]
    fn a_failed_start_leaves_nothing_capturing() {
        let (mic, recorder) = recorder();
        mic.fail.store(true, Ordering::SeqCst);
        assert!(recorder.start(None).is_err());
        assert!(!recorder.is_capturing());
        recorder.push(&tone(1.0, 16_000), 16_000);
        assert!(recorder.samples.lock().is_empty());
    }

    #[test]
    fn cancel_throws_the_audio_away() {
        let (mic, recorder) = recorder();
        recorder.start(None).unwrap();
        recorder.push_i16(&[1000; 16_000], 16_000);
        recorder.cancel();
        assert_eq!(mic.open.load(Ordering::SeqCst), 0);
        assert!(recorder.samples.lock().is_empty());
    }

    #[test]
    fn a_rate_change_mid_recording_keeps_one_rate() {
        let (_mic, recorder) = recorder();
        recorder.start(None).unwrap();
        recorder.push(&tone(1.0, 48_000), 48_000);
        recorder.push(&tone(1.0, 16_000), 16_000);
        let captured = recorder.stop().unwrap();
        assert!((1_900..=2_100).contains(&captured.duration_ms), "{}", captured.duration_ms);
    }

    #[test]
    fn drain_hands_over_chunks_while_recording() {
        let (_mic, recorder) = recorder();
        recorder.start(None).unwrap();
        recorder.push(&tone(5.0, 16_000), 16_000);
        assert!(recorder.drain(10).is_none());
        recorder.push(&tone(6.0, 16_000), 16_000);
        assert!(recorder.drain(10).is_some());
        assert!(recorder.is_capturing());
    }
}
