//! The dictation controller: one actor that owns the whole
//! record → transcribe → clean up → polish → paste flow.
//!
//! Hotkeys, the tray, the overlay and the main window only *send inputs* here.
//! Because a single task processes inputs one at a time, overlapping recordings,
//! double stops and state drift between components can't happen.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::audio::Captured;
use crate::error::{AppError, AppResult};
use crate::input::{Binding, HotkeyAction};
use crate::llm::styles::{self, Style};
use crate::llm::{self, Polisher, Task};
use crate::platform::{AppTarget, Delivered, Desktop};
use crate::settings::{DeliveryMode, Settings, SettingsStore};
use crate::store::{self, NewHistory, Store};
use crate::stt::{SttRequest, Transcriber};
use crate::text;

/// A press shorter than this is a tap, not a hold.
const TAP: Duration = Duration::from_millis(220);
/// How long to wait for the second tap of a double-tap.
const DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(350);
/// Live preview: how often the recent audio is re-transcribed, and how much of it.
const PREVIEW_EVERY: Duration = Duration::from_millis(1_000);
const PREVIEW_WINDOW_SECONDS: u32 = 20;

// ---- seams -------------------------------------------------------------------

pub trait Recorder: Send + Sync {
    fn start(&self, device: Option<&str>) -> AppResult<()>;
    fn stop(&self) -> AppResult<Captured>;
    fn cancel(&self);
    fn level(&self) -> f32;
    /// Whisper-ready copy of the most recent audio while still recording (live preview).
    fn snapshot(&self) -> Option<Captured> {
        None
    }
}

impl Recorder for crate::audio::AudioRecorder {
    fn start(&self, device: Option<&str>) -> AppResult<()> {
        crate::audio::AudioRecorder::start(self, device)
    }
    fn stop(&self) -> AppResult<Captured> {
        crate::audio::AudioRecorder::stop(self)
    }
    fn cancel(&self) {
        crate::audio::AudioRecorder::cancel(self)
    }
    fn level(&self) -> f32 {
        crate::audio::AudioRecorder::level(self)
    }
    fn snapshot(&self) -> Option<Captured> {
        Some(crate::audio::AudioRecorder::snapshot(self, PREVIEW_WINDOW_SECONDS))
    }
}

/// Where status updates go (the overlay and main window in the real app).
pub trait Ui: Send + Sync {
    fn status(&self, status: &Status);
    fn level(&self, level: f32);
    /// Live preview text while recording.
    fn partial(&self, _text: &str) {}
    /// History, stats or the dictionary changed on disk.
    fn data_changed(&self) {}
}

pub type PolisherFactory = Arc<dyn Fn(&Settings) -> Arc<dyn Polisher> + Send + Sync>;

#[derive(Clone)]
pub struct Deps {
    pub recorder: Arc<dyn Recorder>,
    pub stt: Arc<dyn Transcriber>,
    pub desktop: Arc<dyn Desktop>,
    pub polisher: PolisherFactory,
    pub ui: Arc<dyn Ui>,
    pub store: Arc<Store>,
    pub settings: Arc<SettingsStore>,
    /// Read by the keyboard hook so Escape is only swallowed while recording.
    pub recording: Arc<AtomicBool>,
    /// Our own process id: dictating into Hams itself only copies.
    pub own_pid: i64,
}

// ---- status sent to the UI -----------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    #[default]
    Idle,
    Recording,
    Transcribing,
    Polishing,
    Delivering,
    Done,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    #[default]
    Dictate,
    Command,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct Status {
    pub phase: Phase,
    pub kind: Kind,
    /// Hands-free (double-tap or mic button) instead of hold-to-talk.
    pub locked: bool,
    /// Unix ms when recording started, for the overlay timer.
    pub started_at: i64,
    pub message: Option<String>,
    pub text: Option<String>,
    pub app: Option<String>,
    /// Live preview is on for this recording, so the overlay makes room for text.
    pub preview: bool,
}

// ---- inputs ----------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Input {
    Hotkey(HotkeyAction),
    /// Mic button in the main window: start hands-free, or finish.
    Toggle,
    /// Finish button in the overlay.
    Stop,
    /// Cancel button in the overlay.
    Cancel,
    PasteLast,
    TapExpired(u64),
    Finished(u64),
}

