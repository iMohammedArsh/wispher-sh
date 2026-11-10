# Used by scripts/build-mobile-core.sh: whisper.cpp (built by whisper-rs-sys via
# the cmake crate) needs the NDK toolchain with the ABI and API level set.
set(ANDROID_ABI x86_64)
set(ANDROID_PLATFORM android-26)
set(ANDROID_STL c++_shared)
include($ENV{ANDROID_NDK_HOME}/build/cmake/android.toolchain.cmake)
