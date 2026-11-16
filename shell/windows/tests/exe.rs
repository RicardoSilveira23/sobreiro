//! The build gives one exe with nothing beside it: the fonts and the C runtime are inside, and it
//! runs from a folder that holds nothing else. It has 1.1's icon and version details.
// A test fails by panicking, in the helpers below as in the tests themselves; the version details
// and the icon are read with Windows' own calls.
#![allow(unsafe_code, clippy::unwrap_used)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use sobreiro_core::connect::bridge_command::{command_for, status_line_json};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};
use windows::Win32::System::Threading::{OpenMutexW, SYNCHRONIZATION_SYNCHRONIZE};
use windows::Win32::UI::Shell::ExtractIconExW;
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, HICON};
use windows::core::{HSTRING, PCWSTR};

const EXE: &str = env!("CARGO_BIN_EXE_Sobreiro");
const FONTS: [&str; 3] = [
    "Inter-Regular.ttf",
    "Inter-Medium.ttf",
    "Unbounded-Bold.ttf",
];
const TABULAR_FONTS: [&str; 3] = [
    "InterTabular-Regular.ttf",
    "InterTabular-Medium.ttf",
    "UnboundedTabular-Bold.ttf",
];

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// A new folder for one test, with a copy of the exe alone in its `app` folder. The data folder and
/// the Claude config folder are `data` and `claude` beside `app`, so no run of a test can read or
/// write the user's, and nothing is written next to the exe.
struct Layout {
    root: PathBuf,
}

impl Layout {
    fn new(name: &str) -> Layout {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("app")).unwrap();
        std::fs::copy(EXE, root.join("app").join("Sobreiro.exe")).unwrap();
        Layout { root }
    }

    fn data(&self) -> PathBuf {
        self.root.join("data")
    }

    fn bridge(&self) -> PathBuf {
        self.data().join("sobreiro-bridge.exe")
    }

    /// Writes the Claude config folder's `settings.json`.
    fn settings(&self, text: &str) {
        std::fs::create_dir_all(self.root.join("claude")).unwrap();
        std::fs::write(self.root.join("claude").join("settings.json"), text).unwrap();
    }

    /// Writes a bridge into the data folder.
    fn put_bridge(&self, bytes: &[u8]) {
        std::fs::create_dir_all(self.data()).unwrap();
        std::fs::write(self.bridge(), bytes).unwrap();
    }

    fn run(&self, argument: &str) -> Output {
        let app = self.root.join("app");
        let output = Command::new(app.join("Sobreiro.exe"))
            .arg(argument)
            .current_dir(&app)
            .env("SOBREIRO_DATA_DIR", self.data())
            .env("CLAUDE_CONFIG_DIR", self.root.join("claude"))
            .output()
            .unwrap();
        let left: Vec<_> = std::fs::read_dir(&app)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(left, ["Sobreiro.exe"], "the run left files beside the exe");
        output
    }

    /// Starts the app with no argument, in this folder. The copy is ended when the guard drops.
    /// Its polls start at once: the GitHub tokens of the environment and `gh` on the PATH are kept
    /// from it, but a Copilot sign-in in Credential Manager is still found.
    fn spawn(&self) -> Running {
        let app = self.root.join("app");
        let child = Command::new(app.join("Sobreiro.exe"))
            .current_dir(&app)
            .env("SOBREIRO_DATA_DIR", self.data())
            .env("CLAUDE_CONFIG_DIR", self.root.join("claude"))
            .env_remove("COPILOT_GITHUB_TOKEN")
            .env_remove("GH_TOKEN")
            .env_remove("GITHUB_TOKEN")
            .env("PATH", "")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        Running(child)
    }

    /// The name of the mutex a copy on this folder holds.
    fn mutex_name(&self) -> String {
        let id = sobreiro_core::app_paths::AppPaths::new(Some(self.data().as_os_str()), &self.root)
            .instance_id();
        format!("Local\\{id}.Tray")
    }

    /// Waits until a copy on this folder holds its mutex.
    fn wait_for_mutex(&self) {
        let name = HSTRING::from(self.mutex_name());
        let end = Instant::now() + Duration::from_secs(15);
        loop {
            // SAFETY: the name outlives the call; a handle that comes back is closed at once.
            if let Ok(handle) = unsafe { OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, false, &name) } {
                let _ = unsafe { CloseHandle(handle) };
                return;
            }
            assert!(Instant::now() < end, "the copy never took its mutex");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// What `--refresh-bridge` says: the one word of the outcome.
    fn refresh(&self) -> String {
        let output = self.run("--refresh-bridge");
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }
}

