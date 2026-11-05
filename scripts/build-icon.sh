#!/usr/bin/env bash
# Rebuild every app icon from the vector sources in src-tauri/icons/source.
#
#   Hams.icon  (Icon Composer format) → macos/Assets.car: the adaptive macOS 26+ icon
#                                        (light, dark, clear and tinted variants)
#                                     → icon.icns: the same artwork pre-rendered for macOS 12–15
#   source/icon.svg                   → PNG and ICO sizes for Windows and the tray
#
# Needs Xcode 26 or later (actool with .icon support) and Node.
set -euo pipefail
cd "$(dirname "$0")/../src-tauri"

cp icons/source/glyph.svg icons/Hams.icon/Assets/glyph.svg

npx --yes tauri icon icons/source/icon.svg -o icons >/dev/null 2>&1
rm -rf icons/android icons/ios

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
xcrun actool "$PWD/icons/Hams.icon" --compile "$work" --platform macosx \
  --minimum-deployment-target 12.0 --app-icon Hams \
  --output-partial-info-plist "$work/partial.plist" >/dev/null
mkdir -p icons/macos
cp "$work/Assets.car" icons/macos/Assets.car
cp "$work/Hams.icns" icons/icon.icns
echo "Icons rebuilt."

# Monochrome menu-bar "template" icon (macOS tints it for light/dark menu bars).
tray="$(mktemp -d)"
npx --yes tauri icon icons/source/tray.svg -o "$tray" >/dev/null 2>&1
mkdir -p icons/tray
cp "$tray/32x32.png" icons/tray/tray-template.png
cp "$tray/64x64.png" icons/tray/tray-template@2x.png
rm -rf "$tray"
