using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;

namespace Sobreiro.Tests;

public class GlyphFillTests
{
    private static LimitView View(double used, double? projected, bool hasData = true) =>
        new(LimitKind.Session, "Session", hasData, "", "", used, projected, UsageLevel.Normal, null, null, null);

    [Fact]
    public void Mockup_session_has_seven_left_and_five_soon()
    {
        var fill = GlyphFill.From(View(0.27, 0.7431));
        Assert.Equal(7, fill.LeftCells);
        Assert.Equal(5, fill.SoonCells);
        Assert.Equal(73, fill.LeftPercent, 0.0001);
        Assert.Equal(47.31, fill.SoonPercent, 0.0001);
    }

    [Fact]
    public void Mockup_weekly_has_three_left_all_soon()
    {
        var fill = GlyphFill.From(View(0.74, 1.0));
        Assert.Equal(3, fill.LeftCells);
        Assert.Equal(3, fill.SoonCells);
        Assert.Equal(26, fill.LeftPercent, 0.0001);
        Assert.Equal(26, fill.SoonPercent, 0.0001);
    }

    [Fact]
    public void No_projection_has_nothing_soon()
    {
        var fill = GlyphFill.From(View(0.27, null));
        Assert.Equal(7, fill.LeftCells);
        Assert.Equal(0, fill.SoonCells);
        Assert.Equal(0, fill.SoonPercent);
    }

    [Fact]
    public void Full_use_has_nothing_left()
    {
        var fill = GlyphFill.From(View(1.0, null));
        Assert.Equal(new GlyphFill(0, 0, 0, 0), fill);
    }

    [Fact]
    public void No_data_has_nothing_left() =>
        Assert.Equal(new GlyphFill(0, 0, 0, 0), GlyphFill.From(View(0, null, hasData: false)));
}
