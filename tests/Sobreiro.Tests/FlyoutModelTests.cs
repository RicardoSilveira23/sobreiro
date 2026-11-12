using Sobreiro.Core.Copilot;
using System.Globalization;
using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Live;
using Sobreiro.Core.Usage;

namespace Sobreiro.Tests;

public class FlyoutModelTests
{
    private static readonly DateTimeOffset Now = DateTimeOffset.Parse("2026-11-09T10:00:00Z");
    private static readonly CultureInfo En = CultureInfo.GetCultureInfo("en-US");
    private static DateTimeOffset T(string s) => DateTimeOffset.Parse(s);

    private static FlyoutModel Build(RateLimits? limits, DateTimeOffset? observed = null) =>
        FlyoutModel.From(limits is null ? [] : LimitList.From(new UsageSnapshot(limits, observed ?? Now), Now), Now, TimeZoneInfo.Utc, En);

    private static readonly RateLimits Mockup = new(
        new LimitWindow(27, Now.AddMinutes(191)),
        new LimitWindow(74, Now.AddHours(85)),
        null);

    [Fact]
    public void Mockup_session()
    {
        var s = Build(Mockup).Session;
        Assert.Equal("Session", s.Label);
        Assert.Equal("27%", s.UsedFigure);
        Assert.Equal("73%", s.LeftFigure);
        Assert.Equal("resets in 3h 11m", s.ResetsIn);
        Assert.Equal(new LimitStatus(StatusKind.OnTrack, "On track"), s.Status);
        Assert.Equal(0.7431, s.ProjectedFraction!.Value, 0.0001);
        Assert.Equal(UsageLevel.Normal, s.Level);
    }

    [Fact]
    public void Mockup_weekly()
    {
        var w = Build(Mockup).Weekly;
        Assert.Equal(UsageLevel.Warning, w.Level);
        Assert.Equal("resets in 3d 13h", w.ResetsIn);
        Assert.Equal(new LimitStatus(StatusKind.Ends, "Ends Tue 3:09 PM"), w.Status);
        Assert.Equal(1.0, w.ProjectedFraction);
        Assert.Equal(0.74, w.UsedFraction, 1e-9);
    }

    [Fact]
    public void Missing_weekly()
    {
        var w = Build(new RateLimits(new LimitWindow(27, Now.AddMinutes(191)), null, null)).Weekly;
        Assert.False(w.HasData);
        Assert.Equal("Weekly", w.Label);
        Assert.Equal("–", w.UsedFigure);
        Assert.Equal("–", w.LeftFigure);
        Assert.Equal("No reading yet", w.ResetsIn);
        Assert.Null(w.Status);
        Assert.Equal(0, w.UsedFraction);
    }

    [Fact]
    public void Reset_session()
    {
        var s = Build(new RateLimits(new LimitWindow(60, T("2026-11-09T09:00:00Z")), null, null)).Session;
        Assert.Equal("0%", s.UsedFigure);
        Assert.Equal("reset at 9:00 AM", s.ResetsIn);
        Assert.Equal(StatusKind.NotUsedYet, s.Status!.Kind);
        Assert.Null(s.ProjectedFraction);
    }

    [Fact]
    public void Window_without_reset_time_has_no_reset_text()
    {
        var s = Build(new RateLimits(new LimitWindow(20, null), null, null)).Session;
        Assert.True(s.HasData);
        Assert.Null(s.ResetsIn);
        Assert.Null(s.Status);
    }

    [Fact]
    public void Spend_monthly_with_amounts()
    {
        var spend = new SpendLimit(63, T("2026-12-01T00:00:00Z"), 31.5, 50, "monthly");
        var v = Build(new RateLimits(null, null, spend)).Spend;
        Assert.Equal("Spend", v!.Label);
        Assert.Equal("$31.50 of $50.00 this month", v.Detail);
    }

    [Fact]
    public void Spend_daily_only_used() =>
        Assert.Equal("$1,234.50 spent today",
            Build(new RateLimits(null, null, new SpendLimit(10, null, 1234.5, null, "daily"))).Spend!.Detail);

    [Fact]
    public void Spend_without_amounts() =>
        Assert.Null(Build(new RateLimits(null, null, new SpendLimit(10, null, null, null, null))).Spend!.Detail);

    [Fact]
    public void No_spend() => Assert.Null(Build(Mockup).Spend);

    [Theory]
    [InlineData(20, "Updated just now")]
    [InlineData(180, "Updated 3 min ago")]
    [InlineData(2 * 3600 + 5 * 60, "Updated 2h 5m ago")]
    [InlineData(3 * 86400, "Updated Fri 10:00 AM")]
    public void Footer_age(int secondsAgo, string expected) =>
        Assert.Equal(expected, Build(Mockup, Now.AddSeconds(-secondsAgo)).Footer);

    [Fact]
    public void Footer_without_limits() =>
        Assert.Equal("No numbers yet. They arrive with Claude Code's next response.", Build(null).Footer);

    [Theory]
    [InlineData(LiveState.Ok, "No numbers yet.")]
    [InlineData(LiveState.NoSignIn, "No numbers yet. Live usage needs Claude Code to be signed in.")]
    [InlineData(LiveState.Refused, "No numbers yet. Anthropic didn't accept Claude Code's sign-in. Live usage waits for a new one.")]
    [InlineData(LiveState.Failing, "No numbers yet. Live usage can't reach Anthropic.")]
    public void Footer_without_limits_with_live_usage_on(LiveState live, string expected) =>
        Assert.Equal(expected, FlyoutModel.From(LimitList.From(null, Now), Now, TimeZoneInfo.Utc, En, live).Footer);

