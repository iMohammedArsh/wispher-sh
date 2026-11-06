//! Automatic updates from GitHub Releases (`latest.json`, see `scripts/release-macos.sh`).
//! Every download is checked against the public key in `tauri.conf.json` before
//! it's installed, so only builds signed with the release key can replace Hams.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use parking_lot::Mutex;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Give the app time to settle (and the network to come up after login).
const FIRST_CHECK: Duration = Duration::from_secs(30);
const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

/// The update found by the last check, kept so "Install" doesn't ask again.
#[derive(Default)]
pub struct Updates {
    pending: Mutex<Option<Update>>,
    installing: AtomicBool,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    pub notes: String,
}

#[derive(Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct UpdateProgress {
    downloaded: u64,
    total: Option<u64>,
}

fn updater_error(error: tauri_plugin_updater::Error) -> AppError {
    AppError::Other(format!("update check failed: {error}"))
}

/// Ask the release server for a newer version. Emits `update-available` when there is one.
pub async fn check(app: &AppHandle) -> AppResult<Option<UpdateInfo>> {
    let update = app.updater().map_err(updater_error)?.check().await.map_err(updater_error)?;
    let info = update.as_ref().map(|u| UpdateInfo {
        version: u.version.clone(),
        current_version: u.current_version.clone(),
        notes: u.body.clone().unwrap_or_default(),
    });
    *app.state::<Updates>().pending.lock() = update;
    if let Some(info) = &info {
        tracing::info!("Hams {} is available", info.version);
        let _ = app.emit("update-available", info);
    }
    Ok(info)
}

/// Download, verify and install the pending update, then restart into it.
pub async fn install(app: &AppHandle) -> AppResult<()> {
    let updates = app.state::<Updates>();
    let update = updates.pending.lock().clone().ok_or_else(|| AppError::Other("no update is waiting".into()))?;
    if updates.installing.swap(true, Ordering::AcqRel) {
        return Err(AppError::Other("the update is already being installed".into()));
    }
    let mut downloaded = 0u64;
    let result = update
        .download_and_install(
            |chunk, total| {
                downloaded += chunk as u64;
                let _ = app.emit("update-progress", UpdateProgress { downloaded, total });
            },
            || tracing::info!("Update downloaded; installing"),
        )
        .await;
    if let Err(error) = result {
        updates.installing.store(false, Ordering::Release);
        return Err(AppError::Other(format!("the update couldn't be installed: {error}")));
    }
    tracing::info!("Updated to {}; restarting", update.version);
    app.restart();
}

/// Check shortly after launch and then every few hours, if the user allows it.
/// Debug builds never check: their version isn't a release.
pub fn spawn_checks(app: AppHandle) {
    if cfg!(debug_assertions) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK).await;
        loop {
            if app.state::<AppState>().settings.get().check_updates {
                if let Err(error) = check(&app).await {
                    // Offline, or no release published yet: try again later, quietly.
                    tracing::debug!("{error}");
                }
            }
            tokio::time::sleep(CHECK_EVERY).await;
        }
    });
}
