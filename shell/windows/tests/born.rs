//! `Sobreiro.exe --born <owner>` shows a small window that is born owned by the given window and
//! never activated, for the pill, and makes the pill's tooltip window the way the shell does, then
//! shows it and hides it three times. This test runs it for a few seconds beside a window of its own
//! that holds the focus, and looks at both windows from outside: their owner from the first look,
//! their extended style, that neither they nor their thread ever takes the foreground, the focus or
//! the active state, that the tooltip is shown and hidden when the exe says so, and that after its
//! three turns the pill's stand-in still has its owner and the owner has not been raised.
// The test calls Windows to make and look at windows, and fails by panicking.
#![allow(unsafe_code, clippy::unwrap_used, clippy::expect_used)]

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{GetDC, GetPixel, ReleaseDC};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, SetFocus, VK_MENU, keybd_event,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, DispatchMessageW, GUITHREADINFO, GW_HWNDNEXT, GW_OWNER,
    GWL_EXSTYLE, GetForegroundWindow, GetGUIThreadInfo, GetWindow, GetWindowLongPtrW,
    GetWindowRect, GetWindowThreadProcessId, IsWindowVisible, MSG, PM_REMOVE, PeekMessageW,
    SetForegroundWindow, TranslateMessage, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_POPUP, WS_VISIBLE,
};
use windows::core::w;

const EXE: &str = env!("CARGO_BIN_EXE_Sobreiro");

/// A plain window of the "STATIC" class, which needs no class of its own. The test thread pumps
/// its messages while it waits.
fn make_window(visible: bool, x: i32) -> HWND {
    let style = if visible {
        WS_POPUP | WS_VISIBLE
    } else {
        WS_POPUP
    };
    // SAFETY: creates a window on this thread from a system class; the strings are literals.
    unsafe {
        let module = GetModuleHandleW(None).unwrap();
        CreateWindowExW(
            Default::default(),
            w!("STATIC"),
            w!("born test"),
            style,
            x,
            200,
            120,
            80,
            None,
            None,
            Some(module.into()),
            None,
        )
        .unwrap()
    }
}

