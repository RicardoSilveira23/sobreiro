using Sobreiro.Core.Limits;

namespace Sobreiro.Tests;

public class UsageLevelTests
{
    private static readonly DateTimeOffset Now = DateTimeOffset.Parse("2026-11-09T10:00:00Z");
    private static Limit L(LimitKind k, double pct) => new("Claude Code", k, k.ToString(), pct, null, Now, false);

    [Theory]
    [InlineData(0, UsageLevel.Normal)]
    [InlineData(69, UsageLevel.Normal)]
    [InlineData(70, UsageLevel.Warning)]
    [InlineData(89, UsageLevel.Warning)]
    [InlineData(90, UsageLevel.Error)]
    [InlineData(100, UsageLevel.Error)]
    public void For_maps_thresholds(int pct, UsageLevel expected) => Assert.Equal(expected, UsageLevels.For(pct));

    [Fact]
    public void Highest_picks_worst() =>
        Assert.Equal(UsageLevel.Error, UsageLevels.Highest([L(LimitKind.Session, 72), L(LimitKind.Weekly, 91)]));

    [Fact]
    public void Highest_of_none_is_normal() => Assert.Equal(UsageLevel.Normal, UsageLevels.Highest([]));
}
