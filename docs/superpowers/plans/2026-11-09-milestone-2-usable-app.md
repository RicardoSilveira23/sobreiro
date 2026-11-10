# Milestone 2 (Usable app) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the milestone 1 prototype into an app Ricardo can live with: a Night flyout on left click with three styles and a bar colour option, colour levels, a pace line, Connect/Disconnect, Start with Windows, toasts at 80% and 95%, and the milestone 1 review backlog.

**Architecture:** Every decision stays in `OpenHeadroom.Core` with an xUnit test: merging readings, pace, the text the flyout shows, colour contrast, placement geometry, config and alert state, and the `settings.json` edit. The tray app (`src/OpenHeadroom`) only reads Windows state (theme, accent, registry, monitors), draws what Core hands it, and wires the menu. The bridge keeps its one-way flow, now merging into `usage.json` instead of overwriting it.

**Tech Stack:** C# / .NET 10 (SDK 10.0.403), xUnit, System.Text.Json (`JsonDocument`, `Utf8JsonReader`, `Utf8JsonWriter` only; Core stays AOT-safe), WPF for the flyout (built in code, no XAML beyond `App.xaml`), WinForms `NotifyIcon` and `ContextMenuStrip`, System.Drawing for the icon. Inter and Unbounded TTFs embedded as WPF resources.

**Spec:** [docs/plan.md](../../plan.md) (UI, Connecting to Claude Code, Security rules, milestone 2), the confirmed mockup (measures copied below), Ricardo's Night guide (the design system's `language.md`), and the milestone 1 handoff backlog (listed under Task 1, 2, 7).

## Global Constraints

- C# / .NET 10. Logic goes in `OpenHeadroom.Core` and gets an xUnit test. Core targets `net10.0` with `<IsAotCompatible>true</IsAotCompatible>` and uses no reflection-based JSON.
- Nullable enabled, warnings as errors (`Directory.Build.props`). The build must end with 0 warnings.
- Data comes only from the status line JSON via the bridge. No network calls at runtime.
- Never read `~/.claude/.credentials.json`. Never log, print, cache or commit any token.
- The bridge writes only `rate_limits` fields and timestamps to `usage.json`. Never the rest of stdin.
- Edit the user's `settings.json` only through Connect/Disconnect, with a backup, and only the `statusLine` key.
- **Ricardo's live setup must keep working.** Never touch his real `%USERPROFILE%\.claude\settings.json`, his real `%LOCALAPPDATA%\OpenHeadroom\`, his HKCU `Run` key, or `C:\Dev\openheadroom\publish\`. Tests use temp folders. Manual runs set `OPENHEADROOM_DATA_DIR` and `CLAUDE_CONFIG_DIR` to scratch folders.
- Product text says "Claude Code" only in plain words; no Anthropic or Claude logo.
- Night rules for every label: sentence case, no tracking, no capitals, no full stops on headings, no shadows, no gradients.
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Run tests with `dotnet test` from the repo root; build with `dotnet build` (0 warnings).

## Decisions this plan makes (not in the spec)

- **Merge rule.** Within one reset period, used % only rises, so a reading replaces the stored window when it belongs to a later period, or to the same period with a used % at least as high. Two `resets_at` values within one hour of each other are the same period (periods differ by at least the window length). A stored window the new reading lacks is kept until its `resets_at` passes. Each window in `usage.json` gets its own `observed_at`; the top-level `observed_at` is the time of the write. Files from milestone 1 (no per-window time) still read, using the top-level time.
- **The bridge merges.** It reads `usage.json`, merges, and writes. Two bridges racing can lose one write; the next refresh repairs it.
- **Chain timeout** 5 s, then the whole process tree is killed and the default line printed. PowerShell chains get `[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); ` prefixed.
- **Pace** is computed from the time of the reading (`observed_at`), not from now: rate = used ÷ time elapsed in the window at that moment. No pace line when less than 5% of the window had passed, at 0 elapsed, at or above 100% used, after a reset, or without a reset time or window length. Window length: session 5 h, weekly 7 d, spend by its `period` (daily 1 d, weekly 7 d, monthly the calendar month before `resets_at`).
- **Durations** floor and drop a zero second part: `3d 13h`, `2d`, `3h 11m`, `2h`, `45m`, `less than a minute`.
- **Day names.** A time within 6 local days of now shows `Fri 12:00 AM`; further away (a weekly reset exactly 7 days out) shows `Mon 16 Nov 12:00 AM`.
- **The flyout follows the system (taskbar) theme**, `SystemUsesLightTheme`, like Windows' own tray flyouts and the tray icon. The accent comes from `HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Accent\AccentPalette` (8 RGBA quads: Light3, Light2, Light1, Accent, Dark1, Dark2, Dark3, unused).
- **Night colours for the flyout** (from the mockup): dark ground `#0b0b0b`, ink `#f4f4f4`, soft `#8d8d8d`, line and track `#262626`, amber `#ffb020`, red `#ff6b6b`, purple `#7b62ff`; light ground `#ffffff`, ink `#0d0d0d`, soft `#767676`, line and track `#e4e4e4`, amber `#a35f00`, red `#c4262e`, purple `#5a3dd1`.
- **Spend limit** shows as a bars-style row below session and weekly in every style, with `$31.50 of $50.00 this month`.
- **Toasts** are `NotifyIcon.ShowBalloonTip`, which Windows 10/11 show as notifications. No WinRT and no package. One toast per refresh at most (the highest threshold). Alert state lives in its own `alerts.json`, so `config.json`, which the bridge reads on every call, only changes when the user changes a setting.
- **`config.json` keys:** `chained_command` (string), `previous_status_line` (the exact `statusLine` value Connect replaced, or JSON `null` when there was none; absent when the bridge was connected by hand), `style` (`bars`, `battery`, `thermometer`), `bar_colour` (`purple`, `accent`). Unknown keys are kept. The tray reloads the file before each change.
- **Connect** writes `"\"C:/path/openheadroom-bridge.exe\""` (forward slashes, quoted, for Git Bash), keeps any other properties of the old `statusLine` object (`padding` and the like), finds the bridge next to `OpenHeadroom.exe` or in `..\bridge\`, and confirms with a message box that shows the file, the `statusLine` now and after, and the backup name `settings.json.openheadroom-yyyyMMdd-HHmmss.bak`. Disconnect restores `previous_status_line` exactly; with none saved (a manual setup like Ricardo's), it restores the chained command if there is one, else removes `statusLine`.
- **One tray per data folder.** The single-instance mutex is `Local\OpenHeadroom.Tray` as before, with a suffix from the data folder when `OPENHEADROOM_DATA_DIR` is set, so a test instance runs beside Ricardo's. Starting `OpenHeadroom.exe` while it runs opens the flyout of the running one.
- **`OpenHeadroom.exe --snapshot <file.png>`** renders a contact sheet (each style in both themes and both colours from a sample reading, plus the tray icon at each level and size) and exits. It is how the looks are checked without changing Windows' theme, and it gives milestone 4 its screenshots.
- WPF has no letter spacing, so the Unbounded figures drop the mockup's `.06em` tracking.
- Version `0.2.0` in `Directory.Build.props`.

## Review Focus

- An idle session writes a lower % for the same period after a busy one wrote a higher % → the higher number stays (Task 1 test `Merge_keeps_higher_reading_in_same_period`).
- `settings.json` with comments, a BOM, non-ASCII text, or keys after `statusLine` → only the `statusLine` span changes; invalid JSON → `FormatException` and nothing is written (Task 5 tests).
- `statusLine` already runs an `openheadroom-bridge` (Ricardo's manual setup) → Connect never saves it as `chained_command`; Disconnect with nothing saved removes `statusLine` (Task 5 tests).
- `settings.json` edited between the confirmation and the write → the write is refused (Task 5 test `Apply_refuses_when_settings_changed_since_the_plan`).
- A window that just started (under 5% elapsed), or 100% used → no pace line rather than a wild projection; 0% used → "On track · about 100% left at the reset" (Task 2 tests, `Zero_percent_projects_zero`).

## File Structure

```text
Directory.Build.props                          + <Version>0.2.0</Version>
src/OpenHeadroom.Core/
  AppPaths.cs                                  + AlertsFile(), InstanceId()
  IO/AtomicFile.cs                             temp file + rename, retries (UsageFile uses it)
  Usage/RateLimits.cs                          + per-window ObservedAt
  Usage/RateLimitsJson.cs                      + per-window observed_at read/write
  Usage/ResetPeriod.cs                         same-period test (1 h tolerance)
  Usage/UsageMerge.cs                          merge a reading into the stored snapshot
  Bridge/BridgeRunner.cs                       merges before writing
  Bridge/ChainRunner.cs                        timeout, process-tree kill, UTF-8 PowerShell
  Limits/Limit.cs                              + Window, UsedUsd, LimitUsd, Period; per-window ObservedAt
  Limits/UsageLevel.cs                         Normal / Warning (70) / Error (90)
  Display/When.cs                              day name only within 6 days
  Display/Duration.cs                          "3h 11m"
  Display/Pace.cs                              projection to the reset
  Display/FlyoutStyle.cs                       Bars / Battery / Thermometer
  Display/FlyoutModel.cs                       everything the flyout prints
  Display/FlyoutPlacement.cs                   where the flyout goes, in pixels
  Display/TrayIconModel.cs                     + Level
  Colours/Rgb.cs, Contrast.cs, NightPalette.cs, AccentPalette.cs, BarColour.cs
  Config/AppConfig.cs                          config.json read/write, unknown keys kept
  Alerts/AlertPolicy.cs, AlertState.cs         80% / 95% once per period
  Connect/ClaudePaths.cs                       CLAUDE_CONFIG_DIR or %USERPROFILE%\.claude
  Connect/SettingsJson.cs                      read and splice the statusLine value
  Connect/BridgeCommand.cs                     bridge command text, IsBridge
  Connect/Connection.cs                        plan and apply Connect / Disconnect
  Connect/BridgeLocator.cs                     find openheadroom-bridge.exe
  Startup/AutoStartCommand.cs                  Run value text and match
