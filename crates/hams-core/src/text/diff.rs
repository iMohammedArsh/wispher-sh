//! Word-level diff between what Whisper heard and what Hams wrote, for the
//! Heard / Written pair. A port of `src/lib/diff.ts` (keep the two in step):
//! words are compared without case or punctuation, so "um see you" → "See you."
//! reads as one dropped word, not three changes.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunKind {
    Keep,
    Drop,
    Add,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Run {
    pub kind: RunKind,
    pub words: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WordDiff {
    /// The heard text as runs of kept and dropped words.
    pub heard: Vec<Run>,
    /// The written text as runs of kept and added words.
    pub written: Vec<Run>,
    /// Words dropped from what was heard (fillers, "scratch that", rewrites).
    pub dropped: u32,
    /// Words added or changed in what was written.
    pub added: u32,
}

/// Words, with each line break kept as its own "\n" token so paragraphs survive.
fn tokens(text: &str) -> Vec<String> {
    text.replace("\r\n", "\n")
        .replace('\n', " \n ")
        .split([' ', '\t'])
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

/// Compare words without case or surrounding punctuation.
pub fn word_key(word: &str) -> String {
    word.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase()
}

fn push(runs: &mut Vec<Run>, kind: RunKind, word: &str) {
    match runs.last_mut() {
        Some(last) if last.kind == kind => last.words.push(word.to_string()),
        _ => runs.push(Run { kind, words: vec![word.to_string()] }),
    }
}

pub fn diff_words(heard_text: &str, written_text: &str) -> WordDiff {
    let a = tokens(heard_text);
    let b = tokens(written_text);
    let ka: Vec<String> = a.iter().map(|w| word_key(w)).collect();
    let kb: Vec<String> = b.iter().map(|w| word_key(w)).collect();
    let (n, m) = (a.len(), b.len());
    // Longest common subsequence table, filled from the end.
    let mut lcs = vec![vec![0u16; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if ka[i] == kb[j] && !ka[i].is_empty() {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let mut diff = WordDiff { heard: Vec::new(), written: Vec::new(), dropped: 0, added: 0 };
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && ka[i] == kb[j] && !ka[i].is_empty() {
            push(&mut diff.heard, RunKind::Keep, &a[i]);
            push(&mut diff.written, RunKind::Keep, &b[j]);
            i += 1;
            j += 1;
        } else if j < m && (i >= n || lcs[i][j + 1] >= lcs[i + 1][j]) {
            // A token that is only punctuation ("—", "-") isn't a real addition.
            if kb[j].is_empty() {
                push(&mut diff.written, RunKind::Keep, &b[j]);
            } else {
                push(&mut diff.written, RunKind::Add, &b[j]);
                diff.added += 1;
            }
            j += 1;
        } else {
            if ka[i].is_empty() {
                push(&mut diff.heard, RunKind::Keep, &a[i]);
            } else {
                push(&mut diff.heard, RunKind::Drop, &a[i]);
                diff.dropped += 1;
            }
            i += 1;
        }
    }
    diff
}

/// "2 words removed · 1 fixed", or "" when nothing changed but the punctuation.
pub fn change_summary(diff: &WordDiff) -> String {
    let plural = |n: u32| if n == 1 { "word" } else { "words" };
    let mut parts = Vec::new();
    if diff.dropped > 0 {
        parts.push(format!("{} {} removed", diff.dropped, plural(diff.dropped)));
    }
    if diff.added > 0 {
        parts.push(format!("{} {} changed or added", diff.added, plural(diff.added)));
    }
    parts.join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mirrors src/lib/diff.test.ts.

    fn text(runs: &[Run], kind: RunKind) -> Vec<String> {
        runs.iter().filter(|r| r.kind == kind).map(|r| r.words.join(" ")).collect()
    }

    #[test]
    fn fillers_are_dropped_ignoring_case_and_punctuation() {
        let d = diff_words("um so see you tomorrow", "So, see you tomorrow.");
        assert_eq!(text(&d.heard, RunKind::Drop), ["um"]);
        assert!(d.written.iter().all(|r| r.kind == RunKind::Keep));
        assert_eq!((d.dropped, d.added), (1, 0));
    }

    #[test]
    fn a_dictionary_fix_is_a_drop_and_an_add() {
        let d = diff_words("see you on tory tomorrow", "See you on Tauri tomorrow");
        assert_eq!(text(&d.heard, RunKind::Drop), ["tory"]);
        assert_eq!(text(&d.written, RunKind::Add), ["Tauri"]);
        assert_eq!(change_summary(&d), "1 word removed · 1 word changed or added");
    }

    #[test]
    fn runs_stay_together_and_empty_sides_work() {
        assert_eq!(
            diff_words("", "Hello there").written,
            [Run { kind: RunKind::Add, words: vec!["Hello".into(), "there".into()] }]
        );
        assert_eq!(
            diff_words("hello there", "").heard,
            [Run { kind: RunKind::Drop, words: vec!["hello".into(), "there".into()] }]
        );
        assert_eq!(change_summary(&diff_words("same words", "Same words.")), "");
    }

    #[test]
    fn punctuation_only_tokens_are_not_words() {
        assert_eq!(diff_words("first second", "First — second").added, 0);
    }

    #[test]
    fn line_breaks_are_tokens_that_never_count() {
        let d = diff_words("hi priya thanks", "Hi Priya,\n\nThanks");
        assert_eq!(d.added, 0);
        let words: Vec<&str> = d.written.iter().flat_map(|r| r.words.iter().map(String::as_str)).collect();
        assert_eq!(words, ["Hi", "Priya,", "\n", "\n", "Thanks"]);
    }

    #[test]
    fn non_latin_words_compare_by_letters() {
        // Malayalam: the key keeps the letters, so a case/punctuation change is a keep.
        let d = diff_words("നന്ദി", "നന്ദി.");
        assert_eq!((d.dropped, d.added), (0, 0));
    }
}
