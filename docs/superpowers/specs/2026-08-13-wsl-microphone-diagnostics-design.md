# WSL Microphone Capture and Diagnostics

## Goal

Make Wisper reliably capture speech through WSLg's Windows-backed `RDPSource` microphone, provide immediate on-screen evidence of microphone health, and report transcription failures accurately rather than presenting them as successful dictation.

## Scope

This change remains a Linux/WSL Tauri build. It does not move the application to a native Windows build or add streaming transcription.

## Capture design

`AudioRecorder` will create the input stream synchronously so that a missing device, unsupported format, or stream-start failure is returned to the UI immediately. It will retain the active device name and its native input sample rate, convert all supported CPAL sample formats to mono `f32`, and resample the captured signal to 16 kHz before creating the WAV passed to whisper.cpp.

During capture, the recorder will publish a lightweight snapshot containing the device name, native sample rate, buffered duration, peak amplitude, RMS level, and clipping flag. The snapshot is emitted as a Tauri event at a bounded frequency so the UI can render a live meter without collecting per-sample events.

The recorder will reject recordings that are too short or whose RMS level indicates silence. It will warn on clipping, but will still retain the recording and attempt transcription.

## WSL audio configuration

The development command will retain the WSLg PulseAudio endpoint and select `RDPSource` only when it exists. It will no longer force a 200% source volume. Windows remains the authority for the microphone input volume.

## Application UI

The telemetry card will show:

- microphone connection state, device name, and native format;
- a live level meter while recording, including a clipping warning;
- buffered duration and capture state;
- a plain-language capture or transcription error with diagnostics.

After recording, the telemetry area will show the transcript only on successful transcription. If transcription fails, it will present an error heading and the audio diagnostics rather than injecting an error string into the clipboard or describing it as dictated text.

## Transcription and error handling

The Whisper command will retain stderr and return a specific failure when the CLI produces no usable speech. Failures will include actionable context: invalid model/binary paths, silent input, clipped input, or CLI output. Clipboard injection will occur only when non-empty transcribed text is returned.

## Verification

1. `cargo check` for the Rust backend and `npm run build` for the frontend pass.
2. In WSLg, the app reports `RDPSource` (or the active input device) and displays a moving level meter while speaking.
3. A normal-volume recording yields a 16 kHz mono WAV and Whisper text.
4. A silent recording reports silence; a deliberately loud recording reports clipping; neither is pasted into the active application.
