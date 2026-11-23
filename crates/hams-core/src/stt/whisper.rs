//! In-process whisper.cpp (Metal-accelerated on macOS). The model stays loaded
//! between dictations, so there's no server process, port or WAV file involved.

use std::path::PathBuf;
use parking_lot::Mutex;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState};

use super::{models, pick_language, strip_stock_phrases, sanitize, vocabulary_prompt, CancelFn, SttRequest, Transcriber};
use crate::error::{AppError, AppResult};

struct Loaded {
    path: PathBuf,
    // Field order matters: the state must drop before the context.
    state: WhisperState,
    _context: WhisperContext,
}

#[derive(Default)]
pub struct WhisperEngine {
    loaded: Mutex<Option<Loaded>>,
}

impl WhisperEngine {
    pub fn new() -> Self {
        whisper_rs::install_logging_hooks();
        Self::default()
    }

    fn ensure_loaded<'a>(&self, slot: &'a mut Option<Loaded>, model: &str) -> AppResult<&'a mut Loaded> {
        let path = models::path_for(model).ok_or_else(|| AppError::Model(format!("unknown model \"{model}\"")))?;
        if !path.is_file() {
            return Err(AppError::Model(format!(
                "the {} model isn't downloaded yet. Open Settings → Model to download it.",
                models::info(model).map(|m| m.label).unwrap_or(model)
            )));
        }
        if slot.as_ref().map(|l| l.path != path).unwrap_or(true) {
            *slot = None; // free the old model first
            let started = std::time::Instant::now();
            let context = WhisperContext::new_with_params(&path.to_string_lossy(), WhisperContextParameters::default())
                .map_err(|e| AppError::Model(format!("could not load {}: {e}", path.display())))?;
            let state = context
                .create_state()
                .map_err(|e| AppError::Model(format!("could not prepare the model: {e}")))?;
            tracing::info!("Loaded Whisper model {} in {:?}", path.display(), started.elapsed());
            *slot = Some(Loaded { path, state, _context: context });
        }
        Ok(slot.as_mut().expect("just loaded"))
    }
}

fn threads() -> usize {
    // Phones mix big and little cores; more than four threads lands work on the
    // slow ones and makes inference slower, not faster.
    let max = if cfg!(any(target_os = "android", target_os = "ios")) { 4 } else { 8 };
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(1, max)
}

/// Whisper's language scores for the first 30 s of `samples`, narrowed to the
/// allowed languages by [`pick_language`]. "auto" if detection fails.
fn detect_language(loaded: &mut Loaded, samples: &[f32], allowed: &[String]) -> String {
    match language_scores(loaded, samples) {
        Some(scores) => {
            let chosen = pick_language(&scores, allowed);
            let best = scores.iter().max_by(|a, b| a.1.total_cmp(&b.1));
            tracing::debug!("Language: Whisper's best guess {best:?}, allowed {allowed:?}, using {chosen}");
            chosen
        }
        None => "auto".into(),
    }
}

/// Whisper's probability for every language it knows, from the first 30 s.
fn language_scores(loaded: &mut Loaded, samples: &[f32]) -> Option<Vec<(&'static str, f32)>> {
    let window = &samples[..samples.len().min(30 * 16_000)];
    let detected = loaded.state.pcm_to_mel(window, threads()).and_then(|()| loaded.state.lang_detect(0, threads()));
    match detected {
        Ok((_, probs)) => Some(
            probs
                .iter()
                .enumerate()
                .filter_map(|(id, p)| whisper_rs::get_lang_str(id as i32).map(|code| (code, *p)))
                .collect(),
        ),
        Err(error) => {
            tracing::warn!("Language detection failed: {error}");
            None
        }
    }
}

fn is_auto(language: &str) -> bool {
    language.is_empty() || language == "auto"
}

/// English-only models (`*.en`) can't detect or write other languages.
fn english_only(model: &str) -> bool {
    models::info(model).is_some_and(|m| !m.multilingual)
}

/// Segments Whisper thinks are probably not speech are dropped above this
/// probability.
const NO_SPEECH_LIMIT: f32 = 0.8;
/// For a whisper or a far-away voice. Whisper often scores that kind of speech as
/// "no speech" even when it has transcribed it correctly, so the bar is higher.
/// The stock-phrase filter below still catches the hallucinations this lets in.
const NO_SPEECH_LIMIT_QUIET: f32 = 0.95;