/// Cheap, cloneable handle used by everything that talks to the controller.
#[derive(Clone)]
pub struct Controller {
    tx: UnboundedSender<Input>,
}

impl Controller {
    /// Must be called from within a Tokio runtime.
    pub fn spawn(deps: Deps) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let actor = Actor { deps, tx: tx.clone(), state: State::Idle, next_session: 1 };
        tokio::spawn(actor.run(rx));
        Self { tx }
    }

    pub fn send(&self, input: Input) {
        let _ = self.tx.send(input);
    }
}

// ---- the actor -------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Hold,
    /// Released quickly; waiting to see if a second tap follows.
    PendingTap,
    Locked,
}

struct Recording {
    session: u64,
    kind: Kind,
    binding: Binding,
    mode: Mode,
    pressed_at: Instant,
    ignore_next_release: bool,
    target: Option<AppTarget>,
    started_at: i64,
    meter: JoinHandle<()>,
    preview: Option<JoinHandle<()>>,
}

impl Recording {
    /// Stop the background tasks tied to this recording.
    fn stop_tasks(&self) {
        self.meter.abort();
        if let Some(preview) = &self.preview {
            preview.abort();
        }
    }
}

enum State {
    Idle,
    Recording(Recording),
    Processing(u64),
}

struct Actor {
    deps: Deps,
    tx: UnboundedSender<Input>,
    state: State,
    next_session: u64,
}

impl Actor {
    async fn run(mut self, mut rx: UnboundedReceiver<Input>) {
        while let Some(input) = rx.recv().await {
            self.handle(input).await;
        }
    }

    async fn handle(&mut self, input: Input) {
        match input {
            Input::Hotkey(HotkeyAction::Pressed(binding @ (Binding::Dictate | Binding::Command))) => {
                self.on_press(binding).await
            }
            Input::Hotkey(HotkeyAction::Released(binding @ (Binding::Dictate | Binding::Command))) => {
                self.on_release(binding)
            }
            Input::Hotkey(HotkeyAction::Pressed(Binding::PasteLast)) | Input::PasteLast => self.paste_last(),
            Input::Hotkey(HotkeyAction::Released(Binding::PasteLast)) => {}
            Input::Hotkey(HotkeyAction::Escape) | Input::Cancel => self.cancel(),
            Input::Hotkey(HotkeyAction::Interrupted(binding)) => {
                if matches!(&self.state, State::Recording(r) if r.binding == binding && r.mode == Mode::Hold) {
                    self.cancel();
                }
            }
            Input::TapExpired(session) => {
                if matches!(&self.state, State::Recording(r) if r.session == session && r.mode == Mode::PendingTap) {
                    self.cancel();
                }
            }
            Input::Toggle => match self.state {
                State::Idle => self.start(Kind::Dictate, Binding::Dictate, Mode::Locked).await,
                State::Recording(_) => self.finish(),
                State::Processing(_) => {}
            },
            Input::Stop => {
                if matches!(self.state, State::Recording(_)) {
                    self.finish();
                }
            }
            Input::Finished(session) => {
                if matches!(self.state, State::Processing(s) if s == session) {
                    self.state = State::Idle;
                }
            }
        }
    }

    async fn on_press(&mut self, binding: Binding) {
        let kind = if binding == Binding::Command { Kind::Command } else { Kind::Dictate };
        let finish = match &mut self.state {
            State::Idle => {
                self.start(kind, binding, Mode::Hold).await;
                false
            }
            State::Recording(rec) if rec.binding == binding => match rec.mode {
                Mode::PendingTap => {
                    // Second tap: keep recording hands-free.
                    rec.mode = Mode::Locked;
                    rec.ignore_next_release = true;
                    self.deps.ui.status(&recording_status(rec));
                    false
                }
                Mode::Locked => true,
                Mode::Hold => false,
            },
            State::Recording(_) => false,
            State::Processing(_) => {
                tracing::debug!("Ignoring shortcut: still processing the previous dictation");
                false
            }
        };
        if finish {
            self.finish();
        }
    }

