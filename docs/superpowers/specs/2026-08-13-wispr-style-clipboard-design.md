# Wispr-style Clipboard and Paste Flow

## Goal

After a successful dictation, make the final transcript immediately useful in the focused application and remain available on the system clipboard for repeat pastes.

## Default behavior: Copy and paste

1. Wisper completes local transcription successfully.
2. It writes the final transcript to the system clipboard and **does not restore** the previous clipboard text.
3. It waits for the WSLg clipboard bridge to synchronize.
4. It sends the platform paste shortcut to the currently focused application.
5. The operating system replaces the current selection if the focused control has one; otherwise it inserts at the caret. This is native paste behavior and does not require reading or modifying selected content.
6. The UI shows `Copied to clipboard` followed by either `Pasted into focused app` or a paste error. The dictation remains copyable from the Telemetry screen.

## User controls

Add a `Dictation delivery` setting with three modes:

- **Copy and paste** (default): retain dictated text on the clipboard and paste it into the focused app.
- **Copy only**: retain dictated text on the clipboard without sending a synthetic paste shortcut.
- **Paste and restore clipboard**: paste then restore the previous text clipboard value, matching the project's prior privacy-preserving behavior.

The selected mode persists with the existing settings and is included in the dashboard status.

## Selection behavior

Wisper will not inspect selected text or application content. It relies on the platform's paste semantics: paste replaces a standard editable selection, while a non-editable selection or unsupported target may ignore the shortcut. This is the reliable cross-application behavior available from WSLg without intrusive accessibility APIs.

## Clipboard injection design

Split the current clipboard injector into a delivery result with independent `copied` and `pasted` outcomes. Clipboard creation/write failures stop the delivery and surface an error. Paste-key errors are surfaced without discarding the newly copied transcript. Clipboard restoration is performed only in `Paste and restore clipboard` mode.

Use bounded waits before and after the paste to allow the WSLg clipboard bridge and target application to process the change. Under WSL, issue the paste shortcut through the Windows host so a Windows-native foreground application receives it; elsewhere use the native platform input API. Do not silently ignore keyboard injection errors.

## UI and status

The status event includes delivery information: chosen mode, copied flag, paste attempted flag, and paste outcome. The telemetry card displays the latest delivery outcome and a visible note that a selected editable field is replaced by normal paste behavior. Error states distinguish transcription failure, clipboard write failure, and paste injection failure.

## Verification

1. In Copy and paste mode, dictate into a plain text editor with no selection: text appears at the caret and remains available through a manual paste.
2. In Copy and paste mode, select existing text in a plain text editor: dictation replaces it and remains on the clipboard.
3. In Copy only mode, dictation reaches the clipboard and does not change the focused app.
4. In Paste and restore clipboard mode, the original text clipboard entry is restored after the paste.
5. Rust and TypeScript builds pass; clipboard/paste errors appear in the UI and do not mark delivery as successful.
