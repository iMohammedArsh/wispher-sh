pub mod audio;
pub mod class_sessions;
pub mod clipboard;
pub mod gemini;
pub mod history;
pub mod hotkey;
pub mod insights;
pub mod overlay;
pub mod transcribe;

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State,
};

use audio::{AudioDiagnostics, AudioRecorder};
use class_sessions::{fallback_notes, ClassRecordingStatus, ClassSession, ClassSessionStore};
use clipboard::{ClipboardInjector, DeliveryMode, ForegroundTarget};
#[allow(unused_imports)]
use gemini::GeminiFormatter;
use history::{DictationHistory, DictationRecord};
use hotkey::HotkeyListener;
use insights::{category_for_process, foreground_process_name, InsightsSummary, LocalInsights};
use overlay::OverlayController;
use transcribe::{WarmWhisperServer, WhisperTranscriber};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppSettings {
    pub gemini_api_key: String,
    pub gemini_model: String,
    pub custom_dictionary: String,
    pub whisper_model_path: String,
    pub whisper_binary_path: String,
    pub audio_gain_factor: f32,
    pub hotkey_combo: String,
    #[serde(default = "default_transcription_language")]
    pub transcription_language: String,
    #[serde(default = "default_delivery_mode")]
    pub delivery_mode: String,
    #[serde(default)]
    pub app_usage_tracking: bool,
}

fn default_delivery_mode() -> String {
    "copy_and_paste".into()
}

fn default_transcription_language() -> String {
    "auto".into()
}

fn default_hotkey_combo() -> String {
    if cfg!(target_os = "macos") {
        "Cmd+Shift+Space".into()
    } else {
        "Ctrl+Shift+Space".into()
    }
}

fn bundled_whisper_paths() -> (std::path::PathBuf, std::path::PathBuf) {
    let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    #[cfg(target_os = "windows")]
    {
        let root = project_root.join("tools").join("whisper.cpp");
        let multilingual_small = root.join("models").join("ggml-small.bin");
        let fallback = root.join("models").join("ggml-base.en.bin");
        return (
            root.join("Release").join("whisper-cli.exe"),
            if multilingual_small.is_file() {
                multilingual_small
            } else {
                fallback
            },
        );
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(executable) = std::env::current_exe() {
            if let Some(contents) = executable.parent().and_then(std::path::Path::parent) {
                let resources = contents.join("Resources").join("whisper");
                let binary = resources.join("whisper-cli");
                let model = resources.join("ggml-small.bin");
                if binary.is_file() && model.is_file() {
                    return (binary, model);
                }
            }
        }
        let root = project_root.join("tools").join("whisper.cpp");
        return (
            root.join("macos").join("whisper-cli"),
            root.join("models").join("ggml-small.bin"),
        );
    }
    #[cfg(target_os = "linux")]
    return (
        std::path::PathBuf::from("/home/administrator/whisper.cpp/main"),
        std::path::PathBuf::from("/home/administrator/whisper.cpp/models/ggml-base.en.bin"),
    );

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    return (
        std::path::PathBuf::from("whisper-cli"),
        std::path::PathBuf::from("ggml-small.bin"),
    );
}

impl Default for AppSettings {
    fn default() -> Self {
        let (whisper_binary_path, whisper_model_path) = bundled_whisper_paths();
        Self {
            gemini_api_key: String::new(),
            gemini_model: "gemini-3.5-flash".into(),
            custom_dictionary: String::new(),
            whisper_model_path: whisper_model_path.to_string_lossy().into_owned(),
            whisper_binary_path: whisper_binary_path.to_string_lossy().into_owned(),
            audio_gain_factor: 4.0,
            hotkey_combo: default_hotkey_combo(),
            transcription_language: default_transcription_language(),
            delivery_mode: default_delivery_mode(),
            app_usage_tracking: false,
        }
    }
}

