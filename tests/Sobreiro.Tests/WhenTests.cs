using System.Globalization;
using Sobreiro.Core.Display;

namespace Sobreiro.Tests;

public class WhenTests
{
    private static readonly DateTimeOffset Now = DateTimeOffset.Parse("2026-11-09T10:00:00Z");
    private static readonly CultureInfo En = CultureInfo.GetCultureInfo("en-US");

    private static string F(string t, CultureInfo? c = null) =>
        When.Format(DateTimeOffset.Parse(t), Now, TimeZoneInfo.Utc, c ?? En);

    [Fact]
    public void Same_day() => Assert.Equal("2:20 PM", F("2026-11-09T14:20:00Z"));

    [Fact]
    public void Within_six_days_gets_day_name() => Assert.Equal("Sat 12:00 AM", F("2026-11-14T00:00:00Z"));

    [Fact]
    public void Seven_days_ahead_gets_date() => Assert.Equal("Mon 16 Nov 12:00 AM", F("2026-11-16T00:00:00Z"));

    [Fact]
    public void Seven_days_back_gets_date() => Assert.Equal("Mon 2 Nov 2:02 PM", F("2026-11-02T14:02:00Z"));

    [Fact]
    public void Six_days_back_gets_day_name() => Assert.Equal("Tue 2:02 PM", F("2026-11-03T14:02:00Z"));

    [Fact]
    public void Uses_culture_time_pattern() => Assert.Equal("14:20", F("2026-11-09T14:20:00Z", CultureInfo.GetCultureInfo("pt-PT")));
}
