# Modifier-only hands-free hotkey

## Goal

Use Ctrl + Shift + Alt alone as the hands-free toggle, without requiring Space, while preserving Ctrl + Shift + Space for hold-to-talk.

## Design

The Windows low-level hook detects the key-down of the final modifier when Ctrl, Shift, and Alt are all held. It emits exactly one hands-free toggle event and marks the chord held. Releasing any modifier resets the chord, allowing the next complete chord to toggle again. Space is intentionally excluded from the toggle path.

The state machine remains unchanged: first chord starts hands-free recording; the next chord stops it and begins processing. It ignores toggles while a different non-recording operation is underway.

## Verification

Run `cargo fmt --check` and `cargo check`. Confirm terminal startup logging advertises Ctrl + Shift + Alt for hands-free. Test two separate complete chords: start, then stop.
