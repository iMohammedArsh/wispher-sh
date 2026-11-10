//! Where Hams keeps its database, models and logs. Desktop apps use the OS data
//! folder; phones pass their sandbox folder to [`init`] before anything else.

use std::path::PathBuf;
use std::sync::OnceLock;

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Use `dir` as the data folder. Call once, before opening the store. Returns
/// false if a folder was already set.
pub fn init(dir: impl Into<PathBuf>) -> bool {
    DATA_DIR.set(dir.into()).is_ok()
}

/// `~/Library/Application Support/Hams` on macOS, `%APPDATA%\Hams` on Windows,
/// or whatever [`init`] was given.
pub fn data_dir() -> PathBuf {
    let dir = DATA_DIR
        .get()
        .cloned()
        .unwrap_or_else(|| dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("Hams"));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn database_path() -> PathBuf {
    data_dir().join("hams.sqlite3")
}

pub fn models_dir() -> PathBuf {
    let dir = data_dir().join("models");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn logs_dir() -> PathBuf {
    let dir = data_dir().join("logs");
    let _ = std::fs::create_dir_all(&dir);
    dir
}
