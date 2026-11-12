# Pace over weekdays only: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A tray menu setting, Pace › Every day / Weekdays only, that makes the pace projection count Monday to Friday for limits whose window is longer than a day.

**Architecture:** A new `WorkingTime` helper in `Sobreiro.Core` measures weekday time between two moments in a time zone. `Pace.Project` uses it when the setting is Weekdays; the setting rides as an optional last argument through `LimitStatus.For`, `FlyoutModel.From` and `AlertPolicy.Check`, so every existing caller keeps today's behaviour. `AppConfig` stores it and `TrayMenu` sets it.

**Tech Stack:** C# / .NET 10, xUnit, WinForms menu in the WPF app.

**Spec:** [docs/superpowers/specs/2026-11-12-working-day-pace-design.md](../specs/2026-11-12-working-day-pace-design.md)

## Global Constraints

- Logic goes in `Sobreiro.Core` and gets an xUnit test (`tests/Sobreiro.Tests`).
- Every day is the default everywhere: a caller that does not pass the setting gets exactly today's results, and every existing test passes unchanged.
- Weekdays are Monday to Friday in the PC's time zone, midnight to midnight.
- Weekday counting applies only to windows **longer than one day**, and only when the window holds some working time; otherwise calendar time.
- No new network calls, no new dependencies. `settings.json` is not touched.
- Menu copy, exactly: submenu "Pace", items "Every day" and "Weekdays only".
- Config, exactly: key `pace`, values `everyday` and `weekdays`.
- Do not name or link other people's projects in code comments, docs or commit messages.
- Commit messages are plain sentences and end with
  `Co-Authored-By: Claude Fable 5.2 <noreply@anthropic.com>`.
- Tests run with `dotnet test Sobreiro.slnx` from the repository root.

## Review Focus

Each line has its test in the task named.

1. **A time zone far from UTC.** Friday 23:00 UTC is already Saturday in a +09:00 zone; the day of the week is the local one. (Task 1)
2. **A reading taken after the reset time.** It gives no projection in either mode, even when the days after the reset are a weekend. (Task 2)
3. **No working time gone yet** (a Sunday after a Saturday reset). "Too early", never a division by zero. (Task 2)
4. **A stale reading whose weekday run-out has passed.** The badge reads "Likely used up", as it does today. (Task 3)
5. **A hand-edited config with `"pace": 5`.** It reads as every day and does not throw. (Task 4)

---

### Task 1: WorkingTime

**Files:**
- Create: `src/Sobreiro.Core/Display/WorkingTime.cs`
- Test: `tests/Sobreiro.Tests/WorkingTimeTests.cs`

**Interfaces:**
- Produces:
  - `public static TimeSpan WorkingTime.Between(DateTimeOffset from, DateTimeOffset to, TimeZoneInfo zone)`:
    the time between the two moments that falls on Monday to Friday in `zone`; zero when `to <= from`.
  - `public static DateTimeOffset WorkingTime.After(DateTimeOffset from, TimeSpan working, TimeZoneInfo zone)`:
    the moment at which `working` weekday time has passed after `from`; `from` itself when
    `working <= TimeSpan.Zero`. The result's offset is not specified; tests compare moments.

- [ ] **Step 1: Write the failing tests**