impl AppSettings {
    pub fn get_config_path() -> std::path::PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        path.push("wisper");
        let _ = std::fs::create_dir_all(&path);
        path.push("settings.json");
        path
    }

    pub fn load() -> Self {
        let path = Self::get_config_path();
        let mut settings = if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(s) = serde_json::from_str::<AppSettings>(&content) {
                    println!("[Settings] Loaded settings from disk at {:?}", path);
                    s
                } else {
                    eprintln!(
                        "[Settings] Ignoring malformed settings file at {:?}; using defaults.",
                        path
                    );
                    Self::default()
                }
            } else {
                Self::default()
            }
        } else {
            Self::default()
        };

        // Alt+Space is reserved for the Windows system menu. Keep existing installations
        // aligned with the non-conflicting system-wide shortcut.
        if settings.hotkey_combo == "Alt+Space" || settings.hotkey_combo == "Ctrl+Alt+Space" {
            settings.hotkey_combo = default_hotkey_combo();
            let _ = settings.save();
        }

        #[cfg(target_os = "macos")]
        if settings.hotkey_combo == "Ctrl+Shift+Space" {
            settings.hotkey_combo = default_hotkey_combo();
            let _ = settings.save();
        }

        // Migrate stale WSL paths to the bundled native Windows Whisper installation.
        let (default_bin, default_model) = bundled_whisper_paths();
        let mut paths_updated = false;

        if !std::path::Path::new(&settings.whisper_binary_path).exists() && default_bin.is_file() {
            settings.whisper_binary_path = default_bin.to_string_lossy().into_owned();
            paths_updated = true;
        }
        if !std::path::Path::new(&settings.whisper_model_path).exists() && default_model.is_file() {
            settings.whisper_model_path = default_model.to_string_lossy().into_owned();
            paths_updated = true;
        }
        // Existing English-only selections cannot transcribe Malayalam. When Auto or
        // Malayalam is selected and the bundled multilingual model is available,
        // migrate to it without removing the user's English-only model.
        let selected_is_english_only = settings.whisper_model_path.ends_with(".en.bin");
        if settings.transcription_language != "en"
            && selected_is_english_only
            && default_model
                .file_name()
                .is_some_and(|name| name == "ggml-small.bin")
        {
            settings.whisper_model_path = default_model.to_string_lossy().into_owned();
            paths_updated = true;
        }
        if paths_updated {
            let _ = settings.save();
        }

        settings
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::get_config_path();
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;
        let temporary_path = path.with_extension("json.tmp");
        std::fs::write(&temporary_path, content)
            .map_err(|e| format!("Failed to write temporary settings file: {}", e))?;
        std::fs::rename(&temporary_path, &path)
            .map_err(|e| format!("Failed to replace settings at {:?}: {}", path, e))?;
        println!("[Settings] Saved settings to disk at {:?}", path);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppStatus {
    pub state_name: String,
    pub recording_mode: Option<String>,
    pub last_dictation: String,
    pub error_message: Option<String>,
    pub delivery_message: Option<String>,
}

impl Default for AppStatus {
    fn default() -> Self {
        Self {
            state_name: "Idle".into(),
            recording_mode: None,
            last_dictation: String::new(),
            error_message: None,
            delivery_message: None,
        }
    }
}

pub struct AppState {
    pub settings: Arc<Mutex<AppSettings>>,
    pub status: Arc<Mutex<AppStatus>>,
    pub recorder: Arc<AudioRecorder>,
    pub history: Arc<Mutex<DictationHistory>>,
    pub insights: Arc<Mutex<LocalInsights>>,
    pub delivery_target: Arc<Mutex<Option<ForegroundTarget>>>,
    pub recording_category: Arc<Mutex<String>>,
    pub overlay: OverlayController,
    pub warm_whisper: Arc<Mutex<WarmWhisperServer>>,
    pub class_recorder: Arc<AudioRecorder>,
    pub class_sessions: Arc<Mutex<ClassSessionStore>>,
    pub class_recording: Arc<Mutex<ClassRecordingStatus>>,
}

pub mod commands {
    use super::*;

    #[tauri::command]
    pub fn get_settings(state: State<'_, AppState>) -> AppSettings {
        state.settings.lock().unwrap().clone()
    }

    #[tauri::command]
    pub fn save_settings(state: State<'_, AppState>, settings: AppSettings) -> Result<(), String> {
        settings.save()?;
        let mut current = state.settings.lock().unwrap();
        *current = settings;
        Ok(())
    }

    #[tauri::command]
    pub fn get_app_status(state: State<'_, AppState>) -> AppStatus {
        state.status.lock().unwrap().clone()
    }

    #[tauri::command]
    pub fn overlay_ready(app: AppHandle, state: State<'_, AppState>) {
        state.overlay.reveal_when_ready(&app);
    }

