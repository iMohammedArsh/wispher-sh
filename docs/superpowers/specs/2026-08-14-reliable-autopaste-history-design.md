# Reliable Auto-Paste and Local Dictation History

## Goal

Make each successful Wisper dictation behave like a dependable Wispr-style action: copy the cleaned transcript to the operating-system clipboard, paste it into the currently focused editable control, retain it on the clipboard for a later manual paste, and retain a private local history inside Wisper.

## Agreed behavior

1. A successful, non-empty transcription becomes the delivery text. Telemetry is never copied or pasted.
2. Wisper saves that text to its local history before attempting clipboard delivery. The history remains available while the app is hidden in the system tray and after restart.
3. Wisper writes the delivery text through the Rust backend to the system clipboard. It does not clear or restore the clipboard after a normal auto-paste; the text remains available until another application replaces it.
4. After a bounded WSLg synchronization wait, Wisper sends the platform paste shortcut to the foreground application. Native paste inserts at the caret or replaces the active editable selection, such as selected text in a word-processing field.
5. The default delivery mode is `Copy and paste`. The existing `Copy only` and `Paste and restore clipboard` modes remain explicit opt-ins.
6. The dashboard Copy button calls the same native clipboard writer, never the browser clipboard API. It reports success only after the backend confirms the write.

## Architecture

### Clipboard delivery

`ClipboardInjector` remains the sole OS-clipboard and keyboard-injection boundary. It gains a copy-only operation for the dashboard and returns structured delivery details: copied, paste attempted, pasted, and a message. The dictation pipeline continues to invoke it after local transcription.

On WSL, clipboard writing is checked and the paste shortcut is sent through `powershell.exe` to the Windows foreground application. The existing short bridge delay remains bounded. A write failure prevents paste and is visible in status. A paste failure leaves the copied text on the clipboard and is visible separately.

### Local history

A small `DictationHistory` store owns an ordered, capped collection of records containing an ID, transcript, and capture timestamp. It is persisted atomically in Wisper's configuration directory, separate from settings. Load errors fall back safely to an empty in-memory history and surface only a diagnostic message; they never block dictation.

Every successful non-empty transcript is recorded once, before delivery. Persist failures do not discard the transcript, clipboard copy, or paste attempt; they are reported as a history-specific status message. The store is held in `AppState` so the background hotkey flow and dashboard commands share one source of truth.

### UI and commands

The backend exposes commands to retrieve local history and to copy a selected transcript. The dashboard renders the latest dictated text and a compact local-history list. Its manual Copy action uses the backend command and changes state only after confirmation. Delivery status distinguishes copied, pasted, clipboard write failure, paste failure, and history persistence failure.

## Error handling

- Empty transcripts are neither saved, copied, nor pasted.
- Clipboard-write failures prevent the synthetic paste and retain the locally saved history record.
- Paste failures retain the copied transcript on the system clipboard and leave a retryable history entry.
- Unsupported focused targets may ignore Ctrl+V; Wisper reports the injection result but cannot verify that a third-party application accepted pasted content without invasive accessibility integration.
- Corrupt or unavailable local-history storage does not stop transcription, copying, or pasting.

## Verification

1. Run Rust and TypeScript builds.
2. In default mode, dictate into a plain editor and confirm text inserts at the caret and remains available through a later manual Ctrl+V.
3. Select a word in an editable field, dictate, and confirm native paste replaces that selection.
4. With Wisper hidden to the tray, dictate and confirm the resulting transcript appears in local history after reopening the window and after restarting the app.
5. Click Copy on a current or historical transcript, then manually paste it elsewhere; the UI shows success only if the backend operation succeeds.
6. Exercise a clipboard or paste failure where practical and confirm history is retained and the failure message identifies the failed delivery stage.