src/OpenHeadroom/
  Fonts/Inter-Regular.ttf, Inter-Medium.ttf, Unbounded-Bold.ttf, OFL-Inter.txt, OFL-Unbounded.txt
  Flyout/FlyoutColours.cs                      palette + chosen bar fill → WPF brushes
  Flyout/FlyoutView.cs                         builds the element tree for a model and style
  Flyout/FlyoutWindow.cs                       borderless window, placement, hide on deactivate
  Snapshot.cs                                  --snapshot contact sheet
  WindowsTheme.cs                              (was TaskbarTheme.cs) light theme + accent palette
  AutoStart.cs                                 HKCU Run value
  ConnectFlow.cs                               dialogs around Core's Connection
  IconRenderer.cs                              level colours, centred taller digits
  TrayController.cs                            menu, left click, alerts, system events
  App.xaml.cs                                  --snapshot, per-folder mutex, show event
tests/OpenHeadroom.Tests/                      one test file per Core class above
```

---

### Task 1: Bridge: merge readings by window, chain timeout, UTF-8 PowerShell

**Files:**
- Modify: `src/OpenHeadroom.Core/Usage/RateLimits.cs`, `Usage/RateLimitsJson.cs`, `Limits/Limit.cs` (`LimitList.Make` only), `Bridge/BridgeRunner.cs`, `Bridge/ChainRunner.cs`
- Create: `src/OpenHeadroom.Core/Usage/ResetPeriod.cs`, `Usage/UsageMerge.cs`
- Test: `tests/OpenHeadroom.Tests/UsageMergeTests.cs`, `ResetPeriodTests.cs`; extend `RateLimitsJsonTests.cs`, `BridgeRunnerTests.cs`, `ChainRunnerTests.cs`, `LimitListTests.cs`

**Interfaces:**
- Produces:
  - `LimitWindow(double UsedPercentage, DateTimeOffset? ResetsAt, DateTimeOffset? ObservedAt = null)`
  - `SpendLimit(double UsedPercentage, DateTimeOffset? ResetsAt, double? UsedUsd, double? LimitUsd, string? Period, DateTimeOffset? ObservedAt = null)`
  - `ResetPeriod.Tolerance` (1 h), `bool ResetPeriod.Same(DateTimeOffset? a, DateTimeOffset? b)`: both null → true, one null → false, else |a − b| ≤ 1 h.
  - `UsageSnapshot UsageMerge.Merge(UsageSnapshot? stored, RateLimits incoming, DateTimeOffset now)`
  - `ChainResult? ChainRunner.Run(string command, string stdin, string? bashPath, TimeSpan? timeout = null)` (default 5 s)
  - `Limit.ObservedAt` is the window's own time (`window.ObservedAt ?? snapshot.ObservedAt`).

- [ ] **Step 1: Write the failing tests**

`ResetPeriodTests`: null/null same; null/value not same; 59 min apart same; 61 min apart not same.

`UsageMergeTests` (T = 2026-11-09 10:00 UTC, session resets T+3h, weekly resets T+4d):
- `Merge_with_nothing_stored_takes_reading_and_stamps_now`: every window's `ObservedAt == now`, snapshot `ObservedAt == now`.
- `Merge_keeps_higher_reading_in_same_period`: stored session 60% observed T−5m; incoming 40% with `resets_at` 10 s later → session stays 60% with `ObservedAt` T−5m.
- `Merge_takes_equal_or_higher_reading_in_same_period`: incoming 60% → taken, `ObservedAt == now`.
- `Merge_takes_lower_reading_from_a_later_period`: incoming 5% resets T+8h → taken.
- `Merge_ignores_reading_from_an_earlier_period`: stored resets T+8h at 5%; incoming resets T+3h at 70% → stored stays.
- `Merge_takes_reading_when_either_reset_time_is_missing`.
- `Merge_keeps_stored_window_missing_from_reading_until_its_reset_passes`: incoming has only session → stored weekly kept; with stored weekly `resets_at` ≤ now → weekly dropped.
- `Merge_fills_window_observed_at_from_old_snapshot`: stored windows with `ObservedAt == null` and snapshot `ObservedAt` T−1h, kept → `ObservedAt == T−1h` in the result.
- Spend merges by the same rule (one test: same period lower % keeps stored, `UsedUsd` and `Period` kept with it).

`RateLimitsJsonTests`: `WriteUsageFile` writes `"observed_at"` inside each window that has one; `ParseUsageFile` reads it; a milestone 1 file without window times parses with window `ObservedAt == null`. `ParseStatusLine` ignores any `observed_at` inside stdin windows (always null).

`LimitListTests`: a window with its own `ObservedAt` gives `Limit.ObservedAt` = that time; without, the snapshot's.

`BridgeRunnerTests`: `Run_does_not_lower_same_period_reading`: write a usage.json with session 60% (same `resets_at`), run with stdin at 40% → file still has 60%; the printed default line still shows `5h 40%` (the line shows what this session sent).

`ChainRunnerTests`:
- `StartInfo_falls_back_to_powershell` now expects the last argument `"[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); echo hi"`.
- `Run_times_out_and_returns_null`: with Git Bash (skip as the existing test does when none), `Run("sleep 20", "", bash, TimeSpan.FromMilliseconds(500))` returns null within 5 s (assert with a `Stopwatch`).
- `Run_powershell_output_is_utf8`: `Run("Write-Output '5h · 7d'", "", null)` → `Output.TrimEnd() == "5h · 7d"`.

- [ ] **Step 2: Run them and see them fail**

Run: `dotnet test`
Expected: compile errors for `ResetPeriod`, `UsageMerge`, the new parameters.

- [ ] **Step 3: Implement**

Records gain the optional `ObservedAt`. `RateLimitsJson` writes window `observed_at` as Unix seconds after `resets_at`, and reads it only in `ParseUsageFile`. `UsageMerge` per window: absent stored → incoming stamped `now`; both → incoming stamped `now` when `ResetsAt` is missing on either, or incoming is later by more than 1 h, or same period with `UsedPercentage >=` stored; else stored with its time resolved (`window.ObservedAt ?? stored.ObservedAt`); absent incoming → stored kept (time resolved) unless its `ResetsAt <= now`. `BridgeRunner` reads `UsageFile.Read(usagePath)`, merges, writes. `ChainRunner.Run` waits with `WaitForExit(timeout)`; on timeout `Kill(entireProcessTree: true)` and return null; read stdout asynchronously as now so a full pipe cannot deadlock.

- [ ] **Step 4: Run the tests**

Run: `dotnet test` then `dotnet build`
Expected: all pass; 0 warnings.

- [ ] **Step 5: Commit**

```bash
git add -A src/OpenHeadroom.Core tests/OpenHeadroom.Tests
git commit -m "Bridge: merge readings by window, time out a hung chain, UTF-8 PowerShell"
```

---

### Task 2: What the flyout says: levels, durations, pace, day names, the flyout model

**Files:**
- Modify: `src/OpenHeadroom.Core/Limits/Limit.cs`, `Display/When.cs`, `Display/TrayIconModel.cs`
- Create: `Limits/UsageLevel.cs`, `Display/Duration.cs`, `Display/Pace.cs`, `Display/FlyoutStyle.cs`, `Display/FlyoutModel.cs`
- Test: `UsageLevelTests.cs`, `DurationTests.cs`, `PaceTests.cs`, `FlyoutModelTests.cs`, `WhenTests.cs`; extend `LimitListTests.cs`, `TrayIconModelTests.cs`

**Interfaces:**
- Consumes: Task 1's per-window `ObservedAt`.
- Produces:
  - `Limit(... existing ..., TimeSpan? Window = null, double? UsedUsd = null, double? LimitUsd = null, string? Period = null)`; `LimitList.From` fills `Window` (session 5 h, weekly 7 d, spend by period: `daily` 1 d, `weekly` 7 d, `monthly` = `resetsAt − resetsAt.AddMonths(-1)`, else null) and the spend fields.
  - `enum UsageLevel { Normal, Warning, Error }`; `UsageLevels.WarningFrom = 70`, `ErrorFrom = 90`; `UsageLevel UsageLevels.For(int displayPercent)`; `UsageLevel UsageLevels.Highest(IReadOnlyList<Limit> limits)` (Normal when empty).
  - `TrayIconModel(string Text, double? WeeklyFraction, bool HasData, UsageLevel Level)`; `Level = UsageLevels.Highest(limits)`.
  - `string Duration.Format(TimeSpan span)`
  - `record PaceProjection(double ProjectedPercent, TimeSpan? RunsOutBeforeReset)`; `Pace.MinElapsedShare = 0.05`; `PaceProjection? Pace.Project(Limit limit)`
  - `enum FlyoutStyle { Bars, Battery, Thermometer }`
  - `record LimitView(LimitKind Kind, string Label, bool HasData, string UsedFigure, string LeftFigure, double UsedFraction, double? ProjectedFraction, UsageLevel Level, string? ResetText, string? PaceText, bool PaceLate, string? Detail)`
  - `record FlyoutModel(string Title, LimitView Session, LimitView Weekly, LimitView? Spend, string Footer)`; `FlyoutModel FlyoutModel.From(IReadOnlyList<Limit> limits, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture)`
  - `string FlyoutModel.ResetLine(Limit limit, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture)` (public; Task 4's toast body uses it).

- [ ] **Step 1: Write the failing tests**

Use zone UTC, culture `en-US`, now = Mon 2026-11-09 10:00 UTC.

- `UsageLevelTests`: 69 → Normal, 70 → Warning, 89 → Warning, 90 → Error; `Highest` of session 72 and weekly 91 → Error; empty → Normal.
- `TrayIconModelTests`: session 50, weekly 75 → `Level == Warning`.
- `DurationTests`: 3h11m → `3h 11m`; 3d13h59m → `3d 13h`; 2d0h30m → `2d`; 2h0m30s → `2h`; 45m → `45m`; 30 s → `less than a minute`; negative → `less than a minute`.
- `WhenTests` (move or add): Sat 2026-11-14 00:00 → `Sat 12:00 AM`; Mon 2026-11-16 00:00 (7 days out) → `Mon 16 Nov 12:00 AM`; same day 14:20 → `2:20 PM`; Mon 2026-11-02 14:02 (7 days back) → `Mon 2 Nov 2:02 PM`.
- `PaceTests` (observed = now):
  - session 27%, window 5 h, resets in 191 min → `ProjectedPercent` ≈ 74.31 (±0.01), `RunsOutBeforeReset == null`.
  - weekly 74%, window 7 d, resets in 85 h → `ProjectedPercent` ≈ 149.78; `RunsOutBeforeReset` ≈ 55.84 h (±1 min).
  - elapsed share 4% → null; percent 100 → null; `IsReset` → null; no `ResetsAt` → null; no `Window` → null; percent 0 at 50% elapsed → projected 0.
  - Pace uses `ObservedAt`, not now: same session reading observed 60 min ago, resets 131 min from now → same 74.31.
- `FlyoutModelTests`:
  - The mockup reading gives session `UsedFigure "27%"`, `LeftFigure "73%"`, `ResetText "Resets 1:11 PM · in 3h 11m"`, `PaceText "On track · about 26% left at the reset"`, `PaceLate false`, `ProjectedFraction` ≈ 0.7431; weekly (74%) `Level Warning`, `ResetText "Resets Thu 11:00 PM · in 3d 13h"`, `PaceText "At this pace it runs out 2d 7h before the reset"`, `PaceLate true`, `ProjectedFraction 1.0`.
  - Missing weekly → `HasData false`, figures `"–"`, `ResetText "No reading yet"`, `PaceText null`, `UsedFraction 0`.
  - A reset session (`IsReset`) → `UsedFigure "0%"`, `ResetText "Reset at 9:00 AM"`, no pace.
  - Spend 63% with `UsedUsd 31.5`, `LimitUsd 50`, `Period "monthly"` → `Spend.Label "Spend"`, `Detail "$31.50 of $50.00 this month"`; only `UsedUsd 1234.5`, `Period "daily"` → `"$1,234.50 spent today"`; no amounts → `Detail null`; no spend → `Spend == null`.
  - Footer: newest window observed 20 s ago → `"Updated just now"`; 3 min → `"Updated 3 min ago"`; 2h5m → `"Updated 2h 5m ago"`; 3 days → `"Updated Fri 10:00 AM"`; no limits → `"No numbers yet. They arrive with Claude Code's next response."`
  - `Title == "Claude Code"`.

- [ ] **Step 2: Run them and see them fail**

Run: `dotnet test` — Expected: compile errors for the new types.

- [ ] **Step 3: Implement**

`When.Format`: day name when the local dates are at most 6 days apart in either direction, else `ddd d MMM` (invariant) plus the clock. Pace: `elapsed = Window − (ResetsAt − ObservedAt)`, `share = elapsed / Window`; null outside `[0.05, 1]`; `projected = Percent / share`; when `projected > 100`, `runOutAt = ObservedAt + elapsed × (100 − Percent) / Percent` and `RunsOutBeforeReset = ResetsAt − runOutAt`. Pace text: on track `On track · about {round(100 − projected)}% left at the reset` (round half away from zero); late `At this pace it runs out {Duration} before the reset`. `ProjectedFraction = min(projected, 100) / 100`, null without a pace. Reset line: `Resets {When} · in {Duration}`, after a reset `Reset at {When}`, null with no reset time. Money: `"$" + value.ToString("#,0.00", InvariantCulture)`; period suffix ` today` / ` this week` / ` this month`. Footer from the newest `ObservedAt`: under 1 min, under 60 min (`{m} min ago`), under 24 h (`{Duration} ago`), else `Updated {When}`.

- [ ] **Step 4: Run the tests** — `dotnet test`, `dotnet build`: all pass, 0 warnings.

- [ ] **Step 5: Commit** — `git commit -m "Core: levels, pace, durations and the flyout's text"`

---

### Task 3: Colours and placement

**Files:**
- Create: `src/OpenHeadroom.Core/Colours/Rgb.cs`, `Contrast.cs`, `NightPalette.cs`, `AccentPalette.cs`, `BarColour.cs`; `Display/FlyoutPlacement.cs`
- Test: `ContrastTests.cs`, `AccentPaletteTests.cs`, `BarColourTests.cs`, `FlyoutPlacementTests.cs`

**Interfaces:**
- Consumes: `UsageLevel` (Task 2).
- Produces:
  - `readonly record struct Rgb(byte R, byte G, byte B)`; `Rgb.Parse("#RRGGBB")`; `string ToHex()` → `"#RRGGBB"` upper case.
  - `double Contrast.Ratio(Rgb a, Rgb b)` (WCAG 2.x relative luminance, sRGB threshold 0.04045).
  - `record NightPalette(Rgb Ground, Rgb Ink, Rgb Soft, Rgb Line, Rgb Track, Rgb Warning, Rgb Error, Rgb Purple)`; `NightPalette.Dark`, `.Light`, `.For(bool dark)` with the values under "Decisions"; `Rgb LevelColour(UsageLevel level, Rgb normal)` (Warning → `Warning`, Error → `Error`, else `normal`).
  - `record AccentPalette(Rgb Light3, Rgb Light2, Rgb Light1, Rgb Accent, Rgb Dark1, Rgb Dark2, Rgb Dark3)`; `AccentPalette? AccentPalette.FromRegistry(byte[]? bytes)` (null under 28 bytes; RGBA quads in that order).
  - `enum BarColourOption { Purple, Accent }`; `enum BarColourSource { Purple, Accent, AccentShade, PurpleFallback }`; `record BarColourChoice(Rgb Colour, BarColourSource Source)`; `BarColour.MinContrast = 3.0`; `BarColourChoice BarColour.Pick(BarColourOption option, AccentPalette? accent, bool dark)`.
  - `readonly record struct PixelRect(int Left, int Top, int Right, int Bottom)` with `Width`, `Height`; `enum TaskbarEdge { Bottom, Top, Left, Right }`; `TaskbarEdge FlyoutPlacement.Edge(PixelRect monitor, PixelRect workArea, int cursorX, int cursorY)`; `(int X, int Y) FlyoutPlacement.Place(PixelRect monitor, PixelRect workArea, int cursorX, int cursorY, int width, int height, int margin)`.

- [ ] **Step 1: Write the failing tests**

- `ContrastTests`: black/white 21.00; `#767676` on white 4.54 (±0.005); symmetric.
- `AccentPaletteTests`: the bytes `99 EB FF 00 4C C2 FF 00 00 91 F8 00 00 78 D4 00 00 67 C0 00 00 3E 92 00 00 1A 68 00 F7 63 0C 00` give Light3 `#99EBFF`, Light2 `#4CC2FF`, Light1 `#0091F8`, Accent `#0078D4`, Dark1 `#0067C0`, Dark2 `#003E92`, Dark3 `#001A68`; null and 27 bytes → null.
- `BarColourTests`:
  - Purple option → `#7B62FF` dark, `#5A3DD1` light, source Purple, whatever the accent.
  - Blue palette above → `#0078D4` source Accent in both themes (track ratios 3.34 and 3.56).
  - Storm palette (Accent `#4C4A48`, Light1 `#6B6967`, Light2 `#8F8D8B`, Light3 `#B5B3B1`, Dark1 `#3A3836`, Dark2 `#2A2826`, Dark3 `#1A1816`) → dark `#8F8D8B` AccentShade (Light1 fails the track at 2.77); light `#4C4A48` Accent.
  - All seven `#FFFF00` → light `#5A3DD1` PurpleFallback; dark `#FFFF00` Accent.
  - Accent option with null palette → purple, PurpleFallback.
