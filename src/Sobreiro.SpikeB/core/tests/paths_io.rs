// Ports of AtomicFileTests, AppPathsTests (the parts that do not need the log) and ClaudePathsTests,
// and the AppPaths cases of UsageFileTests. The environment is passed in, so no test touches the real one.
mod common;
use common::*;
use sobreiro_core::app_paths::AppPaths;
use sobreiro_core::connect::ClaudePaths;
use sobreiro_core::io::AtomicFile;
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;

fn tmp_files(dir: &str) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".tmp"))
        .collect()
}

#[test]
fn writes_the_text_without_a_bom_by_default() {
    let d = Scratch::new();
    let path = d.path("a.txt");
    AtomicFile::write_all_text(&path, "héllo", false).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(String::from_utf8(bytes.clone()).unwrap(), "héllo");
    assert_ne!(bytes[0], 0xEF);
}

#[test]
fn writes_a_bom_when_asked() {
    let d = Scratch::new();
    let path = d.path("a.txt");
    AtomicFile::write_all_text(&path, "x", true).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), vec![0xEF, 0xBB, 0xBF, b'x']);
}

#[test]
fn creates_a_missing_folder_and_replaces_an_existing_file() {
    let d = Scratch::new();
    let path = d.path("deep/er/a.txt");
    AtomicFile::write_all_text(&path, "one", false).unwrap();
    AtomicFile::write_all_text(&path, "two", false).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "two");
}

#[test]
fn a_failed_rename_leaves_no_temp_file_and_returns_the_error() {
    let d = Scratch::new();
    let path = d.path("a.txt");
    std::fs::write(&path, "old").unwrap();
    let _held = std::fs::OpenOptions::new().read(true).share_mode(0).open(&path).unwrap();
    assert!(AtomicFile::write_all_text(&path, "new", false).is_err());
    assert!(tmp_files(&d.dir()).is_empty());
}

#[test]
fn leaves_no_temp_file_behind() {
    let d = Scratch::new();
    AtomicFile::write_all_text(&d.path("a.txt"), "x", false).unwrap();
    assert!(tmp_files(&d.dir()).is_empty());
}

#[test]
fn write_from_a_stream_replaces_the_file() {
    let d = Scratch::new();
    let path = d.path("a.bin");
    std::fs::write(&path, "old").unwrap();
    AtomicFile::write(&path, &mut &b"new"[..]).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    assert!(tmp_files(&d.dir()).is_empty());
}

// ---- AppPaths

const SCRATCH: &str = r"C:\Temp\Scratch";

fn env_with(dir: Option<&'static str>) -> impl Fn(&str) -> Option<String> {
    move |name| match name {
        "SOBREIRO_DATA_DIR" => dir.map(String::from),
        "LOCALAPPDATA" => Some(r"C:\Users\ana\AppData\Local".to_string()),
        _ => None,
    }
}

fn id(dir: Option<&'static str>) -> String {
    AppPaths::instance_id_with(&env_with(dir))
}

#[test]
fn instance_id_without_override_is_plain() {
    assert_eq!(id(None), "Sobreiro");
}

#[test]
fn instance_id_with_override_is_a_hash_that_ignores_case() {
    let a = id(Some(SCRATCH));
    let hex = a.strip_prefix("Sobreiro.").expect("prefix");
    assert_eq!(hex.len(), 8);
    assert!(hex.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));
    assert_eq!(a, id(Some(r"c:\temp\SCRATCH")));
    assert_ne!(a, id(Some(r"C:\Temp\Other")));
}

#[test]
fn instance_id_ignores_a_trailing_separator() {
    assert_eq!(id(Some(SCRATCH)), id(Some(r"C:\Temp\Scratch\")));
}

#[test]
fn instance_id_with_an_override_naming_the_default_folder_is_plain() {
    assert_eq!(id(Some(r"C:\Users\ana\AppData\Local\Sobreiro")), "Sobreiro");
    assert_eq!(id(Some(r"C:\Users\ana\AppData\Local\SOBREIRO\")), "Sobreiro");
    assert_eq!(id(Some(r"c:\users\ana\appdata\local\sobreiro")), "Sobreiro");
}

#[test]
fn alerts_file_is_alerts_json() {
    assert!(AppPaths::alerts_file().ends_with("alerts.json"));
}

#[test]
fn log_file_lives_in_the_data_directory() {
    assert!(AppPaths::log_file().ends_with("log.txt"));
    let env = env_with(Some(SCRATCH));
    let dir = AppPaths::data_directory_with(&env);
    assert_eq!(dir, SCRATCH);
}

#[test]
fn bridge_file_is_in_the_data_folder() {
    assert_eq!(
        AppPaths::bridge_file(),
        Path::new(&AppPaths::data_directory()).join("sobreiro-bridge.exe").to_string_lossy()
    );
}

#[test]
fn app_paths_uses_data_dir_variable_when_set() {
    let env = env_with(Some(SCRATCH));
    assert_eq!(AppPaths::data_directory_with(&env), SCRATCH);
    assert_eq!(AppPaths::usage_file_with(&env), r"C:\Temp\Scratch\usage.json");
    assert_eq!(AppPaths::config_file_with(&env), r"C:\Temp\Scratch\config.json");
}

#[test]
fn app_paths_ignores_relative_and_empty_data_dir_variable() {
    let local = r"C:\Users\ana\AppData\Local\Sobreiro";
    assert_eq!(AppPaths::data_directory_with(&env_with(Some(r"relative\dir"))), local);
    assert_eq!(AppPaths::data_directory_with(&env_with(Some(""))), local);
    assert_eq!(AppPaths::data_directory_with(&env_with(None)), local);
}

// ---- ClaudePaths

fn cenv(config: Option<&'static str>) -> impl Fn(&str) -> Option<String> {
    move |name| match name {
        "CLAUDE_CONFIG_DIR" => config.map(String::from),
        "USERPROFILE" => Some(r"C:\Users\ana".to_string()),
        _ => None,
    }
}

#[test]
fn credentials_file_sits_in_the_config_dir() {
    assert_eq!(ClaudePaths::credentials_file(&cenv(Some(r"D:\cfg\claude"))), r"D:\cfg\claude\.credentials.json");
    assert_eq!(ClaudePaths::credentials_file(&cenv(None)), r"C:\Users\ana\.claude\.credentials.json");
}

#[test]
fn config_dir_variable_is_used_when_fully_qualified() {
    assert_eq!(ClaudePaths::config_directory(&cenv(Some(r"D:\cfg\claude"))), r"D:\cfg\claude");
    assert_eq!(ClaudePaths::settings_file(&cenv(Some(r"D:\cfg\claude"))), r"D:\cfg\claude\settings.json");
}

#[test]
fn relative_or_empty_config_dir_is_ignored() {
    for v in [None, Some(""), Some(r"relative\dir")] {
        assert_eq!(ClaudePaths::config_directory(&cenv(v)), r"C:\Users\ana\.claude");
        assert_eq!(ClaudePaths::settings_file(&cenv(v)), r"C:\Users\ana\.claude\settings.json");
    }
}
