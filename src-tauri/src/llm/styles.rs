//! App-aware styles: the frontmost app decides how a dictation is formatted.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(specta::Type))]
pub enum Style {
    Default,
    Messaging,
    Email,
    Code,
    Docs,
}

impl Style {
    pub const ALL: [Style; 5] = [Style::Default, Style::Messaging, Style::Email, Style::Code, Style::Docs];

    pub fn id(self) -> &'static str {
        match self {
            Style::Default => "default",
            Style::Messaging => "messaging",
            Style::Email => "email",
            Style::Code => "code",
            Style::Docs => "docs",
        }
    }

    #[cfg(test)]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.id() == id)
    }

    pub fn label(self) -> &'static str {
        match self {
            Style::Default => "Default",
            Style::Messaging => "Messaging",
            Style::Email => "Email",
            Style::Code => "Code & terminal",
            Style::Docs => "Documents",
        }
    }

    pub fn default_prompt(self) -> &'static str {
        match self {
            Style::Default => "Clear, natural writing.",
            Style::Messaging => "Casual chat message. Keep it short and conversational, sentence case, no sign-off. Omit the final period of a single-sentence message.",
            Style::Email => "Professional email tone with proper paragraphs. Do not invent a greeting or sign-off that wasn't spoken.",
            Style::Code => "Developer context. Preserve code identifiers, file names, CLI commands and symbols exactly; spoken symbols like 'dash dash' or 'dot' become characters when clearly part of code. Minimal punctuation changes.",
            Style::Docs => "Well-structured document prose with complete sentences and paragraphs.",
        }
    }

    /// Code editors and terminals get raw text by default: AI rewrites there
    /// are more likely to break a command than to help.
    pub fn polish_by_default(self) -> bool {
        self != Style::Code
    }

    /// Local, AI-free adjustment applied after formatting.
    pub fn finish(self, text: &str) -> String {
        match self {
            Style::Messaging => {
                let trimmed = text.trim_end();
                let single_sentence = trimmed.matches(['.', '!', '?']).count() <= 1 && !trimmed.contains('\n');
                if single_sentence && trimmed.ends_with('.') && !trimmed.ends_with("...") {
                    trimmed[..trimmed.len() - 1].to_string()
                } else {
                    trimmed.to_string()
                }
            }
            _ => text.to_string(),
        }
    }
}

/// User edits to a built-in style. `None` keeps the built-in value.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
#[cfg_attr(test, derive(specta::Type))]
pub struct StyleOverride {
    pub prompt: Option<String>,
    pub polish: Option<bool>,
}

/// "Always use this style in this app." Matched case-insensitively against the
/// bundle id / exe name or the app's display name.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(test, derive(specta::Type))]
pub struct AppRule {
    pub app: String,
    pub style: Style,
}

impl AppRule {
    fn matches(&self, app_id: &str, app_name: &str) -> bool {
        let wanted = self.app.trim();
        !wanted.is_empty() && (wanted.eq_ignore_ascii_case(app_id) || wanted.eq_ignore_ascii_case(app_name))
    }
}

/// The style a dictation will use, with the user's edits applied.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub style: Style,
    pub prompt: String,
    pub polish: bool,
}

pub const MAX_PROMPT_CHARS: usize = 1_500;

/// Pick and configure the style for an app: user rules first, then detection.
pub fn resolve(
    app: Option<(&str, &str)>,
    app_aware: bool,
    rules: &[AppRule],
    overrides: &BTreeMap<Style, StyleOverride>,
) -> Resolved {
    let style = match app {
        Some((id, name)) if app_aware => {
            rules.iter().find(|r| r.matches(id, name)).map(|r| r.style).unwrap_or_else(|| detect(id, name))
        }
        _ => Style::Default,
    };
    configure(style, overrides)
}

/// A given style with the user's edits applied (used for re-styling from History).
pub fn configure(style: Style, overrides: &BTreeMap<Style, StyleOverride>) -> Resolved {
    let edit = overrides.get(&style);
    let prompt = edit
        .and_then(|o| o.prompt.as_deref())
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .unwrap_or(style.default_prompt())
        .to_string();
    let polish = edit.and_then(|o| o.polish).unwrap_or(style.polish_by_default());
    Resolved { style, prompt, polish }
}

/// Best-guess style from a macOS bundle id or Windows exe name.
pub fn detect(app_id: &str, app_name: &str) -> Style {
    let haystack = format!("{} {}", app_id, app_name).to_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| haystack.contains(n));
    if has(&[
        "slack", "discord", "whatsapp", "telegram", "messages", "ichat", "signal", "teams", "messenger", "wechat", "line.exe", "skype",
    ]) {
        Style::Messaging
    } else if has(&["mail", "outlook", "spark", "airmail", "superhuman", "thunderbird", "mimestream"]) {
        Style::Email
    } else if has(&[
        "vscode", "code.exe", "visual studio", "cursor", "dev.zed", "xcode", "jetbrains", "intellij", "pycharm", "webstorm", "terminal",
        "iterm", "warp", "ghostty", "alacritty", "kitty", "wezterm", "windowsterminal", "powershell", "cmd.exe", "sublime", "nvim",
    ]) {
        Style::Code
    } else if has(&["pages", "microsoft word", "winword", "notion", "obsidian", "bear", "craft", "docs", "textedit", "notes", "ulysses"]) {
        Style::Docs
    } else {
        Style::Default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_apps() {
        assert_eq!(detect("com.tinyspeck.slackmacgap", "Slack"), Style::Messaging);
        assert_eq!(detect("com.apple.mail", "Mail"), Style::Email);
        assert_eq!(detect("com.microsoft.VSCode", "Code"), Style::Code);
        assert_eq!(detect("code.exe", ""), Style::Code);
        assert_eq!(detect("com.apple.Notes", "Notes"), Style::Docs);
        assert_eq!(detect("com.google.Chrome", "Google Chrome"), Style::Default);
    }

    #[test]
    fn user_rules_and_overrides_win() {
        let rules = vec![AppRule { app: "Google Chrome".into(), style: Style::Messaging }];
        let mut overrides = BTreeMap::new();
        overrides.insert(Style::Messaging, StyleOverride { prompt: Some(" Lowercase, no emoji. ".into()), polish: None });
        overrides.insert(Style::Code, StyleOverride { prompt: Some("   ".into()), polish: Some(true) });

        let chrome = resolve(Some(("com.google.Chrome", "Google Chrome")), true, &rules, &overrides);
        assert_eq!(chrome.style, Style::Messaging);
        assert_eq!(chrome.prompt, "Lowercase, no emoji.");
        assert!(chrome.polish);

        let code = resolve(Some(("com.microsoft.VSCode", "Code")), true, &rules, &overrides);
        assert_eq!(code.prompt, Style::Code.default_prompt(), "blank override falls back to the built-in prompt");
        assert!(code.polish, "polish override turns AI on for code");

        let off = resolve(Some(("com.google.Chrome", "Google Chrome")), false, &rules, &overrides);
        assert_eq!(off.style, Style::Default);
        assert_eq!(Style::from_id("docs"), Some(Style::Docs));
    }

    #[test]
    fn messaging_drops_single_final_period() {
        assert_eq!(Style::Messaging.finish("Sounds good."), "Sounds good");
        assert_eq!(Style::Messaging.finish("Sure. See you at 5."), "Sure. See you at 5.");
        assert_eq!(Style::Messaging.finish("Wait..."), "Wait...");
        assert_eq!(Style::Email.finish("Hello."), "Hello.");
    }
}
