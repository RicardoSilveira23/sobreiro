//! Every library in the build is under a licence that fits GPL-3.0-or-later. A library whose
//! licence is not on the list below fails this test until someone has read that licence and
//! added it. These are readings, not legal advice.
//!
//! The other test makes `notices.txt` from the libraries' own licence files, and fails when the
//! committed file differs. `SOBREIRO_WRITE_NOTICES=1` writes it instead.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The licences read so far, by their SPDX names.
const FITS: &[&str] = &[
    // Sobreiro's own.
    "GPL-3.0-or-later",
    // The UI toolkit is offered under this one among others. It is why the exe as shipped is
    // under version 3 of the GPL, while Sobreiro's own files stay "or later".
    "GPL-3.0-only",
    // Fits version 3 of the GPL, not version 2.
    "Apache-2.0",
    // Short permissive licences: keep the notice, no further conditions.
    "MIT",
    "ISC",
    "Zlib",
    "BSL-1.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "Unicode-3.0",
];

/// Whether a licence expression fits: with `OR` one side is enough, with `AND` both are needed.
/// `/` is the old spelling of `OR`. A licence with an exception (`WITH`) is one name.
fn fits(expression: &str) -> bool {
    let spaced = expression
        .replace('(', " ( ")
        .replace(')', " ) ")
        .replace('/', " OR ");
    let tokens: Vec<&str> = spaced.split_whitespace().collect();
    let mut at = 0;
    let result = any_of(&tokens, &mut at);
    result.is_some_and(|fits| fits && at == tokens.len())
}

fn any_of(tokens: &[&str], at: &mut usize) -> Option<bool> {
    let mut result = all_of(tokens, at)?;
    while tokens.get(*at) == Some(&"OR") {
        *at += 1;
        result |= all_of(tokens, at)?;
    }
    Some(result)
}

fn all_of(tokens: &[&str], at: &mut usize) -> Option<bool> {
    let mut result = one(tokens, at)?;
    while tokens.get(*at) == Some(&"AND") {
        *at += 1;
        result &= one(tokens, at)?;
    }
    Some(result)
}

fn one(tokens: &[&str], at: &mut usize) -> Option<bool> {
    let token = *tokens.get(*at)?;
    *at += 1;
    match token {
        "(" => {
            let inner = any_of(tokens, at)?;
            (tokens.get(*at) == Some(&")")).then(|| *at += 1)?;
            Some(inner)
        }
        ")" | "AND" | "OR" | "WITH" => None,
        name if tokens.get(*at) == Some(&"WITH") => {
            let exception = tokens.get(*at + 1)?;
            *at += 2;
            Some(FITS.contains(&format!("{name} WITH {exception}").as_str()))
        }
        name => Some(FITS.contains(&name)),
    }
}

#[test]
fn one_side_of_or_is_enough() {
    assert!(fits("MIT"));
    assert!(fits("MIT OR No-Such-Licence"));
    assert!(fits("No-Such-Licence OR Apache-2.0"));
    assert!(fits("No-Such-Licence/MIT"));
    assert!(!fits("No-Such-Licence"));
    assert!(!fits("No-Such-Licence OR Another-Unknown"));
}

#[test]
fn both_sides_of_and_are_needed() {
    assert!(fits("MIT AND Apache-2.0"));
    assert!(!fits("MIT AND No-Such-Licence"));
    assert!(fits("(MIT OR No-Such-Licence) AND Apache-2.0"));
    assert!(!fits("(MIT OR Apache-2.0) AND No-Such-Licence"));
    assert!(fits("MIT AND No-Such-Licence OR Apache-2.0"));
}

#[test]
fn an_exception_makes_another_licence() {
    assert!(!fits("MIT WITH No-Such-Exception"));
    assert!(fits("MIT WITH No-Such-Exception OR Apache-2.0"));
}

#[test]
fn what_cannot_be_read_does_not_fit() {
    for expression in [
        "",
        "MIT OR",
        "(MIT",
        "MIT)",
        "MIT Apache-2.0",
        "MIT WITH",
        "AND MIT",
    ] {
        assert!(!fits(expression), "{expression:?} was taken to fit");
    }
}

