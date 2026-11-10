//! The Hams engine for phones, exposed to Kotlin and Swift through UniFFI.
//!
//! The phone app implements [`Host`] (microphone, the text field under the
//! keyboard, UI updates) and [`SecretVault`] (API keys), then drives one
//! [`HamsEngine`]. Everything else (transcription, clean-up, dictionary, styles,
//! AI polish, history) is the same `hams-core` the desktop app runs, so a
//! dictation comes out the same on every device.
//!
//! Threading: methods marked "blocking" do disk or network work; call them off
//! the UI thread. [`Host`] callbacks arrive on engine threads; marshal UI work
//! to the main thread on the platform side.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::Serialize;

use hams_core::audio::push::MicControl;
use hams_core::audio::PushRecorder;
use hams_core::controller::{self, Controller, Deps, Input, Recorder, Ui};
use hams_core::desktop::{AppTarget, Delivered, Desktop};
use hams_core::error::{AppError, AppResult};
use hams_core::input::{Binding, HotkeyAction};
use hams_core::llm::styles::Style;
use hams_core::llm::{self, LlmClient, Polisher, Task};
use hams_core::settings::{AiProvider, DeliveryMode, SecretCache, SecretStore, Settings, SettingsStore};
use hams_core::store::Store;
use hams_core::stt::{models, Transcriber};
use hams_core::text;

mod field;
mod logging;

pub use field::FieldKind;
use field::{format_for_field, join, utf16_len};

uniffi::setup_scaffolding!();

// ---- errors -----------------------------------------------------------------------

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum HamsError {
    /// Every failure carries a sentence that can be shown to the user as is.
    /// (Named `reason`: `message` would clash with Kotlin's Throwable.message.)
    #[error("{reason}")]
    Failed { reason: String },
}

impl From<AppError> for HamsError {
    fn from(error: AppError) -> Self {
        HamsError::Failed { reason: error.to_string() }
    }
}

impl From<serde_json::Error> for HamsError {
    fn from(error: serde_json::Error) -> Self {
        HamsError::Failed { reason: format!("Bad data: {error}") }
    }
}

fn failed(message: impl Into<String>) -> HamsError {
    HamsError::Failed { reason: message.into() }
}

type Result<T> = std::result::Result<T, HamsError>;

// ---- records shared with the phone ------------------------------------------------------

/// The text field the keyboard is attached to.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FieldInfo {
    /// Changes every time the keyboard attaches to a field. 0 = no field.
    pub token: u64,
    /// Android package or iOS bundle id of the app that owns the field.
    pub package: String,
    /// Human-readable app name ("WhatsApp"), for history and per-app styles.
    pub app_name: String,
    pub kind: FieldKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum Phase {
    Idle,
    Recording,
    Transcribing,
    Polishing,
    Delivering,
    Done,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum DictationKind {
    Dictate,
    /// Speak an instruction that rewrites the selected text.
    Command,
    Note,
}

/// What the keyboard should show. Mirrors `controller::Status`.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct DictationStatus {
    pub phase: Phase,
    pub kind: DictationKind,
    /// Hands-free (double-tap or tap-to-toggle) instead of hold-to-talk.
    pub locked: bool,
    /// Unix ms when recording started, for the timer.
    pub started_at: i64,
    pub message: Option<String>,
    /// The written text, on Done.
    pub text: Option<String>,
    pub app: Option<String>,
    /// Live preview is on for this recording.
    pub preview: bool,
}

impl From<&controller::Status> for DictationStatus {
    fn from(status: &controller::Status) -> Self {
        use controller::{Kind, Phase as P};
        DictationStatus {
            phase: match status.phase {
                P::Idle => Phase::Idle,
                P::Recording => Phase::Recording,
                P::Transcribing => Phase::Transcribing,
                P::Polishing => Phase::Polishing,
                P::Delivering => Phase::Delivering,
                P::Done => Phase::Done,
                P::Error => Phase::Error,
            },
            kind: match status.kind {
                Kind::Dictate => DictationKind::Dictate,
                Kind::Command => DictationKind::Command,
                Kind::Note => DictationKind::Note,
            },
            locked: status.locked,
            started_at: status.started_at,
            message: status.message.clone(),
            text: status.text.clone(),
            app: status.app.clone(),
            preview: status.preview,
        }
    }
}