fn no_speech_limit(quiet: bool) -> f32 {
    if quiet { NO_SPEECH_LIMIT_QUIET } else { NO_SPEECH_LIMIT }
}

/// Whisper reads audio in 30 s windows and decodes each window on its own.
const WINDOW_SECONDS: f32 = 30.0;

/// The most tokens one window may decode, scaled to the audio in it. Real speech
/// is a handful of tokens a second (more in scripts that take several tokens a
/// word), so this is generous; it only stops a repetition loop from running to
/// Whisper's own 220-token limit on a short clip.
fn max_tokens_for(samples: usize) -> i32 {
    let seconds = (samples as f32 / 16_000.0).min(WINDOW_SECONDS);
    ((seconds * 20.0) as i32 + 32).clamp(48, 220)
}

/// Encoder frames to use for a clip shorter than a window, or 0 for Whisper's
/// default (the full 30 s, which costs the same for a 3 s clip as for a 30 s one).
/// 50 frames per second of audio plus a margin, rounded up to 64.
fn audio_ctx_for(samples: usize, enabled: bool) -> i32 {
    if !enabled || samples as f32 / 16_000.0 >= WINDOW_SECONDS - 1.0 {
        return 0;
    }
    let frames = (samples as f32 / 16_000.0 * 50.0) as i32 + 64;
    ((frames + 63) / 64 * 64).clamp(384, 1500)
}

/// Encode only as much audio as there is. Off: it hasn't been benchmarked on the
/// phone model yet. To try it, set this to `cfg!(any(target_os = "android", target_os = "ios"))`
/// and compare `quiet_speech_bench` / `language_bench` with `HAMS_TEST_MODEL=base-q5_1`
/// before and after; keep it only if WER moves by 2 points or less and language
/// detection doesn't change.
const SHORT_AUDIO_CTX: bool = false;

/// Called by whisper.cpp between encoder and decoder steps; `data` is a `*const CancelFn`.
unsafe extern "C" fn abort_trampoline(data: *mut std::ffi::c_void) -> bool {
    let cancel = &*(data as *const CancelFn);
    cancel()
}

/// One pass of Whisper over `samples`, written in `language` (a code, or "auto").
/// `cancel` stops the pass within one encoder or decoder step.
fn decode(
    loaded: &mut Loaded,
    samples: &[f32],
    request: &SttRequest,
    language: &str,
    cancel: Option<&CancelFn>,
) -> AppResult<String> {
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_n_threads(threads() as i32);
    params.set_translate(false);
    params.set_no_context(true);
    params.set_no_timestamps(true);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_suppress_blank(true);
    params.set_suppress_nst(true);
    params.set_temperature(0.0);
    // One decode per window. Whisper's default (0.2) re-decodes a window at up to
    // five higher temperatures whenever it isn't confident, which on a phone CPU
    // turned quiet or accented audio into a minute of waiting.
    params.set_temperature_inc(0.0);
    params.set_max_tokens(max_tokens_for(samples.len()));
    params.set_audio_ctx(audio_ctx_for(samples.len(), SHORT_AUDIO_CTX));
    params.set_language(Some(language));
    let prompt = vocabulary_prompt(&request.vocabulary);
    if !prompt.is_empty() {
        params.set_initial_prompt(&prompt);
    }
    // Not `set_abort_callback_safe`: in whisper-rs 0.15 its trampoline reads the
    // user data as the wrong type and crashes on the first call. `cancel` outlives
    // the `full` call below, which is the only time whisper.cpp uses the pointer.
    if let Some(cancel) = cancel {
        // SAFETY: the pointer is to a `CancelFn` that lives until the end of this function.
        unsafe {
            params.set_abort_callback(Some(abort_trampoline));
            params.set_abort_callback_user_data(cancel as *const CancelFn as *mut std::ffi::c_void);
        }
    }

    let started = std::time::Instant::now();
    loaded.state.full(params, samples).map_err(|e| {
        if cancel.is_some_and(|c| c()) {
            AppError::Transcription("cancelled".into())
        } else {
            AppError::Transcription(format!("Whisper failed: {e}"))
        }
    })?;
    let mut text = String::new();
    for segment in loaded.state.as_iter() {
        if segment.no_speech_probability() > no_speech_limit(request.quiet) {
            continue;
        }
        if let Ok(words) = segment.to_str_lossy() {
            let kept = strip_stock_phrases(&words);
            if kept.len() != words.len() {
                tracing::debug!("Dropped a likely hallucination from {words:?}");
            }
            text.push_str(&kept);
            text.push(' ');
        }
    }
    tracing::debug!(
        "Whisper ({language}): {:.1}s of audio in {:?}",
        samples.len() as f32 / 16_000.0,
        started.elapsed()
    );
    Ok(sanitize(&text))
}

