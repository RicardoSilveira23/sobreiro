//! SPIKE step 1, before any drawing: does the taskbar take a toolkit window? Borderless, transparent per
//! pixel, never taking focus, owned by the taskbar, placed inside it. Writes what it finds to the trace
//! file and leaves after 12 seconds.
use crate::{Probe, native, shell, spike, trace};
use slint::ComponentHandle;
use std::time::Duration;

pub fn run() -> Result<(), slint::PlatformError> {
    let taskbar = native::find_taskbar();
    let foreground_before = native::foreground();
    // SOBREIRO_SPIKE_OWNER=after: the window is born as the toolkit makes any window, and owned afterwards.
    if spike::env("SOBREIRO_SPIKE_OWNER").as_deref() != Some("after") {
        shell::next_window_owned_by(taskbar);
    }
    let ui = Probe::new()?;
    ui.window().set_position(slint::PhysicalPosition::new(-32000, -32000));
    ui.show()?;
    // The toolkit makes its window once the event loop runs: the handle is asked for from inside the loop.
    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(300), move || {
        let Some(ui) = weak.upgrade() else { return };
        let hwnd = shell::hwnd_of(ui.window());
        trace!(
            "probe render={} taskbar={:?} hwnd={:?} exstyle-after-show=0x{:x} owner-is-taskbar-after-show={}",
            if spike::gpu() { "gpu" } else { "software" },
            taskbar.0,
            hwnd.0,
            native::ex_style(hwnd),
            native::owner(hwnd) == taskbar
        );
        if spike::env("SOBREIRO_SPIKE_OWNER").as_deref() == Some("after") {
            native::set_owner(hwnd, taskbar);
        }
        native::add_ex_style(hwnd, native::WS_EX_OURS);
    });

    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(800), move || {
        let Some(ui) = weak.upgrade() else { return };
        let hwnd = shell::hwnd_of(ui.window());
        let (Some(bar), Some(own)) = (native::window_rect(taskbar), native::window_rect(hwnd)) else { return };
        let x = bar.right - 760;
        let y = bar.top + (bar.height() - own.height()) / 2;
        let moved = native::place(hwnd, x, y, true);
        trace!("moved={moved} to {x},{y}; taskbar [{},{},{},{}]", bar.left, bar.top, bar.right, bar.bottom);
    });

    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(2500), move || {
        let Some(ui) = weak.upgrade() else { return };
        let hwnd = shell::hwnd_of(ui.window());
        let scale = ui.window().scale_factor();
        let own = native::window_rect(hwnd).unwrap_or_default();
        let bar = native::window_rect(taskbar).unwrap_or_default();
        let ex = native::ex_style(hwnd);
        let inside = own.left >= bar.left && own.right <= bar.right && own.top >= bar.top && own.bottom <= bar.bottom;
        trace!(
            "owner-is-taskbar={} visible={} rect=[{},{},{},{}] size={}x{} inside-taskbar={inside} above-taskbar={} \
             foreground-unchanged={} style=0x{:x} exstyle=0x{ex:x} layered={} no-redirection-bitmap={} toolwindow={} \
             noactivate={} window-dpi={} taskbar-dpi={} toolkit-scale={scale}",
            native::owner(hwnd) == taskbar,
            native::is_visible(hwnd),
            own.left,
            own.top,
            own.right,
            own.bottom,
            own.width(),
            own.height(),
            native::is_above(hwnd, taskbar),
            native::foreground() == foreground_before,
            native::style(hwnd),
            ex & 0x0008_0000 != 0,
            ex & 0x0020_0000 != 0,
            ex & 0x80 != 0,
            ex & 0x0800_0000 != 0,
            native::dpi(hwnd),
            native::dpi(taskbar),
        );
    });

    slint::Timer::single_shot(Duration::from_secs(12), || {
        let _ = slint::quit_event_loop();
    });
    slint::run_event_loop()
}
