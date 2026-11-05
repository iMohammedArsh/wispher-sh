# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Hams is a voice-typing desktop app for macOS and Windows, built with Tauri 2: Rust in `src-tauri/`, Svelte 5 + TypeScript in `src/`. The user holds a shortcut and speaks. Local Whisper transcribes the audio, then the text goes through local clean-up, an optional single AI polish call, and is pasted into the app that had focus. Bundle id `com.wusla.hams`. The repo directory and the v1 data folder still use the old name "wisper".

v2 is a full rewrite on branch `rewrite/v2`. v1 (React, a `whisper-server` subprocess, JSON files) is in git history on `recovery/icloud-2026-10-21`. `RECOVERY-NOTES.md` explains the iCloud recovery: don't delete duplicate copies of the project.

## Commands

```bash
npm ci
npm run tauri:dev                 # app in dev mode (Vite strict port 1430, devUrl /main.html)
npm run check                     # svelte-check (the frontend type/lint gate)
npm test                          # vitest (src/**/*.test.ts)
npm run build                     # vite build → dist/
cd src-tauri
cargo test                        # all Rust tests (builds whisper.cpp; first build takes a few minutes)
cargo test --no-default-features  # fast: skips the local-stt feature (no C++ build)
cargo test controller::           # one module; or a single test name
# Real speech, end to end (macOS, needs the "small" model downloaded):
say -o /tmp/hams.wav --data-format=LEI16@22050 "Hello from Hams. This is a dictation test."
HAMS_TEST_WAV=/tmp/hams.wav cargo test --release real_speech -- --ignored --nocapture
```

The npm `tauri` script prepends Homebrew rustup to `PATH`. Do the same in a shell if `cargo` isn't found. Dev builds compile `whisper-rs-sys`, `rubato` and the FFT crates at `opt-level = 3` (set in `Cargo.toml`), otherwise model load and inference are unusably slow.

## Architecture

**`controller.rs` is the core.** A single Tokio actor owns the record → transcribe → clean up → polish → deliver → persist flow. It receives `Input`s (hotkey actions, UI toggle/stop/cancel, paste-last) over a channel and processes them one at a time. Everything else only sends messages to it. That's why overlapping recordings, double stops and state drift can't happen, so keep it that way: don't add recording state anywhere else.
- Tap vs hold timing lives here, not in the keyboard layer: `TAP` (220 ms) and `DOUBLE_TAP_WINDOW` (350 ms). A quick tap waits as `PendingTap`. A second tap switches to `Locked` (hands-free). A lone tap is cancelled silently.
- Dependencies are traits in `Deps`: `Recorder`, `Transcriber`, `Desktop`, `Polisher` (built per dictation by `PolisherFactory`), and `Ui`. The tests in `controller.rs` drive the actor with fakes under paused Tokio time. Extend those tests when you change controller behaviour.
- Pipeline order matters:
  1. `text::clean` handles fillers and the voice commands "scratch that", "new line" and "new paragraph".
  2. Then either a snippet expansion (whole-utterance match only, skips AI) or the AI polish. The polish is skipped for code-style apps and for dictations under `smart_skip_words`.
  3. `text::apply_replacements` runs *after* polish, so the AI can't undo dictionary fixes.
  4. `Style::finish`.
  5. Deliver.
  6. Write history **after** delivery.
- Dictating into Hams itself (target pid == own pid) is forced to copy-only.

**Keyboard input** is split into platform-neutral logic and OS adapters.
- `input/shortcut.rs` parses shortcut strings like `"Meta+Shift+Space"`, `"Fn"` and `"RightAlt"`.
- `input/matcher.rs` is a pure state machine: raw key → `HotkeyAction` plus whether to swallow the event.
- The OS layers (`platform/macos/hotkeys.rs`, `platform/windows/hotkeys.rs`) only translate native key codes into `KeyId` and drop swallowed events.
- macOS uses a raw-FFI `CGEventTap` on its own run-loop thread. The `core-graphics` wrapper can't drop events, which is why it isn't used. Without Accessibility permission it falls back to listen-only.
- Windows uses `WH_KEYBOARD_LL`, ignores injected events, and sends a "menu mask" key when a solo Alt/Win shortcut is released.
- `AppState::apply_shortcuts` updates the shared `Matcher` live when settings change.

