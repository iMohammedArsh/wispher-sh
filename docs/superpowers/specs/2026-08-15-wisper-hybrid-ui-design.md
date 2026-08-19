# Wisper hybrid desktop UI design

## Objective

Replace the current dark, developer-oriented UI with a premium Windows desktop interface inspired by Apple’s calm utility apps and Linear’s organized productivity software. Preserve every existing Wisper Flow-style dictation capability and all Tauri commands.

## Product principles

- Make voice writing the obvious primary action.
- Use a warm mineral interface rather than pure white or a blue-gradient “AI” aesthetic.
- Feel native on Windows: familiar window controls, keyboard shortcuts, and compact desktop density.
- Keep AI in the background. Use plain human language such as “Natural & clear,” not decorative AI labels or chat-bot patterns.
- Make recording, processing, success, and error states unmistakable without visual noise.

## Visual system

### Tokens

| Role | Value | Use |
| --- | --- | --- |
| App canvas | `#E8E8E3` | Outer window background; not pure white |
| Main surface | `#F8F8F5` | Reading and writing content surface |
| Sidebar/title bar | `#EFEFEB` / `#ECECE7` | Quiet structural regions |
| Divider | `#DDDED8` | Borders and separators |
| Ink | `#1C1D1B` | Primary text |
| Muted ink | `#656861` | Descriptions and secondary controls |
| Action green | `#276149` | Record action, positive status, focus accents |
| Error red | `#B7433C` | Error notice and close-button hover only |

Typography uses the platform UI font stack for controls and a restrained serif display face for the primary page heading. No gradients, mascots, animal imagery, floating sparkles, or oversized AI illustrations.

## App shell

The normal Tauri window becomes a frameless custom shell with a 48px title bar. It contains the `wisper` wordmark, current page title, and right-aligned Windows controls: minimize, maximize/restore, and close. Controls retain their expected Windows behavior. They have neutral hover states; only Close becomes red on hover. The title-bar region remains draggable except over interactive controls.

Below it, the app uses a 205px sidebar and a flexible content pane. The sidebar contains Write, History (with a count), Vocabulary, and Settings; the current page has a soft gray selection state. Help and shortcuts sit quietly at the bottom. At narrow sizes, hide the sidebar and retain access through a compact navigation control.

## Core experience

### Write page

1. Heading: “Write by voice.”
2. A small mode chip showing the selected transform mode, such as “Natural & clear.”
3. A centered, soft stone recording surface with one circular green hold-to-speak control, the current instruction, and the active hotkey.
4. A compact “Last transcript” card below it. When idle, it shows the latest delivered dictation. Copy and paste actions remain available in this card but are secondary.

State transitions:

- Idle: green record control, “Hold to speak,” key hint.
- Recording: control gains a gentle pulse; label switches to the active recording mode and a live duration/audio indicator appears.
- Transcribing/pasting: control is disabled, label communicates the operation, and a small progress indicator appears.
- Delivered: transcript updates, success status appears in muted green, and normal record affordance returns.
- Error: a concise inline error replaces the status text and exposes a retry action. The rest of the page stays usable.

### Secondary pages

History uses the same content pane with a searchable chronological transcript list. Each item has clear copy and paste actions. Vocabulary uses chips/rows and an obvious save state. Settings groups controls by Dictation, Text delivery, AI cleanup, and Advanced, avoiding one long form. The existing accessibility guide remains available from Settings or Help rather than occupying a top-level primary tab.

### Floating overlay

The existing Dynamic Island window becomes a compact warm-charcoal or deep-ink status pill, independent of the main light theme so it stays readable over arbitrary applications. It uses the same status language, a small microphone/waveform indicator, and no gradients. It must remain unobtrusive and have no interaction requirements during active dictation.

## Technical boundaries

- Keep `App.tsx` as the integration layer for Tauri invocations, status listeners, settings, and history state.
- Introduce presentational shell components: `AppShell`, `Sidebar`, `TitleBar`, and `DictationStatusSurface`.
- Existing `DictationStudio`, `TranscriptCard`, `HistoryStream`, `VocabularyManager`, and `SettingsPanel` retain their data contracts; update their markup/classes incrementally to fit the new system.
- Use the Tauri window API for minimize, toggle maximize, close, and drag-region behavior. Do not alter Rust dictation, hotkey, microphone, clipboard, or persistence code.
- Define shared semantic color and spacing tokens in the global stylesheet; do not scatter raw color literals across components.

## Accessibility and quality requirements

- All normal text and actionable icons meet at least 4.5:1 contrast against their active surfaces.
- Buttons and icon controls have visible keyboard focus and accessible labels.
- The record action is at least 44px in each dimension; the visual target will be 72px.
- Motion is limited to 150–300ms state feedback and respects reduced-motion preferences.
- Do not communicate recording, processing, or errors through color alone.
- Preserve the current keyboard-first workflow and all existing Tauri command behavior.

## Verification

1. Run TypeScript and Vite production build successfully.
2. Verify normal window control behavior on Windows: minimize, maximize/restore, close, and drag.
3. Exercise dictation state changes: idle, recording, transcribing, delivered, and failure.
4. Verify copy, paste, history, vocabulary save, and settings save still call the existing commands.
5. Check keyboard navigation, focus visibility, reduced motion, and light-surface contrast.
6. Compare desktop and narrow-window layouts for clipping, horizontal scrolling, and control reachability.

## Scope guardrails

This release is a visual/interaction-shell overhaul. It does not add cloud sync, billing, user accounts, a new model provider, or changes to the speech-to-text pipeline. Those are separate business milestones after the redesigned core experience is validated.