- `FlyoutPlacementTests` (monitor 0,0,1920,1080; flyout 330×400; margin 12):
  - Work area bottom 1032, cursor (1800,1060) → Bottom, (1578, 620).
  - Cursor (300,1060) → x 135.
  - Work area top 48 (taskbar on top), cursor (1800,20) → Top, y 60.
  - Work area left 48, cursor (20,900) → Left, (60, 668).
  - Work area right 1872, cursor (1900,500) → Right, x 1530, y 300.
  - Auto-hide (work area = monitor), cursor (1800,1075) → Bottom, y 668.
  - Second monitor at −1920,0,0,1080 with work area bottom 1032, cursor (−100,1060) → x −342.

- [ ] **Step 2: Run them and see them fail** — `dotnet test`: compile errors.

- [ ] **Step 3: Implement**

`BarColour.Pick` tries, in order, Accent then Light1, Light2, Light3 on dark, or Accent then Dark1, Dark2, Dark3 on light; the first with `Ratio ≥ 3` against both `Ground` and `Track` of `NightPalette.For(dark)` wins (Accent source for the accent itself, AccentShade for a shade), else `Purple` with PurpleFallback. `Edge`: the side where the work area is inset from the monitor; with no inset, the monitor edge nearest the cursor. `Place`: Bottom → x centred on the cursor and clamped to `[work.Left + margin, work.Right − width − margin]`, y = `work.Bottom − height − margin`; Top → y = `work.Top + margin`; Left/Right → x against that edge, y centred on the cursor and clamped the same way.

