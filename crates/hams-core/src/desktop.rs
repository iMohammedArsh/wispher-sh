//! Where dictated text goes. On a computer that's the frontmost app (paste via
//! the clipboard); on a phone it's the text field the keyboard is attached to.

use serde::Serialize;

use crate::error::AppResult;
use crate::settings::DeliveryMode;

/// The app (and on phones, the field) that had focus when recording started.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct AppTarget {
    pub pid: i64,
    /// macOS bundle id ("com.tinyspeck.slackmacgap"), Windows exe ("slack.exe")
    /// or Android package ("com.whatsapp").
    pub id: String,
    pub name: String,
    /// Native window handle (Windows HWND). Not sent to the UI.
    #[serde(skip)]
    pub window: isize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Delivered {
    Pasted,
    Copied,
}

/// Side effects the dictation controller needs. A trait so the controller can
/// be tested without touching the real clipboard, keyboard or text field.
pub trait Desktop: Send + Sync {
    fn frontmost(&self) -> Option<AppTarget>;
    fn deliver(&self, text: &str, target: Option<&AppTarget>, mode: DeliveryMode) -> AppResult<Delivered>;
    fn copy(&self, text: &str) -> AppResult<()>;
    /// Currently selected text in the target app (for command mode).
    /// `allow_copy_fallback` permits a ⌘C/Ctrl+C probe when Accessibility can't
    /// read the selection. Off for code editors, which copy the whole line when
    /// nothing is selected.
    fn selected_text(&self, target: Option<&AppTarget>, allow_copy_fallback: bool) -> Option<String>;
    /// Full text of the focused field in `target` (for opt-in learning from edits).
    /// None when it can't be read or is a password field.
    fn field_text(&self, _target: &AppTarget) -> Option<String> {
        None
    }
}
