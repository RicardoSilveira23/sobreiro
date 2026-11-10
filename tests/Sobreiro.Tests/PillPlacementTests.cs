using Sobreiro.Core.Display;
using Sobreiro.Core.Taskbar;

namespace Sobreiro.Tests;

public class PillPlacementTests
{
    private static readonly PixelRect Monitor = new(0, 0, 2560, 1440);
    private static readonly TaskbarAnchors At100 = new(2314, 1589, 0, 48);

    [Fact]
    public void Places_left_of_the_tray_at_100_percent() =>
        Assert.Equal(new PillPlace(2222, 1397, null, 1392), PillPlacement.Place(Monitor, new(0, 1392, 2560, 1440), At100, 88, 38, 4));

    [Fact]
    public void Places_left_of_the_tray_at_125_percent() =>
        Assert.Equal(new PillPlace(2138, 1386, null, 1380), PillPlacement.Place(Monitor, new(0, 1380, 2560, 1440), new TaskbarAnchors(2253, 1693, 0, 60), 110, 47, 5));

    [Fact]
    public void Places_on_a_monitor_not_at_the_origin() =>
        Assert.Equal(new PillPlace(-338, 1397, null, 1392), PillPlacement.Place(new(-2560, 0, 0, 1440), new(-2560, 1392, 0, 1440), At100, 88, 38, 4));

    [Fact]
    public void No_anchors_hides() =>
        Assert.Equal(PillHidden.NoAnchors, PillPlacement.Place(Monitor, new(0, 1392, 2560, 1440), null, 88, 38, 4).Hidden);

    [Theory, InlineData(1404), InlineData(1438)] // sliding away, then auto-hidden
    public void A_taskbar_partly_off_the_monitor_hides(int top) =>
        Assert.Equal(PillHidden.OffScreen, PillPlacement.Place(Monitor, new(0, top, 2560, top + 48), At100, 88, 38, 4).Hidden);

    [Fact]
    public void Hides_when_the_pill_is_taller_than_the_band() =>
        Assert.Equal(PillHidden.NoRoom, PillPlacement.Place(Monitor, new(0, 1392, 2560, 1440), At100, 88, 50, 4).Hidden);

    [Fact]
    public void Hides_when_it_would_cover_the_app_buttons() =>
        Assert.Equal(PillHidden.NoRoom, PillPlacement.Place(Monitor, new(0, 1392, 2560, 1440), At100 with { ButtonsRight = 2230 }, 88, 38, 4).Hidden);

    [Fact]
    public void Just_enough_room_shows() => // x 2222 == buttons 2218 + gap 4
        Assert.True(PillPlacement.Place(Monitor, new(0, 1392, 2560, 1440), At100 with { ButtonsRight = 2218 }, 88, 38, 4).IsShown);

    [Fact]
    public void A_taskbar_at_the_top_hides() =>
        Assert.Equal(PillHidden.NotAtBottom, PillPlacement.Place(Monitor, new(0, 0, 2560, 48), At100 with { BandTop = 0, BandBottom = 48 }, 88, 38, 4).Hidden);

    [Fact]
    public void A_taskbar_on_the_side_hides() =>
        Assert.Equal(PillHidden.NotAtBottom, PillPlacement.Place(Monitor, new(2512, 0, 2560, 1440), At100, 88, 38, 4).Hidden);
}