```csharp
using Sobreiro.Core.Display;

namespace Sobreiro.Tests;

public class WorkingTimeTests
{
    private static readonly TimeZoneInfo Utc = TimeZoneInfo.Utc;
    private static DateTimeOffset T(string s) => DateTimeOffset.Parse(s);
    // 2026-11-09 is a Monday.

    [Fact] public void Weekdays_count_in_full() =>
        Assert.Equal(TimeSpan.FromHours(72), WorkingTime.Between(T("2026-11-09T09:00:00Z"), T("2026-11-12T09:00:00Z"), Utc));

    [Fact] public void A_weekend_in_the_middle_is_skipped() =>
        Assert.Equal(TimeSpan.FromHours(15), WorkingTime.Between(T("2026-11-06T18:00:00Z"), T("2026-11-09T09:00:00Z"), Utc));

    [Fact] public void Inside_a_weekend_is_zero() =>
        Assert.Equal(TimeSpan.Zero, WorkingTime.Between(T("2026-11-07T12:00:00Z"), T("2026-11-08T12:00:00Z"), Utc));

    [Fact] public void Backwards_is_zero() =>
        Assert.Equal(TimeSpan.Zero, WorkingTime.Between(T("2026-11-10T00:00:00Z"), T("2026-11-09T00:00:00Z"), Utc));

    [Fact] public void A_whole_november_has_21_weekdays() =>
        Assert.Equal(TimeSpan.FromHours(504), WorkingTime.Between(T("2026-11-01T00:00:00Z"), T("2026-12-01T00:00:00Z"), Utc));

    [Fact]
    public void The_day_of_the_week_is_the_local_one()
    {
        var plusNine = TimeZoneInfo.CreateCustomTimeZone("plus9", TimeSpan.FromHours(9), "plus9", "plus9");
        // Fri 22:00Z to Sat 02:00Z is Sat 07:00 to Sat 11:00 at +09:00.
        Assert.Equal(TimeSpan.Zero, WorkingTime.Between(T("2026-11-06T22:00:00Z"), T("2026-11-07T02:00:00Z"), plusNine));
        // Sun 20:00Z to Sun 23:00Z is Mon 05:00 to Mon 08:00 at +09:00.
        Assert.Equal(TimeSpan.FromHours(3), WorkingTime.Between(T("2026-11-08T20:00:00Z"), T("2026-11-08T23:00:00Z"), plusNine));
    }

    [Fact]
    public void A_clock_change_at_the_weekend_moves_midnight()
    {
        // Lisbon goes from +01:00 to +00:00 on Sunday 2026-10-25.
        var lisbon = TimeZoneInfo.FindSystemTimeZoneById("Europe/Lisbon");
        var friNoon = T("2026-10-23T11:00:00Z"); // 12:00 local
        var monNoon = T("2026-10-26T12:00:00Z"); // 12:00 local
        Assert.Equal(TimeSpan.FromHours(24), WorkingTime.Between(friNoon, monNoon, lisbon));
        Assert.Equal(monNoon, WorkingTime.After(friNoon, TimeSpan.FromHours(24), lisbon));
    }

    [Fact] public void After_skips_the_weekend() =>
        Assert.Equal(T("2026-11-10T04:00:00Z"), WorkingTime.After(T("2026-11-06T00:00:00Z"), TimeSpan.FromHours(52), Utc));

    [Fact] public void After_from_a_saturday_starts_on_monday() =>
        Assert.Equal(T("2026-11-09T01:00:00Z"), WorkingTime.After(T("2026-11-07T12:00:00Z"), TimeSpan.FromHours(1), Utc));

    [Fact] public void After_nothing_is_the_start() =>
        Assert.Equal(T("2026-11-07T12:00:00Z"), WorkingTime.After(T("2026-11-07T12:00:00Z"), TimeSpan.Zero, Utc));
}
```

- [ ] **Step 2: Run them and see them fail**

Run: `dotnet test Sobreiro.slnx --filter "FullyQualifiedName~WorkingTimeTests"`
Expected: a build error, `WorkingTime` does not exist.

- [ ] **Step 3: Implement `WorkingTime` in `src/Sobreiro.Core/Display/WorkingTime.cs`**

Walk the local calendar days from `from`'s local date. A day runs from its local midnight to
the next local midnight, each turned into a moment with the zone's offset at that midnight;
measure on moments, so a day is as long as the clock made it. A window is at most a month, so
day by day is fine. `After` walks the same days, spending `working` on weekdays.

- [ ] **Step 4: Run them and see them pass**

Run: `dotnet test Sobreiro.slnx --filter "FullyQualifiedName~WorkingTimeTests"`
Expected: 10 passed.

- [ ] **Step 5: Commit**

```bash
git add src/Sobreiro.Core/Display/WorkingTime.cs tests/Sobreiro.Tests/WorkingTimeTests.cs
git commit -m "Measure weekday time between two moments"
```

---

### Task 2: The projection over weekdays

**Files:**
- Create: `src/Sobreiro.Core/Display/PaceDays.cs`
- Modify: `src/Sobreiro.Core/Display/Pace.cs`
- Test: `tests/Sobreiro.Tests/PaceTests.cs` (add to it; the existing tests stay as they are)

**Interfaces:**
- Consumes: `WorkingTime.Between`, `WorkingTime.After` (Task 1).
- Produces:
  - `public enum PaceDays { EveryDay, Weekdays }` in namespace `Sobreiro.Core.Display`.
  - `public static PaceProjection? Pace.Project(Limit limit, PaceDays days, TimeZoneInfo zone)`.
  - `Pace.Project(Limit limit)` stays and means every day.

