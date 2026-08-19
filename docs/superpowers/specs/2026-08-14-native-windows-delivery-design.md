# Native Windows Delivery Design

## Goal

Make Wisper behave like a reliable Windows dictation utility: record without taking focus, show a compact voice capsule above other applications, copy every completed transcript, attempt a standard paste into the foreground application, and always retain a recovery path.

## Decision

The production runtime is a native Windows Tauri executable. PowerShell and WSL are not used for global keyboard handling, overlay rendering, clipboard writes, or paste injection. The current WSL bridge remains only as a development compatibility fallback until Windows builds are available.

## Delivery contract

1. Save every non-empty clean transcript in local Wisper history before delivery.
2. Copy the transcript to the Windows clipboard.
3. In copy-only mode, stop and report that the text is ready for Ctrl+V.
4. In paste modes, send the normal Ctrl+V key sequence to the foreground Windows application without activating Wisper.
5. In restore mode, restore the previously captured text clipboard only after the input sequence is successfully sent.
6. If copying or input injection cannot begin, retain the transcript in local history and report a recovery action.
7. Never report that text was definitely inserted: Windows input injection cannot prove a target application accepted the paste.

## Native components

- `clipboard.rs`: Native Windows clipboard writes through the existing clipboard crate and Ctrl+V through `SendInput`; no PowerShell, COM, or fixed WSL clipboard delay on Windows.
- `hotkey.rs`: Native low-level Windows keyboard hook for Alt+Space hold-to-talk and Ctrl+Alt+Space hands-free mode. It must not make the main application active.
- `overlay.rs`: Native non-activating topmost window. Idle is a thin pill; recording is a compact waveform capsule; processing is compact. It is excluded from the task bar and does not accept focus.
- `history.rs`: Retains local transcript history independent of clipboard state.

## Fallbacks and boundaries

- No editable foreground target: transcript remains on clipboard and in history, with a clear ready-to-paste state.
- Elevated target application: Windows blocks a lower-integrity application from sending input. Wisper explains that it must itself run elevated; it does not bypass UAC.
- WSL terminals, remote desktops, and software with disabled clipboard access may reject automatic paste. The recovery action remains available.
- Clipboard restore is text-only in the first native release. If the prior clipboard has an unsupported non-text payload, Wisper preserves the dictated text rather than pretending the original data was restored.

## Verification matrix

Verify on a Windows build with Notepad, Chrome, Word, VS Code editor, Windows Terminal/PowerShell, an elevated application, a WSL terminal, and an RDP session when available. For each target confirm overlay visibility, no focus theft, transcript history persistence, clipboard outcome, attempted paste outcome, and recovery messaging.

## Acceptance criteria

- No `powershell.exe` is spawned by the Windows clipboard or paste path.
- Global controls work when another ordinary Windows app is focused.
- The voice capsule remains visible and never becomes the active window.
- Each dictated text is recoverable from both history and clipboard after a failed delivery.
- Build and automated tests cover native clipboard/injection decisions; manual matrix results are recorded before release.
