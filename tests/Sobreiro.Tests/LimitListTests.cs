using Sobreiro.Core.Copilot;
using Sobreiro.Core.Connect;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Usage;

namespace Sobreiro.Tests;

public class LimitListTests
{
    private static readonly DateTimeOffset Now = DateTimeOffset.Parse("2026-11-09T12:00:00Z");

    private static UsageSnapshot Snap(RateLimits l) => new(l, Now.AddMinutes(-2));

    [Fact]
    public void From_null_is_empty() => Assert.Empty(LimitList.From(null, Now));

    [Fact]
    public void From_maps_windows_in_order_with_labels()
    {
        var five = Now.AddHours(2);
        var seven = Now.AddDays(5);
        var spend = Now.AddDays(7);
        var list = LimitList.From(Snap(new RateLimits(
            new LimitWindow(8, five), new LimitWindow(44, seven),
            new SpendLimit(62.8, spend, 6.28, 10, "monthly"))), Now);

        Assert.Collection(list,
            l => { Assert.Equal(LimitKind.Session, l.Kind); Assert.Equal("Session", l.Label); Assert.Equal("Claude Code", l.Provider); Assert.Equal(8, l.Percent); Assert.Equal(five, l.ResetsAt); Assert.False(l.IsReset); Assert.Equal(Now.AddMinutes(-2), l.ObservedAt); },
            l => { Assert.Equal(LimitKind.Weekly, l.Kind); Assert.Equal("Weekly", l.Label); Assert.Equal(44, l.Percent); Assert.Equal(seven, l.ResetsAt); },
            l => { Assert.Equal(LimitKind.Spend, l.Kind); Assert.Equal("Spend", l.Label); Assert.Equal(62.8, l.Percent); Assert.Equal(spend, l.ResetsAt); });
    }

    [Fact]
    public void From_marks_passed_window_as_reset_at_zero()
    {
        var resets = DateTimeOffset.Parse("2026-11-09T11:00:00Z");
        var list = LimitList.From(Snap(new RateLimits(new LimitWindow(30, resets), null, null)), Now);

        var l = Assert.Single(list);
        Assert.True(l.IsReset);
        Assert.Equal(0, l.Percent);
        Assert.Equal(resets, l.ResetsAt);
    }

    [Fact]
    public void From_skips_absent_windows()
    {
        var list = LimitList.From(Snap(new RateLimits(null, new LimitWindow(44, Now.AddDays(1)), null)), Now);
        Assert.Equal(LimitKind.Weekly, Assert.Single(list).Kind);
    }

    [Theory]
    [InlineData(8.9, 8)]
    [InlineData(99.6, 99)]
    [InlineData(100, 100)]
    [InlineData(130, 100)]
    [InlineData(-5, 0)]
    public void DisplayPercent_floors_and_clamps(double percent, int expected)
    {
        var l = new Limit("Claude Code", LimitKind.Session, "Session", percent, null, Now, false);
        Assert.Equal(expected, l.DisplayPercent);
    }

    [Fact]
    public void From_uses_the_windows_own_observed_at_when_it_has_one()
    {
        var own = Now.AddHours(-3);
        var list = LimitList.From(Snap(new RateLimits(new LimitWindow(10, Now.AddHours(2), own), new LimitWindow(20, Now.AddDays(2)), null)), Now);

        Assert.Equal(own, list[0].ObservedAt);
        Assert.Equal(Now.AddMinutes(-2), list[1].ObservedAt);
    }

    [Fact]
    public void From_fills_windows_and_spend_fields()
    {
        var resets = DateTimeOffset.Parse("2026-12-01T00:00:00Z");
        var list = LimitList.From(Snap(new RateLimits(
            new LimitWindow(8, Now.AddHours(2)), new LimitWindow(44, Now.AddDays(5)),
            new SpendLimit(63, resets, 31.5, 50, "monthly"))), Now);

        Assert.Equal(TimeSpan.FromHours(5), list[0].Window);
        Assert.Equal(TimeSpan.FromDays(7), list[1].Window);
        Assert.Equal(TimeSpan.FromDays(30), list[2].Window);
        Assert.Equal(31.5, list[2].UsedUsd);
        Assert.Equal(50, list[2].LimitUsd);
        Assert.Equal("monthly", list[2].Period);
    }

    [Theory]
    [InlineData("daily", 1)]
    [InlineData("weekly", 7)]
    [InlineData("yearly", null)]
    [InlineData(null, null)]
    public void Spend_window_follows_the_period(string? period, int? days)
    {
        var list = LimitList.From(Snap(new RateLimits(null, null, new SpendLimit(10, Now.AddDays(1), null, null, period))), Now);
        Assert.Equal(days is { } d ? TimeSpan.FromDays(d) : null, list[0].Window);
    }

    private static CopilotReading Reading(double used = 2644.2, DateTimeOffset? resets = null) =>
        new(15000, used, resets ?? DateTimeOffset.Parse("2026-12-01T00:00:00Z"), Now.AddMinutes(-1), CopilotSource.Live);

    [Fact]
    public void Copilot_reading_becomes_one_credits_limit()
    {
        var l = Assert.Single(LimitList.FromCopilot(Reading(), Now));
        Assert.Equal("GitHub Copilot", l.Provider);
        Assert.Equal(LimitKind.Credits, l.Kind);
        Assert.Equal("AI credits", l.Label);
        Assert.Equal(17.628, l.Percent, 3);
        Assert.Equal(2644.2, l.UsedCredits);
        Assert.Equal(15000, l.CreditBudget);
        Assert.Equal(TimeSpan.FromDays(30), l.Window);
        Assert.Equal(DateTimeOffset.Parse("2026-12-01T00:00:00Z"), l.ResetsAt);
        Assert.False(l.IsReset);
        Assert.Equal(Now.AddMinutes(-1), l.ObservedAt);
    }

    [Fact]
    public void No_copilot_reading_gives_no_limits() => Assert.Empty(LimitList.FromCopilot(null, Now));

    [Fact]
    public void Credits_without_reset_time_have_no_window()
    {
        var l = Assert.Single(LimitList.FromCopilot(Reading() with { ResetsAt = null }, Now));
        Assert.Null(l.Window);
        Assert.False(l.IsReset);
    }

    [Fact]
    public void Passed_reset_reads_zero_and_reset()
    {
        var l = Assert.Single(LimitList.FromCopilot(Reading(resets: DateTimeOffset.Parse("2026-11-01T00:00:00Z")), Now));
        Assert.Equal(0, l.Percent);
        Assert.True(l.IsReset);
    }

    [Theory]
    [InlineData(true, true, true)]
    [InlineData(true, false, true)]
    [InlineData(false, false, true)]
    [InlineData(false, true, false)]
    public void Claude_shows_when_on_pc_or_when_nothing_else_shows(bool onPc, bool copilotShown, bool expected) =>
        Assert.Equal(expected, LimitList.ShowClaude(onPc, copilotShown));

    [Theory]
    [InlineData(false, 0, ConnectionState.NotConnected, false)]
    [InlineData(true, 0, ConnectionState.NotConnected, true)]
    [InlineData(false, 1, ConnectionState.NotConnected, true)]
    [InlineData(false, 0, ConnectionState.Connected, true)]
    [InlineData(false, 0, ConnectionState.Legacy, true)]
    public void Claude_is_on_pc_by_credentials_readings_or_connection(bool file, int count, ConnectionState state, bool expected) =>
        Assert.Equal(expected, LimitList.ClaudeOnPc(file, count, state));
}
