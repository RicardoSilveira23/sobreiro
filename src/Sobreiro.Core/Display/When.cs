using System.Globalization;

namespace Sobreiro.Core.Display;

internal static class When
{
    /// <summary>
    /// Clock time in the culture's short pattern. Another local day within six days gets its English day name;
    /// anything further out (or back) gets the day name, date and month.
    /// </summary>
    public static string Format(DateTimeOffset t, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture)
    {
        var local = TimeZoneInfo.ConvertTime(t, zone);
        var localNow = TimeZoneInfo.ConvertTime(now, zone);
        var clock = local.ToString(culture.DateTimeFormat.ShortTimePattern, culture);
        var days = Math.Abs((local.Date - localNow.Date).Days);
        if (days == 0) return clock;
        var date = days <= 6 ? "ddd" : "ddd d MMM";
        return local.ToString(date, CultureInfo.InvariantCulture) + " " + clock;
    }
}