- [ ] **Step 4: Run the tests** — all pass, 0 warnings.

- [ ] **Step 5: Commit** — `git commit -m "Core: contrast check, accent palette, bar colour and flyout placement"`

---

### Task 4: Config, alerts and atomic writes

**Files:**
- Create: `src/OpenHeadroom.Core/IO/AtomicFile.cs`, `Config/AppConfig.cs`, `Alerts/AlertState.cs`, `Alerts/AlertPolicy.cs`
- Modify: `Usage/UsageFile.cs` (write through `AtomicFile`), `AppPaths.cs`, `Directory.Build.props`
- Test: `AtomicFileTests.cs`, `AppConfigTests.cs`, `AlertPolicyTests.cs`, `AlertStateTests.cs`, `AppPathsTests.cs`

**Interfaces:**
- Consumes: `FlyoutStyle` (Task 2), `BarColourOption` (Task 3), `ResetPeriod` (Task 1), `FlyoutModel.ResetLine` (Task 2).
- Produces:
  - `void AtomicFile.WriteAllText(string path, string text, bool utf8Bom = false)`: creates the folder, writes a temp file beside the target, renames over it with 5 tries 20 ms apart, deletes the temp file on failure.
  - `record PreviousStatusLine(string? RawJson)`
  - `record AppConfig(string? ChainedCommand, PreviousStatusLine? Previous, FlyoutStyle Style, BarColourOption BarColour, IReadOnlyDictionary<string, string> Extra)`; `AppConfig.Default`; `AppConfig AppConfig.Load(string path)`; `void Save(string path)`.
  - `record AlertMark(DateTimeOffset? ResetsAt, int Threshold)`; `record AlertState(IReadOnlyDictionary<LimitKind, AlertMark> Marks)`; `AlertState.Empty`, `Load(string path)`, `Save(string path)`.
  - `record Alert(LimitKind Kind, int Threshold, string Title, string Body)`; `AlertPolicy.Thresholds = [80, 95]`; `(IReadOnlyList<Alert> Alerts, AlertState Next) AlertPolicy.Check(IReadOnlyList<Limit> limits, AlertState state, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture)`.
  - `string AppPaths.AlertsFile()`; `string AppPaths.InstanceId()`.