/// What `cargo metadata` says of the libraries the shell is built for on Windows.
fn metadata() -> serde_json::Value {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let output = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--locked"])
        .args(["--filter-platform", "x86_64-pc-windows-msvc"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn text(value: &serde_json::Value) -> String {
    value.as_str().unwrap().to_string()
}

/// Asks cargo for every package the lock file gives a Windows build, the ones only tests and
/// build scripts use among them, and checks the licence each one states.
#[test]
fn every_library_in_the_build_fits() {
    let metadata = metadata();
    let ours: HashSet<String> = metadata["workspace_members"]
        .as_array()
        .unwrap()
        .iter()
        .map(text)
        .collect();
    let built: HashSet<String> = metadata["resolve"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| text(&node["id"]))
        .collect();

    let mut checked = 0;
    let mut wrong = Vec::new();
    for package in metadata["packages"].as_array().unwrap() {
        let id = text(&package["id"]);
        if ours.contains(&id) || !built.contains(&id) {
            continue;
        }
        checked += 1;
        let licence = package["license"].as_str().unwrap_or("");
        if !fits(licence) {
            wrong.push(format!(
                "{} {}: {licence:?}",
                text(&package["name"]),
                text(&package["version"])
            ));
        }
    }
    assert!(
        checked > 100,
        "only {checked} libraries were found: the question to cargo is wrong"
    );
    assert!(
        wrong.is_empty(),
        "{} of {checked} libraries have a licence not read yet:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// One library of the exe, with the licence and notice files in its folder.
struct Library {
    name: String,
    version: String,
    licence: String,
    folder: PathBuf,
    /// File name and normalised text, by file name.
    files: Vec<(String, String)>,
}

/// The start of the name of a file that holds a licence or a notice, in lower case.
const FILE_PREFIXES: [&str; 5] = ["licen", "copying", "notice", "copyright", "unlicense"];

/// CRLF and a lone CR become LF, trailing whitespace goes off every line, and leading and
/// trailing blank lines go off the text.
fn normalise(text: &str) -> String {
    let unified = text.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = unified.lines().map(str::trim_end).collect();
    let first = lines.iter().position(|line| !line.is_empty());
    let last = lines.iter().rposition(|line| !line.is_empty());
    match (first, last) {
        (Some(first), Some(last)) => lines[first..=last].join("\n"),
        _ => String::new(),
    }
}

fn read_file(path: &Path) -> String {
    let bytes = std::fs::read(path)
        .expect("a licence file of a library cannot be read; are its sources downloaded?");
    normalise(&String::from_utf8_lossy(&bytes))
}

/// The libraries the shell and the bridge it carries are built from, by name and then version.
fn libraries(metadata: &serde_json::Value) -> Vec<Library> {
    let packages: HashMap<String, &serde_json::Value> = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|package| (text(&package["id"]), package))
        .collect();
    let nodes: HashMap<String, &serde_json::Value> = metadata["resolve"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| (text(&node["id"]), node))
        .collect();
    let ours: HashSet<String> = metadata["workspace_members"]
        .as_array()
        .unwrap()
        .iter()
        .map(text)
        .collect();

    let mut pending: Vec<String> = ours
        .iter()
        .filter(|id| {
            let name = text(&packages[*id]["name"]);
            name == "sobreiro" || name == "sobreiro-bridge"
        })
        .cloned()
        .collect();
    assert_eq!(pending.len(), 2, "the shell or the bridge was not found");
    let mut seen: HashSet<String> = pending.iter().cloned().collect();
    while let Some(id) = pending.pop() {
        for dep in nodes[&id]["deps"].as_array().unwrap() {
            let built = dep["dep_kinds"]
                .as_array()
                .unwrap()
                .iter()
                .any(|kind| kind["kind"].is_null() || kind["kind"].as_str() == Some("build"));
            let dep_id = text(&dep["pkg"]);
            if built && seen.insert(dep_id.clone()) {
                pending.push(dep_id);
            }
        }
    }

    let mut found: Vec<Library> = seen
        .iter()
        .filter(|id| !ours.contains(*id))
        .map(|id| {
            let package = packages[id];
            let manifest = PathBuf::from(text(&package["manifest_path"]));
            let folder = manifest.parent().unwrap().to_path_buf();
            let name = text(&package["name"]);
            let version = text(&package["version"]);
            assert!(
                folder.is_dir(),
                "the folder of {name} {version} is missing; its sources are not downloaded"
            );
            let mut names: Vec<String> = std::fs::read_dir(&folder)
                .unwrap()
                .map(|entry| entry.unwrap())
                .filter(|entry| entry.path().is_file())
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .filter(|file| {
                    let lower = file.to_lowercase();
                    FILE_PREFIXES.iter().any(|prefix| lower.starts_with(prefix))
                })
                .collect();
            let mut files: Vec<(String, String)> = Vec::new();
            for file in names.drain(..) {
                let content = read_file(&folder.join(&file));
                files.push((file, content));
            }
            if let Some(named) = package["license_file"].as_str() {
                let path = folder.join(named);
                let file = path.file_name().unwrap().to_string_lossy().into_owned();
                if !files.iter().any(|(have, _)| *have == file) {
                    files.push((file, read_file(&path)));
                }
            }
            files.sort_by(|a, b| a.0.cmp(&b.0));
            Library {
                licence: package["license"]
                    .as_str()
                    .unwrap_or("see its licence file")
                    .to_string(),
                name,
                version,
                folder,
                files,
            }
        })
        .collect();
    found.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    found
}

/// The text of `notices.txt`.
fn notices(libraries: &[Library]) -> String {
    let mut out = String::from(
        "These are the libraries Sobreiro.exe is built from, with the bridge it carries. Each is \
         listed with its version and its licence. After the list come the licence and notice \
         texts the libraries ship; a text shipped word for word by several libraries is given \
         once, under the names of all of them.\n\n",
    );
    for library in libraries {
        out.push_str(&format!(
            "{} {}: {}",
            library.name, library.version, library.licence
        ));
        if library.files.iter().all(|(_, content)| content.is_empty()) {
            out.push_str("; no licence file, the licence's standard text applies");
        }
        out.push('\n');
    }

    // The texts in the order of the first library that ships each, by their words.
    let mut texts: Vec<(String, &str, Vec<String>)> = Vec::new();
    for library in libraries {
        for (_, content) in library
            .files
            .iter()
            .filter(|(_, content)| !content.is_empty())
        {
            let words = content.split_whitespace().collect::<Vec<_>>().join(" ");
            let label = format!("{} {}", library.name, library.version);
            match texts.iter_mut().find(|(key, _, _)| *key == words) {
                Some((_, _, names)) => {
                    if !names.contains(&label) {
                        names.push(label);
                    }
                }
                None => texts.push((words, content, vec![label])),
            }
        }
    }
    for (_, content, names) in &texts {
        out.push_str(&format!("\n{}\n", "=".repeat(80)));
        for name in names {
            out.push_str(name);
            out.push('\n');
        }
        out.push_str(&format!("{}\n{content}\n", "-".repeat(80)));
    }
    out
}

/// Whether a text holds a drive path: a letter, a colon and a backslash.
fn has_drive_path(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    chars
        .windows(3)
        .any(|w| w[0].is_ascii_alphabetic() && w[1] == ':' && w[2] == '\\')
}

/// `notices.txt` is what the libraries' own files say, and the committed file is that. Run with
/// `SOBREIRO_WRITE_NOTICES=1` to write it.
#[test]
fn the_notices_are_what_the_libraries_ship() {
    let metadata = metadata();
    let libraries = libraries(&metadata);
    assert!(
        libraries.len() > 100,
        "only {} libraries were found: the question to cargo is wrong",
        libraries.len()
    );
    let content = notices(&libraries);

    for library in &libraries {
        let folder = library.folder.to_string_lossy().to_string();
        for spelling in [folder.replace('\\', "/"), folder] {
            assert!(
                !content.contains(&spelling),
                "the notices hold the folder of {}",
                library.name
            );
        }
    }
    for variable in ["CARGO_HOME", "USERPROFILE", "HOME"] {
        if let Ok(value) = std::env::var(variable) {
            let value = value.trim_end_matches(['\\', '/']).to_string();
            for spelling in [value.replace('\\', "/"), value] {
                assert!(
                    spelling.len() < 4 || !content.contains(&spelling),
                    "the notices hold the folder in {variable}"
                );
            }
        }
    }
    assert!(!has_drive_path(&content), "the notices hold a drive path");

    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("notices.txt");
    if std::env::var("SOBREIRO_WRITE_NOTICES").as_deref() == Ok("1") {
        std::fs::write(&file, &content).unwrap();
        return;
    }
    let committed = std::fs::read_to_string(&file)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert!(
        committed == content,
        "shell/windows/notices.txt is not what the libraries ship; run this test with \
         SOBREIRO_WRITE_NOTICES=1 to write it"
    );
}
