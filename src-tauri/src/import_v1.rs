//! One-time import from Hams v1 (`~/Library/Application Support/wisper` on macOS,
//! `%APPDATA%\wisper` on Windows). The old files are only read, never changed.

use serde_json::Value;

use crate::settings::{AiProvider, DeliveryMode, SecretCache, SettingsStore};
use crate::store::{NewHistory, Store};

const DONE_KEY: &str = "import_v1_done";

pub fn run(store: &Store, settings_store: &SettingsStore, secrets: &SecretCache) {
    if matches!(store.get_json::<bool>(DONE_KEY), Ok(Some(true))) {
        return;
    }
    // Hams v1 was named "wisper" at the time. Read-only: never deleted.
    let dir = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("wisper");
    if dir.is_dir() {
        let mut summary = Vec::new();
        let read = |name: &str| -> Option<Value> {
            let text = std::fs::read_to_string(dir.join(name)).ok()?;
            serde_json::from_str(&text).ok()
        };

        if let Some(old) = read("settings.json") {
            let mut settings = settings_store.get();
            if let Some(key) = old.get("gemini_api_key").and_then(Value::as_str).filter(|k| !k.trim().is_empty()) {
                if secrets.set(AiProvider::Gemini, key).is_ok() {
                    settings.ai.provider = AiProvider::Gemini;
                    if let Some(model) = old.get("gemini_model").and_then(Value::as_str) {
                        settings.ai.model = model.to_string();
                    }
                    summary.push("Gemini key → Keychain".to_string());
                }
            }
            if let Some(language) = old.get("transcription_language").and_then(Value::as_str) {
                settings.language = language.to_string();
            }
            settings.delivery = match old.get("delivery_mode").and_then(Value::as_str) {
                Some("copy_only") => DeliveryMode::CopyOnly,
                Some("copy_and_paste") => DeliveryMode::Paste,
                _ => settings.delivery,
            };
            let terms = old.get("custom_dictionary").and_then(Value::as_str).unwrap_or_default();
            let mut count = 0;
            for term in terms.split([',', '\n']).map(str::trim).filter(|t| !t.is_empty()) {
                if store.upsert_dictionary(term, None, "imported").is_ok() {
                    count += 1;
                }
            }
            if count > 0 {
                summary.push(format!("{count} vocabulary words"));
            }
            if let Some(path) = old.get("whisper_model_path").and_then(Value::as_str) {
                adopt_model(std::path::Path::new(path));
            }
            if let Err(error) = settings_store.save(settings) {
                tracing::warn!("Could not save imported settings: {error}");
            }
        }

        if let Some(Value::Array(records)) = read("dictation-history.json") {
            let mut count = 0;
            for record in records.iter().rev() {
                let text = record.get("text").and_then(Value::as_str).unwrap_or_default();
                let created_at = record.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
                if !text.trim().is_empty()
                    && store
                        .add_history_at(&NewHistory { text, delivery: "imported", ..Default::default() }, created_at)
                        .is_ok()
                {
                    count += 1;
                }
            }
            summary.push(format!("{count} history items"));
        }

        if let Some(rules) = read("insights.json").and_then(|v| v.get("correction_rules").cloned()) {
            for rule in rules.as_array().into_iter().flatten() {
                let enabled = rule.get("enabled").and_then(Value::as_bool).unwrap_or(true);
                let source = rule.get("source").and_then(Value::as_str).unwrap_or_default();
                let replacement = rule.get("replacement").and_then(Value::as_str).unwrap_or_default();
                if enabled && !source.is_empty() {
                    let _ = store.upsert_dictionary(source, Some(replacement), "imported");
                }
            }
        }
        tracing::info!("Imported Hams v1 data from {}: {}", dir.display(), summary.join(", "));
    }
    let _ = store.set_json(DONE_KEY, &true);
}

/// Reuse an already-downloaded v1 model instead of downloading it again.
/// Hard-links when possible (no extra disk space), otherwise copies in the background.
fn adopt_model(source: &std::path::Path) {
    let Some(file_name) = source.file_name().and_then(|n| n.to_str()) else { return };
    let Some(model) = crate::stt::models::CATALOG.iter().find(|m| m.file == file_name) else { return };
    let target = crate::paths::models_dir().join(model.file);
    if target.exists() || !source.is_file() {
        return;
    }
    if std::fs::hard_link(source, &target).is_ok() {
        tracing::info!("Reused v1 model {}", source.display());
        return;
    }
    let source = source.to_path_buf();
    std::thread::spawn(move || {
        let partial = target.with_extension("bin.part");
        if std::fs::copy(&source, &partial).is_ok() && std::fs::rename(&partial, &target).is_ok() {
            tracing::info!("Copied v1 model {}", source.display());
        }
    });
}
