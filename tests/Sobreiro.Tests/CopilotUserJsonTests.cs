using Sobreiro.Core.Copilot;

namespace Sobreiro.Tests;

public class CopilotUserJsonTests
{
    private static readonly DateTimeOffset Now = DateTimeOffset.Parse("2026-11-10T12:00:00Z");

    private const string Business = """
        {
          "copilot_plan": "business",
          "token_based_billing": true,
          "quota_reset_date_utc": "2026-12-01T00:00:00.000Z",
          "quota_snapshots": {
            "premium_interactions": {
              "entitlement": 15000, "remaining": 12355, "quota_remaining": 12355.8,
              "percent_remaining": 82.3, "unlimited": false, "has_quota": true,
              "overage_permitted": true, "overage_count": 0, "overage_entitlement": 0,
              "token_based_billing": true, "timestamp_utc": "2026-11-10T09:25:19.365Z"
            },
            "chat": { "unlimited": true },
            "completions": { "unlimited": true }
          }
        }
        """;

    private static string Interactions(string fields) =>
        "{\"quota_snapshots\":{\"premium_interactions\":{" + fields + "}}}";

    [Fact]
    public void Business_seat_reads_used_from_quota_remaining()
    {
        var r = CopilotUserJson.Parse(Business, Now)!;
        Assert.Equal(15000, r.Budget);
        Assert.Equal(2644.2, r.Used, 1);
        Assert.Equal(17.628, r.Percent, 3);
        Assert.Equal(DateTimeOffset.Parse("2026-12-01T00:00:00Z"), r.ResetsAt);
        Assert.Equal(DateTimeOffset.Parse("2026-11-10T09:25:19.365Z"), r.ObservedAt);
        Assert.Equal(CopilotSource.Live, r.Source);
    }

    [Fact]
    public void Premium_models_wins_over_premium_interactions()
    {
        var body = """
            {"quota_snapshots":{
              "premium_models":{"entitlement":300,"quota_remaining":150},
              "premium_interactions":{"entitlement":15000,"quota_remaining":12000}}}
            """;
        var r = CopilotUserJson.Parse(body, Now)!;
        Assert.Equal(300, r.Budget);
        Assert.Equal(150, r.Used, 3);
    }

    [Fact]
    public void Free_plan_has_no_budget()
    {
        var body = """
            {"quota_reset_at":0,"quota_snapshots":{
              "premium_interactions":{"entitlement":0,"has_quota":false,"remaining":0},
              "chat":{"entitlement":200,"remaining":150},
              "completions":{"entitlement":2000,"remaining":1900}}}
            """;
        Assert.Null(CopilotUserJson.Parse(body, Now));
    }

    [Fact]
    public void Unlimited_has_no_budget() =>
        Assert.Null(CopilotUserJson.Parse(Interactions("\"unlimited\":true,\"entitlement\":15000,\"quota_remaining\":100"), Now));

    [Fact]
    public void Used_falls_back_to_remaining()
    {
        var r = CopilotUserJson.Parse(Interactions("\"entitlement\":15000,\"remaining\":12355"), Now)!;
        Assert.Equal(2645, r.Used, 3);
    }

    [Fact]
    public void Used_falls_back_to_percent_remaining()
    {
        var r = CopilotUserJson.Parse(Interactions("\"entitlement\":15000,\"percent_remaining\":82.3"), Now)!;
        Assert.Equal(2655, r.Used, 3);
    }

    [Fact]
    public void Used_falls_back_to_credits_used()
    {
        var r = CopilotUserJson.Parse(Interactions("\"entitlement\":15000,\"credits_used\":2644.2"), Now)!;
        Assert.Equal(2644.2, r.Used, 3);
    }

    [Fact]
    public void No_used_field_means_nothing() =>
        Assert.Null(CopilotUserJson.Parse(Interactions("\"entitlement\":15000"), Now));

    [Fact]
    public void Overage_count_reads_over_budget()
    {
        var r = CopilotUserJson.Parse(Interactions("\"entitlement\":15000,\"quota_remaining\":0,\"overage_count\":210"), Now)!;
        Assert.Equal(15210, r.Used, 3);
        Assert.Equal(101.4, r.Percent, 3);
    }

    [Fact]
    public void Negative_quota_remaining_reads_over_budget()
    {
        var r = CopilotUserJson.Parse(Interactions("\"entitlement\":15000,\"quota_remaining\":-50"), Now)!;
        Assert.Equal(15050, r.Used, 3);
    }

    [Fact]
    public void Offset_timestamp_is_kept()
    {
        var r = CopilotUserJson.Parse(
            Interactions("\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-09T03:06:03.986-08:00\""), Now)!;
        Assert.Equal(DateTimeOffset.Parse("2026-11-09T11:06:03.986Z").UtcDateTime, r.ObservedAt.UtcDateTime);
    }

    [Fact]
    public void Missing_timestamp_uses_now()
    {
        var r = CopilotUserJson.Parse(Interactions("\"entitlement\":15000,\"quota_remaining\":100"), Now)!;
        Assert.Equal(Now, r.ObservedAt);
    }

    [Fact]
    public void Missing_reset_date_gives_null_ResetsAt()
    {
        var r = CopilotUserJson.Parse(Interactions("\"entitlement\":15000,\"quota_remaining\":100"), Now)!;
        Assert.Null(r.ResetsAt);
    }

    [Theory]
    [InlineData("9999-12-31T23:59:59Z")]
    [InlineData("2028-01-01T00:00:00Z")]
    public void Absurd_reset_date_counts_as_absent(string reset)
    {
        var json = "{\"quota_reset_date_utc\":\"" + reset + "\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}";
        Assert.Null(CopilotUserJson.Parse(json, Now)!.ResetsAt);
    }

    [Fact]
    public void Reset_date_within_400_days_is_kept()
    {
        var json = "{\"quota_reset_date_utc\":\"2027-10-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}";
        Assert.Equal(DateTimeOffset.Parse("2027-10-01T00:00:00Z"), CopilotUserJson.Parse(json, Now)!.ResetsAt);
    }

    [Theory]
    [InlineData("")]
    [InlineData("[]")]
    [InlineData("{")]
    [InlineData("{\"quota_snapshots\":5}")]
    public void Malformed_bodies_give_null(string body) =>
        Assert.Null(CopilotUserJson.Parse(body, Now));
}