    fn on_release(&mut self, binding: Binding) {
        let double_tap = self.deps.settings.get().double_tap_lock;
        let State::Recording(rec) = &mut self.state else { return };
        if rec.binding != binding {
            return;
        }
        if std::mem::take(&mut rec.ignore_next_release) {
            return;
        }
        if rec.mode != Mode::Hold {
            return;
        }
        if rec.pressed_at.elapsed() >= TAP {
            self.finish();
        } else if double_tap {
            rec.mode = Mode::PendingTap;
            let (tx, session) = (self.tx.clone(), rec.session);
            tokio::spawn(async move {
                tokio::time::sleep(DOUBLE_TAP_WINDOW).await;
                let _ = tx.send(Input::TapExpired(session));
            });
        } else {
            self.cancel();
        }
    }

    async fn start(&mut self, kind: Kind, binding: Binding, mode: Mode) {
        let settings = self.deps.settings.get();
        let desktop = Arc::clone(&self.deps.desktop);
        let recorder = Arc::clone(&self.deps.recorder);
        let device = settings.microphone.clone();
        let pressed_at = Instant::now();

        let (target, started) = tokio::task::spawn_blocking(move || {
            let target = desktop.frontmost();
            (target, recorder.start(device.as_deref()))
        })
        .await
        .unwrap_or_else(|e| (None, Err(AppError::Audio(e.to_string()))));

        if let Err(error) = started {
            self.deps.ui.status(&Status { phase: Phase::Error, kind, message: Some(error.to_string()), ..Status::default() });
            self.deps.ui.status(&Status::default());
            return;
        }

        self.deps.recording.store(true, Ordering::Release);
        let meter = {
            let (recorder, ui) = (Arc::clone(&self.deps.recorder), Arc::clone(&self.deps.ui));
            tokio::spawn(async move {
                let mut tick = tokio::time::interval(Duration::from_millis(50));
                loop {
                    tick.tick().await;
                    ui.level(recorder.level());
                }
            })
        };
        let preview = (settings.live_preview && kind == Kind::Dictate).then(|| spawn_preview(&self.deps, &settings));
        let session = self.next_session;
        self.next_session += 1;
        let rec = Recording {
            session,
            kind,
            binding,
            mode,
            pressed_at,
            ignore_next_release: false,
            target,
            started_at: store::now_ms(),
            meter,
            preview,
        };
        self.deps.ui.status(&recording_status(&rec));
        self.state = State::Recording(rec);
    }

    fn cancel(&mut self) {
        let State::Recording(rec) = std::mem::replace(&mut self.state, State::Idle) else { return };
        rec.stop_tasks();
        self.deps.recording.store(false, Ordering::Release);
        let recorder = Arc::clone(&self.deps.recorder);
        tokio::task::spawn_blocking(move || recorder.cancel());
        self.deps.ui.status(&Status::default());
    }

    fn finish(&mut self) {
        let State::Recording(rec) = std::mem::replace(&mut self.state, State::Idle) else { return };
        rec.stop_tasks();
        self.deps.recording.store(false, Ordering::Release);
        self.state = State::Processing(rec.session);
        let (deps, tx, session) = (self.deps.clone(), self.tx.clone(), rec.session);
        let job = Job { kind: rec.kind, locked: rec.mode == Mode::Locked, target: rec.target, started_at: rec.started_at };
        tokio::spawn(async move {
            process(&deps, job).await;
            let _ = tx.send(Input::Finished(session));
        });
    }

    fn paste_last(&self) {
        if !matches!(self.state, State::Idle) {
            return;
        }
        let deps = self.deps.clone();
        tokio::task::spawn_blocking(move || {
            let Ok(Some(text)) = deps.store.latest_text() else { return };
            let target = deps.desktop.frontmost();
            if let Err(error) = deps.desktop.deliver(&text, target.as_ref(), deps.settings.get().delivery) {
                deps.ui.status(&Status { phase: Phase::Error, message: Some(error.to_string()), ..Status::default() });
            }
        });
    }
}

fn recording_status(rec: &Recording) -> Status {
    Status {
        phase: Phase::Recording,
        kind: rec.kind,
        locked: rec.mode == Mode::Locked,
        started_at: rec.started_at,
        app: rec.target.as_ref().map(|t| t.name.clone()),
        preview: rec.preview.is_some(),
        ..Status::default()
    }
}

