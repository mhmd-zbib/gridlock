pub mod ai;
pub mod entity;
pub mod game;
pub mod spawn;
pub mod systems;
pub mod world;

use std::path::PathBuf;

/// Locate `assets/configs/<subdir>` using a priority chain that works for
/// both `cargo run` (dev) and a distributed release binary:
///
/// 1. Next to the running executable — the release distribution layout where
///    CI copies `assets/` alongside the binary before packaging.
/// 2. Relative to the current working directory — dev flow when running from
///    the workspace root.
/// 3. (debug builds only) `CARGO_MANIFEST_DIR` and its parent — resolves to
///    the source tree when running with `cargo run` inside the workspace.
///    Excluded from release builds because the path is the CI machine path,
///    which does not exist on user machines.
pub(crate) fn resolve_config_dir(subdir: &str) -> Option<PathBuf> {
    let build = |root: PathBuf| root.join("assets").join("configs").join(subdir);

    if let Some(path) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| build(d.to_path_buf())))
        .filter(|p| p.is_dir())
    {
        return Some(path);
    }

    if let Some(path) = std::env::current_dir().ok().map(build).filter(|p| p.is_dir()) {
        return Some(path);
    }

    #[cfg(debug_assertions)]
    {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let candidate = build(manifest.clone());
        if candidate.is_dir() {
            return Some(candidate);
        }
        if let Some(path) = manifest
            .parent()
            .map(|root| build(root.to_path_buf()))
            .filter(|p| p.is_dir())
        {
            return Some(path);
        }
    }

    None
}
