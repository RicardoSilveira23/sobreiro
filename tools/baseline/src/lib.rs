//! The logic of the measuring tool: the figures, the table row and where it goes, and what
//! counts as a pill. No Windows calls here; the binary makes those.

use std::time::Duration;

/// A window's edges in pixels: `right` and `bottom` are the first pixels outside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Rect {
        Rect {
            left,
            top,
            right,
            bottom,
        }
    }

    pub const fn width(&self) -> i32 {
        self.right - self.left
    }

    pub const fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

/// What the tool reads about one top-level window of the app it measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowFacts {
    pub visible: bool,
    pub owner: isize,
    pub rect: Rect,
}

/// True when the window is a pill someone can see: visible, owned by the taskbar, not empty and
/// wholly inside the taskbar. A pill still parked off screen, waiting to be placed, does not
/// count, and neither does a menu or flyout that opens from the taskbar and reaches above it.
pub fn is_pill(window: &WindowFacts, taskbar: isize, taskbar_rect: &Rect) -> bool {
    taskbar != 0
        && window.visible
        && window.owner == taskbar
        && window.rect.width() > 0
        && window.rect.height() > 0
        && window.rect.left >= taskbar_rect.left
        && window.rect.right <= taskbar_rect.right
        && window.rect.top >= taskbar_rect.top
        && window.rect.bottom <= taskbar_rect.bottom
}

/// The processor time used over `window`, scaled to one hour, in seconds. None for no window.
pub fn cpu_seconds_per_hour(before: Duration, after: Duration, window: Duration) -> Option<f64> {
    if window.is_zero() {
        return None;
    }
    Some((after.as_secs_f64() - before.as_secs_f64()) * 3600.0 / window.as_secs_f64())
}

/// The middle value, or the mean of the middle two; None for an empty list.
pub fn median(values: &[Duration]) -> Option<Duration> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort();
    let mid = sorted.len() / 2;
    Some(if sorted.len() % 2 == 1 {
        sorted[mid]
    } else {
        (sorted[mid - 1] + sorted[mid]) / 2
    })
}

/// A calendar date, as the row prints it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

/// The tray after its idle run: memory at the end, processor time scaled to an hour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IdleFigures {
    pub private_bytes: u64,
    pub working_set_bytes: u64,
    pub cpu_seconds_per_hour: f64,
}

/// One measured build: a row of docs/baseline.md. `idle` is None when the tray was not started
/// at all (a bridge-only run); a pill time is None when the tray ran and no pill was seen.
#[derive(Debug, Clone, PartialEq)]
pub struct BaselineRun {
    pub build: String,
    pub date: Date,
    pub idle: Option<IdleFigures>,
    pub first_pill_first_start: Option<Duration>,
    pub first_pill_later_starts: Option<Duration>,
    pub tray_exe_bytes: Option<u64>,
    pub bridge_exe_bytes: Option<u64>,
    pub bridge_run: Option<Duration>,
}

pub const HEADER: &str = "| Build | Date | Private memory | Working set | CPU per idle hour | First pill, first start | First pill, later starts | Sobreiro.exe | Bridge exe | Bridge run |";

pub const DIVIDER: &str = "|---|---|---|---|---|---|---|---|---|---|";

const BYTES_PER_MB: f64 = 1024.0 * 1024.0;
const NOT_MEASURED: &str = "not measured";

/// The run as one table row.
pub fn row(run: &BaselineRun) -> String {
    let mb = |bytes: u64, places: usize| format!("{:.places$} MB", bytes as f64 / BYTES_PER_MB);
    let seconds = |time: Option<Duration>| match time {
        Some(t) => format!("{:.2} s", t.as_secs_f64()),
        None if run.idle.is_none() => NOT_MEASURED.to_string(),
        None => "not seen".to_string(),
    };
    let cells = [
        run.build.clone(),
        format!(
            "{:04}-{:02}-{:02}",
            run.date.year, run.date.month, run.date.day
        ),
        run.idle
            .map_or(NOT_MEASURED.to_string(), |i| mb(i.private_bytes, 0)),
        run.idle
            .map_or(NOT_MEASURED.to_string(), |i| mb(i.working_set_bytes, 0)),
        run.idle.map_or(NOT_MEASURED.to_string(), |i| {
            format!("{:.1} s", i.cpu_seconds_per_hour)
        }),
        seconds(run.first_pill_first_start),
        seconds(run.first_pill_later_starts),
        run.tray_exe_bytes
            .map_or(NOT_MEASURED.to_string(), |b| mb(b, 1)),
        run.bridge_exe_bytes
            .map_or("none".to_string(), |b| mb(b, 1)),
        run.bridge_run.map_or("none".to_string(), |t| {
            format!("{:.0} ms", t.as_secs_f64() * 1000.0)
        }),
    ];
    format!("| {} |", cells.join(" | "))
}

/// The document with the run's row in the first table under `## <heading>`: in place of the row
/// for the same build, else after the last row. None when that heading or its table isn't there.
pub fn add(markdown: &str, heading: &str, run: &BaselineRun) -> Option<String> {
    let mut lines: Vec<&str> = markdown.split('\n').collect();
    let ending = if markdown.contains("\r\n") { "\r" } else { "" };
    let wanted = format!("## {heading}");
    let at = lines.iter().position(|l| l.trim_end() == wanted)?;

    // The table: the first run of lines starting with "|" before the next heading.
    let mut first = at + 1;
    while first < lines.len() && !lines[first].starts_with('|') {
        if lines[first].starts_with('#') {
            return None;
        }
        first += 1;
    }
    let mut end = first;
    while end < lines.len() && lines[end].starts_with('|') {
        end += 1;
    }
    // A header and a divider at least.
    if end - first < 2 {
        return None;
    }

    let new_row = format!("{}{ending}", row(run));
    let same = (first + 2..end).find(|&i| first_cell(lines[i]) == run.build);
    match same {
        Some(i) => lines[i] = &new_row,
        None => lines.insert(end, &new_row),
    }
    Some(lines.join("\n"))
}

fn first_cell(line: &str) -> &str {
    line.split('|').nth(1).map_or("", str::trim)
}
