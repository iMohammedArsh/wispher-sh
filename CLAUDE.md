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
# Notes chunking on real speech: a 45 s+ WAV (`say -f notes.txt -o /tmp/note.wav ...`) as HAMS_TEST_NOTE_WAV
cargo test bindings               # regenerates src/lib/bindings.ts; fails once if it was out of date
scripts/install-macos.sh          # release build → /Applications (signs with "Hams Local Signing" if present)
scripts/release-macos.sh          # signed build + draft GitHub Release with latest.json (see Updates below)
scripts/build-icon.sh             # regenerate all icons from icons/source + Hams.icon (needs Xcode's actool)
```

The npm `tauri` script prepends Homebrew rustup to `PATH`. Do the same in a shell if `cargo` isn't found. Dev builds compile `whisper-rs-sys`, `rubato` and the FFT crates at `opt-level = 3` (set in `Cargo.toml`), otherwise model load and inference are unusably slow. `build.rs` links clang's `libclang_rt.osx.a` because whisper.cpp's Metal code uses `@available`, and release links fail with an undefined `___isPlatformVersionAtLeast` without it.

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
| `overlay.rs` + `ui.rs` | `TauriUi` emits `status`, `level` (overlay only) and `data-changed` events. The overlay window (label `overlay`) is shown without taking focus: `orderFrontRegardless` on macOS, `SW_SHOWNOACTIVATE` plus `WS_EX_NOACTIVATE` on Windows. It uses one fixed pill size (`PILL`, 232×48) for every phase so it never jumps, a taller size only for live preview and a wider one for errors. It's placed bottom-center on the monitor under the cursor, and hides itself after Done or Error. |
| `commands.rs` | Thin IPC wrappers, registered in `lib.rs`. |

**Frontend.** There are two Vite entry points, `main.html` and `overlay.html`. The overlay bundle (`src/overlay/`) must stay tiny, so it never imports the app's routes or CSS.
- `src/lib/bindings.ts` is generated from the Rust types by `cargo test bindings` (`src-tauri/src/bindings.rs`; specta is a dev-dependency, enabled with `#[cfg_attr(test, derive(specta::Type))]`). When you add a type the frontend receives, derive it there and register it in `bindings.rs`. `src/lib/ipc.ts` re-exports the `_Serialize` shapes under short names and holds the hand-written command names and arguments.
- `src/lib/state.svelte.ts` is a single rune store fed by events.
- Screens live in `src/routes/`, shared parts in `src/components/` (`Icon`, `Mark`, `ConfirmButton`; use the latter instead of `window.confirm`). Styling is plain CSS tokens in `src/styles/app.css` plus scoped component styles. There's no Tailwind. The look is deliberately quiet: ink on a sage-tinted ground, 1px borders (shadows only on floating things: toast, `.popover`, overlay), a near-black primary button, system font, type scale 12/13/14/16/20/22 at weights 400/500/600, and colour only on data (`--data` green for charts, changed words and "on" states, `--live` red while recording).
- **The signature element is the Heard/Written pair** (`components/HeardWritten.svelte`): the raw Whisper text (`HistoryItem.raw`) above what was written, with dropped words struck through and added or changed words underlined in `--data`. `lib/diff.ts` does the word-level LCS diff (case and punctuation ignored). It's used on Home, in expanded History rows, in Onboarding, and in the "Try it" panels of Styles and Settings → AI polish (`style_try` runs `controller::restyle` on sample text and saves nothing). Build new dictation views around it rather than adding stat tiles.
- `RewriteMenu` (calls `history_restyle`) and `TeachWord` (a popover that saves to the Dictionary, pre-filled from the selected text) are shared by Home and History.
- Home's "Last dictation" and "Earlier" use `history_recent` (newest first). `history` sorts pinned items first, so don't use it for "latest".
- Settings shows one area at a time: `app.settingsArea`, set with `app.openSettings("speech")`. `app.pendingSuggestions` drives the Dictionary nav count.
- `vite dev` in a plain browser (no Tauri) serves sample data from `src/lib/mock.ts`. It's imported only behind `import.meta.env.DEV`, so it's never bundled. Open a screen directly with `http://127.0.0.1:1430/main.html#notes` or `#settings/speech`. Add `?empty` for a first run (no history, Accessibility off) or `?onboarding` for the setup flow, e.g. `main.html?empty#home`.

## Status and known gaps

