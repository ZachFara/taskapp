fn main() {
    // `generate_context!` embeds the frontend build and fails when the directory is missing, so a
    // fresh checkout (or CI) that has not built the UI yet still compiles.
    std::fs::create_dir_all("../../ui/dist").expect("create ui/dist");
    // tauri-build requires the sidecar file for the target to exist. Debug builds and CI get an empty
    // placeholder; a release build must come after scripts/build-helper.sh, so it fails instead.
    let triple = std::env::var("TARGET").expect("TARGET");
    let helper = std::path::PathBuf::from(format!("binaries/reminders-helper-{triple}"));
    if std::env::var("PROFILE").is_ok_and(|p| p != "release") && !helper.exists() {
        std::fs::create_dir_all("binaries").expect("create binaries");
        std::fs::write(&helper, []).expect("write helper placeholder");
    }
    tauri_build::build();
}
