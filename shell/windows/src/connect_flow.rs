//! Connect to Claude Code, Reconnect and Disconnect: plan, ask, apply, say so. The port of 1.1's
//! flow. It runs on a thread of its own, so the pill's event loop goes on while a box is open,
//! and one flow is open at a time. Every failure before the files are written ends in an error
//! box and leaves them as they were.
use crate::carried_bridge;
use crate::clock;
use crate::providers::Locations;
use crate::win::{local_zone, message_box};
use chrono::NaiveDateTime;
use sobreiro_core::config::AppConfig;
use sobreiro_core::connect::bridge_install;
use sobreiro_core::connect::connect_message::{self, RECONNECT_TITLE};
use sobreiro_core::connect::connection::{self, ConnectError, ConnectionState};
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

const NO_BRIDGE: &str = "Can't find sobreiro-bridge.exe next to Sobreiro.exe or in ..\\bridge.";
const NO_FOLDERS: &str = "Windows did not name the folders Sobreiro needs, so nothing was changed.";

/// The files a flow reads and writes.
struct Files {
    settings: PathBuf,
    config: PathBuf,
    /// The bridge Sobreiro owns; None when there is none.
    bridge: Option<PathBuf>,
}

/// The opener of the carried bridge, from its first byte.
type Opener<'a> = &'a dyn Fn() -> io::Result<&'static [u8]>;

/// The boxes a flow shows.
trait Boxes {
    /// A Yes/No question; true on Yes.
    fn ask(&self, title: &str, text: &str) -> bool;
    fn tell(&self, title: &str, text: &str);
    fn error(&self, text: &str);
}

/// Windows' own boxes.
struct Native;

impl Boxes for Native {
    fn ask(&self, title: &str, text: &str) -> bool {
        message_box::ask(title, text)
    }
    fn tell(&self, title: &str, text: &str) {
        message_box::tell(title, text);
    }
    fn error(&self, text: &str) {
        message_box::error(text);
    }
}

/// Lets one flow in at a time. Taking it gives a guard that lets go on drop.
struct Gate(AtomicBool);

struct Guard<'a>(&'a Gate);

impl Gate {
    const fn new() -> Gate {
        Gate(AtomicBool::new(false))
    }

    fn take(&self) -> Option<Guard<'_>> {
        self.0
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Guard(self))
    }
}

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.0.0.store(false, Ordering::Release);
    }
}

static OPEN: Gate = Gate::new();

/// Where the status line stands now, for the menu. No folders, or a `settings.json` that cannot
/// be read, count as not connected.
pub fn current_state() -> ConnectionState {
    match Locations::find() {
        Some(places) => state_of(
            &places.claude.settings_file(),
            carried_bridge::own(&places.app).as_deref(),
        ),
        None => ConnectionState::NotConnected,
    }
}

fn state_of(settings: &Path, own_bridge: Option<&Path>) -> ConnectionState {
    match connection::read_settings(settings) {
        Ok(snapshot) => connection::state(&snapshot.text, own_bridge),
        Err(_) => ConnectionState::NotConnected,
    }
}

/// Runs one flow on a thread of its own, then calls `then` on that thread, once the next flow may
/// start; a caller that needs the UI's thread hands over to it from there. None, and nothing done,
/// when a flow is already open or the thread cannot start.
pub fn start(connect: bool, then: impl FnOnce() + Send + 'static) -> Option<JoinHandle<()>> {
    let guard = OPEN.take()?;
    std::thread::Builder::new()
        .name("connect-flow".into())
        .spawn(move || {
            run_on_this_thread(connect);
            drop(guard);
            then();
        })
        .ok()
}

