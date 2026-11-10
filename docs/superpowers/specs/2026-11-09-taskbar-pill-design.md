# Design: the taskbar pill and the rings (milestone 3)

Written 2026-11-09 with Ricardo, after the spike. Sits on [plan.md](../../plan.md) (UI ›
"Taskbar pill", milestone 3) and [references.md](../../references.md) (now-playing widget).

## Intent

The tray icon tells you the session % only if you look for it, and Windows 11 tucks new tray
icons into the `^` overflow. The pill puts both limits on the taskbar itself, next to the
clock, where they are seen without a click. People who don't want it keep a tray-only app.
The look is one idea in both places: two rings, like the Claude app's usage circle and the
Apple Watch rings, which people already read at a glance.

## Goals

- A pill on the primary monitor's taskbar, just left of the tray icons: `5h 37%` above
  `7d 61%`, two lines like the clock, in Night type, then two rings.
- The same two rings replace the tray icon's number and weekly bar, always.
- A menu choice between "tray icon only" and "tray icon and taskbar pill"; the pill is on by
  default (Ricardo, 2026-11-09).
- It stays put through auto-hide, a scaling change and an Explorer restart, never covers the
  tray icons or the app buttons, and hides while a full-screen app runs.

## Non-goals

- Other monitors' taskbars (`Shell_SecondaryTrayWnd`). Primary only, as the plan says.
- A taskbar at the top or the sides (Windows 11 has none; the pill hides if it finds one).
- Keyboard access to the pill: the tray icon and its menu stay the keyboard path.
- Any new data: the pill shows what the tray icon and flyout already know.

## Anti-goals

- Stealing focus, showing in Alt+Tab, or taking a taskbar button.
- Covering a tray icon, an app button or the clock, even for a second after a change.
- Drawing over a full-screen game or video.
- Freezing the taskbar: the pill's window is owned by Explorer's taskbar across processes,
  which attaches the two threads' input queues, so the tray's UI thread must never block (no UI
  Automation, file or network work on it).
- Breaking the tray when the taskbar's layout changes in a Windows update: the pill hides, the
  tray icon carries on.

## What the spike settled

Spike on Ricardo's machine (Windows 11 25H2, 2560×1440, one monitor), each check with his OK.
Full notes: `side-session-notes/local_a45cec68…/openheadroom-m3-spike.md`.

- A borderless WPF window with `WindowInteropHelper.Owner = Shell_TrayWnd` (set before Show)
  and `WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`, topmost, sits over the taskbar and never activates.
- UI Automation finds the anchors in 4–30 ms (up to 300 ms just after a restart or DPI
  change), with a cache request, on a background thread. Under `Shell_TrayWnd`'s element, the
  first level holds an `InputSite` pane, and its children are:
  - the tray items, class names starting `SystemTray.` (chevron, icons, network, volume,
    clock, Show desktop): their smallest left edge is where the pill must end, and their top
    and bottom are the drawn band;
  - `Taskbar.TaskbarFrameAutomationPeer`, whose children are Start, Task View and the app
    buttons: their largest right edge is where the pill may start.
- **Auto-hide:** the taskbar window slides down until 2 px show. The pill hides as soon as its
  place leaves the monitor and comes back when the taskbar is fully up. While the taskbar is
  hidden, UI Automation returns no items: keep the last good anchors.
- **Scaling 100% → 125%:** the taskbar grows to 60 px at 120 DPI; the pill re-measured and
  re-centred within 0.7 s. `TrayNotifyWnd`'s rectangle lagged during the change and gave a wrong
  band, so it is not used as a fallback: no UI Automation answer, no pill.
- **Explorer restart:** the pill's window survived its owner's death. `TaskbarCreated` arrived
  when the new taskbar was up; making a new pill window owned by the new `Shell_TrayWnd` placed
  it 0.4 s later.
- The app buttons' right edge moves as apps open and close, with no window event, so UI
  Automation is polled.
- A maximised window reaches 8 px past the monitor, so "a window covers the monitor" misfires.
  `SHQueryUserNotificationState` answers 2 (busy), 3 (Direct3D full screen) or 4 (presentation
  mode) when a full-screen app runs.

## Approaches

1. **A window owned by the taskbar** (chosen, the plan's own and the now-playing widget's).
   Windows keeps it above the taskbar without z-order fights; the spike passed all three checks.
   Cost: undocumented layout, polled, and the cross-process input queue attachment above.
2. **A child window parented into `Shell_TrayWnd`.** Explorer's XAML taskbar redraws over it,
   DPI and z-order go wrong, and a hung tray hangs the taskbar outright. Rejected in the plan.
3. **A plain topmost window, not owned.** It fights the taskbar for the top spot and loses
   whenever the taskbar re-asserts itself. Rejected.

Deskbands are gone from Windows 11.

For the look, Ricardo chose two rings and no flame (option C of three mockups: a ring around a
flame, one ring, two rings).

## The design, as decided

### The rings (pill and tray icon)

- Outer ring: the session (5h). Inner ring: the weekly (7d). Each fills clockwise from 12
  o'clock to its own **% used** (`Limit.DisplayPercent`: floored, clamped 0–100), whatever the
  flyout's Style; Battery and Thermometer showing % left does not flip the rings. A window that
  has reset counts as 0%. At 100% the ring is closed.
