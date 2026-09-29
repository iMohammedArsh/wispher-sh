use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};
#[cfg(target_os = "linux")]
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

#[cfg(target_os = "windows")]
use std::sync::{
    atomic::AtomicBool,
    mpsc::{self, Sender},
    Mutex, OnceLock,
};
#[cfg(target_os = "macos")]
use std::sync::atomic::AtomicBool;
#[cfg(target_os = "macos")]
use rdev::{listen, Event, EventType, Key};
#[cfg(target_os = "windows")]
use windows_sys::Win32::{
    Foundation::{GetLastError, LPARAM, LRESULT, WPARAM},
    UI::{
        Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_MENU, VK_SHIFT, VK_SPACE},
        WindowsAndMessaging::{
            CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
            UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP,
            WM_SYSKEYDOWN, WM_SYSKEYUP,
        },
    },
};

/// Receives global keyboard shortcuts without taking focus from the active application.
pub struct HotkeyListener;

impl HotkeyListener {
    pub fn start_listener<FStart, FStop, FError>(on_start: FStart, on_stop: FStop, on_error: FError)
    where
        FStart: Fn(bool) + Send + Sync + 'static,
        FStop: Fn() + Send + Sync + 'static,
        FError: Fn(String) + Send + Sync + 'static,
    {
        let start = Arc::new(on_start);
        let stop = Arc::new(on_stop);
        let error = Arc::new(on_error);
        std::thread::spawn(move || {
            #[cfg(target_os = "windows")]
            {
                if let Err(message) = listen_natively_on_windows(start, stop) {
                    error(message);
                }
                return;
            }

            #[cfg(target_os = "linux")]
            if is_wsl() {
                if let Err(message) = listen_on_windows_host(start, stop, error.clone()) {
                    error(message);
                }
                return;
            }

            #[cfg(target_os = "macos")]
            {
                if let Err(message) = listen_natively_on_macos(start, stop) {
                    error(message);
                }
                return;
            }

            #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
            error(
                "Background shortcuts are not supported on this platform."
                    .into(),
            );
        });
    }
}