    #[tauri::command]
    pub fn get_audio_diagnostics(state: State<'_, AppState>) -> AudioDiagnostics {
        state.recorder.diagnostics()
    }

    #[tauri::command]
    pub fn get_class_sessions(state: State<'_, AppState>) -> Vec<ClassSession> {
        state.class_sessions.lock().unwrap().sessions()
    }

    #[tauri::command]
    pub fn get_class_recording_status(state: State<'_, AppState>) -> ClassRecordingStatus {
        state.class_recording.lock().unwrap().clone()
    }

    #[tauri::command]
    pub fn start_class_recording(
        app: AppHandle,
        state: State<'_, AppState>,
        title: String,
    ) -> Result<ClassRecordingStatus, String> {
        if state.status.lock().unwrap().state_name == "Recording" {
            return Err("Finish the current dictation before recording a class.".into());
        }
        if state.class_recording.lock().unwrap().is_recording {
            return Ok(state.class_recording.lock().unwrap().clone());
        }
        state.class_recorder.start_recording()?;
        let started_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| format!("Could not timestamp the class recording: {error}"))?
            .as_millis();
        let status = ClassRecordingStatus {
            is_recording: true,
            started_at_ms: Some(started_at_ms),
            title: Some(title.trim().to_string()),
        };
        *state.class_recording.lock().unwrap() = status.clone();
        let _ = app.emit("class-recording-changed", status.clone());
        Ok(status)
    }

    #[tauri::command]
    pub fn stop_class_recording(
        app: AppHandle,
        state: State<'_, AppState>,
    ) -> Result<ClassSession, String> {
        let recording = state.class_recording.lock().unwrap().clone();
        if !recording.is_recording {
            return Err("No class recording is active.".into());
        }
        let gain_factor = state.settings.lock().unwrap().audio_gain_factor;
        let captured = match state.class_recorder.stop_recording_for_archive(gain_factor) {
            Ok(captured) => captured,
            Err(error) => {
                *state.class_recording.lock().unwrap() = ClassRecordingStatus::default();
                let _ = app.emit("class-recording-changed", ClassRecordingStatus::default());
                return Err(error);
            }
        };
        let title = recording.title.unwrap_or_default();
        let session = match state.class_sessions.lock().unwrap().save_recording(
            &title,
            captured.diagnostics.recorded_duration_secs,
            &captured.wav_bytes,
        ) {
            Ok(session) => session,
            Err(error) => {
                *state.class_recording.lock().unwrap() = ClassRecordingStatus::default();
                let _ = app.emit("class-recording-changed", ClassRecordingStatus::default());
                return Err(error);
            }
        };
        *state.class_recording.lock().unwrap() = ClassRecordingStatus::default();
        let _ = app.emit("class-recording-changed", ClassRecordingStatus::default());
        let _ = app.emit(
            "class-sessions-changed",
            state.class_sessions.lock().unwrap().sessions(),
        );

        let (api_key, gemini_model, custom_dict, model_path, binary_path, language) = {
            let settings = state.settings.lock().unwrap();
            (
                settings.gemini_api_key.clone(),
                settings.gemini_model.clone(),
                settings.custom_dictionary.clone(),
                settings.whisper_model_path.clone(),
                settings.whisper_binary_path.clone(),
                settings.transcription_language.clone(),
            )
        };
        let session_id = session.id.clone();
        let session_title = session.title.clone();
        let wav_bytes = captured.wav_bytes;
        let sessions = Arc::clone(&state.class_sessions);
        let app_handle = app.clone();
        tauri::async_runtime::spawn(async move {
            let vad_model_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("tools")
                .join("whisper.cpp")
                .join("models")
                .join("ggml-silero-v6.2.0.bin");
            let transcript_result = tauri::async_runtime::spawn_blocking(move || {
                WhisperTranscriber::new(
                    model_path,
                    binary_path,
                    language,
                    custom_dict,
                    vad_model_path.is_file().then_some(vad_model_path),
                )
                .transcribe(&wav_bytes)
            })
            .await
            .unwrap_or_else(|error| Err(format!("Class transcription task failed: {error}")));

            match transcript_result {
                Ok(transcript) => {
                    let notes = if api_key.trim().is_empty() {
                        fallback_notes(&session_title, &transcript)
                    } else {
                        GeminiFormatter::new(api_key, gemini_model)
                            .create_class_notes(&transcript, &session_title)
                            .await
                            .unwrap_or_else(|error| {
                                eprintln!("[Classes] AI notes fallback: {error}");
                                fallback_notes(&session_title, &transcript)
                            })
                    };
                    if let Err(error) =
                        sessions
                            .lock()
                            .unwrap()
                            .complete(&session_id, transcript, notes)
                    {
                        eprintln!("[Classes] Could not finish class metadata: {error}");
                    }
                }
                Err(error) => {
                    let message = format!(
                        "The full audio was saved, but Hams could not create the transcript: {error}"
                    );
                    if let Err(save_error) = sessions.lock().unwrap().fail(&session_id, message) {
                        eprintln!("[Classes] Could not save class error state: {save_error}");
                    }
                }
            }
            let updated = sessions.lock().unwrap().sessions();
            let _ = app_handle.emit("class-sessions-changed", updated);
        });

        Ok(session)
    }

    #[tauri::command]
    pub fn get_dictation_history(state: State<'_, AppState>) -> Vec<DictationRecord> {
        state.history.lock().unwrap().records()
    }

    #[tauri::command]
    pub fn get_insights_summary(state: State<'_, AppState>) -> InsightsSummary {
        state.insights.lock().unwrap().summary()
    }

    #[tauri::command]
    pub fn add_correction_rule(
        state: State<'_, AppState>,
        source: String,
        replacement: String,
    ) -> Result<InsightsSummary, String> {
        let mut insights = state.insights.lock().unwrap();
        insights.add_correction_rule(source, replacement)?;
        Ok(insights.summary())
    }

    #[tauri::command]
    pub fn copy_to_clipboard(text: String) -> Result<String, String> {
        ClipboardInjector::copy(&text).map(|result| result.message)
    }

    #[tauri::command]
    pub fn copy_last_transcript(state: State<'_, AppState>) -> Result<String, String> {
        let text = state
            .history
            .lock()
            .unwrap()
            .latest_text()
            .ok_or_else(|| "There is no saved dictation to copy yet.".to_string())?;
        ClipboardInjector::copy(&text).map(|result| result.message)
    }

    #[tauri::command]
    pub fn paste_last_transcript(
        app: AppHandle,
        state: State<'_, AppState>,
    ) -> Result<String, String> {
        let text = state
            .history
            .lock()
            .unwrap()
            .latest_text()
            .ok_or_else(|| "There is no saved dictation to paste yet.".to_string())?;
        match deliver_text(&state, &text) {
            Ok(result) => {
                set_delivery_message(&app, &state, result.message.clone());
                Ok(result.message)
            }
            Err(error) => {
                set_delivery_message(
                    &app,
                    &state,
                    format!("Could not paste the last transcript: {error}"),
                );
                Err(error)
            }
        }
    }

    #[tauri::command]
    pub fn paste_text(
        app: AppHandle,
        state: State<'_, AppState>,
        text: String,
    ) -> Result<String, String> {
        match deliver_text(&state, &text) {
            Ok(result) => {
                set_delivery_message(&app, &state, result.message.clone());
                Ok(result.message)
            }
            Err(error) => {
                set_delivery_message(&app, &state, format!("Could not paste text: {error}"));
                Err(error)
            }
        }
    }

    #[tauri::command]
    pub fn report_window_action_failure(
        app: AppHandle,
        state: State<'_, AppState>,
        action: String,
        details: String,
    ) {
        set_delivery_message(
            &app,
            &state,
            format!("Could not {action} the Hams window: {details}"),
        );
    }

    #[tauri::command]
    pub fn trigger_start_dictation(
        app: AppHandle,
        state: State<'_, AppState>,
    ) -> Result<(), String> {
        start_recording_internal(&app, &state, "Manual")
    }

    #[tauri::command]
    pub fn trigger_stop_dictation(
        app: AppHandle,
        state: State<'_, AppState>,
    ) -> Result<(), String> {
        stop_recording_internal(&app, &state)
    }

    #[tauri::command]
    pub fn trigger_cancel_dictation(
        app: AppHandle,
        state: State<'_, AppState>,
    ) -> Result<(), String> {
        if state.status.lock().unwrap().state_name != "Recording" {
            return Ok(());
        }
        state.recorder.discard_recording();
        set_status(&app, &state, "Idle", None, None);
        set_delivery_message(&app, &state, "Recording discarded.".into());
        Ok(())
    }

    #[tauri::command]
    pub fn trigger_toggle_handsfree(
        app: AppHandle,
        state: State<'_, AppState>,
    ) -> Result<(), String> {
        let state_name = state.status.lock().unwrap().state_name.clone();
        match state_name.as_str() {
            "Recording" => stop_recording_internal(&app, &state),
            "Idle" | "Error" => start_recording_internal(&app, &state, "Hands-free"),
            _ => Ok(()),
        }
    }

    pub(crate) fn deliver_text(
        state: &AppState,
        text: &str,
    ) -> Result<clipboard::DeliveryResult, String> {
        let delivery_mode = {
            let settings = state.settings.lock().unwrap();
            DeliveryMode::from_setting(&settings.delivery_mode)
        };
        let target = state.delivery_target.lock().unwrap().clone();
        ClipboardInjector::deliver(text, delivery_mode, target.as_ref())
    }
}

