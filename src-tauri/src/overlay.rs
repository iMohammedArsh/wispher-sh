#[cfg(target_os = "linux")]
use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};
use tauri::AppHandle;
#[cfg(any(target_os = "windows", target_os = "macos"))]
use tauri::{
    window::Color, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder,
};

/// A Windows-native, non-activating Wispr-style status overlay.
#[derive(Clone)]
pub struct OverlayController {
    #[cfg(target_os = "linux")]
    child: Arc<Mutex<Option<std::process::Child>>>,
    #[cfg(target_os = "linux")]
    snapshot: Arc<Mutex<OverlaySnapshot>>,
    #[cfg(target_os = "linux")]
    state_file: Arc<PathBuf>,
}

/// The WSL application and the Windows WPF overlay cannot reliably use a live stdin
/// stream together. This tiny file is the single source of truth the WPF UI polls.
#[cfg(target_os = "linux")]
#[derive(Clone)]
struct OverlaySnapshot {
    state: String,
    level: f32,
}

#[cfg(target_os = "linux")]
impl Default for OverlaySnapshot {
    fn default() -> Self {
        Self {
            state: "idle".into(),
            level: 0.0,
        }
    }
}

#[cfg(target_os = "linux")]
impl Default for OverlayController {
    fn default() -> Self {
        Self {
            child: Arc::default(),
            snapshot: Arc::new(Mutex::new(OverlaySnapshot::default())),
            state_file: Arc::new(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .join(".wisper-overlay-state"),
            ),
        }
    }
}

#[cfg(not(target_os = "linux"))]
impl Default for OverlayController {
    fn default() -> Self {
        Self {}
    }
}

impl OverlayController {
    pub fn start() -> Self {
        let controller = Self::default();
        #[cfg(target_os = "linux")]
        if is_wsl() {
            match windows_path(&format!(
                "{}/scripts/wisper-overlay.ps1",
                env!("CARGO_MANIFEST_DIR")
            )) {
                Ok(script) => {
                    stop_previous_wsl_overlay();
                    let state_file =
                        windows_path(&controller.state_file.to_string_lossy()).unwrap_or_default();
                    match Command::new("powershell.exe")
                        .args([
                            "-NoProfile",
                            "-NonInteractive",
                            "-ExecutionPolicy",
                            "Bypass",
                            "-Sta",
                            "-File",
                            &script,
                            "-StateFile",
                            &state_file,
                        ])
                        .stdin(Stdio::null())
                        .stdout(Stdio::piped())
                        .stderr(Stdio::null())
                        .spawn()
                    {
                        Ok(child) => {
                            *controller.child.lock().unwrap() = Some(child);
                            controller.write_snapshot();
                        }
                        Err(error) => {
                            eprintln!("[Overlay] Could not start Windows overlay: {error}")
                        }
                    }
                }
                Err(error) => eprintln!("[Overlay] Could not locate Windows overlay: {error}"),
            }
        }
        controller
    }