impl WhisperEngine {
    fn run(&self, samples: &[f32], request: &SttRequest, cancel: Option<&CancelFn>) -> AppResult<String> {
        let waiting = std::time::Instant::now();
        let mut slot = self.loaded.lock();
        let waited = waiting.elapsed();
        if waited.as_millis() > 50 {
            tracing::info!("Whisper was busy: waited {waited:?} for the model");
        }
        if cancel.is_some_and(|c| c()) {
            return Err(AppError::Transcription("cancelled".into()));
        }
        let loaded = self.ensure_loaded(&mut slot, &request.model)?;
        // With a list of allowed languages, detect here and narrow the guess to
        // that list; otherwise let Whisper detect on its own.
        let language = if english_only(&request.model) {
            "en".into()
        } else if !is_auto(&request.language) {
            request.language.clone()
        } else if request.languages.iter().any(|l| !is_auto(l)) {
            let started = std::time::Instant::now();
            let chosen = detect_language(loaded, samples, &request.languages);
            tracing::debug!("Language detection took {:?}", started.elapsed());
            chosen
        } else {
            "auto".into()
        };
        decode(loaded, samples, request, &language, cancel)
    }
}

impl Transcriber for WhisperEngine {
    fn transcribe(&self, samples: &[f32], request: &SttRequest) -> AppResult<String> {
        self.run(samples, request, None)
    }

    fn transcribe_cancellable(&self, samples: &[f32], request: &SttRequest, cancel: &CancelFn) -> AppResult<String> {
        self.run(samples, request, Some(cancel))
    }

    fn resolve_language(&self, samples: &[f32], request: &SttRequest) -> String {
        if english_only(&request.model) {
            return "en".into();
        }
        if !is_auto(&request.language) {
            return request.language.clone();
        }
        let mut slot = self.loaded.lock();
        match self.ensure_loaded(&mut slot, &request.model) {
            Ok(loaded) => detect_language(loaded, samples, &request.languages),
            Err(_) => "auto".into(),
        }
    }

    fn warm_up(&self, model: &str) {
        let request = SttRequest { language: "en".into(), model: model.into(), ..SttRequest::default() };
        // One second of near-silence compiles GPU kernels so the first real dictation is fast.
        let silence = vec![0.0_f32; 16_000];
        match self.transcribe(&silence, &request) {
            Ok(_) => tracing::info!("Whisper warmed up"),
            Err(error) => tracing::info!("Whisper warm-up skipped: {error}"),
        }
    }

