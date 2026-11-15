using Sobreiro.Core.Usage;

namespace Sobreiro.Tests;

public class ResetPeriodTests
{
    private static readonly DateTimeOffset T = DateTimeOffset.Parse("2026-11-09T10:00:00Z");

    [Fact]
    public void Both_null_is_same() => Assert.True(ResetPeriod.Same(null, null));

    [Fact]
    public void One_null_is_not_same()
    {
        Assert.False(ResetPeriod.Same(null, T));
        Assert.False(ResetPeriod.Same(T, null));
    }

    [Fact]
    public void Fifty_nine_minutes_apart_is_same()
    {
        Assert.True(ResetPeriod.Same(T, T.AddMinutes(59)));
        Assert.True(ResetPeriod.Same(T.AddMinutes(59), T));
    }

    [Fact]
    public void Sixty_one_minutes_apart_is_not_same() => Assert.False(ResetPeriod.Same(T, T.AddMinutes(61)));
}