- [ ] **Step 1: Write the failing tests** (add inside `PaceTests`)

```csharp
    private static readonly TimeZoneInfo Utc = TimeZoneInfo.Utc;
    private static DateTimeOffset T(string s) => DateTimeOffset.Parse(s);

    private static Limit Long(double pct, string resets, string observed, double windowDays = 7) =>
        new("Claude Code", LimitKind.Weekly, "Weekly", pct, T(resets), T(observed), false, TimeSpan.FromDays(windowDays));

    private static PaceProjection? Weekdays(Limit l) => Pace.Project(l, PaceDays.Weekdays, Utc);

    // Run-out times come from a multiplication, so compare to the second.
    private static void Near(DateTimeOffset expected, DateTimeOffset? actual) =>
        Assert.True(actual is { } a && (a - expected).Duration() < TimeSpan.FromSeconds(1), $"{actual:u} is not {expected:u}");

    [Fact]
    public void Weekdays_remove_the_midweek_false_alarm()
    {
        // Reset Mon 09:00, read Thu 09:00 at 58%: 72 h of 168 h, or 72 h of 120 h.
        var l = Long(58, "2026-11-16T09:00:00Z", "2026-11-12T09:00:00Z");
        Assert.Equal(135.33, Pace.Project(l)!.ProjectedPercent, 0.01);
        var p = Weekdays(l)!;
        Assert.Equal(96.67, p.ProjectedPercent, 0.01);
        Assert.Null(p.RunsOutAt);
        Assert.Null(p.RunsOutBeforeReset);
    }

    [Fact]
    public void Weekdays_remove_the_false_calm_after_a_weekend()
    {
        // Reset Fri 09:00, read Mon 09:00 at 30%: 72 h of 168 h, or 24 h of 120 h.
        var l = Long(30, "2026-11-13T09:00:00Z", "2026-11-09T09:00:00Z");
        Assert.Equal(70.0, Pace.Project(l)!.ProjectedPercent, 0.01);
        var p = Weekdays(l)!;
        Assert.Equal(150.0, p.ProjectedPercent, 0.01);
        Near(T("2026-11-11T17:00:00Z"), p.RunsOutAt);                  // 56 weekday hours on
        Assert.Equal(40.0, p.RunsOutBeforeReset!.Value.TotalHours, 0.001);
    }

    [Fact]
    public void A_run_out_never_lands_on_the_weekend()
    {
        // Window Wed to Wed, read Fri 00:00 at 48%: 52 weekday hours left, past the weekend.
        var p = Weekdays(Long(48, "2026-11-11T00:00:00Z", "2026-11-06T00:00:00Z"))!;
        Assert.Equal(120.0, p.ProjectedPercent, 0.01);
        Near(T("2026-11-10T04:00:00Z"), p.RunsOutAt);
        Assert.Equal(20.0, p.RunsOutBeforeReset!.Value.TotalHours, 0.001);
    }

    [Fact]
    public void Use_on_a_day_off_makes_it_stricter()
    {
        var saturday = Weekdays(Long(70, "2026-11-11T00:00:00Z", "2026-11-07T12:00:00Z"))!;
        var sunday = Weekdays(Long(75, "2026-11-11T00:00:00Z", "2026-11-08T12:00:00Z"))!;
        Assert.Equal(116.67, saturday.ProjectedPercent, 0.01);
        Assert.Equal(125.0, sunday.ProjectedPercent, 0.01);
        Assert.True(saturday.RunsOutAt >= T("2026-11-09T00:00:00Z"));
    }

    [Fact]
    public void With_only_the_weekend_left_the_projection_is_the_current_figure()
    {
        var l = Long(80, "2026-11-16T00:00:00Z", "2026-11-14T10:00:00Z");
        Assert.True(Pace.Project(l)!.ProjectedPercent > 100);
        var p = Weekdays(l)!;
        Assert.Equal(80.0, p.ProjectedPercent, 0.01);
        Assert.Null(p.RunsOutAt);
    }

    [Fact]
    public void After_a_weekend_reset_it_is_too_early_until_five_percent_of_weekday_time()
    {
        Assert.Null(Weekdays(Long(3, "2026-11-14T00:00:00Z", "2026-11-08T12:00:00Z")));   // Sunday: none gone
        Assert.Null(Weekdays(Long(3, "2026-11-14T00:00:00Z", "2026-11-09T05:00:00Z")));   // 5 h of 120 h
        Assert.Equal(51.43, Weekdays(Long(3, "2026-11-14T00:00:00Z", "2026-11-09T07:00:00Z"))!.ProjectedPercent, 0.01);
    }

    [Fact]
    public void A_month_counts_its_weekdays()
    {
        // November 2026, read Mon 16th 00:00 at 40%: 15 of 30 days, or 10 of 21 weekdays.
        var l = Long(40, "2026-12-01T00:00:00Z", "2026-11-16T00:00:00Z", windowDays: 30);
        Assert.Equal(80.0, Pace.Project(l)!.ProjectedPercent, 0.01);
        Assert.Equal(84.0, Weekdays(l)!.ProjectedPercent, 0.01);
    }

    [Fact]
    public void A_window_of_a_day_or_less_keeps_calendar_time()
    {
        // A Saturday: a session, then a daily limit.
        var session = new Limit("Claude Code", LimitKind.Session, "Session", 27,
            T("2026-11-07T13:11:00Z"), T("2026-11-07T10:00:00Z"), false, TimeSpan.FromHours(5));
        Assert.Equal(74.31, Weekdays(session)!.ProjectedPercent, 0.01);
        var daily = Long(40, "2026-11-08T00:00:00Z", "2026-11-07T12:00:00Z", windowDays: 1);
        Assert.Equal(80.0, Weekdays(daily)!.ProjectedPercent, 0.01);
    }

    [Fact]
    public void A_window_with_no_weekday_in_it_keeps_calendar_time() =>
        Assert.Equal(80.0, Weekdays(Long(40, "2026-11-09T00:00:00Z", "2026-11-08T00:00:00Z", windowDays: 2))!.ProjectedPercent, 0.01);

    [Fact]
    public void A_reading_after_the_reset_gives_nothing_in_either_mode()
    {
        // Reset Sat 00:00, read Sat 06:00 and not marked reset.
        var l = Long(60, "2026-11-14T00:00:00Z", "2026-11-14T06:00:00Z");
        Assert.Null(Pace.Project(l));
        Assert.Null(Weekdays(l));
    }

    [Fact]
    public void Every_day_with_a_zone_is_the_same_as_before() =>
        Assert.Equal(149.78, Pace.Project(Make(74, TimeSpan.FromDays(7), TimeSpan.FromHours(85)), PaceDays.EveryDay, Utc)!.ProjectedPercent, 0.01);
```