fn set_status(
    app: &AppHandle,
    state: &AppState,
    name: &str,
    last_text: Option<String>,
    err: Option<String>,
) {
    let mut status = state.status.lock().unwrap();
    status.state_name = name.to_string();
    if name != "Recording" {
        status.recording_mode = None;
    }
    if let Some(t) = last_text {
        status.last_dictation = t;
    }
    status.error_message = err;
    state.overlay.set_state(
        app,
        match name {
            "Recording" => "recording",
            "Transcribing Audio" | "Formatting Text" | "Pasting" => "processing",
            "Error" => "error",
            _ => "idle",
        },
    );
    let _ = app.emit("status-changed", status.clone());
    drop(status);

    // An overlay error is a transient Dynamic Island notification. Keep the full
    // error in app state for the main window, but return the overlay to its idle size.
    if name == "Error" {
        let app = app.clone();
        let overlay = state.overlay.clone();
        let status = Arc::clone(&state.status);
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(5));
            if status.lock().unwrap().state_name == "Error" {
                overlay.set_state(&app, "idle");
            }
        });
    }
}

fn set_delivery_message(app: &AppHandle, state: &AppState, message: String) {
    let mut status = state.status.lock().unwrap();
    status.delivery_message = Some(message);
    let _ = app.emit("status-changed", status.clone());
}

