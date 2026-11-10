//! The engine driven the way a phone keyboard drives it: a fake [`Host`] holds
//! a text field and records what the engine asks of it.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use hams_core::stt::SttRequest;

use super::*;

/// A text field with a cursor at the end.
struct FakeHost {
    field: Mutex<FieldInfo>,
    text: Mutex<String>,
    clipboard: Mutex<Option<String>>,
    statuses: Mutex<Vec<DictationStatus>>,
    mic_allowed: AtomicBool,
    mic_open: AtomicBool,
    /// The engine, so mic_start can feed it speech like a real microphone.
    engine: Mutex<Option<std::sync::Weak<HamsEngine>>>,
}

impl FakeHost {
    fn new(kind: FieldKind) -> Arc<Self> {
        Arc::new(Self {
            field: Mutex::new(FieldInfo { token: 1, package: "com.whatsapp".into(), app_name: "WhatsApp".into(), kind }),
            text: Mutex::new(String::new()),
            clipboard: Mutex::new(None),
            statuses: Mutex::new(Vec::new()),
            mic_allowed: AtomicBool::new(true),
            mic_open: AtomicBool::new(false),
            engine: Mutex::new(None),
        })
    }

    fn last_phase(&self) -> Option<Phase> {
        self.statuses.lock().last().map(|s| s.phase)
    }

    fn wait_for(&self, phase: Phase) -> DictationStatus {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.statuses.lock().iter().rev().find(|s| s.phase == phase) {
                return status.clone();
            }
            assert!(Instant::now() < deadline, "never reached {phase:?}; saw {:?}", self.statuses.lock());
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Host for FakeHost {
    fn mic_start(&self) -> bool {
        if !self.mic_allowed.load(Ordering::SeqCst) {
            return false;
        }
        self.mic_open.store(true, Ordering::SeqCst);
        // Like AudioRecord: frames start arriving shortly after the mic opens.
        if let Some(engine) = self.engine.lock().as_ref().and_then(|w| w.upgrade()) {
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(20));
                let speech: Vec<f32> = (0..32_000).map(|i| (i as f32 * 0.07).sin() * 0.4).collect();
                engine.push_audio(speech, 16_000);
            });
        }
        true
    }
    fn mic_stop(&self) {
        self.mic_open.store(false, Ordering::SeqCst);
    }
    fn field(&self) -> FieldInfo {
        self.field.lock().clone()
    }
    fn text_before_cursor(&self, max_utf16: u32) -> String {
        let text = self.text.lock();
        let units: Vec<u16> = text.encode_utf16().collect();
        let start = units.len().saturating_sub(max_utf16 as usize);
        String::from_utf16_lossy(&units[start..])
    }
    fn selected_text(&self) -> String {
        String::new()
    }
    fn field_text(&self) -> String {
        self.text.lock().clone()
    }
    fn commit(&self, token: u64, text: String) -> bool {
        if token != self.field.lock().token {
            return false;
        }
        self.text.lock().push_str(&text);
        true
    }
    fn delete_before_cursor(&self, token: u64, count: u32) -> bool {
        if token != self.field.lock().token {
            return false;
        }
        let mut text = self.text.lock();
        let units: Vec<u16> = text.encode_utf16().collect();
        let keep = units.len().saturating_sub(count as usize);
        *text = String::from_utf16_lossy(&units[..keep]);
        true
    }
    fn copy(&self, text: String) {
        *self.clipboard.lock() = Some(text);
    }
    fn on_status(&self, status: DictationStatus) {
        self.statuses.lock().push(status);
    }
    fn on_level(&self, _level: f32) {}
    fn on_partial(&self, _text: String) {}
    fn on_data_changed(&self) {}
    fn log(&self, _level: u8, _message: String) {}
}

#[derive(Default)]
struct FakeVault(Mutex<std::collections::HashMap<String, String>>);

