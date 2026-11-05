//! Windows: low-level keyboard hook for shortcuts, SendInput for paste, and
//! Win32 focus handling. Ported from Hams v1 (`hotkey.rs`, `clipboard.rs`).

mod hotkeys;

use std::ffi::c_void;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{CloseHandle, HWND};
use windows_sys::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL,
    VK_LMENU, VK_LSHIFT, VK_LWIN, VK_RMENU, VK_RSHIFT, VK_RWIN,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowLongPtrW, GetWindowThreadProcessId, IsIconic, SetForegroundWindow, SetWindowLongPtrW,
    SetWindowPos, ShowWindow, GWL_EXSTYLE, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
    SW_RESTORE, SW_SHOWNOACTIVATE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

pub use hotkeys::start_hotkeys;

use super::AppTarget;
use crate::error::{AppError, AppResult};

const VK_V: VIRTUAL_KEY = 0x56;
const VK_C: VIRTUAL_KEY = 0x43;

fn process_exe(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut buffer = [0u16; 1024];
        let mut len = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut len) != 0;
        CloseHandle(handle);
        if !ok {
            return None;
        }
        let path = String::from_utf16_lossy(&buffer[..len as usize]);
        path.rsplit(['\\', '/']).next().map(str::to_string)
    }
}

pub fn frontmost() -> Option<AppTarget> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        let exe = process_exe(pid).unwrap_or_default();
        let name = exe.trim_end_matches(".exe").trim_end_matches(".EXE").to_string();
        Some(AppTarget { pid: pid as i64, id: exe.to_lowercase(), name, window: hwnd as isize })
    }
}

/// Restore focus to the original window. Windows only lets the foreground
/// process change focus, so briefly attach to the foreground thread's input.
pub fn focus(target: &AppTarget) {
    let hwnd = target.window as HWND;
    if hwnd.is_null() {
        return;
    }
    unsafe {
        if GetForegroundWindow() == hwnd {
            return;
        }
        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        }
        let foreground_thread = GetWindowThreadProcessId(GetForegroundWindow(), std::ptr::null_mut());
        let current_thread = GetCurrentThreadId();
        let attached = foreground_thread != 0
            && foreground_thread != current_thread
            && AttachThreadInput(current_thread, foreground_thread, 1) != 0;
        SetForegroundWindow(hwnd);
        if attached {
            AttachThreadInput(current_thread, foreground_thread, 0);
        }
    }
    let deadline = Instant::now() + Duration::from_millis(400);
    while Instant::now() < deadline {
        if unsafe { GetForegroundWindow() } == hwnd {
            std::thread::sleep(Duration::from_millis(30));
            return;
        }
        std::thread::sleep(Duration::from_millis(15));
    }
    tracing::warn!("Could not return focus to {}", target.name);
}

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { 0 }, time: 0, dwExtraInfo: 0 },
        },
    }
}

fn send(inputs: &[INPUT]) -> AppResult<()> {
    let sent = unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err(AppError::Delivery(
            "Windows blocked the paste keystroke (the target app may be running as administrator).".into(),
        ))
    }
}

fn send_ctrl(letter: VIRTUAL_KEY) -> AppResult<()> {
    // Release modifiers the user may still be holding from the shortcut, so the
    // target sees Ctrl+V and not Ctrl+Shift+V.
    let mut inputs = Vec::new();
    for vk in [VK_LSHIFT, VK_RSHIFT, VK_LMENU, VK_RMENU, VK_LWIN, VK_RWIN] {
        if unsafe { GetAsyncKeyState(vk as i32) } as u16 & 0x8000 != 0 {
            inputs.push(key(vk, true));
        }
    }
    inputs.extend([key(VK_CONTROL, false), key(letter, false), key(letter, true), key(VK_CONTROL, true)]);
    send(&inputs)
}

pub fn send_paste() -> AppResult<()> {
    send_ctrl(VK_V)
}

pub fn send_copy() -> AppResult<()> {
    send_ctrl(VK_C)
}

pub fn can_send_keys() -> bool {
    true
}

pub fn request_accessibility() -> bool {
    true
}

pub fn open_permission_settings(kind: &str) {
    let uri = if kind == "microphone" { "ms-settings:privacy-microphone" } else { "ms-settings:privacy" };
    let _ = std::process::Command::new("cmd").args(["/C", "start", "", uri]).spawn();
}

/// UI Automation support is planned; until then command mode uses the Ctrl+C probe.
pub fn selected_text_accessibility() -> Option<String> {
    None
}

fn hwnd_of(window: &tauri::WebviewWindow) -> Option<HWND> {
    window.hwnd().ok().map(|h| h.0 as *mut c_void as HWND)
}

/// The overlay must never take focus from the app being dictated into.
pub fn make_window_non_activating(window: &tauri::WebviewWindow) {
    if let Some(hwnd) = hwnd_of(window) {
        unsafe {
            let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | (WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW) as isize);
        }
    }
}

pub fn show_window_no_activate(window: &tauri::WebviewWindow) {
    if let Some(hwnd) = hwnd_of(window) {
        unsafe {
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW);
        }
    }
}
