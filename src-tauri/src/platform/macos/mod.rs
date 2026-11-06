//! macOS: CoreGraphics event tap for shortcuts, CGEvent for paste, NSWorkspace
//! for the frontmost app, and the Accessibility API for selected text.
#![allow(unused_unsafe, non_upper_case_globals)]

mod hotkeys;

use std::ffi::c_void;
use std::time::{Duration, Instant};

use core_foundation::base::{CFRelease, CFType, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication, NSWorkspace};

pub use hotkeys::start_hotkeys;

use super::AppTarget;
use crate::error::{AppError, AppResult};

type AXUIElementRef = *const c_void;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
    fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(element: AXUIElementRef, attribute: CFStringRef, value: *mut CFTypeRef) -> i32;
}

const KEY_V: u16 = 9;
const KEY_C: u16 = 8;

pub fn frontmost() -> Option<AppTarget> {
    unsafe {
        let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
        Some(AppTarget {
            pid: app.processIdentifier() as i64,
            id: app.bundleIdentifier().map(|s| s.to_string()).unwrap_or_default(),
            name: app.localizedName().map(|s| s.to_string()).unwrap_or_default(),
            window: 0,
        })
    }
}

/// Bring the original app back to the front (the overlay or Hams itself may have
/// taken focus), waiting briefly until macOS reports it as frontmost.
pub fn focus(target: &AppTarget) {
    if target.pid <= 0 || frontmost().is_some_and(|f| f.pid == target.pid) {
        return;
    }
    unsafe {
        if let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(target.pid as i32) {
            #[allow(deprecated)]
            app.activateWithOptions(NSApplicationActivationOptions::ActivateIgnoringOtherApps);
        }
    }
    let deadline = Instant::now() + Duration::from_millis(400);
    while Instant::now() < deadline {
        if frontmost().is_some_and(|f| f.pid == target.pid) {
            std::thread::sleep(Duration::from_millis(30));
            return;
        }
        std::thread::sleep(Duration::from_millis(15));
    }
    tracing::warn!("Could not return focus to {}", target.name);
}

fn send_command_key(key: u16) -> AppResult<()> {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| AppError::Delivery("could not create a keyboard event source".into()))?;
    let down = CGEvent::new_keyboard_event(source.clone(), key, true)
        .map_err(|_| AppError::Delivery("could not create a key event".into()))?;
    let up = CGEvent::new_keyboard_event(source, key, false)
        .map_err(|_| AppError::Delivery("could not create a key event".into()))?;
    down.set_flags(CGEventFlags::CGEventFlagCommand);
    up.set_flags(CGEventFlags::CGEventFlagCommand);
    down.post(CGEventTapLocation::HID);
    std::thread::sleep(Duration::from_millis(8));
    up.post(CGEventTapLocation::HID);
    Ok(())
}

/// ⌘V using the raw key code, so it works on every keyboard layout and never
/// touches main-thread-only text input APIs.
pub fn send_paste() -> AppResult<()> {
    send_command_key(KEY_V)
}

pub fn send_copy() -> AppResult<()> {
    send_command_key(KEY_C)
}

pub fn can_send_keys() -> bool {
    unsafe { AXIsProcessTrusted() }
}

pub fn request_accessibility() -> bool {
    let key = CFString::from_static_string("AXTrustedCheckOptionPrompt");
    let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), CFBoolean::true_value().as_CFType())]);
    unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef() as *const c_void) }
}

pub fn open_permission_settings(kind: &str) {
    let anchor = match kind {
        "microphone" => "Privacy_Microphone",
        "input_monitoring" => "Privacy_ListenEvent",
        _ => "Privacy_Accessibility",
    };
    let url = format!("x-apple.systempreferences:com.apple.preference.security?{anchor}");
    let _ = std::process::Command::new("open").arg(url).spawn();
}

/// Copy a string attribute of an element; None when it's missing or not a string.
unsafe fn string_attribute(element: AXUIElementRef, name: &'static str) -> Option<String> {
    let mut value: CFTypeRef = std::ptr::null();
    let attribute = CFString::from_static_string(name);
    if AXUIElementCopyAttributeValue(element, attribute.as_concrete_TypeRef(), &mut value) != 0 || value.is_null() {
        return None;
    }
    CFType::wrap_under_create_rule(value).downcast::<CFString>().map(|s| s.to_string())
}

/// Largest field Hams will read back for learning (characters).
const MAX_FIELD_CHARS: usize = 20_000;

/// The text of the focused field in the target app, through Accessibility.
/// Password fields are never read.
pub fn field_text(target: &AppTarget) -> Option<String> {
    if target.pid <= 0 || !can_send_keys() {
        return None;
    }
    unsafe {
        let app = AXUIElementCreateApplication(target.pid as i32);
        if app.is_null() {
            return None;
        }
        let mut focused: CFTypeRef = std::ptr::null();
        let focused_attr = CFString::from_static_string("AXFocusedUIElement");
        let status = AXUIElementCopyAttributeValue(app, focused_attr.as_concrete_TypeRef(), &mut focused);
        CFRelease(app as CFTypeRef);
        if status != 0 || focused.is_null() {
            return None;
        }
        let secure = [string_attribute(focused, "AXRole"), string_attribute(focused, "AXSubrole")]
            .iter()
            .flatten()
            .any(|r| r == "AXSecureTextField");
        let value = if secure { None } else { string_attribute(focused, "AXValue") };
        CFRelease(focused);
        value.filter(|v| v.chars().count() <= MAX_FIELD_CHARS)
    }
}

/// Read the focused element's selected text through the Accessibility API.
/// Returns None when the app doesn't expose it (many Electron apps), so the
/// caller can fall back to ⌘C.
pub fn selected_text_accessibility() -> Option<String> {
    if !can_send_keys() {
        return None;
    }
    unsafe {
        let system = AXUIElementCreateSystemWide();
        if system.is_null() {
            return None;
        }
        let mut focused: CFTypeRef = std::ptr::null();
        let focused_attr = CFString::from_static_string("AXFocusedUIElement");
        let status = AXUIElementCopyAttributeValue(system, focused_attr.as_concrete_TypeRef(), &mut focused);
        CFRelease(system as CFTypeRef);
        if status != 0 || focused.is_null() {
            return None;
        }
        let mut value: CFTypeRef = std::ptr::null();
        let selected_attr = CFString::from_static_string("AXSelectedText");
        let status = AXUIElementCopyAttributeValue(focused, selected_attr.as_concrete_TypeRef(), &mut value);
        CFRelease(focused);
        if status != 0 || value.is_null() {
            return None;
        }
        let value = CFType::wrap_under_create_rule(value);
        value.downcast::<CFString>().map(|s| s.to_string())
    }
}