/// Re-transcribe the last few seconds about once a second and show it in the
/// overlay. It's a preview only: the pasted text always comes from one final
/// pass over the whole recording, so nothing here can affect the result.
fn spawn_preview(deps: &Deps, settings: &Settings) -> JoinHandle<()> {
    let (recorder, stt, ui) = (Arc::clone(&deps.recorder), Arc::clone(&deps.stt), Arc::clone(&deps.ui));
    let dictionary = deps.store.dictionary().unwrap_or_default();
    let request = SttRequest {
        language: settings.language.clone(),
        model: settings.model.clone(),
        vocabulary: text::vocabulary(&dictionary),
    };
    let options = text::Options { voice_commands: settings.voice_commands, remove_fillers: settings.remove_fillers };
    tokio::spawn(async move {
        let mut shown = String::new();
        loop {
            tokio::time::sleep(PREVIEW_EVERY).await;
            let (recorder, stt, request) = (Arc::clone(&recorder), Arc::clone(&stt), request.clone());
            let heard = tokio::task::spawn_blocking(move || {
                let captured = recorder.snapshot().filter(|c| c.has_speech)?;
                stt.transcribe(&captured.samples, &request).ok()
            })
            .await
            .ok()
            .flatten();
            if let Some(heard) = heard {
                let text = text::clean(&heard, &options);
                if text != shown {
                    ui.partial(&text);
                    shown = text;
                }
            }
        }
    })
}

// ---- the processing pipeline ------------------------------------------------------

struct Job {
    kind: Kind,
    locked: bool,
    target: Option<AppTarget>,
    started_at: i64,
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> AppResult<T> + Send + 'static) -> AppResult<T> {
    tokio::task::spawn_blocking(f).await.map_err(|e| AppError::Other(e.to_string()))?
}

