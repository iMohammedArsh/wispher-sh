//! Deterministic, offline text processing that runs on every dictation, before
//! and after the optional AI polish. Everything here is instant and testable.

use std::sync::OnceLock;

use regex::{Regex, RegexBuilder};

use crate::store::{DictionaryEntry, Snippet};

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub voice_commands: bool,
    pub remove_fillers: bool,
}

fn regex(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| RegexBuilder::new(pattern).case_insensitive(true).build().expect("valid regex"))
}

/// Spoken commands and filler words. Runs before AI polish.
pub fn clean(raw: &str, options: &Options) -> String {
    let mut text = raw.trim().to_string();
    let started_upper = text.chars().find(|c| c.is_alphabetic()).is_some_and(char::is_uppercase);

    if options.remove_fillers {
        static FILLERS: OnceLock<Regex> = OnceLock::new();
        text = regex(&FILLERS, r"\b(?:um+|uh+|uhm+|erm+|hmm+|mhm)\b[,.]?\s*")
            .replace_all(&text, "")
            .into_owned();
    }

    if options.voice_commands {
        text = apply_scratch_that(&text);
        static NEW_PARAGRAPH: OnceLock<Regex> = OnceLock::new();
        static NEW_LINE: OnceLock<Regex> = OnceLock::new();
        text = regex(&NEW_PARAGRAPH, r"\s*\bnew paragraph\b[,.]?\s*")
            .replace_all(&text, "\n\n")
            .into_owned();
        text = regex(&NEW_LINE, r"\s*\bnew line\b[,.]?\s*")
            .replace_all(&text, "\n")
            .into_owned();
        text = capitalize_after_newlines(&text);
    }

    let mut text = tidy_spaces(&text);
    if started_upper {
        text = capitalize_first(&text);
    }
    text
}

/// "Send it tomorrow. Scratch that. Send it Friday." → "Send it Friday."
fn apply_scratch_that(text: &str) -> String {
    static SCRATCH: OnceLock<Regex> = OnceLock::new();
    let re = regex(&SCRATCH, r"[,.!?]?\s*\b(?:scratch that|delete that)\b[,.!?]?\s*");
    let mut parts = re.split(text);
    let mut result = parts.next().unwrap_or_default().to_string();
    for part in parts {
        result = drop_last_sentence(&result);
        if result.is_empty() || result.ends_with(['.', '!', '?', '\n']) {
            if !result.is_empty() && !part.is_empty() {
                result.push(' ');
            }
            result.push_str(&capitalize_first(part));
        } else {
            result.push(' ');
            result.push_str(part);
        }
    }
    result
}

fn drop_last_sentence(text: &str) -> String {
    let trimmed = text.trim_end();
    match trimmed.rfind(['.', '!', '?', '\n']) {
        Some(index) => trimmed[..=index].trim_end().to_string(),
        None => String::new(),
    }
}

