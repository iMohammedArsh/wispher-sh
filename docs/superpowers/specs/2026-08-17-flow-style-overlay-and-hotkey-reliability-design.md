# Flow-Style Overlay and Hotkey Reliability Design

## Goal

Make Wisper's native Windows overlay a calm, taskbar-adjacent status surface
that becomes a recording surface only while dictation is active. The global
push-to-talk shortcut must either operate correctly or show a visible error;
it must never silently stop accepting input. The application UI, local history,
clipboard, and paste delivery must report one consistent result rather than
appearing inactive when a system action fails.

## Overlay states

The overlay stays centered above the Windows taskbar, topmost, unfocused, and
excluded from the taskbar.

- **Idle:** A compact black lozenge (about 108 by 15 CSS pixels) with a subtle
  border and shadow. It contains no microphone icon, label, or button.
- **Recording:** Holding the configured push-to-talk shortcut expands the
  lozenge into a wider black strip (about 202 by 34 CSS pixels). The strip has
  live audio bars, a cancel affordance, and a finish affordance. Releasing the
  shortcut finishes recording; the finish affordance uses that same behavior.
- **Processing:** The strip contracts to a small, quiet black progress lozenge
  while transcription and delivery occur.
- **Error:** A small error state replaces processing only when an action cannot
  complete. It must expose a concise useful message in the main Wisper window.

The UI uses custom CSS bars and controls; it does not reuse Wispr Flow assets,
branding, or visual files.

## Input behavior

The primary workflow remains:

1. The user selects a Windows text field.
2. They hold `Ctrl+Shift+Space`.
3. Wisper captures the selected foreground target and records.
4. Releasing the keys stops capture, transcribes, restores that target, and
   sends the normal Windows paste shortcut.

The overlay remains passive: it does not take focus, change the selected target,
or require a click to begin the standard hotkey workflow.

## Native hotkey reliability

The existing low-level Windows hook remains necessary because the application
must distinguish key press from key release. `RegisterHotKey` alone cannot
deliver the press-and-hold behavior without changing the product interaction.

The listener will be changed so that:

- Hook-installation failures are delivered to the application immediately,
  including the Windows error code/message where available.
- The receiving loop ends rather than waiting forever when hook installation or
  the listener thread fails.
- Recording state is reset safely if the listener exits while a shortcut is
  held.
- Hook startup and shutdown are logged and surfaced through the existing main
  window delivery/status mechanism.

## Unified completion and delivery

Every completed dictation follows one observable sequence:

1. The transcript is added to the persistent local history.
2. The transcript is copied to the Windows clipboard.
3. Wisper restores the previously captured selected window and attempts the
   normal Windows paste shortcut.
4. A single completion result carries the transcript, saved-history outcome,
   clipboard outcome, and paste outcome to both the main interface and overlay.

Copying is successful even if Windows refuses to restore focus for automatic
pasting. In that case the interface says that the text is on the clipboard and
explains that the user can select the intended field and press `Ctrl+V`.

The native delivery routine will wait for the captured target to become the
foreground window before sending the input sequence. It will not claim a paste
was completed merely because the input calls were issued.

## Main interface and history

- The latest-transcript card updates as soon as a transcript completes.
- The History page refreshes immediately from the completion result and always
  shows every saved local record, including records whose automatic paste
  failed.
- Copy, paste-again, record, settings-save, vocabulary-save, and history
  actions await their backend results. Each action provides a success indication
  or a readable failure message; errors are never silently discarded.
- Editing a transcript updates the text shown in Wisper and copies the edited
  text. It does not silently imply persistence unless persistence succeeds.
- Error UI is conditional on actual errors rather than permanently disabled.

## Verification

- Build the frontend and run `cargo check` from a Visual Studio x64 Developer
  PowerShell.
- Start the native application, confirm the idle lozenge is centered above the
  taskbar and never looks like an empty rectangular container.
- Hold and release `Ctrl+Shift+Space`; verify expand, live audio bars,
  processing, collapse, transcription, and paste into the previously selected
  text field.
- Simulate or force a hook-installation failure where practical; verify the
  main window reports it rather than leaving the hotkey inactive without
  feedback.
- Confirm cancel discards the recording without pasting and does not leave the
  overlay expanded.
- Confirm the latest transcript and History page update immediately after every
  completed dictation.
- Test Copy, Paste Again, start/stop, settings save, vocabulary save, and
  history actions. Each test must produce a visible success or failure result.
- When Windows blocks focus restoration, confirm text remains in the clipboard,
  the UI reports the distinction clearly, and the history record remains usable.