- [ ] **Step 1: Write the failing tests**

- `AtomicFileTests`: writes the text with no BOM by default; with `utf8Bom: true` the file starts `EF BB BF`; creates a missing folder; leaves no `*.tmp` behind.
- `AppConfigTests`:
  - Missing, empty, corrupt or non-object file → `Default` (null command, null `Previous`, Bars, Purple).
  - Round trip of every field; `previous_status_line` object text comes back byte for byte (`{"type":"command","command":"x","padding":0}`), and JSON `null` loads as `Previous.RawJson == null` (distinct from an absent key → `Previous == null`).
  - Unknown key `"future": [1, 2]` survives a load and save.
  - `"style": "battery"`, `"bar_colour": "accent"` load; unknown values fall back to Bars / Purple.
  - A saved config with a chained command is read by the existing `BridgeConfig.Load` with the same command.
- `AlertPolicyTests` (session resets T+3h):
  - 79% → no alert. 80% → one alert, `Title "Session limit at 80%"`, `Body "Resets 1:00 PM · in 3h"`, state marks session at 80 for that period.
  - Same period again at 85% → none. 96% → one alert at 95. Jump from 50% straight to 97% → a single 95 alert.
  - New period (resets 6 h later) at 82% → alert again.
  - Reset limit (`IsReset`) → none.
  - Weekly and session both over 80 → two alerts, highest threshold first, then session before weekly.
- `AlertStateTests`: round trip; missing or corrupt file → `Empty`. Keys `session`, `weekly`, `spend`; values `{"resets_at": <unix s or null>, "threshold": 80}`.
- `AppPathsTests`: `InstanceId()` with no override → `"OpenHeadroom"`; with `OPENHEADROOM_DATA_DIR` set to the same path in two casings → equal `"OpenHeadroom."` + 8 lower-case hex; different paths → different ids. `AlertsFile()` ends with `alerts.json`. (Set and restore the variable inside the test; mark the class `[Collection("env")]` so it does not run in parallel with other tests that read it.)

- [ ] **Step 2: Run them and see them fail** — `dotnet test`: compile errors.

- [ ] **Step 3: Implement**

`AppConfig.Save` writes indented JSON with `Utf8JsonWriter` in the order `chained_command` (omitted when null), `previous_status_line` (omitted when `Previous` is null; `WriteRawValue` of `RawJson`, or `null`), `style`, `bar_colour`, then each extra with `WriteRawValue`. `Load` keeps each unknown property's `GetRawText()`. `InstanceId` hashes the upper-invariant full path with SHA-256 and takes the first 4 bytes as hex. Add `<Version>0.2.0</Version>` to `Directory.Build.props`.

