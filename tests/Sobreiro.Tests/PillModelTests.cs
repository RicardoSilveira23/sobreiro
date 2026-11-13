using Sobreiro.Core.Copilot;
using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Live;
using Sobreiro.Core.Polling;

namespace Sobreiro.Tests;

public class PillModelTests
{
    private static readonly DateTimeOffset Now = DateTimeOffset.Parse("2026-11-09T12:00:00Z");

    private static Limit L(LimitKind kind, double pct) => new("Claude Code", kind, kind.ToString(), pct, null, Now, false);

    [Fact]
    public void Both_windows()
    {
        var m = PillModel.From([L(LimitKind.Session, 37.9), L(LimitKind.Weekly, 61)]);
        Assert.Equal(new PillLine("5h", "37%"), m.Session);
        Assert.Equal(new PillLine("7d", "61%"), m.Weekly);
    }

    [Fact]
    public void Missing_windows_read_a_dash()
    {
        var m = PillModel.From([]);
        Assert.Equal(new PillLine("5h", "–"), m.Session);
        Assert.Equal(new PillLine("7d", "–"), m.Weekly);
    }

    [Fact]
    public void Full_reads_100() => Assert.Equal("100%", PillModel.From([L(LimitKind.Session, 100)]).Session.Figure);

    [Fact]
    public void Rings_match_the_rings_model()
    {
        IReadOnlyList<Limit> limits = [L(LimitKind.Session, 74), L(LimitKind.Weekly, 93)];
        Assert.Equal(RingsModel.From(limits), PillModel.From(limits).Rings);
    }

    private static readonly DateTimeOffset CopNow = DateTimeOffset.Parse("2026-11-09T12:00:00Z");

    private static Limit Credits(double used = 2644, DateTimeOffset? observed = null) =>
        LimitList.FromCopilot(new CopilotReading(15000, used, DateTimeOffset.Parse("2026-12-01T00:00:00Z"), observed ?? CopNow, CopilotSource.Live), CopNow)[0];

    [Fact]
    public void Pill_has_copilot_block_when_credits_present()
    {
        var c = PillModel.From([Credits()], TimeZoneInfo.Utc, true).Copilot!;
        Assert.Equal(new PillLine("AI", "17%"), c.Credits);
        Assert.Equal("1 Dec", c.ResetDate);
        Assert.Equal(.176, c.Fraction, 3);
    }

    [Fact]
    public void Pill_reset_date_is_local() =>
        Assert.Equal("30 Nov", PillModel.From([Credits()], TimeZoneInfo.CreateCustomTimeZone("m5", TimeSpan.FromHours(-5), "m5", "m5"), true).Copilot!.ResetDate);

    [Fact]
    public void Pill_without_credits_has_no_copilot_block() =>
        Assert.Null(PillModel.From([L(LimitKind.Session, 10)], TimeZoneInfo.Utc, true).Copilot);

    private static readonly DateTimeOffset WordsNow = DateTimeOffset.Parse("2026-11-13T12:00:00Z");

    private static Limit Aged(LimitKind kind, TimeSpan age) =>
        new("Claude Code", kind, kind.ToString(), 40, null, WordsNow - age, false);

    [Fact]
    public void Stopped_claude_block_carries_the_words_and_empty_rings()
    {
        var m = PillModel.From([Aged(LimitKind.Session, TimeSpan.FromMinutes(20)), Aged(LimitKind.Weekly, TimeSpan.FromMinutes(20))],
            TimeZoneInfo.Utc, true, ProviderState.Expired, 2, ProviderState.Ok, 0, WordsNow);
        Assert.Equal(new PillWords("sign-in", "expired"), m.ClaudeWords);
        Assert.Equal(RingsModel.From([]), m.Rings);
    }

    [Fact]
    public void One_missed_poll_keeps_the_figures()
    {
        var credits = LimitList.FromCopilot(new CopilotReading(15000, 2644, DateTimeOffset.Parse("2026-12-01T00:00:00Z"),
            WordsNow - TimeSpan.FromDays(3), CopilotSource.Live), WordsNow)[0];
        IReadOnlyList<Limit> limits = [Aged(LimitKind.Session, TimeSpan.FromHours(3)), Aged(LimitKind.Weekly, TimeSpan.FromHours(3)), credits];
        var m = PillModel.From(limits, TimeZoneInfo.Utc, true, ProviderState.Offline, 1, ProviderState.Offline, 1, WordsNow);
        Assert.Null(m.ClaudeWords);
        Assert.Null(m.Copilot!.Words);
    }

    [Fact]
    public void Fresh_claude_block_has_no_words()
    {
        IReadOnlyList<Limit> limits = [Aged(LimitKind.Session, TimeSpan.FromMinutes(1)), Aged(LimitKind.Weekly, TimeSpan.FromMinutes(1))];
        var m = PillModel.From(limits, TimeZoneInfo.Utc, true, ProviderState.Expired, 2, ProviderState.Ok, 0, WordsNow);
        Assert.Null(m.ClaudeWords);
        Assert.Equal(RingsModel.From(limits), m.Rings);
    }

    [Fact]
    public void Stopped_copilot_block_carries_the_words_and_an_empty_bar()
    {
        var credits = LimitList.FromCopilot(new CopilotReading(15000, 2644, DateTimeOffset.Parse("2026-12-01T00:00:00Z"),
            WordsNow - TimeSpan.FromHours(25), CopilotSource.Live), WordsNow)[0];
        var c = PillModel.From([credits], TimeZoneInfo.Utc, true, ProviderState.Off, 0, ProviderState.Offline, 2, WordsNow).Copilot!;
        Assert.Equal(new PillWords("no", "network"), c.Words);
        Assert.Equal(0, c.Fraction);
    }

    [Fact]
    public void The_three_argument_From_never_has_words()
    {
        var credits = LimitList.FromCopilot(new CopilotReading(15000, 2644, DateTimeOffset.Parse("2026-12-01T00:00:00Z"),
            WordsNow - TimeSpan.FromDays(3), CopilotSource.Live), WordsNow)[0];
        var m = PillModel.From([Aged(LimitKind.Session, TimeSpan.FromDays(3)), credits], TimeZoneInfo.Utc, true);
        Assert.Null(m.ClaudeWords);
        Assert.Null(m.Copilot!.Words);
    }

    [Fact]
    public void Pill_hides_claude_when_told()
    {
        Assert.False(PillModel.From([Credits()], TimeZoneInfo.Utc, false).ShowClaude);
        Assert.True(PillModel.From([Credits()], TimeZoneInfo.Utc, true).ShowClaude);
    }
}
