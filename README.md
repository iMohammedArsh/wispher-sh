# Hams

Hams is a local-first voice-writing app for macOS and Windows. Hold a global shortcut, speak naturally, and Hams transcribes locally before pasting the result back into the app you were using.

## macOS development

Requirements: macOS 12 or later, Apple Silicon or Intel, Xcode Command Line Tools, Homebrew, and Node.js 20+.

```bash
npm ci
./scripts/setup-macos.sh
npm run tauri:dev
```

The setup script installs Rust and CMake when necessary, builds a Metal-enabled static `whisper.cpp` CLI/server for this Mac, and downloads the multilingual small model. Generated binaries and model files stay untracked.

On first launch, allow Hams under **System Settings → Privacy & Security**:

- **Microphone** for recording
- **Input Monitoring** for the global shortcut
- **Accessibility** for pasting with `⌘V`

Shortcuts on macOS:

- Hold `⌘ + Shift + Space` for push-to-talk
- Press `⌘ + Shift + Option` to start or stop hands-free recording

## Privacy

Speech recognition, history, vocabulary, and correction rules remain on the device. If the user adds a Gemini API key, Hams sends transcript text and custom vocabulary—not recorded audio—to Gemini for optional formatting.

## Useful commands

```bash
npm run dev          # Browser UI preview with native calls gracefully unavailable
npm run build        # Type-check and build the frontend
npm run tauri:dev    # Run the native desktop app
npm run tauri:build  # Produce a platform installer/bundle
```

Windows-specific hooks and paste delivery remain in the same Rust codebase behind target-specific compilation flags.
