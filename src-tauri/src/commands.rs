//! IPC commands for the main window. Thin wrappers: real logic lives in the
//! store, settings, controller and platform modules.

use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::controller::{Input, Status};
use crate::error::{AppError, AppResult};
use crate::llm::styles::Style;
use crate::llm::{self, LlmClient, Polisher, Task};
use crate::settings::{self, AiProvider, Settings};
use crate::state::AppState;
use crate::store::{DictionaryEntry, HistoryItem, Snippet, Stats, Suggestion};
use crate::stt::models::{self, ModelStatus};
use crate::{platform, text};

#[derive(Serialize)]
pub struct SettingsView {
    settings: Settings,
    has_api_key: bool,
    platform: &'static str,
    hotkey_error: Option<String>,
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> SettingsView {
    let settings = state.settings.get();
    let has_api_key = settings::get_secret(settings.ai.provider.keychain_account()).is_some();
    SettingsView {
        settings,
        has_api_key,
        platform: if cfg!(target_os = "macos") { "macos" } else { "windows" },
        hotkey_error: state.hotkey_error.lock().clone(),
    }
}

#[tauri::command]
pub fn save_settings(state: State<AppState>, settings: Settings) -> AppResult<()> {
    let model_changed = state.settings.get().model != settings.model;
    state.settings.save(settings.clone())?;
    state.apply_shortcuts(&settings);
    if model_changed {
        state.warm_up(&settings.model);
    }
    Ok(())
}

#[tauri::command]
pub fn set_api_key(state: State<AppState>, provider: AiProvider, key: String) -> AppResult<()> {
    settings::set_secret(provider.keychain_account(), &key)?;
    state.forget_secret(provider);
    Ok(())
}

#[tauri::command]
pub fn get_status(state: State<AppState>) -> Status {
    state.ui.last()
}

#[tauri::command]
pub fn dictation_toggle(state: State<AppState>) {
    state.controller.send(Input::Toggle);
}

#[tauri::command]
pub fn dictation_stop(state: State<AppState>) {
    state.controller.send(Input::Stop);
}

#[tauri::command]
pub fn dictation_cancel(state: State<AppState>) {
    state.controller.send(Input::Cancel);
}

#[tauri::command]
pub fn list_microphones() -> Vec<String> {
    crate::audio::list_microphones()
}

#[tauri::command]
pub fn list_models() -> Vec<ModelStatus> {
    models::statuses()
}

#[derive(Clone, Serialize)]
struct Progress<'a> {
    id: &'a str,
    progress: f32,
}