impl SecretVault for FakeVault {
    fn get(&self, account: String) -> Option<String> {
        self.0.lock().get(&account).cloned()
    }
    fn set(&self, account: String, value: String) -> bool {
        if value.is_empty() {
            self.0.lock().remove(&account);
        } else {
            self.0.lock().insert(account, value);
        }
        true
    }
}

/// Always "hears" the same words.
struct FakeWhisper(&'static str);

impl Transcriber for FakeWhisper {
    fn transcribe(&self, _: &[f32], _: &SttRequest) -> AppResult<String> {
        Ok(self.0.to_string())
    }
}

fn temp_dir() -> String {
    static N: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "hams-mobile-test-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir.to_string_lossy().into_owned()
}

fn engine(host: &Arc<FakeHost>, heard: &'static str) -> Arc<HamsEngine> {
    let engine = Arc::new(
        HamsEngine::build(&temp_dir(), host.clone(), Arc::new(FakeVault::default()), Arc::new(FakeWhisper(heard)))
            .unwrap(),
    );
    *host.engine.lock() = Some(Arc::downgrade(&engine));
    engine
}

/// Hold the pad long enough to be a hold (not a tap), then let go.
fn hold_and_speak(engine: &HamsEngine, host: &FakeHost) {
    engine.press(Pad::Dictate);
    host.wait_for(Phase::Recording);
    std::thread::sleep(Duration::from_millis(400));
    engine.release(Pad::Dictate);
}

#[test]
fn hold_to_talk_inserts_cleaned_text_into_the_field() {
    let host = FakeHost::new(FieldKind::Text);
    let engine = engine(&host, "um so send it to rahul tomorrow");
    hold_and_speak(&engine, &host);
    let done = host.wait_for(Phase::Done);
    let written = done.text.unwrap();
    assert_eq!(*host.text.lock(), written);
    assert!(written.starts_with("So send it to rahul tomorrow") || written.starts_with("Send it"), "{written}");
    assert!(!host.mic_open.load(Ordering::SeqCst), "the mic stays open only while recording");
    assert!(host.clipboard.lock().is_none());

    // History is written right after delivery.
    let deadline = Instant::now() + Duration::from_secs(2);
    let last = loop {
        if let Some(last) = engine.last_dictation() {
            break last;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(last.raw, "um so send it to rahul tomorrow");
    assert_eq!(last.text, written);
    assert_eq!(last.app, "WhatsApp");
}

#[test]
fn a_second_dictation_gets_a_space_and_undo_removes_only_it() {
    let host = FakeHost::new(FieldKind::Text);
    *host.text.lock() = "Hello.".into();
    let engine = engine(&host, "see you soon");
    hold_and_speak(&engine, &host);
    host.wait_for(Phase::Done);
    // WhatsApp gets the Messaging style: no closing period.
    assert_eq!(*host.text.lock(), "Hello. See you soon");
    assert!(engine.can_undo());
    assert!(engine.undo_last());
    assert_eq!(*host.text.lock(), "Hello.");
    assert!(!engine.undo_last(), "undo applies once");
}

#[test]
fn undo_does_nothing_once_the_user_typed_after_it() {
    let host = FakeHost::new(FieldKind::Text);
    let engine = engine(&host, "see you soon");
    hold_and_speak(&engine, &host);
    host.wait_for(Phase::Done);
    host.text.lock().push_str(" Bye");
    assert!(!engine.undo_last());
    assert!(host.text.lock().ends_with("Bye"));
}

#[test]
fn switching_fields_mid_dictation_copies_instead_of_typing() {
    let host = FakeHost::new(FieldKind::Text);
    let engine = engine(&host, "this was for the other chat");
    engine.press(Pad::Dictate);
    host.wait_for(Phase::Recording);
    std::thread::sleep(Duration::from_millis(400));
    host.field.lock().token = 2; // the user tapped into another field
    engine.release(Pad::Dictate);
    host.wait_for(Phase::Done);
    assert!(host.text.lock().is_empty(), "nothing typed into the new field");
    assert!(host.clipboard.lock().as_deref().unwrap().contains("other chat"));
}

#[test]
fn password_fields_are_refused_before_recording() {
    let host = FakeHost::new(FieldKind::Password);
    let engine = engine(&host, "secret");
    engine.press(Pad::Dictate);
    std::thread::sleep(Duration::from_millis(100));
    let status = host.statuses.lock().last().cloned().unwrap();
    assert_eq!(status.phase, Phase::Error);
    assert!(status.message.unwrap().contains("password"));
    assert!(!host.mic_open.load(Ordering::SeqCst));
    engine.release(Pad::Dictate);
}

#[test]
fn email_fields_get_a_literal_address() {
    let host = FakeHost::new(FieldKind::Email);
    let engine = engine(&host, "rahul at gmail dot com");
    hold_and_speak(&engine, &host);
    host.wait_for(Phase::Done);
    assert_eq!(*host.text.lock(), "rahul@gmail.com");
}

#[test]
fn no_microphone_permission_is_reported() {
    let host = FakeHost::new(FieldKind::Text);
    host.mic_allowed.store(false, Ordering::SeqCst);
    let engine = engine(&host, "hello");
    engine.press(Pad::Dictate);
    let error = host.wait_for(Phase::Error);
    assert!(error.message.unwrap().contains("microphone"));
    engine.release(Pad::Dictate);
}

#[test]
fn double_tap_records_hands_free_until_toggled() {
    let host = FakeHost::new(FieldKind::Text);
    let engine = engine(&host, "hands free works");
    engine.press(Pad::Dictate);
    engine.release(Pad::Dictate);
    std::thread::sleep(Duration::from_millis(60));
    engine.press(Pad::Dictate);
    engine.release(Pad::Dictate);
    let recording = host.wait_for(Phase::Recording);
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(host.last_phase(), Some(Phase::Recording), "still recording after release");
    assert!(host.statuses.lock().iter().any(|s| s.phase == Phase::Recording && s.locked) || recording.locked);
    engine.toggle();
    host.wait_for(Phase::Done);
    assert!(host.text.lock().contains("Hands free works"));
}

#[test]
fn phones_start_on_a_phone_model_and_keep_unknown_settings() {
    let host = FakeHost::new(FieldKind::Text);
    let engine = engine(&host, "");
    let view: serde_json::Value = serde_json::from_str(&engine.settings_json().unwrap()).unwrap();
    assert_eq!(view["settings"]["model"], models::MOBILE_DEFAULT);
    assert_eq!(view["has_api_key"], false);

    let mut settings = view["settings"].clone();
    settings["remove_fillers"] = serde_json::Value::Bool(false);
    settings["model"] = "small".into(); // a desktop model is moved back to a phone one
    engine.save_settings_json(settings.to_string()).unwrap();
    let saved: serde_json::Value = serde_json::from_str(&engine.settings_json().unwrap()).unwrap();
    assert_eq!(saved["settings"]["remove_fillers"], false);
    assert_eq!(saved["settings"]["model"], models::MOBILE_DEFAULT);
}

#[test]
fn api_keys_go_to_the_vault() {
    let host = FakeHost::new(FieldKind::Text);
    let engine = engine(&host, "");
    let mut view: serde_json::Value = serde_json::from_str(&engine.settings_json().unwrap()).unwrap();
    view["settings"]["ai"]["provider"] = "gemini".into();
    engine.save_settings_json(view["settings"].to_string()).unwrap();
    engine.set_api_key("gemini".into(), "key-123".into()).unwrap();
    let view: serde_json::Value = serde_json::from_str(&engine.settings_json().unwrap()).unwrap();
    assert_eq!(view["has_api_key"], true);
    assert!(engine.set_api_key("not-a-provider".into(), "x".into()).is_err());
}

#[test]
fn dictionary_fixes_apply_to_dictation() {
    let host = FakeHost::new(FieldKind::Text);
    let engine = engine(&host, "we built it with tory");
    engine.dictionary_upsert("tory".into(), Some("Tauri".into())).unwrap();
    hold_and_speak(&engine, &host);
    host.wait_for(Phase::Done);
    assert!(host.text.lock().contains("Tauri"), "{}", host.text.lock());
}

#[test]
fn only_phone_models_are_listed() {
    let host = FakeHost::new(FieldKind::Text);
    let engine = engine(&host, "");
    let list: Vec<serde_json::Value> = serde_json::from_str(&engine.models_json().unwrap()).unwrap();
    assert!(!list.is_empty());
    assert!(list.iter().all(|m| m["mobile"] == true));
}

#[test]
fn word_diff_is_exported_as_json() {
    let diff: serde_json::Value = serde_json::from_str(&word_diff_json("um see you".into(), "See you.".into())).unwrap();
    assert_eq!(diff["dropped"], 1);
    assert_eq!(diff["heard"][0]["kind"], "drop");
    assert_eq!(change_summary("um see you".into(), "See you.".into()), "1 word removed");
}

/// Real speech through the real engine, as on a phone. Needs a WAV:
///   say -o /tmp/hams.wav --data-format=LEI16@22050 "Hello from Hams. This is a dictation test."
///   HAMS_TEST_WAV=/tmp/hams.wav cargo test -p hams-mobile --release real_speech_mobile -- --ignored --nocapture
/// Downloads the phone model (57 MB) into the system temp folder the first time.
#[test]
#[ignore]
#[cfg(feature = "local-stt")]
fn real_speech_mobile() {
    let wav = std::env::var("HAMS_TEST_WAV").expect("set HAMS_TEST_WAV");
    let (samples, rate) = read_wav(&wav);
    let data_dir = std::env::temp_dir().join("hams-mobile-real-speech");
    hams_core::paths::init(&data_dir);
    let host = FakeHost::new(FieldKind::Text);
    host.mic_allowed.store(true, Ordering::SeqCst);
    let engine = Arc::new(
        HamsEngine::build(
            &data_dir.to_string_lossy(),
            host.clone(),
            Arc::new(FakeVault::default()),
            transcriber(),
        )
        .unwrap(),
    );
    struct Progress;
    impl DownloadListener for Progress {
        fn on_progress(&self, _: f32) {}
    }
    engine.download_model(models::MOBILE_DEFAULT.into(), Arc::new(Progress)).unwrap();
    engine.press(Pad::Dictate);
    host.wait_for(Phase::Recording);
    for chunk in samples.chunks(rate as usize / 10) {
        engine.push_audio_i16(chunk.to_vec(), rate);
    }
    std::thread::sleep(Duration::from_millis(300));
    engine.release(Pad::Dictate);
    let deadline = Instant::now() + Duration::from_secs(60);
    while !matches!(host.last_phase(), Some(Phase::Done | Phase::Error)) {
        assert!(Instant::now() < deadline, "timed out");
        std::thread::sleep(Duration::from_millis(50));
    }
    let text = host.text.lock().clone();
    println!("Inserted: {text:?}  statuses: {:?}", host.statuses.lock().last());
    assert!(text.to_lowercase().contains("dictation test"), "{text}");
}

#[cfg(feature = "local-stt")]
fn read_wav(path: &str) -> (Vec<i16>, u32) {
    let bytes = std::fs::read(path).unwrap();
    let rate = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
    let mut pos = 12;
    while pos + 8 < bytes.len() {
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        if &bytes[pos..pos + 4] == b"data" {
            let data = &bytes[pos + 8..(pos + 8 + size).min(bytes.len())];
            return (data.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect(), rate);
        }
        pos += 8 + size;
    }
    panic!("no data chunk in {path}");
}
