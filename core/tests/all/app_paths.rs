//! Ported from `tests/Sobreiro.Tests/AppPathsTests.cs`, after the three tests of `AppPaths` in
//! `UsageFileTests.cs`, which came first because the bridge needs them.
//!
//! The C# tests set `SOBREIRO_DATA_DIR` for the whole test run and ask Windows for the user's
//! folder. Here both are handed in, so no test changes what another one reads.
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use chrono::DateTime;
use sobreiro_core::app_paths::AppPaths;
use sobreiro_core::diagnostics::diagnostics_log::DiagnosticsLog;
use sobreiro_core::diagnostics::log_entry::format;

use crate::TestFolder;

/// Stands in for the user's local application data folder.
const LOCAL_APP_DATA: &str = r"C:\Users\me\AppData\Local";

/// The paths with `SOBREIRO_DATA_DIR` set to `dir`.
fn with_override(dir: &str) -> AppPaths {
    AppPaths::new(Some(OsStr::new(dir)), Path::new(LOCAL_APP_DATA))
}

/// The paths with `SOBREIRO_DATA_DIR` not set.
fn without_override() -> AppPaths {
    AppPaths::new(None, Path::new(LOCAL_APP_DATA))
}

/// The instance's name with `SOBREIRO_DATA_DIR` set to `dir`.
fn id(dir: &str) -> String {
    with_override(dir).instance_id()
}

// From `UsageFileTests.cs`.

#[cfg(windows)]
#[test]
fn app_paths_uses_sobreiro_data_dir_when_set() {
    let dir = r"D:\scratch\sobreiro-tests";

    let paths = with_override(dir);

    assert_eq!(paths.data_directory(), Path::new(dir));
    assert_eq!(paths.usage_file(), Path::new(dir).join("usage.json"));
    assert_eq!(paths.config_file(), Path::new(dir).join("config.json"));
}

