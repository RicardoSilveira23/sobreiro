using System.Net;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Live;
using Sobreiro.Core.Polling;
using Sobreiro.Core.Usage;

namespace Sobreiro.Tests;

public class ClaudeProviderTests : IDisposable
{
    private static readonly DateTimeOffset Now = new(2026, 11, 9, 12, 0, 0, TimeSpan.Zero);
    private const string Secret = "sk-ant-oat01-SECRETVALUE";
    private const string Body = "{\"limits\":[{\"group\":\"session\",\"percent\":27,\"resets_at\":\"2026-11-09T14:00:00Z\"}]}";

    private static readonly RateLimits Reading = new(new LimitWindow(27, Now.AddHours(2)), null, null);

    private readonly string _dir = Path.Combine(Path.GetTempPath(), "sobreiro-tests-" + Guid.NewGuid().ToString("N"));
    private readonly LiveSchedule _start = LiveSchedule.Start(Now);
    private readonly List<string> _trace = [];

    public ClaudeProviderTests() => Directory.CreateDirectory(_dir);

    public void Dispose()
    {
        if (Directory.Exists(_dir)) Directory.Delete(_dir, recursive: true);
    }

    private string UsagePath => Path.Combine(_dir, "usage.json");

    private ClaudeProvider Provider(LivePollResult result) =>
        new((_, _, _) => Task.FromResult(result), UsagePath, _trace.Add);

    private LivePollResult Ok() => new(_start, ProviderState.Ok, Reading, "Ok");

    private sealed class CountingHandler(Func<HttpResponseMessage> respond) : HttpMessageHandler
    {
        public List<HttpRequestMessage> Seen { get; } = [];

        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken ct)
        {
            Seen.Add(request);
            return Task.FromResult(respond());
        }
    }

    [Fact]
    public void It_carries_claudes_name()
    {
        Assert.Equal("Claude Code", Provider(Ok()).Name);
    }

    [Fact]
    public async Task A_poll_passes_on_the_schedule_the_state_and_the_outcome()
    {
        var poll = await Provider(new LivePollResult(_start, ProviderState.Ok, Reading, "Ok"))
            .PollAsync(_start, Now, CancellationToken.None);

        Assert.Same(_start, poll.Schedule);
        Assert.Equal(ProviderState.Ok, poll.State);
        Assert.Equal("Live usage Ok", poll.Log);
    }

    [Fact]
    public async Task A_poll_without_a_reading_has_nothing_to_keep()
    {
        var poll = await Provider(new LivePollResult(_start, ProviderState.Expired, null, "Expired"))
            .PollAsync(_start, Now, CancellationToken.None);

        Assert.Null(poll.Keep);
        Assert.Equal("Live usage Expired", poll.Log);
    }

    [Fact]
    public async Task The_reading_reaches_usage_json_only_through_keep()
    {
        var provider = Provider(Ok());
        var poll = await provider.PollAsync(_start, Now, CancellationToken.None);

        Assert.False(File.Exists(UsagePath));
        Assert.Empty(provider.Limits(Now));

        poll.Keep!(Now);

        var limit = Assert.Single(provider.Limits(Now));
        Assert.Equal(LimitKind.Session, limit.Kind);
        Assert.Equal(27, limit.Percent);
    }

    [Fact]
    public async Task Keep_merges_over_what_the_file_holds()
    {
        var weekly = new RateLimits(null, new LimitWindow(60, Now.AddDays(3), Now.AddMinutes(-5)), null);
        UsageFile.Write(UsagePath, new UsageSnapshot(weekly, Now.AddMinutes(-5)));
        var provider = Provider(Ok());

        (await provider.PollAsync(_start, Now, CancellationToken.None)).Keep!(Now);

        var limits = provider.Limits(Now);
        Assert.Contains(limits, l => l.Kind == LimitKind.Weekly && l.Percent == 60);
        Assert.Contains(limits, l => l.Kind == LimitKind.Session && l.Percent == 27);
    }

    [Fact]
    public async Task A_reading_replaces_a_higher_stored_one_in_the_same_period()
    {
        var higher = new RateLimits(new LimitWindow(40, Now.AddHours(2), Now.AddMinutes(-5)), null, null);
        UsageFile.Write(UsagePath, new UsageSnapshot(higher, Now.AddMinutes(-5)));
        var provider = Provider(Ok());

        (await provider.PollAsync(_start, Now, CancellationToken.None)).Keep!(Now);

        var limit = Assert.Single(provider.Limits(Now));
        Assert.Equal(27, limit.Percent);
    }

    [Fact]
    public async Task A_held_usage_json_skips_the_reading()
    {
        File.WriteAllText(UsagePath, "{}");
        var before = File.ReadAllBytes(UsagePath);
        var poll = await Provider(Ok()).PollAsync(_start, Now, CancellationToken.None);

        using (new FileStream(UsagePath, FileMode.Open, FileAccess.ReadWrite, FileShare.None))
        {
            poll.Keep!(Now);
        }

        Assert.Contains("usage.json was busy, so this Live usage reading was not saved", _trace);
        Assert.Equal(before, File.ReadAllBytes(UsagePath));
    }

    [Fact]
    public async Task Limits_keep_the_last_snapshot_when_a_read_fails()
    {
        var provider = Provider(Ok());
        (await provider.PollAsync(_start, Now, CancellationToken.None)).Keep!(Now);
        Assert.Single(provider.Limits(Now));

        using var held = new FileStream(UsagePath, FileMode.Open, FileAccess.ReadWrite, FileShare.None);

        var limit = Assert.Single(provider.Limits(Now));
        Assert.Equal(LimitKind.Session, limit.Kind);
    }

    [Fact]
    public async Task The_public_constructor_reads_the_sign_in_and_asks()
    {
        var credentials = Path.Combine(_dir, ".credentials.json");
        File.WriteAllText(credentials, "{\"claudeAiOauth\":{\"accessToken\":\"" + Secret + "\",\"refreshToken\":\"r\",\"expiresAt\":"
            + Now.AddHours(2).ToUnixTimeMilliseconds() + "}}");
        var handler = new CountingHandler(() => new HttpResponseMessage(HttpStatusCode.OK) { Content = new StringContent(Body) });
        var provider = new ClaudeProvider(
            new LiveUsageClient(new HttpClient(handler), "Sobreiro/0.3.0"), credentials, UsagePath, _trace.Add);

        var poll = await provider.PollAsync(_start, Now, CancellationToken.None);
        poll.Keep!(Now);

        var limit = Assert.Single(provider.Limits(Now));
        Assert.Equal(LimitKind.Session, limit.Kind);
        Assert.Equal(27, limit.Percent);
        Assert.Single(handler.Seen);
        Assert.DoesNotContain(Secret, poll.Log);
        Assert.DoesNotContain(_trace, line => line.Contains(Secret));
    }

    [Fact]
    public async Task Cancellation_passes_through()
    {
        var provider = new ClaudeProvider(
            (_, _, ct) => { ct.ThrowIfCancellationRequested(); return Task.FromResult(Ok()); }, UsagePath, _trace.Add);
        using var cts = new CancellationTokenSource();
        cts.Cancel();

        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => provider.PollAsync(_start, Now, cts.Token));
    }
}