/// Hidden: one flow, waited for. Success when it ran, failure when it could not start.
pub fn run_now(connect: bool) -> ExitCode {
    match start(connect, || {}).map(JoinHandle::join) {
        Some(Ok(())) => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}

fn run_on_this_thread(connect: bool) {
    let Some(places) = Locations::find() else {
        Native.error(NO_FOLDERS);
        return;
    };
    let files = Files {
        settings: places.claude.settings_file(),
        config: places.app.config_file(),
        bridge: if connect {
            carried_bridge::own(&places.app)
        } else {
            None
        },
    };
    let now = clock::now();
    // Offset 0 when Windows will not say, as the log does.
    let local_now = match local_zone::offset_at(now) {
        Some(offset) => now.with_timezone(&offset).naive_local(),
        None => now.naive_utc(),
    };
    let opener: Opener<'_> = &carried_bridge::open;
    let carried = carried_bridge::carried().then_some(opener);
    run(connect, &files, carried, &Native, local_now);
}

/// What a flow that got a Yes did.
struct Done {
    backup: Option<PathBuf>,
    reconnects: bool,
}

/// One flow, in 1.1's order. A No writes nothing; an error ends in the error box.
fn run(
    connect: bool,
    files: &Files,
    carried: Option<Opener<'_>>,
    boxes: &dyn Boxes,
    local_now: NaiveDateTime,
) {
    if connect && files.bridge.is_none() {
        boxes.error(NO_BRIDGE);
        return;
    }
    match attempt(connect, files, carried, boxes, local_now) {
        Ok(Some(done)) => {
            let title = title(connect, done.reconnects);
            let text = connect_message::done(connect, done.backup.as_deref(), done.reconnects);
            boxes.tell(title, &text);
        }
        Ok(None) => {}
        Err(error) => boxes.error(&error.to_string()),
    }
}

/// Plans, asks and, on Yes, writes. None on No.
fn attempt(
    connect: bool,
    files: &Files,
    carried: Option<Opener<'_>>,
    boxes: &dyn Boxes,
    local_now: NaiveDateTime,
) -> Result<Option<Done>, ConnectError> {
    let settings = connection::read_settings(&files.settings)?;
    let config = AppConfig::load(&files.config)?;
    let bridge = files.bridge.as_deref().filter(|_| connect);
    let plan = match bridge {
        Some(bridge) => connection::plan_connect(&settings, &config, bridge)?,
        None => connection::plan_disconnect(&settings, &config)?,
    };
    let title = title(connect, plan.reconnects);
    if !boxes.ask(title, &connect_message::confirm(&plan)) {
        return Ok(None);
    }
    if let (Some(bridge), Some(opener)) = (bridge, carried) {
        bridge_install::ensure(opener, bridge)?;
    }
    let backup = connection::apply(&plan, &files.config, local_now)?;
    Ok(Some(Done {
        backup,
        reconnects: plan.reconnects,
    }))
}

fn title(connect: bool, reconnects: bool) -> &'static str {
    if reconnects {
        RECONNECT_TITLE
    } else {
        connect_message::title(connect)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use sobreiro_core::connect::bridge_command::command_for;
    use std::cell::RefCell;
    use std::fs;

    const OLD: &str = "{\n  \"theme\": \"dark\",\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"echo old\"\n  }\n}\n";
    const BRIDGE_BYTES: &[u8] = b"MZ-test-bridge";

    /// Boxes that answer, and record each call as (kind, title, text).
    struct Fake {
        answer: bool,
        calls: RefCell<Vec<(&'static str, String, String)>>,
    }

    impl Fake {
        fn new(answer: bool) -> Fake {
            Fake {
                answer,
                calls: RefCell::new(Vec::new()),
            }
        }
        fn calls(&self) -> Vec<(&'static str, String, String)> {
            self.calls.borrow().clone()
        }
    }

    impl Boxes for Fake {
        fn ask(&self, title: &str, text: &str) -> bool {
            self.calls
                .borrow_mut()
                .push(("ask", title.into(), text.into()));
            self.answer
        }
        fn tell(&self, title: &str, text: &str) {
            self.calls
                .borrow_mut()
                .push(("tell", title.into(), text.into()));
        }
        fn error(&self, text: &str) {
            self.calls
                .borrow_mut()
                .push(("error", String::new(), text.into()));
        }
    }

    /// A folder of the test's own, removed when it ends.
    struct Sandbox {
        root: PathBuf,
    }

    impl Sandbox {
        fn new(name: &str) -> Sandbox {
            let root = std::env::temp_dir()
                .join(format!("sobreiro-connect-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(root.join("claude")).unwrap();
            fs::create_dir_all(root.join("data")).unwrap();
            Sandbox { root }
        }
        fn files(&self, with_bridge: bool) -> Files {
            Files {
                settings: self.root.join("claude").join("settings.json"),
                config: self.root.join("data").join("config.json"),
                bridge: with_bridge.then(|| self.root.join("data").join("sobreiro-bridge.exe")),
            }
        }
        fn backups(&self) -> Vec<PathBuf> {
            fs::read_dir(self.root.join("claude"))
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| path.file_name().unwrap() != "settings.json")
                .collect()
        }
    }

    impl Drop for Sandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn opener() -> io::Result<&'static [u8]> {
        Ok(BRIDGE_BYTES)
    }

    fn when(minute: u32) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, 11, 16)
            .unwrap()
            .and_hms_opt(12, minute, 0)
            .unwrap()
    }

    fn run_with(connect: bool, files: &Files, boxes: &Fake, minute: u32) {
        let open: Opener<'_> = &opener;
        run(connect, files, Some(open), boxes, when(minute));
    }

    fn text(path: &Path) -> String {
        fs::read_to_string(path).unwrap()
    }

    #[test]
    fn connect_points_the_status_line_at_the_bridge_and_backs_up() {
        let sandbox = Sandbox::new("connect");
        let files = sandbox.files(true);
        fs::write(&files.settings, OLD).unwrap();
        let boxes = Fake::new(true);
        run_with(true, &files, &boxes, 1);

        let bridge = files.bridge.clone().unwrap();
        let settings: serde_json::Value = serde_json::from_str(&text(&files.settings)).unwrap();
        assert_eq!(
            settings["statusLine"]["command"].as_str(),
            Some(command_for(&bridge).as_str())
        );
        assert_eq!(settings["theme"], "dark");
        assert_eq!(fs::read(&bridge).unwrap(), BRIDGE_BYTES);
        assert!(files.config.is_file());
        let backups = sandbox.backups();
        assert_eq!(backups.len(), 1);
        assert_eq!(text(&backups[0]), OLD);

        let calls = boxes.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0, "ask");
        assert_eq!(calls[0].1, "Connect to Claude Code");
        assert_eq!(calls[1].0, "tell");
        assert_eq!(calls[1].1, "Connect to Claude Code");
        assert!(calls[1].2.contains(&backups[0].display().to_string()));
    }

    #[test]
    fn disconnect_puts_the_old_status_line_back_exactly() {
        let sandbox = Sandbox::new("disconnect");
        let files = sandbox.files(true);
        fs::write(&files.settings, OLD).unwrap();
        run_with(true, &files, &Fake::new(true), 1);
        let boxes = Fake::new(true);
        run_with(false, &files, &boxes, 2);

        assert_eq!(text(&files.settings), OLD);
        let calls = boxes.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].1, "Disconnect from Claude Code");
        assert_eq!(calls[1].1, "Disconnect from Claude Code");
        assert!(calls[1].2.starts_with("Disconnected."));
    }

    #[test]
    fn a_bridge_elsewhere_gives_the_reconnect_title() {
        let sandbox = Sandbox::new("reconnect");
        let files = sandbox.files(true);
        let elsewhere = sandbox.root.join("other").join("sobreiro-bridge.exe");
        let settings = format!(
            "{{\"statusLine\":{{\"type\":\"command\",\"command\":{}}}}}",
            serde_like(&command_for(&elsewhere))
        );
        fs::write(&files.settings, &settings).unwrap();
        let boxes = Fake::new(true);
        run_with(true, &files, &boxes, 1);

        let calls = boxes.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].1, "Reconnect to Claude Code");
        assert_eq!(calls[1].1, "Reconnect to Claude Code");
        assert_eq!(calls[0].0, "ask");
        assert_eq!(calls[1].0, "tell");
    }

    /// A JSON string literal for `value`.
    fn serde_like(value: &str) -> String {
        serde_json::to_string(value).unwrap()
    }

    /// The bytes of the two files a refusal must leave alone.
    fn snapshot(files: &Files) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
        (fs::read(&files.settings).ok(), fs::read(&files.config).ok())
    }

    fn refused(sandbox: &Sandbox, files: &Files, connect: bool, expected: String) {
        let before = snapshot(files);
        let boxes = Fake::new(true);
        run_with(connect, files, &boxes, 1);
        assert_eq!(snapshot(files), before);
        assert!(sandbox.backups().is_empty());
        if let Some(bridge) = &files.bridge {
            assert!(!bridge.exists());
        }
        let calls = boxes.calls();
        assert_eq!(calls.len(), 1, "{calls:?}");
        assert_eq!(calls[0].0, "error");
        assert_eq!(calls[0].2, expected);
        assert!(!expected.is_empty());
    }

    #[test]
    fn settings_that_are_not_json_are_refused() {
        let sandbox = Sandbox::new("not-json");
        let files = sandbox.files(true);
        fs::write(&files.settings, "this is not json").unwrap();
        let expected = connection::plan_connect(
            &connection::read_settings(&files.settings).unwrap(),
            &AppConfig::default(),
            files.bridge.as_deref().unwrap(),
        )
        .unwrap_err()
        .to_string();
        refused(&sandbox, &files, true, expected);
    }

    #[test]
    fn settings_that_are_not_utf8_are_refused() {
        let sandbox = Sandbox::new("not-utf8");
        let files = sandbox.files(true);
        fs::write(&files.settings, [0xFF, 0xFE, 0xFA, 0x80]).unwrap();
        let expected = connection::read_settings(&files.settings)
            .unwrap_err()
            .to_string();
        refused(&sandbox, &files, true, expected);
    }

    #[test]
    fn a_config_that_is_not_json_is_refused() {
        let sandbox = Sandbox::new("config-bad");
        let files = sandbox.files(true);
        fs::write(&files.settings, OLD).unwrap();
        fs::write(&files.config, "not json at all").unwrap();
        let expected = AppConfig::load(&files.config).unwrap_err().to_string();
        refused(&sandbox, &files, true, expected);
    }

    #[test]
    fn connecting_with_no_bridge_is_refused() {
        let sandbox = Sandbox::new("no-bridge");
        let files = sandbox.files(false);
        fs::write(&files.settings, OLD).unwrap();
        refused(&sandbox, &files, true, NO_BRIDGE.to_string());
    }

    #[test]
    fn answering_no_changes_nothing() {
        let sandbox = Sandbox::new("no");
        let files = sandbox.files(true);
        fs::write(&files.settings, OLD).unwrap();
        let boxes = Fake::new(false);
        run_with(true, &files, &boxes, 1);

        assert_eq!(text(&files.settings), OLD);
        assert!(!files.config.exists());
        assert!(!files.bridge.clone().unwrap().exists());
        assert!(sandbox.backups().is_empty());
        let calls = boxes.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "ask");
    }

    /// Boxes whose Yes comes after another program rewrote `settings.json`.
    struct ChangedMeanwhile<'a> {
        inner: Fake,
        settings: &'a Path,
    }

    impl Boxes for ChangedMeanwhile<'_> {
        fn ask(&self, title: &str, text: &str) -> bool {
            fs::write(self.settings, "{}\n").unwrap();
            self.inner.ask(title, text)
        }
        fn tell(&self, title: &str, text: &str) {
            self.inner.tell(title, text);
        }
        fn error(&self, text: &str) {
            self.inner.error(text);
        }
    }

    #[test]
    fn a_settings_json_changed_while_the_box_was_open_is_left_alone() {
        let sandbox = Sandbox::new("changed");
        let files = sandbox.files(true);
        fs::write(&files.settings, OLD).unwrap();
        let boxes = ChangedMeanwhile {
            inner: Fake::new(true),
            settings: &files.settings,
        };
        let open: Opener<'_> = &opener;
        run(true, &files, Some(open), &boxes, when(1));

        assert_eq!(text(&files.settings), "{}\n");
        assert!(!files.config.exists());
        assert!(sandbox.backups().is_empty());
        let calls = boxes.inner.calls();
        assert_eq!(calls.len(), 2, "{calls:?}");
        assert_eq!(calls[1].0, "error");
        assert_eq!(calls[1].2, ConnectError::Changed.to_string());
    }

    #[test]
    fn a_second_flow_is_turned_away_while_one_is_open() {
        let gate = Gate::new();
        let first = gate.take();
        assert!(first.is_some());
        assert!(gate.take().is_none());
        drop(first);
        assert!(gate.take().is_some());
    }

    #[test]
    fn the_state_follows_settings_json() {
        let sandbox = Sandbox::new("state");
        let files = sandbox.files(true);
        let own = files.bridge.clone().unwrap();

        // Missing.
        assert_eq!(
            state_of(&files.settings, Some(&own)),
            ConnectionState::NotConnected
        );
        // Not UTF-8.
        fs::write(&files.settings, [0xFF, 0xFE, 0x80]).unwrap();
        assert_eq!(
            state_of(&files.settings, Some(&own)),
            ConnectionState::NotConnected
        );
        // Running the own bridge.
        let running = |path: &Path| {
            format!(
                "{{\"statusLine\":{{\"type\":\"command\",\"command\":{}}}}}",
                serde_like(&command_for(path))
            )
        };
        fs::write(&files.settings, running(&own)).unwrap();
        assert_eq!(
            state_of(&files.settings, Some(&own)),
            ConnectionState::Connected
        );
        // Running one elsewhere.
        let other = sandbox.root.join("other").join("sobreiro-bridge.exe");
        fs::write(&files.settings, running(&other)).unwrap();
        assert_eq!(
            state_of(&files.settings, Some(&own)),
            ConnectionState::Elsewhere
        );
    }
}
