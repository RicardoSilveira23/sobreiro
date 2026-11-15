using System.Globalization;
using Sobreiro.Core.Alerts;
using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;

namespace Sobreiro.Tests;

public class AlertPolicyTests
{
    private static readonly DateTimeOffset Now = new(2026, 11, 9, 10, 0, 0, TimeSpan.Zero);
    private static readonly CultureInfo En = CultureInfo.GetCultureInfo("en-US");

    private static Limit Make(LimitKind kind, double percent, DateTimeOffset? resets, bool isReset = false) =>
        new("Claude Code", kind, kind.ToString(), percent, resets, Now, isReset);

    private static Limit Session(double percent, TimeSpan? resetsIn = null, bool isReset = false, TimeSpan? window = null) =>
        Make(LimitKind.Session, percent, Now + (resetsIn ?? TimeSpan.FromHours(3)), isReset) with { Window = window };

    private static (IReadOnlyList<Alert> Alerts, AlertState Next) Check(AlertState state, params Limit[] limits) =>
        AlertPolicy.Check(limits, state, Now, TimeZoneInfo.Utc, En);

    [Fact]
    public void Thresholds_are_80_and_95() => Assert.Equal(new[] { 80, 95 }, AlertPolicy.Thresholds);

    [Fact]
    public void Below_80_gives_no_alert()
    {
        var (alerts, next) = Check(AlertState.Empty, Session(79));
        Assert.Empty(alerts);
        Assert.Same(AlertState.Empty, next);
    }

    [Fact]
    public void Reaching_80_alerts_once_with_title_and_body_and_marks_the_period()
    {
        var (alerts, next) = Check(AlertState.Empty, Session(80, window: TimeSpan.FromHours(24)));
        var alert = Assert.Single(alerts);
        Assert.Equal(LimitKind.Session, alert.Kind);
        Assert.Equal(80, alert.Threshold);
        Assert.Equal("Session 80% used", alert.Title);
        Assert.Equal("On track · resets in 3h", alert.Body);
        Assert.Equal(new AlertMark(Now.AddHours(3), 80), next.Marks[LimitKind.Session]);
    }

    [Fact]
    public void Body_without_a_status_is_just_the_resets_in_text()
    {
        var (alerts, _) = Check(AlertState.Empty, Session(80));
        Assert.Equal("resets in 3h", Assert.Single(alerts).Body);
    }

    [Fact]
    public void Body_with_a_run_out_time()
    {
        var (alerts, _) = Check(AlertState.Empty, Session(90, window: TimeSpan.FromHours(5)));
        Assert.Equal("Ends 10:13 AM · resets in 3h", Assert.Single(alerts).Body);
    }

    [Fact]
    public void Title_uses_the_floored_percent()
    {
        var (alerts, _) = Check(AlertState.Empty, Session(83.9));
        Assert.Equal("Session 83% used", Assert.Single(alerts).Title);
    }

    [Fact]
    public void Same_period_at_85_does_not_alert_again_and_state_is_unchanged()
    {
        var (_, marked) = Check(AlertState.Empty, Session(80));
        var (alerts, next) = Check(marked, Session(85));
        Assert.Empty(alerts);
        Assert.Same(marked, next);
    }

    [Fact]
    public void Reaching_95_alerts_after_80()
    {
        var (_, marked) = Check(AlertState.Empty, Session(80));
        var (alerts, next) = Check(marked, Session(96));
        var alert = Assert.Single(alerts);
        Assert.Equal(95, alert.Threshold);
        Assert.Equal(95, next.Marks[LimitKind.Session].Threshold);
    }

    [Fact]
    public void Jumping_from_50_to_97_gives_a_single_95_alert()
    {
        var (_, low) = Check(AlertState.Empty, Session(50));
        var (alerts, _) = Check(low, Session(97));
        Assert.Equal(95, Assert.Single(alerts).Threshold);
    }

    [Fact]
    public void A_new_period_alerts_again()
    {
        var (_, marked) = Check(AlertState.Empty, Session(90));
        var (alerts, next) = Check(marked, Session(82, TimeSpan.FromHours(9)));
        Assert.Equal(80, Assert.Single(alerts).Threshold);
        Assert.Equal(Now.AddHours(9), next.Marks[LimitKind.Session].ResetsAt);
    }

    [Fact]
    public void A_reset_limit_gives_no_alert()
    {
        var (alerts, _) = Check(AlertState.Empty, Session(90, isReset: true));
        Assert.Empty(alerts);
    }

    [Fact]
    public void A_limit_with_neither_status_nor_resets_in_has_an_empty_body()
    {
        var (alerts, _) = Check(AlertState.Empty, Make(LimitKind.Session, 90, null));
        Assert.Equal("", Assert.Single(alerts).Body);
    }

    [Fact]
    public void A_limit_without_a_reset_time_alerts_with_an_empty_body()
    {
        var (alerts, _) = Check(AlertState.Empty, Make(LimitKind.Spend, 85, null));
        Assert.Equal("", Assert.Single(alerts).Body);
    }

    [Fact]
    public void Several_alerts_come_highest_threshold_first_then_by_kind()
    {
        var weekly = Make(LimitKind.Weekly, 97, Now.AddDays(3));
        var spend = Make(LimitKind.Spend, 85, Now.AddDays(10));
        var (alerts, _) = Check(AlertState.Empty, spend, Session(85), weekly);
        Assert.Equal(new[] { LimitKind.Weekly, LimitKind.Session, LimitKind.Spend }, alerts.Select(a => a.Kind));
    }

    [Fact]
    public void Weekly_and_session_both_over_80_give_session_first()
    {
        var (alerts, _) = Check(AlertState.Empty, Make(LimitKind.Weekly, 85, Now.AddDays(3)), Session(85));
        Assert.Equal(new[] { LimitKind.Session, LimitKind.Weekly }, alerts.Select(a => a.Kind));
    }

    private static Limit Credits(double percent, bool isReset = false) =>
        new("GitHub Copilot", LimitKind.Credits, "AI credits", percent, Now.AddDays(20), Now, isReset);

    [Fact]
    public void Credits_at_80_toasts_with_copilot_title()
    {
        var (alerts, next) = Check(AlertState.Empty, Credits(80));
        var alert = Assert.Single(alerts);
        Assert.Equal(LimitKind.Credits, alert.Kind);
        Assert.Equal("Copilot AI credits 80% used", alert.Title);
        Assert.Contains(LimitKind.Credits, next.Marks.Keys);
    }

    [Fact]
    public void Reset_credits_never_toast() => Assert.Empty(Check(AlertState.Empty, Credits(90, isReset: true)).Alerts);

    [Fact]
    public void Claude_titles_unchanged() =>
        Assert.Equal("Session 80% used", Assert.Single(Check(AlertState.Empty, Session(80)).Alerts).Title);

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
}