    [Fact]
    public void Footer_adds_the_refused_sentence_after_the_age() =>
        Assert.Equal("Updated 3 min ago. Anthropic didn't accept Claude Code's sign-in. Live usage waits for a new one.",
            FlyoutModel.From(LimitList.From(new UsageSnapshot(Mockup, Now.AddSeconds(-180)), Now), Now, TimeZoneInfo.Utc, En, LiveState.Refused).Footer);

    [Fact]
    public void Footer_adds_the_live_sentence() =>
        Assert.Equal("Updated 3 min ago. Live usage is paused until Claude Code renews its sign-in.",
            FlyoutModel.From(LimitList.From(new UsageSnapshot(Mockup, Now.AddSeconds(-180)), Now), Now, TimeZoneInfo.Utc, En, LiveState.Expired).Footer);

    [Fact]
    public void Footer_has_no_live_sentence_when_ok() =>
        Assert.Equal("Updated just now",
            FlyoutModel.From(LimitList.From(new UsageSnapshot(Mockup, Now), Now), Now, TimeZoneInfo.Utc, En, LiveState.Ok).Footer);

    [Fact]
    public void Title() => Assert.Equal("Claude Code", Build(null).Title);

    private static readonly DateTimeOffset CopNow = DateTimeOffset.Parse("2026-11-09T12:00:00Z");

    private static Limit Credits(double used = 2644, DateTimeOffset? observed = null) =>
        LimitList.FromCopilot(new CopilotReading(15000, used, DateTimeOffset.Parse("2026-12-01T00:00:00Z"), observed ?? CopNow, CopilotSource.Live), CopNow)[0];

    private static Limit ClaudeSession(DateTimeOffset observed) =>
        new(Providers.Claude, LimitKind.Session, "Session", 27, null, observed, false);

    [Fact]
    public void Flyout_copilot_section_has_detail_and_note()
    {
        var m = FlyoutModel.From([Credits()], CopNow, TimeZoneInfo.Utc, En, copilotNote: "Can't reach GitHub.");
        Assert.Equal("2,644 of 15,000 credits", m.Copilot!.Credits.Detail);
        Assert.Equal("Can't reach GitHub.", m.Copilot.Note);
        Assert.Equal("17%", m.Copilot.Credits.UsedFigure);
    }

    [Fact]
    public void Flyout_reset_credits_show_no_stale_detail()
    {
        var reset = LimitList.FromCopilot(new CopilotReading(15000, 2644, CopNow.AddHours(-1), CopNow.AddDays(-3), CopilotSource.Live), CopNow)[0];
        Assert.Null(FlyoutModel.From([reset], CopNow, TimeZoneInfo.Utc, En).Copilot!.Credits.Detail);
    }

    [Fact]
    public void Flyout_title_stays_claude_with_copilot_first_in_list()
    {
        var m = FlyoutModel.From([Credits(), ClaudeSession(CopNow)], CopNow, TimeZoneInfo.Utc, En);
        Assert.Equal("Claude Code", m.Title);
        Assert.Equal("27%", m.Session.UsedFigure);
        Assert.NotNull(m.Copilot);
    }

    [Fact]
    public void Flyout_footer_ignores_copilot_age_when_claude_shown()
    {
        var claude = ClaudeSession(CopNow.AddHours(-3));
        var with = FlyoutModel.From([claude, Credits()], CopNow, TimeZoneInfo.Utc, En);
        var without = FlyoutModel.From([claude], CopNow, TimeZoneInfo.Utc, En);
        Assert.Equal(without.Footer, with.Footer);
        Assert.NotEqual("Updated just now", with.Footer);
    }

    [Fact]
    public void Flyout_copilot_only_footer_uses_copilot_age()
    {
        var m = FlyoutModel.From([Credits(observed: CopNow.AddMinutes(-2))], CopNow, TimeZoneInfo.Utc, En, LiveState.Ok, showClaude: false);
        Assert.Equal("Updated 2 min ago", m.Footer);
        Assert.False(m.ShowClaude);
    }

    [Fact]
    public void The_pace_setting_reaches_the_bar_and_the_badge()
    {
        var at = DateTimeOffset.Parse("2026-11-12T09:00:00Z");
        var weekly = new Limit(Providers.Claude, LimitKind.Weekly, "Weekly", 58,
            DateTimeOffset.Parse("2026-11-16T09:00:00Z"), at, false, TimeSpan.FromDays(7));
        var everyDay = FlyoutModel.From([weekly], at, TimeZoneInfo.Utc, En).Weekly;
        var weekdays = FlyoutModel.From([weekly], at, TimeZoneInfo.Utc, En, days: PaceDays.Weekdays).Weekly;
        Assert.Equal(1.0, everyDay.ProjectedFraction);
        Assert.Equal(StatusKind.Ends, everyDay.Status!.Kind);
        Assert.Equal(0.9667, weekdays.ProjectedFraction!.Value, 0.0001);
        Assert.Equal(StatusKind.OnTrack, weekdays.Status!.Kind);
    }

    [Fact]
    public void The_pace_setting_reaches_copilot_credits()
    {
        var at = DateTimeOffset.Parse("2026-11-16T00:00:00Z");
        var credits = new Limit(Providers.Copilot, LimitKind.Credits, "AI credits", 40,
            DateTimeOffset.Parse("2026-12-01T00:00:00Z"), at, false, TimeSpan.FromDays(30));
        var view = FlyoutModel.From([credits], at, TimeZoneInfo.Utc, En, days: PaceDays.Weekdays).Copilot!.Credits;
        Assert.Equal(0.84, view.ProjectedFraction!.Value, 0.0001);
    }
}
