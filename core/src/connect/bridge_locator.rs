//! Where this Sobreiro's bridge is.
use std::path::{Path, PathBuf};

const EXE_NAME: &str = "sobreiro-bridge.exe";

/// This Sobreiro's bridge: the installed copy when the exe carries one, else the one found
/// beside the app (see [`find`]); `None` when there is none.
///
/// `file_exists` says whether a file is there. The exe hands in the real question to the
/// disk, and a test hands in its own answer.
pub fn own(
    carried: bool,
    installed_path: &Path,
    app_directory: &Path,
    file_exists: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    if carried {
        return Some(installed_path.to_path_buf());
    }
    find(app_directory, file_exists)
}

/// The bridge beside the app, else in `..\bridge\`; `None` when neither exists. Full paths.
pub fn find(app_directory: &Path, file_exists: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    // `absolute` makes a path full the way Windows reads it, and works out the `..`.
    let beside = std::path::absolute(app_directory.join(EXE_NAME)).ok()?;
    if file_exists(&beside) {
        return Some(beside);
    }

    let sibling = app_directory.join("..").join("bridge").join(EXE_NAME);
    let sibling = std::path::absolute(sibling).ok()?;
    file_exists(&sibling).then_some(sibling)
}
