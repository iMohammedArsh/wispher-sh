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

version=$(node -p 'require("./src-tauri/tauri.conf.json").version')
tag="v$version"
case "$(uname -m)" in
  arm64) target="darwin-aarch64" ;;
  x86_64) target="darwin-x86_64" ;;
  *) echo "Unsupported architecture $(uname -m)" >&2; exit 1 ;;
esac
if gh release view "$tag" --repo "$REPO" >/dev/null 2>&1; then
  echo "Release $tag already exists. Bump the version first." >&2
  exit 1
fi

echo "==> Building Hams $version for $target (signed with: ${APPLE_SIGNING_IDENTITY:-ad-hoc})"
npm ci --silent
npm run tauri build -- --bundles app,dmg

bundle="src-tauri/target/release/bundle"
archive="$bundle/macos/Hams.app.tar.gz"
[[ -f "$archive" && -f "$archive.sig" ]] || { echo "Update archive or signature missing in $bundle/macos" >&2; exit 1; }
dmg=$(ls "$bundle"/dmg/*.dmg | head -1)

out="src-tauri/target/release/update"
rm -rf "$out" && mkdir -p "$out"
asset="Hams_${version}_${target}.app.tar.gz"
cp "$archive" "$out/$asset"
notes=$(git log -1 --pretty=%s)
node -e '
  const [version, target, asset, sigPath, repo, tag, notes, outPath] = process.argv.slice(1);
  const fs = require("fs");
  const manifest = {
    version,
    notes,
    pub_date: new Date().toISOString(),
    platforms: {
      [target]: {
        signature: fs.readFileSync(sigPath, "utf8").trim(),
        url: `https://github.com/${repo}/releases/download/${tag}/${asset}`,
      },
    },
  };
  fs.writeFileSync(outPath, JSON.stringify(manifest, null, 2) + "\n");
' "$version" "$target" "$asset" "$archive.sig" "$REPO" "$tag" "$notes" "$out/latest.json"

draft=(--draft)
[[ "${1:-}" == "--publish" ]] && draft=()
echo "==> Creating GitHub release $tag ${draft[*]:-(published)}"
gh release create "$tag" --repo "$REPO" "${draft[@]}" --title "Hams $version" --notes "$notes" \
  "$out/$asset" "$out/latest.json" "$dmg"
echo "==> Done. Installed copies check $tag's latest.json once it's published (not while it's a draft)."
