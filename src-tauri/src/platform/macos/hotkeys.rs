//! Global shortcuts through a CoreGraphics event tap on its own run-loop thread.
//!
//! Written against the raw C API (not the `core-graphics` wrapper) because the
//! wrapper can't drop events, and we must swallow the shortcut key so it doesn't
//! also type a space into the focused app. The callback does no work beyond
//! matching and sending a message, so macOS never disables the tap for being slow.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::Arc;

use core_foundation::base::TCFType;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop, CFRunLoopSource};
use parking_lot::Mutex;
use tokio::sync::mpsc::UnboundedSender;

use crate::error::{AppError, AppResult};
use crate::input::{HotkeyAction, Key, KeyId, Matcher, ModKey};

type CGEventRef = *mut c_void;
type CFMachPortRef = *mut c_void;
type TapCallback = extern "C" fn(*mut c_void, u32, CGEventRef, *mut c_void) -> CGEventRef;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(tap: u32, place: u32, options: u32, mask: u64, callback: TapCallback, info: *mut c_void) -> CFMachPortRef;
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    fn CGEventGetFlags(event: CGEventRef) -> u64;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFMachPortCreateRunLoopSource(allocator: *const c_void, port: CFMachPortRef, order: isize) -> *mut c_void;
}

const SESSION_TAP: u32 = 1;
const HEAD_INSERT: u32 = 0;
const OPTION_DEFAULT: u32 = 0; // can drop events; needs Accessibility
const OPTION_LISTEN_ONLY: u32 = 1; // fallback; needs Input Monitoring
const KEY_DOWN: u32 = 10;
const KEY_UP: u32 = 11;
const FLAGS_CHANGED: u32 = 12;
const TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFF_FFFE;
const TAP_DISABLED_BY_USER: u32 = 0xFFFF_FFFF;
const FIELD_KEYCODE: u32 = 9;

// Device-specific modifier bits (NX_DEVICE*KEYMASK) tell left from right.
const LEFT_CTRL: u64 = 0x0000_0001;
const LEFT_SHIFT: u64 = 0x0000_0002;
const RIGHT_SHIFT: u64 = 0x0000_0004;
const LEFT_CMD: u64 = 0x0000_0008;
const RIGHT_CMD: u64 = 0x0000_0010;
const LEFT_ALT: u64 = 0x0000_0020;
const RIGHT_ALT: u64 = 0x0000_0040;
const RIGHT_CTRL: u64 = 0x0000_2000;
const FN_FLAG: u64 = 0x0080_0000;

struct TapContext {
    matcher: Arc<Mutex<Matcher>>,
    recording: Arc<AtomicBool>,
    actions: UnboundedSender<HotkeyAction>,
    port: AtomicPtr<c_void>,
    can_swallow: bool,
}

pub fn start_hotkeys(
    matcher: Arc<Mutex<Matcher>>,
    recording: Arc<AtomicBool>,
    actions: UnboundedSender<HotkeyAction>,
) -> AppResult<()> {
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<AppResult<()>>();
    std::thread::Builder::new()
        .name("hams-hotkeys".into())
        .spawn(move || {
            let mask = (1u64 << KEY_DOWN) | (1u64 << KEY_UP) | (1u64 << FLAGS_CHANGED);
            let create = |options: u32, context: *mut TapContext| unsafe {
                CGEventTapCreate(SESSION_TAP, HEAD_INSERT, options, mask, tap_callback, context as *mut c_void)
            };
            let make_context = |can_swallow| {
                Box::into_raw(Box::new(TapContext {
                    matcher: Arc::clone(&matcher),
                    recording: Arc::clone(&recording),
                    actions: actions.clone(),
                    port: AtomicPtr::new(std::ptr::null_mut()),
                    can_swallow,
                }))
            };

            let mut context = make_context(true);
            let mut port = create(OPTION_DEFAULT, context);
            if port.is_null() {
                unsafe { drop(Box::from_raw(context)) };
                tracing::warn!("Active keyboard tap unavailable (no Accessibility permission); falling back to listen-only");
                context = make_context(false);
                port = create(OPTION_LISTEN_ONLY, context);
            }
            if port.is_null() {
                unsafe { drop(Box::from_raw(context)) };
                let _ = ready_tx.send(Err(AppError::Permission(
                    "Hams can't see the keyboard shortcut. Allow Hams in System Settings → Privacy & Security → Accessibility, then restart Hams.".into(),
                )));
                return;
            }
            // The context is intentionally leaked: the tap lives for the whole process.
            unsafe { (*context).port.store(port, Ordering::Release) };

            unsafe {
                let source = CFMachPortCreateRunLoopSource(std::ptr::null(), port, 0);
                if source.is_null() {
                    let _ = ready_tx.send(Err(AppError::Other("could not attach the keyboard tap".into())));
                    return;
                }
                let source = CFRunLoopSource::wrap_under_create_rule(source as _);
                CFRunLoop::get_current().add_source(&source, kCFRunLoopCommonModes);
                CGEventTapEnable(port, true);
            }
            tracing::info!("Keyboard shortcuts active");
            let _ = ready_tx.send(Ok(()));
            CFRunLoop::run_current();
            tracing::error!("Keyboard shortcut run loop exited");
        })
        .map_err(|e| AppError::Other(e.to_string()))?;
    ready_rx
        .recv_timeout(std::time::Duration::from_secs(3))
        .map_err(|_| AppError::Other("the keyboard listener didn't start".into()))?
}

