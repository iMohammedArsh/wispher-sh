//! The small always-on-top status pill. It never takes keyboard focus, so the
//! app you're dictating into stays focused while you speak.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::controller::{Phase, Status};

pub const LABEL: &str = "overlay";

static GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn create(app: &AppHandle) {
    if app.get_webview_window(LABEL).is_some() {
        return;
    }
    let builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("overlay.html".into()))
        .title("Hams")
        .inner_size(220.0, 48.0)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .skip_taskbar(true)
        .focused(false)
        .visible(false);
    match builder.build() {
        Ok(window) => platform_setup(&window),
        Err(error) => tracing::error!("Could not create the overlay window: {error}"),
    }
}

/// The pill keeps one size through recording, working and done, so it never
/// jumps between phases. The live caption and errors need more room.
const PILL: (f64, f64) = (232.0, 48.0);

fn size_for(status: &Status) -> Option<(f64, f64)> {
    match status.phase {
        Phase::Idle => None,
        Phase::Recording if status.preview => Some((440.0, 112.0)),
        Phase::Recording | Phase::Transcribing | Phase::Polishing | Phase::Delivering | Phase::Done => Some(PILL),
        Phase::Error => Some((380.0, 64.0)),
    }
}

/// Show, resize or hide the pill for a status. Done and Error hide themselves
/// after a moment unless a newer status arrives first.
pub fn present(app: &AppHandle, status: &Status) {
    let Some(window) = app.get_webview_window(LABEL) else { return };
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let Some((width, height)) = size_for(status) else {
        let _ = window.hide();
        return;
    };
    let _ = window.set_size(LogicalSize::new(width, height));
    position(app, &window, width, height);
    show_without_focus(app, &window);

    let linger = match status.phase {
        Phase::Done => Some(Duration::from_millis(900)),
        Phase::Error => Some(Duration::from_secs(5)),
        _ => None,
    };
    if let Some(linger) = linger {
        let window = window.clone();
        std::thread::spawn(move || {
            std::thread::sleep(linger);
            if GENERATION.load(Ordering::SeqCst) == generation {
                let _ = window.hide();
            }
        });
    }
}

/// Bottom-center of the screen the mouse is on, above the Dock / taskbar.
fn position(app: &AppHandle, window: &WebviewWindow, width: f64, height: f64) {
    let monitors = window.available_monitors().unwrap_or_default();
    let cursor = app.cursor_position().ok();
    let monitor = cursor
        .and_then(|c| {
            monitors.iter().find(|m| {
                let (p, s) = (m.position(), m.size());
                c.x >= p.x as f64 && c.x < (p.x + s.width as i32) as f64 && c.y >= p.y as f64 && c.y < (p.y + s.height as i32) as f64
            })
        })
        .cloned()
        .or_else(|| window.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let (w, h) = ((width * scale).round() as i32, (height * scale).round() as i32);
    let x = area.position.x + (area.size.width as i32 - w) / 2;
    let y = area.position.y + area.size.height as i32 - h - (20.0 * scale).round() as i32;
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

#[cfg(target_os = "macos")]
fn platform_setup(window: &WebviewWindow) {
    use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};
    let window = window.clone();
    let _ = window.clone().run_on_main_thread(move || {
        let Ok(ns) = window.ns_window() else { return };
        // SAFETY: Tauri guarantees this is a live NSWindow, and we're on the main thread.
        let ns: &NSWindow = unsafe { &*(ns as *const NSWindow) };
        ns.setLevel(25); // NSStatusWindowLevel: above normal windows and full-screen apps
        ns.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
        ns.setHidesOnDeactivate(false);
    });
}

#[cfg(target_os = "macos")]
fn show_without_focus(_app: &AppHandle, window: &WebviewWindow) {
    use objc2_app_kit::NSWindow;
    let target = window.clone();
    let _ = window.run_on_main_thread(move || {
        let Ok(ns) = target.ns_window() else { return };
        // orderFrontRegardless shows the window without making Hams the active
        // app, unlike `show()`, which would steal focus from the text field.
        let ns: &NSWindow = unsafe { &*(ns as *const NSWindow) };
        ns.orderFrontRegardless();
    });
}

#[cfg(target_os = "windows")]
fn platform_setup(window: &WebviewWindow) {
    crate::platform::make_window_non_activating(window);
}

#[cfg(target_os = "windows")]
fn show_without_focus(_app: &AppHandle, window: &WebviewWindow) {
    crate::platform::show_window_no_activate(window);
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn platform_setup(_window: &WebviewWindow) {}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn show_without_focus(_app: &AppHandle, window: &WebviewWindow) {
    let _ = window.show();
}