- [ ] **Step 4: Run the tests** — all pass, 0 warnings.

- [ ] **Step 5: Commit** — `git commit -m "Core: app config, alert state and atomic writes"`

---

### Task 5: Connect and Disconnect, Start with Windows command

**Files:**
- Create: `src/OpenHeadroom.Core/Connect/ClaudePaths.cs`, `SettingsJson.cs`, `BridgeCommand.cs`, `Connection.cs`, `BridgeLocator.cs`; `Startup/AutoStartCommand.cs`
- Test: `ClaudePathsTests.cs`, `SettingsJsonTests.cs`, `BridgeCommandTests.cs`, `ConnectionTests.cs`, `BridgeLocatorTests.cs`, `AutoStartCommandTests.cs`

**Interfaces:**
- Consumes: `AppConfig`, `PreviousStatusLine`, `AtomicFile` (Task 4).
- Produces:
  - `string ClaudePaths.ConfigDirectory(Func<string, string?> getEnv)` (`CLAUDE_CONFIG_DIR` when set and fully qualified, else `USERPROFILE\.claude`); `string ClaudePaths.SettingsFile(Func<string, string?> getEnv)`.
  - `string? SettingsJson.ReadStatusLine(string text)`: raw text of the top-level `statusLine` value, null when absent; empty or whitespace text counts as `{}`; throws `FormatException` when the text is not a JSON object. Comments and trailing commas are tolerated.
  - `string SettingsJson.WithStatusLine(string text, string? valueJson)`: replaces only that value's span, removes the property (and its comma) when `valueJson` is null, or appends it as the last property when absent.
  - `string? SettingsJson.CommandOf(string? statusLineJson)`
  - `string BridgeCommand.For(string bridgeExePath)`; `bool BridgeCommand.IsBridge(string? command)`; `string BridgeCommand.StatusLineJson(string command, string? existingStatusLineJson)`.
  - `record SettingsSnapshot(string Path, bool Exists, string Text, bool Bom)`; `SettingsSnapshot Connection.ReadSettings(string path)`.
  - `record ConnectionPlan(bool Connect, SettingsSnapshot Settings, string? Before, string? After, string NewSettingsText, AppConfig NewConfig, bool Chains)`.
  - `bool Connection.IsConnected(string settingsText)` (false on invalid JSON).
  - `ConnectionPlan Connection.PlanConnect(SettingsSnapshot settings, AppConfig config, string bridgeExePath)`; `ConnectionPlan Connection.PlanDisconnect(SettingsSnapshot settings, AppConfig config)` (throws `InvalidOperationException` when not connected).
  - `string? Connection.Apply(ConnectionPlan plan, string configPath, DateTimeOffset localNow)`: returns the backup path (null when there was no file).
  - `string? BridgeLocator.Find(string appDirectory, Func<string, bool> fileExists)`: `<dir>\openheadroom-bridge.exe`, then `<dir>\..\bridge\openheadroom-bridge.exe`, full paths.
  - `AutoStartCommand.ValueName = "OpenHeadroom"`; `string AutoStartCommand.For(string exePath)` → `"\"<exePath>\""`; `bool AutoStartCommand.Matches(string? value, string exePath)` (quotes trimmed, case-insensitive full-path compare).

- [ ] **Step 1: Write the failing tests**

- `SettingsJsonTests`:
  - Read: absent → null; `{"statusLine": {"type":"command","command":"x"}}` → that object text; `[]` and `{oops` → `FormatException`; `// comment` before a key is tolerated.
  - Replace keeps everything else byte for byte: a file with a BOM-free UTF-8 `"model": "café"`, a `// keep me` comment, 4-space indentation and a key after `statusLine` → only the value changes (assert the full expected string).
  - Remove: first, middle and last property positions each give valid JSON with the other keys intact (assert exact strings); removing the only property gives `{}` plus the original trailing newline.
  - Insert into `{\n  "model": "x"\n}\n` → `{\n  "model": "x",\n  "statusLine": V\n}\n`; into `""` → `{\n  "statusLine": V\n}\n`.
- `BridgeCommandTests`: `For(@"C:\Dev\open headroom\openheadroom-bridge.exe")` → `"C:/Dev/open headroom/openheadroom-bridge.exe"` wrapped in double quotes; `IsBridge` true for `"C:/x/OpenHeadroom-Bridge.exe"` and `~/bin/openheadroom-bridge`, false for `~/.claude/statusline.sh` and null; `StatusLineJson(cmd, null)` →
  ```text
  {
      "type": "command",
      "command": "\"C:/x/openheadroom-bridge.exe\""
    }
  ```
  (quotes escaped as `\"`, not `\u0022`); with an existing `{"type":"command","command":"old","padding":2}` the result keeps `"padding": 2` after `command`.
- `ConnectionTests` (temp folder; never the real config dir):
  - Connect with an existing `statusLine` running `~/.claude/statusline.sh`: plan `Chains == true`, `NewConfig.ChainedCommand == "~/.claude/statusline.sh"`, `NewConfig.Previous.RawJson` = the old object text; after `Apply`, `settings.json` runs the bridge, `config.json` has both keys, the backup `settings.json.openheadroom-20261109-143000.bak` holds the old bytes.
  - Connect with no `statusLine` → `Previous.RawJson == null`, `Chains == false`.
  - Connect with no `settings.json` → file created; `Apply` returns null (no backup).
  - Connect when `statusLine` already runs `C:/old/openheadroom-bridge.exe` → the command is not saved as `ChainedCommand`; `Previous` and `ChainedCommand` keep the config's values; the new command is the new bridge path.
  - Disconnect after that Connect restores the original `statusLine` text exactly and clears both config keys.
  - Disconnect of a manual setup (no `previous_status_line`, no `chained_command`) removes `statusLine`; with `chained_command "x"` (not a bridge) sets `{"type":"command","command":"x"}`.
  - `PlanDisconnect` when not connected throws.
  - `Apply_refuses_when_settings_changed_since_the_plan`: change the file after planning → `InvalidOperationException`, file and config untouched.
  - A file with a BOM keeps its BOM.
  - Invalid JSON → `PlanConnect` throws `FormatException`; the file is unchanged.
- `ClaudePathsTests`: `CLAUDE_CONFIG_DIR` used when set to a full path; relative or empty ignored → `USERPROFILE\.claude`.
- `BridgeLocatorTests`: found beside; found in `..\bridge`; neither → null.
- `AutoStartCommandTests`: `For` quotes; `Matches` true for the same path in other case with or without quotes, false for another path and null.

- [ ] **Step 2: Run them and see them fail** — `dotnet test`: compile errors.

