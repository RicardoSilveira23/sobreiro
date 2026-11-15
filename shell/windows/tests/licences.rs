//! Every library in the build is under a licence that fits GPL-3.0-or-later. A library whose
//! licence is not on the list below fails this test until someone has read that licence and
//! added it. These are readings, not legal advice.
use std::collections::HashSet;
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

/// Asks cargo for every package the lock file gives a Windows build, the ones only tests and
/// build scripts use among them, and checks the licence each one states.
#[test]
fn every_library_in_the_build_fits() {
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
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let ids = |value: &serde_json::Value| value.as_str().unwrap().to_string();
    let ours: HashSet<String> = metadata["workspace_members"]
        .as_array()
        .unwrap()
        .iter()
        .map(ids)
        .collect();
    let built: HashSet<String> = metadata["resolve"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| ids(&node["id"]))
        .collect();

    let mut checked = 0;
    let mut wrong = Vec::new();
    for package in metadata["packages"].as_array().unwrap() {
        let id = ids(&package["id"]);
        if ours.contains(&id) || !built.contains(&id) {
            continue;
        }
        checked += 1;
        let licence = package["license"].as_str().unwrap_or("");
        if !fits(licence) {
            wrong.push(format!(
                "{} {}: {licence:?}",
                ids(&package["name"]),
                ids(&package["version"])
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
