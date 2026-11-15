using System.Text.Json;
using Sobreiro.Core.Usage;

namespace Sobreiro.Tests;

public class RateLimitsJsonTests
{
    private const string Full = """
    {"session_id":"abc","transcript_path":"C:/x.jsonl","cwd":"C:/p",
     "rate_limits":{"five_hour":{"used_percentage":23.5,"resets_at":1738425600},
                    "seven_day":{"used_percentage":41.2,"resets_at":1738857600},
                    "spend_limit":{"used_percentage":62.8,"resets_at":1740787200,"used_usd":314.12,"limit_usd":500,"period":"monthly"}}}
    """;

    private static DateTimeOffset T(long s) => DateTimeOffset.FromUnixTimeSeconds(s);

    [Fact]
    public void ParseStatusLine_reads_all_three_windows()
    {
        var limits = RateLimitsJson.ParseStatusLine(Full);

        Assert.NotNull(limits);
        Assert.Equal(new LimitWindow(23.5, T(1738425600)), limits.FiveHour);
        Assert.Equal(new LimitWindow(41.2, T(1738857600)), limits.SevenDay);
        Assert.Equal(new SpendLimit(62.8, T(1740787200), 314.12, 500, "monthly"), limits.Spend);
    }

    [Fact]
    public void ParseStatusLine_returns_null_without_rate_limits()
    {
        Assert.Null(RateLimitsJson.ParseStatusLine("""{"session_id":"abc"}"""));
    }

    [Theory]
    [InlineData("not json")]
    [InlineData("")]
    [InlineData("[]")]
    public void ParseStatusLine_returns_null_for_non_json(string input)
    {
        Assert.Null(RateLimitsJson.ParseStatusLine(input));
    }

    [Fact]
    public void ParseStatusLine_returns_null_when_no_window_has_a_percentage()
    {
        Assert.Null(RateLimitsJson.ParseStatusLine("""{"rate_limits":{}}"""));
    }

    [Fact]
    public void ParseStatusLine_keeps_windows_independently()
    {
        var limits = RateLimitsJson.ParseStatusLine(
            """{"rate_limits":{"seven_day":{"used_percentage":10,"resets_at":1738857600}}}""");

        Assert.NotNull(limits);
        Assert.Null(limits.FiveHour);
        Assert.Equal(new LimitWindow(10, T(1738857600)), limits.SevenDay);
    }

    [Fact]
    public void ParseStatusLine_drops_window_with_non_numeric_percentage()
    {
        var limits = RateLimitsJson.ParseStatusLine(
            """{"rate_limits":{"five_hour":{"used_percentage":"8","resets_at":1738425600},"seven_day":{"used_percentage":5}}}""");

        Assert.NotNull(limits);
        Assert.Null(limits.FiveHour);
        Assert.NotNull(limits.SevenDay);
    }

    [Theory]
    [InlineData("null")]
    [InlineData("\"soon\"")]
    public void ParseStatusLine_keeps_window_without_valid_resets_at(string resetsAt)
    {
        var limits = RateLimitsJson.ParseStatusLine(
            "{\"rate_limits\":{\"five_hour\":{\"used_percentage\":8,\"resets_at\":" + resetsAt + "}}}");

        Assert.NotNull(limits);
        Assert.Equal(new LimitWindow(8, null), limits.FiveHour);
    }

    [Fact]
    public void ParseStatusLine_drops_unknown_period()
    {
        var limits = RateLimitsJson.ParseStatusLine(
            """{"rate_limits":{"spend_limit":{"used_percentage":50,"period":"hourly"}}}""");

        Assert.NotNull(limits);
        Assert.Equal(new SpendLimit(50, null, null, null, null), limits.Spend);
    }

    [Fact]
    public void WriteUsageFile_contains_only_rate_limit_fields_and_timestamp()
    {
        var snapshot = new UsageSnapshot(RateLimitsJson.ParseStatusLine(Full)!, T(1738420000));

        var json = RateLimitsJson.WriteUsageFile(snapshot);

        Assert.DoesNotContain("session_id", json);
        Assert.DoesNotContain("transcript_path", json);
        Assert.DoesNotContain("cwd", json);
        using var doc = JsonDocument.Parse(json);
        var names = doc.RootElement.EnumerateObject().Select(p => p.Name).OrderBy(n => n).ToArray();
        Assert.Equal(new[] { "observed_at", "rate_limits" }, names);
    }

    [Fact]
    public void UsageFile_round_trips()
    {
        var snapshot = new UsageSnapshot(RateLimitsJson.ParseStatusLine(Full)!, T(1738420000));

        var parsed = RateLimitsJson.ParseUsageFile(RateLimitsJson.WriteUsageFile(snapshot));

        Assert.Equal(snapshot, parsed);
    }

    [Theory]
    [InlineData("""{"rate_limits":{"five_hour":{"used_percentage":1}}}""")]
    [InlineData("""{"observed_at":1738420000}""")]
    [InlineData("""{"observed_at":1738420000,"rate_limits":{}}""")]
    [InlineData("nope")]
    public void ParseUsageFile_returns_null_without_observed_at_or_rate_limits(string input)
    {
        Assert.Null(RateLimitsJson.ParseUsageFile(input));
    }

    [Fact]
    public void WriteUsageFile_and_ParseUsageFile_round_trip_window_observed_at()
    {
        var at = T(1738400000);
        var snap = new UsageSnapshot(new RateLimits(
            new LimitWindow(10, T(1738425600), at),
            new LimitWindow(20, T(1738857600)),
            new SpendLimit(30, T(1740787200), 3, 10, "monthly", at)), T(1738500000));

        var json = RateLimitsJson.WriteUsageFile(snap);
        using (var doc = JsonDocument.Parse(json))
        {
            var rl = doc.RootElement.GetProperty("rate_limits");
            Assert.Equal(1738400000, rl.GetProperty("five_hour").GetProperty("observed_at").GetInt64());
            Assert.False(rl.GetProperty("seven_day").TryGetProperty("observed_at", out _));
        }
        Assert.Equal(snap, RateLimitsJson.ParseUsageFile(json));
    }

    [Fact]
    public void ParseUsageFile_reads_a_milestone_1_file_without_window_times()
    {
        var snap = RateLimitsJson.ParseUsageFile("""{"observed_at":1738500000,"rate_limits":{"five_hour":{"used_percentage":10,"resets_at":1738425600}}}""");
        Assert.Null(snap!.Limits.FiveHour!.ObservedAt);
    }

    [Fact]
    public void ParseStatusLine_ignores_observed_at_inside_windows()
    {
        var limits = RateLimitsJson.ParseStatusLine("""{"rate_limits":{"five_hour":{"used_percentage":10,"resets_at":1738425600,"observed_at":1738400000},"spend_limit":{"used_percentage":5,"observed_at":1738400000}}}""");
        Assert.Null(limits!.FiveHour!.ObservedAt);
        Assert.Null(limits.Spend!.ObservedAt);
    }
}
