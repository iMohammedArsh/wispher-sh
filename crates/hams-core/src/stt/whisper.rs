//! In-process whisper.cpp (Metal-accelerated on macOS). The model stays loaded
//! between dictations, so there's no server process, port or WAV file involved.

use std::path::PathBuf;

use parking_lot::Mutex;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState};

use super::{models, pick_language, strip_stock_phrases, sanitize, vocabulary_prompt, SttRequest, Transcriber};
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
    let window = &samples[..samples.len().min(30 * 16_000)];
    let detected = loaded.state.pcm_to_mel(window, threads()).and_then(|()| loaded.state.lang_detect(0, threads()));
    match detected {
        Ok((_, probs)) => {
            let scores: Vec<(&str, f32)> = probs
                .iter()
                .enumerate()
                .filter_map(|(id, p)| whisper_rs::get_lang_str(id as i32).map(|code| (code, *p)))
                .collect();
            let chosen = pick_language(&scores, allowed);
            let best = scores.iter().max_by(|a, b| a.1.total_cmp(&b.1));
            tracing::debug!("Language: Whisper's best guess {best:?}, allowed {allowed:?}, using {chosen}");
            chosen
        }
        Err(error) => {
            tracing::warn!("Language detection failed: {error}");
            "auto".into()
        }
    }
}

fn is_auto(language: &str) -> bool {
    language.is_empty() || language == "auto"
}

impl Transcriber for WhisperEngine {
    fn transcribe(&self, samples: &[f32], request: &SttRequest) -> AppResult<String> {
        let mut slot = self.loaded.lock();
        let loaded = self.ensure_loaded(&mut slot, &request.model)?;

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
        // With a list of allowed languages, detect here and narrow the guess to
        // that list; otherwise let Whisper detect on its own.
        let language = if !is_auto(&request.language) {
            request.language.clone()
        } else if request.languages.iter().any(|l| !is_auto(l)) {
            detect_language(loaded, samples, &request.languages)
        } else {
            "auto".into()
        };
        params.set_language(Some(&language));
        let prompt = vocabulary_prompt(&request.vocabulary);
        if !prompt.is_empty() {
            params.set_initial_prompt(&prompt);
        }

        let started = std::time::Instant::now();
        loaded
            .state
            .full(params, samples)
            .map_err(|e| AppError::Transcription(format!("Whisper failed: {e}")))?;
        let mut text = String::new();
        for segment in loaded.state.as_iter() {
            if segment.no_speech_probability() > 0.8 {
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
            "Whisper: {:.1}s of audio in {:?}",
            samples.len() as f32 / 16_000.0,
            started.elapsed()
        );
        Ok(sanitize(&text))
    }

    fn resolve_language(&self, samples: &[f32], request: &SttRequest) -> String {
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
        if self.loaded.lock().take().is_some() {
            tracing::info!("Released the Whisper model");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    /// buffer, take ~30 s chunks at pauses like `AudioRecorder::drain`, and
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
}
