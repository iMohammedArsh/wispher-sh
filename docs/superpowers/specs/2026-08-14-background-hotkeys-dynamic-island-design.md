# Background Hotkeys and Dynamic Island

## Goal

Let Wisper dictate while another Windows application remains selected. A small, always-visible status island communicates recording and delivery state without stealing focus.

## Agreed shortcuts

- **Alt + Space — push to talk:** press and hold to begin recording; release to stop, transcribe, copy, and paste.
- **Ctrl + Alt + Space — hands-free toggle:** first press begins recording; second press stops, transcribes, copies, and pastes.

Plain `Alt + Space` normally opens the Windows system menu. Wisper will intercept the chord in the Windows host listener while it is running so the active application does not receive it. `Ctrl + Tab` is deliberately not used because it switches tabs in browsers and other applications.

## Architecture

### Windows host shortcut bridge

The app currently runs its Rust process in WSL, and its X11 listener cannot observe keys pressed in Windows-native foreground applications. A small Windows-host helper will use a low-level keyboard hook to observe the two agreed chords, suppress those chord events from the active application, and emit line-delimited `push_down`, `push_up`, and `toggle` events to its WSL parent over standard output.

The Rust `HotkeyListener` will launch, monitor, and restart this helper when necessary. It will translate its events into exactly one recording state transition at a time. The existing X11 listener remains only as a fallback for non-WSL Linux sessions. Failure to start the Windows helper is surfaced through Wisper status and the island rather than silently disabling hotkeys.

### Recording state machine

- `Idle` + `push_down` starts a push-to-talk recording.
- `Recording(push)` + `push_up` stops that recording.
- `Idle` + `toggle` starts a hands-free recording.
- `Recording(hands_free)` + `toggle` stops that recording.
- Duplicate, unmatched, and key-repeat events are ignored.
- A push-to-talk release never stops a hands-free recording, and a hands-free toggle never stops a push-to-talk recording except where the state machine explicitly allows it.

The state machine owns the recording origin so the tray, dashboard button, and host helper cannot race into duplicate starts or stops.

### Dynamic island

Wisper will create a second, frameless overlay window positioned at the bottom-right of the primary monitor. It is always-on-top, non-focusable, skipped from the taskbar, and does not activate when shown. The main dashboard can remain hidden in the tray while the island remains available.

The island has four concise visual states:

1. **Ready:** compact `Wisper ready` pill with the two shortcuts.
2. **Recording:** colored microphone indicator, elapsed time, and the mode (`Hold to talk` or `Hands-free`).
3. **Processing:** `Transcribing…` followed by `Copying and pasting…`.
4. **Outcome:** a short `Pasted`, `Copied`, or error result before returning to Ready.

The island does not need click-through behavior in this scope; it must never take keyboard focus or require the user to select Wisper before dictating.

## Error handling and status

- If Windows refuses the hook or the helper exits, show `Background hotkeys unavailable` in both dashboard and island, with a diagnostic message.
- Clipboard or paste errors retain their existing distinct messages and appear as an outcome state.
- Starting a recording when one is already active has no effect.
- The app’s settings reflect the fixed shortcuts accurately; the existing inactive hotkey selector will be replaced or disabled so it cannot claim to configure behavior it does not control.

## Verification

1. With Chrome, Word, and a plain text editor focused in turn, hold `Alt + Space`, speak, and release; text is pasted into each app without first focusing Wisper.
2. While a browser is focused, `Alt + Space` does not open the Windows system menu, and `Ctrl + Tab` continues to change tabs normally.
3. Press `Ctrl + Alt + Space` once, speak hands-free, then press it again; recording begins and ends exactly once.
4. Minimize the dashboard to the tray; verify the island remains visible, non-focusable, and accurately changes through Ready, Recording, Processing, and Outcome.
5. Confirm the active target application retains focus throughout dictation and normal typed keyboard input continues after delivery.
6. Confirm TypeScript build and WSL Rust check pass. Test a helper startup failure and verify it is reported visibly.
