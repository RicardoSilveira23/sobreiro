//! The runs themselves. Every copy started here gets the scratch folders under `scratch`.

use crate::native;
use sobreiro_baseline::{IdleFigures, Rect, cpu_seconds_per_hour, median};
use std::io::{Read, Write};
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::HANDLE;

const PILL_WAIT: Duration = Duration::from_secs(60);
const QUIT_WAIT: Duration = Duration::from_secs(10);

const STATUS_LINE: &str = r#"{"session_id":"baseline","rate_limits":{"five_hour":{"used_percentage":23.4,"resets_at":1999999999},"seven_day":{"used_percentage":41,"resets_at":1999999999}}}"#;

pub struct Measure<'a> {
    pub scratch: &'a Path,
    pub say: &'a dyn Fn(&str),
}

fn mb(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / (1024.0 * 1024.0))
}

fn ms(time: Duration) -> String {
    format!("{:.0}", time.as_secs_f64() * 1000.0)
}

fn edges(r: &Rect) -> String {
    format!("[{},{},{},{}]", r.left, r.top, r.right, r.bottom)
}

fn handle_of(child: &Child) -> HANDLE {
    HANDLE(child.as_raw_handle())
}

impl Measure<'_> {
    fn folder(&self, name: &str) -> Result<PathBuf, String> {
        let path = self.scratch.join(name);
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("Could not make {}: {e}", path.display()))?;
        Ok(path)
    }

    /// Starts the tray on its own data folder, so it never meets an installed copy. Signed out,
    /// it also gets an empty Claude config folder: repeated starts then make no repeated calls
    /// for Claude's usage.
    fn start_tray(&self, exe: &Path, signed_out: bool) -> Result<Child, String> {
        let mut command = Command::new(exe);
        command
            .current_dir(self.scratch)
            .env("SOBREIRO_DATA_DIR", self.folder("data")?);
        if signed_out {
            command.env("CLAUDE_CONFIG_DIR", self.folder("claude-config")?);
        }
        command
            .spawn()
            .map_err(|e| format!("The tray did not start: {e}"))
    }

    fn wait_for_pill(tray: &mut Child, since_start: Instant) -> Option<Duration> {
        while since_start.elapsed() < PILL_WAIT && matches!(tray.try_wait(), Ok(None)) {
            if native::pill_shown(tray.id()) {
                return Some(since_start.elapsed());
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        None
    }

    /// What was there instead, when no pill was seen.
    fn say_windows(&self, tray: &mut Child) {
        if !matches!(tray.try_wait(), Ok(None)) {
            return;
        }
        let taskbar = match native::taskbar() {
            Some((handle, rect)) => format!("{handle:x} at {}", edges(&rect)),
            None => "not found".to_string(),
        };
        (self.say)(&format!("  taskbar: {taskbar}"));
        for (handle, facts) in native::facts_of(tray.id()) {
            if facts.rect.width() > 0 {
                (self.say)(&format!(
                    "  window {handle:x}: visible={} owner={:x} at {}",
                    if facts.visible { "True" } else { "False" },
                    facts.owner,
                    edges(&facts.rect)
                ));
            }
        }
    }

    fn stop(&self, tray: &mut Child) {
        if !matches!(tray.try_wait(), Ok(None)) {
            return;
        }
        native::ask_to_quit(tray.id());
        let until = Instant::now() + QUIT_WAIT;
        while Instant::now() < until {
            if !matches!(tray.try_wait(), Ok(None)) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        (self.say)(
            "  did not leave when asked: killed (its tray icon stays until the mouse passes over it)",
        );
        let _ = tray.kill();
        let _ = tray.wait();
    }

    /// Time from launch to a pill over the taskbar, once per start.
    pub fn starts(&self, exe: &Path, count: usize) -> Result<Vec<Option<Duration>>, String> {
        let mut times = Vec::new();
        for i in 1..=count {
            let clock = Instant::now();
            let mut tray = self.start_tray(exe, true)?;
            let pill = Self::wait_for_pill(&mut tray, clock);
            times.push(pill);
            match pill {
                Some(t) => (self.say)(&format!("  start {i}: pill after {} ms", ms(t))),
                None => (self.say)(&format!("  start {i}: no pill seen")),
            }
            if pill.is_none() {
                self.say_windows(&mut tray);
            }
            self.stop(&mut tray);
            std::thread::sleep(Duration::from_secs(1));
        }
        Ok(times)
    }

    /// One run with this PC's sign-ins, left alone for `minutes`: memory at the end, and the
    /// processor time from the end of the first minute on, scaled to an hour.
    pub fn idle(&self, exe: &Path, minutes: u64) -> Result<IdleFigures, String> {
        let clock = Instant::now();
        let mut tray = self.start_tray(exe, false)?;
        let result = self.idle_run(&mut tray, clock, minutes);
        self.stop(&mut tray);
        result
    }

    fn idle_run(
        &self,
        tray: &mut Child,
        clock: Instant,
        minutes: u64,
    ) -> Result<IdleFigures, String> {
        let pill = Self::wait_for_pill(tray, clock);
        match pill {
            Some(t) => (self.say)(&format!("  idle run: pill after {} ms", ms(t))),
            None => (self.say)("  idle run: no pill seen"),
        }
        if pill.is_none() {
            self.say_windows(tray);
        }
        let figures = |tray: &Child| {
            native::figures_of(handle_of(tray)).ok_or("Could not read the tray's figures.")
        };
        let mut cpu_after_first_minute = Duration::ZERO;
        let mut last = None;
        for minute in 1..=minutes {
            let until = clock + Duration::from_secs(60 * minute);
            while Instant::now() < until {
                if let Ok(Some(code)) = tray.try_wait() {
                    return Err(format!(
                        "The tray exited after {:.0} s (code {}).",
                        clock.elapsed().as_secs_f64(),
                        code.code().unwrap_or(-1)
                    ));
                }
                std::thread::sleep((until - Instant::now()).min(Duration::from_millis(200)));
            }
            let now = figures(tray)?;
            if minute == 1 {
                cpu_after_first_minute = now.cpu;
            }
            (self.say)(&format!(
                "  minute {minute}: private {} MB, working set {} MB, cpu {:.2} s",
                mb(now.private_bytes),
                mb(now.working_set_bytes),
                now.cpu.as_secs_f64()
            ));
            last = Some(now);
        }
        let end = last.ok_or("The idle run had no minutes.")?;
        let cpu = cpu_seconds_per_hour(
            cpu_after_first_minute,
            end.cpu,
            Duration::from_secs(60 * (minutes - 1)),
        )
        .ok_or("The idle run was too short to measure.")?;
        Ok(IdleFigures {
            private_bytes: end.private_bytes,
            working_set_bytes: end.working_set_bytes,
            cpu_seconds_per_hour: cpu,
        })
    }

    /// The bridge's whole run, launch to exit, on one status line: the median of `count`.
    pub fn bridge(&self, exe: &Path, count: usize) -> Result<Option<Duration>, String> {
        let mut times = Vec::new();
        for _ in 0..count {
            let clock = Instant::now();
            let mut bridge = Command::new(exe)
                .current_dir(self.scratch)
                .env("SOBREIRO_DATA_DIR", self.folder("bridge-data")?)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .map_err(|e| format!("The bridge did not start: {e}"))?;
            if let Some(mut stdin) = bridge.stdin.take() {
                // The bridge may leave before it has read it all; that is its answer to give.
                let _ = stdin.write_all(STATUS_LINE.as_bytes());
            }
            if let Some(mut stdout) = bridge.stdout.take() {
                let mut out = Vec::new();
                let _ = stdout.read_to_end(&mut out);
            }
            bridge
                .wait()
                .map_err(|e| format!("The bridge did not finish: {e}"))?;
            times.push(clock.elapsed());
        }
        let list: Vec<String> = times.iter().map(|t| ms(*t)).collect();
        (self.say)(&format!("  bridge runs, ms: {}", list.join(" ")));
        Ok(median(&times))
    }
}
