using Sobreiro.Core.Colours;

namespace Sobreiro.Flyout;

/// <summary>The Night palette for the theme, and the fill the bars, cells and thermometer take below 70%.</summary>
public sealed record FlyoutColours(NightPalette Palette, Rgb Fill)
{
    public static FlyoutColours For(bool dark, BarColourOption option, AccentPalette? accent) =>
        new(NightPalette.For(dark), BarColour.Pick(option, accent, dark).Colour);
}
