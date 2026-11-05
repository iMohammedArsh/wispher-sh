//! The real `Ui`: Tauri events for the webviews plus overlay window control.

use parking_lot::Mutex;
use tauri::{AppHandle, Emitter};

use crate::controller::{Status, Ui};
use crate::overlay;

pub struct TauriUi {
    app: AppHandle,
    last: Mutex<Status>,
}

impl TauriUi {
    pub fn new(app: AppHandle) -> Self {
        Self { app, last: Mutex::new(Status::default()) }
    }

    pub fn last(&self) -> Status {
        self.last.lock().clone()
    }
}

impl Ui for TauriUi {
    fn status(&self, status: &Status) {
        *self.last.lock() = status.clone();
        let _ = self.app.emit("status", status);
        overlay::present(&self.app, status);
    }

    fn level(&self, level: f32) {
        let _ = self.app.emit_to(overlay::LABEL, "level", level);
    }

    fn data_changed(&self) {
        let _ = self.app.emit("data-changed", ());
    }
}