- [ ] **Step 2: Run them and see them fail**

Run: `dotnet test Sobreiro.slnx --filter "FullyQualifiedName~PaceTests"`
Expected: a build error, `PaceDays` does not exist.

- [ ] **Step 3: Add `PaceDays` and the three-argument `Pace.Project`**

`Project(Limit limit)` calls `Project(limit, PaceDays.EveryDay, TimeZoneInfo.Utc)`. The early
returns stay as they are, plus `ObservedAt > ResetsAt` gives null. Then, with
`start = resetsAt - window`:

```
weekdays = days == Weekdays && window > 1 day && WorkingTime.Between(start, resetsAt) > 0
gone  = weekdays ? WorkingTime.Between(start, ObservedAt) : ObservedAt - start
total = weekdays ? WorkingTime.Between(start, resetsAt)   : window
share = gone / total            // below MinElapsedShare or above 1: null
projected = Percent / share
if projected > 100:
    needed   = gone * ((100 - Percent) / Percent)
    runOutAt = weekdays ? WorkingTime.After(ObservedAt, needed) : ObservedAt + needed
    RunsOutBeforeReset = resetsAt - runOutAt
```

- [ ] **Step 4: Run them and see them pass**

Run: `dotnet test Sobreiro.slnx --filter "FullyQualifiedName~PaceTests"`
Expected: 20 passed (9 old, 11 new).

- [ ] **Step 5: Commit**

```bash
git add src/Sobreiro.Core/Display/PaceDays.cs src/Sobreiro.Core/Display/Pace.cs tests/Sobreiro.Tests/PaceTests.cs
git commit -m "Project the pace over weekdays when asked"
```

---

### Task 3: The setting reaches the badge, the bars and the alerts

**Files:**
- Modify: `src/Sobreiro.Core/Display/LimitStatus.cs`, `src/Sobreiro.Core/Display/FlyoutModel.cs`,
  `src/Sobreiro.Core/Alerts/AlertPolicy.cs`
