using System.Globalization;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;

namespace Sobreiro.Tests;

public class LimitStatusTests
{
    private static readonly DateTimeOffset Now = DateTimeOffset.Parse("2026-11-09T10:00:00Z");
    private static readonly CultureInfo En = CultureInfo.GetCultureInfo("en-US");

    private static Limit Make(double pct, TimeSpan? window, TimeSpan? resetsIn, bool isReset = false) =>
        new("Claude Code", LimitKind.Session, "Session", pct, resetsIn is { } r ? Now + r : null, Now, isReset, window);

    private static LimitStatus? For(Limit l) => LimitStatus.For(l, Now, TimeZoneInfo.Utc, En);

    private static readonly TimeSpan Five = TimeSpan.FromHours(5);

    [Fact]
    public void On_track()
    {
        var s = For(Make(27, Five, TimeSpan.FromMinutes(191)))!;
        Assert.Equal(StatusKind.OnTrack, s.Kind);
        Assert.Equal("On track", s.Text);
    }

    [Fact]
    public void Weekly_ends_with_day_name()
    {
        var s = For(Make(74, TimeSpan.FromDays(7), TimeSpan.FromHours(85)))!;
        Assert.Equal(StatusKind.Ends, s.Kind);
        Assert.Equal("Ends Tue 3:09 PM", s.Text);
    }

    [Fact]
    public void Session_ends_today_without_day_name()
    {
        var s = For(Make(83, Five, Five - TimeSpan.FromMinutes(150)))!;
        Assert.Equal(StatusKind.Ends, s.Kind);
        Assert.Equal("Ends 10:30 AM", s.Text);
    }

    [Fact]
    public void Used_up() => Assert.Equal(new LimitStatus(StatusKind.UsedUp, "Used up"), For(Make(100, Five, TimeSpan.FromHours(2))));

    [Fact]
    public void Not_used_yet_under_two_percent() =>
        Assert.Equal(new LimitStatus(StatusKind.NotUsedYet, "Not used yet"), For(Make(1, Five, TimeSpan.FromHours(2))));

    [Fact]
    public void Not_used_yet_after_a_reset() =>
        Assert.Equal(StatusKind.NotUsedYet, For(Make(0, Five, TimeSpan.FromHours(-1), isReset: true))!.Kind);

    [Fact]
    public void Too_early() =>
        Assert.Equal(new LimitStatus(StatusKind.TooEarly, "Too early"), For(Make(5, Five, Five - TimeSpan.FromMinutes(12))));

    [Fact]
    public void Resetting_when_reset_is_seconds_away() =>
        Assert.Equal(new LimitStatus(StatusKind.Resetting, "Resetting"), For(Make(90, Five, TimeSpan.FromSeconds(20))));

    [Fact]
    public void Stale_reading_whose_run_out_has_passed_is_likely_used_up()
    {
        // Observed 2 h ago, 2 h into the window, at 83%: runs out about 25 min after the reading, long before now.
        var l = Make(83, Five, TimeSpan.FromHours(1)) with { ObservedAt = Now.AddHours(-2) };
        Assert.Equal(new LimitStatus(StatusKind.Ends, "Likely used up"), For(l));
    }

    [Fact]
    public void Exactly_two_percent_is_not_not_used_yet() =>
        Assert.NotEqual(StatusKind.NotUsedYet, For(Make(2, Five, TimeSpan.FromHours(2)))!.Kind);

    [Fact]
    public void Just_under_a_hundred_is_not_used_up() =>
        Assert.NotEqual(StatusKind.UsedUp, For(Make(99.9, Five, TimeSpan.FromHours(2)))!.Kind);

    [Fact]
    public void Exactly_a_hundred_is_used_up() =>
        Assert.Equal(StatusKind.UsedUp, For(Make(100, Five, TimeSpan.FromHours(2)))!.Kind);

    [Fact]
    public void Reset_exactly_thirty_seconds_away_is_not_resetting() =>
        Assert.NotEqual(StatusKind.Resetting, For(Make(90, Five, TimeSpan.FromSeconds(30)))!.Kind);

    [Fact]
    public void Reset_twenty_seconds_away_is_resetting() =>
        Assert.Equal(StatusKind.Resetting, For(Make(90, Five, TimeSpan.FromSeconds(20)))!.Kind);

    [Fact]
    public void Is_reset_never_reads_resetting() =>
        Assert.Equal(StatusKind.NotUsedYet, For(Make(0, Five, TimeSpan.FromSeconds(10), isReset: true))!.Kind);

    [Fact]
    public void No_window_gives_no_badge() => Assert.Null(For(Make(30, null, TimeSpan.FromHours(2))));

    [Fact]
    public void No_reset_time_gives_no_badge() => Assert.Null(For(Make(30, Five, null)));
}

public class BadgeColourTests
{
    [Theory]
    [InlineData(true, StatusKind.OnTrack, "#1A7F37", "#FFFFFF")]
    [InlineData(false, StatusKind.OnTrack, "#1A7F37", "#FFFFFF")]
    [InlineData(true, StatusKind.Ends, "#FFB020", "#0B0B0B")]
    [InlineData(false, StatusKind.Ends, "#A35F00", "#FFFFFF")]
    [InlineData(true, StatusKind.UsedUp, "#FF6B6B", "#0B0B0B")]
    [InlineData(false, StatusKind.UsedUp, "#C4262E", "#FFFFFF")]
    [InlineData(true, StatusKind.NotUsedYet, "#262626", "#F4F4F4")]
    [InlineData(true, StatusKind.TooEarly, "#262626", "#F4F4F4")]
    [InlineData(true, StatusKind.Resetting, "#262626", "#F4F4F4")]
    [InlineData(false, StatusKind.NotUsedYet, "#E4E4E4", "#0D0D0D")]
    [InlineData(false, StatusKind.TooEarly, "#E4E4E4", "#0D0D0D")]
    [InlineData(false, StatusKind.Resetting, "#E4E4E4", "#0D0D0D")]
    public void Values(bool dark, StatusKind kind, string fill, string text)
    {
        var b = NightPalette.For(dark).Badge(kind);
        Assert.Equal(fill, b.Fill.ToHex());
        Assert.Equal(text, b.Text.ToHex());
    }

    [Fact]
    public void Text_on_fill_is_readable_in_both_palettes()
    {
        foreach (var dark in new[] { true, false })
            foreach (var kind in Enum.GetValues<StatusKind>())
            {
                var b = NightPalette.For(dark).Badge(kind);
                Assert.True(Contrast.Ratio(b.Text, b.Fill) >= 4.5, $"{kind} dark={dark}");
            }
    }
}
