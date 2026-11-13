using Sobreiro.Core.Colours;
using Sobreiro.Core.Limits;

namespace Sobreiro.Tests;

public class TaskbarColoursTests
{
    [Fact]
    public void Dark_taskbar_purple() => Assert.Equal(
        new TaskbarColours(Rgb.Parse("#FFFFFF"), Rgb.Parse("#A594FF"), Rgb.Parse("#ffb020"), Rgb.Parse("#ff6b6b")),
        TaskbarColours.For(lightTaskbar: false, accentTaskbar: false, BarColourOption.Purple, null));

    [Fact]
    public void Light_taskbar_purple() => Assert.Equal(
        new TaskbarColours(Rgb.Parse("#1A1A1A"), Rgb.Parse("#5a3dd1"), Rgb.Parse("#a35f00"), Rgb.Parse("#c4262e")),
        TaskbarColours.For(lightTaskbar: true, accentTaskbar: false, BarColourOption.Purple, null));

    [Fact]
    public void Accent_option_uses_the_bar_colour_pick() => Assert.Equal(
        BarColour.Pick(BarColourOption.Accent, AccentPalette.WindowsBlue, dark: true).Colour,
        TaskbarColours.For(false, false, BarColourOption.Accent, AccentPalette.WindowsBlue).Base);

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void Accent_coloured_taskbar_uses_ink_as_base(bool light)
    {
        var c = TaskbarColours.For(light, accentTaskbar: true, BarColourOption.Accent, AccentPalette.WindowsBlue);
        Assert.Equal(c.Ink, c.Base);
    }

    [Fact]
    public void Fill_follows_the_level()
    {
        var c = TaskbarColours.For(false, false, BarColourOption.Purple, null);
        Assert.Equal(c.Base, c.Fill(UsageLevel.Normal));
        Assert.Equal(c.Warning, c.Fill(UsageLevel.Warning));
        Assert.Equal(c.Error, c.Fill(UsageLevel.Error));
    }

    // The arc must stand out from its track on a dark taskbar (#1C1C1C): white at a given alpha over it.
    private static byte Over(byte ink, byte ground, byte alpha) => (byte)Math.Round((ink * alpha + ground * (255 - alpha)) / 255.0);

    private static Rgb WhiteOverDark(byte alpha) => new(Over(255, 0x1C, alpha), Over(255, 0x1C, alpha), Over(255, 0x1C, alpha));

    [Fact] public void Dark_purple_reads_on_the_dark_track() {
        var track = WhiteOverDark(TaskbarColours.TrackAlpha);
        var c = TaskbarColours.For(lightTaskbar: false, accentTaskbar: false, BarColourOption.Purple, null);
        Assert.True(Contrast.Ratio(c.Base, track) >= 3.0); }

    // The 16 px tray icon needs a stronger track than the pill, or it vanishes.
    [Fact]
    public void Icon_track_shows_on_the_dark_taskbar() =>
        Assert.True(Contrast.Ratio(WhiteOverDark(TaskbarColours.IconTrackAlpha), new Rgb(0x1C, 0x1C, 0x1C)) >= 2.0);

    [Fact]
    public void Dark_purple_reads_on_the_icon_track()
    {
        var c = TaskbarColours.For(lightTaskbar: false, accentTaskbar: false, BarColourOption.Purple, null);
        Assert.True(Contrast.Ratio(c.Base, WhiteOverDark(TaskbarColours.IconTrackAlpha)) >= 2.0);
    }

    // A stopped block's fill: the ink at half strength, as drawn in the variant Ricardo picked.
    [Fact]
    public void The_stale_fill_is_the_ink_at_half_strength() => Assert.Equal(0x80, TaskbarColours.StaleAlpha);

    [Fact]
    public void The_stale_fill_reads_on_the_dark_track() =>
        Assert.True(Contrast.Ratio(WhiteOverDark(TaskbarColours.StaleAlpha), WhiteOverDark(TaskbarColours.TrackAlpha)) >= 3.0);

    // On a light taskbar (#F0F0F0) the ink is #1A1A1A.
    private static Rgb InkOverLight(byte alpha) => new(Over(0x1A, 0xF0, alpha), Over(0x1A, 0xF0, alpha), Over(0x1A, 0xF0, alpha));

    [Fact]
    public void The_stale_fill_reads_on_the_light_track() =>
        Assert.True(Contrast.Ratio(InkOverLight(TaskbarColours.StaleAlpha), InkOverLight(TaskbarColours.TrackAlpha)) >= 2.0);
}
