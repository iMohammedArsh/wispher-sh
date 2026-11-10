use serde::{Serialize, Serializer};

/// Every failure names the stage it happened in, so the UI can say what went wrong
/// ("Paste failed: …") instead of a generic error.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Microphone: {0}")]
    Audio(String),
    #[error("Transcription: {0}")]
    Transcription(String),
    #[error("AI polish: {0}")]
    Polish(String),
    #[error("Paste: {0}")]
    Delivery(String),
    #[error("Storage: {0}")]
    Storage(String),
    #[error("Settings: {0}")]
    Settings(String),
    #[error("Model: {0}")]
    Model(String),
    #[error("Permission: {0}")]
    Permission(String),
    #[error("{0}")]
    Other(String),
}

impl From<rusqlite::Error> for AppError {
    fn from(error: rusqlite::Error) -> Self {
        AppError::Storage(error.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        AppError::Storage(error.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        AppError::Other(error.to_string())
    }
}

/// Commands return errors to the webview as plain strings.
impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