extern "C" fn tap_callback(_proxy: *mut c_void, event_type: u32, event: CGEventRef, info: *mut c_void) -> CGEventRef {
    let context = unsafe { &*(info as *const TapContext) };
    if event_type == TAP_DISABLED_BY_TIMEOUT || event_type == TAP_DISABLED_BY_USER {
        let port = context.port.load(Ordering::Acquire);
        if !port.is_null() {
            unsafe { CGEventTapEnable(port, true) };
        }
        return event;
    }
    let keycode = unsafe { CGEventGetIntegerValueField(event, FIELD_KEYCODE) } as u16;
    let (key, down) = match event_type {
        KEY_DOWN | KEY_UP => (key_id(keycode), event_type == KEY_DOWN),
        FLAGS_CHANGED => {
            let Some((modifier, mask)) = modifier(keycode) else { return event };
            let flags = unsafe { CGEventGetFlags(event) };
            (KeyId::Mod(modifier), flags & mask != 0)
        }
        _ => return event,
    };
    let result = context
        .matcher
        .lock()
        .handle(key, down, context.recording.load(Ordering::Acquire));
    if let Some(action) = result.action {
        let _ = context.actions.send(action);
    }
    if result.swallow && context.can_swallow {
        std::ptr::null_mut()
    } else {
        event
    }
}

fn modifier(keycode: u16) -> Option<(ModKey, u64)> {
    Some(match keycode {
        55 => (ModKey::LeftMeta, LEFT_CMD),
        54 => (ModKey::RightMeta, RIGHT_CMD),
        56 => (ModKey::LeftShift, LEFT_SHIFT),
        60 => (ModKey::RightShift, RIGHT_SHIFT),
        58 => (ModKey::LeftAlt, LEFT_ALT),
        61 => (ModKey::RightAlt, RIGHT_ALT),
        59 => (ModKey::LeftCtrl, LEFT_CTRL),
        62 => (ModKey::RightCtrl, RIGHT_CTRL),
        63 => (ModKey::Fn, FN_FLAG),
        _ => return None,
    })
}

/// ANSI virtual key codes (kVK_*). Physical positions, so layouts don't matter.
fn key_id(keycode: u16) -> KeyId {
    const LETTERS: [(u16, char); 36] = [
        (0, 'A'), (11, 'B'), (8, 'C'), (2, 'D'), (14, 'E'), (3, 'F'), (5, 'G'), (4, 'H'), (34, 'I'),
        (38, 'J'), (40, 'K'), (37, 'L'), (46, 'M'), (45, 'N'), (31, 'O'), (35, 'P'), (12, 'Q'),
        (15, 'R'), (1, 'S'), (17, 'T'), (32, 'U'), (9, 'V'), (13, 'W'), (7, 'X'), (16, 'Y'), (6, 'Z'),
        (29, '0'), (18, '1'), (19, '2'), (20, '3'), (21, '4'), (23, '5'), (22, '6'), (26, '7'),
        (28, '8'), (25, '9'),
    ];
    const FKEYS: [(u16, u8); 20] = [
        (122, 1), (120, 2), (99, 3), (118, 4), (96, 5), (97, 6), (98, 7), (100, 8), (101, 9),
        (109, 10), (103, 11), (111, 12), (105, 13), (107, 14), (113, 15), (106, 16), (64, 17),
        (79, 18), (80, 19), (90, 20),
    ];
    match keycode {
        49 => KeyId::Key(Key::Space),
        53 => KeyId::Escape,
        _ => {
            if let Some((_, c)) = LETTERS.iter().find(|(k, _)| *k == keycode) {
                KeyId::Key(Key::Char(*c))
            } else if let Some((_, n)) = FKEYS.iter().find(|(k, _)| *k == keycode) {
                KeyId::Key(Key::F(*n))
            } else {
                KeyId::Other
            }
        }
    }
}
