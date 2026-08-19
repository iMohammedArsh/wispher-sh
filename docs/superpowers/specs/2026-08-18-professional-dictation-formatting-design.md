# Professional dictation formatting

## Goal

Turn a local Whisper transcript into professional, well-structured text before it is saved and pasted, while retaining a reliable offline fallback.

## Design

After local transcription succeeds, Wisper checks whether a Gemini API key is configured. If it is, the existing Gemini formatter receives only the text transcript and returns a polished version: punctuation, grammar, filler removal, and appropriate paragraph or list structure while preserving meaning. That polished text is used for history, the UI, clipboard, and auto-paste.

If no key is configured or formatting fails, Wisper retains and delivers the raw local transcript. The delivery message identifies the fallback so the user knows why the result was not reformatted.

## Verification

Run the frontend build, `cargo fmt --check`, and `cargo check`. With no API key, a dictation must still save and paste its raw transcript with a clear formatting notice. With a valid key, the formatted text must be the saved and pasted result.
