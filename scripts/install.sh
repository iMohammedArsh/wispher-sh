#!/usr/bin/env bash
# Install (or update) Hams from the latest GitHub release:
#
#   curl -fsSL https://hams.wusla.co/install | bash
#
# Files fetched with curl aren't marked as downloaded from the internet, so macOS
# doesn't stop the app on first launch the way it does for a DMG opened from a browser.
# The script only touches /Applications/Hams.app and a temporary folder.
set -euo pipefail

REPO="iMohammedArsh/wispher-sh"
DMG_URL="${HAMS_DMG_URL:-https://github.com/$REPO/releases/latest/download/Hams-macOS.dmg}"
APP="/Applications/Hams.app"

say() { printf '\033[1m==>\033[0m %s\n' "$*"; }
fail() { printf 'Hams install failed: %s\n' "$*" >&2; exit 1; }

[[ "$(uname -s)" == "Darwin" ]] || fail "Hams for this installer is macOS only."
major=$(sw_vers -productVersion | cut -d. -f1)
(( major >= 12 )) || fail "Hams needs macOS 12 Monterey or later (this Mac has $(sw_vers -productVersion))."

work=$(mktemp -d -t hams-install)
mount=""
cleanup() {
  [[ -n "$mount" ]] && hdiutil detach "$mount" -quiet -force >/dev/null 2>&1 || true
  rm -rf "$work"
}
trap cleanup EXIT

say "Downloading Hams"
curl -fL --progress-bar -o "$work/Hams.dmg" "$DMG_URL" || fail "couldn't download $DMG_URL"

say "Opening the disk image"
mount="$work/mnt"
mkdir -p "$mount"
hdiutil attach "$work/Hams.dmg" -nobrowse -readonly -noautoopen -mountpoint "$mount" -quiet \
  || fail "couldn't open the disk image"
[[ -d "$mount/Hams.app" ]] || fail "the disk image has no Hams.app"

if pgrep -xq Hams; then
  say "Quitting the running Hams"
  osascript -e 'tell application id "com.wusla.hams" to quit' >/dev/null 2>&1 || true
  for _ in {1..20}; do pgrep -xq Hams || break; sleep 0.25; done
  pkill -x Hams 2>/dev/null || true
fi

say "Copying Hams to Applications"
need_admin=false
if [[ ! -w /Applications ]] || { [[ -e "$APP" ]] && [[ ! -w "$APP" ]]; }; then
  echo "Your password is needed to write to /Applications."
  need_admin=true
fi
as_admin() { if $need_admin; then sudo "$@"; else "$@"; fi; }
as_admin rm -rf "$APP"
as_admin ditto "$mount/Hams.app" "$APP"
# Belt and braces: clear the flag in case something upstream set it.
as_admin xattr -dr com.apple.quarantine "$APP" 2>/dev/null || true

say "Starting Hams"
open "$APP"
say "Done. Hams is in Applications and in your menu bar."
