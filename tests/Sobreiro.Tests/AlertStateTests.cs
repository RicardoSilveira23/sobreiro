using System.Text.Json;
using Sobreiro.Core.Alerts;
using Sobreiro.Core.Limits;

namespace Sobreiro.Tests;

public class AlertStateTests : IDisposable
{
    private readonly string _dir = Path.Combine(Path.GetTempPath(), "sobreiro-tests-" + Guid.NewGuid().ToString("N"));
    private string PathFor() => Path.Combine(_dir, "alerts.json");

    public void Dispose()
    {
        if (Directory.Exists(_dir)) Directory.Delete(_dir, recursive: true);
    }

    [Fact]
    public void Round_trips()
    {
        var state = new AlertState(new Dictionary<LimitKind, AlertMark>
        {
            [LimitKind.Session] = new(DateTimeOffset.FromUnixTimeSeconds(1738425600), 80),
            [LimitKind.Weekly] = new(null, 95),
        });
        state.Save(PathFor());

        var loaded = AlertState.Load(PathFor());
        Assert.Equal(2, loaded.Marks.Count);
        Assert.Equal(new AlertMark(DateTimeOffset.FromUnixTimeSeconds(1738425600), 80), loaded.Marks[LimitKind.Session]);
        Assert.Equal(new AlertMark(null, 95), loaded.Marks[LimitKind.Weekly]);
    }

    [Fact]
    public void Uses_the_documented_keys_and_values()
    {
        new AlertState(new Dictionary<LimitKind, AlertMark>
        {
            [LimitKind.Spend] = new(DateTimeOffset.FromUnixTimeSeconds(100), 80),
        }).Save(PathFor());
        using var doc = JsonDocument.Parse(File.ReadAllText(PathFor()));
        var spend = doc.RootElement.GetProperty("spend");
        Assert.Equal(100, spend.GetProperty("resets_at").GetInt64());
        Assert.Equal(80, spend.GetProperty("threshold").GetInt32());
    }

    [Fact]
    public void Missing_or_corrupt_file_gives_Empty()
    {
        Assert.Empty(AlertState.Load(PathFor()).Marks);
        Directory.CreateDirectory(_dir);
        File.WriteAllText(PathFor(), "{oops");
        Assert.Empty(AlertState.Load(PathFor()).Marks);
        File.WriteAllText(PathFor(), "[1]");
        Assert.Empty(AlertState.Load(PathFor()).Marks);
    }

    [Fact]
    public void Credits_mark_round_trips_as_copilot_credits()
    {
        var mark = new AlertMark(DateTimeOffset.FromUnixTimeSeconds(1764547200), 95);
        new AlertState(new Dictionary<LimitKind, AlertMark> { [LimitKind.Credits] = mark }).Save(PathFor());

        using var doc = JsonDocument.Parse(File.ReadAllText(PathFor()));
        Assert.True(doc.RootElement.TryGetProperty("copilot_credits", out _));
        Assert.False(doc.RootElement.TryGetProperty("spend", out _));
        Assert.Equal(mark, AlertState.Load(PathFor()).Marks[LimitKind.Credits]);
    }

    [Fact]
    public void Old_file_without_credits_still_loads()
    {
        Directory.CreateDirectory(_dir);
        File.WriteAllText(PathFor(), "{\"session\":{\"resets_at\":1738425600,\"threshold\":80}}");
        var loaded = AlertState.Load(PathFor());
        var only = Assert.Single(loaded.Marks);
        Assert.Equal(LimitKind.Session, only.Key);
    }
}
