use std::sync::Arc;

use parking_lot::Mutex;

use crate::controller::Controller;
use crate::input::{Binding, Matcher, Shortcut};
use crate::llm::{LlmClient, Polisher};
use crate::settings::{SecretCache, Settings, SettingsStore};
use crate::store::Store;
use crate::stt::Transcriber;
use crate::ui::TauriUi;

/// Everything the IPC commands need, managed by Tauri.
pub struct AppState {
    pub controller: Controller,
    pub store: Arc<Store>,
    pub settings: Arc<SettingsStore>,
    pub ui: Arc<TauriUi>,
    pub matcher: Arc<Mutex<Matcher>>,
    pub hotkey_error: Mutex<Option<String>>,
    pub stt: Arc<dyn Transcriber>,
    pub secrets: Arc<SecretCache>,
}

impl AppState {
    pub fn apply_shortcuts(&self, settings: &Settings) {
        self.matcher.lock().set_bindings(bindings(settings));
    }

    pub fn warm_up(&self, model: &str) {
        let (stt, model) = (Arc::clone(&self.stt), model.to_string());
        std::thread::spawn(move || stt.warm_up(&model));
    }

    pub fn polisher_for(&self, settings: &Settings) -> Arc<dyn Polisher> {
        make_polisher(&self.secrets, settings)
    }
}

pub fn make_polisher(secrets: &SecretCache, settings: &Settings) -> Arc<dyn Polisher> {
    let key = if settings.ai.provider.needs_key() { secrets.get(settings.ai.provider) } else { None };
    Arc::new(Arc::new(LlmClient::new(settings.ai.clone(), key)))
}

pub fn bindings(settings: &Settings) -> Vec<(Binding, Shortcut)> {
    [
        (Binding::Dictate, &settings.dictate_shortcut),
        (Binding::Command, &settings.command_shortcut),
        (Binding::PasteLast, &settings.paste_last_shortcut),
    ]
    .into_iter()
    .filter_map(|(binding, text)| {
        let shortcut = Shortcut::parse(text).ok().filter(Shortcut::supported_here)?;
        Some((binding, shortcut))
    })
    .collect()
}
