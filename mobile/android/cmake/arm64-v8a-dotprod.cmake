# Used by scripts/build-mobile-core.sh: whisper.cpp (built by whisper-rs-sys via
# the cmake crate) needs the NDK toolchain with the ABI and API level set.
#
# The fast arm64 build: ARMv8.2 dot-product and fp16 instructions make Whisper's
# quantized matrix maths several times faster. Every phone CPU since about 2018
# (Cortex-A55/A75 and later) has them; older ones load the -compat build instead
# (chosen at runtime in Hams.kt).
set(ANDROID_ABI arm64-v8a)
set(ANDROID_PLATFORM android-26)
set(ANDROID_STL c++_shared)
set(GGML_CPU_ARM_ARCH "armv8.2-a+dotprod+fp16" CACHE STRING "" FORCE)
include($ENV{ANDROID_NDK_HOME}/build/cmake/android.toolchain.cmake)
