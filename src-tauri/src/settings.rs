//! Typed settings stored as one JSON value in SQLite. API keys never live here:
//! they go to the OS keychain (macOS Keychain / Windows Credential Manager).

use std::collections::BTreeMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::input::Shortcut;
use crate::llm::styles::{AppRule, Style, StyleOverride, MAX_PROMPT_CHARS};
use crate::store::Store;

const SETTINGS_KEY: &str = "settings";
const KEYCHAIN_SERVICE: &str = "com.wusla.hams";
/// Earlier v2 builds used this service name; keys found there are moved over.
const LEGACY_KEYCHAIN_SERVICE: &str = "com.mohammedarsh.hams";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(specta::Type))]
pub enum DeliveryMode {
    /// Paste, then put the user's previous clipboard back.
    #[default]
    PasteRestore,
    /// Paste and leave the transcript on the clipboard.
    Paste,
    /// Only copy; the user pastes manually.
    CopyOnly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(specta::Type))]
pub enum AiProvider {
    #[default]
    Off,
    Gemini,
    OpenAi,
    Groq,
    OpenRouter,
    Anthropic,
    /// Local models through Ollama's OpenAI-compatible endpoint. No key needed.
    Ollama,
    /// Any other OpenAI-compatible endpoint.
    Custom,
}

impl AiProvider {
    pub fn keychain_account(self) -> &'static str {
        match self {
            AiProvider::Off => "off",
            AiProvider::Gemini => "gemini",
            AiProvider::OpenAi => "openai",
            AiProvider::Groq => "groq",
            AiProvider::OpenRouter => "openrouter",
            AiProvider::Anthropic => "anthropic",
            AiProvider::Ollama => "ollama",
            AiProvider::Custom => "custom",
        }
    }

    pub fn needs_key(self) -> bool {
        !matches!(self, AiProvider::Off | AiProvider::Ollama)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
#[cfg_attr(test, derive(specta::Type))]
pub struct AiSettings {
    pub provider: AiProvider,
    /// Empty means the provider's recommended fast model.
    pub model: String,
    /// Only used by `Custom` and `Ollama`.
    pub base_url: String,
    /// Dictations with fewer words than this skip AI polish (fast path).
    pub smart_skip_words: u32,
    pub timeout_ms: u64,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            provider: AiProvider::Off,
            model: String::new(),
            base_url: String::new(),
            smart_skip_words: 4,
            timeout_ms: 6_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
#[cfg_attr(test, derive(specta::Type))]
pub struct Settings {
    pub dictate_shortcut: String,
    pub command_shortcut: String,
    /// Empty disables it.
    pub paste_last_shortcut: String,
    /// Tap the dictate shortcut twice quickly to keep recording hands-free.
    pub double_tap_lock: bool,
    /// None = system default input.
    pub microphone: Option<String>,
    /// Keep the microphone open between dictations: instant start, and the
    /// moment just before the key press is included.
    pub keep_mic_warm: bool,
    /// "auto", or a Whisper language code such as "en" or "ml".
    pub language: String,
    /// Model id from the catalog in `stt::models`.
    pub model: String,
    pub delivery: DeliveryMode,
    pub remove_fillers: bool,
    pub voice_commands: bool,
    /// Show a live transcript in the overlay while recording (uses extra CPU/GPU).
    pub live_preview: bool,
    pub app_styles: bool,
    /// Per-style edits to the built-in prompt and AI on/off.
    pub style_overrides: BTreeMap<Style, StyleOverride>,
    /// Apps pinned to a style, checked before automatic detection.
    pub app_rules: Vec<AppRule>,
    pub learn_from_edits: bool,
    /// Opt-in: re-read the field Hams pasted into and learn from corrections made there.
    pub learn_from_fields: bool,
    pub ai: AiSettings,
    /// Look for a new version of Hams every few hours.
    pub check_updates: bool,
    pub onboarding_done: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let mac = cfg!(target_os = "macos");
        Self {
            dictate_shortcut: if mac { "Meta+Shift+Space" } else { "Ctrl+Shift+Space" }.into(),
            command_shortcut: if mac { "Ctrl+Shift+Space" } else { "Ctrl+Alt+Space" }.into(),
            paste_last_shortcut: if mac { "Meta+Alt+Shift+V" } else { "Ctrl+Alt+Shift+V" }.into(),
            double_tap_lock: true,
            microphone: None,
            keep_mic_warm: false,
            language: "auto".into(),
            model: "small".into(),
            delivery: DeliveryMode::PasteRestore,
            remove_fillers: true,
            voice_commands: true,
            live_preview: false,
            app_styles: true,
            style_overrides: BTreeMap::new(),
            app_rules: Vec::new(),
            learn_from_edits: true,
            learn_from_fields: false,
            ai: AiSettings::default(),
            check_updates: true,
            onboarding_done: false,
        }
    }
}

impl Settings {
    pub fn validate(&self) -> AppResult<()> {
        let dictate = Shortcut::parse(&self.dictate_shortcut)
            .map_err(|e| AppError::Settings(format!("Dictation shortcut: {e}")))?;
        let command = Shortcut::parse(&self.command_shortcut)
            .map_err(|e| AppError::Settings(format!("Command shortcut: {e}")))?;
        if dictate == command {
            return Err(AppError::Settings("Dictation and command mode need different shortcuts.".into()));
        }
        if !self.paste_last_shortcut.trim().is_empty() {
            Shortcut::parse(&self.paste_last_shortcut)
                .map_err(|e| AppError::Settings(format!("Paste-last shortcut: {e}")))?;
        }
        for (style, edit) in &self.style_overrides {
            if edit.prompt.as_ref().is_some_and(|p| p.chars().count() > MAX_PROMPT_CHARS) {
                return Err(AppError::Settings(format!(
                    "The {} style prompt is too long (max {MAX_PROMPT_CHARS} characters).",
                    style.label()
                )));
            }
        }
        if self.app_rules.iter().any(|r| r.app.trim().is_empty()) {
            return Err(AppError::Settings("Each app rule needs an app name.".into()));
        }
        if self.ai.timeout_ms < 1_000 || self.ai.timeout_ms > 60_000 {
            return Err(AppError::Settings("AI timeout must be between 1 and 60 seconds.".into()));
        }
        Ok(())
    }
}

/// Shared, cheap-to-read settings with write-through persistence.
pub struct SettingsStore {
    current: RwLock<Settings>,
    store: Arc<Store>,
}

impl SettingsStore {
    pub fn load(store: Arc<Store>) -> Self {
        let current = match store.get_json::<Settings>(SETTINGS_KEY) {
            Ok(Some(settings)) if settings.validate().is_ok() => settings,
            Ok(Some(_)) => {
                tracing::warn!("Stored settings were invalid; using defaults");
                Settings::default()
            }
            Ok(None) => Settings::default(),
            Err(error) => {
                tracing::error!("Could not read settings: {error}");
                Settings::default()
            }
        };
        Self { current: RwLock::new(current), store }
    }

    pub fn get(&self) -> Settings {
        self.current.read().clone()
    }

    pub fn save(&self, settings: Settings) -> AppResult<()> {
        settings.validate()?;
        self.store.set_json(SETTINGS_KEY, &settings)?;
        *self.current.write() = settings;
        Ok(())
    }
}

// ---- secrets ---------------------------------------------------------------

pub fn set_secret(account: &str, value: &str) -> AppResult<()> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, account).map_err(|e| AppError::Settings(e.to_string()))?;
    if value.trim().is_empty() {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(AppError::Settings(e.to_string())),
        }
    } else {
        entry.set_password(value.trim()).map_err(|e| AppError::Settings(e.to_string()))
    }
}

