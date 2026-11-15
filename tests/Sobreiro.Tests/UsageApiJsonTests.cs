using Sobreiro.Core.Live;

namespace Sobreiro.Tests;

public class UsageApiJsonTests
{
    private const string ListShape =
        "{\"limits\":[{\"group\":\"session\",\"percent\":27,\"resets_at\":\"2026-11-09T14:00:00Z\"}," +
        "{\"group\":\"weekly\",\"percent\":74,\"resets_at\":\"2026-11-13T00:00:00Z\"}," +
        "{\"group\":\"weekly\",\"percent\":40,\"scope\":{\"model\":\"Fable\"}}]}";

    [Fact]
    public void The_list_shape_gives_session_and_weekly_and_ignores_scoped_entries()
    {
        var limits = UsageApiJson.Parse(ListShape)!;
        Assert.Equal(27, limits.FiveHour!.UsedPercentage);
        Assert.Equal(new DateTimeOffset(2026, 11, 9, 14, 0, 0, TimeSpan.Zero), limits.FiveHour.ResetsAt);
        Assert.Equal(74, limits.SevenDay!.UsedPercentage);
        Assert.Equal(new DateTimeOffset(2026, 11, 13, 0, 0, 0, TimeSpan.Zero), limits.SevenDay.ResetsAt);
        Assert.Null(limits.FiveHour.ObservedAt);
        Assert.Null(limits.Spend);
    }

    [Fact]
    public void The_keyed_shape_works_and_ignores_per_model_and_app_entries()
    {
        const string json =
            "{\"five_hour\":{\"utilization\":27.5,\"resets_at\":\"2026-11-09T14:00:00Z\"}," +
            "\"seven_day\":{\"utilization\":74,\"resets_at\":\"2026-11-13T00:00:00+00:00\"}," +
            "\"seven_day_opus\":{\"utilization\":90,\"resets_at\":null},\"seven_day_oauth_apps\":{\"utilization\":5}}";
        var limits = UsageApiJson.Parse(json)!;
        Assert.Equal(27.5, limits.FiveHour!.UsedPercentage);
        Assert.Equal(74, limits.SevenDay!.UsedPercentage);
        Assert.NotNull(limits.SevenDay.ResetsAt);
    }

    [Fact]
    public void A_list_with_no_usable_entry_falls_back_to_the_keys()
    {
        const string json =
            "{\"limits\":[{\"group\":\"weekly\",\"percent\":40,\"scope\":{\"model\":\"Fable\"}}]," +
            "\"five_hour\":{\"utilization\":10}}";
        var limits = UsageApiJson.Parse(json)!;
        Assert.Equal(10, limits.FiveHour!.UsedPercentage);
        Assert.Null(limits.SevenDay);
    }

    [Fact]
    public void A_missing_reset_is_a_null_reset()
    {
        var limits = UsageApiJson.Parse("{\"limits\":[{\"group\":\"session\",\"percent\":3}]}")!;
        Assert.Equal(3, limits.FiveHour!.UsedPercentage);
        Assert.Null(limits.FiveHour.ResetsAt);
    }

    [Fact]
    public void A_string_percent_drops_that_window()
    {
        var limits = UsageApiJson.Parse(
            "{\"limits\":[{\"group\":\"session\",\"percent\":\"27\"},{\"group\":\"weekly\",\"percent\":50}]}")!;
        Assert.Null(limits.FiveHour);
        Assert.Equal(50, limits.SevenDay!.UsedPercentage);
    }

    [Fact]
    public void A_bad_reset_text_is_a_null_reset()
    {
        var limits = UsageApiJson.Parse("{\"limits\":[{\"group\":\"session\",\"percent\":3,\"resets_at\":\"soon\"}]}")!;
        Assert.Null(limits.FiveHour!.ResetsAt);
    }

    [Theory]
    [InlineData("{}")]
    [InlineData("[]")]
    [InlineData("")]
    [InlineData("{ broken")]
    [InlineData("null")]
    [InlineData("{\"limits\":\"x\",\"five_hour\":5}")]
    [InlineData("{\"limits\":[1,null,\"a\",{\"group\":5}]}")]
    public void Nothing_usable_is_null(string json) => Assert.Null(UsageApiJson.Parse(json));

    [Fact]
    public void A_window_missing_from_the_list_comes_from_the_keys()
    {
        var limits = UsageApiJson.Parse(
            "{\"limits\":[{\"group\":\"weekly\",\"percent\":60}],\"five_hour\":{\"utilization\":12},\"seven_day\":{\"utilization\":99}}")!;
        Assert.Equal(12, limits.FiveHour!.UsedPercentage);
        Assert.Equal(60, limits.SevenDay!.UsedPercentage);
    }
}
