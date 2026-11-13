using Sobreiro.Core.Baseline;
using Sobreiro.Core.Display;

namespace Sobreiro.Tests;

public class PillSightingTests
{
    private const nint Taskbar = 0x1234;
    private static readonly PixelRect TaskbarRect = new(0, 1392, 2560, 1440);
    private static readonly PixelRect InTaskbar = new(2222, 1397, 2310, 1435);

    [Fact]
    public void A_visible_window_owned_by_the_taskbar_and_inside_it_is_the_pill() =>
        Assert.True(PillSighting.IsPill(new WindowFacts(true, Taskbar, InTaskbar), Taskbar, TaskbarRect));

    [Fact]
    public void A_hidden_window_is_not() =>
        Assert.False(PillSighting.IsPill(new WindowFacts(false, Taskbar, InTaskbar), Taskbar, TaskbarRect));

    [Fact]
    public void A_window_with_another_owner_is_not() =>
        Assert.False(PillSighting.IsPill(new WindowFacts(true, 0, InTaskbar), Taskbar, TaskbarRect));

    // The pill is first shown far off screen, to be measured, and only then moved over the taskbar.
    [Fact]
    public void A_window_parked_off_screen_is_not() =>
        Assert.False(PillSighting.IsPill(new WindowFacts(true, Taskbar, new(-32000, -32000, -31912, -31962)), Taskbar, TaskbarRect));

    [Fact]
    public void A_window_that_reaches_above_the_taskbar_is_not() =>
        Assert.False(PillSighting.IsPill(new WindowFacts(true, Taskbar, new(2100, 900, 2500, 1400)), Taskbar, TaskbarRect));

    [Fact]
    public void An_empty_window_is_not() =>
        Assert.False(PillSighting.IsPill(new WindowFacts(true, Taskbar, new(2222, 1397, 2222, 1397)), Taskbar, TaskbarRect));

    [Fact]
    public void Nothing_is_the_pill_without_a_taskbar() =>
        Assert.False(PillSighting.IsPill(new WindowFacts(true, 0, InTaskbar), 0, TaskbarRect));
}
