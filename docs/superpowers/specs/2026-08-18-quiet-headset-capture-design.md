# Quiet headset capture

## Goal

Accept normal, low-output headset speech without requiring the user to raise their voice, while continuing to reject a disconnected or genuinely silent microphone.

## Design

The capture stage treats only extremely low raw energy as silence. All plausible speech proceeds to the existing 16 kHz preprocessing stage. The voice processor uses an RMS target of 0.16 and can apply a safe adaptive gain of up to four times the configured gain setting, limited by a 0.92 peak ceiling and soft limiter. This boosts quiet voice recordings without hard clipping loud speech.

The audio-gain control supports 1x through 8x and explains that it sets the maximum adaptive boost. A quiet-but-valid input is reported as boosted in diagnostics rather than rejected.

## Verification

Run the frontend build, `cargo fmt --check`, and `cargo check`. Test a normal speaking voice with the headset: it should transcribe without shouting. Test silence: it should still return the microphone/silence error.