    /// Starts the overlay in the same native Windows process. The transparent Tauri window is
    /// topmost, borderless, excluded from the taskbar, and created without requesting focus.
    #[cfg(target_os = "windows")]
    pub fn initialize<F>(&self, app: &AppHandle, _on_toggle: F)
    where
        F: Fn() + Send + 'static,
    {
        if app.get_webview_window("island").is_some() {
            return;
        }
        let result = WebviewWindowBuilder::new(app, "island", WebviewUrl::App("index.html".into()))
            .title("Hams voice status")
            .inner_size(40.0, 8.0)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .background_color(Color(0, 0, 0, 255))
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false)
            // Do not reveal the WebView until its overlay-only CSS has painted. Otherwise
            // Windows can briefly show WebView2's unstyled surface during startup.
            .visible(false)
            .build();
        match result {
            Ok(window) => {
                apply_native_pill_region(&window);
                set_native_opacity(&window, 64);
                if let (Ok(size), Ok(scale_factor)) = (window.inner_size(), window.scale_factor()) {
                    println!(
                        "[Overlay] Native island created: {}x{} physical px at {:.2}x scale.",
                        size.width, size.height, scale_factor
                    );
                }
                if let Ok(Some(monitor)) = window.primary_monitor() {
                    position_native_window(&window, &monitor, 40, 8);
                }
            }
            Err(error) => eprintln!("[Overlay] Could not create native Windows overlay: {error}"),
        }
    }

    /// Creates a small non-focusable island below the macOS menu bar. It stays out of the Dock
    /// and follows Hams while the main window is hidden.
    #[cfg(target_os = "macos")]
    pub fn initialize<F>(&self, app: &AppHandle, _on_toggle: F)
    where
        F: Fn() + Send + 'static,
    {
        if app.get_webview_window("island").is_some() {
            return;
        }
        let result = WebviewWindowBuilder::new(app, "island", WebviewUrl::App("index.html".into()))
            .title("Hams voice status")
            .inner_size(72.0, 14.0)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .background_color(Color(0, 0, 0, 0))
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false)
            .visible(false)
            .build();
        match result {
            Ok(window) => {
                let _ = window.set_shadow(false);
                if let Ok(Some(monitor)) = window.primary_monitor() {
                    position_macos_window(&window, &monitor, 72, 14);
                }
            }
            Err(error) => eprintln!("[Overlay] Could not create the macOS island: {error}"),
        }
    }

    #[cfg(target_os = "linux")]
    pub fn initialize<F>(&self, app: &AppHandle, on_toggle: F)
    where
        F: Fn() + Send + 'static,
    {
        let stdout = self
            .child
            .lock()
            .unwrap()
            .as_mut()
            .and_then(|child| child.stdout.take());
        if let Some(stdout) = stdout {
            let _ = app;
            std::thread::spawn(move || {
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    if line.trim() == "toggle" {
                        on_toggle();
                    }
                }
            });
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    pub fn initialize<F>(&self, _app: &tauri::AppHandle, _on_toggle: F)
    where
        F: Fn() + Send + 'static,
    {
    }

    pub fn set_state(&self, app: &AppHandle, state: &str) {
        #[cfg(target_os = "linux")]
        {
            let _ = app;
            let mut snapshot = self.snapshot.lock().unwrap();
            snapshot.state = state.to_string();
            if state != "recording" {
                snapshot.level = 0.0;
            }
            drop(snapshot);
            self.write_snapshot();
        }
        #[cfg(target_os = "windows")]
        resize_native_window(app, state);
        #[cfg(target_os = "macos")]
        resize_macos_window(app, state);
        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        let _ = (app, state);
    }

    /// Reveal the Windows idle indicator only after the renderer has applied its overlay style.
    #[cfg(target_os = "windows")]
    pub fn reveal_when_ready(&self, app: &AppHandle) {
        if let Some(window) = app.get_webview_window("island") {
            let _ = window.show();
        }
    }

    /// On macOS the idle overlay stays hidden. Active states reveal it from
    /// `resize_macos_window`, avoiding a permanent strip over the user's workspace.
    #[cfg(not(target_os = "windows"))]
    pub fn reveal_when_ready(&self, _app: &AppHandle) {}

    /// The macOS webview calls this after React has committed the matching UI.
    /// Keeping presentation renderer-led prevents an idle frame from appearing
    /// inside a recording-sized native window while status events are in flight.
    pub fn present_renderer_state(&self, app: &AppHandle, state: &str) {
        #[cfg(target_os = "macos")]
        resize_macos_window(app, state);
        #[cfg(not(target_os = "macos"))]
        let _ = (app, state);
    }

    pub fn set_level(&self, rms_amplitude: f32) {
        #[cfg(target_os = "linux")]
        {
            let mut snapshot = self.snapshot.lock().unwrap();
            if snapshot.state != "recording" {
                return;
            }
            snapshot.level = rms_amplitude.clamp(0.0, 1.0);
            drop(snapshot);
            self.write_snapshot();
        }
        #[cfg(not(target_os = "linux"))]
        let _ = rms_amplitude;
    }

    #[cfg(target_os = "linux")]
    fn write_snapshot(&self) {
        let snapshot = self.snapshot.lock().unwrap();
        let contents = format!("{}|{:.4}", snapshot.state, snapshot.level);
        if let Err(error) = std::fs::write(&*self.state_file, contents) {
            eprintln!("[Overlay] Could not update overlay state: {error}");
        }
    }
}

#[cfg(target_os = "macos")]
fn resize_macos_window(app: &AppHandle, state: &str) {
    let (width, height) = match state {
        "recording" => (252, 50),
        "processing" => (224, 50),
        "error" => (300, 58),
        _ => {
            hide_macos_window(app);
            return;
        }
    };
    if let Some(window) = app.get_webview_window("island") {
        let _ = window.set_size(LogicalSize::new(width as f64, height as f64));
        if let Ok(Some(monitor)) = window.primary_monitor() {
            position_macos_window(&window, &monitor, width, height);
        }
        let _ = window.show();
    }
}

#[cfg(target_os = "macos")]
fn hide_macos_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("island") {
        let _ = window.hide();
    }
}

#[cfg(target_os = "macos")]
fn position_macos_window(
    window: &tauri::WebviewWindow,
    monitor: &tauri::Monitor,
    width: u32,
    _height: u32,
) {
    let work_area = monitor.work_area();
    let scale_factor = monitor.scale_factor();
    let physical_width = (width as f64 * scale_factor).round() as i32;
    let _ = window.set_position(PhysicalPosition::new(
        work_area.position.x + (work_area.size.width as i32 - physical_width) / 2,
        work_area.position.y + (10.0 * scale_factor).round() as i32,
    ));
}