/// A copy of the app that runs; it is killed and waited for when this drops.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Runs a copy of the exe that sits alone in a new folder.
fn run_alone(name: &str, argument: &str) -> Output {
    Layout::new(name).run(argument)
}

/// A `settings.json` whose status line runs the bridge at `path`.
fn settings_running(path: &Path) -> String {
    let line = status_line_json(&command_for(path), None).unwrap();
    format!("{{\"statusLine\": {line}}}")
}

/// The bridge this build carries, when it carries one.
fn built_bridge() -> Option<Vec<u8>> {
    option_env!("SOBREIRO_BRIDGE_EXE").map(|path| std::fs::read(path).unwrap())
}

#[test]
fn the_fonts_are_inside_the_exe() {
    let exe = std::fs::read(EXE).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let folders = [
        root.join("src/Sobreiro/Fonts"),
        root.join("shell/windows/fonts"),
    ];
    for (folder, names) in folders.iter().zip([FONTS, TABULAR_FONTS]) {
        for name in names {
            let font = std::fs::read(folder.join(name)).unwrap();
            assert!(contains(&exe, &font), "{name} is not inside the exe");
        }
    }
}

#[test]
fn the_notices_and_licences_are_inside_the_exe() {
    let exe = std::fs::read(EXE).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = [
        "notices.txt",
        "../../LICENSE",
        "../../src/Sobreiro/Fonts/OFL-Inter.txt",
        "../../src/Sobreiro/Fonts/OFL-Unbounded.txt",
    ];
    for name in files {
        let text = std::fs::read_to_string(root.join(name)).unwrap();
        let text = text.replace("\r\n", "\n");
        assert!(
            contains(&exe, text.as_bytes()),
            "{name} is not inside the exe"
        );
    }
}

/// The exe's version resource, as Windows reads it.
struct Details {
    block: Vec<u8>,
}

impl Details {
    fn of_the_exe() -> Details {
        let path = HSTRING::from(EXE);
        // SAFETY: `path` is a NUL-ended string that outlives the calls, and `block` is as long as
        // Windows said the resource is.
        let block = unsafe {
            let size = GetFileVersionInfoSizeW(&path, None);
            assert!(size > 0, "the exe has no version details");
            let mut block = vec![0u8; size as usize];
            GetFileVersionInfoW(&path, None, size, block.as_mut_ptr().cast()).unwrap();
            block
        };
        Details { block }
    }

    /// The bytes `VerQueryValueW` finds at `query`. Windows gives a string's length in WORDs
    /// and a binary value's in bytes, so `text` says which this is.
    fn query(&self, query: &str, text: bool) -> &[u8] {
        let query = HSTRING::from(query);
        let mut found = std::ptr::null_mut();
        let mut length = 0u32;
        // SAFETY: `block` is a version resource Windows filled in; on success `found` points
        // inside it at a value `length` WORDs long for a string and `length` bytes long
        // otherwise, and the slice is not used after `self` is gone.
        unsafe {
            let ok = VerQueryValueW(
                self.block.as_ptr().cast(),
                PCWSTR(query.as_ptr()),
                &mut found,
                &mut length,
            );
            assert!(ok.as_bool(), "the exe has no {query}");
            let bytes = if text {
                length as usize * 2
            } else {
                length as usize
            };
            std::slice::from_raw_parts(found.cast::<u8>(), bytes)
        }
    }

    /// A string under the neutral, Unicode table.
    fn string(&self, name: &str) -> String {
        let bytes = self.query(&format!(r"\StringFileInfo\000004b0\{name}"), true);
        let (pairs, _) = bytes.as_chunks::<2>();
        let units: Vec<u16> = pairs.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
        String::from_utf16(&units)
            .unwrap()
            .trim_end_matches('\0')
            .to_string()
    }
}