    fn release(&self) {
        // Never wait for a transcription that's running (this is called from the
        // phone's main thread): the model is in use, so it isn't idle memory.
        let Some(mut slot) = self.loaded.try_lock() else {
            tracing::info!("Kept the Whisper model: a transcription is running");
            return;
        };
        if slot.take().is_some() {
            tracing::info!("Released the Whisper model");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_cap_scales_with_the_clip_but_stays_generous() {
        assert_eq!(max_tokens_for(0), 48);
        assert_eq!(max_tokens_for(16_000 * 4), 112);
        // A window is at most 30 s, so a long recording gets Whisper's own limit.
        assert_eq!(max_tokens_for(16_000 * 30), 220);
        assert_eq!(max_tokens_for(16_000 * 600), 220);
        // ~7 tokens a second would already be fast English speech.
        assert!(max_tokens_for(16_000 * 10) > 70);
    }

    #[test]
    fn audio_context_is_whisper_default_unless_enabled_and_the_clip_is_short() {
        assert_eq!(audio_ctx_for(16_000 * 4, false), 0);
        assert_eq!(audio_ctx_for(16_000 * 30, true), 0);
        assert_eq!(audio_ctx_for(16_000 * 29, true), 0, "a nearly full window uses the whole window");
        assert!(!SHORT_AUDIO_CTX, "enable only after benchmarking on the phone model");
    }

    #[test]
    fn a_short_audio_context_covers_the_audio_in_steps_of_64() {
        for seconds in [1.0_f32, 4.0, 12.5, 25.0] {
            let ctx = audio_ctx_for((seconds * 16_000.0) as usize, true);
            assert!(ctx >= (seconds * 50.0) as i32, "{seconds}s → {ctx} frames can't hold the audio");
            assert!(ctx % 64 == 0 && (384..=1500).contains(&ctx), "{seconds}s → {ctx}");
        }
    }

    /// Minimal 16-bit PCM WAV reader for the test fixture.
    fn read_wav(path: &str) -> (Vec<f32>, u32) {
        let bytes = std::fs::read(path).expect("read wav");
        let (mut rate, mut channels) = (16_000u32, 1usize);
        let mut i = 12;
        while i + 8 <= bytes.len() {
            let id = &bytes[i..i + 4];
            let size = u32::from_le_bytes(bytes[i + 4..i + 8].try_into().unwrap()) as usize;
            let body = &bytes[i + 8..(i + 8 + size).min(bytes.len())];
            if id == b"fmt " {
                channels = u16::from_le_bytes([body[2], body[3]]) as usize;
                rate = u32::from_le_bytes(body[4..8].try_into().unwrap());
            } else if id == b"data" {
                let samples = body
                    .chunks_exact(2 * channels)
                    .map(|f| i16::from_le_bytes([f[0], f[1]]) as f32 / 32768.0)
                    .collect();
                return (samples, rate);
            }
            i += 8 + size + (size & 1);
        }
        panic!("no data chunk");
    }

    /// End-to-end: real speech through DSP and Whisper. Needs a downloaded model.
    /// macOS: `say -o /tmp/hams.wav --data-format=LEI16@22050 "Hello from Hams. This is a dictation test."`
    /// then `HAMS_TEST_WAV=/tmp/hams.wav cargo test --release real_speech -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_speech_is_transcribed() {
        let path = std::env::var("HAMS_TEST_WAV").expect("set HAMS_TEST_WAV");
        let model = std::env::var("HAMS_TEST_MODEL").unwrap_or_else(|_| "small".into());
        let (raw, rate) = read_wav(&path);
        let captured = crate::audio::dsp::prepare(&raw, rate);
        assert!(captured.has_speech);
        let engine = WhisperEngine::new();
        let request = SttRequest { language: "en".into(), model, vocabulary: vec!["Hams".into()], ..SttRequest::default() };
        engine.transcribe(&captured.samples[..16_000], &request).ok(); // warm-up
        let started = std::time::Instant::now();
        let text = engine.transcribe(&captured.samples, &request).unwrap();
        println!("{:?} in {:?}", text, started.elapsed());
        let lower = text.to_lowercase();
        assert!(lower.contains("hello") && lower.contains("dictation"), "got {text:?}");
    }

    /// A transcription that is cancelled stops within a step or two instead of
    /// running to the end (so a live-preview pass can't make the final text wait),
    /// and the engine is fine afterwards. Same fixture as `real_speech_is_transcribed`.
    #[test]
    #[ignore]
    fn a_cancelled_transcription_stops_early_and_leaves_the_engine_usable() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        let path = std::env::var("HAMS_TEST_WAV").expect("set HAMS_TEST_WAV");
        let model = std::env::var("HAMS_TEST_MODEL").unwrap_or_else(|_| "small".into());
        let (raw, rate) = read_wav(&path);
        let one = crate::audio::dsp::prepare(&raw, rate).samples;
        // Several windows long, so a full pass takes a good while.
        let long: Vec<f32> = one.iter().cycle().take(one.len().max(1) * 8).copied().collect();
        let engine = WhisperEngine::new();
        let request = SttRequest { language: "en".into(), model, ..SttRequest::default() };
        engine.transcribe(&one, &request).unwrap(); // load the model

        let started = std::time::Instant::now();
        engine.transcribe(&long, &request).unwrap();
        let full = started.elapsed();

        let flag = Arc::new(AtomicBool::new(false));
        let cancel: CancelFn = { let flag = Arc::clone(&flag); Arc::new(move || flag.load(Ordering::Relaxed)) };
        let trigger = { let flag = Arc::clone(&flag); std::thread::spawn(move || { std::thread::sleep(full / 10); flag.store(true, Ordering::Relaxed); }) };
        let started = std::time::Instant::now();
        let result = engine.transcribe_cancellable(&long, &request, &cancel);
        let cancelled_after = started.elapsed();
        trigger.join().unwrap();
        println!("full pass {full:?}; cancelled after {cancelled_after:?}: {result:?}");
        assert!(result.is_err(), "a cancelled pass reports an error");
        assert!(cancelled_after < full / 2, "cancelling saved most of the work");

        // Already cancelled before it starts: no work at all.
        let started = std::time::Instant::now();
        assert!(engine.transcribe_cancellable(&long, &request, &cancel).is_err());
        assert!(started.elapsed() < full / 4);

        // The aborted state is reusable and gives the same text as before.
        let again = engine.transcribe(&one, &request).unwrap();
        assert_eq!(again, engine.transcribe(&one, &request).unwrap());
        assert!(!again.is_empty());
    }