#[tauri::command]
pub async fn download_model(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<()> {
    let emit_id = id.clone();
    models::download(&id, |progress| {
        let _ = app.emit("model-progress", Progress { id: &emit_id, progress });
    })
    .await?;
    if state.settings.get().model == id {
        state.warm_up(&id);
    }
    Ok(())
}

#[tauri::command]
pub fn delete_model(id: String) -> AppResult<()> {
    models::delete(&id)
}

#[tauri::command]
pub fn history(state: State<AppState>, query: String, limit: Option<i64>) -> AppResult<Vec<HistoryItem>> {
    state.store.history(&query, limit.unwrap_or(200))
}

/// Saving an edit also learns from it (as suggestions the user can approve).
#[tauri::command]
pub fn history_update(state: State<AppState>, id: i64, text: String) -> AppResult<usize> {
    let before = state.store.history_text(id)?.unwrap_or_default();
    state.store.update_history_text(id, &text)?;
    let mut learned = 0;
    if state.settings.get().learn_from_edits {
        for (heard, correct) in text::learn_corrections(&before, &text) {
            state.store.add_suggestion(&heard, &correct)?;
            learned += 1;
        }
    }
    Ok(learned)
}

#[tauri::command]
pub fn history_pin(state: State<AppState>, id: i64, pinned: bool) -> AppResult<()> {
    state.store.set_pinned(id, pinned)
}

#[tauri::command]
pub fn history_delete(state: State<AppState>, id: i64) -> AppResult<()> {
    state.store.delete_history(id)
}

#[tauri::command]
pub fn history_clear(state: State<AppState>) -> AppResult<()> {
    state.store.clear_history(true)
}

#[tauri::command]
pub fn stats(state: State<AppState>) -> AppResult<Stats> {
    state.store.stats()
}

#[tauri::command]
pub fn dictionary(state: State<AppState>) -> AppResult<Vec<DictionaryEntry>> {
    state.store.dictionary()
}

#[tauri::command]
pub fn dictionary_upsert(state: State<AppState>, term: String, replacement: Option<String>) -> AppResult<()> {
    state.store.upsert_dictionary(&term, replacement.as_deref(), "user")
}

#[tauri::command]
pub fn dictionary_delete(state: State<AppState>, id: i64) -> AppResult<()> {
    state.store.delete_dictionary(id)
}

#[tauri::command]
pub fn suggestions(state: State<AppState>) -> AppResult<Vec<Suggestion>> {
    state.store.suggestions()
}

#[tauri::command]
pub fn suggestion_resolve(state: State<AppState>, id: i64, approve: bool) -> AppResult<()> {
    state.store.resolve_suggestion(id, approve)
}

#[tauri::command]
pub fn snippets(state: State<AppState>) -> AppResult<Vec<Snippet>> {
    state.store.snippets()
}

#[tauri::command]
pub fn snippet_upsert(state: State<AppState>, trigger: String, expansion: String) -> AppResult<()> {
    state.store.upsert_snippet(&trigger, &expansion)
}

#[tauri::command]
pub fn snippet_delete(state: State<AppState>, id: i64) -> AppResult<()> {
    state.store.delete_snippet(id)
}

#[tauri::command]
pub fn copy_text(text: String) -> AppResult<()> {
    use crate::platform::Desktop;
    platform::NativeDesktop.copy(&text)
}

#[tauri::command]
pub fn permissions() -> platform::Permissions {
    platform::permissions()
}

#[tauri::command]
pub fn request_accessibility() -> bool {
    platform::request_accessibility()
}

#[tauri::command]
pub fn open_permission_settings(kind: String) {
    platform::open_permission_settings(&kind)
}

#[derive(Serialize)]
pub struct StyleInfo {
    id: &'static str,
    label: &'static str,
    prompt: &'static str,
    polish: bool,
}

#[tauri::command]
pub fn styles() -> Vec<StyleInfo> {
    Style::ALL
        .iter()
        .map(|s| StyleInfo { id: s.id(), label: s.label(), prompt: s.default_prompt(), polish: s.polish_by_default() })
        .collect()
}

#[tauri::command]
pub fn recent_apps(state: State<'_, AppState>) -> AppResult<Vec<String>> {
    state.store.recent_apps(20)
}

#[tauri::command]
pub fn default_ai_model(provider: AiProvider) -> &'static str {
    LlmClient::default_model(provider)
}

/// Send a sample sentence through the configured provider so the user can check
/// their key and model before relying on them.
#[tauri::command]
pub async fn test_ai(state: State<'_, AppState>) -> AppResult<String> {
    let polisher: Arc<dyn Polisher> = state.polisher_for(&state.settings.get());
    if !polisher.enabled() {
        return Err(AppError::Polish("choose a provider first".into()));
    }
    polisher
        .run(llm::Request {
            task: Task::Polish { transcript: "um so this is a quick test of the the hams formatter does it work".into() },
            style_prompt: String::new(),
            vocabulary: vec!["Hams".into()],
        })
        .await
}

#[tauri::command]
pub fn autostart_enabled(app: AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> AppResult<()> {
    use tauri_plugin_autostart::ManagerExt;
    let manager = app.autolaunch();
    if enabled { manager.enable() } else { manager.disable() }.map_err(|e| AppError::Settings(e.to_string()))
}
