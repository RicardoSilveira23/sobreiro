using Sobreiro.Core.Limits;

namespace Sobreiro.Core.Colours;

/// <summary>Ink and ring colours for the taskbar pill and tray rings.</summary>
public sealed record TaskbarColours(Rgb Ink, Rgb Base, Rgb Warning, Rgb Error)
{
    /// <summary>Alpha of the ring track (the ink at about 15%).</summary>
    public const byte TrackAlpha = 0x26;

    /// <summary>Alpha of the tray icon's ring track (the ink at about 35%): at 16 px the pill's 15% vanishes.</summary>
    public const byte IconTrackAlpha = 0x59;

    public static TaskbarColours For(bool lightTaskbar, bool accentTaskbar, BarColourOption option, AccentPalette? accent)
    {
        var ink = lightTaskbar ? Rgb.Parse("#1A1A1A") : Rgb.Parse("#FFFFFF");
        var night = NightPalette.For(dark: !lightTaskbar);
        var baseColour = accentTaskbar ? ink : BarColour.Pick(option, accent, dark: !lightTaskbar).Colour;
        return new TaskbarColours(ink, baseColour, night.Warning, night.Error);
    }

    public Rgb Fill(UsageLevel level) => level switch
    {
        UsageLevel.Warning => Warning,
        UsageLevel.Error => Error,
        _ => Base,
    };
}