    /// Accented speech and the allowed-languages list. English read by the
    /// macOS Malay voice is "English in a Malaysian accent":
    /// `say -v Amira -o /tmp/accent.wav --data-format=LEI16@22050 "Today we talk about the constitution and the powers of the king."`
    /// then `HAMS_TEST_ACCENT_WAV=/tmp/accent.wav cargo test --release real_speech_accent -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_speech_accent_stays_in_an_allowed_language() {
        let path = std::env::var("HAMS_TEST_ACCENT_WAV").expect("set HAMS_TEST_ACCENT_WAV");
        let model = std::env::var("HAMS_TEST_MODEL").unwrap_or_else(|_| "small".into());
        let (raw, rate) = read_wav(&path);
        let captured = crate::audio::dsp::prepare(&raw, rate);
        let engine = WhisperEngine::new();
        let any = SttRequest { language: "auto".into(), model: model.clone(), ..SttRequest::default() };
        let mine = SttRequest { languages: vec!["en".into(), "ml".into()], ..any.clone() };
        println!("any language: {} → {:?}", engine.resolve_language(&captured.samples, &any), engine.transcribe(&captured.samples, &any).unwrap());
        let chosen = engine.resolve_language(&captured.samples, &mine);
        let text = engine.transcribe(&captured.samples, &mine).unwrap();
        println!("en or ml:     {chosen} → {text:?}");
        assert_eq!(chosen, "en");
    }

    /// Notes mode end to end on real speech: stream a long recording into a
    /// buffer, take ~30 s chunks at pauses like `PushRecorder::drain`, and
    /// check the joined chunks read like one pass over the whole recording.
    /// Make a long WAV with `say -f some.txt -o /tmp/note.wav --data-format=LEI16@22050`,
    /// then `HAMS_TEST_NOTE_WAV=/tmp/note.wav cargo test --release real_speech_note -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_speech_note_is_chunked_at_pauses() {
        use crate::audio::dsp;
        let path = std::env::var("HAMS_TEST_NOTE_WAV").expect("set HAMS_TEST_NOTE_WAV");
        let (raw, rate) = read_wav(&path);
        assert!(raw.len() > rate as usize * 45, "use a recording longer than 45 s");
        let engine = WhisperEngine::new();
        let request = SttRequest { language: "en".into(), model: "small".into(), ..SttRequest::default() };
        let transcribe = |samples: &[f32]| {
            let captured = dsp::prepare(samples, rate);
            if captured.has_speech { engine.transcribe(&captured.samples, &request).unwrap() } else { String::new() }
        };

        // Arrives in 100 ms blocks; checked once a second, like the controller.
        let (mut buffer, mut chunks) = (Vec::new(), Vec::new());
        for (i, block) in raw.chunks(rate as usize / 10).enumerate() {
            buffer.extend_from_slice(block);
            if i % 10 == 9 {
                if let Some(chunk) = dsp::take_chunk(&mut buffer, rate, 30) {
                    println!("chunk {}: {:.1} s", chunks.len() + 1, chunk.len() as f32 / rate as f32);
                    chunks.push(transcribe(&chunk));
                }
            }
        }
        chunks.push(transcribe(&buffer));
        let chunked = chunks.join(" ");
        let whole = transcribe(&raw);
        println!("chunked: {chunked}\nwhole:   {whole}");

        let words = |t: &str| -> Vec<String> {
            t.split_whitespace().map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase()).filter(|w| !w.is_empty()).collect()
        };
        let (a, b) = (words(&chunked), words(&whole));
        let found = a.iter().filter(|w| b.contains(w)).count();
        let overlap = found as f32 / a.len().max(b.len()) as f32;
        println!("{} chunks, {} vs {} words, overlap {:.0}%", chunks.len(), a.len(), b.len(), overlap * 100.0);
        assert!(chunks.len() >= 2, "expected the recording to be split");
        assert!(overlap > 0.97, "chunked transcript drifted from the single pass");
    }

    // ---- Quiet-speech benchmark ------------------------------------------

    /// Word error rate: word-level Levenshtein distance over the reference length.
    fn wer(reference: &str, hypothesis: &str) -> f32 {
        let words = |t: &str| -> Vec<String> {
            t.split_whitespace()
                .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
                .filter(|w| !w.is_empty())
                .collect()
        };
        let (r, h) = (words(reference), words(hypothesis));
        if r.is_empty() {
            return if h.is_empty() { 0.0 } else { 1.0 };
        }
        let mut row: Vec<usize> = (0..=h.len()).collect();
        for (i, rw) in r.iter().enumerate() {
            let mut diagonal = row[0];
            row[0] = i + 1;
            for (j, hw) in h.iter().enumerate() {
                let above = row[j + 1];
                row[j + 1] = (diagonal + usize::from(rw != hw)).min(row[j] + 1).min(above + 1);
                diagonal = above;
            }
        }
        row[h.len()] as f32 / r.len() as f32
    }

    #[test]
    fn a_quiet_recording_gets_a_more_forgiving_no_speech_limit() {
        assert_eq!(no_speech_limit(false), 0.8);
        assert!(no_speech_limit(true) > no_speech_limit(false));
        assert!(no_speech_limit(true) < 1.0, "a limit of 1 would keep every segment");
    }

    #[test]
    fn wer_counts_substitutions_insertions_and_deletions() {
        assert_eq!(wer("the cat sat", "The cat sat."), 0.0);
        assert!((wer("the cat sat", "the dog sat") - 1.0 / 3.0).abs() < 1e-6);
        assert!((wer("the cat sat", "the cat") - 1.0 / 3.0).abs() < 1e-6);
        assert!((wer("the cat", "the big cat") - 0.5).abs() < 1e-6);
        assert_eq!(wer("hello", ""), 1.0);
    }

    /// Small deterministic noise source (xorshift) so every run sees the same room.
    struct Noise(u32);
    impl Noise {
        fn white(&mut self) -> f32 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 17;
            self.0 ^= self.0 << 5;
            self.0 as f32 / u32::MAX as f32 * 2.0 - 1.0
        }
    }

    /// Add `kind` noise at `dbfs` (RMS) across the whole clip.
    /// `fan` is low-passed (rumbly, like HVAC); `office` is a band-limited hiss
    /// with a slow wobble, a rough stand-in for distant chatter.
    fn add_room(samples: &mut [f32], kind: &str, dbfs: f32, rate: u32) {
        if kind == "silent" {
            return;
        }
        let gain = 10f32.powf(dbfs / 20.0) * 3f32.sqrt(); // white is uniform: RMS = peak / sqrt(3)
        let mut noise = Noise(0x9E37_79B9);
        let mut low = 0.0f32;
        for (i, sample) in samples.iter_mut().enumerate() {
            let white = noise.white();
            let shaped = match kind {
                "fan" => {
                    low += 0.02 * (white - low);
                    low * 6.0
                }
                _ => {
                    let wobble = 0.6 + 0.4 * (i as f32 / rate as f32 * 2.0 * std::f32::consts::PI * 3.0).sin();
                    white * wobble
                }
            };
            *sample += shaped * gain;
        }
    }

    /// Whisper in a noisy room: each clip at several levels and rooms through
    /// `dsp::prepare` and Whisper. Prints a table and writes a CSV so a before/after
    /// run can be compared. Make clips with `scripts/make-quiet-fixtures.sh`.
    ///   HAMS_BENCH=1 [HAMS_BENCH_DIR=/tmp/hams-bench] [HAMS_TEST_MODEL=small] \
    ///   cargo test -p hams-core --release quiet_speech_bench -- --ignored --nocapture
    #[test]
    #[ignore]
    fn quiet_speech_bench() {
        use crate::audio::dsp;
        std::env::var("HAMS_BENCH").expect("set HAMS_BENCH=1");
        let dir = std::env::var("HAMS_BENCH_DIR").unwrap_or_else(|_| "/tmp/hams-bench".into());
        let model = std::env::var("HAMS_TEST_MODEL").unwrap_or_else(|_| "small".into());
        // HAMS_BENCH_VERBOSE=1 prints what Whisper heard for every clip that went wrong.
        let verbose = std::env::var("HAMS_BENCH_VERBOSE").is_ok();
        let mut clips: Vec<(String, Vec<f32>, u32, String)> = Vec::new();
        for entry in std::fs::read_dir(&dir).expect("HAMS_BENCH_DIR: run scripts/make-quiet-fixtures.sh first") {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "wav") {
                let reference = std::fs::read_to_string(path.with_extension("txt")).expect("a .txt next to every .wav");
                let (samples, rate) = read_wav(path.to_str().unwrap());
                let stem = path.file_stem().unwrap().to_string_lossy().to_string();
                clips.push((stem, samples, rate, reference));
            }
        }
        clips.sort_by(|a, b| a.0.cmp(&b.0));
        assert!(!clips.is_empty(), "no .wav files in {dir}");
        if verbose {
            // How loud each clip is on its own: the 90th percentile of 30 ms frame levels.
            for (name, raw, rate, _) in &clips {
                let mut levels: Vec<f32> = raw
                    .chunks(*rate as usize * 30 / 1000)
                    .map(|f| 10.0 * (f.iter().map(|s| s * s).sum::<f32>() / f.len() as f32).max(1e-12).log10())
                    .collect();
                levels.sort_by(|a, b| a.total_cmp(b));
                println!("  level {name}: {:.1} dBFS", levels[levels.len() * 9 / 10]);
            }
        }

        let engine = WhisperEngine::new();
        let request = SttRequest { language: "en".into(), model: model.clone(), ..SttRequest::default() };
        engine.transcribe(&vec![0.0; 16_000], &request).ok(); // warm-up

        // The voice kind is the clip's name up to the dash ("whisper-3" -> "whisper").
        // Rows: voice × attenuation × room.
        let mut rows: std::collections::BTreeMap<(String, i32, &str), (usize, usize, f32, u128)> = Default::default();
        for (name, raw, rate, reference) in &clips {
            let voice = name.split('-').next().unwrap_or(name).to_string();
            for attenuation in [0, -20, -30, -40] {
                for room in ["silent", "fan", "office"] {
                    let gain = 10f32.powf(attenuation as f32 / 20.0);
                    let mut audio: Vec<f32> = raw.iter().map(|s| s * gain).collect();
                    // Lead-in and tail so the room is heard on its own, like a real recording.
                    let pad = vec![0.0; *rate as usize];
                    audio.splice(0..0, pad.iter().copied());
                    audio.extend_from_slice(&pad);
                    add_room(&mut audio, room, if room == "fan" { -65.0 } else { -55.0 }, *rate);

                    let started = std::time::Instant::now();
                    let captured = dsp::prepare(&audio, *rate);
                    let (heard, error) = if captured.has_speech {
                        let clip_request = SttRequest { quiet: captured.quiet, ..request.clone() };
                        let text = engine.transcribe(&captured.samples, &clip_request).unwrap_or_default();
                        let error = wer(reference, &text);
                        if verbose && error > 0.2 {
                            println!(
                                "  [{name} {attenuation} dB {room}] quiet={} {:.1}s of {:.1}s WER {:.0}% → {text:?}",
                                captured.quiet,
                                captured.samples.len() as f32 / 16_000.0,
                                audio.len() as f32 / *rate as f32,
                                error * 100.0
                            );
                        }
                        (1, error)
                    } else {
                        if verbose {
                            println!("  [{name} {attenuation} dB {room}] no speech found");
                        }
                        (0, 1.0)
                    };
                    let entry = rows.entry((voice.clone(), attenuation, room)).or_default();
                    entry.0 += 1;
                    entry.1 += heard;
                    entry.2 += error;
                    entry.3 += started.elapsed().as_millis();
                }
            }
        }

        let mut csv = String::from("model,voice,attenuation_db,room,clips,has_speech_rate,wer,avg_ms\n");
        println!("\nmodel {model}\n{:<8} {:>5} {:<7} {:>7} {:>7} {:>7}", "voice", "dB", "room", "heard", "WER", "ms");
        for ((voice, attenuation, room), (count, heard, error, millis)) in &rows {
            let n = *count as f32;
            println!(
                "{voice:<8} {attenuation:>5} {room:<7} {:>6.0}% {:>6.0}% {:>7}",
                *heard as f32 / n * 100.0,
                error / n * 100.0,
                millis / *count as u128
            );
            csv.push_str(&format!(
                "{model},{voice},{attenuation},{room},{count},{:.2},{:.3},{}\n",
                *heard as f32 / n,
                error / n,
                millis / *count as u128
            ));
        }
        let out = std::env::var("HAMS_BENCH_CSV").unwrap_or_else(|_| format!("{dir}/results-{model}.csv"));
        std::fs::write(&out, csv).expect("write csv");
        println!("\nCSV: {out}");
    }

    /// Language detection on a folder of clips of one language (`clip00.wav`,
    /// `clip00.txt` with the reference text, …). For each clip it prints Whisper's
    /// top guesses and what [`detect_language`] would choose for a few allowed
    /// lists, then a summary. Malayalam clips: the SMC Malayalam Speech Corpus.
    /// `HAMS_TEST_LANG_DIR=/path HAMS_TEST_LANG=ml cargo test --release language_bench -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn language_bench() {
        let dir = std::env::var("HAMS_TEST_LANG_DIR").expect("set HAMS_TEST_LANG_DIR");
        let expected = std::env::var("HAMS_TEST_LANG").unwrap_or_else(|_| "ml".into());
        let model = std::env::var("HAMS_TEST_MODEL").unwrap_or_else(|_| "small".into());
        let engine = WhisperEngine::new();
        let mut slot = engine.loaded.lock();
        let loaded = engine.ensure_loaded(&mut slot, &model).expect("model");
        let lists: Vec<(&str, Vec<String>)> = vec![
            ("any", vec![]),
            ("ml,en", vec!["ml".into(), "en".into()]),
            ("en,ml", vec!["en".into(), "ml".into()]),
            ("ml,en,ar,zh", ["ml", "en", "ar", "zh"].map(String::from).to_vec()),
            ("en,ml,ar,zh", ["en", "ml", "ar", "zh"].map(String::from).to_vec()),
        ];
        let mut right = vec![0usize; lists.len()];
        let mut clips = 0;
        let mut names: Vec<_> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "wav")).collect();
        names.sort();
        for path in names {
            let (raw, rate) = read_wav(&path.to_string_lossy());
            let captured = crate::audio::dsp::prepare(&raw, rate);
            if !captured.has_speech {
                continue;
            }
            let Some(scores) = language_scores(loaded, &captured.samples) else { continue };
            let mut top: Vec<_> = scores.clone();
            top.sort_by(|a, b| b.1.total_cmp(&a.1));
            let top: Vec<String> = top.iter().take(4).map(|(c, p)| format!("{c} {:.2}", p)).collect();
            let picks: Vec<String> = lists.iter().map(|(_, l)| pick_language(&scores, l)).collect();
            for (i, p) in picks.iter().enumerate() {
                if *p == expected || (i == 0 && *p == expected) {
                    right[i] += 1;
                }
            }
            clips += 1;
            println!("{:<12} {:.1}s  {}  → {}", path.file_stem().unwrap().to_string_lossy(), captured.samples.len() as f32 / 16_000.0, top.join(", "), picks.join(" | "));
        }
        println!("\nmodel {model}, {clips} clips of {expected}");
        for ((name, _), n) in lists.iter().zip(&right) {
            println!("  allowed {name:<12} {n}/{clips} correct");
        }
    }
}