#[cfg(target_os = "macos")]
fn listen_natively_on_macos<FStart, FStop>(
    on_start: Arc<FStart>,
    on_stop: Arc<FStop>,
) -> Result<(), String>
where
    FStart: Fn(bool) + Send + Sync + 'static,
    FStop: Fn() + Send + Sync + 'static,
{
    let command_down = Arc::new(AtomicBool::new(false));
    let shift_down = Arc::new(AtomicBool::new(false));
    let option_down = Arc::new(AtomicBool::new(false));
    let space_down = Arc::new(AtomicBool::new(false));
    let toggle_latched = Arc::new(AtomicBool::new(false));
    let mode = Arc::new(AtomicU8::new(0)); // 0 idle, 1 push-to-talk, 2 hands-free

    println!(
        "[Hotkey] macOS shortcuts ready: hold Cmd+Shift+Space; toggle Cmd+Shift+Option."
    );
    listen(move |event: Event| {
        let (key, is_press, is_release) = match event.event_type {
            EventType::KeyPress(key) => (key, true, false),
            EventType::KeyRelease(key) => (key, false, true),
            _ => return,
        };

        match key {
            Key::MetaLeft | Key::MetaRight => command_down.store(is_press, Ordering::SeqCst),
            Key::ShiftLeft | Key::ShiftRight => shift_down.store(is_press, Ordering::SeqCst),
            Key::Alt | Key::AltGr => option_down.store(is_press, Ordering::SeqCst),
            Key::Space => {
                if is_press
                    && command_down.load(Ordering::SeqCst)
                    && shift_down.load(Ordering::SeqCst)
                    && !option_down.load(Ordering::SeqCst)
                    && !space_down.swap(true, Ordering::SeqCst)
                    && mode
                        .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
                        .is_ok()
                {
                    on_start(false);
                } else if is_release && space_down.swap(false, Ordering::SeqCst) {
                    if mode
                        .compare_exchange(1, 0, Ordering::SeqCst, Ordering::SeqCst)
                        .is_ok()
                    {
                        on_stop();
                    }
                }
            }
            _ => {}
        }

        let toggle_chord = command_down.load(Ordering::SeqCst)
            && shift_down.load(Ordering::SeqCst)
            && option_down.load(Ordering::SeqCst);
        if toggle_chord && !toggle_latched.swap(true, Ordering::SeqCst) {
            match mode.load(Ordering::SeqCst) {
                0 if mode
                    .compare_exchange(0, 2, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok() => on_start(true),
                2 if mode
                    .compare_exchange(2, 0, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok() => on_stop(),
                _ => {}
            }
        } else if !toggle_chord && is_release {
            toggle_latched.store(false, Ordering::SeqCst);
        }
    })
    .map_err(|error| {
        format!(
            "macOS could not start global shortcuts: {error:?}. Allow Hams in System Settings → Privacy & Security → Input Monitoring, then restart Hams."
        )
    })
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
enum WindowsHotkeyEvent {
    PushDown,
    PushUp,
    Toggle,
}

#[cfg(target_os = "windows")]
type WindowsHotkeyMessage = Result<WindowsHotkeyEvent, String>;
#[cfg(target_os = "windows")]
static WINDOWS_HOTKEY_SENDER: OnceLock<Mutex<Option<Sender<WindowsHotkeyMessage>>>> =
    OnceLock::new();
#[cfg(target_os = "windows")]
static WINDOWS_PUSH_HELD: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "windows")]
static WINDOWS_TOGGLE_HELD: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "windows")]
fn listen_natively_on_windows<FStart, FStop>(
    on_start: Arc<FStart>,
    on_stop: Arc<FStop>,
) -> Result<(), String>
where
    FStart: Fn(bool) + Send + Sync + 'static,
    FStop: Fn() + Send + Sync + 'static,
{
    let (sender, receiver) = mpsc::channel();
    let slot = WINDOWS_HOTKEY_SENDER.get_or_init(|| Mutex::new(None));
    *slot.lock().unwrap() = Some(sender);

    std::thread::spawn(move || unsafe {
        let hook: HHOOK = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(windows_keyboard_proc),
            std::ptr::null_mut(),
            0,
        );
        if hook == std::ptr::null_mut() {
            let error_code = GetLastError();
            if let Some(slot) = WINDOWS_HOTKEY_SENDER.get() {
                if let Ok(mut sender) = slot.lock() {
                    if let Some(sender) = sender.take() {
                        let _ = sender.send(Err(format!(
                            "Windows could not install the global hotkey listener (error code {error_code})."
                        )));
                    }
                }
            }
            return;
        }
        println!(
            "[Hotkey] Native Windows shortcuts ready: hold Ctrl+Shift+Space; toggle Ctrl+Shift+Alt."
        );
        let mut message: MSG = std::mem::zeroed();
        while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        UnhookWindowsHookEx(hook);
        if let Some(slot) = WINDOWS_HOTKEY_SENDER.get() {
            if let Ok(mut sender) = slot.lock() {
                *sender = None;
            }
        }
    });

    let mode = AtomicU8::new(0); // 0 idle, 1 push-to-talk, 2 hands-free
    for message in receiver {
        let event = message?;
        match event {
            WindowsHotkeyEvent::PushDown
                if mode
                    .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok() =>
            {
                println!("[Hotkey] Hold-to-talk detected.");
                on_start(false)
            }
            WindowsHotkeyEvent::PushUp
                if mode
                    .compare_exchange(1, 0, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok() =>
            {
                println!("[Hotkey] Hold-to-talk released.");
                on_stop()
            }
            WindowsHotkeyEvent::Toggle => match mode.load(Ordering::SeqCst) {
                0 if mode
                    .compare_exchange(0, 2, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok() =>
                {
                    println!("[Hotkey] Hands-free recording started.");
                    on_start(true)
                }
                2 if mode
                    .compare_exchange(2, 0, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok() =>
                {
                    println!("[Hotkey] Hands-free recording stopped.");
                    on_stop()
                }
                _ => {}
            },
            _ => {}
        }
    }
    Err("The native Windows global hotkey listener stopped unexpectedly.".into())
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn windows_keyboard_proc(
    code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let key = *(l_param as *const KBDLLHOOKSTRUCT);
        let ctrl_down = GetAsyncKeyState(VK_CONTROL as i32) < 0;
        let shift_down = GetAsyncKeyState(VK_SHIFT as i32) < 0;
        let alt_down = GetAsyncKeyState(VK_MENU as i32) < 0;
        let message = w_param as u32;
        let sender = WINDOWS_HOTKEY_SENDER
            .get()
            .and_then(|slot| slot.lock().ok().and_then(|sender| sender.clone()));

        // Ctrl + Shift + Alt is a modifier-only chord. Windows reports left/right
        // modifier virtual keys as well as generic ones, so include both variants.
        let is_ctrl_modifier = matches!(key.vkCode, 0x11 | 0xA2 | 0xA3);
        let is_shift_modifier = matches!(key.vkCode, 0x10 | 0xA0 | 0xA1);
        let is_alt_modifier = matches!(key.vkCode, 0x12 | 0xA4 | 0xA5);
        let is_toggle_modifier = is_ctrl_modifier || is_shift_modifier || is_alt_modifier;

        if message == WM_KEYDOWN || message == WM_SYSKEYDOWN {
            // Low-level hooks run before Windows updates GetAsyncKeyState for the
            // current key. Count the modifier that caused this event as down too.
            let toggle_chord_down = (ctrl_down || is_ctrl_modifier)
                && (shift_down || is_shift_modifier)
                && (alt_down || is_alt_modifier);
            if is_toggle_modifier && toggle_chord_down {
                if !WINDOWS_TOGGLE_HELD.swap(true, Ordering::SeqCst) {
                    if let Some(sender) = sender {
                        let _ = sender.send(Ok(WindowsHotkeyEvent::Toggle));
                    }
                }
                return 1;
            }

            if key.vkCode == VK_SPACE as u32 && ctrl_down && shift_down && !alt_down {
                WINDOWS_PUSH_HELD.store(true, Ordering::SeqCst);
                if let Some(sender) = sender {
                    let _ = sender.send(Ok(WindowsHotkeyEvent::PushDown));
                }
                return 1;
            }
        }

        if message == WM_KEYUP || message == WM_SYSKEYUP {
            // A new modifier-only chord is allowed only after the previous chord is
            // released. This prevents repeat key events from toggling repeatedly.
            if is_toggle_modifier && WINDOWS_TOGGLE_HELD.swap(false, Ordering::SeqCst) {
                return 1;
            }

            if key.vkCode == VK_SPACE as u32 && WINDOWS_PUSH_HELD.swap(false, Ordering::SeqCst) {
                if let Some(sender) = sender {
                    let _ = sender.send(Ok(WindowsHotkeyEvent::PushUp));
                }
                return 1;
            }
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, w_param, l_param)
}

#[cfg(target_os = "linux")]
fn is_wsl() -> bool {
    std::env::var_os("WSL_INTEROP").is_some()
        || std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .map(|release| release.to_ascii_lowercase().contains("microsoft"))
            .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn listen_on_windows_host<FStart, FStop, FError>(
    on_start: Arc<FStart>,
    on_stop: Arc<FStop>,
    _on_error: Arc<FError>,
) -> Result<(), String>
where
    FStart: Fn(bool) + Send + Sync + 'static,
    FStop: Fn() + Send + Sync + 'static,
    FError: Fn(String) + Send + Sync + 'static,
{
    stop_previous_windows_hotkey_helper();
    let script = windows_path(&format!(
        "{}/scripts/wisper-hotkeys.ps1",
        env!("CARGO_MANIFEST_DIR")
    ))?;
    let mut child = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Sta",
            "-File",
            &script,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not start Windows background hotkeys: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Could not read Windows hotkey events.".to_string())?;
    let mode = AtomicU8::new(0); // 0 idle, 1 push-to-talk, 2 hands-free
    let mut saw_ready = false;

    for line in BufReader::new(stdout).lines() {
        let event =
            line.map_err(|error| format!("Could not read Windows hotkey events: {error}"))?;
        match event.trim() {
            "ready" => {
                saw_ready = true;
                println!("[Hotkey] Windows host shortcuts ready: Ctrl+Shift+Space, Ctrl+Shift+Alt");
            }
            "push_down"
                if mode
                    .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok() =>
            {
                on_start(false)
            }
            "push_up"
                if mode
                    .compare_exchange(1, 0, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok() =>
            {
                on_stop()
            }
            "toggle" => match mode.load(Ordering::SeqCst) {
                0 if mode
                    .compare_exchange(0, 2, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok() =>
                {
                    on_start(true)
                }
                2 if mode
                    .compare_exchange(2, 0, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok() =>
                {
                    on_stop()
                }
                _ => {}
            },
            _ => {}
        }
    }

    let output = child
        .wait_with_output()
        .map_err(|error| format!("Could not finish the Windows hotkey helper: {error}"))?;
    let details = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    Err(if saw_ready {
        format!("Windows background hotkeys stopped unexpectedly: {details}")
    } else {
        format!("Could not enable Windows background hotkeys: {details}")
    })
}

#[cfg(target_os = "linux")]
fn stop_previous_windows_hotkey_helper() {
    // The WSL development process can be stopped without taking its Windows child down.
    // Remove only Wisper's named helper before registering a new global hook, so a stale
    // hook cannot swallow the shortcut before the current app receives it.
    let _ = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-CimInstance Win32_Process -Filter \"Name = 'powershell.exe'\" | Where-Object { $_.CommandLine -like '*wisper-hotkeys.ps1*' } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force }",
        ])
        .status();
}

#[cfg(target_os = "linux")]
fn windows_path(path: &str) -> Result<String, String> {
    let bytes = path.as_bytes();
    if bytes.len() < 7 || !path.starts_with("/mnt/") {
        return Err(format!(
            "Could not convert the WSL path to a Windows path: {path}"
        ));
    }
    Ok(format!(
        "{}:{}",
        (bytes[5] as char).to_ascii_uppercase(),
        path[6..].replace('/', "\\\\")
    ))
}