/// Handles this thread's messages for `time`, so its windows stay responsive, and calls `look` on
/// every turn, until it answers true.
fn pump(time: Duration, mut look: impl FnMut() -> bool) {
    let end = Instant::now() + time;
    while Instant::now() < end {
        let mut message = MSG::default();
        // SAFETY: the usual message loop, on the thread that made the windows.
        unsafe {
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        if look() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Gives the test's window the foreground and the focus. Windows only allows it to a process that
/// has the user's last input, so a tap of Alt comes first, as is usual.
fn take_foreground(window: HWND) {
    // SAFETY: plain calls on a window of this thread; the key events are the user's own Alt key,
    // pressed and let go.
    unsafe {
        keybd_event(VK_MENU.0 as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(VK_MENU.0 as u8, 0, KEYEVENTF_KEYUP, 0);
        let _ = SetForegroundWindow(window);
        let _ = SetFocus(Some(window));
    }
}

/// The exe, ended when the test is left, also by a failed assertion; it would otherwise run on.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Sends each line the exe prints, as it comes.
fn lines(child: &mut Child) -> mpsc::Receiver<String> {
    let stdout = child.stdout.take().unwrap();
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = send.send(line);
        }
    });
    receive
}

/// Waits for the next line, which holds a window's handle. The test thread pumps meanwhile: the
/// exe gives its windows an owner that lives on this thread, and Windows waits for that thread to
/// answer.
fn next_handle(receive: &mpsc::Receiver<String>) -> HWND {
    let mut line = None;
    pump(Duration::from_secs(20), || {
        line = receive.try_recv().ok();
        line.is_some()
    });
    let handle: isize = line
        .expect("the exe printed no window handle")
        .trim()
        .parse()
        .expect("the line is not a handle");
    HWND(handle as *mut _)
}

/// The colour on the screen at a window's top left corner, 3 DIPs in: past the edge (1 DIP, drawn
/// on whole pixels) and in the padding (6 DIPs), at the window's own scale.
/// Reads what the screen shows, so a window that was never painted gives what is behind it.
fn pixel_inside(window: HWND) -> u32 {
    let mut rect = RECT::default();
    // SAFETY: reads a window that is alive, and the screen's colour at one point.
    unsafe {
        GetWindowRect(window, &mut rect).unwrap();
        let dpi = i32::try_from(GetDpiForWindow(window)).unwrap().max(96);
        let inside = (3 * dpi + 48) / 96;
        let screen = GetDC(None);
        let colour = GetPixel(screen, rect.left + inside, rect.top + inside).0;
        ReleaseDC(None, screen);
        colour
    }
}

/// Whether `upper` is above `lower` in the z-order, as far as the windows below `upper` go.
fn is_above(upper: HWND, lower: HWND) -> bool {
    let mut at = upper;
    // SAFETY: walks the windows below `upper`, which are asked about by handle only.
    unsafe {
        while let Ok(next) = GetWindow(at, GW_HWNDNEXT) {
            if next == lower {
                return true;
            }
            at = next;
        }
    }
    false
}

/// What must hold of a born window at every look.
struct Setting {
    mine: HWND,
    stand_in: HWND,
    holds_foreground: bool,
}

/// Whether the window must be shown at a look: when the exe has said so, or has said it is hiding.
/// Nothing is asserted while it is between the two.
fn look(born: HWND, setting: &Setting, looks: u32, visible: Option<bool>) {
    let Setting {
        mine,
        stand_in,
        holds_foreground,
    } = *setting;
    // SAFETY: reads from windows and a thread that are alive.
    unsafe {
        if let Some(visible) = visible {
            assert_eq!(
                IsWindowVisible(born).as_bool(),
                visible,
                "look {looks}: the window is {}",
                if visible { "not shown" } else { "shown" }
            );
        }
        assert_eq!(
            GetWindow(born, GW_OWNER).ok(),
            Some(stand_in),
            "look {looks}: not owned by the stand-in"
        );
        let style = GetWindowLongPtrW(born, GWL_EXSTYLE) as u32;
        assert_ne!(style & WS_EX_TOOLWINDOW.0, 0, "no tool-window style");
        assert_ne!(style & WS_EX_NOACTIVATE.0, 0, "no no-activate style");
        assert_eq!(style & WS_EX_APPWINDOW.0, 0, "the app-window style is set");

        let foreground = GetForegroundWindow();
        assert_ne!(foreground, born, "the window became the foreground one");
        if holds_foreground {
            assert_eq!(foreground, mine, "the foreground moved off the test window");
        }
        let mut info = GUITHREADINFO {
            cbSize: size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        GetGUIThreadInfo(GetWindowThreadProcessId(born, None), &mut info).unwrap();
        // The owner is a window of another thread, so Windows joins the two threads' input
        // state: the exe's thread then reports the window that is active for both, which is the
        // test's own. What counts is that it is never the born window.
        if holds_foreground {
            assert!(
                info.hwndActive.0.is_null() || info.hwndActive == mine,
                "another window than the test's is active"
            );
            assert!(
                info.hwndFocus.0.is_null() || info.hwndFocus == mine,
                "the focus moved off the test window"
            );
        }
        assert_ne!(info.hwndActive, born, "the window is active");
        assert_ne!(info.hwndFocus, born, "the window has the focus");
    }
}

/// The exe makes two windows, one after the other, and both are alive together; each is looked at
/// the same way from the moment its handle is printed. The second is the tooltip's: the exe hides
/// it when it is taken, then shows and hides it three times, and tells the test as it goes.
#[test]
fn the_windows_are_owned_and_never_activated() {
    // Aware of the display's scale, so the rectangles and the screen's pixels the test reads are
    // the physical ones the exe (which is aware) places its windows by.
    // SAFETY: sets this process's awareness, before it makes a window; a refusal is ignored.
    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    let mine = make_window(true, 100);
    let stand_in = make_window(false, 300);
    pump(Duration::from_millis(200), || false);
    take_foreground(mine);
    pump(Duration::from_millis(300), || false);
    // A runner without an interactive desktop does not let any window take the foreground. The
    // foreground checks then have nothing to hold on to and are left out; everything else is
    // still checked, and so is that the born windows never become the foreground one.
    // SAFETY: reads which window is in the foreground.
    let holds_foreground = unsafe { GetForegroundWindow() } == mine;
    if !holds_foreground {
        eprintln!("no foreground for the test window here: the foreground checks are left out");
    }
    let setting = Setting {
        mine,
        stand_in,
        holds_foreground,
    };

    let mut child = Running(
        Command::new(EXE)
            .arg("--born")
            .arg((stand_in.0 as isize).to_string())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let receive = lines(&mut child.0);
    let first = next_handle(&receive);
    look(first, &setting, 0, Some(true));
    let second = next_handle(&receive);
    assert_ne!(first, second, "the same window twice");
    // Where the owner stands against the test's own window, before the tooltip's turns.
    let order_before = (is_above(mine, stand_in), is_above(stand_in, mine));

    // The tooltip's window is hidden until its first turn; then what the exe last said holds. A
    // line comes a moment after what it tells of, and the next turn is 300 ms away, so a state is
    // held to for 200 ms after the line that says it was read, and not asserted after that.
    let mut tooltip_visible = Some(false);
    let mut said_at = Instant::now();
    let (mut shown, mut hidden, mut painted, mut checked) = (0, 0, 0, 0);
    let mut looks = 0;
    pump(Duration::from_millis(2500), || {
        while let Ok(line) = receive.try_recv() {
            said_at = Instant::now();
            match line.as_str() {
                "showing" | "hiding" => tooltip_visible = None,
                "shown" => {
                    shown += 1;
                    tooltip_visible = Some(true);
                }
                "hidden" => {
                    hidden += 1;
                    tooltip_visible = Some(false);
                }
                other => panic!("an unexpected line from the exe: {other}"),
            }
        }
        // Some time after a show the tooltip must be painted: the screen at its corner is its
        // ground, 1.1's tooltip colour, and not what is behind it. Once for each show.
        if holds_foreground
            && tooltip_visible == Some(true)
            && said_at.elapsed() > Duration::from_millis(120)
            && checked < shown
        {
            // Every show is checked: a turn that is missed is not made up for by a later one.
            checked = shown;
            painted += 1;
            assert_eq!(
                // A COLORREF is 0x00BBGGRR: the ground is #F1F2F7.
                pixel_inside(second),
                0x00F7_F2F1,
                "the tooltip is shown but not painted (turn {shown})"
            );
        }
        let expected = tooltip_visible.filter(|_| said_at.elapsed() < Duration::from_millis(200));
        looks += 1;
        look(first, &setting, looks, Some(true));
        look(second, &setting, looks, expected);
        false
    });
    assert!(looks > 50, "too few looks: {looks}");
    assert_eq!((shown, hidden), (3, 3), "the tooltip's three turns");
    assert!(
        painted == 3 || !holds_foreground,
        "the tooltip was painted {painted} of 3 times"
    );
    // After the three turns the stand-in for the pill still has its owner (looked at above, at
    // every look), and the owner was not moved in the z-order along with the tooltip: it stands
    // against the test's own window as it did before.
    assert_eq!(
        (is_above(mine, stand_in), is_above(stand_in, mine)),
        order_before,
        "the owner moved in the z-order"
    );

    // The exe's windows are owned by a window of this thread, so it keeps pumping while it waits.
    let mut status = None;
    pump(Duration::from_secs(15), || {
        status = child.0.try_wait().unwrap();
        status.is_some()
    });
    let status = status.expect("the exe did not end in time");
    // SAFETY: the windows are this thread's and still exist.
    unsafe {
        let _ = DestroyWindow(stand_in);
        let _ = DestroyWindow(mine);
    }
    assert!(status.success(), "the exe ended with {status}");
}

#[test]
fn a_missing_or_wrong_owner_gives_no_window() {
    for arguments in [
        &["--born"][..],
        &["--born", "not a number"],
        &["--born", "1"],
    ] {
        let output = Command::new(EXE).args(arguments).output().unwrap();
        assert_eq!(output.status.code(), Some(2), "{arguments:?}");
        assert!(output.stdout.is_empty(), "{arguments:?} printed a handle");
    }
}
