//! Desktop microphone capture (cpal). The recorder itself, the audio types and
//! the DSP are in hams-core; this module only feeds them from the sound card.

mod capture;

pub use capture::{list_microphones, new_recorder};
