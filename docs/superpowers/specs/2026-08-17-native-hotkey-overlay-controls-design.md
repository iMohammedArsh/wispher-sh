# Native Hotkey, Overlay, and Window Controls Design

## Goal

On native Windows, Wisper delivers a selected-field dictation through one
predictable sequence: hotkey down starts audio capture, hotkey up stops capture,
the result is transcribed, and the transcript is pasted into the Windows input
field selected before recording began.

## Hotkey and delivery behavior

- No always-listening or automatic voice activation is added.
- The native Windows global hotkey listener starts recording on key down and
  stops it on key up.
- The foreground target is captured before recording and reused for delivery.
- Completion writes the transcript to the Windows clipboard and sends the
  normal Windows paste shortcut to that captured target.

## Overlay behavior

- The native overlay is always visible while Wisper is running and always
  contains a microphone/status icon; it must never appear as a blank transparent
  capsule.
- Idle: muted microphone and ready status.
- Recording: active microphone and live audio waveform.
- Processing: visible processing icon and status.
- Error: visible error icon and an actionable status.
- The overlay stays topmost, avoids activating itself, and does not interfere
  with the selected input field.

## Main-window controls

- Minimize and close remain outside the drag region and invoke native Tauri
  window actions.
- Buttons expose proper button types, accessible labels, and visible hit areas.
- Native window-action failures are surfaced to the application status instead
  of being silently discarded.

## Verification

- Build the React frontend and native Windows Rust application with the Visual
  Studio x64 Developer PowerShell.
- Verify hotkey down/up transitions, overlay icon visibility, title-bar minimize
  and close, and paste into a selected Windows text field.
- Verify an input-injection failure preserves the transcript on the clipboard.
