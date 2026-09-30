use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClassSession {
    pub id: String,
    pub title: String,
    pub created_at_ms: u128,
    pub duration_secs: f32,
    pub audio_path: String,
    pub transcript: String,
    pub notes: String,
    pub status: String,
    pub error_message: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ClassRecordingStatus {
    pub is_recording: bool,
    pub started_at_ms: Option<u128>,
    pub title: Option<String>,
}

#[derive(Debug, Default)]
pub struct ClassSessionStore {
    sessions: Vec<ClassSession>,
}

impl ClassSessionStore {
    fn root_directory() -> PathBuf {
        let mut directory = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        directory.push("wisper");
        directory.push("classes");
        directory
    }

    fn metadata_path() -> PathBuf {
        Self::root_directory().join("sessions.json")
    }

    pub fn load() -> Self {
        let path = Self::metadata_path();
        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(sessions) => Self { sessions },
                Err(error) => {
                    eprintln!("[Classes] Ignoring malformed metadata at {path:?}: {error}");
                    Self::default()
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(error) => {
                eprintln!("[Classes] Could not read metadata at {path:?}: {error}");
                Self::default()
            }
        }
    }

    pub fn sessions(&self) -> Vec<ClassSession> {
        self.sessions.clone()
    }

    pub fn save_recording(
        &mut self,
        title: &str,
        duration_secs: f32,
        wav_bytes: &[u8],
    ) -> Result<ClassSession, String> {
        let created_at_ms = now_ms()?;
        let id = format!("{created_at_ms}-{}", self.sessions.len());
        let directory = Self::root_directory();
        std::fs::create_dir_all(&directory)
            .map_err(|error| format!("Could not create the class recordings directory: {error}"))?;
        let audio_path = directory.join(format!("{id}.wav"));
        let temporary_audio = audio_path.with_extension("wav.tmp");
        std::fs::write(&temporary_audio, wav_bytes)
            .map_err(|error| format!("Could not save the class audio: {error}"))?;
        std::fs::rename(&temporary_audio, &audio_path)
            .map_err(|error| format!("Could not finish saving the class audio: {error}"))?;

        let session = ClassSession {
            id,
            title: normalized_title(title, created_at_ms),
            created_at_ms,
            duration_secs,
            audio_path: audio_path.to_string_lossy().into_owned(),
            transcript: String::new(),
            notes: String::new(),
            status: "processing".into(),
            error_message: None,
        };
        self.sessions.insert(0, session.clone());
        self.persist()?;
        Ok(session)
    }

    pub fn complete(&mut self, id: &str, transcript: String, notes: String) -> Result<(), String> {
        let session = self
            .sessions
            .iter_mut()
            .find(|session| session.id == id)
            .ok_or_else(|| "The class recording no longer exists.".to_string())?;
        session.transcript = transcript;
        session.notes = notes;
        session.status = "ready".into();
        session.error_message = None;
        self.persist()
    }

    pub fn fail(&mut self, id: &str, message: String) -> Result<(), String> {
        let session = self
            .sessions
            .iter_mut()
            .find(|session| session.id == id)
            .ok_or_else(|| "The class recording no longer exists.".to_string())?;
        // The original recording is still useful and remains playable even if
        // transcription or note generation fails.
        session.status = "error".into();
        session.error_message = Some(message);
        self.persist()
    }

    fn persist(&self) -> Result<(), String> {
        let path = Self::metadata_path();
        let directory = path
            .parent()
            .ok_or_else(|| "Class metadata path has no parent directory.".to_string())?;
        std::fs::create_dir_all(directory)
            .map_err(|error| format!("Could not create the class recordings directory: {error}"))?;
        let serialized = serde_json::to_string_pretty(&self.sessions)
            .map_err(|error| format!("Could not serialize class recordings: {error}"))?;
        let temporary_path = path.with_extension("json.tmp");
        std::fs::write(&temporary_path, serialized)
            .map_err(|error| format!("Could not write class metadata: {error}"))?;
        std::fs::rename(&temporary_path, &path)
            .map_err(|error| format!("Could not save class metadata: {error}"))
    }
}

pub fn fallback_notes(title: &str, transcript: &str) -> String {
    let sentences = transcript
        .split_inclusive(['.', '?', '!'])
        .map(str::trim)
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    let overview = sentences
        .iter()
        .take(3)
        .copied()
        .collect::<Vec<_>>()
        .join(" ");
    let key_points = if sentences.is_empty() {
        format!("- Review the full transcript for {title}.")
    } else {
        sentences
            .iter()
            .take(12)
            .map(|sentence| format!("- {sentence}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        "# {title}\n\n## Overview\n{}\n\n## Key takeaways\n{}",
        if overview.is_empty() {
            "The recording was saved, but no spoken words were detected."
        } else {
            &overview
        },
        key_points
    )
}

fn now_ms() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| format!("Could not timestamp the class recording: {error}"))
}

fn normalized_title(title: &str, _created_at_ms: u128) -> String {
    let title = title.trim();
    if title.is_empty() {
        "Untitled class".into()
    } else {
        title.chars().take(100).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_notes_keep_the_transcript_facts() {
        let notes = fallback_notes(
            "Biology",
            "Cells are the basic unit of life. Mitochondria produce energy.",
        );
        assert!(notes.contains("# Biology"));
        assert!(notes.contains("Cells are the basic unit of life."));
        assert!(notes.contains("Mitochondria produce energy."));
    }
}
