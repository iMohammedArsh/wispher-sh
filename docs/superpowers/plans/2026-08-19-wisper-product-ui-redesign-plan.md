# Wisper Product UI Redesign Implementation Plan

## Goal

Implement the approved warm, privacy-first Wisper main-window redesign without changing native overlay, transcription, delivery, or settings semantics.

## Steps

1. Create app-wide visual tokens and replace the old dark shell with the new off-white window, sidebar, content canvas, focus states, and responsive behavior.
2. Align the navigation to the real product screens and remove presentation that implies unimplemented modules.
3. Rebuild the Dictation page as a home workspace using existing status, hotkey, history, and Insights data.
4. Restyle History, Insights, Vocabulary, Settings, and Help while keeping their existing commands and behavior unchanged.
5. Add reusable local-privacy and empty-state presentation, then verify keyboard focus and contrast.
6. Run frontend and Rust build checks. The user then verifies the actual desktop window; do not launch or stop their app process.

## Constraints

- Do not modify `DynamicIsland`, Rust overlay code, hotkey code, audio capture, or paste delivery code.
- Do not display invented analytics, app integrations, trials, upgrades, teams, or cloud capabilities.
- Existing settings fields must still save through the same Tauri commands.
- Future feature concepts remain out of the live sidebar.