- Test: `tests/Sobreiro.Tests/LimitStatusTests.cs`, `FlyoutModelTests.cs`, `AlertPolicyTests.cs` (add to each)

**Interfaces:**
- Consumes: `PaceDays`, `Pace.Project(Limit, PaceDays, TimeZoneInfo)` (Task 2).
- Produces, each with the new argument last and optional:
  - `LimitStatus.For(Limit limit, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture, PaceDays days = PaceDays.EveryDay)`
  - `FlyoutModel.From(..., bool showClaude = true, PaceDays days = PaceDays.EveryDay)`
  - `AlertPolicy.Check(limits, state, now, zone, culture, PaceDays days = PaceDays.EveryDay)`

- [ ] **Step 1: Write the failing tests**

In `LimitStatusTests`:

```csharp
    private static DateTimeOffset T(string s) => DateTimeOffset.Parse(s);
    private static Limit Weekly(double pct, string resets, string observed) =>
        new("Claude Code", LimitKind.Weekly, "Weekly", pct, T(resets), T(observed), false, TimeSpan.FromDays(7));

    [Fact]
    public void Weekdays_turn_a_midweek_alarm_into_on_track()
    {
        var l = Weekly(58, "2026-11-16T09:00:00Z", "2026-11-12T09:00:00Z");
        var at = T("2026-11-12T09:00:00Z");
        Assert.Equal(StatusKind.Ends, LimitStatus.For(l, at, TimeZoneInfo.Utc, En)!.Kind);
        Assert.Equal("On track", LimitStatus.For(l, at, TimeZoneInfo.Utc, En, PaceDays.Weekdays)!.Text);
    }

    [Fact]
    public void Weekdays_show_a_run_out_on_a_working_day()
    {
        // Reset Fri 09:00, read Mon 09:00 at 40%: 24 weekday hours gone, 36 more to run out.
        var l = Weekly(40, "2026-11-13T09:00:00Z", "2026-11-09T09:00:00Z");
        var at = T("2026-11-09T09:00:00Z");
        Assert.Equal("On track", LimitStatus.For(l, at, TimeZoneInfo.Utc, En)!.Text);
        Assert.Equal("Ends Tue 9:00 PM", LimitStatus.For(l, at, TimeZoneInfo.Utc, En, PaceDays.Weekdays)!.Text);
    }

    [Fact]
    public void A_stale_reading_past_its_weekday_run_out_is_likely_used_up()
    {
        var l = Weekly(40, "2026-11-13T09:00:00Z", "2026-11-09T09:00:00Z");
        Assert.Equal("Likely used up",
            LimitStatus.For(l, T("2026-11-12T09:00:00Z"), TimeZoneInfo.Utc, En, PaceDays.Weekdays)!.Text);
    }
```

In `FlyoutModelTests` (it needs `using Sobreiro.Core.Display;` and `Sobreiro.Core.Limits;` if absent):

```csharp
    [Fact]
    public void The_pace_setting_reaches_the_bar_and_the_badge()
    {
        var en = CultureInfo.GetCultureInfo("en-US");
        var at = DateTimeOffset.Parse("2026-11-12T09:00:00Z");
        var weekly = new Limit(Providers.Claude, LimitKind.Weekly, "Weekly", 58,
            DateTimeOffset.Parse("2026-11-16T09:00:00Z"), at, false, TimeSpan.FromDays(7));
        var everyDay = FlyoutModel.From([weekly], at, TimeZoneInfo.Utc, en).Weekly;
        var weekdays = FlyoutModel.From([weekly], at, TimeZoneInfo.Utc, en, days: PaceDays.Weekdays).Weekly;
        Assert.Equal(1.0, everyDay.ProjectedFraction);
        Assert.Equal(StatusKind.Ends, everyDay.Status!.Kind);
        Assert.Equal(0.9667, weekdays.ProjectedFraction!.Value, 0.0001);
        Assert.Equal(StatusKind.OnTrack, weekdays.Status!.Kind);
    }

    [Fact]
    public void The_pace_setting_reaches_copilot_credits()
    {
        var en = CultureInfo.GetCultureInfo("en-US");
        var at = DateTimeOffset.Parse("2026-11-16T00:00:00Z");
        var credits = new Limit(Providers.Copilot, LimitKind.Credits, "AI credits", 40,
            DateTimeOffset.Parse("2026-12-01T00:00:00Z"), at, false, TimeSpan.FromDays(30));
        var view = FlyoutModel.From([credits], at, TimeZoneInfo.Utc, en, days: PaceDays.Weekdays).Copilot!.Credits;
        Assert.Equal(0.84, view.ProjectedFraction!.Value, 0.0001);
    }
```

