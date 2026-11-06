//! Hams: hold a key, speak, and polished text appears wherever you're typing.

mod audio;
#[cfg(test)]
mod bindings;
mod commands;
mod controller;
mod error;
mod import_v1;
mod input;
mod llm;
mod logging;
mod overlay;
mod paths;
mod platform;
mod settings;
mod state;
mod store;
mod stt;
mod text;
mod ui;
mod updater;

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, OnceLock};

use parking_lot::Mutex;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, RunEvent, WindowEvent};

use controller::{Controller, Deps, Input};
use input::Matcher;
use settings::{SecretCache, SettingsStore};
use state::AppState;
use store::Store;
use stt::Transcriber;

static LOG_GUARD: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();

fn open_store() -> Arc<Store> {
    let path = paths::database_path();
    match Store::open(&path) {
        Ok(store) => Arc::new(store),
        Err(error) => {
            tracing::error!("Could not open {}: {error}. Using a temporary database.", path.display());
            Arc::new(Store::open_in_memory().expect("in-memory database"))
        }
    }
}

#[cfg(feature = "local-stt")]
fn transcriber() -> Arc<dyn Transcriber> {
    Arc::new(stt::WhisperEngine::new())
}

#[cfg(not(feature = "local-stt"))]
fn transcriber() -> Arc<dyn Transcriber> {
    struct Unavailable;
    impl Transcriber for Unavailable {
        fn transcribe(&self, _: &[f32], _: &stt::SttRequest) -> error::AppResult<String> {
            Err(error::AppError::Transcription("this build has no local speech engine".into()))
        }
    }
    Arc::new(Unavailable)
}

/// Launched by "Open at login": start quietly in the menu bar.
const HIDDEN_ARG: &str = "--hidden";

fn show_main(app: &AppHandle) {
    // The Dock icon is only shown while the window is open; otherwise Hams lives in the menu bar.
    #[cfg(target_os = "macos")]
    let _ = app.set_dock_visibility(true);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();
    let store = open_store();
    let settings = Arc::new(SettingsStore::load(Arc::clone(&store)));
    import_v1::run(&store, &settings);
    let current = settings.get();

    overlay::create(&handle);
    let ui = Arc::new(ui::TauriUi::new(handle.clone()));
    let stt = transcriber();
    let secrets = Arc::new(SecretCache::default());
    let recording = Arc::new(AtomicBool::new(false));

    let recorder = Arc::new(audio::AudioRecorder::new());
    let deps = Deps {
        recorder: recorder.clone(),
        stt: Arc::clone(&stt),
        desktop: Arc::new(platform::NativeDesktop),
        polisher: {
            let secrets = Arc::clone(&secrets);
            Arc::new(move |settings| state::make_polisher(&secrets, settings))
        },
        ui: ui.clone(),
        store: Arc::clone(&store),
        settings: Arc::clone(&settings),
        recording: Arc::clone(&recording),
        own_pid: std::process::id() as i64,
    };
    let controller = tauri::async_runtime::block_on(async { Controller::spawn(deps) });

    // Keyboard shortcuts → controller.
    let matcher = Arc::new(Mutex::new(Matcher::new(state::bindings(&current))));
    let (hotkey_tx, mut hotkey_rx) = tokio::sync::mpsc::unbounded_channel();
    let hotkey_error = match platform::start_hotkeys(Arc::clone(&matcher), recording, hotkey_tx) {
        Ok(()) => None,
        Err(error) => {
            tracing::error!("{error}");
            Some(error.to_string())
        }
    };
    {
        let controller = controller.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(action) = hotkey_rx.recv().await {
                controller.send(Input::Hotkey(action));
            }
        });
    }

    let state = AppState {
        controller,
        recorder,
        store,
        settings,
        ui,
        matcher,
        hotkey_error: Mutex::new(hotkey_error),
        stt,
        secrets,
    };
    state.warm_up(&current.model);
    state.apply_microphone(&current);
    app.manage(state);
    app.manage(updater::Updates::default());
    updater::spawn_checks(handle.clone());

    // Menu-bar / tray menu.
    let show = MenuItem::with_id(app, "show", "Open Hams", true, None::<&str>)?;
    let paste = MenuItem::with_id(app, "paste_last", "Paste Last Dictation", true, None::<&str>)?;
    let note = MenuItem::with_id(app, "note", "Start or Finish a Voice Note", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Hams", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &note, &paste, &separator, &quit])?;
    let mut tray = TrayIconBuilder::with_id("hams").menu(&menu).tooltip("Hams");
    #[cfg(target_os = "macos")]
    {
        // Monochrome template image: macOS tints it to match the menu bar.
        let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray/tray-template@2x.png"))?;
        tray = tray.icon(icon).icon_as_template(true);
    }
    #[cfg(not(target_os = "macos"))]
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.on_menu_event(|app, event| match event.id.as_ref() {
        "show" => show_main(app),
        "paste_last" => app.state::<AppState>().controller.send(Input::PasteLast),
        "note" => app.state::<AppState>().controller.send(Input::Note),
        "quit" => app.exit(0),
        _ => {}
    })
    .build(app)?;

    let launched_at_login = std::env::args().any(|a| a == HIDDEN_ARG);
    if !launched_at_login || !current.onboarding_done {
        show_main(&handle);
    } else {
        #[cfg(target_os = "macos")]
        let _ = handle.set_dock_visibility(false);
    }
    Ok(())
}

pub fn run() {
    let _ = LOG_GUARD.set(logging::init());
    tracing::info!("Hams {} starting", env!("CARGO_PKG_VERSION"));

    let app = tauri::Builder::default()
        // Must be first: a second launch just brings the running Hams forward.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![HIDDEN_ARG]),
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(setup)
        .on_window_event(|window, event| {
            // Closing the main window keeps Hams running in the menu bar.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                    #[cfg(target_os = "macos")]
                    let _ = window.app_handle().set_dock_visibility(false);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::set_api_key,
            commands::get_status,
            commands::dictation_toggle,
            commands::dictation_stop,
            commands::note_toggle,
            commands::notes,
            commands::note_rename,
            commands::note_delete,
            commands::dictation_cancel,
            commands::list_microphones,
            commands::list_models,
            commands::download_model,
            commands::delete_model,
            commands::history,
            commands::history_update,
            commands::history_restyle,
            commands::history_pin,
            commands::history_delete,
            commands::history_clear,
            commands::stats,
            commands::dictionary,
            commands::dictionary_upsert,
            commands::dictionary_delete,
            commands::suggestions,
            commands::suggestion_resolve,
            commands::snippets,
            commands::snippet_upsert,
            commands::snippet_delete,
            commands::copy_text,
            commands::permissions,
            commands::request_accessibility,
            commands::open_permission_settings,
            commands::styles,
            commands::recent_apps,
            commands::default_ai_model,
            commands::test_ai,
            commands::autostart_enabled,
            commands::set_autostart,
            commands::app_version,
            commands::update_check,
            commands::update_install,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build Hams");

    app.run(|app, event| {
        if let RunEvent::Reopen { .. } = event {
            show_main(app);
        }
    });
}
