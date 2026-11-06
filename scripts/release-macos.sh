#!/usr/bin/env bash
# Build a signed release and publish it as a GitHub Release that installed
# copies of Hams pick up as an update.
#
#   scripts/release-macos.sh            # build + create a *draft* release
#   scripts/release-macos.sh --publish  # build + publish right away
#
# Before running: bump "version" in src-tauri/tauri.conf.json, src-tauri/Cargo.toml
# and package.json, and commit. Needs the update key at ~/.tauri/hams.key
# (back it up: without it, existing installs can never be updated again) and
# the GitHub CLI logged in (`gh auth login`).
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH"

REPO="iMohammedArsh/wispher-sh"
UPDATE_KEY="${HAMS_UPDATE_KEY:-$HOME/.tauri/hams.key}"
[[ -f "$UPDATE_KEY" ]] || { echo "Update key not found at $UPDATE_KEY" >&2; exit 1; }
export TAURI_SIGNING_PRIVATE_KEY="$(cat "$UPDATE_KEY")"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}"

IDENTITY_NAME="Hams Local Signing"
if [[ -z "${APPLE_SIGNING_IDENTITY:-}" ]] && security find-identity -v -p codesigning | grep -q "$IDENTITY_NAME"; then
  export APPLE_SIGNING_IDENTITY="$IDENTITY_NAME"
fi
# Otherwise sign ad-hoc ("-"). Tauri then signs with the hardened runtime and
# src-tauri/Entitlements.plist, which the microphone needs, before it makes the
# DMG and the update archive.
export APPLE_SIGNING_IDENTITY="${APPLE_SIGNING_IDENTITY:--}"

version=$(node -p 'require("./src-tauri/tauri.conf.json").version')
tag="v$version"
# One universal build (Apple Silicon + Intel) serves both updater platforms.
rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
commit=$(git rev-parse HEAD)
# The tag is made on GitHub from this commit, so it has to be there already.
if ! git branch -r --contains "$commit" 2>/dev/null | grep -q .; then
  echo "Commit $commit isn't pushed. Push the branch first (git push -u origin $(git branch --show-current))." >&2
  exit 1
fi
if gh release view "$tag" --repo "$REPO" >/dev/null 2>&1; then
  echo "Release $tag already exists. Bump the version first." >&2
  exit 1
fi

echo "==> Building Hams $version, universal (signed with: $APPLE_SIGNING_IDENTITY)"
npm ci --silent
npm run tauri build -- --target universal-apple-darwin --bundles app,dmg

bundle="src-tauri/target/universal-apple-darwin/release/bundle"
archive="$bundle/macos/Hams.app.tar.gz"
[[ -f "$archive" && -f "$archive.sig" ]] || { echo "Update archive or signature missing in $bundle/macos" >&2; exit 1; }
dmg=$(ls "$bundle"/dmg/*.dmg | head -1)
codesign -d --entitlements - "$bundle/macos/Hams.app" 2>&1 | grep -q com.apple.security.device.audio-input \
  || { echo "$bundle/macos/Hams.app was signed without the microphone entitlement (src-tauri/Entitlements.plist)" >&2; exit 1; }

out="src-tauri/target/universal-apple-darwin/release/update"
rm -rf "$out" && mkdir -p "$out"
asset="Hams_${version}_universal.app.tar.gz"
cp "$archive" "$out/$asset"
# A copy under a fixed name, so releases/latest/download/Hams-macOS.dmg (the website's
# download button) always points at the newest DMG.
cp "$dmg" "$out/Hams-macOS.dmg"
# The website's one-line install: curl …/releases/latest/download/install.sh | bash
cp scripts/install.sh "$out/install.sh"
notes=$(git log -1 --pretty=%s)
node -e '
  const [version, asset, sigPath, repo, tag, notes, outPath] = process.argv.slice(1);
  const fs = require("fs");
  const entry = {
    signature: fs.readFileSync(sigPath, "utf8").trim(),
    url: `https://github.com/${repo}/releases/download/${tag}/${asset}`,
  };
  const manifest = {
    version,
    notes,
    pub_date: new Date().toISOString(),
    platforms: { "darwin-aarch64": entry, "darwin-x86_64": entry },
  };
  fs.writeFileSync(outPath, JSON.stringify(manifest, null, 2) + "\n");
' "$version" "$asset" "$archive.sig" "$REPO" "$tag" "$notes" "$out/latest.json"

draft="--draft"
[[ "${1:-}" == "--publish" ]] && draft="--draft=false"
echo "==> Creating GitHub release $tag ($draft)"
gh release create "$tag" --repo "$REPO" "$draft" --target "$commit" --title "Hams $version" --notes "$notes" \
  "$out/$asset" "$out/latest.json" "$dmg" "$out/Hams-macOS.dmg" "$out/install.sh"
echo "==> Done. Installed copies check $tag's latest.json once it's published (not while it's a draft)."