#[test]
fn the_exe_has_the_version_details() {
    let details = Details::of_the_exe();
    let major = env!("CARGO_PKG_VERSION_MAJOR");
    let minor = env!("CARGO_PKG_VERSION_MINOR");
    let patch = env!("CARGO_PKG_VERSION_PATCH");
    let file_version = format!("{major}.{minor}.{patch}.0");
    let expected = [
        ("CompanyName", "Sobreiro"),
        ("FileDescription", "Sobreiro"),
        ("FileVersion", file_version.as_str()),
        ("InternalName", "Sobreiro"),
        ("LegalCopyright", "Copyright 2026 Ricardo Silveira"),
        ("OriginalFilename", "Sobreiro.exe"),
        ("ProductName", "Sobreiro"),
        ("ProductVersion", env!("CARGO_PKG_VERSION")),
    ];
    for (name, value) in expected {
        assert_eq!(details.string(name), value, "{name}");
    }

    // The table Explorer is pointed at: language neutral, Unicode.
    let translation = details.query(r"\VarFileInfo\Translation", false);
    assert_eq!(translation, [0x00, 0x00, 0xB0, 0x04], "translation");

    // The fixed numbers: the file's and the product's version, with 0 as the fourth part.
    let fixed = details.query(r"\", false);
    let dword = |at: usize| u32::from_le_bytes(fixed[at..at + 4].try_into().unwrap());
    assert_eq!(dword(0), 0xFEEF_04BD, "signature");
    let number = |text: &str| text.parse::<u32>().unwrap();
    let high = (number(major) << 16) | number(minor);
    let low = number(patch) << 16;
    assert_eq!((dword(8), dword(12)), (high, low), "file version");
    assert_eq!((dword(16), dword(20)), (high, low), "product version");
}

#[test]
fn the_exe_has_the_icon() {
    let path = HSTRING::from(EXE);
    // SAFETY: with an index of -1 and no arrays, this only counts the icon groups.
    let groups = unsafe { ExtractIconExW(&path, -1, None, None, 0) };
    assert_eq!(groups, 1, "the exe should have one icon");
    // The group loads as an icon, its entries pointing at the images.
    let mut large = HICON::default();
    // SAFETY: one icon asked for, into one handle that lives on this stack; destroyed below.
    let loaded = unsafe { ExtractIconExW(&path, 0, Some(&mut large), None, 1) };
    assert_eq!(loaded, 1, "the exe's icon does not load");
    assert!(!large.is_invalid(), "the exe's icon does not load");
    // SAFETY: the handle came from `ExtractIconExW` above and is destroyed once.
    unsafe { DestroyIcon(large) }.unwrap();

    // Each image of 1.1's icon is inside the exe as it is in the file.
    let ico = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src/Sobreiro/Assets/sobreiro.ico"),
    )
    .unwrap();
    let exe = std::fs::read(EXE).unwrap();
    let count = usize::from(u16::from_le_bytes([ico[4], ico[5]]));
    assert!(count > 0);
    for index in 0..count {
        let entry = &ico[6 + 16 * index..][..16];
        let size = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as usize;
        let start = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as usize;
        assert!(
            contains(&exe, &ico[start..start + size]),
            "image {index} of the icon is not inside the exe"
        );
    }
}

#[test]
fn the_c_runtime_is_inside_the_exe() {
    let exe = std::fs::read(EXE).unwrap().to_ascii_lowercase();
    for dll in ["vcruntime", "api-ms-win-crt"] {
        assert!(
            !contains(&exe, dll.as_bytes()),
            "the exe asks Windows for a {dll} DLL"
        );
    }
}

#[test]
fn alone_in_a_folder_it_says_its_version() {
    let output = run_alone("version", "--version");
    assert!(output.status.success());
    let expected = format!("Sobreiro {}\n", env!("CARGO_PKG_VERSION"));
    assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
}

/// `--specimen` draws one line of text in each font, with no window, and prints how many pixels
/// wide the ink of each line is. Three different faces give three different widths; a font that
/// was not found would be drawn in another one's face.
#[test]
fn alone_in_a_folder_it_draws_each_font() {
    let output = run_alone("specimen", "--specimen");
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    let widths: Vec<(&str, u32)> = text
        .lines()
        .map(|line| {
            line.split_once(": ")
                .unwrap_or_else(|| panic!("not a line of the specimen: {line}"))
        })
        .map(|(name, width)| (name, width.parse().unwrap()))
        .collect();
    let names: Vec<&str> = widths.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, ["Inter 400", "Inter 500", "Unbounded 700"]);
    let (regular, medium, bold) = (widths[0].1, widths[1].1, widths[2].1);
    assert!(regular > 0, "Inter 400 drew nothing");
    assert!(
        medium > regular,
        "Inter 500 ({medium}) is not wider than Inter 400 ({regular})"
    );
    assert!(
        bold > medium,
        "Unbounded 700 ({bold}) is not wider than Inter 500 ({medium})"
    );
}

#[test]
#[cfg_attr(debug_assertions, ignore = "only the release build carries the bridge")]
fn the_bridge_is_inside_the_exe() {
    let bridge = built_bridge().unwrap();
    assert!(bridge.starts_with(b"MZ"), "the bridge is not a PE file");
    let exe = std::fs::read(EXE).unwrap();
    assert!(contains(&exe, &bridge), "the bridge is not inside the exe");
}

#[test]
#[cfg_attr(not(debug_assertions), ignore = "the release build carries the bridge")]
fn without_the_bridge_the_step_says_so() {
    let layout = Layout::new("bridge-not-carried");
    layout.settings(&settings_running(&layout.bridge()));
    assert_eq!(layout.refresh(), "not-carried");
    assert!(!layout.bridge().exists());
}

