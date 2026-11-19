//! Microphone permission through AVFoundation. Reading the status never opens
//! the mic, so the macOS recording indicator stays off.

use block2::RcBlock;
use objc2::msg_send;
use objc2::runtime::{AnyClass, Bool};
use objc2_foundation::NSString;

use super::super::MicPermission;

// AVCaptureDevice lives here; the class is looked up by name at run time.
#[link(name = "AVFoundation", kind = "framework")]
extern "C" {}

/// `AVMediaTypeAudio`.
const AUDIO: &str = "soun";

pub fn microphone_status() -> MicPermission {
    let Some(class) = AnyClass::get(c"AVCaptureDevice") else {
        return MicPermission::Unknown;
    };
    let media = NSString::from_str(AUDIO);
    // AVAuthorizationStatus: 0 notDetermined, 1 restricted, 2 denied, 3 authorized.
    let status: isize = unsafe { msg_send![class, authorizationStatusForMediaType: &*media] };
    match status {
        3 => MicPermission::Granted,
        1 | 2 => MicPermission::Denied,
        0 => MicPermission::Undetermined,
        _ => MicPermission::Unknown,
    }
}

/// Ask macOS to show its microphone prompt. Does nothing once the user has
/// answered (macOS never asks twice; the caller should open System Settings).
pub fn request_microphone() {
    let Some(class) = AnyClass::get(c"AVCaptureDevice") else {
        return;
    };
    let media = NSString::from_str(AUDIO);
    let done = RcBlock::new(|_granted: Bool| {});
    unsafe {
        let _: () = msg_send![class, requestAccessForMediaType: &*media, completionHandler: &*done];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs the real AVFoundation call: `msg_send!` checks its argument and
    /// return encodings at run time, so this catches a wrong signature. Only
    /// reads the status; it never shows the permission prompt.
    #[test]
    fn the_microphone_status_can_be_read() {
        assert_ne!(microphone_status(), MicPermission::Unknown);
    }
}
