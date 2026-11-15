using Sobreiro.Core.Copilot;
using System.Globalization;
using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Usage;

namespace Sobreiro.Tests;

public class TooltipTextTests
{
    private static readonly DateTimeOffset Now = DateTimeOffset.Parse("2026-11-09T12:00:00Z");
    private static readonly DateTimeOffset Observed = DateTimeOffset.Parse("2026-11-09T11:58:00Z");
    private static readonly CultureInfo En = CultureInfo.GetCultureInfo("en-US");

    private static string Render(RateLimits? limits, CultureInfo? culture = null) =>
        TooltipText.Render(limits is null ? [] : LimitList.From(new UsageSnapshot(limits, Observed), Now), Now, TimeZoneInfo.Utc, culture ?? En);

    private static DateTimeOffset T(string s) => DateTimeOffset.Parse(s);

    [Fact]
    public void No_limits() => Assert.Equal("Sobreiro · no data yet", Render(null));

    [Fact]
    public void Session_and_weekly() =>
        Assert.Equal("Session 8% · resets 2:20 PM\nWeekly 44% · resets Sat 12:00 AM\nas of 11:58 AM",
            Render(new RateLimits(new LimitWindow(8, T("2026-11-09T14:20:00Z")), new LimitWindow(44, T("2026-11-14T00:00:00Z")), null)));

    [Fact]
    public void Absent_session_shows_dash() =>
        Assert.StartsWith("Session –\n", Render(new RateLimits(null, new LimitWindow(44, T("2026-11-14T00:00:00Z")), null)));

    [Fact]
    public void Reset_session() =>
        Assert.StartsWith("Session 0% · reset 11:00 AM\n", Render(new RateLimits(new LimitWindow(30, T("2026-11-09T11:00:00Z")), null, null)));

    [Fact]
    public void Spend_line_comes_before_as_of() =>
        Assert.Equal("Session 8% · resets 2:20 PM\nWeekly 44% · resets Sat 12:00 AM\nSpend 62% · resets Mon 16 Nov 12:00 AM\nas of 11:58 AM",
            Render(new RateLimits(new LimitWindow(8, T("2026-11-09T14:20:00Z")), new LimitWindow(44, T("2026-11-14T00:00:00Z")),
                new SpendLimit(62.8, T("2026-11-16T00:00:00Z"), null, null, null))));

    [Fact]
    public void Limit_without_reset_time_shows_only_percentage() =>
        Assert.StartsWith("Session 8%\n", Render(new RateLimits(new LimitWindow(8, null), null, null)));

    [Fact]
    public void Long_culture_pattern_is_truncated()
    {
        var c = (CultureInfo)En.Clone();
        c.DateTimeFormat.ShortTimePattern = "HH:mm:ss 'oclock' zzz 'and then some more words to make it long'";
        var text = Render(new RateLimits(new LimitWindow(8, T("2026-11-09T14:20:00Z")), new LimitWindow(44, T("2026-11-14T00:00:00Z")),
            new SpendLimit(62.8, T("2026-11-16T00:00:00Z"), null, null, null)), c);
        Assert.Equal(TooltipText.MaxLength, text.Length);
    }

    private static readonly DateTimeOffset CopNow = DateTimeOffset.Parse("2026-11-09T12:00:00Z");

    private static Limit Credits(double used = 2644, DateTimeOffset? observed = null) =>
        LimitList.FromCopilot(new CopilotReading(15000, used, DateTimeOffset.Parse("2026-12-01T00:00:00Z"), observed ?? CopNow, CopilotSource.Live), CopNow)[0];

    [Fact]
    public void Claude_tooltip_ignores_copilot()
    {
        var claude = new Limit(Providers.Claude, LimitKind.Session, "Session", 8, null, Observed, false);
        var text = TooltipText.Render([Credits(), claude], Now, TimeZoneInfo.Utc, En);
        Assert.DoesNotContain("AI credits", text);
        Assert.EndsWith("as of 11:58 AM", text);
    }

    [Fact]
    public void Copilot_tooltip_has_three_lines()
    {
        var text = TooltipText.RenderCopilot(Credits(observed: Observed), Now, TimeZoneInfo.Utc, En);
        Assert.Equal("AI credits 17% · resets Tue 1 Dec 12:00 AM\n2,644 of 15,000 credits\nas of 11:58 AM", text);
        Assert.True(text.Length <= TooltipText.MaxLength);
    }
}
