using System.Globalization;

namespace Sobreiro.Core.Display;

public static class Duration
{
    /// <summary>"3d 13h", "3h 11m", "45m", "under 1m"; the smaller unit is dropped when zero.</summary>
    public static string Format(TimeSpan span)
    {
        var minutes = (long)span.TotalMinutes;
        if (span <= TimeSpan.Zero || minutes < 1) return "under 1m";
        var d = minutes / 1440;
        var h = minutes % 1440 / 60;
        var m = minutes % 60;
        if (d > 0) return h == 0 ? N(d) + "d" : $"{N(d)}d {N(h)}h";
        if (h > 0) return m == 0 ? N(h) + "h" : $"{N(h)}h {N(m)}m";
        return N(m) + "m";
    }

    private static string N(long v) => v.ToString(CultureInfo.InvariantCulture);
}
