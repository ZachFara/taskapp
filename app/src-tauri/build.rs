fn main() {
    // `generate_context!` embeds the frontend build and fails when the directory is missing, so a
    // fresh checkout (or CI) that has not built the UI yet still compiles.
    std::fs::create_dir_all("../../ui/dist").expect("create ui/dist");
    tauri_build::build();
}
