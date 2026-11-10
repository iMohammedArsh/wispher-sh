//! OS integration behind small, testable seams. Each OS implements the same
//! functions; everything above this module is platform-neutral.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::Serialize;
use tokio::sync::mpsc::UnboundedSender;

use crate::error::AppResult;
use crate::input::{HotkeyAction, Matcher};
use crate::settings::DeliveryMode;

mod clipboard;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
use macos as native;
#[cfg(target_os = "windows")]
use windows as native;
#[cfg(target_os = "windows")]
pub use windows::{make_window_non_activating, show_window_no_activate};

pub use hams_core::desktop::{AppTarget, Delivered, Desktop};

pub struct NativeDesktop;

impl Desktop for NativeDesktop {
    fn frontmost(&self) -> Option<AppTarget> {
        native::frontmost()
    }

    fn deliver(&self, text: &str, target: Option<&AppTarget>, mode: DeliveryMode) -> AppResult<Delivered> {
        if mode == DeliveryMode::CopyOnly {
            clipboard::set_text(text)?;
            return Ok(Delivered::Copied);
        }
        if !native::can_send_keys() {
            clipboard::set_text(text)?;
            return Err(crate::error::AppError::Permission(
                "Hams needs Accessibility permission to paste. The text is on your clipboard.".into(),
            ));
        }
        let saved = (mode == DeliveryMode::PasteRestore).then(clipboard::snapshot);
        clipboard::set_text(text)?;
        if let Some(target) = target {
            native::focus(target);
        }
        native::send_paste()?;
        if let Some(saved) = saved {
            clipboard::restore_later(saved, text.to_string());
        }
        Ok(Delivered::Pasted)
    }

    fn copy(&self, text: &str) -> AppResult<()> {
        clipboard::set_text(text)
    }

    fn selected_text(&self, target: Option<&AppTarget>, allow_copy_fallback: bool) -> Option<String> {
        if let Some(text) = native::selected_text_accessibility() {
            return Some(text).filter(|t| !t.trim().is_empty());
        }
        if !allow_copy_fallback || !native::can_send_keys() {
            return None;
        }
        // Fallback: copy the selection, read it, and put the clipboard back.
        let saved = clipboard::snapshot();
        let marker = format!("hams-selection-probe-{}", crate::store::now_ms());
        clipboard::set_text(&marker).ok()?;
        if let Some(target) = target {
            native::focus(target);
        }
        native::send_copy().ok()?;
        let mut selection = None;
        for _ in 0..12 {
            std::thread::sleep(std::time::Duration::from_millis(25));
            match clipboard::get_text() {
                Some(text) if text != marker => {
                    selection = Some(text);
                    break;
                }
                _ => {}
            }
        }
        clipboard::restore_now(saved);
        selection.filter(|t| !t.trim().is_empty())
    }

    fn field_text(&self, target: &AppTarget) -> Option<String> {
        native::field_text(target)
    }
}

/// Start the global keyboard listener. `matcher` can be updated later (new
/// shortcuts from Settings) without restarting the listener.
pub fn start_hotkeys(
    matcher: Arc<Mutex<Matcher>>,
    recording: Arc<AtomicBool>,
    actions: UnboundedSender<HotkeyAction>,
) -> AppResult<()> {
    native::start_hotkeys(matcher, recording, actions)
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct Permissions {
    /// Needed for the global shortcut, paste, and reading selected text.
    pub accessibility: bool,
    /// Whether this OS has permission prompts at all.
    pub applicable: bool,
}

pub fn permissions() -> Permissions {
    Permissions { accessibility: native::can_send_keys(), applicable: cfg!(target_os = "macos") }
}

/// Open the OS settings page for a permission ("accessibility" or "microphone").
pub fn open_permission_settings(kind: &str) {
    native::open_permission_settings(kind)
}

/// Ask the OS to show its permission prompt (macOS Accessibility).
pub fn request_accessibility() -> bool {
    native::request_accessibility()
}
