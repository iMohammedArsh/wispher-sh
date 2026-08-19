# Malayalam and noise-aware dictation

## Goal

Support English and Malayalam dictation, including mixed-language use, while reducing background non-speech processing and retaining soft speech.

## Design

Wisper uses the multilingual Whisper `small` model for Auto and Malayalam modes, and passes the selected language to whisper.cpp (`auto`, `en`, or `ml`). English-only models remain available on disk but are not selected for Auto or Malayalam. The initial transcription prompt explicitly preserves short English terms and uses the configured custom vocabulary.

Before transcription, whisper.cpp's bundled Silero VAD filters audio to speech segments. VAD is configured with a low speech threshold, short minimum speech duration, and speech padding to avoid cutting off a soft whisper. If the VAD model is absent, transcription continues without VAD rather than failing.

## Verification

Install official multilingual `small` and Silero VAD models. Build frontend and Rust. Confirm Settings exposes Auto, English, and Malayalam. Confirm the CLI receives the language and VAD options when both model files exist.
