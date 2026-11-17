//! The few Windows calls the tool needs. It only looks, except for `ask_to_quit`.
// Unsafe code is allowed here, as in the shell's `win` files: each block says why it is sound.
#![allow(unsafe_code)]

use sobreiro_baseline::{Date, Rect, WindowFacts, is_pill};
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::time::Duration;
use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Media::{timeBeginPeriod, timeEndPeriod};
use windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FindWindowW, GW_OWNER, GetWindow, GetWindowRect, GetWindowThreadProcessId,
    IsWindowVisible, PostMessageW, WM_QUERYENDSESSION,
};
use windows::core::{BOOL, PCWSTR, PWSTR, w};

fn handle_of(window: HWND) -> isize {
    window.0 as isize
}

fn window_of(handle: isize) -> HWND {
    HWND(handle as *mut c_void)
}

fn rect_of(r: RECT) -> Rect {
    Rect::new(r.left, r.top, r.right, r.bottom)
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

/// The taskbar's window and where it is.
pub fn taskbar() -> Option<(isize, Rect)> {
    // SAFETY: a class name and no window name; the call only reads them.
    let window = unsafe { FindWindowW(w!("Shell_TrayWnd"), PCWSTR::null()) }.ok()?;
    let mut r = RECT::default();
    // SAFETY: `r` is a RECT, alive for the call.
    unsafe { GetWindowRect(window, &mut r) }.ok()?;
    Some((handle_of(window), rect_of(r)))
}

struct Found {
    handle: isize,
    process_id: u32,
    facts: WindowFacts,
}

unsafe extern "system" fn collect(window: HWND, list: LPARAM) -> BOOL {
    // SAFETY: `list` is the address of the Vec that `windows()` passed, alive during the call.
    let found = unsafe { &mut *(list.0 as *mut Vec<Found>) };
    let mut process_id = 0u32;
    // SAFETY: `process_id` is a u32, alive for the call.
    unsafe { GetWindowThreadProcessId(window, Some(&mut process_id)) };
    let mut r = RECT::default();
    // SAFETY: `r` is a RECT, alive for the call.
    if unsafe { GetWindowRect(window, &mut r) }.is_ok() {
        // SAFETY: plain queries about a window handle; a stale one just answers no.
        let (visible, owner) = unsafe {
            (
                IsWindowVisible(window).as_bool(),
                GetWindow(window, GW_OWNER).map_or(0, handle_of),
            )
        };
        found.push(Found {
            handle: handle_of(window),
            process_id,
            facts: WindowFacts {
                visible,
                owner,
                rect: rect_of(r),
            },
        });
    }
    BOOL(1)
}

/// Every top-level window, hidden ones included, with the process it belongs to.
fn windows() -> Vec<Found> {
    let mut found: Vec<Found> = Vec::new();
    // SAFETY: the callback only uses the pointer during this call, while `found` is alive.
    let _ = unsafe {
        EnumWindows(
            Some(collect),
            LPARAM(&mut found as *mut Vec<Found> as isize),
        )
    };
    found
}

fn windows_of(process_id: u32) -> Vec<Found> {
    windows()
        .into_iter()
        .filter(|w| w.process_id == process_id)
        .collect()
}

/// The windows of the process, with the facts about each: handle first.
pub fn facts_of(process_id: u32) -> Vec<(isize, WindowFacts)> {
    windows_of(process_id)
        .into_iter()
        .map(|w| (w.handle, w.facts))
        .collect()
}

pub fn pill_shown(process_id: u32) -> bool {
    taskbar().is_some_and(|(handle, rect)| {
        windows_of(process_id)
            .iter()
            .any(|w| is_pill(&w.facts, handle, &rect))
    })
}

/// The process of a pill that is already in the taskbar, if any: one that is not Explorer's own.
/// A second pill finds no room beside it and stays hidden, so nothing can be timed while it is
/// there.
pub fn pill_of_another_app() -> Option<u32> {
    let (handle, rect) = taskbar()?;
    let mut explorer = 0u32;
    // SAFETY: `explorer` is a u32, alive for the call.
    unsafe { GetWindowThreadProcessId(window_of(handle), Some(&mut explorer)) };
    windows()
        .into_iter()
        .find(|w| w.process_id != explorer && is_pill(&w.facts, handle, &rect))
        .map(|w| w.process_id)
}

/// Asks the app to leave the way Windows does when a session ends, so it takes its tray icon
/// with it: the shell's hidden window answers by quitting. An app that does not answer is
/// killed by the caller.
pub fn ask_to_quit(process_id: u32) {
    for (handle, _) in facts_of(process_id) {
        // SAFETY: posts a message with no pointers in it to a window handle.
        let _ = unsafe {
            PostMessageW(
                Some(window_of(handle)),
                WM_QUERYENDSESSION,
                WPARAM(0),
                LPARAM(0),
            )
        };
    }
}

/// Sleeps are a millisecond long, not the default 15, while this value lives.
pub struct FineTimer;

impl FineTimer {
    pub fn start() -> FineTimer {
        // SAFETY: a plain request to the system timer, undone in `drop`.
        unsafe { timeBeginPeriod(1) };
        FineTimer
    }
}

impl Drop for FineTimer {
    fn drop(&mut self) {
        // SAFETY: undoes the request made in `start`.
        unsafe { timeEndPeriod(1) };
    }
}

fn duration_of(time: FILETIME) -> Duration {
    let ticks = (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime);
    // A tick is 100 ns.
    Duration::from_nanos(ticks.saturating_mul(100))
}

/// A running process's figures.
pub struct ProcessFigures {
    pub private_bytes: u64,
    pub working_set_bytes: u64,
    pub cpu: Duration,
}

/// The figures of the process behind `handle`, which must allow querying it, as a child's own
/// handle does.
pub fn figures_of(handle: HANDLE) -> Option<ProcessFigures> {
    let mut memory = PROCESS_MEMORY_COUNTERS_EX::default();
    let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
    memory.cb = size;
    // SAFETY: the structure is the one this call fills, with its size, alive for the call. It
    // starts with the base counters structure, as the call expects.
    unsafe {
        GetProcessMemoryInfo(
            handle,
            (&mut memory as *mut PROCESS_MEMORY_COUNTERS_EX).cast(),
            size,
        )
    }
    .ok()?;
    let (mut created, mut exited, mut kernel, mut user) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    // SAFETY: four FILETIMEs, alive for the call.
    unsafe { GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user) }.ok()?;
    Some(ProcessFigures {
        private_bytes: memory.PrivateUsage as u64,
        working_set_bytes: memory.WorkingSetSize as u64,
        cpu: duration_of(kernel) + duration_of(user),
    })
}

