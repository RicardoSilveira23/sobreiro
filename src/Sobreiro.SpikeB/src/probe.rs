//! SPIKE step 1, before any drawing: does the taskbar take a toolkit window? Borderless, transparent per
//! pixel, never taking focus, owned by the taskbar, placed inside it. Writes what it finds to the trace
//! file and leaves after 12 seconds (`SOBREIRO_SPIKE_PROBE_SECONDS`). `SOBREIRO_SPIKE_BORN` picks how the
//! window comes to be owned and left inactive; `SOBREIRO_SPIKE_PROBE_OWNER=own` gives it an owner made
//! here, for a PC without a taskbar.
use crate::{Probe, native, shell, spike, trace};
use slint::ComponentHandle;
use std::time::Duration;

pub fn run() -> Result<(), slint::PlatformError> {
    let born = shell::Born::chosen();
    let stand_in = spike::env("SOBREIRO_SPIKE_PROBE_OWNER").as_deref() == Some("own");
    let taskbar = if stand_in { native::plain_window() } else { native::find_taskbar() };
    let seconds = spike::env("SOBREIRO_SPIKE_PROBE_SECONDS").and_then(|s| s.parse::<u64>().ok()).unwrap_or(12);
    #[cfg(feature = "toolkit-hook")]
    if born == shell::Born::Hook {
        shell::next_window_owned_by(taskbar);
    }
    let ui = Probe::new()?;
    ui.window().set_position(slint::PhysicalPosition::new(-32000, -32000));

    // Shown from inside the event loop: the toolkit makes the window during the call.
    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::ZERO, move || {
        let Some(ui) = weak.upgrade() else { return };
        let foreground_before = native::foreground();
        let want = shell::Want { owner: taskbar, ex_style: native::WS_EX_OURS, inactive: true };
        shell::show_born(born, want, || ui.show());

        let weak = ui.as_weak();
        slint::Timer::single_shot(Duration::from_millis(300), move || {
            let Some(ui) = weak.upgrade() else { return };
            let hwnd = shell::hwnd_of(ui.window());
            trace!(
                "probe born={born:?} render={} taskbar={:?} hwnd={:?} birth={:?} exstyle-after-show=0x{:x} \
                 owner-is-taskbar-after-show={} active-after-show={} foreground-unchanged-after-show={}",
                if spike::gpu() { "gpu" } else { "software" },
                taskbar.0,
                hwnd.0,
                shell::birth_of(hwnd),
                native::ex_style(hwnd),
                native::owner(hwnd) == taskbar,
                native::active() == hwnd,
                native::foreground() == foreground_before,
            );
            if born.owned_afterwards() {
                native::set_owner(hwnd, taskbar);
                native::add_ex_style(hwnd, native::WS_EX_OURS);
            }
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

        // The toolkit writes the window's whole extended style when one of its own settings changes.
        if spike::env("SOBREIRO_SPIKE_PROBE_RESTYLE").is_some() {
            for (after, on_top) in [(1200, false), (1600, true)] {
                let weak = ui.as_weak();
                slint::Timer::single_shot(Duration::from_millis(after), move || {
                    let Some(ui) = weak.upgrade() else { return };
                    let hwnd = shell::hwnd_of(ui.window());
                    ui.set_on_top(on_top);
                    trace!("on-top={on_top} asked; before it: exstyle=0x{:x} birth={:?}", native::ex_style(hwnd), shell::birth_of(hwnd));
                });
            }
        }

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
                 active={} foreground-unchanged={} birth={:?} style=0x{:x} exstyle=0x{ex:x} toolwindow={} noactivate={} \
                 window-dpi={} taskbar-dpi={} toolkit-scale={scale}",
                native::owner(hwnd) == taskbar,
                native::is_visible(hwnd),
                own.left,
                own.top,
                own.right,
                own.bottom,
                own.width(),
                own.height(),
                native::is_above(hwnd, taskbar),
                native::active() == hwnd,
                native::foreground() == foreground_before,
                shell::birth_of(hwnd),
                native::style(hwnd),
                ex & 0x80 != 0,
                ex & 0x0800_0000 != 0,
                native::dpi(hwnd),
                native::dpi(taskbar),
            );
        });
    });

    slint::Timer::single_shot(Duration::from_secs(seconds), || {
        let _ = slint::quit_event_loop();
    });
    slint::run_event_loop()
}
