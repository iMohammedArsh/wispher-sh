//! The Hams dictation engine, shared by every app:
//! record → transcribe → clean up → polish → deliver → persist.
//!
//! Nothing in here knows about Tauri, a window system or a keyboard hook. Each
//! app supplies the platform pieces through the traits in [`controller`]
//! (`Recorder`, `Ui`), [`desktop::Desktop`], [`stt::Transcriber`],
//! [`llm::Polisher`] and [`settings::SecretStore`].

pub mod audio;
pub mod controller;
pub mod desktop;
pub mod error;
pub mod input;
pub mod llm;
pub mod paths;
pub mod settings;
pub mod store;
pub mod stt;
pub mod text;
