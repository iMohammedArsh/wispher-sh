fn main() {
    link_clang_runtime();
    tauri_build::build()
}

/// whisper.cpp's Metal backend uses `@available(...)`. When the deployment
/// target is older than the SDK, clang lowers that to `__isPlatformVersionAtLeast`,
/// which lives in clang's compiler runtime. rustc links with `-nodefaultlibs`,
/// so pull that runtime in explicitly.
fn link_clang_runtime() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.contains("apple-darwin") || std::env::var_os("CARGO_FEATURE_LOCAL_STT").is_none() {
        return;
    }
    let Ok(output) = std::process::Command::new("clang").arg("--print-runtime-dir").output() else { return };
    let dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !dir.is_empty() && std::path::Path::new(&dir).join("libclang_rt.osx.a").exists() {
        println!("cargo:rustc-link-search=native={dir}");
        println!("cargo:rustc-link-lib=static=clang_rt.osx");
    }
}
