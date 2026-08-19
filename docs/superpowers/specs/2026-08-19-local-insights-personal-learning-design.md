# Local Insights and Personal Learning

## Goal
Give Wisper a private Insights area that explains actual dictation habits and learns from user-approved corrections without storing recordings, reading screen content, or inventing conclusions.

## Privacy boundary
- Dictation history, metrics, correction rules, and style preferences stay on the local computer.
- Raw audio is not retained after transcription.
- App-usage tracking is opt-in and disabled by default.
- When enabled, Wisper records only a local category and the active process name, such as `Code.exe` or `OUTLOOK.EXE`. It never stores window titles, filenames, webpages, screen content, or typed text.
- Any future anonymous contribution feature is separate from this scope and must have a new explicit consent design.
- Gemini remains optional formatting only. The Insights feature never requires Gemini and must work in Local-only mode.

## Product surface
Add an `Insights` tab to the existing main navigation with two views.

### Your usage
- Total words dictated, completed dictations, and words per minute.
- Local fixes applied, separated into formatting fixes, vocabulary rules, and user-approved corrections.
- Activity breakdown by local category: coding, email, messages, documents, browser, and other.
- Dictation activity calendar, current streak, and longest streak.
- Factual milestones calculated from actual totals; no invented comparisons or personality claims.

### Your voice
- Most-used words and recurring phrases, excluding a local stop-word list.
- Malayalam/English language mix when discernible from transcript script.
- Correction pairs such as `heard -> approved`, with use count and last-used time.
- Vocabulary and style preferences learned only from explicit user actions.
- An honest empty state until enough data exists. The complete profile unlocks after 2,000 completed words; basic counts and correction controls remain available immediately.

## Local data model
Create a versioned local `insights.json` beside the existing Wisper settings/history files. It contains:
- aggregate totals by day and category;
- completed-dictation duration, word count, and timestamp;
- opt-in process-to-category mapping;
- correction rules with source, approved replacement, count, enabled state, and timestamps;
- vocabulary additions and style preferences;
- schema version.

Do not duplicate full transcript text in the Insights store. Existing local history remains the source for text-derived reports. Store only aggregates, rule metadata, and the minimum data needed to update reports.

## Event flow
1. A recording begins; Wisper captures monotonic start time.
2. If app tracking is enabled, Wisper reads the foreground process name locally and maps it to a category. Unknown processes are `other`.
3. A local transcript completes and is delivered.
4. Wisper records local duration, word count, timestamp, and category in the Insights store.
5. Local report functions derive counts, WPM, streaks, activity calendar, and word trends.
6. When the user corrects text, they choose either one-time correction, vocabulary addition, or a reusable replacement rule.
7. Only reusable user-approved rules are used in future dictations. A rule can always be disabled or deleted.

## Classification
Category classification is a small local mapping, not an AI judgement. Initial mappings cover known executables for code editors, email clients, document editors, and browsers. The UI lets the user override a category or exclude a process. No screen content is inspected.

## Learning safeguards
- No automatic replacement rule from a single transcript edit.
- Reusable rules require an explicit user choice.
- Rules have narrow matching, preserve case where possible, and are applied before final smart formatting.
- Conflicting or disabled rules are skipped and shown in the Insights UI.
- The app never claims to know emotion, personality, or speech quality from text alone.

## Failure behavior
- If Insights storage is unreadable, dictation and pasting still finish; Insights shows a clear local-storage warning.
- If foreground-process reading fails, record category `other` without blocking dictation.
- If an aggregate update fails, do not save partial/corrupt data; preserve the last valid file.
- If there are fewer than 2,000 words, show transparent progress toward the profile rather than generated conclusions.

## Delivery stages
1. Local Insights data store and safe lifecycle events.
2. Insights navigation and Your usage cards using only actual metrics.
3. Opt-in foreground-process category tracking and activity calendar.
4. Correction review controls, vocabulary learning, and reusable replacement rules.
5. Your voice reports, language mix, and 2,000-word profile gate.

## Verification
- Unit tests for WPM, streaks, word counts, process mapping, 2,000-word gate, and rule conflict behavior.
- Storage migration and corrupted-file recovery tests.
- Manual privacy check confirms disabled tracking writes no foreground-process data and enabled tracking stores no window titles.
- Manual dictation verifies Insights failure cannot block transcript delivery or auto-paste.