#[test]
fn app_paths_ignores_relative_sobreiro_data_dir() {
    let paths = with_override(r"relative\dir");

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

// From `AppPathsTests.cs`.

#[test]
fn instance_id_without_override_is_plain() {
    assert_eq!(without_override().instance_id(), "Sobreiro");
}

#[cfg(windows)]
#[test]
fn instance_id_with_override_is_a_hash_that_ignores_case() {
    let a = id(r"C:\Temp\Scratch");

    // The C# test holds the name against the pattern `^Sobreiro\.[0-9a-f]{8}$`.
    let digits = a.strip_prefix("Sobreiro.").unwrap();
    assert_eq!(digits.len(), 8, "{a}");
    assert!(
        digits
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "{a}"
    );
    assert_eq!(a, id(r"c:\temp\SCRATCH"));
    assert_ne!(a, id(r"C:\Temp\Other"));
}

#[cfg(windows)]
#[test]
fn instance_id_ignores_a_trailing_separator() {
    assert_eq!(id(r"C:\Temp\Scratch"), id(r"C:\Temp\Scratch\"));
}

#[cfg(windows)]
#[test]
fn instance_id_with_an_override_naming_the_default_folder_is_plain() {
    assert_eq!(id(&format!(r"{LOCAL_APP_DATA}\Sobreiro")), "Sobreiro");
    assert_eq!(id(&format!(r"{LOCAL_APP_DATA}\SOBREIRO\")), "Sobreiro");
    assert_eq!(
        id(&format!(r"{}\sobreiro", LOCAL_APP_DATA.to_lowercase())),
        "Sobreiro"
    );
}

#[test]
fn alerts_file_is_alerts_json() {
    assert!(without_override().alerts_file().ends_with("alerts.json"));
}

#[cfg(windows)]
#[test]
fn log_file_lives_in_the_data_directory() {
    let dir = r"D:\scratch\sobreiro-tests-log";

    assert_eq!(
        with_override(dir).log_file(),
        Path::new(dir).join("log.txt")
    );
}

#[test]
fn log_written_with_the_override_lands_in_the_scratch_folder() {
    let folder = TestFolder::new("app_paths_log_written_with_the_override");
    let paths = AppPaths::new(Some(folder.path().as_os_str()), Path::new(LOCAL_APP_DATA));
    let entry = format(
        DateTime::parse_from_rfc3339("2026-11-09T14:03:12.345+01:00").unwrap(),
        1,
        "start version=test instance=x",
        None,
    );

    assert!(DiagnosticsLog::new(&paths.log_file()).append(&entry));

    assert!(
        fs::read_to_string(folder.file("log.txt"))
            .unwrap()
            .contains("start version=test instance=x")
    );
}

#[test]
fn bridge_file_is_in_the_data_folder() {
    let paths = without_override();

    assert_eq!(
        paths.bridge_file(),
        paths.data_directory().join("sobreiro-bridge.exe")
    );
}

// Not in the C# tests.

/// A second copy must get the name 1.x gives the same folder, or one of each could run on one
/// folder at the same time. These are the names the C# of 1.1.0 gave on 2026-11-14, however the
/// folder is spelled.
#[cfg(windows)]
#[test]
fn instance_id_is_the_one_the_csharp_gives() {
    let rows = [
        (r"C:\Temp\Scratch", "7b69f39a"),
        (r"C:\Temp\Other", "4e7cdab8"),
        // The same folder, spelled in other ways.
        ("C:/Temp/Scratch", "7b69f39a"),
        ("C:/Temp/Scratch/", "7b69f39a"),
        (r"C:\Temp\.\Scratch\..\Scratch", "7b69f39a"),
        (r"C:\Temp\\Scratch", "7b69f39a"),
        (r"C:\Temp\Scratch\\", "7b69f39a"),
        (r"C:\Temp\Scratch.", "7b69f39a"),
        (r"C:\Temp\Scratch ", "7b69f39a"),
        // A drive's own folder keeps its separator.
        (r"C:\", "28f709d7"),
        (r"D:\", "25176a62"),
        // Folders on another computer.
        (r"\\server\share", "0f2d7f64"),
        (r"\\server\share\", "0f2d7f64"),
        (r"\\server\share\data\", "abacea68"),
        ("//server/share/data", "abacea68"),
        // The long spellings Windows also takes.
        (r"\\?\C:\Temp\Scratch", "1bce0a47"),
        (r"\\?\C:\Temp\Scratch\", "1bce0a47"),
        (r"\\?\C:\", "299f5a42"),
        (r"\\.\C:\Temp\Scratch", "886700f0"),
        (r"\\?\UNC\server\share", "fa9e31fe"),
        (r"\\?\UNC\server\share\", "fa9e31fe"),
        // Letters outside English: capitals and small letters are one folder.
        (r"C:\Temp\ação", "e3cce2f5"),
        (r"C:\Temp\AÇÃO", "e3cce2f5"),
        (r"C:\Temp\ÿ", "3149051d"),
        (r"C:\Temp\µ", "91955c29"),
        (r"C:\Temp\ſ", "c59f483e"),
        (r"C:\Temp\σς", "9e9ff250"),
        (r"C:\Temp\ǆ", "ce7a6012"),
        (r"C:\Temp\ǅ", "ce7a6012"),
        (r"C:\Temp\𐐨", "f4fc03f3"),
        (r"C:\Temp\😀", "645c74cf"),
        // A letter whose capital is written with two letters stays as it is.
        (r"C:\Temp\Straße", "340b14fa"),
        (r"C:\Temp\straße", "340b14fa"),
        (r"C:\Temp\STRASSE", "180a8635"),
        (r"C:\Temp\ŉ", "25cd6c82"),
        (r"C:\Temp\ﬁ", "6a9a55b6"),
        // The Turkish letters: an i without its dot is not an I.
        (r"C:\Temp\ı", "b8c4af2e"),
        (r"C:\Temp\I", "7d661b7d"),
        (r"C:\Temp\i", "7d661b7d"),
        (r"C:\Temp\İ", "2a4e49fe"),
    ];
    for (dir, digits) in rows {
        assert_eq!(id(dir), format!("Sobreiro.{digits}"), "{dir}");
    }
}

/// What counts as a full path is what the C# counts as one, and the folder comes back as it
/// was written.
#[cfg(windows)]
#[test]
fn an_override_is_taken_when_it_names_drive_and_folder() {
    let taken = [
        "C:/Temp/Scratch",
        r"C:\",
        r"\\server\share",
        "//server/share/data",
        r"\\?\C:\Temp\Scratch",
        r"\\.\C:\Temp\Scratch",
        // Taken too, though no folder is named: better to fail there than to fall back on
        // the user's own data.
        r"\\server",
        r"\\",
        "//",
        r"\??\C:\x",
    ];
    for dir in taken {
        assert_eq!(with_override(dir).data_directory(), Path::new(dir));
    }

    let ignored = [
        "C:",
        "C:Temp",
        r"\Temp",
        r"\.\x",
        "1:\\x",
        r"relative\dir",
        "",
    ];
    for dir in ignored {
        let paths = with_override(dir);
        assert_eq!(
            paths.data_directory(),
            Path::new(LOCAL_APP_DATA).join("Sobreiro"),
            "{dir}"
        );
        assert_eq!(paths.instance_id(), "Sobreiro", "{dir}");
    }
}

#[test]
fn the_default_folder_does_not_follow_the_override() {
    for paths in [without_override(), with_override(r"D:\scratch\elsewhere")] {
        assert_eq!(
            paths.default_directory(),
            Path::new(LOCAL_APP_DATA).join("Sobreiro")
        );
    }
}

#[cfg(windows)]
#[test]
fn the_data_folder_is_the_default_one_without_an_override_or_with_one_naming_it() {
    assert!(without_override().is_default_data_directory());
    assert!(with_override(r"relative\dir").is_default_data_directory());
    assert!(with_override(&format!(r"{LOCAL_APP_DATA}\Sobreiro")).is_default_data_directory());
    assert!(with_override(&format!(r"{LOCAL_APP_DATA}\SOBREIRO\")).is_default_data_directory());
    assert!(
        with_override(&format!(r"{LOCAL_APP_DATA}\Other\..\sobreiro")).is_default_data_directory()
    );

    assert!(!with_override(r"D:\scratch\elsewhere").is_default_data_directory());
    assert!(
        !with_override(&format!(r"{LOCAL_APP_DATA}\Sobreiro\inner")).is_default_data_directory()
    );
}

// Not in the C# tests.

#[test]
fn new_full_has_no_paths_without_a_full_path_to_keep_them_in() {
    for local in [
        None,
        Some(Path::new("")),
        Some(Path::new(r"relative\Local")),
    ] {
        for overridden in [
            None,
            Some(OsStr::new("")),
            Some(OsStr::new(r"relative\data")),
        ] {
            assert_eq!(
                AppPaths::new_full(overridden, local),
                None,
                "{local:?} {overridden:?}"
            );
        }
    }
}

#[test]
fn new_full_takes_the_local_application_data_folder() {
    let paths = AppPaths::new_full(None, Some(Path::new(LOCAL_APP_DATA))).unwrap();

    assert_eq!(paths, without_override());
}

#[test]
fn new_full_takes_a_full_path_override_with_no_local_folder() {
    let paths = AppPaths::new_full(Some(OsStr::new(r"D:\data")), None).unwrap();

    assert_eq!(paths.usage_file(), Path::new(r"D:\data\usage.json"));
    // A relative local folder does not matter either, when the override is a full path.
    let paths = AppPaths::new_full(Some(OsStr::new(r"D:\data")), Some(Path::new("rel"))).unwrap();
    assert_eq!(paths.config_file(), Path::new(r"D:\data\config.json"));
}