**Platform seam (`platform/mod.rs`).** `NativeDesktop` implements `Desktop`: frontmost app, focus restore, paste with clipboard save/restore (`platform/clipboard.rs`, restore happens 450 ms later and only if the clipboard is unchanged), and selected text. Selected text tries Accessibility first and falls back to a ⌘C/Ctrl+C probe, which is disabled for code editors because they copy the whole line. The per-OS modules expose the same free functions.

**Other modules**

| Module | What it does |
|---|---|
| `audio/` | A dedicated thread owns the cpal stream; `AudioRecorder` talks to it over a channel. `dsp::prepare` does FFT resampling to 16 kHz with rubato, rumble removal, adaptive-threshold VAD trim and capped normalization. Recordings with no speech are dropped silently. |
| `stt/` | `WhisperEngine` keeps a whisper.cpp context and state loaded in-process, behind the `local-stt` cargo feature (default on). `models.rs` is the catalog and resumable `.part` downloads into `paths::models_dir()`. `sanitize` strips Whisper annotations and common hallucinations. |
| `llm/` | One `LlmClient` covers Gemini, Anthropic and the OpenAI-compatible providers (OpenAI, Groq, OpenRouter, Ollama, custom). It uses a hard timeout and never puts keys in URLs. `prompts.rs` wraps user text in tags and escapes `</`; the prompt says the transcript is data, not instructions. `styles.rs` maps bundle id or exe name to a `Style`. |
| `store.rs` | SQLite via rusqlite (bundled), migrated through `PRAGMA user_version` with entries in `MIGRATIONS`. Tables: kv (settings JSON), history plus an FTS5 mirror kept in sync by triggers, dictionary, snippets, and suggestions (learned corrections waiting for user approval). |
| `settings.rs` | `Settings` uses `#[serde(default)]` throughout, so new fields need sane defaults. Saved settings are checked by `validate()`. API keys live in the OS keychain under service `com.wusla.hams` and are cached by `SecretCache`. Keys from the earlier `com.mohammedarsh.hams` service are migrated when first read. |
| `import_v1.rs` | Runs once and reads `dirs::config_dir()/wisper/*.json`: settings, Gemini key, vocabulary, history, correction rules. It hard-links an existing v1 model when it can. |
| `overlay.rs` + `ui.rs` | `TauriUi` emits `status`, `level` (overlay only) and `data-changed` events. The overlay window (label `overlay`) is shown without taking focus: `orderFrontRegardless` on macOS, `SW_SHOWNOACTIVATE` plus `WS_EX_NOACTIVATE` on Windows. It's resized for each phase, placed bottom-center on the monitor under the cursor, and hides itself after Done or Error. |
| `commands.rs` | Thin IPC wrappers, registered in `lib.rs`. |

**Frontend.** There are two Vite entry points, `main.html` and `overlay.html`. The overlay bundle (`src/overlay/`) must stay tiny, so it never imports the app's routes or CSS.
- `src/lib/ipc.ts` holds hand-written types that mirror the Rust serde structs. Keep them in sync when you change a Rust type.
- `src/lib/state.svelte.ts` is a single rune store fed by events.
- Screens live in `src/routes/`. Styling is plain CSS tokens in `src/styles/app.css` plus scoped component styles. There's no Tailwind.

## Status and known gaps

- The Windows module (`platform/windows/`) is ported from v1 but has **not been compiled**. A macOS cross-check needs the MSVC SDK headers (for example via `cargo-xwin`). Build and test it on Windows before relying on it.
- Not yet built: UI Automation selected-text on Windows, reading edited fields back for learning (only History edits are learned today), user-editable style prompts, live partial transcripts, Notes (long recording) mode, autostart, single instance, updater, and `tauri-specta` generated bindings.
- The design specs in `docs/superpowers/` describe v1 but still record the product intent: privacy, overlay behaviour, formatting rules and Malayalam support.
