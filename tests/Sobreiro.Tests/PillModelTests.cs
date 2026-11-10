using Sobreiro.Core.Copilot;
using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;

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

    [Fact]
    public void Pill_hides_claude_when_told()
    {
        Assert.False(PillModel.From([Credits()], TimeZoneInfo.Utc, false).ShowClaude);
        Assert.True(PillModel.From([Credits()], TimeZoneInfo.Utc, true).ShowClaude);
    }
}
