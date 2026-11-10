# Taskbar pill and rings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A pill over the taskbar, left of the tray icons, showing `5h 37%` above `7d 61%` and two usage rings; the same two rings become the tray icon; a menu choice between the tray icon only and the tray icon with the pill.

**Architecture:** Core gains what the rings and pill show (`RingsModel`, `PillModel`, `TaskbarColours`) and where the pill goes (`TaskbarLayout.Anchors`, `PillPlacement.Place`, `FullScreen`), all tested. The app draws the rings in GDI+ for the icon and in WPF for the pill, and a `PillHost` keeps one borderless window owned by `Shell_TrayWnd` in place with a 2 s poll, WinEvent hooks and UI Automation on a background thread. `TrayController` feeds it on every refresh; `TrayMenu` gets a "Show" submenu backed by a new `show` config key.

**Tech Stack:** C# / .NET 10, WPF, WinForms `NotifyIcon`, GDI+, `System.Windows.Automation` (UIAutomationClient, part of the WindowsDesktop runtime), user32/shell32 P/Invoke, xUnit.

**Spec:** [docs/superpowers/specs/2026-11-09-taskbar-pill-design.md](../specs/2026-11-09-taskbar-pill-design.md). Spike code (throwaway, a working reference for Tasks 4 and 5): `C:\Users\DESKTOP\AppData\Local\Temp\claude\3243916385c19bd1\dad4337d-dc21-4a3b-828f-c69a5cd2a557\scratchpad\spike\PillSpike\Program.cs`.

## Global Constraints

- C# / .NET 10. Logic goes in `OpenHeadroom.Core` with an xUnit test; Core stays `net10.0` and `IsAotCompatible` (no WPF, WinForms or UI Automation in Core).
- `TreatWarningsAsErrors` is on: `dotnet build OpenHeadroom.slnx` ends with 0 warnings. Run tests with `dotnet test tests/OpenHeadroom.Tests`.
- No network calls, no new packages. The pill reads nothing but the taskbar's windows and UI Automation tree.
- The tray's UI thread never blocks: UI Automation runs on a thread-pool thread, one query at a time; nothing on the UI thread waits for it.
- Never stop or touch Ricardo's running tray, his `%LOCALAPPDATA%\OpenHeadroom\`, his `settings.json`, the HKCU `Run` key or `C:\Dev\openheadroom\publish\`. Manual runs use `OPENHEADROOM_DATA_DIR` and `CLAUDE_CONFIG_DIR` scratch folders.
- Do not edit `src/OpenHeadroom/App.xaml.cs`: PR #7 (diagnostics log) changes it.
- Night rules for labels: sentence case, plain words. Menu text exactly: `Show`, `Tray icon only`, `Tray icon and taskbar pill`.
- Colours: rings below 70% take the base colour, from 70% amber, from 90% red (`UsageLevels.For`). Track alpha `0x59`. Taskbar ink: `#FFFFFF` on a dark taskbar, `#1A1A1A` on a light one.
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Decisions this plan makes

