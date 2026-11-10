using System.Globalization;
using Sobreiro.Core.Colours;

namespace Sobreiro.Core.Media;

public enum Services { Claude, Copilot, Both }

/// <summary>The inputs every rendered picture is drawn with, so two runs give the same files.</summary>
public static class MediaPins
{
    public static DateTimeOffset Now { get; } = DateTimeOffset.Parse("2026-11-10T14:00:00+00:00", CultureInfo.InvariantCulture);
    public static TimeZoneInfo Zone { get; } = TimeZoneInfo.FindSystemTimeZoneById("GMT Standard Time");
    public static CultureInfo Culture { get; } = CultureInfo.GetCultureInfo("en-GB");
    public static AccentPalette Accent => AccentPalette.WindowsBlue;
}