- [ ] **Step 3: Implement**

`SettingsJson` works on the UTF-8 bytes with `Utf8JsonReader` (`CommentHandling.Skip`, `AllowTrailingCommas`), walks only the top level, records each property's name start and value end (`TokenStartIndex`, then `Skip()` and `BytesConsumed`), and splices bytes; it never re-serialises the rest. `StatusLineJson` uses `Utf8JsonWriter` with `JavaScriptEncoder.UnsafeRelaxedJsonEscaping` and indents the object for a top-level property (inner keys 4 spaces, closing brace 2). `Apply` re-reads the settings file and compares it with `plan.Settings` (existence and text) before writing anything; it then copies the file to the backup name (local time), and writes in this order: Connect saves `config.json` first, then `settings.json`; Disconnect writes `settings.json` first, then `config.json`. Settings are written through `AtomicFile` with the original BOM.

- [ ] **Step 4: Run the tests** — all pass, 0 warnings.

- [ ] **Step 5: Commit** — `git commit -m "Core: Connect and Disconnect edit only statusLine, with a backup"`

---

### Task 6: The Night flyout, fonts and the snapshot sheet

**Files:**
- Add (already fetched by the controller): `src/OpenHeadroom/Fonts/Inter-Regular.ttf`, `Inter-Medium.ttf`, `Unbounded-Bold.ttf`, `OFL-Inter.txt`, `OFL-Unbounded.txt`
- Create: `src/OpenHeadroom/Flyout/FlyoutColours.cs`, `Flyout/FlyoutView.cs`, `Flyout/FlyoutWindow.cs`, `Snapshot.cs`
- Modify: `src/OpenHeadroom/OpenHeadroom.csproj` (`<Resource Include="Fonts\*.ttf" />`, the `.txt` licences copied to output), `App.xaml.cs` (`--snapshot` only), `IconRenderer.cs`, rename `TaskbarTheme.cs` → `WindowsTheme.cs`

**Interfaces:**
- Consumes: `FlyoutModel`, `LimitView`, `FlyoutStyle` (Task 2); `NightPalette`, `BarColour`, `AccentPalette`, `FlyoutPlacement`, `PixelRect` (Task 3); `UsageLevel`.
- Produces:
  - `WindowsTheme.IsLight()` (as `TaskbarTheme.IsLight` today); `AccentPalette? WindowsTheme.Accent()`.
  - `record FlyoutColours(NightPalette Palette, Rgb Fill)`; `FlyoutColours.For(bool dark, BarColourOption option, AccentPalette? accent)`.
  - `FrameworkElement FlyoutView.Build(FlyoutModel model, FlyoutStyle style, FlyoutColours colours)`
  - `FlyoutWindow : Window` with `void Render(FlyoutModel model, FlyoutStyle style, FlyoutColours colours)`, `void ShowNear(int cursorX, int cursorY)`, `bool HiddenJustNow` (true for 300 ms after it hid itself on deactivation).
  - `IconRenderer.Render(TrayIconModel model, bool lightTaskbar, int size)` colours the digits and bar by `model.Level` (Normal: white on a dark taskbar, `#1A1A1A` on a light one, grey `#8A8A8A` without data; Warning and Error: `NightPalette.For(!lightTaskbar)` amber and red).
  - `int Snapshot.Write(string pngPath)` (0 on success).

- [ ] **Step 1: Build the view from the mockup's measures**

All sizes in DIPs. Fonts: `new FontFamily(new Uri("pack://application:,,,/"), "./Fonts/#Inter")` and `#Unbounded`; check the family names WPF reports for the files and use them.
- Window card: width 330, ground fill, 1 px `Line` border, corner radius 10, padding 18 18 18 14, Inter 15 px, line height 1.45 × size, `Ink` text.
- Title `model.Title`, `Soft`, padding-bottom 12, then a 1 px `Ink` hairline.
- Figures: Unbounded Bold 22 px, tabular; the unit (`used` / `left`) Inter 14 px `Soft`, 4 px left margin. Reset and pace lines Inter 14 px `Soft`; a late pace line in `Warning`.
- **Bars** (also the spend row in every style): each limit padded 14 top and bottom with a 1 px `Line` under it; label left and figure right on one baseline, then a 6 px meter (`Track`) whose fill is `UsedFraction` wide in `Palette.LevelColour(Level, Fill)`; a 2 px × 12 px `Ink` tick centred vertically on the meter at `ProjectedFraction` when it is below 1; then the reset line and the pace line, 6 px gaps. `Detail` (spend) sits under the reset line.
- **Battery**: session and weekly side by side (two equal columns, 12 px gap, padding 16 top 14 bottom, `Line` under), centred: a 128 × 40 battery (outline x1 y1 w117 h38 radius 5, stroke `Soft` 1.5; nub x120 y13 w5 h14 radius 1.5 `Soft`; ten cells 8 × 30 at x = 5 + 11i, y 5, radius 1.5). `left = 10 − round(used% / 10)`, `soon = min(left, max(0, round((projected% − used%) / 10)))`; cell i < left − soon is on, i < left is soon (opacity .35), else off (`Track`). On and soon cells take `Fill`, or the level colour from 70%. Then figure `LeftFigure` + `left`, the label in Inter Medium, the reset line, the pace line, 4 px gaps.
- **Thermometer**: same columns; a 30 × 104 glyph: tube x10 y4 w10 h86 radius 5 `Track`; with `h = 80`, `yLeft = 4 + h × (1 − left%/100)`, `ySoon = yLeft + h × soon%/100` (soon% = clamp(projected% − used%, 0, left%)): a faint rect (opacity .35) from `yLeft` to `ySoon`, a solid rect from `ySoon` to 90, a bulb circle centre (15, 93) radius 9; colour `Fill` or the level colour from 70%. Beside it a scale `100%` over `0%`, Inter 11 px `Soft`, 88 px tall, 10 px gap.
- Footer: `model.Footer`, Inter 14 px `Soft`, padding-top 12.
- No shadow, no animation.

- [ ] **Step 2: The window**

`FlyoutWindow`: `WindowStyle.None`, `AllowsTransparency`, transparent background, `ShowInTaskbar = false`, `Topmost`, `ResizeMode.NoResize`, `SizeToContent.WidthAndHeight`, `WS_EX_TOOLWINDOW`; Esc and `Deactivated` hide it. `ShowNear`: show it off-screen, measure, find the monitor under the cursor (`MonitorFromPoint`, `GetMonitorInfo`, `GetDpiForMonitor`), convert the DIP size to pixels at that DPI, ask `FlyoutPlacement.Place` with margin 12 × DPI / 96, move it with `SetWindowPos` in pixels, then `Activate()`. Place it again on `DpiChanged`.

- [ ] **Step 3: Icon digits**