/// The newest dictation, for the keyboard's Heard / Written strip.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Dictation {
    pub id: i64,
    pub created_at: i64,
    /// What Whisper heard.
    pub raw: String,
    /// What was written.
    pub text: String,
    pub app: String,
    pub style: String,
}

/// Which pad was pressed on the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum Pad {
    Dictate,
    Command,
}

impl Pad {
    fn binding(self) -> Binding {
        match self {
            Pad::Dictate => Binding::Dictate,
            Pad::Command => Binding::Command,
        }
    }
}

// ---- what the phone implements ---------------------------------------------------------

/// The phone side: microphone, the text field, and UI updates.
#[uniffi::export(with_foreign)]
pub trait Host: Send + Sync {
    /// Open the microphone and start calling `push_audio`. False if it can't
    /// (no permission, or another app holds it).
    fn mic_start(&self) -> bool;
    fn mic_stop(&self);

    /// The field the keyboard is attached to right now.
    fn field(&self) -> FieldInfo;
    /// Up to `max_utf16` UTF-16 units just before the cursor.
    fn text_before_cursor(&self, max_utf16: u32) -> String;
    /// The selected text, or empty.
    fn selected_text(&self) -> String;
    /// The whole field's text where readable (for learning from edits), or empty.
    fn field_text(&self) -> String;
    /// Insert `text` at the cursor of the field with `token`. False if that
    /// field is no longer attached.
    fn commit(&self, token: u64, text: String) -> bool;
    /// Delete `count` UTF-16 units before the cursor of the field with `token`.
    fn delete_before_cursor(&self, token: u64, count: u32) -> bool;
    fn copy(&self, text: String);

    fn on_status(&self, status: DictationStatus);
    /// Microphone level 0..1 while recording, several times a second.
    fn on_level(&self, level: f32);
    /// Live preview text while recording (when enabled).
    fn on_partial(&self, text: String);
    /// History, the dictionary or settings changed.
    fn on_data_changed(&self);
    /// Engine log lines (level: 1 error … 5 trace).
    fn log(&self, level: u8, message: String);
}

/// Secure storage for API keys (Android Keystore, iOS Keychain).
#[uniffi::export(with_foreign)]
pub trait SecretVault: Send + Sync {
    fn get(&self, account: String) -> Option<String>;
    /// An empty value deletes the key. False if it couldn't be saved.
    fn set(&self, account: String, value: String) -> bool;
}

/// Progress of a model download.
#[uniffi::export(with_foreign)]
pub trait DownloadListener: Send + Sync {
    /// 0..1
    fn on_progress(&self, fraction: f32);
}

// ---- adapters from the phone to hams-core's seams ----------------------------------------------

struct HostMic(Arc<dyn Host>);

impl MicControl for HostMic {
    fn start(&self) -> AppResult<()> {
        if self.0.mic_start() {
            Ok(())
        } else {
            Err(AppError::Permission("Hams can't use the microphone. Open the Hams app to allow it.".into()))
        }
    }
    fn stop(&self) {
        self.0.mic_stop()
    }
}

struct HostUi(Arc<dyn Host>);

impl Ui for HostUi {
    fn status(&self, status: &controller::Status) {
        self.0.on_status(status.into());
    }
    fn level(&self, level: f32) {
        self.0.on_level(level);
    }
    fn partial(&self, text: &str) {
        self.0.on_partial(text.to_string());
    }
    fn data_changed(&self) {
        self.0.on_data_changed();
    }
}

struct VaultSecrets(Arc<dyn SecretVault>);

impl SecretStore for VaultSecrets {
    fn get(&self, account: &str) -> Option<String> {
        self.0.get(account.to_string()).filter(|s| !s.is_empty())
    }
    fn set(&self, account: &str, value: &str) -> AppResult<()> {
        if self.0.set(account.to_string(), value.trim().to_string()) {
            Ok(())
        } else {
            Err(AppError::Settings("the key couldn't be saved on this phone".into()))
        }
    }
}

/// What the keyboard last inserted, so Undo and Rewrite can find it again.
#[derive(Debug, Clone)]
struct LastCommit {
    token: u64,
    text: String,
}

/// The text field as the controller's [`Desktop`]: "frontmost app" is the
/// field's app, and delivering means inserting at the cursor.
struct PhoneField {
    host: Arc<dyn Host>,
    last: Arc<Mutex<Option<LastCommit>>>,
}