#[cfg(target_os = "windows")]
fn resize_native_window(app: &AppHandle, state: &str) {
    let (width, height) = match state {
        "recording" => (102, 28),
        "processing" => (102, 28),
        "error" => (190, 44),
        _ => (40, 8),
    };
    if let Some(window) = app.get_webview_window("island") {
        // Tauri's webview dimensions are logical pixels. Passing a PhysicalSize here made
        // the overlay half-sized on high-DPI displays (for example, 200% Windows scaling).
        let _ = window.set_size(LogicalSize::new(width as f64, height as f64));
        apply_native_pill_region(&window);
        set_native_opacity(&window, if state == "idle" { 64 } else { 255 });
        if let (Ok(size), Ok(scale_factor)) = (window.inner_size(), window.scale_factor()) {
            println!(
                "[Overlay] State {state}: {}x{} physical px at {:.2}x scale.",
                size.width, size.height, scale_factor
            );
        }
        if let Ok(Some(monitor)) = window.primary_monitor() {
            position_native_window(&window, &monitor, width, height);
        }
    }
}

/// WebView2 does not reliably composite CSS alpha over a transparent native surface.
/// Apply opacity to the clipped HWND instead, so the idle pill stays black while Windows
/// blends it with the app beneath it.
#[cfg(target_os = "windows")]
fn set_native_opacity(window: &tauri::WebviewWindow, alpha: u8) {
    use windows_sys::Win32::{
        Foundation::COLORREF,
        UI::WindowsAndMessaging::{SetLayeredWindowAttributes, LWA_ALPHA},
    };

    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    let result = unsafe { SetLayeredWindowAttributes(hwnd.0, 0 as COLORREF, alpha, LWA_ALPHA) };
    if result == 0 {
        eprintln!("[Overlay] Could not set native window opacity.");
    }
}

/// Clip the actual Windows window, not merely its HTML content. This removes the
/// rectangular WebView2 background that otherwise appears around rounded CSS.
#[cfg(target_os = "windows")]
fn apply_native_pill_region(window: &tauri::WebviewWindow) {
    use windows_sys::Win32::{
        Foundation::{POINT, RECT},
        Graphics::Gdi::{ClientToScreen, CreateRoundRectRgn, SetWindowRgn},
        UI::WindowsAndMessaging::{GetClientRect, GetWindowRect},
    };

    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    let mut client_rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let mut window_rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let mut client_origin = POINT { x: 0, y: 0 };
    let geometry_available = unsafe {
        GetClientRect(hwnd.0, &mut client_rect) != 0
            && GetWindowRect(hwnd.0, &mut window_rect) != 0
            && ClientToScreen(hwnd.0, &mut client_origin) != 0
    };
    if !geometry_available {
        return;
    }

    // The WebView client area is offset inside the native HWND by invisible
    // Windows resize margins. Apply the region to the client rectangle so the
    // black web content and the native rounded shape align exactly.
    let x = client_origin.x - window_rect.left;
    let y = client_origin.y - window_rect.top;
    let width = client_rect.right - client_rect.left;
    let height = client_rect.bottom - client_rect.top;
    if width <= 0 || height <= 0 {
        return;
    }

    // SetWindowRgn takes ownership of a successful region handle.
    let region = unsafe { CreateRoundRectRgn(x, y, x + width + 1, y + height + 1, height, height) };
    if region.is_null() {
        return;
    }
    let result = unsafe { SetWindowRgn(hwnd.0, region, 1) };
    if result == 0 {
        // The system did not take ownership on failure; leaking a tiny region is
        // preferable to destroying a handle that Windows might have accepted.
        eprintln!("[Overlay] Could not apply rounded native window region.");
    }
}

#[cfg(target_os = "linux")]
impl Drop for OverlayController {
    fn drop(&mut self) {
        if Arc::strong_count(&self.child) != 1 {
            return;
        }
        if let Some(mut child) = self.child.lock().unwrap().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(target_os = "windows")]
fn position_native_window(
    window: &tauri::WebviewWindow,
    monitor: &tauri::Monitor,
    width: u32,
    height: u32,
) {
    // Windows reports the usable work area with the taskbar excluded. Positioning
    // from the full monitor height can place a topmost overlay over the taskbar.
    let work_area = monitor.work_area();
    let scale_factor = monitor.scale_factor();
    let physical_width = (width as f64 * scale_factor).round() as i32;
    let physical_height = (height as f64 * scale_factor).round() as i32;
    let _ = window.set_position(PhysicalPosition::new(
        work_area.position.x + (work_area.size.width as i32 - physical_width) / 2,
        // A visible gap keeps the capsule hovering above the taskbar rather than touching it.
        work_area.position.y + work_area.size.height as i32 - physical_height - 18,
    ));
}

#[cfg(target_os = "linux")]
fn is_wsl() -> bool {
    std::env::var_os("WSL_INTEROP").is_some()
        || std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .map(|release| release.to_ascii_lowercase().contains("microsoft"))
            .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn stop_previous_wsl_overlay() {
    let _ = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-CimInstance Win32_Process -Filter \"Name = 'powershell.exe'\" | Where-Object { $_.CommandLine -like '*wisper-overlay.ps1*' } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force }",
        ])
        .status();
}

#[cfg(target_os = "linux")]
fn windows_path(path: &str) -> Result<String, String> {
    let bytes = path.as_bytes();
    if bytes.len() < 7 || !path.starts_with("/mnt/") {
        return Err(path.into());
    }
    Ok(format!(
        "{}:{}",
        (bytes[5] as char).to_ascii_uppercase(),
        path[6..].replace('/', "\\")
    ))
}
