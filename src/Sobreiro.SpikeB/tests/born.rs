//! The pill's window is owned and left inactive from its birth, by Windows' own means. What that leans on in
//! the toolkit is not in its interface: that a window shown is the next top-level window made on the UI
//! thread, and that the toolkit rewrites a window's styles. This test runs the exe's test window for three
//! seconds, owned by a stand-in window so it needs no taskbar, and fails when an upgrade of the toolkit
//! changes either. It needs a desktop session. `cargo test --release --test born`
use std::process::Command;

fn run(extra: &[(&str, &str)]) -> Vec<String> {
    let trace = std::env::temp_dir().join(format!("sobreiro-born-{}-{}.txt", std::process::id(), extra.len()));
    let _ = std::fs::remove_file(&trace);
    let mut command = Command::new(env!("CARGO_BIN_EXE_Sobreiro"));
    command
        .arg("--probe")
        .env("SOBREIRO_SPIKE_TRACE", &trace)
        .env("SOBREIRO_SPIKE_PROBE_OWNER", "own")
        .env("SOBREIRO_SPIKE_PROBE_SECONDS", "3")
        .env_remove("SOBREIRO_SPIKE_BORN")
        .env_remove("SOBREIRO_SPIKE_OURS");
    for (name, value) in extra {
        command.env(name, value);
    }
    assert!(command.status().expect("the exe starts").success(), "the test window left with an error");
    let text = std::fs::read_to_string(&trace).expect("the test window wrote its trace");
    let _ = std::fs::remove_file(&trace);
    text.lines().map(str::to_owned).collect()
}

fn line<'a>(lines: &'a [String], start: &str) -> &'a str {
    lines.iter().find(|l| l.starts_with(start)).unwrap_or_else(|| panic!("no line starting with {start:?} in {lines:#?}"))
}

fn assert_all(line: &str, facts: &[&str]) {
    for fact in facts {
        assert!(line.contains(fact), "expected {fact:?} in: {line}");
    }
}

#[test]
fn the_window_is_owned_and_inactive_from_its_birth() {
    let lines = run(&[]);
    // 0.3 s after it was shown, before anything else touched it.
    assert_all(
        line(&lines, "probe born=Ours"),
        &["birth=Some(", "owner-is-taskbar-after-show=true", "active-after-show=false", "foreground-unchanged-after-show=true"],
    );
    assert_all(
        line(&lines, "owner-is-taskbar="),
        &["owner-is-taskbar=true", "visible=true", "active=false", "foreground-unchanged=true", "toolwindow=true", "noactivate=true"],
    );
}

#[test]
fn the_style_bits_stay_when_the_toolkit_rewrites_the_style() {
    let lines = run(&[("SOBREIRO_SPIKE_PROBE_RESTYLE", "1")]);
    assert_all(line(&lines, "on-top=true asked"), &["birth=Some("]);
    assert_all(line(&lines, "owner-is-taskbar="), &["owner-is-taskbar=true", "active=false", "toolwindow=true", "noactivate=true"]);
}
