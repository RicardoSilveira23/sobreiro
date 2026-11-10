using System.Text.Json;
using Sobreiro.Core.Bridge;
using Sobreiro.Core.Usage;

namespace Sobreiro.Tests;

public class BridgeRunnerTests : IDisposable
{
    private const string WithLimits = """{"session_id":"abc","transcript_path":"C:/x.jsonl","cwd":"C:/x","rate_limits":{"five_hour":{"used_percentage":23.4,"resets_at":1999999999},"seven_day":{"used_percentage":41,"resets_at":1999999999},"spend_limit":{"used_percentage":62,"resets_at":1999999999}}}""";
    private const string DefaultLine = "5h 23% · 7d 41% · spend 62%\n";
    private static readonly DateTimeOffset Now = DateTimeOffset.FromUnixTimeSeconds(1_800_000_000);

    private readonly string _dir = Directory.CreateDirectory(Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString())).FullName;
    private string Usage => Path.Combine(_dir, "usage.json");
    private string Config => Path.Combine(_dir, "config.json");

    public void Dispose() => Directory.Delete(_dir, true);

    private (int Code, string Out) Run(string stdin, Func<string, string, ChainResult?>? chain = null, string? usage = null, bool insideChain = false)
    {
        var writer = new StringWriter();
        var code = new BridgeRunner(usage ?? Usage, Config, chain ?? ((_, _) => null), insideChain).Run(stdin, writer, Now);
        return (code, writer.ToString());
    }

    [Fact]
    public void Writes_usage_and_prints_default_line_without_chain()
    {
        var (code, output) = Run(WithLimits);
        Assert.Equal(0, code);
        Assert.Equal(DefaultLine, output);
        Assert.Equal(Now, UsageFile.Read(Usage)!.ObservedAt);
    }

    [Fact]
    public void Does_not_write_without_rate_limits()
    {
        var (code, output) = Run("""{"session_id":"abc"}""");
        Assert.Equal(0, code);
        Assert.Equal("5h – · 7d –\n", output);
        Assert.False(File.Exists(Usage));
    }

    [Fact]
    public void Keeps_previous_usage_when_rate_limits_absent()
    {
        Run(WithLimits);
        var before = File.ReadAllText(Usage);
        Run("not json at all");
        Assert.Equal(before, File.ReadAllText(Usage));
    }

    [Fact]
    public void Runs_chain_with_full_stdin_and_passes_output_and_exit_code()
    {
        File.WriteAllText(Config, """{"chained_command": "my-line"}""");
        var calls = new List<(string Command, string Stdin)>();
        var (code, output) = Run(WithLimits, (c, s) => { calls.Add((c, s)); return new ChainResult(7, "theirs"); });
        Assert.Equal(7, code);
        Assert.Equal("theirs", output);
        Assert.Equal([("my-line", WithLimits)], calls);
        Assert.NotNull(UsageFile.Read(Usage));
    }

    [Fact]
    public void Does_not_run_chain_when_already_inside_a_chain()
    {
        File.WriteAllText(Config, """{"chained_command": "my-line"}""");
        var called = false;
        var (code, output) = Run(WithLimits, (_, _) => { called = true; return new ChainResult(7, "theirs"); }, insideChain: true);
        Assert.False(called);
        Assert.Equal(0, code);
        Assert.Equal(DefaultLine, output);
        Assert.Equal(Now, UsageFile.Read(Usage)!.ObservedAt);
    }

    [Fact]
    public void Falls_back_to_default_line_when_chain_cannot_start()
    {
        File.WriteAllText(Config, """{"chained_command": "my-line"}""");
        var (code, output) = Run(WithLimits, (_, _) => null);
        Assert.Equal(0, code);
        Assert.Equal(DefaultLine, output);
    }

    [Fact]
    public void Still_prints_when_usage_write_fails()
    {
        var (code, output) = Run(WithLimits, usage: _dir); // an existing directory
        Assert.Equal(0, code);
        Assert.Equal(DefaultLine, output);
    }

    [Fact]
    public void Still_prints_when_chain_throws()
    {
        File.WriteAllText(Config, """{"chained_command": "my-line"}""");
        var (code, output) = Run(WithLimits, (_, _) => throw new IOException("boom"));
        Assert.Equal(0, code);
        Assert.Equal(DefaultLine, output);
    }

    [Fact]
    public void Usage_file_never_contains_other_stdin_fields()
    {
        Run(WithLimits);
        var text = File.ReadAllText(Usage);
        foreach (var leaked in new[] { "session_id", "transcript_path", "cwd", "abc", "C:/x.jsonl" })
            Assert.DoesNotContain(leaked, text);
        using var doc = JsonDocument.Parse(text);
        Assert.Equal(["observed_at", "rate_limits"], doc.RootElement.EnumerateObject().Select(p => p.Name));
    }

    [Fact]
    public void Run_does_not_lower_same_period_reading()
    {
        Run(WithLimits.Replace("23.4", "60"));
        var (_, output) = Run(WithLimits.Replace("23.4", "40"));

        Assert.Equal("5h 40% · 7d 41% · spend 62%\n", output);
        Assert.Equal(60, UsageFile.Read(Usage)!.Limits.FiveHour!.UsedPercentage);
    }
}
