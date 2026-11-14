//! The three tests of `AppPaths` in `tests/Sobreiro.Tests/UsageFileTests.cs`, which the bridge
//! needs. `AppPathsTests.cs` itself comes with the area "Paths and config".
//!
//! The C# tests set `SOBREIRO_DATA_DIR` for the whole test run and ask Windows for the user's
//! folder. Here both are handed in, so no test changes what another one reads.
use std::ffi::OsStr;
use std::path::Path;

use sobreiro_core::app_paths::AppPaths;

/// Stands in for the user's local application data folder.
const LOCAL_APP_DATA: &str = r"C:\Users\me\AppData\Local";

#[cfg(windows)]
#[test]
fn app_paths_uses_sobreiro_data_dir_when_set() {
    let dir = r"D:\scratch\sobreiro-tests";

    let paths = AppPaths::new(Some(OsStr::new(dir)), Path::new(LOCAL_APP_DATA));

    assert_eq!(paths.data_directory(), Path::new(dir));
    assert_eq!(paths.usage_file(), Path::new(dir).join("usage.json"));
    assert_eq!(paths.config_file(), Path::new(dir).join("config.json"));
}

#[test]
fn app_paths_ignores_relative_sobreiro_data_dir() {
    let paths = AppPaths::new(Some(OsStr::new(r"relative\dir")), Path::new(LOCAL_APP_DATA));

    assert_eq!(
        paths.data_directory(),
        Path::new(LOCAL_APP_DATA).join("Sobreiro")
    );
}

#[test]
fn app_paths_defaults_to_local_application_data_when_unset() {
    // The C# test sets the variable to nothing, which Windows takes as removing it.
    for unset in [Some(OsStr::new("")), None] {
        let paths = AppPaths::new(unset, Path::new(LOCAL_APP_DATA));

        assert_eq!(
            paths.data_directory(),
            Path::new(LOCAL_APP_DATA).join("Sobreiro"),
            "{unset:?}"
        );
    }
}