fn read_secret(service: &str, account: &str) -> Option<String> {
    keyring::Entry::new(service, account)
        .ok()?
        .get_password()
        .ok()
        .filter(|s| !s.is_empty())
}

pub fn get_secret(account: &str) -> Option<String> {
    if let Some(secret) = read_secret(KEYCHAIN_SERVICE, account) {
        return Some(secret);
    }
    let legacy = read_secret(LEGACY_KEYCHAIN_SERVICE, account)?;
    if set_secret(account, &legacy).is_ok() {
        if let Ok(entry) = keyring::Entry::new(LEGACY_KEYCHAIN_SERVICE, account) {
            let _ = entry.delete_credential();
        }
        tracing::info!("Moved the {account} API key to the {KEYCHAIN_SERVICE} keychain entry");
    }
    Some(legacy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        Settings::default().validate().unwrap();
    }

    #[test]
    fn unknown_fields_and_missing_fields_still_load() {
        let settings: Settings = serde_json::from_str(r#"{"language":"ml","future_field":1}"#).unwrap();
        assert_eq!(settings.language, "ml");
        assert_eq!(settings.model, "small");
    }

    #[test]
    fn same_shortcut_twice_is_rejected() {
        let mut settings = Settings::default();
        settings.command_shortcut = settings.dictate_shortcut.clone();
        assert!(settings.validate().is_err());
    }

    #[test]
    fn save_and_reload() {
        let store = Arc::new(Store::open_in_memory().unwrap());
        let settings = SettingsStore::load(store.clone());
        let mut changed = settings.get();
        changed.language = "en".into();
        settings.save(changed).unwrap();
        assert_eq!(SettingsStore::load(store).get().language, "en");
    }
}

/// Keychain reads can be slow (and on macOS may prompt after a rebuild), so keys
/// are read once and cached in memory.
#[derive(Default)]
pub struct SecretCache {
    cache: parking_lot::Mutex<std::collections::HashMap<&'static str, Option<String>>>,
}

impl SecretCache {
    pub fn get(&self, provider: AiProvider) -> Option<String> {
        let account = provider.keychain_account();
        self.cache.lock().entry(account).or_insert_with(|| get_secret(account)).clone()
    }

    pub fn forget(&self, provider: AiProvider) {
        self.cache.lock().remove(provider.keychain_account());
    }
}
