#!/usr/bin/env bash
# Build the Hams engine (crates/hams-mobile) for the phone apps.
#
#   scripts/build-mobile-core.sh            # Android: arm64-v8a + x86_64 (emulator), release
#   scripts/build-mobile-core.sh --debug    # faster build, larger and slower .so
#   ABIS="arm64-v8a" scripts/build-mobile-core.sh   # one ABI only
#
# Writes mobile/android/app/src/main/jniLibs/<abi>/{libhams_mobile.so,libc++_shared.so}
# (plus arm64-v8a/libhams_mobile_compat.so for older CPUs)
# and the generated Kotlin bindings to mobile/android/app/src/main/java/com/wusla/hams/core/.
# Gradle runs this automatically when the .so files are missing (see app/build.gradle.kts).
#
# Needs: rustup targets aarch64-linux-android x86_64-linux-android, cargo-ndk,
# cmake, and an Android NDK (ANDROID_NDK_HOME, or the newest 27.x in the SDK).
set -euo pipefail
cd "$(dirname "$0")/.."
root="$PWD"

export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH"
sdk="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
  ANDROID_NDK_HOME="$(ls -d "$sdk"/ndk/27.* 2>/dev/null | sort -V | tail -1 || true)"
  [[ -n "$ANDROID_NDK_HOME" ]] || ANDROID_NDK_HOME="$(ls -d "$sdk"/ndk/* | sort -V | tail -1)"
fi
export ANDROID_NDK_HOME ANDROID_HOME="$sdk"
echo "NDK: $ANDROID_NDK_HOME"

profile=release
cargo_profile=(--release)
if [[ "${1:-}" == "--debug" ]]; then
  profile=debug
  cargo_profile=()
fi

abis=(${ABIS:-arm64-v8a x86_64})
jni="$root/mobile/android/app/src/main/jniLibs"
stub="$root/mobile/android/cmake/stub"

# whisper.cpp is built by CMake inside whisper-rs-sys; point it at the NDK
# toolchain per ABI (cargo doesn't notice edits to these .cmake files: after
# changing one, run `cargo clean -p whisper-rs-sys --target <triple> --release`),
# and give the linker the empty ggml-blas stub (see stub/README.md).
export CMAKE_TOOLCHAIN_FILE_aarch64_linux_android="$root/mobile/android/cmake/arm64-v8a-dotprod.cmake"
export CMAKE_TOOLCHAIN_FILE_x86_64_linux_android="$root/mobile/android/cmake/x86_64.cmake"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS="-L native=$stub"
export CARGO_TARGET_X86_64_LINUX_ANDROID_RUSTFLAGS="-L native=$stub"

targets=()
for abi in "${abis[@]}"; do targets+=(-t "$abi"); done
cargo ndk "${targets[@]}" -P 26 --link-libcxx-shared -o "$jni" build -p hams-mobile "${cargo_profile[@]}"

# arm64 also gets a baseline build for CPUs without dot-product instructions
# (see mobile/android/cmake/arm64-v8a-compat.cmake). Its own target dir keeps
# both whisper.cpp builds cached.
if [[ " ${abis[*]} " == *" arm64-v8a "* ]]; then
  CMAKE_TOOLCHAIN_FILE_aarch64_linux_android="$root/mobile/android/cmake/arm64-v8a-compat.cmake" \
  CARGO_TARGET_DIR="$root/src-tauri/target/android-compat" \
    cargo ndk -t arm64-v8a -P 26 build -p hams-mobile "${cargo_profile[@]}"
  cp "$root/src-tauri/target/android-compat/aarch64-linux-android/$profile/libhams_mobile.so" \
    "$jni/arm64-v8a/libhams_mobile_compat.so"
fi

# Kotlin bindings come from the host build: the stripped release .so has no
# UniFFI metadata left to read, and the API is the same on every target.
cargo build -p hams-mobile --no-default-features
host_lib="$root/src-tauri/target/debug/libhams_mobile.dylib"
[[ -f "$host_lib" ]] || host_lib="$root/src-tauri/target/debug/libhams_mobile.so"
out="$root/mobile/android/app/src/main/java"
cargo run -q -p hams-mobile --no-default-features --features bindgen --bin uniffi-bindgen -- \
  generate --library "$host_lib" --language kotlin --out-dir "$out" \
  --no-format

echo
echo "Built ($profile):"
ls -la "$jni"/*/
echo "Kotlin bindings: $out/com/wusla/hams/core/"
