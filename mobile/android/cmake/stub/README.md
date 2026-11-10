`libggml-blas.a` is an empty static library. whisper-rs-sys 0.14 decides whether
to link `ggml-blas` with `cfg!(target_os = "macos")` in its build script, which
checks the *build machine*, so cross-compiling for Android from a Mac asks for a
library that whisper.cpp never builds there. An empty archive satisfies the
linker; nothing in it is used. `scripts/build-mobile-core.sh` adds this folder to
the link path for Android targets only.