fn start_recording_internal(
    app: &AppHandle,
    state: &AppState,
    recording_mode: &str,
) -> Result<(), String> {
    if state.class_recording.lock().unwrap().is_recording {
        return Err("A class recording is active. Finish it before starting dictation.".into());
    }
    // Preserve the field the user selected before recording changes any Wisper UI state.
    *state.delivery_target.lock().unwrap() = ClipboardInjector::capture_foreground_target();
    let app_usage_tracking = state.settings.lock().unwrap().app_usage_tracking;
    *state.recording_category.lock().unwrap() = if app_usage_tracking {
        category_for_process(foreground_process_name().as_deref())
    } else {
        "other".into()
    };

    if let Err(e) = state.recorder.start_recording() {
        eprintln!("[Recording] Could not start microphone capture: {e}");
        set_status(app, state, "Error", None, Some(e.clone()));
        return Err(e);
    }
    set_status(app, state, "Recording", None, None);
    let mut status = state.status.lock().unwrap();
    status.recording_mode = Some(recording_mode.into());
    let _ = app.emit("status-changed", status.clone());
    drop(status);

    let overlay = state.overlay.clone();
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let app_for_level = app.clone();
    let recorder = Arc::clone(&state.recorder);
    std::thread::spawn(move || {
        while recorder.diagnostics().is_recording {
            let level = recorder.diagnostics().rms_amplitude;
            overlay.set_level(level);
            // The native Windows overlay is a Tauri webview and uses this event.
            // WSL uses the separate WPF overlay state file, so sending 16 events/sec
            // through the main WebKit renderer would only create needless repaint load.
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            let _ = app_for_level.emit("audio-level", level);
            std::thread::sleep(std::time::Duration::from_millis(60));
        }
    });
    Ok(())
}