impl PhoneField {
    fn target(field: &FieldInfo) -> Option<AppTarget> {
        (field.token != 0).then(|| AppTarget {
            pid: 0,
            id: field.package.clone(),
            name: if field.app_name.is_empty() { field.package.clone() } else { field.app_name.clone() },
            // On phones the "window" is the field: its token.
            window: field.token as isize,
        })
    }
}

impl Desktop for PhoneField {
    fn frontmost(&self) -> Option<AppTarget> {
        Self::target(&self.host.field())
    }

    fn deliver(&self, text: &str, target: Option<&AppTarget>, _mode: DeliveryMode) -> AppResult<Delivered> {
        let field = self.host.field();
        let token = target.map(|t| t.window as u64).unwrap_or(field.token);
        // The user moved to another field (or closed the keyboard) while this
        // was being transcribed: never type into the wrong place.
        if token == 0 || field.token != token || field.kind == FieldKind::Password {
            self.host.copy(text.to_string());
            return Ok(Delivered::Copied);
        }
        let shaped = format_for_field(field.kind, text);
        let before = self.host.text_before_cursor(2);
        let joined = join(&before, &shaped, field.kind);
        if self.host.commit(token, joined.clone()) {
            *self.last.lock() = Some(LastCommit { token, text: joined });
            Ok(Delivered::Pasted)
        } else {
            self.host.copy(text.to_string());
            Ok(Delivered::Copied)
        }
    }

    fn copy(&self, text: &str) -> AppResult<()> {
        self.host.copy(text.to_string());
        Ok(())
    }

    fn selected_text(&self, _target: Option<&AppTarget>, _allow_copy_fallback: bool) -> Option<String> {
        Some(self.host.selected_text()).filter(|s| !s.trim().is_empty())
    }

    fn field_text(&self, target: &AppTarget) -> Option<String> {
        let field = self.host.field();
        if field.token != target.window as u64 || field.kind == FieldKind::Password {
            return None;
        }
        Some(self.host.field_text()).filter(|s| !s.is_empty())
    }
}

fn make_polisher(secrets: &SecretCache, settings: &Settings) -> Arc<dyn Polisher> {
    let key = if settings.ai.provider.needs_key() { secrets.get(settings.ai.provider) } else { None };
    Arc::new(Arc::new(LlmClient::new(settings.ai.clone(), key)))
}

#[cfg(feature = "local-stt")]
fn transcriber() -> Arc<dyn Transcriber> {
    Arc::new(hams_core::stt::WhisperEngine::new())
}

#[cfg(not(feature = "local-stt"))]
fn transcriber() -> Arc<dyn Transcriber> {
    struct Unavailable;
    impl Transcriber for Unavailable {
        fn transcribe(&self, _: &[f32], _: &hams_core::stt::SttRequest) -> AppResult<String> {
            Err(AppError::Transcription("this build has no speech engine".into()))
        }
    }
    Arc::new(Unavailable)
}

/// Settings that make sense on a phone, applied once on first run (and whenever
/// a desktop-only value turns up).
fn phone_defaults(mut settings: Settings) -> Settings {
    if !models::info(&settings.model).is_some_and(|m| m.mobile) {
        settings.model = models::MOBILE_DEFAULT.into();
    }
    // Paste/clipboard modes don't apply: the keyboard inserts directly.
    settings.delivery = DeliveryMode::Paste;
    settings
}

fn style_from(id: &str) -> Result<Style> {
    Style::from_id(id).ok_or_else(|| failed(format!("Unknown style \"{id}\"")))
}

fn provider_from(id: &str) -> Result<AiProvider> {
    serde_json::from_value(serde_json::Value::String(id.to_string()))
        .map_err(|_| failed(format!("Unknown AI provider \"{id}\"")))
}

fn json<T: Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string(value)?)
}

// ---- the engine --------------------------------------------------------------------------

#[derive(uniffi::Object)]
pub struct HamsEngine {
    runtime: tokio::runtime::Runtime,
    controller: Controller,
    recorder: Arc<PushRecorder>,
    store: Arc<Store>,
    settings: Arc<SettingsStore>,
    secrets: Arc<SecretCache>,
    stt: Arc<dyn Transcriber>,
    host: Arc<dyn Host>,
    recording: Arc<AtomicBool>,
    last: Arc<Mutex<Option<LastCommit>>>,
}

