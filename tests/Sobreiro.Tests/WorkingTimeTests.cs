using Sobreiro.Core.Display;

namespace Sobreiro.Tests;

public class WorkingTimeTests
{
    private static readonly TimeZoneInfo Utc = TimeZoneInfo.Utc;
    private static DateTimeOffset T(string s) => DateTimeOffset.Parse(s);
    // 2026-11-09 is a Monday.

    [Fact] public void Weekdays_count_in_full() =>
        Assert.Equal(TimeSpan.FromHours(72), WorkingTime.Between(T("2026-11-09T09:00:00Z"), T("2026-11-12T09:00:00Z"), Utc));

    [Fact] public void A_weekend_in_the_middle_is_skipped() =>
        Assert.Equal(TimeSpan.FromHours(15), WorkingTime.Between(T("2026-11-06T18:00:00Z"), T("2026-11-09T09:00:00Z"), Utc));

    [Fact] public void Inside_a_weekend_is_zero() =>
        Assert.Equal(TimeSpan.Zero, WorkingTime.Between(T("2026-11-07T12:00:00Z"), T("2026-11-08T12:00:00Z"), Utc));

    [Fact] public void Backwards_is_zero() =>
        Assert.Equal(TimeSpan.Zero, WorkingTime.Between(T("2026-11-10T00:00:00Z"), T("2026-11-09T00:00:00Z"), Utc));

    [Fact] public void A_whole_november_has_21_weekdays() =>
        Assert.Equal(TimeSpan.FromHours(504), WorkingTime.Between(T("2026-11-01T00:00:00Z"), T("2026-12-01T00:00:00Z"), Utc));

    [Fact]
    public void The_day_of_the_week_is_the_local_one()
    {
        var plusNine = TimeZoneInfo.CreateCustomTimeZone("plus9", TimeSpan.FromHours(9), "plus9", "plus9");
        // Fri 22:00Z to Sat 02:00Z is Sat 07:00 to Sat 11:00 at +09:00.
        Assert.Equal(TimeSpan.Zero, WorkingTime.Between(T("2026-11-06T22:00:00Z"), T("2026-11-07T02:00:00Z"), plusNine));
        // Sun 20:00Z to Sun 23:00Z is Mon 05:00 to Mon 08:00 at +09:00.
        Assert.Equal(TimeSpan.FromHours(3), WorkingTime.Between(T("2026-11-08T20:00:00Z"), T("2026-11-08T23:00:00Z"), plusNine));
    }

    [Fact]
    public void A_clock_change_at_the_weekend_moves_midnight()
    {
        // Lisbon goes from +01:00 to +00:00 on Sunday 2026-10-25.
        var lisbon = TimeZoneInfo.FindSystemTimeZoneById("Europe/Lisbon");
        var friNoon = T("2026-10-23T11:00:00Z"); // 12:00 local
        var monNoon = T("2026-10-26T12:00:00Z"); // 12:00 local
        Assert.Equal(TimeSpan.FromHours(24), WorkingTime.Between(friNoon, monNoon, lisbon));
        Assert.Equal(monNoon, WorkingTime.After(friNoon, TimeSpan.FromHours(24), lisbon));
    }

    [Fact] public void After_skips_the_weekend() =>
        Assert.Equal(T("2026-11-10T04:00:00Z"), WorkingTime.After(T("2026-11-06T00:00:00Z"), TimeSpan.FromHours(52), Utc));

    [Fact] public void After_from_a_saturday_starts_on_monday() =>
        Assert.Equal(T("2026-11-09T01:00:00Z"), WorkingTime.After(T("2026-11-07T12:00:00Z"), TimeSpan.FromHours(1), Utc));

    [Fact] public void After_nothing_is_the_start() =>
        Assert.Equal(T("2026-11-07T12:00:00Z"), WorkingTime.After(T("2026-11-07T12:00:00Z"), TimeSpan.Zero, Utc));
}
