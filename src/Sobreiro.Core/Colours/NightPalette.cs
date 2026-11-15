using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;

namespace Sobreiro.Core.Colours;

public sealed record BadgeColours(Rgb Fill, Rgb Text);

public sealed record NightPalette(Rgb Ground, Rgb Ink, Rgb Soft, Rgb Line, Rgb Track, Rgb Warning, Rgb Error, Rgb Purple, bool IsDark)
{
    public static NightPalette Dark { get; } = new(
        Ground: Rgb.Parse("#0b0b0b"), Ink: Rgb.Parse("#f4f4f4"), Soft: Rgb.Parse("#8d8d8d"),
        Line: Rgb.Parse("#262626"), Track: Rgb.Parse("#262626"),
        Warning: Rgb.Parse("#ffb020"), Error: Rgb.Parse("#ff6b6b"), Purple: Rgb.Parse("#a594ff"), IsDark: true);

    public static NightPalette Light { get; } = new(
        Ground: Rgb.Parse("#ffffff"), Ink: Rgb.Parse("#0d0d0d"), Soft: Rgb.Parse("#767676"),
        Line: Rgb.Parse("#e4e4e4"), Track: Rgb.Parse("#e4e4e4"),
        Warning: Rgb.Parse("#a35f00"), Error: Rgb.Parse("#c4262e"), Purple: Rgb.Parse("#5a3dd1"), IsDark: false);

    public static NightPalette For(bool dark) => dark ? Dark : Light;

    /// <summary>Fill and text of a status badge. Text on fill is at least 4.5:1 in both palettes.</summary>
    public BadgeColours Badge(StatusKind kind)
    {
        return kind switch
        {
            StatusKind.OnTrack => new(Rgb.Parse("#1a7f37"), Rgb.Parse("#ffffff")),
            StatusKind.Ends => new(Warning, IsDark ? Ground : Rgb.Parse("#ffffff")),
            StatusKind.UsedUp => new(Error, IsDark ? Ground : Rgb.Parse("#ffffff")),
            _ => new(Track, Ink),
        };
    }

    public Rgb LevelColour(UsageLevel level, Rgb normal) => level switch
    {
        UsageLevel.Warning => Warning,
        UsageLevel.Error => Error,
        _ => normal,
    };
}
