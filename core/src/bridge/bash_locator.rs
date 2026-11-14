//! Finds Git Bash the way Claude Code does on Windows, so the user's previous status line
//! command runs in the same shell it ran in before.

/// The path of `bash.exe`, or `None` when there is no Git Bash.
///
/// `get_env` gives an environment variable's value and `file_exists` says whether a file is
/// there. Both are handed in so the search can be tried against a made-up PC.
pub fn find(
    get_env: impl Fn(&str) -> Option<String>,
    file_exists: impl Fn(&str) -> bool,
) -> Option<String> {
    // 1. Where the user told Claude Code to look.
    if let Some(configured) = get_env("CLAUDE_CODE_GIT_BASH_PATH")
        && !configured.is_empty()
        && file_exists(&configured)
    {
        return Some(configured);
    }

    // Windows' own folders hold a `bash.exe` that starts the Linux subsystem, not Git Bash.
    let mut skipped = Vec::new();
    if let Some(system_root) = get_env("SystemRoot")
        && !system_root.is_empty()
    {
        skipped.push(normalise(&join(&system_root, "System32")));
        skipped.push(normalise(&join(&system_root, "SysWOW64")));
    }

    let path = get_env("PATH").unwrap_or_default();
    let directories: Vec<String> = path
        .split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(normalise)
        .collect();

    // 2. A `bash.exe` in a folder on PATH.
    for directory in &directories {
        if skipped.iter().any(|skip| same_name(skip, directory)) {
            continue;
        }
        // The store's stand-in for an app that is not installed.
        if same_name(file_name(directory), "WindowsApps") {
            continue;
        }
        let bash = join(directory, "bash.exe");
        if file_exists(&bash) {
            return Some(bash);
        }
    }

    // 3. Beside a `git.exe` on PATH: Git's `cmd` folder is on PATH, its `bin` folder is not.
    for directory in &directories {
        if !file_exists(&join(directory, "git.exe")) {
            continue;
        }
        let bash = full_path(&join(directory, r"..\bin\bash.exe"));
        if file_exists(&bash) {
            return Some(bash);
        }
    }

    // 4. Where Git installs itself.
    if let Some(program_files) = get_env("ProgramFiles")
        && !program_files.is_empty()
    {
        let bash = full_path(&join(&program_files, r"Git\bin\bash.exe"));
        if file_exists(&bash) {
            return Some(bash);
        }
    }
    None
}

/// `directory\name`.
fn join(directory: &str, name: &str) -> String {
    if directory.ends_with(['\\', '/']) {
        format!("{directory}{name}")
    } else {
        format!("{directory}\\{name}")
    }
}

/// A folder's full path without a slash at its end, so two spellings of one folder compare equal.
fn normalise(directory: &str) -> String {
    full_path(directory)
        .trim_end_matches(['\\', '/'])
        .to_string()
}

/// The path with `..` worked out and a drive in front. Nothing on disk is looked at. A path
/// Windows cannot make sense of stays as it is.
fn full_path(path: &str) -> String {
    match std::path::absolute(path) {
        Ok(full) => full.to_string_lossy().into_owned(),
        Err(_) => path.to_string(),
    }
}

/// The last part of a path: what follows its last slash.
fn file_name(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

/// Whether two names are the same to Windows, which does not tell capitals from small letters.
fn same_name(a: &str, b: &str) -> bool {
    a.to_uppercase() == b.to_uppercase()
}