In `AlertPolicyTests` (add `using Sobreiro.Core.Display;`):

```csharp
    [Fact]
    public void The_alert_body_follows_the_pace_setting()
    {
        var at = new DateTimeOffset(2026, 11, 14, 10, 0, 0, TimeSpan.Zero);   // a Saturday
        var weekly = new Limit("Claude Code", LimitKind.Weekly, "Weekly", 80,
            new DateTimeOffset(2026, 11, 16, 0, 0, 0, TimeSpan.Zero), at, false, TimeSpan.FromDays(7));
        var everyDay = AlertPolicy.Check([weekly], AlertState.Empty, at, TimeZoneInfo.Utc, En).Alerts;
        var weekdays = AlertPolicy.Check([weekly], AlertState.Empty, at, TimeZoneInfo.Utc, En, PaceDays.Weekdays).Alerts;
        Assert.StartsWith("Ends ", Assert.Single(everyDay).Body);
        Assert.StartsWith("On track · ", Assert.Single(weekdays).Body);
    }
```

- [ ] **Step 2: Run them and see them fail**

Run: `dotnet test Sobreiro.slnx --filter "FullyQualifiedName~LimitStatusTests|FullyQualifiedName~FlyoutModelTests|FullyQualifiedName~AlertPolicyTests"`
Expected: build errors, no overload takes a `PaceDays`.

- [ ] **Step 3: Add the argument to the three methods and pass it down**

`LimitStatus.For` calls `Pace.Project(limit, days, zone)`. `FlyoutModel.From` hands `days` to
its private `View`, which uses it for both its own `Pace.Project` call and `LimitStatus.For`.
`AlertPolicy.Check` hands it to `LimitStatus.For`. Nothing else changes.

- [ ] **Step 4: Run the whole suite**

Run: `dotnet test Sobreiro.slnx`
Expected: all pass, the six new tests among them, and no existing test edited.

- [ ] **Step 5: Commit**

```bash
git add src/Sobreiro.Core/Display/LimitStatus.cs src/Sobreiro.Core/Display/FlyoutModel.cs src/Sobreiro.Core/Alerts/AlertPolicy.cs tests/Sobreiro.Tests/LimitStatusTests.cs tests/Sobreiro.Tests/FlyoutModelTests.cs tests/Sobreiro.Tests/AlertPolicyTests.cs
git commit -m "Carry the pace setting to the badge, the bars and the alerts"
```

---

### Task 4: The setting in config.json

**Files:**
- Modify: `src/Sobreiro.Core/Config/AppConfig.cs`
- Test: `tests/Sobreiro.Tests/AppConfigTests.cs` (add to it)

**Interfaces:**
- Consumes: `PaceDays` (Task 2).
- Produces: `public PaceDays PaceDays { get; init; } = PaceDays.EveryDay;` on `AppConfig`,
  read from and written to the key `pace`, the way `Show` uses `show`.

- [ ] **Step 1: Write the failing tests**

```csharp
    [Fact] public void Pace_defaults_to_every_day() => Assert.Equal(PaceDays.EveryDay, AppConfig.Load(PathFor()).PaceDays);

    [Theory]
    [InlineData("\"weekdays\"", PaceDays.Weekdays)]
    [InlineData("\"Weekdays\"", PaceDays.Weekdays)]
    [InlineData("\"everyday\"", PaceDays.EveryDay)]
    [InlineData("\"workdays\"", PaceDays.EveryDay)]
    [InlineData("5", PaceDays.EveryDay)]
    [InlineData("null", PaceDays.EveryDay)]
    public void Pace_loads_and_unknown_values_fall_back(string json, PaceDays expected)
    {
        var loaded = AppConfig.Load(Write("{\"pace\":" + json + "}"));
        Assert.Equal(expected, loaded.PaceDays);
        Assert.DoesNotContain("pace", loaded.Extra.Keys);
    }

    [Fact]
    public void Pace_round_trips_and_is_written_lower_case()
    {
        (AppConfig.Default with { PaceDays = PaceDays.Weekdays }).Save(PathFor());
        Assert.Contains("\"pace\": \"weekdays\"", File.ReadAllText(PathFor()));
        Assert.Equal(PaceDays.Weekdays, AppConfig.Load(PathFor()).PaceDays);
    }
```

