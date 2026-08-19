# Ultra-quiet whisper capture

## Goal

Recognize an intentionally very soft headset whisper when it is present in the microphone signal, without accepting a silent or disconnected microphone as dictation.

## Design

The raw-input silence floor is lowered to -82 dBFS. The post-filter processor no longer drops whisper-level signals. It normalizes valid quiet input toward an RMS of 0.12 and allows up to 800x adaptive gain for the current 2.5x saved setting (up to 1200x at higher settings). A peak ceiling and soft limiter remain in place.

This can amplify a real whisper, but cannot reconstruct words from a microphone signal that contains only room noise.

## Verification

Run the frontend build, `cargo fmt --check`, and `cargo check`. Test with a deliberately soft whisper and with silence. Whisper should reach transcription; silence should still report no voice.