- The Windows module (`platform/windows/`) is ported from v1 but has **not been compiled**. A macOS cross-check needs the MSVC SDK headers (for example via `cargo-xwin`). Build and test it on Windows before relying on it.
- Not yet built: UI Automation on Windows (selected text and field read-back). Commands are still invoked by name in `ipc.ts` (only the data types are generated).
- Warm mic (`keep_mic_warm`, off by default): the audio thread keeps the cpal stream open between dictations, and the callback fills a 700 ms `preroll` ring that a new recording starts with. The stream is reopened if it errored, the chosen device changed, or (for "System default") the default input changed. `AppState::apply_microphone` applies the setting at startup and on save.
- Rewrite from History (`history_restyle` → `controller::restyle`): starts from the stored `raw` Whisper text, runs the normal clean → polish (with `styles::configure` for the chosen style) → dictionary → `Style::finish` path, and saves in place. AI errors are reported, not hidden.
- Updates (`updater.rs`, `tauri-plugin-updater`): release builds check `https://github.com/iMohammedArsh/wispher-sh/releases/latest/download/latest.json` 30 s after launch and every 6 h (`check_updates`), and emit `update-available`. Install happens only on click, then the app restarts. The public key is in `tauri.conf.json`; the private key is `~/.tauri/hams.key` (never in the repo; losing it means existing installs can't update). `install-macos.sh` signs update archives when the key exists and skips them otherwise.
- Notes chunk cuts (`dsp::take_chunk` / `quiet_split`) look for the quietest ~200 ms stretch, not one frame, so a silent stop consonant inside a word isn't chosen. `stt::strip_stock_phrases` drops subtitle hallucinations ("Thanks for watching") per sentence; short phrases like "Thank you." are only dropped when they're the whole transcript.
- Notes mode (`Kind::Note`, `Input::Note`, from the Notes screen or the tray): `spawn_note_chunks` drains about 30 s of audio at a time (`AudioRecorder::drain` cuts at a pause via `dsp::quiet_split`), transcribes each chunk while recording continues and emits `note-progress`. `process_note` adds the tail, asks the AI for a title and key points (`Task::Summarize`, 60 s minimum timeout, skipped under 30 words), saves to the `notes` table (migration v2) and never pastes. Escape and the dictation shortcuts don't touch a note in progress.
- Learning from fields (`learn_from_fields`, opt-in, macOS only): after a paste, `spawn_field_learning` re-reads the focused field through `Desktop::field_text` (AX, skips secure fields) every 5 s for a minute. Once the text settles, `text::learn_from_field` only learns from edits *inside* the pasted text and adds suggestions, never replacements. The field text isn't stored.
- Language detection: with `language = "auto"`, `Settings.languages` ("Languages you speak", main language first) restricts detection. `WhisperEngine` runs `lang_detect` on the first 30 s and `stt::pick_language` keeps only allowed codes; a non-main language needs `SURE_SHARE` (70%) of the allowed probability, otherwise the main one wins. This matters because Whisper *writes in the language it detects*: accented English detected as Malay comes out translated into Malay. Voice notes call `Transcriber::resolve_language` once on the first chunk with speech and lock that language for the rest of the note and the tail. `real_speech_accent` (ignored) checks this with `say -v Amira` (the macOS Malay voice reading English).
- Styles: `styles::resolve` applies `Settings.app_rules` (case-insensitive match on bundle id/exe or app name) before `detect`, then `style_overrides` (prompt and AI on/off; the UI stores only differences from the built-ins).
- Live preview (`live_preview`, off by default): `spawn_preview` in the controller re-transcribes the last 20 s every second and emits `partial` to the overlay. It's display-only. Its task is aborted with the recording, but an in-flight Whisper call can delay the final transcription by one preview pass.
- App lifecycle: `tauri-plugin-single-instance` (must stay the first plugin) and `tauri-plugin-autostart` (LaunchAgent with `--hidden`, which starts in the menu bar). On macOS the Dock icon is shown only while the main window is open.
- Icons: `icons/Hams.icon` (Icon Composer) → `icons/macos/Assets.car`, which is bundled as a resource with `CFBundleIconName` = `Hams` in `Info.plist`. The tray uses `icons/tray/tray-template@2x.png` as a template image.
- The design specs in `docs/superpowers/` describe v1 but still record the product intent: privacy, overlay behaviour, formatting rules and Malayalam support.
