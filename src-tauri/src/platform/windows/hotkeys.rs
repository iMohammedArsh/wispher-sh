//! Global shortcuts through a low-level keyboard hook (WH_KEYBOARD_LL) on its
//! own message-loop thread. The hook only matches and sends a message, so it
//! returns well within Windows' hook timeout.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use parking_lot::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, KBDLLHOOKSTRUCT, LLKHF_INJECTED,
    MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use crate::error::{AppError, AppResult};
use crate::input::{HotkeyAction, Key, KeyId, Matcher, ModKey};

struct HookContext {
    matcher: Arc<Mutex<Matcher>>,
    recording: Arc<AtomicBool>,
    actions: UnboundedSender<HotkeyAction>,
}

static CONTEXT: OnceLock<HookContext> = OnceLock::new();

pub fn start_hotkeys(
    matcher: Arc<Mutex<Matcher>>,
    recording: Arc<AtomicBool>,
    actions: UnboundedSender<HotkeyAction>,
) -> AppResult<()> {
    if CONTEXT.set(HookContext { matcher, recording, actions }).is_err() {
        return Ok(()); // already running
    }
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<AppResult<()>>();
    std::thread::Builder::new()
        .name("hams-hotkeys".into())
        .spawn(move || unsafe {
            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), GetModuleHandleW(std::ptr::null()), 0);
            if hook.is_null() {
                let _ = ready_tx.send(Err(AppError::Other("Windows refused to register the keyboard shortcut hook.".into())));
                return;
            }
            tracing::info!("Keyboard shortcuts active");
            let _ = ready_tx.send(Ok(()));
            let mut message: MSG = std::mem::zeroed();
            while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        })
        .map_err(|e| AppError::Other(e.to_string()))?;
    ready_rx
        .recv_timeout(std::time::Duration::from_secs(3))
        .map_err(|_| AppError::Other("the keyboard listener didn't start".into()))?
}

/// Releasing Alt or Win on its own opens the menu bar / Start menu. Tapping an
/// unassigned key first ("menu mask", the same trick AutoHotkey uses) prevents it.
fn mask_menu_key() {
    const VK_UNASSIGNED: u16 = 0xE8;
    let make = |up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: VK_UNASSIGNED, wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { 0 }, time: 0, dwExtraInfo: 0 },
        },
    };
    let inputs = [make(false), make(true)];
    unsafe { SendInput(2, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam);
    }
    let info = &*(lparam as *const KBDLLHOOKSTRUCT);
    // Ignore our own synthetic keystrokes (paste, menu mask).
    if info.flags & LLKHF_INJECTED != 0 {
        return CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam);
    }
    let down = match wparam as u32 {
        WM_KEYDOWN | WM_SYSKEYDOWN => true,
        WM_KEYUP | WM_SYSKEYUP => false,
        _ => return CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam),
    };
    let Some(context) = CONTEXT.get() else {
        return CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam);
    };
    let key = key_id(info.vkCode);
    let result = context.matcher.lock().handle(key, down, context.recording.load(Ordering::Acquire));
    if let Some(action) = result.action {
        if let (HotkeyAction::Released(_), KeyId::Mod(ModKey::LeftAlt | ModKey::RightAlt | ModKey::LeftMeta | ModKey::RightMeta)) =
            (action, key)
        {
            mask_menu_key();
        }
        let _ = context.actions.send(action);
    }
    if result.swallow {
        1
    } else {
        CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
    }
}

fn key_id(vk: u32) -> KeyId {
    match vk {
        0x20 => KeyId::Key(Key::Space),
        0x1B => KeyId::Escape,
        0xA0 => KeyId::Mod(ModKey::LeftShift),
        0xA1 => KeyId::Mod(ModKey::RightShift),
        0xA2 => KeyId::Mod(ModKey::LeftCtrl),
        0xA3 => KeyId::Mod(ModKey::RightCtrl),
        0xA4 => KeyId::Mod(ModKey::LeftAlt),
        0xA5 => KeyId::Mod(ModKey::RightAlt),
        0x5B => KeyId::Mod(ModKey::LeftMeta),
        0x5C => KeyId::Mod(ModKey::RightMeta),
        0x30..=0x39 | 0x41..=0x5A => KeyId::Key(Key::Char(char::from(vk as u8))),
        0x70..=0x87 => KeyId::Key(Key::F((vk - 0x70 + 1) as u8)),
        _ => KeyId::Other,
    }
}
