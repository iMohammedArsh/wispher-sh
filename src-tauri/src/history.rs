use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const MAX_HISTORY_ITEMS: usize = 100;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DictationRecord {
    pub id: String,
    pub text: String,
    pub created_at_ms: u128,
}

#[derive(Debug, Default)]
pub struct DictationHistory {
    records: Vec<DictationRecord>,
}

impl DictationHistory {
    fn file_path() -> PathBuf {
        let mut directory = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        directory.push("wisper");
        directory.push("dictation-history.json");
        directory
    }

    pub fn load() -> Self {
        let path = Self::file_path();
        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(records) => Self { records },
                Err(error) => {
                    eprintln!("[History] Ignoring malformed history at {path:?}: {error}");
                    Self::default()
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(error) => {
                eprintln!("[History] Could not read history at {path:?}: {error}");
                Self::default()
            }
        }
    }

    pub fn records(&self) -> Vec<DictationRecord> {
        self.records.clone()
    }

    pub fn latest_text(&self) -> Option<String> {
        self.records.first().map(|record| record.text.clone())
    }

    pub fn add(&mut self, text: &str) -> Result<(), String> {
        let text = text.trim();
        if text.is_empty() {
            return Err("Refusing to save an empty dictation to history.".into());
        }
        let created_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("Could not timestamp dictation history: {error}"))?
            .as_millis();
        self.records.insert(
            0,
            DictationRecord {
                id: format!("{created_at_ms}-{}", self.records.len()),
                text: text.to_owned(),
                created_at_ms,
            },
        );
        self.records.truncate(MAX_HISTORY_ITEMS);
        self.persist()
    }

    fn persist(&self) -> Result<(), String> {
        let path = Self::file_path();
        let directory = path.parent().expect("history file has a parent directory");
        std::fs::create_dir_all(directory)
            .map_err(|error| format!("Could not create local history directory: {error}"))?;
        let serialized = serde_json::to_string_pretty(&self.records)
            .map_err(|error| format!("Could not serialize local dictation history: {error}"))?;
        let temporary_path = path.with_extension("json.tmp");
        std::fs::write(&temporary_path, serialized)
            .map_err(|error| format!("Could not write local dictation history: {error}"))?;
        std::fs::rename(&temporary_path, &path)
            .map_err(|error| format!("Could not save local dictation history: {error}"))
    }
}
