#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WHISPER_ROOT="$PROJECT_ROOT/tools/whisper.cpp"
MAC_BIN="$WHISPER_ROOT/macos"
MODEL_DIR="$WHISPER_ROOT/models"
MODEL_PATH="$MODEL_DIR/ggml-small.bin"

if ! command -v brew >/dev/null 2>&1; then
  echo "Homebrew is required: https://brew.sh"
  exit 1
fi

brew list rustup >/dev/null 2>&1 || brew install rustup
brew list cmake >/dev/null 2>&1 || brew install cmake
export PATH="$(brew --prefix rustup)/bin:$HOME/.cargo/bin:$PATH"
rustup default stable

BUILD_ROOT="$(mktemp -d)"
trap 'rm -rf "$BUILD_ROOT"' EXIT

git clone --depth 1 --branch v1.9.4 https://github.com/ggerganov/whisper.cpp.git "$BUILD_ROOT/whisper.cpp"
cmake -S "$BUILD_ROOT/whisper.cpp" -B "$BUILD_ROOT/build" \
  -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_SHARED_LIBS=OFF \
  -DWHISPER_BUILD_TESTS=OFF \
  -DWHISPER_BUILD_EXAMPLES=ON \
  -DGGML_METAL=ON
cmake --build "$BUILD_ROOT/build" --config Release --target whisper-cli whisper-server -j "$(sysctl -n hw.logicalcpu)"

mkdir -p "$MAC_BIN" "$MODEL_DIR"
cp "$BUILD_ROOT/build/bin/whisper-cli" "$MAC_BIN/whisper-cli"
cp "$BUILD_ROOT/build/bin/whisper-server" "$MAC_BIN/whisper-server"
chmod +x "$MAC_BIN/whisper-cli" "$MAC_BIN/whisper-server"

if [ ! -f "$MODEL_PATH" ]; then
  curl --fail --location --progress-bar \
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin" \
    --output "$MODEL_PATH.incomplete"
  mv "$MODEL_PATH.incomplete" "$MODEL_PATH"
fi

echo "Hams macOS tooling is ready. Run: npm run tauri:dev"
