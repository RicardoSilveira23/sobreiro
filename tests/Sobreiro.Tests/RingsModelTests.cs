using Sobreiro.Core.Copilot;
using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;

namespace Sobreiro.Tests;

public class RingsModelTests
{
    private static readonly DateTimeOffset Now = DateTimeOffset.Parse("2026-11-09T12:00:00Z");

    private static Limit L(LimitKind kind, double pct) => new("Claude Code", kind, kind.ToString(), pct, null, Now, false);

    [Fact]
    public void No_limits_gives_two_empty_rings() =>
        Assert.Equal(new RingsModel(Ring.None, Ring.None), RingsModel.From([]));

    [Fact]
    public void Outer_is_session_and_inner_is_weekly() =>
        Assert.Equal(new RingsModel(new Ring(37, UsageLevel.Normal, true), new Ring(61, UsageLevel.Normal, true)),
            RingsModel.From([L(LimitKind.Weekly, 61), L(LimitKind.Session, 37.9)]));

    [Theory]
    [InlineData(69, UsageLevel.Normal)]
    [InlineData(70, UsageLevel.Warning)]
    [InlineData(89, UsageLevel.Warning)]
    [InlineData(90, UsageLevel.Error)]
    public void Each_ring_takes_its_own_level(int pct, UsageLevel level)
    {
        var m = RingsModel.From([L(LimitKind.Session, pct), L(LimitKind.Weekly, 10)]);
        Assert.Equal(level, m.Outer.Level);
        Assert.Equal(UsageLevel.Normal, m.Inner.Level);
    }

    [Fact]
    public void Percent_is_clamped_to_100() =>
        Assert.Equal(100, RingsModel.From([L(LimitKind.Session, 130)]).Outer.Percent);

    [Fact]
    public void A_reset_window_is_zero() =>
        Assert.Equal(new Ring(0, UsageLevel.Normal, true),
            RingsModel.From([new Limit("Claude Code", LimitKind.Session, "Session", 0, Now, Now, true)]).Outer);

    [Fact]
    public void Spend_is_not_drawn() =>
        Assert.Equal(new RingsModel(Ring.None, Ring.None), RingsModel.From([L(LimitKind.Spend, 50)]));

    [Theory]
    [InlineData(0, 0.0)]
    [InlineData(25, 90.0)]
    [InlineData(100, 360.0)]
    public void Sweep_is_clockwise_degrees(int pct, double degrees) =>
        Assert.Equal(degrees, new Ring(pct, UsageLevel.Normal, true).SweepDegrees);

    [Fact]
    public void No_data_sweeps_nothing() => Assert.Equal(0, Ring.None.SweepDegrees);

    [Fact]
    public void Capped_sweep_takes_the_round_caps_off_the_arc() =>
        Assert.Equal(297.93, new Ring(95, UsageLevel.Normal, true).CappedSweep(3.25, 2.5), 2);

    [Fact]
    public void Capped_sweep_is_the_full_circle_at_100() =>
        Assert.Equal(360, new Ring(100, UsageLevel.Normal, true).CappedSweep(3.25, 2.5));

    [Fact]
    public void Capped_sweep_is_nothing_at_0_or_without_data()
    {
        Assert.Equal(0, new Ring(0, UsageLevel.Normal, true).CappedSweep(3.25, 2.5));
        Assert.Equal(0, Ring.None.CappedSweep(3.25, 2.5));
    }

    [Fact]
    public void Capped_sweep_leaves_a_dot_for_tiny_percentages() =>
        Assert.Equal(0.5, new Ring(3, UsageLevel.Normal, true).CappedSweep(3.25, 2.5));

    private static readonly DateTimeOffset CopNow = DateTimeOffset.Parse("2026-11-09T12:00:00Z");

    private static Limit Credits(double used = 2644, DateTimeOffset? observed = null) =>
        LimitList.FromCopilot(new CopilotReading(15000, used, DateTimeOffset.Parse("2026-12-01T00:00:00Z"), observed ?? CopNow, CopilotSource.Live), CopNow)[0];

    [Fact]
    public void Copilot_rings_have_one_ring()
    {
        var m = RingsModel.Copilot([Credits()]);
        Assert.False(m.HasInner);
        Assert.Equal(17, m.Outer.Percent);
        Assert.Equal(Ring.None, m.Inner);
    }
}