- Each ring takes its own colour: the base colour below 70%, amber from 70%, red from 90%. The
  base colour follows the Bar colour setting: OpenHeadroom purple, or the Windows accent where it
  passes the existing contrast check, which stays the flyout's check for the taskbar's theme (my
  call: the dark taskbar and the flyout's ground are both near-black, the light ones near-white).
  When the taskbar itself shows the accent colour (`ColorPrevalence` = 1), the base colour is the
  taskbar's text colour instead, since accent or purple can vanish on it (my call). Amber and red
  are the Night palette's for the taskbar's theme.
- The unfilled part of each ring is a faint track in the taskbar's text colour, so it reads on
  a dark or a light taskbar. A window with no data draws its track only; with no data at all both
  rings are tracks, which replaces today's grey `–`.
- The tray icon keeps its tooltip (`Session 8% · resets 2:20 PM` …), which still carries the
  reading's age. Its number, `!` and the weekly bar go: used up is a closed red ring, and "not
  connected" or "no reading yet" is no data. Old readings get no ring treatment, as the icon has
  none today (my call).

### The pill

- Text: `5h 37%` above `7d 61%`, right-aligned like the clock: the label in Inter, the figure
  in Unbounded. A window with no data reads `5h –`. Text colour follows the taskbar's light or
  dark theme (`SystemUsesLightTheme`).
- The two rings sit to the right of the text, vertically centred in the taskbar band.
- Left-click opens the flyout above the pill, or closes it when it is open. The pill never
  activates, so a click on it doesn't deactivate the flyout: the click handler finds the flyout
  still visible and hides it, with no reopen flicker (my call; checked in the running app).
  Right-click opens the tray icon's menu. Hovering shows the tray icon's tooltip and a faint
  rounded highlight, like a taskbar button.
- Its width follows its text (`5h 9%` is narrower than `5h 100%`), measured on each redraw; the
  right edge stays put and the left edge moves, like the clock's (my call).
- Placement: its right edge a small gap (4 px at 100%, scaled) left of the tray items, centred
  in the band. It hides when:
  - "tray icon only" is chosen;
  - a full-screen app runs (`SHQueryUserNotificationState` 2, 3 or 4);
  - its place is not wholly on the monitor (auto-hide, or the taskbar sliding);
  - it would overlap the app buttons (left edge less than a gap right of them);
  - the taskbar isn't found, isn't along the monitor's bottom, or UI Automation has given no
    anchors yet (just after start or an Explorer restart).
- It re-places itself on: the taskbar window moving (a location WinEvent on Explorer's taskbar
  thread), the foreground window changing (full screen), display and DPI changes,
  `TaskbarCreated`, and a 2 s poll that also asks UI Automation again.
- Each placing checks the z-order too. Explorer raises the taskbar when a full-screen window
  closes (the Snipping Tool overlay, a full-screen photo viewer), and a pill hidden for it would
  come back under the taskbar: a pill that is hidden or below the taskbar goes back to the top of
  the topmost band, without moving the taskbar (`SWP_NOOWNERZORDER`).
- When the taskbar window changes (Explorer restarted), it makes a new pill window owned by the
  new taskbar and closes the old one.

### The menu and the setting

- A "Show" submenu in the right-click menu with two choices, checked like Style:
  "Tray icon only" and "Tray icon and taskbar pill". The tray icon always stays.
- Saved in `config.json` as `show`, `"trayandpill"` (the default, also for a missing or unknown
  value) or `"trayonly"`, through the existing reload-change-save path.

### Where the code goes

- Core, with xUnit tests: the rings model (fraction and level per ring, data or not); the pill
  text; the anchors from UI Automation items (class name and rectangle in, tray left, app
  buttons right and band out); the placement geometry (monitor, taskbar, anchors, pill size,
  gap in, a position or a reason to hide out); which notification states mean full screen; the
  `show` config key.
- App: the pill window, its view, the taskbar tracker (find, hooks, poll, UI Automation on a
  background thread, recreate), the ring drawing in WPF (pill) and GDI+ (icon), the menu
  submenu, the wiring in `TrayController`.
- `--snapshot` draws the ring icon at every size and level (both taskbars) and the pill in
  both themes at no data, 37/61, 74/61 and 93/100.

### Release

- Version 0.4.0. `plan.md` (UI: tray icon and pill; milestone 3 results) and the README (how it
  works, menu, the pill and its limits) are updated.

## Changed after the live check (2026-11-09)

- The purple didn't stand out on the taskbar: its arc and the 35% track were about 1.3:1 apart.
  Ricardo chose `#A594FF` as the project's purple on dark grounds (flyout included; the light
  palette keeps `#5A3DD1`). The pill's tracks are the ink at 15%, its rings 26 DIP across with a
  3 DIP stroke and a 2 DIP gap. The tray icon was still too faint, so from rendered variants he
  chose 2.5 px rings at 16 px (scaled up with the icon) and a 35% track.
- The tooltip lost its last line: WPF makes a popup unowned unless its parent is connected to
  the foreground window, which the pill never is, so the taskbar rose above the part of the
  tooltip that overlapped it. The tooltip now opens 8 DIP above the taskbar band.

## Left to the builder

- Exact DIP measures of the pill (font sizes, ring diameter and stroke, gaps, padding), within
  Night and the spike's proportions (Inter 12, Unbounded 11 bold, a ring about 20 DIP across),
  checked against the clock on Ricardo's taskbar.
- Ring stroke, gap and caps per icon size (16, 20, 24, 32 px), as long as both rings stay
  distinct at 16 px.
- Track alpha, and the hover highlight's colour per taskbar theme.
- How the right-click menu is shown from the pill so it closes when clicking elsewhere.
- The tracker's class split, and how it reports failures (the diagnostics log from PR #7 may
  land first; until then `Debug.WriteLine` like the rest of the app).
