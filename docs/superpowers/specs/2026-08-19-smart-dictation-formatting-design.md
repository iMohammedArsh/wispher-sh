# Smart dictation formatting

## Goal

Turn a completed voice transcript into clear, professional writing without changing its meaning, inventing facts, or losing Malayalam and English content.

## Scope

This feature starts after local speech recognition has completed. It does not change microphone capture, Whisper model selection, hotkeys, delivery, or the overlay.

## Data flow

1. Local Whisper produces the raw transcript.
2. If a Gemini API key is configured, Wisper sends the transcript and the user vocabulary to the existing text-only formatter.
3. The formatter returns only the completed text.
4. Wisper copies and pastes the result through the existing delivery path.
5. If formatting is unavailable, empty, or fails, Wisper delivers the unmodified raw transcript and reports that formatting was skipped.

Recorded audio is never sent to the formatting service.

## Smart formatting rules

- Preserve the speaker's language. Keep Malayalam in Malayalam script and preserve English words in mixed-language dictation.
- Correct obvious grammar, capitalization, punctuation, spacing, and paragraph breaks.
- Convert clearly enumerated speech such as "first", "second", and "third" into an ordered list only when there are at least two distinct items.
- Convert explicit bullet or item dictation into a bullet list.
- Add a question mark only for an actual question.
- Use a heading only when the speaker explicitly dictates a heading or title.
- Keep action requests concise and structured when the speaker clearly gives tasks or next steps.
- Preserve names, jargon, acronyms, commands, numbers, URLs, and the user's custom vocabulary exactly where possible.
- Do not add explanations, greetings, summaries, advice, facts, or content not present in the dictated text.
- Do not convert a short message into a long document.

## Formatter instruction

The existing Gemini formatter will be changed from a generic grammar prompt to a precise smart-dictation instruction covering the rules above. It will return only final paste-ready text and will include the existing custom vocabulary as protected context.

## Failure handling

- Missing API key: raw transcript is delivered; UI states that Smart formatting is off.
- Request error, invalid response, or empty formatted response: raw transcript is delivered; the delivery notice names the formatting problem.
- Local transcription failure remains a transcription failure and must not call the formatter.

## Verification

Automated checks cover formatter prompt construction and existing Rust/TypeScript builds. Manual checks use:

1. A spoken three-point list that becomes a numbered list.
2. A spoken question that ends in a question mark.
3. A short single-sentence message that stays a short paragraph.
4. Malayalam and mixed Malayalam-English text that retains the original languages.
5. A disabled/failed formatter that pastes the raw transcript.
