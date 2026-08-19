# Wisper Product UI Redesign

## Purpose

Replace the current dark, utility-heavy main window with a calm, premium desktop workspace for local voice writing. The existing black floating overlay remains a separate, compact interaction surface above the Windows taskbar.

This redesign draws inspiration from the clarity, warmth, spacing, and information hierarchy in the supplied Flow references. It does not reproduce Flow branding, copy, imagery, premium prompts, team features, or screen layouts.

## Design direction

**Design read:** a privacy-first Windows voice-writing application for an individual user, with a warm editorial workspace and a quiet, functional delivery overlay.

- Theme: warm off-white workspace, ink-black text, soft stone borders, restrained forest-green status accent.
- Shape: 12 px controls, 16 px content cards, 22 px feature panels. No visible glass, neon, gradients used as decoration, or high-contrast dark dashboard surfaces.
- Typography: high-legibility sans-serif for all working UI. A display serif may be used only for large feature-panel headlines, never for controls, navigation, or transcripts.
- Motion: minimal and purposeful. Navigation/content transition is 150 to 200 ms. Recording, processing, success, and error remain represented by the native overlay rather than full-window animations.
- Accessibility: visible keyboard focus, AA contrast for all interactive elements, no meaning conveyed only through color, and no permanently low-contrast text.

## Product boundary

The live app currently provides Dictation, History, Insights, Vocabulary, Settings, Help, native overlay states, local speech recognition, optional formatting, and paste/copy delivery.

The following are product concepts only and must not appear as working navigation until implemented: Notes, Snippets, Style presets by destination app, Transforms, Scratchpad, account, sharing, team features, billing, and cloud sync. Their visual direction can be developed later as independent modules.

## Information architecture for the first redesign

The first implementation keeps only real screens in the navigation:

1. **Dictation**: the home workspace and primary action.
2. **History**: past dictations with copy and paste actions.
3. **Insights**: private local usage and correction progress.
4. **Vocabulary**: the local custom dictionary.
5. **Settings**: configuration and privacy controls.
6. **Help**: Windows setup and troubleshooting.

The sidebar has the wordmark at the top, the six product destinations in the middle, and Settings/Help at the bottom. It contains no trial, upgrade, referral, sharing, account, or team elements.

## Dictation home screen

The home screen is the product’s central workspace.

### Header

- Eyebrow: `VOICE WORKSPACE`.
- Heading: `Write without breaking your flow.`
- One sentence explains that Wisper records locally and delivers text to the selected field.
- A compact shortcut reminder on the right shows the actual active hotkey. It never displays a hard-coded shortcut different from Settings.

### Primary panel

- A single expressive, dark photo-like color panel introduces dictation.
- It contains the button that starts/stops dictation and a short statement about using Wisper from any selected input.
- This is not a fake waveform, a fake microphone meter, or a duplicate of the native overlay.
- When recording begins, the panel changes only to a clear state summary; the native overlay remains the live visual feedback.

### Private progress panel

- Uses actual local data only: total words, average WPM where available, and streak where available.
- Clearly labels its storage location as local.
- Displays an honest empty state if insufficient dictation data exists.

### Today list

- Uses real History records.
- Shows time, short text preview, and outcome such as Pasted or Copied.
- Provides a link to History instead of a non-functional search or filter.

## History

History becomes a readable activity list rather than a technical stream.

- Page title and a brief local-storage message.
- Search and sort controls only if wired to working behavior.
- Each entry provides timestamp, text, word count, copy, and paste actions.
- Empty state explains that completed dictations appear here.

## Insights

Insights is a privacy-first account of usage and learning.

- At top, a local-data explanation and a visible opt-in status for app-category tracking.
- Metric cards use actual values: total words, WPM, current/longest streak, and correction rules.
- App category breakdown appears only after the user has opted in and categories exist. It never stores window titles, typed text, or documents.
- Voice-learning progress explains the 2,000-word threshold without creating an invented personality profile.
- Corrections use a small `Heard` to `Use instead` control and clearly state that rules run locally.
- Calendar visualisation, phrase reports, and correction-rule management are deferred until their data and controls are implemented.

## Vocabulary

Vocabulary becomes the user’s local language memory.

- Main action: `Add word`.
- Current custom terms displayed as editable, searchable rows or chips.
- Existing save behavior remains explicit; no fabricated team/shared tabs.
- Copy explains use for names, Malayalam/English terms, product names, and technical vocabulary.

## Settings

Settings moves from a dense single form to a modal-like, two-column settings workspace within the main window.

- Left list: General, Dictation, Delivery, Privacy, Advanced.
- General: active hotkey; app-language selection is excluded until the app has translated strings.
- Dictation: microphone, language, model, audio gain.
- Delivery: copy/paste mode and delivery safety behavior.
- Privacy: app-category tracking and local storage explanation.
- Advanced: Gemini formatting key/model and binary/model paths.
- The save action remains visible, and every option preserves the existing backend setting names/behavior.

## Overlay relationship

The overlay stays native, black, compact, and positioned above the taskbar. The main-window redesign must not resize, re-theme, or replace it.

Overlay states remain:

- Idle: small, translucent black pill without controls.
- Recording: fixed-size black pill with Cancel, waveform, Finish.
- Processing: fixed-size black pill with processing motion.
- Error: expands temporarily for a concise actionable message, then returns to idle.

## Component structure

- `AppShell`: title bar, sidebar, content container, responsive layout.
- `SidebarNavigation`: real destinations only; derives active state from `ActiveTab`.
- `DictationHome`: header, primary dictation panel, live local progress, today list.
- `HistoryPage`: presentation layer above the existing history data/actions.
- `InsightsPage`: presentation layer above local Insights commands/data.
- `VocabularyPage`: presentation layer above custom dictionary save behavior.
- `SettingsWorkspace`: sectional layout wrapping the existing settings inputs.
- `EmptyState` and `LocalPrivacyNote`: reusable honest states.

The React components remain presentation-oriented; Rust commands and the overlay implementation are unchanged unless a screen needs a small, well-defined data field.

## Delivery order

1. Establish design tokens and application shell with real navigation.
2. Redesign Dictation home using existing status, history, settings, and Insights data.
3. Restyle History, Insights, Vocabulary, and Settings without changing feature behavior.
4. Add polished empty, loading, and error states.
5. Test at the actual desktop window sizes, keyboard navigation, and light-mode contrast.
6. Do not add the conceptual feature modules until separately designed and implemented.

## Acceptance criteria

- The live main window visually follows the approved mockup direction.
- No existing dictation, hotkey, paste, copy, model, language, privacy, History, or Insights behavior regresses.
- All user-visible information is either live data or clearly an empty state.
- The overlay is unaffected and remains visually independent.
- The app does not contain competitor names, copied branding, trial/billing UI, pretend app integrations, or fake analytics.
- `npm run build` and `cargo check` pass after implementation, followed by a user-run desktop verification.
