use std::path::Path;

type R = Result<(), Box<dyn std::error::Error>>;

const COMMITTED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/lib/bindings.ts");

/// Regenerate with `UPDATE_BINDINGS=1 cargo test -p taskapp-app --test bindings`.
#[test]
fn bindings_are_current() -> R {
    if std::env::var_os("UPDATE_BINDINGS").is_some() {
        taskapp_app::export_bindings(Path::new(COMMITTED))?;
    }
    let dir = tempfile::tempdir()?;
    let fresh = dir.path().join("bindings.ts");
    taskapp_app::export_bindings(&fresh)?;
    assert_eq!(
        std::fs::read_to_string(COMMITTED)?,
        std::fs::read_to_string(fresh)?,
        "ui/src/lib/bindings.ts is stale"
    );
    Ok(())
}
