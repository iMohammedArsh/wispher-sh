#!/usr/bin/env bash
# Build a release Hams.app and install it into /Applications.
#
#   scripts/install-macos.sh
#
# Signing: if you ran scripts/create-signing-identity.sh, the app is signed with
# that stable identity, so macOS keeps the Accessibility / Microphone permission
# across rebuilds. Otherwise it is ad-hoc signed and you may need to re-grant
# Accessibility after each install.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH"

IDENTITY_NAME="Hams Local Signing"
if [[ -z "${APPLE_SIGNING_IDENTITY:-}" ]] && security find-identity -v -p codesigning | grep -q "$IDENTITY_NAME"; then
  export APPLE_SIGNING_IDENTITY="$IDENTITY_NAME"
fi
echo "==> Signing with: ${APPLE_SIGNING_IDENTITY:-ad-hoc}"

# Update signing: builds signed with ~/.tauri/hams.key can be offered as updates.
# Without the key, build without the update archive instead of failing.
UPDATE_KEY="${HAMS_UPDATE_KEY:-$HOME/.tauri/hams.key}"
build_args=(--bundles app)
if [[ -f "$UPDATE_KEY" ]]; then
  export TAURI_SIGNING_PRIVATE_KEY="$(cat "$UPDATE_KEY")"
  export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}"
else
  build_args+=(--config '{"bundle":{"createUpdaterArtifacts":false}}')
fi

echo "==> Building release app (first build takes a few minutes)…"
npm ci --silent
npm run tauri build -- "${build_args[@]}"

app="src-tauri/target/release/bundle/macos/Hams.app"
[[ -d "$app" ]] || { echo "Build output not found at $app" >&2; exit 1; }
if [[ -z "${APPLE_SIGNING_IDENTITY:-}" ]]; then
  codesign --force --deep --sign - "$app"
fi

echo "==> Quitting the running Hams…"
osascript -e 'tell application id "com.wusla.hams" to quit' >/dev/null 2>&1 || true
pkill -x hams >/dev/null 2>&1 || true
sleep 1

if [[ -d /Applications/Hams.app ]]; then
  echo "==> Moving the previous /Applications/Hams.app to the Trash"
  osascript -e 'tell application "Finder" to delete POSIX file "/Applications/Hams.app"' >/dev/null
fi

ditto "$app" /Applications/Hams.app
echo "==> Installed /Applications/Hams.app"
open /Applications/Hams.app
