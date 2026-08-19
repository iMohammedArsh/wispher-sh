# Wisper quiet workspace design

## Goal

Make Wisper feel like a premium, calm voice-writing workspace. The desktop window is for reviewing and organizing work; the always-on-top overlay is the exclusive source of live dictation feedback.

## Structure

- A composed left navigation remains available across the workspace.
- The Write page gains a welcoming, high-readability introduction, a stable dictation launch surface, a recent-transcript area, and a compact right-side context rail.
- The right rail shows only useful local context: completed dictation count, saved-word count, and the shortcut/overlay behavior. It contains no advertising, trial messaging, fake achievements, or decorative imagery.
- At narrower widths, the existing compact navigation appears, the side rail moves beneath content, and no content is clipped.

## Live-state rule

- Starting, recording, processing, success, and error states do not change the main-window layout or labels.
- The overlay alone communicates live state. Idle: tiny jet-black capsule with a small microphone mark. Recording: compact jet-black island with microphone, divider, and real audio waveform.
- The transcript feed updates only when a completed dictation is available, so it behaves as a work record rather than a live status display.

## Boundaries

- Preserve the current Tauri commands, hotkeys, microphone pipeline, persistence, copy/paste, and history data.
- No Wispr Flow branding, paid-plan UI, marketing content, or copied artwork.
- Verify the main app remains visually stable during hotkey-driven recording, while the overlay transitions correctly.