- `TrayIconModel` is removed; `RingsModel` replaces it for the icon.
- The tray icon's colours come from `TaskbarColours` too, so the icon and the pill always agree.
- `AppConfig.Show` is an init property defaulting to `TrayAndPill`, not a new constructor parameter, so existing callers stay as they are. It is always written, like `style`.
- Anchors are offsets from the taskbar window's top-left corner, so a sliding taskbar moves the pill without asking UI Automation again.
- After a display or DPI change, or a new taskbar window, the anchors are dropped and the pill stays hidden until UI Automation answers (the spike saw `TrayNotifyWnd` lag there).
- Starting measures (Task 4 may tune them against the clock on Ricardo's taskbar): Inter 12 label, Unbounded Bold 11 figure, rings 22 DIP across with a 2.5 DIP stroke and a 1.5 DIP gap between rings, 8 DIP between text and rings, padding 8 × 4 DIP, gap to the tray 4 px at 96 DPI scaled by the taskbar's DPI.

## Review Focus

- **UI Automation fails or finds nothing** (Explorer busy, an element gone mid-walk, the taskbar auto-hidden): no exception escapes, the pill keeps its last good anchors, the tray keeps running. Pinned by `Anchors_without_tray_items_is_null` (Task 2) and Task 5 step 3.
- **A primary monitor not at the origin** (a second monitor to its left makes its coordinates start at 1920 or −2560): the pill still lands left of the tray. Pinned by `Places_on_a_monitor_not_at_the_origin` (Task 2).
- **Text that grows into the app buttons** (`5h 100%` with a long row of apps): the pill hides rather than overlaps. Pinned by `Hides_when_it_would_cover_the_app_buttons` (Task 2).
- **Explorer restarting while the pill shows:** exactly one pill comes back, owned by the new taskbar. Pinned by Task 7 step 4 (live, with Ricardo's OK).
- **Clicking or hovering the pill takes focus from the user's app:** it must not. Pinned by Task 7 step 3 (live).

---

### Task 1: Core: rings, pill text and taskbar colours

**Files:**
- Create: `src/OpenHeadroom.Core/Display/Rings.cs`, `src/OpenHeadroom.Core/Display/PillModel.cs`, `src/OpenHeadroom.Core/Colours/TaskbarColours.cs`
- Test: `tests/OpenHeadroom.Tests/RingsModelTests.cs`, `tests/OpenHeadroom.Tests/PillModelTests.cs`, `tests/OpenHeadroom.Tests/TaskbarColoursTests.cs`

**Interfaces:**
- Consumes: `Limit`, `LimitKind`, `Limit.DisplayPercent`, `UsageLevel`, `UsageLevels.For(int)`, `BarColour.Pick`, `NightPalette.For(bool dark)`, `AccentPalette`, `Rgb`.
- Produces:
  - `public readonly record struct Ring(int Percent, UsageLevel Level, bool HasData)` with `public static Ring None { get; }` (0, Normal, false) and `public double SweepDegrees` (0 without data, else `Percent * 360.0 / 100`).
  - `public sealed record RingsModel(Ring Outer, Ring Inner)` with `public static RingsModel From(IReadOnlyList<Limit> limits)`: Outer from the `Session` limit, Inner from `Weekly`, `Ring.None` where missing. Spend is not drawn.
  - `public sealed record PillLine(string Label, string Figure)` and `public sealed record PillModel(PillLine Session, PillLine Weekly, RingsModel Rings)` with `public static PillModel From(IReadOnlyList<Limit> limits)`. Labels `5h` and `7d`; figure `37%` (invariant culture) or `–` (U+2013) when the window is missing.
  - `public sealed record TaskbarColours(Rgb Ink, Rgb Base, Rgb Warning, Rgb Error)` with `public const byte TrackAlpha = 0x59;`, `public static TaskbarColours For(bool lightTaskbar, bool accentTaskbar, BarColourOption option, AccentPalette? accent)` and `public Rgb Fill(UsageLevel level)`.

- [ ] **Step 1: Write the failing tests**

```csharp
// RingsModelTests (L = new Limit("Claude Code", kind, kind.ToString(), pct, null, Now, false))
[Fact] public void No_limits_gives_two_empty_rings() =>
    Assert.Equal(new RingsModel(Ring.None, Ring.None), RingsModel.From([]));
[Fact] public void Outer_is_session_and_inner_is_weekly() =>
    Assert.Equal(new RingsModel(new Ring(37, UsageLevel.Normal, true), new Ring(61, UsageLevel.Normal, true)),
        RingsModel.From([L(LimitKind.Weekly, 61), L(LimitKind.Session, 37.9)]));
[Theory, InlineData(69, UsageLevel.Normal), InlineData(70, UsageLevel.Warning), InlineData(89, UsageLevel.Warning), InlineData(90, UsageLevel.Error)]
public void Each_ring_takes_its_own_level(int pct, UsageLevel level) {
    var m = RingsModel.From([L(LimitKind.Session, pct), L(LimitKind.Weekly, 10)]);
    Assert.Equal(level, m.Outer.Level); Assert.Equal(UsageLevel.Normal, m.Inner.Level); }
[Fact] public void Percent_is_clamped_to_100() => Assert.Equal(100, RingsModel.From([L(LimitKind.Session, 130)]).Outer.Percent);
[Fact] public void A_reset_window_is_zero() =>   // IsReset = true, Percent 0, as LimitList builds it
    Assert.Equal(new Ring(0, UsageLevel.Normal, true), RingsModel.From([new Limit("Claude Code", LimitKind.Session, "Session", 0, Now, Now, true)]).Outer);
[Fact] public void Spend_is_not_drawn() => Assert.Equal(new RingsModel(Ring.None, Ring.None), RingsModel.From([L(LimitKind.Spend, 50)]));
[Theory, InlineData(0, 0.0), InlineData(25, 90.0), InlineData(100, 360.0)]
public void Sweep_is_clockwise_degrees(int pct, double degrees) => Assert.Equal(degrees, new Ring(pct, UsageLevel.Normal, true).SweepDegrees);
[Fact] public void No_data_sweeps_nothing() => Assert.Equal(0, Ring.None.SweepDegrees);

// PillModelTests
[Fact] public void Both_windows() {
    var m = PillModel.From([L(LimitKind.Session, 37.9), L(LimitKind.Weekly, 61)]);
    Assert.Equal(new PillLine("5h", "37%"), m.Session); Assert.Equal(new PillLine("7d", "61%"), m.Weekly); }
[Fact] public void Missing_windows_read_a_dash() {
    var m = PillModel.From([]);
    Assert.Equal(new PillLine("5h", "–"), m.Session); Assert.Equal(new PillLine("7d", "–"), m.Weekly); }
[Fact] public void Full_reads_100() => Assert.Equal("100%", PillModel.From([L(LimitKind.Session, 100)]).Session.Figure);
[Fact] public void Rings_match_the_rings_model() {
    IReadOnlyList<Limit> limits = [L(LimitKind.Session, 74), L(LimitKind.Weekly, 93)];
    Assert.Equal(RingsModel.From(limits), PillModel.From(limits).Rings); }

// TaskbarColoursTests
[Fact] public void Dark_taskbar_purple() => Assert.Equal(
    new TaskbarColours(Rgb.Parse("#FFFFFF"), Rgb.Parse("#7b62ff"), Rgb.Parse("#ffb020"), Rgb.Parse("#ff6b6b")),
    TaskbarColours.For(lightTaskbar: false, accentTaskbar: false, BarColourOption.Purple, null));
[Fact] public void Light_taskbar_purple() => Assert.Equal(
    new TaskbarColours(Rgb.Parse("#1A1A1A"), Rgb.Parse("#5a3dd1"), Rgb.Parse("#a35f00"), Rgb.Parse("#c4262e")),
    TaskbarColours.For(lightTaskbar: true, accentTaskbar: false, BarColourOption.Purple, null));
[Fact] public void Accent_option_uses_the_bar_colour_pick() => Assert.Equal(
    BarColour.Pick(BarColourOption.Accent, AccentPalette.WindowsBlue, dark: true).Colour,
    TaskbarColours.For(false, false, BarColourOption.Accent, AccentPalette.WindowsBlue).Base);
[Theory, InlineData(false), InlineData(true)]
public void Accent_coloured_taskbar_uses_ink_as_base(bool light) {
    var c = TaskbarColours.For(light, accentTaskbar: true, BarColourOption.Accent, AccentPalette.WindowsBlue);
    Assert.Equal(c.Ink, c.Base); }
[Fact] public void Fill_follows_the_level() {
    var c = TaskbarColours.For(false, false, BarColourOption.Purple, null);
    Assert.Equal(c.Base, c.Fill(UsageLevel.Normal)); Assert.Equal(c.Warning, c.Fill(UsageLevel.Warning)); Assert.Equal(c.Error, c.Fill(UsageLevel.Error)); }
```

- [ ] **Step 2: Run `dotnet test tests/OpenHeadroom.Tests`** — expected: build fails, `RingsModel`, `PillModel`, `TaskbarColours` not defined.
- [ ] **Step 3: Implement the three files** with the signatures above. `Ring` uses `Limit.DisplayPercent` and `UsageLevels.For(DisplayPercent)`. `TaskbarColours.For`: Ink by `lightTaskbar`; Base = Ink when `accentTaskbar`, else `BarColour.Pick(option, accent, dark: !lightTaskbar).Colour`; Warning and Error from `NightPalette.For(dark: !lightTaskbar)`.
- [ ] **Step 4: Run the tests** — expected: all pass, 0 warnings.
- [ ] **Step 5: Commit** — `Core: rings, pill text and taskbar colours`.

### Task 2: Core: taskbar anchors, pill placement and full screen

**Files:**
- Create: `src/OpenHeadroom.Core/Taskbar/TaskbarLayout.cs`, `src/OpenHeadroom.Core/Taskbar/PillPlacement.cs`, `src/OpenHeadroom.Core/Taskbar/FullScreen.cs`
- Test: `tests/OpenHeadroom.Tests/TaskbarLayoutTests.cs`, `tests/OpenHeadroom.Tests/PillPlacementTests.cs`, `tests/OpenHeadroom.Tests/FullScreenTests.cs`

**Interfaces:**
- Consumes: `PixelRect` (`OpenHeadroom.Core.Display`).
- Produces (namespace `OpenHeadroom.Core.Taskbar`):
  - `public readonly record struct TaskbarAnchors(int TrayLeft, int ButtonsRight, int BandTop, int BandBottom)`: offsets in physical pixels from the taskbar window's left (`TrayLeft`, `ButtonsRight`) and top (`BandTop`, `BandBottom`).
  - `public static class TaskbarLayout { public const string TrayClassPrefix = "SystemTray."; public static TaskbarAnchors? Anchors(PixelRect taskbar, IEnumerable<(string ClassName, PixelRect Rect)> items, IEnumerable<PixelRect> buttons); }` — `items` are the children of the taskbar's `InputSite` pane, `buttons` the children of `Taskbar.TaskbarFrameAutomationPeer`. Tray items: class starts with the prefix (ordinal) and width and height > 0; none → null. `TrayLeft` = smallest left, `BandTop`/`BandBottom` = smallest top / largest bottom of the tray items; `ButtonsRight` = largest right of the non-empty buttons, or 0 (the taskbar's left) when there are none.
  - `public enum PillHidden { NoAnchors, NotAtBottom, NoRoom, OffScreen }`
  - `public readonly record struct PillPlace(int X, int Y, PillHidden? Hidden) { public bool IsShown => Hidden is null; }`
  - `public static class PillPlacement { public static PillPlace Place(PixelRect monitor, PixelRect taskbar, TaskbarAnchors? anchors, int width, int height, int gap); }`
  - `public static class FullScreen { public static bool IsFullScreen(int notificationState); }` — true for 2, 3 and 4 (`QUNS_BUSY`, `QUNS_RUNNING_D3D_FULL_SCREEN`, `QUNS_PRESENTATION_MODE`).

`Place`, in this order: null anchors → `NoAnchors`; a taskbar that is not wider than tall, or whose top is above the monitor's vertical middle → `NotAtBottom`; `right = taskbar.Left + TrayLeft − gap`, `x = right − width`; `x < taskbar.Left + ButtonsRight + gap` → `NoRoom`; `y = taskbar.Top + BandTop + (BandBottom − BandTop − height) / 2` (integer division); the rectangle `x, y, width, height` not wholly inside `monitor` → `OffScreen`; else shown at `(x, y)`.

- [ ] **Step 1: Write the failing tests** (values from the spike on Ricardo's 2560×1440 monitor)

```csharp
// TaskbarLayoutTests
static readonly PixelRect Bar = new(0, 1392, 2560, 1440);
[Fact] public void Anchors_from_the_spike() {
    var items = new[] { ("SystemTray.NormalButton", new PixelRect(2314, 1392, 2346, 1440)),
                        ("SystemTray.OmniButton", new PixelRect(2474, 1392, 2544, 1440)),
                        ("Taskbar.TaskbarFrameAutomationPeer", new PixelRect(0, 1392, 2560, 1440)) };
    var buttons = new[] { new PixelRect(972, 1392, 1017, 1440), new PixelRect(1545, 1392, 1589, 1440) };
    Assert.Equal(new TaskbarAnchors(2314, 1589, 0, 48), TaskbarLayout.Anchors(Bar, items, buttons)); }
[Fact] public void Anchors_without_tray_items_is_null() =>
    Assert.Null(TaskbarLayout.Anchors(Bar, [("Taskbar.TaskbarFrameAutomationPeer", Bar)], []));
[Fact] public void Empty_tray_items_are_skipped() =>
    Assert.Null(TaskbarLayout.Anchors(Bar, [("SystemTray.NormalButton", new PixelRect(2314, 1392, 2314, 1440))], []));
[Fact] public void No_buttons_means_the_taskbars_left_edge() =>
    Assert.Equal(0, TaskbarLayout.Anchors(Bar, [("SystemTray.OmniButton", new PixelRect(2474, 1392, 2544, 1440))], [])!.Value.ButtonsRight);
[Fact] public void Anchors_are_offsets_from_the_taskbar() =>
    Assert.Equal(new TaskbarAnchors(314, 89, 0, 48), TaskbarLayout.Anchors(new PixelRect(2000, 1392, 4560, 1440),
        [("SystemTray.NormalButton", new PixelRect(2314, 1392, 2346, 1440))], [new PixelRect(2050, 1392, 2089, 1440)]));

// PillPlacementTests
static readonly PixelRect Monitor = new(0, 0, 2560, 1440);
static readonly TaskbarAnchors At100 = new(2314, 1589, 0, 48);
[Fact] public void Places_left_of_the_tray_at_100_percent() =>
    Assert.Equal(new PillPlace(2222, 1397, null), PillPlacement.Place(Monitor, new(0, 1392, 2560, 1440), At100, 88, 38, 4));
[Fact] public void Places_left_of_the_tray_at_125_percent() =>
    Assert.Equal(new PillPlace(2138, 1386, null), PillPlacement.Place(Monitor, new(0, 1380, 2560, 1440), new TaskbarAnchors(2253, 1693, 0, 60), 110, 47, 5));
[Fact] public void Places_on_a_monitor_not_at_the_origin() =>
    Assert.Equal(new PillPlace(-338, 1397, null), PillPlacement.Place(new(-2560, 0, 0, 1440), new(-2560, 1392, 0, 1440), At100, 88, 38, 4));
[Fact] public void No_anchors_hides() => Assert.Equal(PillHidden.NoAnchors, PillPlacement.Place(Monitor, new(0, 1392, 2560, 1440), null, 88, 38, 4).Hidden);
[Theory, InlineData(1404), InlineData(1438)]   // sliding away, then auto-hidden
public void A_taskbar_partly_off_the_monitor_hides(int top) =>
    Assert.Equal(PillHidden.OffScreen, PillPlacement.Place(Monitor, new(0, top, 2560, top + 48), At100, 88, 38, 4).Hidden);
[Fact] public void Hides_when_it_would_cover_the_app_buttons() =>
    Assert.Equal(PillHidden.NoRoom, PillPlacement.Place(Monitor, new(0, 1392, 2560, 1440), At100 with { ButtonsRight = 2230 }, 88, 38, 4).Hidden);
[Fact] public void Just_enough_room_shows() =>   // x 2222 == buttons 2218 + gap 4
    Assert.True(PillPlacement.Place(Monitor, new(0, 1392, 2560, 1440), At100 with { ButtonsRight = 2218 }, 88, 38, 4).IsShown);
[Fact] public void A_taskbar_at_the_top_hides() =>
    Assert.Equal(PillHidden.NotAtBottom, PillPlacement.Place(Monitor, new(0, 0, 2560, 48), At100 with { BandTop = 0, BandBottom = 48 }, 88, 38, 4).Hidden);
[Fact] public void A_taskbar_on_the_side_hides() =>
    Assert.Equal(PillHidden.NotAtBottom, PillPlacement.Place(Monitor, new(2512, 0, 2560, 1440), At100, 88, 38, 4).Hidden);

// FullScreenTests
[Theory, InlineData(2, true), InlineData(3, true), InlineData(4, true), InlineData(1, false), InlineData(5, false), InlineData(7, false)]
public void Busy_d3d_and_presentation_are_full_screen(int state, bool full) => Assert.Equal(full, FullScreen.IsFullScreen(state));
```

- [ ] **Step 2: Run the tests** — expected: build fails, types not defined.
- [ ] **Step 3: Implement the three files** as specified above.
- [ ] **Step 4: Run the tests** — expected: all pass, 0 warnings.
- [ ] **Step 5: Commit** — `Core: taskbar anchors, pill placement and full screen`.

### Task 3: The tray icon becomes two rings

**Files:**
- Modify: `src/OpenHeadroom/IconRenderer.cs` (whole drawing), `src/OpenHeadroom/WindowsTheme.cs`, `src/OpenHeadroom/TrayController.cs:210-228` (`UpdateIcon`), `src/OpenHeadroom/Snapshot.cs:37` and `:232-297` (icon strip)
- Delete: `src/OpenHeadroom.Core/Display/TrayIconModel.cs`, `tests/OpenHeadroom.Tests/TrayIconModelTests.cs`

**Interfaces:**
- Consumes: `RingsModel`, `Ring`, `TaskbarColours` (Task 1); `TrayController.ConfigForDisplay()`.
- Produces: `IconRenderer.Render(RingsModel model, TaskbarColours colours, int size) -> Icon` and `IconRenderer.Draw(RingsModel model, TaskbarColours colours, int size) -> Bitmap`; `WindowsTheme.AccentTaskbar() -> bool` (HKCU `...\Themes\Personalize` `ColorPrevalence` is a non-zero int). The GDI+ ring drawing stays inside `IconRenderer`; the pill draws its own in WPF (Task 4).

- [ ] **Step 1: Delete `TrayIconModel` and its tests; change `IconRenderer`** to draw, on a transparent square of `size` px with anti-aliasing: each ring's track as a full circle in `Ink` at `TaskbarColours.TrackAlpha`, then, when the ring has data and `SweepDegrees > 0`, an arc from −90° (12 o'clock) sweeping `SweepDegrees` clockwise in `colours.Fill(ring.Level)`, round caps, a full circle at 360°. Starting geometry: stroke `max(2, size / 8)`, outer ring's centre-line radius `size / 2 − stroke / 2`, gap between the rings `max(1, size / 16)`, inner radius `outer − stroke − gap`. Both rings must stay distinct at 16 px.
- [ ] **Step 2: Wire `UpdateIcon`** to `RingsModel.From(limits)` and `TaskbarColours.For(WindowsTheme.IsLight(), WindowsTheme.AccentTaskbar(), config.BarColour, WindowsTheme.Accent())`, reading the config with `ConfigForDisplay()`. The tooltip is unchanged.
- [ ] **Step 3: Change the snapshot's icon strip** to readings `(Session, Weekly)`: `(none, none), (8, 44), (27, 74), (74, 27), (93, 61), (100, 100)`, at sizes 16, 20, 24 and 32, on both taskbars, purple base.
- [ ] **Step 4: Build, test and render** — `dotnet build OpenHeadroom.slnx` (0 warnings), `dotnet test tests/OpenHeadroom.Tests` (all pass), then `src\OpenHeadroom\bin\Debug\net10.0-windows\OpenHeadroom.exe --snapshot <scratch>\icons.png` exits 0. Look at the icon strip: both rings readable at 16 px, amber at 74, red at 93, closed rings at 100, tracks only with no data.
- [ ] **Step 5: Commit** — `Tray icon: two rings instead of the number and weekly bar`.

### Task 4: The pill window and its view

**Files:**
- Create: `src/OpenHeadroom/Pill/PillView.cs`, `src/OpenHeadroom/Pill/PillWindow.cs`
- Modify: `src/OpenHeadroom/Snapshot.cs` (a pill strip under the icon strips)

**Interfaces:**
- Consumes: `PillModel`, `PillLine`, `RingsModel`, `TaskbarColours` (Task 1). Fonts as in `FlyoutView` (`pack://application:,,,/` + `./Fonts/#Inter`, `./Fonts/#Unbounded`).
- Produces:
  - `public static class PillView { public static FrameworkElement Build(PillModel model, TaskbarColours colours); }` — the two right-aligned lines (label in Inter, figure in Unbounded Bold, both in `Ink`), then the rings, as the design's measures above. The rings are drawn the same way as the icon's (track, then a clockwise arc from 12 o'clock in `Fill(level)`, round caps, a full circle at 360°).
  - `internal sealed class PillWindow : Window` with `public PillWindow(IntPtr owner)` (sets `WindowInteropHelper.Owner` before the handle exists), `public IntPtr Handle`, `public event Action? LeftClicked`, `public event Action? RightClicked`, `public void Render(PillModel model, TaskbarColours colours, string tooltip)`, `public (int Width, int Height) PixelSize` (its DIP size at its own DPI, rounded up), `public void MoveTo(int x, int y)` (`SetWindowPos` `HWND_TOPMOST`, `SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW`), `public void HideNow()` (`ShowWindow` `SW_HIDE`).

- [ ] **Step 1: Write `PillView`.** The face is a `Border` with a 4 DIP corner radius whose background is `Ink` at alpha 1 (so clicks land on the window, not through it) and `Ink` at alpha `0x1A` while the mouse is over it.
- [ ] **Step 2: Write `PillWindow`:** `Title = "OpenHeadroom pill"`, `WindowStyle.None`, `AllowsTransparency`, transparent background, `ShowInTaskbar = false`, `ShowActivated = false`, `Topmost`, `Focusable = false`, `ResizeMode.NoResize`, `SizeToContent.WidthAndHeight`, manual start location off screen; in `OnSourceInitialized` add `WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE` to the extended style (as `FlyoutWindow` does for the tool-window bit). `Render` swaps the content and sets a `ToolTip` with the text. Mouse left and right button up raise `LeftClicked` and `RightClicked`.
- [ ] **Step 3: Add a pill strip to the snapshot:** `PillView.Build` for no data, 37/61, 74/61 and 93/100, on a dark (`#1C1C1C`) and a light (`#F0F0F0`) strip, purple base.
- [ ] **Step 4: Build, test and render** — 0 warnings, all tests pass, `--snapshot <scratch>\pill.png` exits 0; look at the pill strip: two lines right-aligned like the clock, rings to the right, amber and red rings at 74 and 93/100, dashes and tracks only with no data, readable on both strips.
- [ ] **Step 5: Commit** — `Pill: window and view`.

### Task 5: The taskbar tracker

**Files:**
- Create: `src/OpenHeadroom/Pill/PillHost.cs`, `src/OpenHeadroom/Pill/TaskbarProbe.cs`, `src/OpenHeadroom/Pill/TaskbarNative.cs`

**Interfaces:**
- Consumes: `PillWindow` (Task 4); `TaskbarLayout.Anchors`, `PillPlacement.Place`, `PillPlace`, `FullScreen.IsFullScreen` (Task 2); `PillModel`, `TaskbarColours` (Task 1).
- Produces:
  - `internal static class TaskbarProbe { public static TaskbarAnchors? Query(IntPtr taskbar); }` — UI Automation with a `CacheRequest` (class name and bounding rectangle, element and children): `AutomationElement.FromHandle(taskbar)`, its children, and each child's children feed `items`; the children of the `Taskbar.TaskbarFrameAutomationPeer` element feed `buttons`; then `TaskbarLayout.Anchors`. Any exception → null. The spike's `Query` is a working version.
  - `internal sealed class PillHost : IDisposable` with `public event Action? LeftClicked`, `public event Action? RightClicked`, `public void SetEnabled(bool enabled)`, `public void Update(PillModel model, TaskbarColours colours, string tooltip)`, `public void Dispose()`.

Behaviour of `PillHost` (the spike's `Host` does most of this; keep its findings, not its logging):
- Enabled: a 2 s `DispatcherTimer`; each tick finds `Shell_TrayWnd`. None → hide. A different handle from last time → drop the anchors, close the old `PillWindow`, hook `EVENT_OBJECT_LOCATIONCHANGE` on the new taskbar's process and thread (out of context), note its `TrayNotifyWnd`, make a new `PillWindow(taskbar)` and render the latest `Update` into it. Then ask UI Automation again and place.
- UI Automation: `Task.Run(() => TaskbarProbe.Query(taskbar))`, never two at once (ask again when the running one ends if asked meanwhile); back on the dispatcher, ignore the answer if the taskbar has changed since; else keep `answer ?? lastAnchors`.
- Place: if a full-screen app runs (`SHQueryUserNotificationState` → `FullScreen.IsFullScreen`) hide; else `PillPlacement.Place(monitor of the taskbar (MonitorFromWindow, nearest), taskbar window rect, anchors, window.PixelSize, gap = round(4 × GetDpiForWindow(taskbar) / 96))` → `MoveTo` or `HideNow`.
- Re-place on: the taskbar's location event (`idObject == 0`, hwnd == taskbar; with no anchors yet, also ask UI Automation), a location event on `TrayNotifyWnd` (ask UI Automation), `EVENT_SYSTEM_FOREGROUND` (skip own process), `SystemEvents.DisplaySettingsChanged` (drop the anchors, ask again; marshal to the dispatcher), the window's `DpiChanged`, each `Update` (the text width may change), and the registered `TaskbarCreated` message received by a hidden top-level `HwndSource` (not message-only: broadcasts skip those).
- Disabled, or disposed: stop the timer, unhook both WinEvent hooks, close the window, drop the anchors; `SetEnabled(true)` starts again. The WinEvent delegates are fields so they aren't collected.
- `PillWindow` click events are forwarded to `LeftClicked` / `RightClicked`.
- Every callback catches its own exceptions (`Debug.WriteLine($"OpenHeadroom: pill … failed: {ex.GetType().Name}")`): the tray must keep running.

- [ ] **Step 1: Write `TaskbarNative`** (the P/Invokes `PillHost` needs, `PillWindow` keeps its own), then `TaskbarProbe`.
- [ ] **Step 2: Write `PillHost`** as above.
- [ ] **Step 3: Check the failure paths by reading them:** `TaskbarProbe.Query` returns null on any exception; a null answer keeps the last anchors; a closed or destroyed window is never used again (`IsWindow` before `SetWindowPos`; a new window after a taskbar change); nothing on the UI thread waits for UI Automation.
- [ ] **Step 4: Build and test** — 0 warnings, all tests pass. (Nothing uses `PillHost` until Task 6.)
- [ ] **Step 5: Commit** — `Pill: taskbar tracker`.

### Task 6: The Show setting, the menu and the wiring

**Files:**
- Modify: `src/OpenHeadroom.Core/Config/AppConfig.cs`, `src/OpenHeadroom/TrayMenu.cs`, `src/OpenHeadroom/TrayController.cs`
- Test: `tests/OpenHeadroom.Tests/AppConfigTests.cs`

**Interfaces:**
- Consumes: `PillHost` (Task 5), `PillModel`, `TaskbarColours` (Task 1), `TooltipText.Render`.
- Produces: `public enum ShowOption { TrayAndPill, TrayOnly }` in `OpenHeadroom.Core.Config`; `AppConfig.Show { get; init; } = ShowOption.TrayAndPill`, key `show`, values `trayandpill` / `trayonly` (case-insensitive on load, unknown → `TrayAndPill`, written lower-case after `bar_colour`).

- [ ] **Step 1: Write the failing config tests**

```csharp
[Fact] public void Show_defaults_to_tray_and_pill() => Assert.Equal(ShowOption.TrayAndPill, AppConfig.Load(PathFor()).Show);
[Fact] public void Show_loads_and_unknown_values_fall_back() {
    Assert.Equal(ShowOption.TrayOnly, AppConfig.Load(Write("{\"show\":\"TrayOnly\"}")).Show);
    Assert.Equal(ShowOption.TrayAndPill, AppConfig.Load(Write("{\"show\":\"everywhere\"}")).Show); }
[Fact] public void Show_round_trips_and_is_written_lower_case() {
    (AppConfig.Default with { Show = ShowOption.TrayOnly }).Save(PathFor());
    Assert.Contains("\"show\": \"trayonly\"", File.ReadAllText(PathFor()));
    Assert.Equal(ShowOption.TrayOnly, AppConfig.Load(PathFor()).Show); }
```
Also add `Assert.Equal(ShowOption.TrayAndPill, c.Show);` to `AssertDefault`, and check `show` doesn't land in `Extra`.

- [ ] **Step 2: Run the tests** — expected: build fails, `ShowOption` not defined.
- [ ] **Step 3: Implement `ShowOption` and `AppConfig.Show`** (parsed with the existing `ParseEnum`).
- [ ] **Step 4: Run the tests** — all pass.
- [ ] **Step 5: Menu.** In `TrayMenu`, a `Show` submenu after `Bar colour` with `Tray icon only` and `Tray icon and taskbar pill`, checked from `ConfigForDisplay()` on opening like Style, each saving through `ChangeConfig(c => c with { Show = … })`.
- [ ] **Step 6: Wiring.** In `TrayController`: one `PillHost`, created in the constructor and disposed in `Dispose` before the flyout closes. `Refresh` reads the config once (`ConfigForDisplay()`) for the icon, the pill and the flyout. A new guarded part `pill`: `SetEnabled(config.Show == ShowOption.TrayAndPill)`, and when enabled `Update(PillModel.From(limits), TaskbarColours.For(WindowsTheme.IsLight(), WindowsTheme.AccentTaskbar(), config.BarColour, WindowsTheme.Accent()), TooltipText.Render(limits, now, zone, culture))`. `LeftClicked`: hide the flyout when visible, else `OpenFlyout(Cursor.Position)`. `RightClicked`: show `_menu.Strip` at `Cursor.Position` so that it closes on Esc and on a click elsewhere (bring the strip's own window to the foreground first, as `NotifyIcon` does; the click on the pill was this process's input, so Windows allows it).
- [ ] **Step 7: Build, test and smoke-check** — 0 warnings, all tests pass. Start the Debug build as a scratch instance (`OPENHEADROOM_DATA_DIR` and `CLAUDE_CONFIG_DIR` set to new scratch folders, a `usage.json` there with `five_hour` 37 and `seven_day` 61, both resetting in an hour) and note its PID. With a small PowerShell P/Invoke script, check that a visible window titled `OpenHeadroom pill` exists whose owner (`GetWindow(h, GW_OWNER)`) is `Shell_TrayWnd` and whose right edge is left of `TrayNotifyWnd`'s left edge. Write `{"show":"trayonly"}` to the scratch `config.json`, wait for the next 30 s refresh, and check the pill window is gone or hidden. Then stop the scratch instance **by its PID only**: Ricardo's own `OpenHeadroom.exe` is running and must not be touched.
- [ ] **Step 8: Commit** — `Pill: Show setting, menu and wiring`.

### Task 7: Live check, docs and version (controller)

- [ ] **Step 1:** Version `0.4.0` in `Directory.Build.props`.
- [ ] **Step 2:** In a scratch instance on Ricardo's taskbar: left-click opens the flyout above the pill and a second click closes it with no flicker; right-click opens the menu, which closes on Esc and on a click elsewhere; `Show › Tray icon only` hides the pill and `Tray icon and taskbar pill` brings it back; the tooltip shows on hover; 37/61 → 74/61 → 93/100 in `usage.json` turn the rings amber then red within a second; Quit removes the pill and the icon.
- [ ] **Step 3:** Focus: with a text editor focused, hover and click the pill; typing still goes to the editor (`GetForegroundWindow` unchanged).
- [ ] **Step 4:** Ask Ricardo before each: an Explorer restart (exactly one pill returns, owned by the new taskbar), and a full-screen app (the pill hides, then returns).
- [ ] **Step 5:** `docs/plan.md`: UI › tray icon and taskbar pill as built (rings, Show menu), milestone 3 results (spike and live check). `README.md`: how it works (the rings, the pill, the Show menu), status line, the pill's limits (primary monitor only; hides in full screen and when there is no room; undocumented taskbar layout).
- [ ] **Step 6:** Commit — `Docs: taskbar pill and rings; version 0.4.0`.

### Task 8: Brighter rings (after the live check)

Ricardo, 2026-11-09, on his taskbar: the purple rings didn't stand out (arc `#7B62FF` against a track of white at 35% over `#1C1C1C` ≈ `#6B6B6B`, about 1.3:1), and the two rings were hard to tell apart. He chose a brighter purple as the project's purple on dark grounds (flyout included), with a darker track and bigger rings.

**Files:**
- Modify: `src/OpenHeadroom.Core/Colours/NightPalette.cs` (`Dark.Purple`), `src/OpenHeadroom.Core/Colours/TaskbarColours.cs` (`TrackAlpha`), `src/OpenHeadroom/Pill/PillView.cs` (ring measures)
- Test: `tests/OpenHeadroom.Tests/BarColourTests.cs`, `tests/OpenHeadroom.Tests/TaskbarColoursTests.cs`

**Interfaces:** unchanged. `NightPalette.Dark.Purple` = `#A594FF`; `TaskbarColours.TrackAlpha` = `0x26` (15%); the light palette is unchanged (`#5A3DD1`).

- [ ] **Step 1: Write the failing tests:** in `BarColourTests` and `TaskbarColoursTests`, the dark purple expectations become `#A594FF`; add to `TaskbarColoursTests`:

```csharp
// The arc must stand out from its track on a dark taskbar (#1C1C1C): white at TrackAlpha over it.
[Fact] public void Dark_purple_reads_on_the_dark_track() {
    static byte Over(byte ink, byte ground) => (byte)Math.Round((ink * TaskbarColours.TrackAlpha + ground * (255 - TaskbarColours.TrackAlpha)) / 255.0);
    var track = new Rgb(Over(255, 0x1C), Over(255, 0x1C), Over(255, 0x1C));
    var c = TaskbarColours.For(lightTaskbar: false, accentTaskbar: false, BarColourOption.Purple, null);
    Assert.True(Contrast.Ratio(c.Base, track) >= 3.0); }
```
- [ ] **Step 2: Run the tests** — expected: the purple expectations and the new test fail.
- [ ] **Step 3: Change** `NightPalette.Dark.Purple` to `#a594ff`, `TrackAlpha` to `0x26`, and the pill's rings to 26 DIP across, a 3 DIP stroke and a 2 DIP gap between rings. The tray icon's geometry stays.
- [ ] **Step 4: Build, test and render** — 0 warnings, all tests pass, `--snapshot` exits 0; look at the flyout cards (dark), the icon strip and the pill strip: purple arcs stand out from their tracks, both rings distinct.
- [ ] **Step 5: Commit** — `Brighter purple and darker ring tracks`.

### Task 9: The tooltip sits above the taskbar (after the live check)

Found live: the pill's tooltip lost its last line a moment after appearing. Root cause, confirmed with a probe: WPF gives a popup an owner only when its parent is connected to the foreground window, and the pill never is, so the tooltip is an unowned topmost window. WPF places it on the mouse, so its lower part overlaps the taskbar band; the tracker's 2 s `SetWindowPos(HWND_TOPMOST)` on the pill raises the pill and its owner, the taskbar, above the tooltip, and the taskbar covers that part. Fix: place the tooltip wholly above the band, so nothing can cover it whatever the z-order.

**Files:**
- Modify: `src/OpenHeadroom.Core/Taskbar/PillPlacement.cs`, `src/OpenHeadroom/Pill/PillWindow.cs`, `src/OpenHeadroom/Pill/PillHost.cs`
- Test: `tests/OpenHeadroom.Tests/PillPlacementTests.cs`

**Interfaces:**
- `public readonly record struct PillPlace(int X, int Y, PillHidden? Hidden, int BandTop = 0)`: `BandTop` is the taskbar band's top edge in screen pixels (`taskbar.Top + anchors.BandTop`) when shown, 0 when hidden.
- `PillWindow.MoveTo(int x, int y, int bandTop)` replaces `MoveTo(int x, int y)`.

- [ ] **Step 1: Update the failing tests:** the three shown cases in `PillPlacementTests` expect `BandTop` 1392, 1380 and 1392: `new PillPlace(2222, 1397, null, 1392)`, `new PillPlace(2138, 1386, null, 1380)`, `new PillPlace(-338, 1397, null, 1392)`.
- [ ] **Step 2: Run the tests** — expected: those three fail.
- [ ] **Step 3: Implement:** `Place` returns the band top with a shown place. `PillWindow.MoveTo(x, y, bandTop)` moves as before and sets the face's tooltip to open above the pill: `ToolTipService.Placement = PlacementMode.Top`, `PlacementTarget` = the face, `VerticalOffset` = −((y − bandTop) in DIPs at the window's DPI + 8), so its bottom edge sits 8 DIP above the band. Keep the last values so a face built by a later `Render` gets the same tooltip settings. `PillHost` passes `place.BandTop`.
- [ ] **Step 4: Build and test** — 0 warnings, all tests pass.
- [ ] **Step 5: Commit** — `Pill: tooltip above the taskbar`.

### Task 10: A stronger track for the tray icon (after the second live check)

Ricardo, 2026-11-09: the pill's rings now read well, but in the 16 px tray icon the 15% track vanished and only the arcs showed. Compared at 16 px on `#1C1C1C`: 15% nearly invisible, 25% shows the whole circle with the arc still clearly brighter, 35% heavier. The icon takes 25%; the pill keeps 15%.

**Files:**
- Modify: `src/OpenHeadroom.Core/Colours/TaskbarColours.cs`, `src/OpenHeadroom/IconRenderer.cs`
- Test: `tests/OpenHeadroom.Tests/TaskbarColoursTests.cs`

**Interfaces:** `TaskbarColours.IconTrackAlpha = 0x40` (25%, the tray icon's track); `TrackAlpha` (`0x26`) stays for the pill.

- [ ] **Step 1: Write the failing tests** in `TaskbarColoursTests`, compositing white at the alpha over `#1C1C1C` as `Dark_purple_reads_on_the_dark_track` does: `Icon_track_shows_on_the_dark_taskbar` (the composite against `#1C1C1C` is at least 2.0:1) and `Dark_purple_reads_on_the_icon_track` (the dark purple base against the composite is at least 2.5:1).
- [ ] **Step 2: Run the tests** — expected: they fail to build (`IconTrackAlpha` not defined).
- [ ] **Step 3: Add `IconTrackAlpha`** and use it for the icon's tracks in `IconRenderer`.
- [ ] **Step 4: Build and test** — 0 warnings, all tests pass.
- [ ] **Step 5: Commit** — `Tray icon: a stronger ring track`.

### Task 11: A bolder tray icon (after the third live check)

Ricardo, 2026-11-09: with a 25% track the 16 px icon was still very subtle. From five variants rendered at 16 px on his taskbar colour he chose C: 2.5 px rings at 16 px and a 35% track. The tooltip fix (Task 9) is confirmed live.

**Files:**
- Modify: `src/OpenHeadroom.Core/Colours/TaskbarColours.cs`, `src/OpenHeadroom/IconRenderer.cs`
- Test: `tests/OpenHeadroom.Tests/TaskbarColoursTests.cs`

**Interfaces:** `TaskbarColours.IconTrackAlpha = 0x59` (35%). Icon geometry: stroke `size * 5 / 32` (2.5 at 16, 3.125 at 20, 3.75 at 24, 5 at 32), gap `max(1, size / 16)`, outer centre-line radius `size / 2 − stroke / 2`, inner `outer − stroke − gap`.

- [ ] **Step 1: Update the tests:** `Dark_purple_reads_on_the_icon_track` now asserts at least 2.0:1 (Ricardo chose the look by eye: about 2.1:1); `Icon_track_shows_on_the_dark_taskbar` stays at least 2.0:1.
- [ ] **Step 2: Change** `IconTrackAlpha` and the icon's stroke as above.
- [ ] **Step 3: Build, test and render** — 0 warnings, all tests pass, `--snapshot` exits 0; the icon strip shows both rings distinct at 16 px.
- [ ] **Step 4: Commit** — `Tray icon: bolder rings`.
