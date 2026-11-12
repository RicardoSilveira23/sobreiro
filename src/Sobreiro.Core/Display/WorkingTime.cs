namespace Sobreiro.Core.Display;

/// <summary>Measures time that falls on Monday to Friday in a zone, midnight to midnight.</summary>
public static class WorkingTime
{
    /// <summary>The time between the two moments that falls on a weekday in <paramref name="zone"/>; zero when <paramref name="to"/> is not after <paramref name="from"/>.</summary>
    public static TimeSpan Between(DateTimeOffset from, DateTimeOffset to, TimeZoneInfo zone)
    {
        var total = TimeSpan.Zero;
        if (to <= from) return total;

        var date = DateOnly.FromDateTime(TimeZoneInfo.ConvertTime(from, zone).DateTime);
        while (true)
        {
            var start = Midnight(date, zone);
            if (start >= to) break;
            if (IsWeekday(date))
            {
                var end = Midnight(date.AddDays(1), zone);
                var span = Min(end, to) - Max(start, from);
                if (span > TimeSpan.Zero) total += span;
            }
            date = date.AddDays(1);
        }
        return total;
    }

    /// <summary>The moment at which <paramref name="working"/> weekday time has passed after <paramref name="from"/>; <paramref name="from"/> itself when it is not positive.</summary>
    public static DateTimeOffset After(DateTimeOffset from, TimeSpan working, TimeZoneInfo zone)
    {
        if (working <= TimeSpan.Zero) return from;

        var remaining = working;
        var date = DateOnly.FromDateTime(TimeZoneInfo.ConvertTime(from, zone).DateTime);
        while (true)
        {
            if (IsWeekday(date))
            {
                var start = Max(Midnight(date, zone), from);
                var available = Midnight(date.AddDays(1), zone) - start;
                if (available > TimeSpan.Zero)
                {
                    if (remaining <= available) return start + remaining;
                    remaining -= available;
                }
            }
            date = date.AddDays(1);
        }
    }

    private static bool IsWeekday(DateOnly date) => date.DayOfWeek is not (DayOfWeek.Saturday or DayOfWeek.Sunday);

    /// <summary>The moment a local day begins. A midnight the clock skips begins at the first valid time after it.</summary>
    private static DateTimeOffset Midnight(DateOnly date, TimeZoneInfo zone)
    {
        var local = date.ToDateTime(TimeOnly.MinValue);
        while (zone.IsInvalidTime(local)) local = local.AddMinutes(30);
        return new DateTimeOffset(local, zone.GetUtcOffset(local));
    }

    private static DateTimeOffset Min(DateTimeOffset a, DateTimeOffset b) => a < b ? a : b;
    private static DateTimeOffset Max(DateTimeOffset a, DateTimeOffset b) => a > b ? a : b;
}