fn tidy_spaces(text: &str) -> String {
    text.split('\n')
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn capitalize_after_newlines(text: &str) -> String {
    text.split('\n').map(capitalize_first).collect::<Vec<_>>().join("\n")
}

fn normalize_phrase(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// If the whole dictation is a snippet trigger (or "insert <trigger>"), return
/// the expansion. Inline matches are not expanded, so saying "I changed my
/// address" never inserts your address by accident.
pub fn expand_snippet(text: &str, snippets: &[Snippet]) -> Option<String> {
    let spoken = normalize_phrase(text);
    let spoken = spoken.strip_prefix("insert ").unwrap_or(&spoken);
    snippets
        .iter()
        .find(|s| normalize_phrase(&s.trigger) == spoken)
        .map(|s| s.expansion.clone())
}

/// Dictionary replacements ("heard" → "written"), whole words only and
/// case-insensitive. Runs after AI polish so the AI can't undo them.
pub fn apply_replacements(text: &str, dictionary: &[DictionaryEntry]) -> String {
    let mut out = text.to_string();
    for entry in dictionary {
        let Some(replacement) = entry.replacement.as_deref() else { continue };
        let term = entry.term.trim();
        if term.is_empty() {
            continue;
        }
        // Word boundaries only make sense next to word characters ("c++" ends in a symbol).
        let edge = |c: Option<char>| if c.is_some_and(|c| c.is_alphanumeric() || c == '_') { r"\b" } else { "" };
        let pattern = format!(r"(?i){}{}{}", edge(term.chars().next()), regex::escape(term), edge(term.chars().last()));
        if let Ok(re) = Regex::new(&pattern) {
            out = re.replace_all(&out, regex::NoExpand(replacement)).into_owned();
        }
    }
    out
}

/// Words to bias Whisper toward: plain vocabulary terms plus replacement targets.
pub fn vocabulary(dictionary: &[DictionaryEntry]) -> Vec<String> {
    dictionary
        .iter()
        .map(|e| e.replacement.clone().unwrap_or_else(|| e.term.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> Options {
        Options { voice_commands: true, remove_fillers: true }
    }

    fn entry(term: &str, replacement: Option<&str>) -> DictionaryEntry {
        DictionaryEntry { id: 0, term: term.into(), replacement: replacement.map(Into::into), source: "user".into() }
    }

    #[test]
    fn removes_fillers_and_keeps_capitalization() {
        assert_eq!(clean("Um, so I think, uh, we should go.", &opts()), "So I think, we should go.");
        assert_eq!(clean("The drum is loud.", &opts()), "The drum is loud.");
        assert_eq!(clean("Hummus is good.", &opts()), "Hummus is good.");
    }

    #[test]
    fn scratch_that_removes_the_previous_sentence() {
        assert_eq!(clean("Send it tomorrow. Scratch that. Send it Friday.", &opts()), "Send it Friday.");
        assert_eq!(clean("Hi team. Meet at 5, scratch that, meet at 6.", &opts()), "Hi team. Meet at 6.");
        assert_eq!(clean("scratch that", &opts()), "");
    }

    #[test]
    fn new_line_and_paragraph() {
        assert_eq!(clean("Dear Sam, new line. Thanks for the notes.", &opts()), "Dear Sam,\nThanks for the notes.");
        assert_eq!(clean("First part. New paragraph. second part.", &opts()), "First part.\n\nSecond part.");
    }

    #[test]
    fn commands_can_be_disabled() {
        let off = Options::default();
        assert_eq!(clean("um new line", &off), "um new line");
    }

    #[test]
    fn snippets_match_whole_dictation_only() {
        let snippets = vec![Snippet { id: 1, trigger: "my address".into(), expansion: "1 Main St".into() }];
        assert_eq!(expand_snippet("My address.", &snippets).as_deref(), Some("1 Main St"));
        assert_eq!(expand_snippet("Insert my address", &snippets).as_deref(), Some("1 Main St"));
        assert_eq!(expand_snippet("I changed my address", &snippets), None);
    }

    #[test]
    fn replacements_respect_word_boundaries() {
        let dict = vec![entry("cat", Some("Kat")), entry("tauri", Some("Tauri")), entry("c++", Some("C++"))];
        assert_eq!(apply_replacements("concatenate the cat", &dict), "concatenate the Kat");
        assert_eq!(apply_replacements("Cat and TAURI", &dict), "Kat and Tauri");
        assert_eq!(apply_replacements("I like c++ a lot", &dict), "I like C++ a lot");
        assert_eq!(apply_replacements("$1 cat", &[entry("cat", Some("$0"))]), "$1 $0");
    }

    #[test]
    fn vocabulary_uses_written_forms() {
        let dict = vec![entry("Svelte", None), entry("hams", Some("Hams"))];
        assert_eq!(vocabulary(&dict), vec!["Svelte".to_string(), "Hams".to_string()]);
    }
}

/// Learn from a user's edit of a dictation: if they changed a short span of
/// words (e.g. "hams app" → "Hams app"), return (heard, correct) pairs to
/// *suggest* as dictionary replacements. Nothing is applied without approval.
pub fn learn_corrections(before: &str, after: &str) -> Vec<(String, String)> {
    fn words(text: &str) -> Vec<String> {
        text.split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'' && c != '+' && c != '#').to_string())
            .filter(|w| !w.is_empty())
            .collect()
    }
    let (a, b) = (words(before), words(after));
    let prefix = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let suffix = a[prefix..]
        .iter()
        .rev()
        .zip(b[prefix..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let (old, new) = (&a[prefix..a.len() - suffix], &b[prefix..b.len() - suffix]);
    if old.is_empty() || new.is_empty() || old.len() > 3 || new.len() > 3 {
        return Vec::new();
    }
    // A rewrite of a short sentence isn't a vocabulary fix.
    if a.len() <= 3 && old.len() == a.len() && !old.iter().zip(new).all(|(x, y)| x.eq_ignore_ascii_case(y)) {
        return Vec::new();
    }
    vec![(old.join(" "), new.join(" "))]
}

#[cfg(test)]
mod learning_tests {
    use super::learn_corrections;

    #[test]
    fn learns_short_word_fixes() {
        assert_eq!(
            learn_corrections("I pushed the hams app today.", "I pushed the Hams app today."),
            vec![("hams".to_string(), "Hams".to_string())]
        );
        assert_eq!(
            learn_corrections("Ask jon doe about it", "Ask Jon Doh about it"),
            vec![("jon doe".to_string(), "Jon Doh".to_string())]
        );
    }

    #[test]
    fn ignores_rewrites_and_punctuation() {
        assert!(learn_corrections("Hello there.", "Hello there!").is_empty());
        assert!(learn_corrections("The quick brown fox jumps", "A totally different sentence here now").is_empty());
        assert!(learn_corrections("ok", "sounds good").is_empty());
        assert!(learn_corrections("same", "same").is_empty());
    }
}
