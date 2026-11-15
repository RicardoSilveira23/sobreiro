using Sobreiro.Core.Display;

namespace Sobreiro.Tests;

public class FlyoutPlacementTests
{
    private static readonly PixelRect Monitor = new(0, 0, 1920, 1080);
    private const int W = 330, H = 400, M = 12;

    private static (TaskbarEdge Edge, int X, int Y) Run(PixelRect monitor, PixelRect work, int cx, int cy)
    {
        var (x, y) = FlyoutPlacement.Place(monitor, work, cx, cy, W, H, M);
        return (FlyoutPlacement.Edge(monitor, work, cx, cy), x, y);
    }

    [Fact]
    public void BottomTaskbarClampsToTheRight() =>
        Assert.Equal((TaskbarEdge.Bottom, 1578, 620), Run(Monitor, new(0, 0, 1920, 1032), 1800, 1060));

    [Fact]
    public void BottomTaskbarCentresOnTheCursor() =>
        Assert.Equal(135, Run(Monitor, new(0, 0, 1920, 1032), 300, 1060).X);

    [Fact]
    public void TopTaskbar() =>
        Assert.Equal((TaskbarEdge.Top, 1578, 60), Run(Monitor, new(0, 48, 1920, 1080), 1800, 20));

    [Fact]
    public void LeftTaskbar() =>
        Assert.Equal((TaskbarEdge.Left, 60, 668), Run(Monitor, new(48, 0, 1920, 1080), 20, 900));

    [Fact]
    public void RightTaskbar() =>
        Assert.Equal((TaskbarEdge.Right, 1530, 300), Run(Monitor, new(0, 0, 1872, 1080), 1900, 500));

    [Fact]
    public void AutoHideUsesTheNearestMonitorEdge()
    {
        var (edge, _, y) = Run(Monitor, Monitor, 1800, 1075);
        Assert.Equal(TaskbarEdge.Bottom, edge);
        Assert.Equal(668, y);
    }

    [Fact]
    public void SecondMonitorOnTheLeft() =>
        Assert.Equal(-342, Run(new(-1920, 0, 0, 1080), new(-1920, 0, 0, 1032), -100, 1060).X);

    [Fact]
    public void RectSizes()
    {
        var r = new PixelRect(10, 20, 110, 70);
        Assert.Equal(100, r.Width);
        Assert.Equal(50, r.Height);
    }
}