Also add `Assert.Equal(PaceDays.EveryDay, c.PaceDays);` to the file's `AssertDefault` helper.

- [ ] **Step 2: Run them and see them fail**

Run: `dotnet test Sobreiro.slnx --filter "FullyQualifiedName~AppConfigTests"`
Expected: a build error, `AppConfig` has no `PaceDays`.

- [ ] **Step 3: Add the property, the `pace` case in `Load`, and the line in `ToJson`**

`ToJson` writes `pace` after `show`, always, lower case, as it does `show`. If an existing test
compares the whole saved text, add that one line to its expected text and change nothing else.

- [ ] **Step 4: Run them and see them pass**

Run: `dotnet test Sobreiro.slnx --filter "FullyQualifiedName~AppConfigTests"`
Expected: all pass, 8 of them new (one fact, six theory rows, one fact).

- [ ] **Step 5: Commit**

```bash
git add src/Sobreiro.Core/Config/AppConfig.cs tests/Sobreiro.Tests/AppConfigTests.cs
git commit -m "Keep the pace setting in config.json"
```

---

### Task 5: The Pace menu, the tray wiring and the docs

**Files:**
- Modify: `src/Sobreiro/TrayMenu.cs` (fields near line 27, constructor lines 43-63, `OnOpening` line 97, setters line 129)
- Modify: `src/Sobreiro/TrayController.cs` (lines 256, 345, 350-352)
- Modify: `docs/plan.md` (the Flyout and Menu bullets, lines 102-121), `site/manual/index.html` (after line 121)

**Interfaces:**
- Consumes: `AppConfig.PaceDays` (Task 4); the `days` arguments of `FlyoutModel.From` and `AlertPolicy.Check` (Task 3).

- [ ] **Step 1: Add the menu**

In `TrayMenu`, beside `_shows`: a `_paces` array of `(ToolStripMenuItem Item, PaceDays Days)`
with "Every day" (`PaceDays.EveryDay`) then "Weekdays only" (`PaceDays.Weekdays`); a
`new ToolStripMenuItem("Pace", null, …)` submenu straight after "Bar colour"; the tick set in
`OnOpening` from `config.PaceDays`; and
`private void SetPace(PaceDays days) => ChangeConfig(c => c with { PaceDays = days });`.

- [ ] **Step 2: Pass the setting from the tray**

In `TrayController`: `FlyoutModel.From(…, showClaude, config.PaceDays)` at line 345;
`CheckAlerts` takes the `AppConfig` it is called beside at line 256 and passes
`config.PaceDays` to `AlertPolicy.Check`. `Snapshot.cs` and `Media/` stay as they are.

- [ ] **Step 3: Update the docs**

`docs/plan.md`, in the Flyout bullet, after the sentence that lists the badges:

> With Pace › Weekdays only (2026-11-12; Every day is the default) the projection counts Monday
> to Friday for windows longer than a day, so the badge, the tick and the alert text skip
> weekends ([design](superpowers/specs/2026-11-12-working-day-pace-design.md)).

In the Menu bullet, add "Pace" after "Bar colour".

`site/manual/index.html`, a new row straight after the "Bar colour" row:

```html
    <div class="row"><span class="k"></span><span class="n">Pace</span><p class="s">Every day, or Weekdays only. With Weekdays only, the weekly limit and Copilot's credits are judged over Monday to Friday, so two days off don't make you look ahead or behind. The session limit is judged the same either way.</p></div>
```

- [ ] **Step 4: Build and run everything**

Run: `dotnet build Sobreiro.slnx`
Expected: Build succeeded, with no new warnings.

Run: `dotnet test Sobreiro.slnx`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/Sobreiro/TrayMenu.cs src/Sobreiro/TrayController.cs docs/plan.md site/manual/index.html
git commit -m "Add the Pace menu: every day or weekdays only"
```

- [ ] **Step 6: Say what was not checked**

The menu and the live badge are not covered by a test. Report that they need a look in the
running app: Pace shows after Bar colour, the tick follows the choice, the badge changes when it
is switched, and `config.json` gains `"pace": "weekdays"`.
