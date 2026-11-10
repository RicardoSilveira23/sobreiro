using Sobreiro.Core.Display;

namespace Sobreiro.Tests;

public class DurationTests
{
    [Theory]
    [InlineData(3 * 60 + 11, "3h 11m")]
    [InlineData(3 * 1440 + 13 * 60 + 59, "3d 13h")]
    [InlineData(2 * 1440 + 30, "2d")]
    [InlineData(45, "45m")]
    public void Formats_minutes(int minutes, string expected) =>
        Assert.Equal(expected, Duration.Format(TimeSpan.FromMinutes(minutes)));

    [Fact]
    public void Drops_seconds_at_whole_hours() => Assert.Equal("2h", Duration.Format(new TimeSpan(2, 0, 30)));

    [Fact]
    public void Under_a_minute() => Assert.Equal("under 1m", Duration.Format(TimeSpan.FromSeconds(30)));

    [Fact]
    public void Negative() => Assert.Equal("under 1m", Duration.Format(TimeSpan.FromHours(-2)));
}