Draw the text as a `GraphicsPath` (Segoe UI Bold), scale it so the path's bounds of `"00"` fill the width with 1 px side padding and the height above the weekly bar less a 1 px gap, and centre the actual text's bounds in that area. The digits come out about 20% taller than milestone 1 and centred.

- [ ] **Step 4: Snapshot sheet**

`OpenHeadroom.exe --snapshot <png>` (handled first in `App.OnStartup`, then `Shutdown(code)`, no tray, no mutex): a sample reading (session 27% resetting in 191 min, weekly 74% resetting in 85 h, observed 3 min ago) rendered as a grid: rows Bars, Battery, Thermometer; columns dark purple, dark accent, light purple, light accent, each card on a 16 px backdrop (`#2B3542` dark, `#C9D3DE` light). The accent is this machine's palette, or the blue palette from Task 3 when there is none. A last row draws the tray icon for no data, 8, 27, 74, 93 and 100 (`!`) at 16, 20, 24 and 32 px on `#1C1C1C` and `#F0F0F0` strips, each also scaled ×4 nearest-neighbour. Render with `RenderTargetBitmap` at 96 DPI and save with `PngBitmapEncoder`.

- [ ] **Step 5: Check it**

Run: `dotnet build` (0 warnings), then `src/OpenHeadroom/bin/Debug/net10.0-windows/OpenHeadroom.exe --snapshot <scratch>/sheet.png` and compare the sheet with the mockup: type faces and sizes, hairlines, the tick, the battery cells, the thermometer, amber on the 74% weekly, the late pace line in amber, light and dark grounds, centred icon digits. Fix what differs.

- [ ] **Step 6: Commit** — `git commit -m "Tray: Night flyout with three styles, bundled fonts, snapshot sheet"`

---

### Task 7: Tray: left click, menu, levels, toasts, Windows events

**Files:**
- Create: `src/OpenHeadroom/AutoStart.cs`, `ConnectFlow.cs`
- Modify: `src/OpenHeadroom/TrayController.cs`, `App.xaml.cs`

**Interfaces:**
- Consumes: everything above. `Connection`, `ClaudePaths`, `BridgeLocator`, `AppConfig`, `AlertPolicy`, `AlertState`, `AutoStartCommand`, `AppPaths.InstanceId()`, `FlyoutWindow`, `FlyoutColours`, `WindowsTheme`.
- Produces:
  - `AutoStart.IsOn()`, `AutoStart.Set(bool on)` on `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, value `OpenHeadroom`, for `Environment.ProcessPath`.
  - `ConnectFlow.Run(bool connect)`.

- [ ] **Step 1: App start**

Mutex `Local\{InstanceId}.Tray`. A second instance opens the `Local\{InstanceId}.Show` auto-reset event, calls `AllowSetForegroundWindow(ASFW_ANY)`, sets it, and exits 0. The first creates that event and opens the flyout when it is set (registered wait → dispatcher).

- [ ] **Step 2: Left click and refresh**

Left click (`MouseClick`, left button) toggles the flyout at the cursor, unless `HiddenJustNow`. `Refresh` builds the limits once and feeds the tooltip, the icon (with its level), the flyout when visible, and `AlertPolicy.Check`; it shows at most the first alert as a balloon (`ToolTipIcon.None`) and saves `alerts.json` when the state changed. The 30 s tick stays.

- [ ] **Step 3: Menu**

In this order, sentence case: `Connect to Claude Code` or `Disconnect from Claude Code` (decided on `Opening` with `Connection.IsConnected` on the settings file), `Style` ▸ `Bars` / `Battery` / `Thermometer` (checked), `Bar colour` ▸ `OpenHeadroom purple` / `Windows accent` (checked), separator, `Start with Windows` (checked from `AutoStart.IsOn()`), `Open data folder`, `About OpenHeadroom`, separator, `Quit`. Style and colour reload `config.json`, change one field, save, and redraw. About is a message box: `OpenHeadroom 0.2.0`, "Shows Claude Code's session and weekly limits from its status line. It makes no network calls.", "Not affiliated with or endorsed by Anthropic.", "Fonts: Inter and Unbounded, SIL Open Font License."

- [ ] **Step 4: Connect flow**

`ConnectFlow.Run`: settings path from `ClaudePaths.SettingsFile(Environment.GetEnvironmentVariable)`; for Connect the bridge from `BridgeLocator.Find(AppContext.BaseDirectory, File.Exists)` or an error box ("Can't find openheadroom-bridge.exe next to OpenHeadroom.exe or in ..\bridge."). Plan; on `FormatException` an error box ("settings.json isn't valid JSON, so OpenHeadroom left it alone."). A Yes/No box titled `Connect to Claude Code` / `Disconnect from Claude Code` shows the file path, `statusLine now:` (or `not set`), `statusLine after:` (or `removed`), "Your current status line keeps running: the bridge passes its output through." when `Chains`, and "A backup is saved first." On Yes, `Apply`; then a balloon ("Connected. The numbers arrive with Claude Code's next response." / "Disconnected."). Any exception → an error box with its message; nothing else written.

- [ ] **Step 5: Windows events (milestone 1 backlog)**

`UserPreferenceChanged`: only `General`, `Color`, `VisualStyle` and `Locale`; on `Locale` first `CultureInfo.CurrentCulture.ClearCachedData()`. `SystemEvents.TimeChanged`: `TimeZoneInfo.ClearCachedData()`. `DisplaySettingsChanged`: redraw the icon. Each refreshes on the dispatcher; unsubscribe on dispose.

- [ ] **Step 6: Check it in the running app**

Build, then start the tray from the build output with `OPENHEADROOM_DATA_DIR` and `CLAUDE_CONFIG_DIR` set to scratch folders. Write `usage.json` readings at 50%, 75% and 92% (the bridge with sample stdin, or the file directly) and check the icon colour, the flyout (start the exe again to open it), and the balloon at 80% and 95% only once. Check that Ricardo's own tray keeps running beside it.

- [ ] **Step 7: Commit** — `git commit -m "Tray: flyout on click, menu with Connect, style, colour and autostart, toasts"`

---

### Task 8: Docs

**Files:**
- Modify: `README.md`, `docs/plan.md`

- [ ] **Step 1: README**

Status: milestone 2. How it works: the flyout (left click), the styles and bar colour (right click), toasts. Replace the manual steps 2–3 with "Right-click the icon → Connect to Claude Code", keeping the manual setup as a section for people who prefer it. Known limits: add "The desktop app doesn't run the status line either (seen 2026-11-09)."; replace the "last one to refresh wins" line with the merge rule in plain words. Note the bundled fonts and their licence.

- [ ] **Step 2: plan.md**

Known limits: the desktop app doesn't run the status line (seen 2026-11-09). Risks: several sessions now merge by window. Milestone 2: a short results list (what shipped, what milestone 3 needs).

- [ ] **Step 3: Commit** — `git commit -m "Docs: milestone 2"`