/// The name of a process without its extension, or "gone" when it cannot be asked.
pub fn name_of(process_id: u32) -> String {
    let name = || -> Option<String> {
        // SAFETY: opens a handle to read the name only; closed below.
        let handle =
            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }.ok()?;
        let mut buffer = vec![0u16; 1024];
        let mut length = buffer.len() as u32;
        // SAFETY: the buffer and its length are alive for the call.
        let asked = unsafe {
            QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut length,
            )
        };
        // SAFETY: the handle was opened above and is closed once.
        let _ = unsafe { CloseHandle(handle) };
        asked.ok()?;
        let path = String::from_utf16_lossy(buffer.get(..length as usize)?);
        Path::new(&path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
    };
    name().unwrap_or_else(|| "gone".to_string())
}

/// Today's date and the time of day on this PC's clock: hour, minute, second.
pub fn local_now() -> (Date, u16, u16, u16) {
    // SAFETY: takes no arguments and returns the time by value.
    let t = unsafe { GetLocalTime() };
    (
        Date {
            year: t.wYear,
            month: t.wMonth as u8,
            day: t.wDay as u8,
        },
        t.wHour,
        t.wMinute,
        t.wSecond,
    )
}

/// "0.10.0" from an exe's product version "0.10.0+commit": the version without the commit.
/// None when the file has none.
pub fn product_version(path: &Path) -> Option<String> {
    let file: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: `file` is a NUL-ended path, alive for the calls below.
    let size = unsafe { GetFileVersionInfoSizeW(PCWSTR(file.as_ptr()), None) };
    if size == 0 {
        return None;
    }
    let mut block = vec![0u8; size as usize];
    // SAFETY: the block is `size` bytes, as asked.
    unsafe { GetFileVersionInfoW(PCWSTR(file.as_ptr()), None, size, block.as_mut_ptr().cast()) }
        .ok()?;

    // The first language and code page the file lists, then that language's ProductVersion.
    let (language, length) = query(&block, "\\VarFileInfo\\Translation")?;
    // Its length is in bytes: one language and code page take 4.
    if length < 4 {
        return None;
    }
    let pair = units_in(&block, language, 2)?;
    let (lang, page) = (*pair.first()?, *pair.get(1)?);
    let (text, chars) = query(
        &block,
        &format!("\\StringFileInfo\\{lang:04x}{page:04x}\\ProductVersion"),
    )?;
    let version = String::from_utf16_lossy(&units_in(&block, text, chars)?);
    let version = version
        .trim_end_matches('\0')
        .split('+')
        .next()?
        .to_string();
    (!version.is_empty()).then_some(version)
}

/// `count` UTF-16 units at `at`, read as bytes from `block`: none when they do not lie wholly
/// inside it, so a malformed version block cannot make this read past it or unaligned.
fn units_in(block: &[u8], at: *const c_void, count: u32) -> Option<Vec<u16>> {
    let start = (at as usize).checked_sub(block.as_ptr() as usize)?;
    let end = start.checked_add(usize::try_from(count).ok()?.checked_mul(2)?)?;
    let bytes = block.get(start..end)?;
    Some(
        bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes(*pair))
            .collect(),
    )
}

/// A pointer into `block` and the length `VerQueryValueW` gives for `sub_block`. The pointer
/// is only compared with `block`'s bounds, never read through.
fn query(block: &[u8], sub_block: &str) -> Option<(*const c_void, u32)> {
    let name = wide(sub_block);
    let mut value: *mut c_void = std::ptr::null_mut();
    let mut length = 0u32;
    // SAFETY: `block` is the whole version block, `name` is NUL-ended, and both outputs are
    // locals, alive for the call. The pointer returned points into `block`.
    let found = unsafe {
        VerQueryValueW(
            block.as_ptr().cast(),
            PCWSTR(name.as_ptr()),
            &mut value,
            &mut length,
        )
    };
    (found.as_bool() && !value.is_null()).then_some((value.cast_const(), length))
}
