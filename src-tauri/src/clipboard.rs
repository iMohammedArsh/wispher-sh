use std::thread;
use std::time::Duration;
#[cfg(target_os = "linux")]
use std::{
    io::Write,
    process::{Command, Stdio},
};

use arboard::Clipboard;
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use serde::Serialize;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL, VK_V,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow};

/// The native window that had the user's caret before recording started.
///
/// Keeping this identity lets delivery return to the selected input field even when an overlay,
/// tray menu, or processing window briefly becomes foreground during dictation.
#[derive(Clone, Debug)]
pub struct ForegroundTarget {
    #[cfg(target_os = "windows")]
    // Store the handle as a number so AppState remains Send + Sync for Tauri commands.
    // It is converted back to HWND only at the Win32 API boundary.
    handle: usize,
    #[cfg(target_os = "linux")]
    handle: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveryMode {
    CopyAndPaste,
    CopyOnly,
    PasteAndRestoreClipboard,
}

impl DeliveryMode {
    pub fn from_setting(value: &str) -> Self {
        match value {
            "copy_only" => Self::CopyOnly,
            "paste_and_restore" => Self::PasteAndRestoreClipboard,
            _ => Self::CopyAndPaste,
        }
    }
}

#[derive(Clone, Serialize, Debug)]
pub struct DeliveryResult {
    pub copied: bool,
    pub paste_attempted: bool,
    pub pasted: bool,
    pub message: String,
}

/// Delivers a successful dictation through the system clipboard.
///
/// The operating system decides whether Ctrl+V inserts at the caret or replaces an existing
/// editable selection. Wisper deliberately does not inspect selected application content.
pub struct ClipboardInjector;

impl ClipboardInjector {
    /// Captures the currently focused native target without changing focus.
    pub fn capture_foreground_target() -> Option<ForegroundTarget> {
        #[cfg(target_os = "windows")]
        {
            let handle = unsafe { GetForegroundWindow() };
            return (handle != std::ptr::null_mut()).then_some(ForegroundTarget {
                handle: handle as usize,
            });
        }

        #[cfg(target_os = "linux")]
        if is_wsl() {
            return capture_windows_foreground_target().ok();
        }

        #[allow(unreachable_code)]
        None
    }

    /// Writes text to the system clipboard without injecting a paste shortcut.
    pub fn copy(text: &str) -> Result<DeliveryResult, String> {
        if text.trim().is_empty() {
            return Err("Refusing to copy an empty dictation.".into());
        }
        let mut clipboard = Clipboard::new()
            .map_err(|error| format!("Could not access the system clipboard: {error}"))?;
        write_clipboard_text(&mut clipboard, text)?;
        Ok(DeliveryResult {
            copied: true,
            paste_attempted: false,
            pasted: false,
            message: "Copied to clipboard. Paste it anywhere with Ctrl+V.".into(),
        })
    }

