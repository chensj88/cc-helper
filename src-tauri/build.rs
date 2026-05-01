fn main() {
    // Copy cc-helper-hook binary to src-tauri/binaries/ with target triple suffix
    let target = std::env::var("TARGET").unwrap_or_default();
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let ext = if cfg!(target_os = "windows") { ".exe" } else { "" };

    let src = format!("../target/{profile}/cc-helper-hook{ext}");
    let dst_dir = "binaries";
    let dst = format!("{dst_dir}/cc-helper-hook-{target}{ext}");

    let _ = std::fs::create_dir_all(dst_dir);
    if std::path::Path::new(&src).exists() {
        std::fs::copy(&src, &dst).expect("Failed to copy cc-helper-hook to binaries/");
    }

    tauri_build::build()
}