fn stop_recording_internal(app: &AppHandle, state: &AppState) -> Result<(), String> {
    if state.status.lock().unwrap().state_name != "Recording" {
        return Ok(());
    }
    set_status(app, state, "Transcribing Audio", None, None);

    let (gain_factor, api_key, gemini_model, custom_dict, model_path, binary_path, language) = {
        let s = state.settings.lock().unwrap();
        (
            s.audio_gain_factor,
            s.gemini_api_key.clone(),
            s.gemini_model.clone(),
            s.custom_dictionary.clone(),
            s.whisper_model_path.clone(),
            s.whisper_binary_path.clone(),
            s.transcription_language.clone(),
        )
    };

    let captured_audio = match state.recorder.stop_recording_and_process(gain_factor) {
        Ok(info) => info,
        Err(e) => {
            set_status(app, state, "Error", None, Some(e.clone()));
            return Err(e);
        }
    };

    let audio_diagnostics = captured_audio.diagnostics.clone();
    let wav_bytes = captured_audio.wav_bytes;
    let sample_count = (audio_diagnostics.recorded_duration_secs
        * audio_diagnostics.native_sample_rate as f32) as usize;
    let duration_secs = audio_diagnostics.recorded_duration_secs;
    let peak_amplitude = audio_diagnostics.peak_amplitude;
    let kb_size = wav_bytes.len() as f32 / 1024.0;
    let app_handle = app.clone();
    let recorder_ref = Arc::clone(&state.recorder);
    let settings_ref = Arc::clone(&state.settings);
    let status_ref = Arc::clone(&state.status);
    let history_ref = Arc::clone(&state.history);
    let insights_ref = Arc::clone(&state.insights);
    let delivery_target_ref = Arc::clone(&state.delivery_target);
    let recording_category_ref = Arc::clone(&state.recording_category);
    let overlay_ref = state.overlay.clone();
    let warm_whisper_ref = Arc::clone(&state.warm_whisper);
    let class_recorder_ref = Arc::clone(&state.class_recorder);
    let class_sessions_ref = Arc::clone(&state.class_sessions);
    let class_recording_ref = Arc::clone(&state.class_recording);

    tauri::async_runtime::spawn(async move {
        let local_state = AppState {
            settings: settings_ref,
            status: status_ref,
            recorder: recorder_ref,
            history: history_ref,
            insights: insights_ref,
            delivery_target: delivery_target_ref,
            recording_category: recording_category_ref,
            overlay: overlay_ref,
            warm_whisper: warm_whisper_ref,
            class_recorder: class_recorder_ref,
            class_sessions: class_sessions_ref,
            class_recording: class_recording_ref,
        };

        // Step 1: Offline local Whisper.cpp speech recognition (No cloud API / No Gemini)
        let vad_model_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("tools")
            .join("whisper.cpp")
            .join("models")
            .join("ggml-silero-v6.2.0.bin");
        let warm_endpoint = local_state
            .warm_whisper
            .lock()
            .unwrap()
            .ensure_started(&model_path, &binary_path);
        let transcribed_words = match match warm_endpoint {
            Ok(endpoint) => {
                WarmWhisperServer::transcribe(&endpoint, &wav_bytes, &language, &custom_dict)
                    .await
                    .or_else(|warm_error| {
                        eprintln!("[Local Whisper] Warm server fallback: {warm_error}");
                        WhisperTranscriber::new(
                            model_path,
                            binary_path,
                            language,
                            custom_dict.clone(),
                            vad_model_path.is_file().then_some(vad_model_path),
                        )
                        .transcribe(&wav_bytes)
                    })
            }
            Err(warm_error) => {
                eprintln!("[Local Whisper] Warm server unavailable: {warm_error}");
                WhisperTranscriber::new(
                    model_path,
                    binary_path,
                    language,
                    custom_dict.clone(),
                    vad_model_path.is_file().then_some(vad_model_path),
                )
                .transcribe(&wav_bytes)
            }
        } {
            Ok(words) => words,
            Err(e) => {
                eprintln!("[Local Whisper Error]: {}", e);
                let clipping_hint = if audio_diagnostics.is_clipping {
                    " The microphone clipped; lower its level in Windows Sound settings and retry."
                } else {
                    ""
                };
                set_status(
                    &app_handle,
                    &local_state,
                    "Error",
                    None,
                    Some(format!(
                        "Transcription failed after {:.1}s from {} (peak {:.0}%, RMS {:.0}%). {}{}",
                        audio_diagnostics.recorded_duration_secs,
                        audio_diagnostics.device_name,
                        audio_diagnostics.peak_amplitude * 100.0,
                        audio_diagnostics.rms_amplitude * 100.0,
                        e,
                        clipping_hint,
                    )),
                );
                return;
            }
        };

        let _telemetry_display = format!(
            "{}\n\n───── Dictation Telemetry ─────\n⏱️ {:.2}s | 📊 {} samples | 🔊 Peak: {:.2} | 📁 {:.1} KB",
            transcribed_words, duration_secs, sample_count, peak_amplitude, kb_size
        );

        println!(
            "\n[Offline Dictation Completed]\nSpoken Words: {}\nStats: {:.2}s, {:.1}KB",
            transcribed_words, duration_secs, kb_size
        );

        let learned_transcript = match local_state
            .insights
            .lock()
            .unwrap()
            .apply_correction_rules(&transcribed_words)
        {
            Ok(text) => text,
            Err(error) => {
                eprintln!("[Insights] Could not apply approved correction rules: {error}");
                transcribed_words.clone()
            }
        };

        // Step 2: Polish the local transcript when the user has configured the optional
        // formatter. Gemini receives text only, never the recorded audio.
        let (final_display, formatting_notice) = if api_key.trim().is_empty() {
            (
                learned_transcript.clone(),
                Some("AI formatting is off because no Gemini API key is configured; pasted the raw transcript.".to_string()),
            )
        } else {
            set_status(&app_handle, &local_state, "Formatting Text", None, None);
            match GeminiFormatter::new(api_key, gemini_model)
                .format_text(&learned_transcript, &custom_dict)
                .await
            {
                Ok(formatted) => (formatted, None),
                Err(error) => {
                    eprintln!("[AI Formatting] {error}");
                    (
                        learned_transcript.clone(),
                        Some(format!(
                            "AI formatting was unavailable; pasted the raw transcript. {error}"
                        )),
                    )
                }
            }
        };

        // Step 3: Render and deliver before writing local history/insights. This
        // keeps disk I/O out of the latency-sensitive paste path.
        set_status(
            &app_handle,
            &local_state,
            "Pasting",
            Some(final_display.clone()),
            None,
        );

        // Step 4: Copy and optionally paste. Native paste replaces a selected editable field.
        match commands::deliver_text(&local_state, &final_display) {
            Ok(result) => {
                let message = match formatting_notice {
                    Some(notice) => format!("{} {notice}", result.message),
                    None => result.message,
                };
                set_delivery_message(&app_handle, &local_state, message);
            }
            Err(error) => {
                eprintln!("[Clipboard] Delivery error: {error}");
                set_delivery_message(
                    &app_handle,
                    &local_state,
                    format!("Dictation was transcribed, but delivery failed: {error}"),
                );
            }
        }

        // Step 5: Persist after delivery. A persistence failure must never delay or
        // reverse a successful paste into the selected input.
        if let Err(error) = local_state.history.lock().unwrap().add(&final_display) {
            eprintln!("[History] Persistence error: {error}");
        }

        let category = local_state.recording_category.lock().unwrap().clone();
        if let Err(error) = local_state
            .insights
            .lock()
            .unwrap()
            .add_completed_dictation(
                &final_display,
                (duration_secs * 1_000.0).max(0.0) as u64,
                category,
            )
        {
            eprintln!("[Insights] Persistence error: {error}");
        }

        set_status(&app_handle, &local_state, "Idle", Some(final_display), None);
    });

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let loaded_settings = AppSettings::load();
    let app_state = AppState {
        settings: Arc::new(Mutex::new(loaded_settings)),
        status: Arc::new(Mutex::new(AppStatus::default())),
        recorder: Arc::new(AudioRecorder::new()),
        history: Arc::new(Mutex::new(DictationHistory::load())),
        insights: Arc::new(Mutex::new(LocalInsights::load())),
        delivery_target: Arc::new(Mutex::new(None)),
        recording_category: Arc::new(Mutex::new("other".into())),
        overlay: OverlayController::start(),
        warm_whisper: Arc::new(Mutex::new(WarmWhisperServer::default())),
        class_recorder: Arc::new(AudioRecorder::new()),
        class_sessions: Arc::new(Mutex::new(ClassSessionStore::load())),
        class_recording: Arc::new(Mutex::new(ClassRecordingStatus::default())),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .setup(|app| {
            // Warm the fully local model once at startup. The recorder still works
            // if this fails; its legacy CLI path remains available as a fallback.
            let settings = Arc::clone(&app.state::<AppState>().settings);
            let warm_whisper = Arc::clone(&app.state::<AppState>().warm_whisper);
            std::thread::spawn(move || {
                let settings = settings.lock().unwrap().clone();
                if let Err(error) = warm_whisper
                    .lock()
                    .unwrap()
                    .ensure_started(&settings.whisper_model_path, &settings.whisper_binary_path)
                {
                    eprintln!("[Local Whisper] Background warm-up skipped: {error}");
                }
            });

            // On a native Windows build this creates the focus-safe capsule in-process.
            // WSL development keeps using the existing host fallback until it is retired.
            let overlay_app = app.handle().clone();
            app.state::<AppState>()
                .overlay
                .initialize(&app.handle(), move || {
                    let state = overlay_app.state::<AppState>();
                    let _ = commands::trigger_toggle_handsfree(overlay_app.clone(), state);
                });

            // System Tray Menu Setup
            let show_i = MenuItem::with_id(app, "show", "Show Settings", true, None::<&str>)?;
            let hide_i = MenuItem::with_id(app, "hide", "Minimize to Tray", true, None::<&str>)?;
            let paste_i = MenuItem::with_id(
                app,
                "paste_last",
                "Paste Last Transcript",
                true,
                None::<&str>,
            )?;
            let copy_i =
                MenuItem::with_id(app, "copy_last", "Copy Last Transcript", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit Assistant", true, None::<&str>)?;
            let menu = Menu::with_id_and_items(
                app,
                "tray_menu",
                &[&show_i, &hide_i, &paste_i, &copy_i, &quit_i],
            )?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "hide" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                    "paste_last" => {
                        let state = app.state::<AppState>();
                        let latest = state.history.lock().unwrap().latest_text();
                        match latest {
                            Some(text) => match commands::deliver_text(&state, &text) {
                                Ok(result) => set_delivery_message(app, &state, result.message),
                                Err(error) => set_delivery_message(
                                    app,
                                    &state,
                                    format!("Could not paste the last transcript: {error}"),
                                ),
                            },
                            None => set_delivery_message(
                                app,
                                &state,
                                "There is no saved dictation to paste yet.".into(),
                            ),
                        }
                    }
                    "copy_last" => {
                        let state = app.state::<AppState>();
                        let latest = state.history.lock().unwrap().latest_text();
                        match latest {
                            Some(text) => match ClipboardInjector::copy(&text) {
                                Ok(result) => set_delivery_message(app, &state, result.message),
                                Err(error) => set_delivery_message(
                                    app,
                                    &state,
                                    format!("Could not copy the last transcript: {error}"),
                                ),
                            },
                            None => set_delivery_message(
                                app,
                                &state,
                                "There is no saved dictation to copy yet.".into(),
                            ),
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            // Register Global Background Hotkey Listener
            let app_handle_down = app.handle().clone();
            let app_handle_up = app.handle().clone();
            let app_handle_error = app.handle().clone();

            HotkeyListener::start_listener(
                move |hands_free| {
                    let state = app_handle_down.state::<AppState>();
                    let mode = if hands_free {
                        "Hands-free"
                    } else {
                        "Hold to talk"
                    };
                    let _ = start_recording_internal(&app_handle_down, &state, mode);
                },
                move || {
                    let state = app_handle_up.state::<AppState>();
                    let _ = stop_recording_internal(&app_handle_up, &state);
                },
                move |message| {
                    let state = app_handle_error.state::<AppState>();
                    set_status(&app_handle_error, &state, "Error", None, Some(message));
                },
            );

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::get_app_status,
            commands::overlay_ready,
            commands::get_audio_diagnostics,
            commands::get_class_sessions,
            commands::get_class_recording_status,
            commands::start_class_recording,
            commands::stop_class_recording,
            commands::get_dictation_history,
            commands::get_insights_summary,
            commands::add_correction_rule,
            commands::copy_to_clipboard,
            commands::copy_last_transcript,
            commands::paste_last_transcript,
            commands::paste_text,
            commands::report_window_action_failure,
            commands::trigger_start_dictation,
            commands::trigger_stop_dictation,
            commands::trigger_cancel_dictation,
            commands::trigger_toggle_handsfree
        ])
        .run(tauri::generate_context!())
        .expect("error while running Smart Dictation Assistant application");
}
