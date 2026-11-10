//! Desktop microphone capture (cpal). The audio types and DSP are in hams-core.

mod capture;

pub use capture::{list_microphones, AudioRecorder};
pub use hams_core::audio::{dsp, Captured};

use hams_core::controller::{Recorder, PREVIEW_WINDOW_SECONDS};
use hams_core::error::AppResult;

impl Recorder for AudioRecorder {
    fn start(&self, device: Option<&str>) -> AppResult<()> {
        AudioRecorder::start(self, device)
    }
    fn stop(&self) -> AppResult<Captured> {
        AudioRecorder::stop(self)
    }
    fn cancel(&self) {
        AudioRecorder::cancel(self)
    }
    fn level(&self) -> f32 {
        AudioRecorder::level(self)
    }
    fn snapshot(&self) -> Option<Captured> {
        Some(AudioRecorder::snapshot(self, PREVIEW_WINDOW_SECONDS))
    }
    fn drain(&self, min_seconds: u32) -> Option<Captured> {
        AudioRecorder::drain(self, min_seconds)
    }
}
