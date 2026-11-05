use std::path::PathBuf;

/// `~/Library/Application Support/Hams` on macOS, `%APPDATA%\Hams` on Windows.
pub fn data_dir() -> PathBuf {
    let dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Hams");
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

/// Data folder used by Hams v1 (named "wisper" at the time). Read-only: never deleted.
pub fn v1_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("wisper")
}