async fn process(deps: &Deps, job: Job) {
    let released = Instant::now();
    let status = |phase: Phase| Status {
        phase,
        kind: job.kind,
        locked: job.locked,
        started_at: job.started_at,
        app: job.target.as_ref().map(|t| t.name.clone()),
        ..Status::default()
    };
    let fail = |error: AppError| {
        tracing::warn!("Dictation failed: {error}");
        deps.ui.status(&Status { message: Some(error.to_string()), ..status(Phase::Error) });
    };

    deps.ui.status(&status(Phase::Transcribing));
    let recorder = Arc::clone(&deps.recorder);
    let captured = match blocking(move || recorder.stop()).await {
        Ok(captured) => captured,
        Err(error) => return fail(error),
    };
    if !captured.has_speech {
        tracing::debug!("No speech detected; nothing to do");
        return deps.ui.status(&Status::default());
    }

    let settings = deps.settings.get();
    let dictionary = deps.store.dictionary().unwrap_or_default();
    let request = SttRequest {
        language: settings.language.clone(),
        model: settings.model.clone(),
        vocabulary: text::vocabulary(&dictionary),
    };
    let stt = Arc::clone(&deps.stt);
    let samples = captured.samples;
    let raw = match blocking(move || stt.transcribe(&samples, &request)).await {
        Ok(raw) => raw,
        Err(error) => return fail(error),
    };
    let transcribed_in = released.elapsed();
    let options = text::Options { voice_commands: settings.voice_commands, remove_fillers: settings.remove_fillers };
    let cleaned = text::clean(&raw, &options);
    if cleaned.is_empty() {
        return deps.ui.status(&Status::default());
    }

    let resolved = styles::resolve(
        job.target.as_ref().map(|t| (t.id.as_str(), t.name.as_str())),
        settings.app_styles,
        &settings.app_rules,
        &settings.style_overrides,
    );
    let style = resolved.style;
    let vocabulary = text::vocabulary(&dictionary);
    let polisher = (deps.polisher)(&settings);

    let (final_text, notice) = match job.kind {
        Kind::Dictate => {
            let snippets = deps.store.snippets().unwrap_or_default();
            if let Some(expansion) = text::expand_snippet(&cleaned, &snippets) {
                (expansion, None)
            } else {
                let long_enough = store::word_count(&cleaned) >= settings.ai.smart_skip_words as i64;
                let (polished, notice) = if polisher.enabled() && resolved.polish && long_enough {
                    deps.ui.status(&status(Phase::Polishing));
                    let request = llm::Request {
                        task: Task::Polish { transcript: cleaned.clone() },
                        style_prompt: resolved.prompt.clone(),
                        vocabulary,
                    };
                    match polisher.run(request).await {
                        Ok(text) => (text, None),
                        Err(error) => (cleaned.clone(), Some(format!("{error}. Pasted the plain transcript."))),
                    }
                } else {
                    (cleaned.clone(), None)
                };
                (style.finish(&text::apply_replacements(&polished, &dictionary)), notice)
            }
        }
        Kind::Command => {
            if !polisher.enabled() {
                return fail(AppError::Polish(
                    "command mode needs an AI provider. Choose one in Settings → AI.".into(),
                ));
            }
            deps.ui.status(&status(Phase::Polishing));
            let desktop = Arc::clone(&deps.desktop);
            let target = job.target.clone();
            let allow_copy = style != Style::Code;
            let selection = blocking(move || Ok(desktop.selected_text(target.as_ref(), allow_copy)))
                .await
                .unwrap_or(None);
            let request = llm::Request {
                task: Task::Edit { instruction: cleaned.clone(), selection },
                style_prompt: resolved.prompt.clone(),
                vocabulary,
            };
            match polisher.run(request).await {
                Ok(text) => (text::apply_replacements(&text, &dictionary), None),
                Err(error) => return fail(error),
            }
        }
    };

    deps.ui.status(&status(Phase::Delivering));
    let into_self = job.target.as_ref().is_some_and(|t| t.pid == deps.own_pid);
    let mode = if into_self { DeliveryMode::CopyOnly } else { settings.delivery };
    let (desktop, target, text_to_send) = (Arc::clone(&deps.desktop), job.target.clone(), final_text.clone());
    let delivered = blocking(move || desktop.deliver(&text_to_send, target.as_ref(), mode)).await;
    tracing::info!(
        "Dictation delivered {} ms after release (transcription {} ms, {} words)",
        released.elapsed().as_millis(),
        transcribed_in.as_millis(),
        store::word_count(&final_text)
    );

    let delivery = match &delivered {
        Ok(Delivered::Pasted) => "pasted",
        Ok(Delivered::Copied) => "copied",
        Err(_) => "failed",
    };
    match delivered {
        Ok(_) => deps.ui.status(&Status { text: Some(final_text.clone()), message: notice, ..status(Phase::Done) }),
        Err(error) => deps.ui.status(&Status {
            text: Some(final_text.clone()),
            message: Some(format!("{error} It's saved in History.")),
            ..status(Phase::Error)
        }),
    }

    // Persist after delivery so disk I/O never delays the paste.
    let store = Arc::clone(&deps.store);
    let app = job.target.map(|t| t.name).unwrap_or_default();
    let duration_ms = captured.duration_ms as i64;
    let style_id = style.id();
    let _ = blocking(move || {
        store.add_history(&NewHistory {
            text: &final_text,
            raw: &raw,
            app: &app,
            style: style_id,
            delivery,
            duration_ms,
        })
    })
    .await
    .map_err(|e| tracing::error!("Could not save history: {e}"));
    deps.ui.data_changed();
}