#[uniffi::export]
impl HamsEngine {
    /// Open (or create) the database in `data_dir` and start the engine.
    /// Create one per process and keep it for the life of the app.
    #[uniffi::constructor]
    pub fn new(data_dir: String, host: Arc<dyn Host>, vault: Arc<dyn SecretVault>) -> Result<Arc<Self>> {
        logging::init(Arc::clone(&host));
        hams_core::paths::init(&data_dir);
        tracing::info!("Hams engine {} starting in {data_dir}", env!("CARGO_PKG_VERSION"));
        Self::build(&data_dir, host, vault, transcriber()).map(Arc::new)
    }

    // ---- the keyboard ----------------------------------------------------------------

    /// The mic pad went down. Hold to talk; tap twice quickly for hands-free.
    pub fn press(&self, pad: Pad) {
        if self.host.field().kind == FieldKind::Password {
            self.host.on_status(DictationStatus {
                phase: Phase::Error,
                kind: DictationKind::Dictate,
                locked: false,
                started_at: 0,
                message: Some("Hams is off in password fields.".into()),
                text: None,
                app: None,
                preview: false,
            });
            return;
        }
        self.controller.send(Input::Hotkey(HotkeyAction::Pressed(pad.binding())));
    }

    /// The mic pad came up.
    pub fn release(&self, pad: Pad) {
        self.controller.send(Input::Hotkey(HotkeyAction::Released(pad.binding())));
    }

    /// Start hands-free, or finish the current recording.
    pub fn toggle(&self) {
        if !self.is_recording() && self.host.field().kind == FieldKind::Password {
            return self.press(Pad::Dictate);
        }
        self.controller.send(Input::Toggle);
    }

    /// Finish the current recording and insert it.
    pub fn stop(&self) {
        self.controller.send(Input::Stop);
    }

    /// Throw the current recording away.
    pub fn cancel(&self) {
        self.controller.send(Input::Cancel);
    }

    pub fn is_recording(&self) -> bool {
        self.recording.load(std::sync::atomic::Ordering::Relaxed) || self.recorder.is_capturing()
    }

    /// Mono audio in -1..1 from the microphone, while recording.
    pub fn push_audio(&self, frames: Vec<f32>, sample_rate: u32) {
        self.recorder.push(&frames, sample_rate);
    }

    /// 16-bit PCM from the microphone, while recording.
    pub fn push_audio_i16(&self, frames: Vec<i16>, sample_rate: u32) {
        self.recorder.push_i16(&frames, sample_rate);
    }