#[test]
#[cfg_attr(debug_assertions, ignore = "only the release build carries the bridge")]
fn the_start_writes_the_bridge_and_then_finds_it_current() {
    let layout = Layout::new("bridge-written");
    layout.settings(&settings_running(&layout.bridge()));

    assert_eq!(layout.refresh(), "written");
    assert_eq!(
        std::fs::read(layout.bridge()).unwrap(),
        built_bridge().unwrap()
    );
    assert_eq!(layout.refresh(), "current");
    the_written_bridge_runs(&layout);

    let log = std::fs::read_to_string(layout.data().join("log.txt")).unwrap();
    let line = log
        .lines()
        .find(|line| line.contains("bridge written"))
        .unwrap();
    assert!(!line.contains('\\') && !line.contains(":/"), "{line}");
}

#[test]
#[cfg_attr(debug_assertions, ignore = "only the release build carries the bridge")]
fn a_stale_copy_is_replaced_at_the_start() {
    let layout = Layout::new("bridge-stale");
    layout.settings(&settings_running(&layout.bridge()));
    layout.put_bridge(b"an older bridge");

    assert_eq!(layout.refresh(), "written");
    assert_eq!(
        std::fs::read(layout.bridge()).unwrap(),
        built_bridge().unwrap()
    );
}

#[test]
#[cfg_attr(debug_assertions, ignore = "only the release build carries the bridge")]
fn a_status_line_that_runs_something_else_leaves_the_copy_alone() {
    let layout = Layout::new("bridge-elsewhere");
    layout.settings(&settings_running(Path::new(
        "C:/elsewhere/sobreiro-bridge.exe",
    )));

    assert_eq!(layout.refresh(), "not-used");
    assert!(!layout.bridge().exists());

    layout.put_bridge(b"an older bridge");
    assert_eq!(layout.refresh(), "not-used");
    assert_eq!(std::fs::read(layout.bridge()).unwrap(), b"an older bridge");
}

#[test]
#[cfg_attr(debug_assertions, ignore = "only the release build carries the bridge")]
fn without_settings_the_copy_is_not_used() {
    let layout = Layout::new("bridge-no-settings");
    assert_eq!(layout.refresh(), "not-used");
    assert!(!layout.bridge().exists());
}

/// Runs the data folder's copy as Claude Code's status line does, with a status line's JSON on
/// its input: it prints the line and writes the limits.
fn the_written_bridge_runs(layout: &Layout) {
    let input = r#"{"rate_limits":{"five_hour":{"used_percentage":23.5,"resets_at":1738425600}}}"#;
    let mut child = Command::new(layout.bridge())
        .env("SOBREIRO_DATA_DIR", layout.data())
        .env("CLAUDE_CONFIG_DIR", layout.root.join("claude"))
        .env_remove("SOBREIRO_IN_CHAIN")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .starts_with("5h 23%"),
        "the bridge printed no status line"
    );
    let usage = std::fs::read_to_string(layout.data().join("usage.json")).unwrap();
    assert!(usage.contains("\"used_percentage\":23.5"), "{usage}");
}

#[test]
fn one_copy_runs_per_data_folder() {
    let a = Layout::new("one-copy-a");
    let b = Layout::new("one-copy-b");

    let mut first = a.spawn();
    a.wait_for_mutex();
    assert!(first.0.try_wait().unwrap().is_none());
    // Arguments take no mutex.
    let version = a.run("--version");
    assert!(version.status.success());
    let expected = format!("Sobreiro {}\n", env!("CARGO_PKG_VERSION"));
    assert_eq!(String::from_utf8_lossy(&version.stdout), expected);

    // A second launch on the same folder asks and ends.
    let mut second = a.spawn();
    let end = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = second.0.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < end, "the second launch did not end");
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(status.success());
    // The second launch says so, and the first hears the ask on its UI thread.
    let said = format!("pid={} already-running", second.0.id());
    let heard = format!("pid={} show-asked", first.0.id());
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        let log = std::fs::read_to_string(a.data().join("log.txt")).unwrap_or_default();
        if log.contains(&said) && log.contains(&heard) {
            break;
        }
        assert!(Instant::now() < end, "{log}");
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(first.0.try_wait().unwrap().is_none());

    // Another folder has a copy of its own.
    let mut other = b.spawn();
    b.wait_for_mutex();
    std::thread::sleep(Duration::from_secs(1));
    assert!(other.0.try_wait().unwrap().is_none());

    drop(first);
    drop(second);
    drop(other);
    for layout in [&a, &b] {
        let left: Vec<_> = std::fs::read_dir(layout.root.join("app"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(left, ["Sobreiro.exe"]);
    }
}
