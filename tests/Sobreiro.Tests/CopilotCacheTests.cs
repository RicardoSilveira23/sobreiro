using Sobreiro.Core.Copilot;

namespace Sobreiro.Tests;

public class CopilotCacheTests
{
    private const string Header = "// Copilot user cache\n// do not edit\n";

    private static string Business(int remaining, string? timestamp = null) =>
        "{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":"
        + remaining + (timestamp is null ? "" : ",\"timestamp_utc\":\"" + timestamp + "\"") + "}}}";

    private const string Free =
        "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":0,\"has_quota\":false,\"remaining\":0}}}";

    private static string Entry(string key, string retrievedAt, string response) =>
        "\"" + key + "\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"" + retrievedAt + "\",\"response\":" + response + "}";

    private static string Cache(params string[] entries) =>
        Header + "{\"copilotUserCache\":{" + string.Join(",", entries) + "}}";

    [Fact]
    public void Newest_entry_with_a_budget_wins()
    {
        var text = Cache(
            Entry("v1:a", "2026-11-10T09:25:00Z", Business(12000)),
            Entry("v1:b", "2026-11-10T10:00:00Z", Business(10000)));
        Assert.Equal(5000, CopilotCache.Parse(text)!.Used, 3);
    }

    [Fact]
    public void Newer_free_entry_does_not_hide_business()
    {
        var text = Cache(
            Entry("v1:a", "2026-11-10T09:25:00Z", Business(12000)),
            Entry("v1:b", "2026-11-10T11:00:00Z", Free));
        Assert.Equal(3000, CopilotCache.Parse(text)!.Used, 3);
    }

    [Fact]
    public void Reading_is_marked_cache()
    {
        var text = Cache(Entry("v1:a", "2026-11-10T09:25:00Z", Business(12000)));
        Assert.Equal(CopilotSource.Cache, CopilotCache.Parse(text)!.Source);
    }

    [Fact]
    public void Retrieved_at_is_used_when_snapshot_has_no_timestamp()
    {
        var text = Cache(Entry("v1:a", "2026-11-10T09:25:00Z", Business(12000)));
        Assert.Equal(DateTimeOffset.Parse("2026-11-10T09:25:00Z"), CopilotCache.Parse(text)!.ObservedAt);
    }

    [Fact]
    public void Comment_lines_with_leading_spaces_are_dropped()
    {
        var text = "   // x\n" + Cache(Entry("v1:a", "2026-11-10T09:25:00Z", Business(12000)));
        Assert.NotNull(CopilotCache.Parse(text));
    }

    [Fact]
    public void No_entries_with_a_budget_gives_null()
    {
        var text = Cache(Entry("v1:a", "2026-11-10T09:25:00Z", Free));
        Assert.Null(CopilotCache.Parse(text));
    }

    [Theory]
    [InlineData("")]
    [InlineData("// only a comment")]
    [InlineData("{\"copilotUserCache\":{\"v1:a\":{\"retrievedAt\":\"x\",\"response\":{")]
    public void Malformed_or_truncated_cache_gives_null(string text) =>
        Assert.Null(CopilotCache.Parse(text));

    [Fact]
    public void Missing_file_gives_null() =>
        Assert.Null(CopilotCache.Read(Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N"), "none.json")));

    [Fact]
    public void Invalid_path_gives_null() => Assert.Null(CopilotCache.Read("bad\0path.json"));

    [Fact]
    public void Locked_file_gives_null()
    {
        var path = Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N") + ".json");
        File.WriteAllText(path, Cache(Entry("v1:a", "2026-11-10T09:25:00Z", Business(12000))));
        try
        {
            using var held = new FileStream(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
            Assert.Null(CopilotCache.Read(path));
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Fact]
    public void Read_parses_an_existing_file()
    {
        var path = Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N") + ".json");
        File.WriteAllText(path, Cache(Entry("v1:a", "2026-11-10T09:25:00Z", Business(12000))));
        try
        {
            Assert.Equal(3000, CopilotCache.Read(path)!.Used, 3);
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Fact]
    public void Default_path_is_under_local_app_data() =>
        Assert.Equal(@"C:\L\copilot\copilot-user-cache.json",
            CopilotCache.DefaultPath(k => k == "LOCALAPPDATA" ? @"C:\L" : null));
}
