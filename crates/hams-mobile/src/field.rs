//! Fitting dictated text into the field under the cursor. Pure functions, shared
//! by every phone keyboard and tested here rather than in Kotlin and Swift twice.

/// What kind of field the keyboard is attached to (from Android's `inputType`
/// or iOS's `keyboardType` / `textContentType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FieldKind {
    Text,
    /// Single-line text where Enter sends (chat boxes, search).
    ShortText,
    Email,
    Url,
    Number,
    Phone,
    /// Never dictated into.
    Password,
}

impl FieldKind {
    /// Fields where AI polish would only hurt: addresses and numbers are
    /// written as heard, without sentence punctuation.
    pub fn is_literal(self) -> bool {
        matches!(self, FieldKind::Email | FieldKind::Url | FieldKind::Number | FieldKind::Phone)
    }
}

/// Reshape `text` for the field: an address has no spaces and no closing
/// period, a phone number keeps only what dials.
pub fn format_for_field(kind: FieldKind, text: &str) -> String {
    match kind {
        FieldKind::Email | FieldKind::Url => {
            let spoken = text
                .replace(" at ", "@")
                .replace(" dot ", ".")
                .replace(" slash ", "/")
                .replace(" underscore ", "_")
                .replace(" dash ", "-");
            let joined: String = spoken.split_whitespace().collect::<String>().to_lowercase();
            joined.trim_end_matches(['.', ',', '!', '?']).to_string()
        }
        FieldKind::Number | FieldKind::Phone => {
            let kept: String = text
                .chars()
                .filter(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '(' | ')' | ' ' | '.' | ','))
                .collect();
            kept.trim().trim_end_matches(['.', ',']).to_string()
        }
        FieldKind::Text | FieldKind::ShortText | FieldKind::Password => text.to_string(),
    }
}

/// `text` as it should be inserted after `before` (the characters just before
/// the cursor): a space between words, none after an opening bracket or before
/// closing punctuation, and a capital letter when a new sentence starts.
pub fn join(before: &str, text: &str, kind: FieldKind) -> String {
    if text.is_empty() || kind.is_literal() {
        return text.to_string();
    }
    let last = before.chars().last();
    let first = text.chars().next().unwrap_or(' ');
    let needs_space = match last {
        None => false,
        Some(c) if c.is_whitespace() => false,
        Some('(' | '[' | '{' | '"' | '\'' | '“' | '‘' | '/' | '@' | '#') => false,
        Some(_) => !matches!(first, ',' | '.' | '!' | '?' | ';' | ':' | ')' | ']' | '}' | '\n' | ' '),
    };
    let trimmed = before.trim_end_matches([' ', '\t']);
    let new_sentence = trimmed.is_empty() || trimmed.ends_with(['.', '!', '?', '\n']);
    let body = if new_sentence { capitalize(text) } else { text.to_string() };
    if needs_space {
        format!(" {body}")
    } else {
        body
    }
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(c) if c.is_lowercase() => c.to_uppercase().chain(chars).collect(),
        _ => text.to_string(),
    }
}

/// Length in UTF-16 code units, the unit Android and iOS text APIs count in.
pub fn utf16_len(text: &str) -> u32 {
    text.encode_utf16().count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spaces_between_words_but_not_around_punctuation() {
        assert_eq!(join("Hello", "there.", FieldKind::Text), " there.");
        assert_eq!(join("Hello ", "there.", FieldKind::Text), "there.");
        assert_eq!(join("(", "see below)", FieldKind::Text), "see below)");
        assert_eq!(join("done", ", thanks", FieldKind::Text), ", thanks");
        assert_eq!(join("", "Hi.", FieldKind::Text), "Hi.");
    }

    #[test]
    fn new_sentences_start_with_a_capital() {
        assert_eq!(join("", "send it", FieldKind::Text), "Send it");
        assert_eq!(join("Done. ", "send it", FieldKind::Text), "Send it");
        assert_eq!(join("Done.", "send it", FieldKind::Text), " Send it");
        assert_eq!(join("line\n", "next", FieldKind::Text), "Next");
        assert_eq!(join("I think", "so", FieldKind::Text), " so");
    }

    #[test]
    fn addresses_are_written_literally() {
        assert_eq!(format_for_field(FieldKind::Email, "Rahul at Gmail dot com."), "rahul@gmail.com");
        assert_eq!(format_for_field(FieldKind::Url, "Hams dot wusla dot co"), "hams.wusla.co");
        assert_eq!(join("x", "hams.wusla.co", FieldKind::Url), "hams.wusla.co");
    }

    #[test]
    fn numbers_keep_only_what_dials() {
        assert_eq!(format_for_field(FieldKind::Phone, "+91 98470 12345."), "+91 98470 12345");
        assert_eq!(format_for_field(FieldKind::Number, "It's 42."), "42");
    }

    #[test]
    fn utf16_counts_like_android() {
        assert_eq!(utf16_len("abc"), 3);
        assert_eq!(utf16_len("👍"), 2);
        assert_eq!(utf16_len("നന്ദി"), 5);
    }
}
