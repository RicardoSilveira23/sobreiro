using Sobreiro.Core.Display;
using Sobreiro.Core.Taskbar;

namespace Sobreiro.Tests;

public class TaskbarLayoutTests
{
    private static readonly PixelRect Bar = new(0, 1392, 2560, 1440);
    private const int Explorer = 4556;
    private const int Own = 444;

    [Fact]
    public void Anchors_from_the_spike()
    {
        var items = new[]
        {
            ("SystemTray.NormalButton", new PixelRect(2314, 1392, 2346, 1440), Explorer),
            ("SystemTray.OmniButton", new PixelRect(2474, 1392, 2544, 1440), Explorer),
            ("Taskbar.TaskbarFrameAutomationPeer", new PixelRect(0, 1392, 2560, 1440), Explorer),
        };
        var buttons = new[] { new PixelRect(972, 1392, 1017, 1440), new PixelRect(1545, 1392, 1589, 1440) };
        Assert.Equal(new TaskbarAnchors(2314, 1589, 0, 48), TaskbarLayout.Anchors(Bar, items, buttons, Own));
    }

    [Fact]
    public void Anchors_without_tray_items_is_null() =>
        Assert.Null(TaskbarLayout.Anchors(Bar, [("Taskbar.TaskbarFrameAutomationPeer", Bar, Explorer)], [], Own));

    [Fact]
    public void Empty_tray_items_are_skipped() =>
        Assert.Null(TaskbarLayout.Anchors(Bar, [("SystemTray.NormalButton", new PixelRect(2314, 1392, 2314, 1440), Explorer)], [], Own));

    [Fact]
    public void No_buttons_means_the_taskbars_left_edge() =>
        Assert.Equal(0, TaskbarLayout.Anchors(Bar, [("SystemTray.OmniButton", new PixelRect(2474, 1392, 2544, 1440), Explorer)], [], Own)!.Value.ButtonsRight);

    [Fact]
    public void Anchors_are_offsets_from_the_taskbar() =>
        Assert.Equal(new TaskbarAnchors(314, 89, 0, 48), TaskbarLayout.Anchors(new PixelRect(2000, 1392, 4560, 1440),
            [("SystemTray.NormalButton", new PixelRect(2314, 1392, 2346, 1440), Explorer)], [new PixelRect(2050, 1392, 2089, 1440)], Own));

    [Fact]
    public void An_unknown_item_left_of_the_tray_is_an_obstacle() =>
        Assert.Equal(2250, TaskbarLayout.Anchors(Bar,
            [("SystemTray.NormalButton", new PixelRect(2314, 1392, 2346, 1440), Explorer), ("ToggleButton", new PixelRect(2200, 1392, 2250, 1440), Explorer)],
            [new PixelRect(972, 1392, 1017, 1440)], Own)!.Value.ButtonsRight);

    [Fact]
    public void A_pane_spanning_the_whole_taskbar_is_not_an_obstacle() =>
        Assert.Equal(1017, TaskbarLayout.Anchors(Bar,
            [("SystemTray.NormalButton", new PixelRect(2314, 1392, 2346, 1440), Explorer), ("Windows.UI.Input.InputSite.WindowClass", Bar, Explorer)],
            [new PixelRect(972, 1392, 1017, 1440)], Own)!.Value.ButtonsRight);

    [Fact]
    public void Own_windows_are_not_obstacles() =>
        Assert.Equal(1017, TaskbarLayout.Anchors(Bar,
            [("SystemTray.NormalButton", new PixelRect(2314, 1392, 2346, 1440), Explorer), ("Window", new PixelRect(2200, 1392, 2300, 1440), Own)],
            [new PixelRect(972, 1392, 1017, 1440)], Own)!.Value.ButtonsRight);
}
