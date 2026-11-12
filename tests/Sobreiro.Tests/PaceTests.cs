using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;

namespace Sobreiro.Tests;

public class PaceTests
{
    private static readonly DateTimeOffset Now = DateTimeOffset.Parse("2026-11-09T10:00:00Z");

    private static Limit Make(double pct, TimeSpan? window, TimeSpan? resetsIn, DateTimeOffset? observed = null, bool isReset = false) =>
        new("Claude Code", LimitKind.Session, "Session", pct, resetsIn is { } r ? Now + r : null, observed ?? Now, isReset, window);

    [Fact]
    public void Session_on_track()
    {
        var p = Pace.Project(Make(27, TimeSpan.FromHours(5), TimeSpan.FromMinutes(191)));
        Assert.NotNull(p);
        Assert.Equal(74.31, p.ProjectedPercent, 0.01);
        Assert.Null(p.RunsOutBeforeReset);
        Assert.Null(p.RunsOutAt);
    }

    [Fact]
    public void Weekly_runs_out()
    {
        var p = Pace.Project(Make(74, TimeSpan.FromDays(7), TimeSpan.FromHours(85)));
        Assert.NotNull(p);
        Assert.Equal(149.78, p.ProjectedPercent, 0.01);
        Assert.Equal(55.84, p.RunsOutBeforeReset!.Value.TotalHours, 1.0 / 60);
        Assert.Equal(Now.AddHours(85) - p.RunsOutBeforeReset.Value, p.RunsOutAt);
    }

    [Fact]
    public void Too_early_is_null() =>
        Assert.Null(Pace.Project(Make(5, TimeSpan.FromHours(5), TimeSpan.FromHours(5) - TimeSpan.FromMinutes(12))));

    [Fact]
    public void Full_is_null() => Assert.Null(Pace.Project(Make(100, TimeSpan.FromHours(5), TimeSpan.FromHours(2))));

    [Fact]
    public void Reset_is_null() => Assert.Null(Pace.Project(Make(0, TimeSpan.FromHours(5), TimeSpan.FromHours(2), isReset: true)));

    [Fact]
    public void No_reset_time_is_null() => Assert.Null(Pace.Project(Make(30, TimeSpan.FromHours(5), null)));

    [Fact]
    public void No_window_is_null() => Assert.Null(Pace.Project(Make(30, null, TimeSpan.FromHours(2))));

    [Fact]
    public void Zero_percent_projects_zero()
    {
        var p = Pace.Project(Make(0, TimeSpan.FromHours(4), TimeSpan.FromHours(2)));
        Assert.Equal(0, p!.ProjectedPercent);
        Assert.Null(p.RunsOutBeforeReset);
    }

    [Fact]
    public void Uses_observed_time_not_now()
    {
        var p = Pace.Project(Make(27, TimeSpan.FromHours(5), TimeSpan.FromMinutes(131), Now.AddMinutes(-60)));
        Assert.Equal(74.31, p!.ProjectedPercent, 0.01);
    }

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
}