    /// 16-bit little-endian PCM bytes straight from `AudioRecord.read(ByteArray)`:
    /// the cheapest way across the FFI (no boxed samples).
    pub fn push_audio_pcm16(&self, bytes: Vec<u8>, sample_rate: u32) {
        let frames: Vec<f32> =
            bytes.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32_768.0).collect();
        self.recorder.push(&frames, sample_rate);
    }

    /// Remove the last inserted dictation, if it's still right before the cursor.
    pub fn undo_last(&self) -> bool {
        let Some(last) = self.last.lock().clone() else { return false };
        let field = self.host.field();
        if field.token != last.token {
            return false;
        }
        let count = utf16_len(&last.text);
        if !self.host.text_before_cursor(count).ends_with(&last.text) {
            return false;
        }
        let removed = self.host.delete_before_cursor(last.token, count);
        if removed {
            *self.last.lock() = None;
        }
        removed
    }

    /// Whether Undo / Rewrite still apply to the field under the keyboard.
    pub fn can_undo(&self) -> bool {
        self.last.lock().as_ref().is_some_and(|l| l.token == self.host.field().token)
    }

    /// Blocking (may call the AI). Rewrite the last dictation in `style`
    /// ("messaging", "email", …), replace it in the field and in history, and
    /// return the new text.
    pub fn restyle_last(&self, style: String) -> Result<String> {
        let style = style_from(&style)?;
        let last = self.last.lock().clone().ok_or_else(|| failed("There's nothing to rewrite yet."))?;
        let item = self
            .store
            .recent_history(1)?
            .into_iter()
            .next()
            .ok_or_else(|| failed("There's nothing to rewrite yet."))?;
        let settings = self.settings.get();
        let dictionary = self.store.dictionary()?;
        let polisher = make_polisher(&self.secrets, &settings);
        let restyled = self
            .runtime
            .block_on(controller::restyle(&item.raw, style, &settings, &dictionary, polisher.as_ref()))?;

        let field = self.host.field();
        let count = utf16_len(&last.text);
        let before = self.host.text_before_cursor(count + 2);
        if field.token != last.token || !before.ends_with(&last.text) {
            self.host.copy(restyled.text.clone());
            return Err(failed("The text moved, so the rewrite was copied instead."));
        }
        let prefix = &before[..before.len() - last.text.len()];
        let joined = join(prefix, &format_for_field(field.kind, &restyled.text), field.kind);
        if !self.host.delete_before_cursor(last.token, count) || !self.host.commit(last.token, joined.clone()) {
            self.host.copy(restyled.text.clone());
            return Err(failed("The field changed, so the rewrite was copied instead."));
        }
        *self.last.lock() = Some(LastCommit { token: last.token, text: joined });
        self.store.restyle_history(item.id, &restyled.text, style.id())?;
        self.host.on_data_changed();
        Ok(restyled.text)
    }

    /// Blocking (disk). The newest dictation, for the Heard / Written strip.
    pub fn last_dictation(&self) -> Option<Dictation> {
        let item = self.store.recent_history(1).ok()?.into_iter().next()?;
        Some(Dictation {
            id: item.id,
            created_at: item.created_at,
            raw: item.raw,
            text: item.text,
            app: item.app,
            style: item.style,
        })
    }

    /// Load the speech model in the background so the first dictation is fast.
    pub fn warm_up(&self) {
        let (stt, model) = (Arc::clone(&self.stt), self.settings.get().model);
        std::thread::spawn(move || stt.warm_up(&model));
    }

    /// Free the speech model (the phone is low on memory). It loads again on
    /// the next dictation.
    pub fn release_model(&self) {
        if !self.is_recording() {
            self.stt.release();
        }
    }

    // ---- settings ----------------------------------------------------------------------

    /// `{"settings": {...}, "has_api_key": bool}`. The settings object is the
    /// desktop app's `Settings` (see `src/lib/bindings.ts`); change fields in
    /// place and send the whole object back to keep fields this app doesn't know.
    pub fn settings_json(&self) -> Result<String> {
        let settings = self.settings.get();
        let has_api_key = self.secrets.has(settings.ai.provider);
        json(&serde_json::json!({ "settings": settings, "has_api_key": has_api_key }))
    }

    /// Blocking (disk). Validate and save the settings object.
    pub fn save_settings_json(&self, settings: String) -> Result<()> {
        let incoming: Settings = serde_json::from_str(&settings)?;
        let incoming = phone_defaults(incoming);
        let model_changed = self.settings.get().model != incoming.model;
        self.settings.save(incoming)?;
        if model_changed {
            self.warm_up();
        }
        self.host.on_data_changed();
        Ok(())
    }

    /// Save (or with an empty key, remove) the API key for `provider` ("gemini", "anthropic", …).
    pub fn set_api_key(&self, provider: String, key: String) -> Result<()> {
        Ok(self.secrets.set(provider_from(&provider)?, &key)?)
    }

    pub fn default_ai_model(&self, provider: String) -> Result<String> {
        Ok(LlmClient::default_model(provider_from(&provider)?).to_string())
    }

    /// Blocking (network). Send a sample sentence through the configured provider.
    pub fn test_ai(&self) -> Result<String> {
        let polisher = make_polisher(&self.secrets, &self.settings.get());
        if !polisher.enabled() {
            return Err(failed("Choose an AI provider first."));
        }
        let request = llm::Request {
            task: Task::Polish { transcript: "um so this is a quick test of the the hams formatter does it work".into() },
            style_prompt: String::new(),
            vocabulary: vec!["Hams".into()],
        };
        Ok(self.runtime.block_on(polisher.run(request))?)
    }

    // ---- history -----------------------------------------------------------------------

    /// JSON array of `HistoryItem`, newest first (pinned not moved up).
    pub fn history_recent_json(&self, limit: i64) -> Result<String> {
        json(&self.store.recent_history(limit)?)
    }

    /// JSON array of `HistoryItem` matching `query` (full-text), pinned first.
    pub fn history_json(&self, query: String, limit: i64) -> Result<String> {
        json(&self.store.history(&query, limit)?)
    }

    /// Save an edit; learns suggested corrections from it when enabled. Returns how many.
    pub fn history_update(&self, id: i64, text: String) -> Result<u32> {
        let before = self.store.history_text(id)?.unwrap_or_default();
        self.store.update_history_text(id, &text)?;
        let mut learned = 0;
        if self.settings.get().learn_from_edits {
            for (heard, correct) in text::learn_corrections(&before, &text) {
                self.store.add_suggestion(&heard, &correct)?;
                learned += 1;
            }
        }
        self.host.on_data_changed();
        Ok(learned)
    }

    /// Blocking (may call the AI). Rewrite a history item in `style`, save it in
    /// place, and return `{"text": …, "polished": bool}`.
    pub fn history_restyle(&self, id: i64, style: String) -> Result<String> {
        let style = style_from(&style)?;
        let source = self.store.history_source(id)?.ok_or_else(|| failed("That dictation is gone."))?;
        let settings = self.settings.get();
        let dictionary = self.store.dictionary()?;
        let polisher = make_polisher(&self.secrets, &settings);
        let restyled =
            self.runtime.block_on(controller::restyle(&source, style, &settings, &dictionary, polisher.as_ref()))?;
        self.store.restyle_history(id, &restyled.text, style.id())?;
        self.host.on_data_changed();
        json(&serde_json::json!({ "text": restyled.text, "polished": restyled.polished }))
    }

    pub fn history_pin(&self, id: i64, pinned: bool) -> Result<()> {
        self.store.set_pinned(id, pinned)?;
        self.host.on_data_changed();
        Ok(())
    }

    pub fn history_delete(&self, id: i64) -> Result<()> {
        self.store.delete_history(id)?;
        self.host.on_data_changed();
        Ok(())
    }

    /// Delete all history except pinned items.
    pub fn history_clear(&self) -> Result<()> {
        self.store.clear_history(true)?;
        self.host.on_data_changed();
        Ok(())
    }

    /// JSON `Stats`.
    pub fn stats_json(&self) -> Result<String> {
        json(&self.store.stats()?)
    }

    /// JSON array of app names seen in history (for per-app style rules).
    pub fn recent_apps_json(&self) -> Result<String> {
        json(&self.store.recent_apps(20)?)
    }

    // ---- dictionary, suggestions, snippets ----------------------------------------------------

    pub fn dictionary_json(&self) -> Result<String> {
        json(&self.store.dictionary()?)
    }

    /// Add a word ("Tauri"), or a replacement when `replacement` is set
    /// (`term` is what Whisper hears).
    pub fn dictionary_upsert(&self, term: String, replacement: Option<String>) -> Result<()> {
        let replacement = replacement.filter(|r| !r.trim().is_empty());
        self.store.upsert_dictionary(term.trim(), replacement.as_deref().map(str::trim), "user")?;
        self.host.on_data_changed();
        Ok(())
    }

    pub fn dictionary_delete(&self, id: i64) -> Result<()> {
        self.store.delete_dictionary(id)?;
        self.host.on_data_changed();
        Ok(())
    }

    pub fn suggestions_json(&self) -> Result<String> {
        json(&self.store.suggestions()?)
    }

    pub fn suggestion_resolve(&self, id: i64, approve: bool) -> Result<()> {
        self.store.resolve_suggestion(id, approve)?;
        self.host.on_data_changed();
        Ok(())
    }

    pub fn snippets_json(&self) -> Result<String> {
        json(&self.store.snippets()?)
    }

    pub fn snippet_upsert(&self, trigger: String, expansion: String) -> Result<()> {
        self.store.upsert_snippet(trigger.trim(), expansion.trim())?;
        self.host.on_data_changed();
        Ok(())
    }

    pub fn snippet_delete(&self, id: i64) -> Result<()> {
        self.store.delete_snippet(id)?;
        self.host.on_data_changed();
        Ok(())
    }

    // ---- styles ---------------------------------------------------------------------------

    /// JSON array of `{id, label, prompt, polish}` for the built-in styles.
    pub fn styles_json(&self) -> Result<String> {
        let styles: Vec<_> = Style::ALL
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s, "label": s.label(), "prompt": s.default_prompt(), "polish": s.polish_by_default()
                })
            })
            .collect();
        json(&styles)
    }

    /// Blocking (may call the AI). Run spoken text through a style without
    /// saving anything. Returns `{"text": …, "polished": bool}`.
    pub fn style_try(&self, style: String, text: String) -> Result<String> {
        let style = style_from(&style)?;
        let settings = self.settings.get();
        let dictionary = self.store.dictionary()?;
        let polisher = make_polisher(&self.secrets, &settings);
        let restyled =
            self.runtime.block_on(controller::restyle(&text, style, &settings, &dictionary, polisher.as_ref()))?;
        json(&serde_json::json!({ "text": restyled.text, "polished": restyled.polished }))
    }

    // ---- speech models ----------------------------------------------------------------------

    /// JSON array of phone models: `{id, file, label, size_mb, multilingual, note, mobile, installed}`.
    pub fn models_json(&self) -> Result<String> {
        json(&models::statuses(true))
    }

    /// Blocking (network). Download a model; resumes an interrupted download.
    pub fn download_model(&self, id: String, listener: Arc<dyn DownloadListener>) -> Result<()> {
        self.runtime.block_on(models::download(&id, |fraction| listener.on_progress(fraction)))?;
        if self.settings.get().model == id {
            self.warm_up();
        }
        self.host.on_data_changed();
        Ok(())
    }

    pub fn delete_model(&self, id: String) -> Result<()> {
        if self.settings.get().model == id {
            self.stt.release();
        }
        models::delete(&id)?;
        self.host.on_data_changed();
        Ok(())
    }

    /// Whether the selected model is downloaded (the keyboard can't work without it).
    pub fn model_ready(&self) -> bool {
        models::is_installed(&self.settings.get().model)
    }
}


