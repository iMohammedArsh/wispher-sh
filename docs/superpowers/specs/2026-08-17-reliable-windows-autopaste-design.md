# Reliable Windows Auto-Paste Design

## Goal

After a user selects an editable text field in a Windows application and finishes
dictating, Wisper must insert the completed transcript exactly as a normal
`Ctrl+V` paste would. A selected editable range is replaced by the transcript.

## Scope

- Support Windows target applications while Wisper is running through WSL.
- Keep the selected delivery mode authoritative for automatic delivery and every
  manual re-paste entry point.
- Preserve the transcript in the Windows clipboard when delivery cannot be
  completed, so the user can paste it manually.

## Delivery flow

1. Immediately before recording begins, capture the current Windows foreground
   window handle. The Wisper overlay must not become the foreground window.
2. When transcription completes, write the transcript to the Windows clipboard
   and wait for the host clipboard operation to complete.
3. For a paste-enabled delivery mode, restore the captured target window only
   when another window has taken foreground focus, then send a native Windows
   `Ctrl+V` input sequence to that target.
4. Report delivery as an attempted paste, because Windows cannot prove that a
   third-party application accepted clipboard content. If target restoration or
   input injection fails, report the reason and leave the transcript copied.
5. For `copy_only`, copy without injecting paste. For `paste_and_restore`,
   restore the previous clipboard text only after the paste delivery delay.

## Unified entry points

Automatic completion, the tray action, the transcript-card action, and history
re-paste must all call one backend delivery command that accepts the explicit
text to deliver. That command reads the current saved delivery mode, rather than
hard-coding `copy_and_paste`. This removes divergent behavior and ensures
history re-paste uses the requested history text, not merely the latest record.

## Verification

- Unit-test delivery-mode parsing and the decision to inject or omit paste.
- Unit-test that every frontend paste action uses the unified command with its
  intended text.
- Run TypeScript and Rust checks.
- Manually verify in a Windows text field that a selected word is replaced after
  dictation, and verify that a simulated input failure leaves the transcript on
  the clipboard with an error message.
