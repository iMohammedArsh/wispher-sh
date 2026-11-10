# Baseline ARMv8.0 build of whisper.cpp for phones without dot-product
# instructions (Cortex-A53-only CPUs). Slower, but runs everywhere.
set(ANDROID_ABI arm64-v8a)
set(ANDROID_PLATFORM android-26)
set(ANDROID_STL c++_shared)
include($ENV{ANDROID_NDK_HOME}/build/cmake/android.toolchain.cmake)
