pub mod models;
#[cfg(feature = "local-stt")]
mod whisper;

#[cfg(feature = "local-stt")]
pub use whisper::WhisperEngine;

use crate::error::AppResult;

#[derive(Debug, Clone, Default)]
pub struct SttRequest {
    /// "auto" or a Whisper language code.
    pub language: String,
    /// Model id from [`models::CATALOG`].
    pub model: String,
    /// Names and jargon to bias recognition toward.
    pub vocabulary: Vec<String>,
}

/// Speech-to-text. Synchronous: callers run it on a blocking thread.
pub trait Transcriber: Send + Sync {
    fn transcribe(&self, samples: &[f32], request: &SttRequest) -> AppResult<String>;
    /// Load the model ahead of time so the first dictation is fast.
    fn warm_up(&self, _model: &str) {}
}

/// Whisper sometimes "hears" these on near-silent audio.
const HALLUCINATIONS: &[&str] = &[
    "thank you.",
    "thank you",
    "thanks for watching!",
    "thanks for watching.",
    "you",
    "bye.",
    "subtitles by the amara.org community",
];

/// Phrases from video subtitles that Whisper invents on silence (often at the
/// edge of a voice-note chunk). Nobody dictates these, so a segment that is
/// only one of them is dropped wherever it appears. Short phrases like
/// "Thank you." are real dictation and only dropped when they're everything.
const STOCK_PHRASES: &[&str] = &[
    "thanks for watching",
    "thank you for watching",
    "thank you so much for watching",
    "thanks for watching and see you next time",
    "please subscribe",
    "please like and subscribe",
    "don't forget to like and subscribe",
    "see you in the next video",
    "subtitles by the amara.org community",
];

/// Remove sentences that are only a stock subtitle phrase from a Whisper segment.
pub fn strip_stock_phrases(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    let mut sentence = String::new();
    let mut chars = segment.chars().peekable();
    while let Some(c) = chars.next() {
        sentence.push(c);
        let ends = matches!(c, '.' | '!' | '?') && chars.peek().is_none_or(|n| n.is_whitespace());
        if ends || chars.peek().is_none() {
            if !is_stock_phrase(&sentence) {
                out.push_str(&sentence);
            }
            sentence.clear();
        }
    }
    out
}

/// Is this sentence only a stock subtitle phrase?
fn is_stock_phrase(segment: &str) -> bool {
    let normalized = segment
        .trim()
        .trim_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace())
        .to_lowercase()
        .replace(['!', '.', ','], "");
    !normalized.is_empty() && STOCK_PHRASES.iter().any(|p| p.replace(['.', ','], "") == normalized)
}

/// Strip whisper.cpp annotations ("[BLANK_AUDIO]", "(music)", "*laughs*") and
/// collapse whitespace. Ported from Hams v1 `sanitize_transcript`.
pub fn sanitize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth = 0usize;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' if depth > 0 => depth -= 1,
            '*' if depth == 0 && chars.peek().is_some_and(|n| n.is_alphabetic()) => {
                // Skip "*laughs*" style stage directions.
                for n in chars.by_ref() {
                    if n == '*' {
                        break;
                    }
                }
            }
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    let cleaned = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if HALLUCINATIONS.contains(&cleaned.to_lowercase().as_str()) {
        return String::new();
    }
    cleaned
}

/// Whisper's initial prompt: a short, natural list of the user's vocabulary.
pub fn vocabulary_prompt(vocabulary: &[String]) -> String {
    let mut prompt = String::new();
    for term in vocabulary {
        if prompt.len() + term.len() > 220 {
            break;
        }
        if !prompt.is_empty() {
            prompt.push_str(", ");
        }
        prompt.push_str(term);
    }
    if !prompt.is_empty() {
        prompt.push('.');
    }
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stock_subtitle_phrases_are_dropped_but_real_thanks_stay() {
        assert_eq!(strip_stock_phrases(" colleagues quickly. Thank you for watching."), " colleagues quickly.");
        assert_eq!(strip_stock_phrases(" Thanks for watching!"), "");
        assert_eq!(strip_stock_phrases(" Please like and subscribe. So we will add it."), " So we will add it.");
        assert_eq!(strip_stock_phrases(" Thank you. See you Monday."), " Thank you. See you Monday.");
        assert_eq!(strip_stock_phrases(" version 2.0 is out"), " version 2.0 is out");
    }

    #[test]
    fn sanitize_strips_annotations() {
        assert_eq!(sanitize(" [BLANK_AUDIO] hello   world (music) "), "hello world");
        assert_eq!(sanitize("so *laughs* anyway"), "so anyway");
        assert_eq!(sanitize("2 * 3 is six"), "2 * 3 is six");
        assert_eq!(sanitize("Thank you."), "");
        assert_eq!(sanitize("Thank you for the help."), "Thank you for the help.");
    }

    #[test]
    fn vocabulary_prompt_is_bounded() {
        assert_eq!(vocabulary_prompt(&[]), "");
        assert_eq!(vocabulary_prompt(&["Hams".into(), "Tauri".into()]), "Hams, Tauri.");
        let many: Vec<String> = (0..200).map(|i| format!("term{i}")).collect();
        assert!(vocabulary_prompt(&many).len() < 240);
    }
}