// ---- tests -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::future::BoxFuture;
    use parking_lot::Mutex;

    #[derive(Default)]
    struct FakeRecorder {
        starts: Mutex<u32>,
        cancels: Mutex<u32>,
        speech: Mutex<bool>,
    }
    impl Recorder for FakeRecorder {
        fn start(&self, _: Option<&str>) -> AppResult<()> {
            *self.starts.lock() += 1;
            Ok(())
        }
        fn stop(&self) -> AppResult<Captured> {
            Ok(Captured { samples: vec![0.0; 16_000], duration_ms: 1_500, has_speech: *self.speech.lock() })
        }
        fn cancel(&self) {
            *self.cancels.lock() += 1;
        }
        fn level(&self) -> f32 {
            0.5
        }
        fn snapshot(&self) -> Option<Captured> {
            Some(Captured { samples: vec![0.0; 16_000], duration_ms: 1_000, has_speech: *self.speech.lock() })
        }
    }

    struct FakeStt(String);
    impl Transcriber for FakeStt {
        fn transcribe(&self, _: &[f32], _: &SttRequest) -> AppResult<String> {
            Ok(self.0.clone())
        }
    }

    #[derive(Default)]
    struct FakeDesktop {
        delivered: Mutex<Vec<(String, DeliveryMode)>>,
    }
    impl Desktop for FakeDesktop {
        fn frontmost(&self) -> Option<AppTarget> {
            Some(AppTarget { pid: 42, id: "com.apple.TextEdit".into(), name: "TextEdit".into(), window: 0 })
        }
        fn deliver(&self, text: &str, _: Option<&AppTarget>, mode: DeliveryMode) -> AppResult<Delivered> {
            self.delivered.lock().push((text.to_string(), mode));
            Ok(Delivered::Pasted)
        }
        fn copy(&self, _: &str) -> AppResult<()> {
            Ok(())
        }
        fn selected_text(&self, _: Option<&AppTarget>, _: bool) -> Option<String> {
            Some("hello there".into())
        }
    }

    #[derive(Default)]
    struct FakeUi {
        statuses: Mutex<Vec<Status>>,
        partials: Mutex<Vec<String>>,
    }
    impl Ui for FakeUi {
        fn status(&self, status: &Status) {
            self.statuses.lock().push(status.clone());
        }
        fn level(&self, _: f32) {}
        fn partial(&self, text: &str) {
            self.partials.lock().push(text.to_string());
        }
    }

    struct FakePolisher(Option<AppResult<String>>);
    impl Polisher for FakePolisher {
        fn enabled(&self) -> bool {
            self.0.is_some()
        }
        fn run(&self, request: llm::Request) -> BoxFuture<'static, AppResult<String>> {
            let result = match (&self.0, request.task) {
                (Some(Ok(text)), Task::Edit { selection, .. }) => Ok(format!("{text}|{}", selection.unwrap_or_default())),
                (Some(Ok(text)), _) => Ok(text.clone()),
                (Some(Err(_)), _) => Err(AppError::Polish("offline".into())),
                (None, _) => Err(AppError::Polish("off".into())),
            };
            Box::pin(async move { result })
        }
    }

    struct Harness {
        controller: Controller,
        recorder: Arc<FakeRecorder>,
        desktop: Arc<FakeDesktop>,
        ui: Arc<FakeUi>,
        store: Arc<Store>,
        settings: Arc<SettingsStore>,
    }

    fn harness(transcript: &str, polish: Option<AppResult<String>>) -> Harness {
        let recorder = Arc::new(FakeRecorder { speech: Mutex::new(true), ..Default::default() });
        let desktop = Arc::new(FakeDesktop::default());
        let ui = Arc::new(FakeUi::default());
        let store = Arc::new(Store::open_in_memory().unwrap());
        let settings = Arc::new(SettingsStore::load(Arc::clone(&store)));
        let settings_handle = Arc::clone(&settings);
        let polisher: Arc<dyn Polisher> = Arc::new(FakePolisher(polish));
        let controller = Controller::spawn(Deps {
            recorder: recorder.clone(),
            stt: Arc::new(FakeStt(transcript.into())),
            desktop: desktop.clone(),
            polisher: Arc::new(move |_| Arc::clone(&polisher)),
            ui: ui.clone(),
            store: store.clone(),
            settings: settings.clone(),
            recording: Arc::new(AtomicBool::new(false)),
            own_pid: 1,
        });
        Harness { controller, recorder, desktop, ui, store, settings: settings_handle }
    }

    impl Harness {
        fn send(&self, input: Input) {
            self.controller.send(input);
        }
        fn press(&self) {
            self.send(Input::Hotkey(HotkeyAction::Pressed(Binding::Dictate)));
        }
        fn release(&self) {
            self.send(Input::Hotkey(HotkeyAction::Released(Binding::Dictate)));
        }
        fn delivered(&self) -> Vec<String> {
            self.desktop.delivered.lock().iter().map(|(t, _)| t.clone()).collect()
        }
        fn phases(&self) -> Vec<Phase> {
            self.ui.statuses.lock().iter().map(|s| s.phase).collect()
        }
    }

    async fn settle() {
        for _ in 0..50 {
            tokio::task::yield_now().await;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    #[tokio::test(start_paused = true)]
    async fn hold_to_talk_delivers_and_saves_history() {
        let h = harness("Um, hello world.", None);
        h.press();
        tokio::time::sleep(Duration::from_millis(400)).await;
        h.release();
        settle().await;
        assert_eq!(h.delivered(), vec!["Hello world.".to_string()]);
        assert_eq!(h.phases().last(), Some(&Phase::Done));
        assert_eq!(h.store.history("", 10).unwrap()[0].text, "Hello world.");
        assert_eq!(h.store.history("", 10).unwrap()[0].delivery, "pasted");
    }

    #[tokio::test(start_paused = true)]
    async fn live_preview_shows_text_while_recording_only() {
        let h = harness("um live words", None);
        h.settings.save(Settings { live_preview: true, ..Settings::default() }).unwrap();
        h.press();
        tokio::time::sleep(Duration::from_millis(2_500)).await;
        assert_eq!(*h.ui.partials.lock(), vec!["Live words".to_string()], "unchanged text is sent once");
        assert!(h.ui.statuses.lock().iter().any(|s| s.phase == Phase::Recording && s.preview));
        h.release();
        settle().await;
        let after_release = h.ui.partials.lock().len();
        tokio::time::sleep(Duration::from_secs(3)).await;
        assert_eq!(h.ui.partials.lock().len(), after_release, "preview stops when recording ends");
        assert_eq!(h.delivered(), vec!["Live words".to_string()]);
    }

    #[tokio::test(start_paused = true)]
    async fn app_rule_and_style_override_shape_the_polish_request() {
        use crate::llm::styles::{AppRule, StyleOverride};
        let h = harness("please send the report today", Some(Ok("Polished.".into())));
        let mut settings = Settings::default();
        settings.ai.provider = crate::settings::AiProvider::Gemini;
        settings.app_rules = vec![AppRule { app: "textedit".into(), style: Style::Code }];
        h.settings.save(settings).unwrap();
        h.press();
        tokio::time::sleep(Duration::from_millis(400)).await;
        h.release();
        settle().await;
        // Code style skips AI by default, so the cleaned transcript is pasted as-is.
        assert_eq!(h.delivered(), vec!["Please send the report today".to_string()]);
        assert_eq!(h.store.history("", 1).unwrap()[0].style, "code");

        let mut settings = h.settings.get();
        settings.style_overrides.insert(Style::Code, StyleOverride { prompt: None, polish: Some(true) });
        h.settings.save(settings).unwrap();
        h.press();
        tokio::time::sleep(Duration::from_millis(400)).await;
        h.release();
        settle().await;
        assert_eq!(h.delivered().last().map(String::as_str), Some("Polished."));
    }

    #[tokio::test(start_paused = true)]
    async fn single_quick_tap_is_ignored() {
        let h = harness("hello", None);
        h.press();
        tokio::time::sleep(Duration::from_millis(50)).await;
        h.release();
        settle().await;
        assert!(h.delivered().is_empty());
        assert_eq!(*h.recorder.cancels.lock(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn double_tap_locks_until_next_press() {
        let h = harness("hands free text", None);
        h.press();
        tokio::time::sleep(Duration::from_millis(50)).await;
        h.release();
        tokio::time::sleep(Duration::from_millis(100)).await;
        h.press();
        tokio::time::sleep(Duration::from_millis(50)).await;
        h.release();
        tokio::time::sleep(Duration::from_secs(3)).await;
        assert!(h.delivered().is_empty(), "still recording hands-free");
        assert!(h.ui.statuses.lock().iter().any(|s| s.locked));
        h.press();
        settle().await;
        h.release();
        settle().await;
        assert_eq!(h.delivered(), vec!["Hands free text".to_string()]);
        assert_eq!(*h.recorder.starts.lock(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn escape_cancels_without_pasting() {
        let h = harness("secret", None);
        h.press();
        tokio::time::sleep(Duration::from_millis(400)).await;
        h.send(Input::Hotkey(HotkeyAction::Escape));
        h.release();
        settle().await;
        assert!(h.delivered().is_empty());
        assert_eq!(h.phases().last(), Some(&Phase::Idle));
    }

    #[tokio::test(start_paused = true)]
    async fn no_speech_means_no_paste() {
        let h = harness("ghost", None);
        *h.recorder.speech.lock() = false;
        h.press();
        tokio::time::sleep(Duration::from_millis(400)).await;
        h.release();
        settle().await;
        assert!(h.delivered().is_empty());
        assert!(h.store.history("", 10).unwrap().is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn ai_polish_is_used_and_failures_fall_back() {
        let h = harness("this is a longer dictation", Some(Ok("Polished text.".into())));
        h.press();
        tokio::time::sleep(Duration::from_millis(400)).await;
        h.release();
        settle().await;
        assert_eq!(h.delivered(), vec!["Polished text.".to_string()]);
        assert!(h.phases().contains(&Phase::Polishing));

        let h = harness("this is a longer dictation", Some(Err(AppError::Polish("x".into()))));
        h.press();
        tokio::time::sleep(Duration::from_millis(400)).await;
        h.release();
        settle().await;
        assert_eq!(h.delivered(), vec!["This is a longer dictation".to_string()]);
        let last = h.ui.statuses.lock().last().cloned().unwrap();
        assert!(last.message.unwrap().contains("plain transcript"));
    }

    #[tokio::test(start_paused = true)]
    async fn short_dictations_skip_ai() {
        let h = harness("Yes.", Some(Ok("SHOULD NOT APPEAR".into())));
        h.press();
        tokio::time::sleep(Duration::from_millis(400)).await;
        h.release();
        settle().await;
        assert_eq!(h.delivered(), vec!["Yes.".to_string()]);
    }

    #[tokio::test(start_paused = true)]
    async fn snippets_expand_without_ai() {
        let h = harness("My email.", Some(Ok("SHOULD NOT APPEAR".into())));
        h.store.upsert_snippet("my email", "me@example.com").unwrap();
        h.press();
        tokio::time::sleep(Duration::from_millis(400)).await;
        h.release();
        settle().await;
        assert_eq!(h.delivered(), vec!["me@example.com".to_string()]);
    }

    #[tokio::test(start_paused = true)]
    async fn presses_while_processing_are_ignored() {
        let h = harness("first", None);
        h.press();
        tokio::time::sleep(Duration::from_millis(400)).await;
        h.release();
        h.press(); // arrives while the first dictation is processing
        h.release();
        settle().await;
        assert_eq!(*h.recorder.starts.lock(), 1);
        assert_eq!(h.delivered().len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn command_mode_edits_the_selection() {
        let h = harness("make it formal", Some(Ok("EDITED".into())));
        h.send(Input::Hotkey(HotkeyAction::Pressed(Binding::Command)));
        tokio::time::sleep(Duration::from_millis(400)).await;
        h.send(Input::Hotkey(HotkeyAction::Released(Binding::Command)));
        settle().await;
        assert_eq!(h.delivered(), vec!["EDITED|hello there".to_string()]);
    }

    #[tokio::test(start_paused = true)]
    async fn command_mode_without_ai_explains_why() {
        let h = harness("make it formal", None);
        h.send(Input::Hotkey(HotkeyAction::Pressed(Binding::Command)));
        tokio::time::sleep(Duration::from_millis(400)).await;
        h.send(Input::Hotkey(HotkeyAction::Released(Binding::Command)));
        settle().await;
        assert!(h.delivered().is_empty());
        let last = h.ui.statuses.lock().last().cloned().unwrap();
        assert_eq!(last.phase, Phase::Error);
        assert!(last.message.unwrap().contains("AI provider"));
    }

    #[tokio::test(start_paused = true)]
    async fn dictionary_replacements_apply_after_polish() {
        let h = harness("ship it on tauri today please", Some(Ok("Ship it on tauri today, please.".into())));
        h.store.upsert_dictionary("tauri", Some("Tauri"), "user").unwrap();
        h.press();
        tokio::time::sleep(Duration::from_millis(400)).await;
        h.release();
        settle().await;
        assert_eq!(h.delivered(), vec!["Ship it on Tauri today, please.".to_string()]);
    }
}