impl HamsEngine {
    /// The engine with a given speech engine (tests use a fake one). The
    /// database lives directly in `data_dir`.
    fn build(
        data_dir: &str,
        host: Arc<dyn Host>,
        vault: Arc<dyn SecretVault>,
        stt: Arc<dyn Transcriber>,
    ) -> Result<Self> {
        let _ = std::fs::create_dir_all(data_dir);
        let store = Arc::new(Store::open(&std::path::Path::new(data_dir).join("hams.sqlite3"))?);
        let settings = Arc::new(SettingsStore::load(Arc::clone(&store)));
        let current = settings.get();
        let adjusted = phone_defaults(current.clone());
        if adjusted != current {
            settings.save(adjusted)?;
        }

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("hams-engine")
            .enable_all()
            .build()
            .map_err(|e| failed(format!("Couldn't start the engine: {e}")))?;

        let recorder = Arc::new(PushRecorder::new(Arc::new(HostMic(Arc::clone(&host)))));
        let secrets = Arc::new(SecretCache::new(Arc::new(VaultSecrets(vault))));
        let recording = Arc::new(AtomicBool::new(false));
        let last = Arc::new(Mutex::new(None));
        let deps = Deps {
            recorder: Arc::clone(&recorder) as Arc<dyn Recorder>,
            stt: Arc::clone(&stt),
            desktop: Arc::new(PhoneField { host: Arc::clone(&host), last: Arc::clone(&last) }),
            polisher: {
                let secrets = Arc::clone(&secrets);
                Arc::new(move |settings| make_polisher(&secrets, settings))
            },
            ui: Arc::new(HostUi(Arc::clone(&host))),
            store: Arc::clone(&store),
            settings: Arc::clone(&settings),
            recording: Arc::clone(&recording),
            // Dictating into Hams's own screens (onboarding's "Try it") should
            // insert like anywhere else, so nothing counts as "self".
            own_pid: -1,
        };
        let controller = {
            let _guard = runtime.enter();
            Controller::spawn(deps)
        };
        Ok(Self { runtime, controller, recorder, store, settings, secrets, stt, host, recording, last })
    }
}

/// JSON `WordDiff` (`{heard: [{kind, words}], written: [...], dropped, added}`)
/// for drawing the Heard / Written pair.
#[uniffi::export]
pub fn word_diff_json(heard: String, written: String) -> String {
    serde_json::to_string(&text::diff_words(&heard, &written)).unwrap_or_else(|_| "{}".into())
}

/// "2 words removed · 1 word changed or added", or "" when only punctuation changed.
#[uniffi::export]
pub fn change_summary(heard: String, written: String) -> String {
    text::change_summary(&text::diff_words(&heard, &written))
}

#[cfg(test)]
mod tests;
