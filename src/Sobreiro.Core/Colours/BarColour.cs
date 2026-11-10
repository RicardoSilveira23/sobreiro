namespace Sobreiro.Core.Colours;

public enum BarColourOption { Purple, Accent }

public enum BarColourSource { Purple, Accent, AccentShade, PurpleFallback }

public sealed record BarColourChoice(Rgb Colour, BarColourSource Source);

public static class BarColour
{
    public const double MinContrast = 3.0;

    /// <summary>Purple, or the Windows accent where it (or one of its shades) reaches 3:1 on the ground and the track.</summary>
    public static BarColourChoice Pick(BarColourOption option, AccentPalette? accent, bool dark)
    {
        var night = NightPalette.For(dark);
        if (option == BarColourOption.Purple)
            return new BarColourChoice(night.Purple, BarColourSource.Purple);

        if (accent is not null)
        {
            Rgb[] shades = dark
                ? [accent.Light1, accent.Light2, accent.Light3]
                : [accent.Dark1, accent.Dark2, accent.Dark3];

            if (Reads(accent.Accent, night))
                return new BarColourChoice(accent.Accent, BarColourSource.Accent);
            foreach (var shade in shades)
                if (Reads(shade, night))
                    return new BarColourChoice(shade, BarColourSource.AccentShade);
        }
        return new BarColourChoice(night.Purple, BarColourSource.PurpleFallback);
    }

    private static bool Reads(Rgb c, NightPalette night) =>
        Contrast.Ratio(c, night.Ground) >= MinContrast && Contrast.Ratio(c, night.Track) >= MinContrast;
}
