//! Audio types and signal processing. Capturing from a microphone is the app's
//! job (cpal on desktop, AudioRecord/AVAudioEngine on phones); the result goes
//! through [`dsp::prepare`] to become a [`Captured`].

pub mod dsp;
pub mod push;

pub use push::PushRecorder;

/// Whisper's input rate.
pub const TARGET_RATE: u32 = 16_000;

/// A finished recording, ready for speech-to-text.
#[derive(Debug, Clone, Default)]
pub struct Captured {
    /// 16 kHz mono, cleaned up and trimmed to the speech.
    pub samples: Vec<f32>,
    /// Length of the raw recording.
    pub duration_ms: u64,
    pub has_speech: bool,
}
