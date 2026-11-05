//! Clipboard helpers with save/restore so pasting doesn't destroy what the user
//! had copied. Text only: arboard can't round-trip every rich format.

use std::time::Duration;

use crate::error::{AppError, AppResult};

pub struct Snapshot(Option<String>);

fn clipboard() -> AppResult<arboard::Clipboard> {
    arboard::Clipboard::new().map_err(|e| AppError::Delivery(format!("clipboard unavailable: {e}")))
}

pub fn set_text(text: &str) -> AppResult<()> {
    clipboard()?
        .set_text(text.to_string())
        .map_err(|e| AppError::Delivery(format!("could not copy: {e}")))
}

pub fn get_text() -> Option<String> {
    clipboard().ok()?.get_text().ok()
}

pub fn snapshot() -> Snapshot {
    Snapshot(get_text())
}

pub fn restore_now(snapshot: Snapshot) {
    if let Ok(mut cb) = clipboard() {
        let _ = match snapshot.0 {
            Some(text) => cb.set_text(text),
            None => cb.clear(),
        };
    }
}

/// Restore after the target app has had time to read the pasted text, but only
/// if the clipboard still holds our transcript (the user may have copied since).
pub fn restore_later(snapshot: Snapshot, pasted: String) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(450));
        if get_text().as_deref() == Some(pasted.as_str()) {
            restore_now(snapshot);
        }
    });
}