    pub fn deliver(
        text: &str,
        mode: DeliveryMode,
        target: Option<&ForegroundTarget>,
    ) -> Result<DeliveryResult, String> {
        if text.trim().is_empty() {
            return Err("Refusing to deliver an empty dictation.".into());
        }

        let mut clipboard = Clipboard::new()
            .map_err(|error| format!("Could not access the system clipboard: {error}"))?;
        let previous_clipboard_text = if mode == DeliveryMode::PasteAndRestoreClipboard {
            clipboard.get_text().ok()
        } else {
            None
        };

        write_clipboard_text(&mut clipboard, text)?;

        if mode == DeliveryMode::CopyOnly {
            return Ok(DeliveryResult {
                copied: true,
                paste_attempted: false,
                pasted: false,
                message: "Copied to clipboard. Paste it anywhere with Ctrl+V.".into(),
            });
        }

        // Native Windows clipboard ownership becomes available promptly. Keep a short
        // settle window for reliability without adding a noticeable delivery delay.
        thread::sleep(Duration::from_millis(25));
        paste_into_target_app(target)?;

        if mode == DeliveryMode::PasteAndRestoreClipboard {
            // Allow the target to consume Ctrl+V before replacing the user's clipboard.
            thread::sleep(Duration::from_millis(75));
            if let Some(previous_text) = previous_clipboard_text {
                if let Err(error) = write_clipboard_text(&mut clipboard, &previous_text) {
                    return Ok(DeliveryResult {
                        copied: true,
                        paste_attempted: true,
                        pasted: true,
                        message: format!(
                            "Pasted into the focused app, but could not restore the previous clipboard text: {error}"
                        ),
                    });
                }
            }
            return Ok(DeliveryResult {
                copied: true,
                paste_attempted: true,
                pasted: true,
                message: "Sent paste to the focused app and restored the previous clipboard text."
                    .into(),
            });
        }

        Ok(DeliveryResult {
            copied: true,
            paste_attempted: true,
            pasted: true,
            message: "Copied to clipboard and sent paste to the focused app. An editable selection is replaced by normal paste behavior.".into(),
        })
    }
}

fn write_clipboard_text(clipboard: &mut Clipboard, text: &str) -> Result<(), String> {
    clipboard
        .set_text(text)
        .map_err(|error| format!("Could not copy dictation to the system clipboard: {error}"))?;

    #[cfg(target_os = "linux")]
    if is_wsl() {
        write_to_windows_clipboard(text)?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn paste_into_target_app(target: Option<&ForegroundTarget>) -> Result<(), String> {
    send_windows_paste(target)
}

#[cfg(target_os = "linux")]
fn paste_into_target_app(target: Option<&ForegroundTarget>) -> Result<(), String> {
    if is_wsl() {
        return paste_into_foreground_windows_app(target);
    }
    Err("Automatic paste is not supported outside a Windows desktop session.".into())
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn paste_into_target_app(_target: Option<&ForegroundTarget>) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|error| format!("Could not initialize keyboard input for paste: {error}"))?;

    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;

    enigo
        .key(modifier, Direction::Press)
        .map_err(|error| format!("Could not press the paste modifier key: {error}"))?;
    let paste_result = enigo.key(Key::Unicode('v'), Direction::Click);
    let release_result = enigo.key(modifier, Direction::Release);
    paste_result.map_err(|error| format!("Could not send the paste shortcut: {error}"))?;
    release_result.map_err(|error| format!("Could not release the paste modifier key: {error}"))?;
    Ok(())
}

/// Restores the captured target, then sends the standard Windows Ctrl+V sequence.
/// `SendInput` reports input-queue acceptance; it cannot prove that the target application
/// accepted the clipboard contents, so callers describe this as a paste *attempt*.
#[cfg(target_os = "windows")]
fn send_windows_paste(target: Option<&ForegroundTarget>) -> Result<(), String> {
    if let Some(target) = target {
        let target_handle = target.handle as HWND;
        let current = unsafe { GetForegroundWindow() };
        if current != target_handle && unsafe { SetForegroundWindow(target_handle) } == 0 {
            return Err("Windows could not return focus to the selected text field. The transcript remains on the clipboard.".into());
        }
        // Windows may accept SetForegroundWindow before the target application's input queue
        // becomes active. Wait briefly for the actual foreground hand-off before Ctrl+V.
        let mut focused = unsafe { GetForegroundWindow() } == target_handle;
        for _ in 0..10 {
            if focused {
                break;
            }
            thread::sleep(Duration::from_millis(30));
            focused = unsafe { GetForegroundWindow() } == target_handle;
        }
        if !focused {
            return Err("Windows did not activate the selected text field in time. The transcript remains on the clipboard; select the field and press Ctrl+V.".into());
        }
        thread::sleep(Duration::from_millis(20));
    }

    fn key(vk: u16, flags: u32) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    let inputs = [
        key(VK_CONTROL, 0),
        key(VK_V, 0),
        key(VK_V, KEYEVENTF_KEYUP),
        key(VK_CONTROL, KEYEVENTF_KEYUP),
    ];
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    if sent == inputs.len() as u32 {
        Ok(())
    } else {
        Err(format!(
            "Windows accepted only {sent} of {} paste input events. The transcript remains on the clipboard.",
            inputs.len()
        ))
    }
}

#[cfg(target_os = "linux")]
fn is_wsl() -> bool {
    std::env::var_os("WSL_INTEROP").is_some()
        || std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .map(|release| release.to_ascii_lowercase().contains("microsoft"))
            .unwrap_or(false)
}

/// On WSL, Enigo's X11 events target Linux windows only. Sending the standard paste shortcut
/// through the Windows host lets the foreground Windows app handle its own selection/caret.
#[cfg(target_os = "linux")]
fn paste_into_foreground_windows_app(target: Option<&ForegroundTarget>) -> Result<(), String> {
    let target_handle = target.map(|target| target.handle.as_str()).unwrap_or("");
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$signature = '[DllImport(\"user32.dll\")] public static extern IntPtr GetForegroundWindow(); [DllImport(\"user32.dll\")] public static extern bool SetForegroundWindow(IntPtr hWnd);'; Add-Type -Name WisperNative -Namespace Wisper -MemberDefinition $signature; $target = [IntPtr]::Zero; if ($args[0]) { $target = [IntPtr]::new([Int64]$args[0]); } if ($target -ne [IntPtr]::Zero -and [Wisper.WisperNative]::GetForegroundWindow() -ne $target -and -not [Wisper.WisperNative]::SetForegroundWindow($target)) { Write-Error 'Could not return focus to the selected text field.'; exit 1 }; Start-Sleep -Milliseconds 75; $shell = New-Object -ComObject WScript.Shell; $shell.SendKeys('^v')",
            target_handle,
        ])
        .output()
        .map_err(|error| format!("Could not request paste from the Windows host: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("Windows host rejected the paste shortcut: {error}"))
    }
}

#[cfg(target_os = "linux")]
fn capture_windows_foreground_target() -> Result<ForegroundTarget, String> {
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$signature = '[DllImport(\"user32.dll\")] public static extern IntPtr GetForegroundWindow();'; Add-Type -Name WisperNative -Namespace Wisper -MemberDefinition $signature; [Wisper.WisperNative]::GetForegroundWindow().ToInt64()",
        ])
        .output()
        .map_err(|error| format!("Could not identify the selected Windows target: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Windows could not identify the selected text field: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let handle = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if handle.is_empty() || handle == "0" {
        return Err("Windows did not report an active text-field window.".into());
    }
    Ok(ForegroundTarget { handle })
}

/// WSLg clipboard synchronization is asynchronous. Writing to the Windows host as well makes
/// text available to the actual foreground Windows application before Ctrl+V is sent.
#[cfg(target_os = "linux")]
fn write_to_windows_clipboard(text: &str) -> Result<(), String> {
    let mut process = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Console]::InputEncoding = [Text.Encoding]::UTF8; $inputText = [Console]::In.ReadToEnd(); Set-Clipboard -Value $inputText",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not access the Windows system clipboard: {error}"))?;

    process
        .stdin
        .take()
        .ok_or_else(|| "Could not open the Windows clipboard input stream.".to_string())?
        .write_all(text.as_bytes())
        .map_err(|error| {
            format!("Could not write text to the Windows system clipboard: {error}")
        })?;

    let output = process
        .wait_with_output()
        .map_err(|error| format!("Could not finish the Windows clipboard operation: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Windows could not store the dictated text on the system clipboard: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
