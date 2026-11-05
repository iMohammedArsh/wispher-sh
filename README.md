# Hams

Hams is a voice typer for macOS and Windows. Hold a shortcut in any app, speak, and let go: clean text appears at your cursor.

- **Local speech recognition.** Whisper runs inside the app, Metal-accelerated on Apple Silicon. It works offline, and your audio never leaves the computer.
- **Optional AI polish.** One call to Gemini, OpenAI, Anthropic, Groq, OpenRouter, a custom endpoint, or a local Ollama model. Only the transcript text is sent, never audio. If the call fails or times out, Hams pastes the locally cleaned text.
- **Command mode.** Select text, hold the command shortcut, and say "make this more formal" or "translate to English".
- **App-aware styles.** Writing is casual in chat apps, formal in email, and raw in code editors and terminals.
- **Snippets, a personal dictionary that learns from your edits, voice commands** ("new line", "scratch that"), **Esc to cancel, and double-tap for hands-free.**
- **Searchable history and local stats.** Everything is stored in one SQLite file. API keys live in the OS keychain.

## Development

Requirements: Node 20+, Rust (stable), CMake and Xcode Command Line Tools (macOS). On Windows you need Visual Studio Build Tools instead.

```bash
npm ci
npm run tauri:dev     # run the app (Vite on 127.0.0.1:1430 + Rust)
npm run check         # svelte-check + TypeScript
npm test              # frontend unit tests (vitest)
npm run build         # production frontend build
npm run tauri:build   # installer / .app + .dmg
(cd src-tauri && cargo test)   # Rust unit tests
```

The first launch walks you through permissions and the model download. Models are stored in `~/Library/Application Support/Hams/models` (macOS) or `%APPDATA%\Hams\models` (Windows).

macOS needs **Accessibility** access for the shortcut, pasting and reading selected text, and **Microphone** access, which macOS asks for the first time you dictate.

Hams v1 data (`…/wisper/`) is imported once on first launch. The old files are read and never modified.
