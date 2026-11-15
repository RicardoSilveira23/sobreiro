using Sobreiro.Core.Display;
using Sobreiro.Core.Media;

namespace Sobreiro.Tests;

public class TaskbarPlateTests
{
    private static string PlaceholderJson() =>
        File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "Fixtures", "plate-placeholder.json"));

    private static TaskbarPlate Placeholder() => TaskbarPlate.Parse(PlaceholderJson());

    [Fact]
    public void ParsesThePlaceholder()
    {
        var p = Placeholder();

        Assert.Equal("plate-placeholder.png", p.Image);
        Assert.True(File.Exists(Path.Combine(AppContext.BaseDirectory, "Fixtures", p.Image)));
        Assert.Equal(DateTimeOffset.Parse("2026-11-10T21:58:00+00:00"), p.CapturedAt);
        Assert.True(p.Dark);
        Assert.Equal((1960, 1392), (p.OriginX, p.OriginY));
        Assert.Equal(new PixelRect(0, 0, 2560, 1440), p.Monitor);
        Assert.Equal(new PixelRect(0, 0, 2560, 1392), p.WorkArea);
        Assert.Equal(new PixelRect(0, 1392, 2560, 1440), p.Taskbar);
        Assert.Equal((1396, 1436), (p.BandTop, p.BandBottom));
        Assert.Equal(new PixelRect(2300, 1402, 2400, 1430), p.Pill);
        Assert.Equal(new PixelRect(2410, 1404, 2434, 1428), p.ClaudeIcon);
        Assert.Null(p.CopilotIcon);
        Assert.Equal((2100, 2290), (p.GroundLeft, p.GroundRight));
    }

    [Fact]
    public void ParseNamesTheMissingField()
    {
        var json = PlaceholderJson();
        var start = json.IndexOf("\"pill\"", StringComparison.Ordinal);
        var end = json.IndexOf('\n', start);
        var without = json.Remove(start, end - start + 1);

        var ex = Assert.Throws<FormatException>(() => TaskbarPlate.Parse(without));

        Assert.Contains("pill", ex.Message);
    }

    [Fact]
    public void ClearedPillWidensLeftKeepingTheRightEdge()
    {
        var cleared = Placeholder().ClearedPill(160);

        Assert.Equal(new PixelRect(2240, 1402, 2400, 1430), cleared);
    }

    [Fact]
    public void ClearedPillNeverShrinks()
    {
        var p = Placeholder();

        Assert.Equal(p.Pill, p.ClearedPill(40));
    }

    [Fact]
    public void PillAtCentresInTheBand()
    {
        var (x, y) = Placeholder().PillAt(80, 28);

        Assert.Equal(2400 - 80, x);
        Assert.Equal(1396 + (40 - 28) / 2, y);
    }

    [Fact]
    public void DetailsSitAboveTheTaskbar()
    {
        var p = Placeholder();

        var (x, y) = p.DetailsAt(320, 400, 12);

        Assert.True(y + 400 <= p.WorkArea.Bottom);
        Assert.Equal(p.WorkArea.Bottom - 400 - 12, y);
        Assert.Equal(2350 - 160, x);
    }

    [Fact]
    public void IconPxDefaultsToSixteen() => Assert.Equal(16, Placeholder().IconPx);

    [Fact]
    public void IconPxIsReadWhenGiven()
    {
        var json = PlaceholderJson().Replace("\"ground\"", "\"iconPx\": 20, \"ground\"");
        Assert.Equal(20, TaskbarPlate.Parse(json).IconPx);
    }

    [Fact]
    public void GroundColumnInsideTheStretchIsThatColumn() => Assert.Equal(2150, Placeholder().GroundColumn(2150));

    [Fact]
    public void GroundColumnWrapsAfterTheStretch()
    {
        var p = Placeholder(); // 2100..2290, width 190
        Assert.Equal(2100, p.GroundColumn(2290));
        Assert.Equal(2105, p.GroundColumn(2295));
    }

    [Fact]
    public void GroundColumnLeftOfTheStretchWrapsBackwards()
    {
        var p = Placeholder();
        Assert.Equal(2289, p.GroundColumn(2099));
        Assert.Equal(2280, p.GroundColumn(2090));
    }

    [Fact]
    public void CropLeftIsTheMarginLeftOfTheLeftmostRect()
    {
        var x = Placeholder().CropLeft([new PixelRect(2300, 0, 2400, 10), new PixelRect(2250, 0, 2290, 10), new PixelRect(2320, 0, 2330, 10)], 24);
        Assert.Equal(2226, x);
    }

    [Fact]
    public void CropLeftNeverGoesLeftOfTheCapture()
    {
        var p = Placeholder(); // origin x 1960
        Assert.Equal(1960, p.CropLeft([new PixelRect(1970, 0, 2000, 10)], 24));
        Assert.Equal(1960, p.CropLeft([], 24));
    }
}
