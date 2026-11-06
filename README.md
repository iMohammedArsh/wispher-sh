# Hams

Hams is a voice typer for macOS and Windows. Hold a shortcut in any app, speak, and let go: clean text appears at your cursor.

- **Local speech recognition.** Whisper runs inside the app, Metal-accelerated on Apple Silicon. It works offline, and your audio never leaves the computer.
- **Optional AI polish.** One call to Gemini, OpenAI, Anthropic, Groq, OpenRouter, a custom endpoint, or a local Ollama model. Only the transcript text is sent, never audio. If the call fails or times out, Hams pastes the locally cleaned text.
- **Command mode.** Select text, hold the command shortcut, and say "make this more formal" or "translate to English".
- **App-aware styles.** Writing is casual in chat apps, formal in email, and raw in code editors and terminals. You can edit each style's instructions and pin any app to a style.
- **Live preview (optional).** Words appear in the overlay while you speak.
- **Voice notes.** Record a lecture, meeting or walk-and-talk for as long as you like. Hams transcribes it in chunks while you talk, then saves the transcript with an AI title and key points.
- **Snippets, a personal dictionary that learns from your edits** (in History, or opt-in in the app you pasted into)**, voice commands** ("new line", "scratch that"), **Esc to cancel, and double-tap for hands-free.**
- **Rewrite from History.** Turn an old dictation into an email, a chat message or document prose with one click.
- **Keep the mic ready (optional).** Recording starts instantly and includes the moment before you pressed the key.
- **Automatic updates.** Signed updates from GitHub Releases, installed only when you click Install.
- **Searchable history and local stats.** Everything is stored in one SQLite file. API keys live in the OS keychain.

## Getting Hams (for everyone)

On any Mac with macOS 12 or later, Apple Silicon or Intel, paste this into Terminal:

```bash
curl -fsSL https://hams.wusla.co/install | bash
```

It downloads the newest DMG, copies Hams into Applications and opens it ([the script](scripts/install.sh)). Hams isn't registered with Apple, so if you download the DMG in a browser instead, macOS refuses to open it the first time: go to System Settings → Privacy & Security, scroll down and click **Open Anyway**. Files fetched with curl aren't flagged that way, and neither are updates installed from inside Hams.

## Installing from source on your own Mac

```bash
scripts/create-signing-identity.sh   # once: a local signing certificate (asks for your password)
scripts/install-macos.sh             # build a release Hams.app and install it to /Applications
```

The install script quits the running Hams, moves the old `/Applications/Hams.app` to the Trash, copies the new build in and opens it. Signing every build with the same local certificate makes macOS remember the Accessibility and Microphone permissions, so you don't have to grant them again after each update. Without the certificate the app is ad-hoc signed and still works, but you may have to re-enable Hams under Privacy & Security → Accessibility after reinstalling.

Turn on **Settings → General → Open Hams at login** to start Hams quietly in the menu bar. Closing the window keeps Hams running in the menu bar, and opening it again just brings the window forward.

## Publishing an update

Bump the version in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and `package.json`, commit, then run `scripts/release-macos.sh`. The branch has to be pushed first, because the release tag is made from that commit. It builds one universal app (Apple Silicon and Intel) and creates a draft GitHub Release with the update archive, `latest.json` (both architectures), the DMG, a fixed-name `Hams-macOS.dmg` for the website and `install.sh`. Publish the draft and installed copies offer the update within a few hours (or right away from Settings → Updates → Check now). Updates are signed with `~/.tauri/hams.key`; keep a backup of it, because installs can't be updated without it.

## Website

`site/` is the landing page, built with Svelte and Vite: `npm run site:dev` to work on it, `npm run site:build` for a static, prerendered copy in `site/dist`.

It's served at **https://hams.wusla.co** by Cloudflare Pages, connected to this repository:

- Build command `npm ci && npm run site:build`, output directory `site/dist`, environment variable `NODE_VERSION=22`.
- `site/public/_redirects` gives the site short links to the newest release: `/download` (the DMG) and `/install` (the install script), so the page and the Terminal line never change between versions.
- `/privacy/` is the Hams for Android privacy policy, kept at its old address for the Play Store listing. `/privacy/mac/` covers the Mac app.

## App icon

`src-tauri/icons/Hams.icon` is an Icon Composer document (the vector glyph plus a gradient), so macOS renders the light, dark, tinted and clear variants itself. `scripts/build-icon.sh` compiles it with `actool` into `icons/macos/Assets.car` and regenerates the `.icns`, PNG, ICO and menu-bar template icons from the SVGs in `icons/source/`. It needs Xcode.

## Development

Requirements: Node 20+, Rust (stable), CMake and Xcode Command Line Tools (macOS). On Windows you need Visual Studio Build Tools instead.

```bash
npm ci
npm run tauri:dev     # run the app with hot reload (Vite on 127.0.0.1:1430 + Rust)
npm run check         # svelte-check + TypeScript
npm test              # frontend unit tests (vitest)
npm run build         # production frontend build
npm run tauri:build   # installer / .app + .dmg
(cd src-tauri && cargo test)   # Rust unit tests
```

The first launch walks you through permissions and the model download. Models are stored in `~/Library/Application Support/Hams/models` (macOS) or `%APPDATA%\Hams\models` (Windows).

macOS needs **Accessibility** access for the shortcut, pasting and reading selected text, and **Microphone** access, which macOS asks for the first time you dictate.

Hams v1 data (`…/wisper/`) is imported once on first launch. The old files are read and never modified.
