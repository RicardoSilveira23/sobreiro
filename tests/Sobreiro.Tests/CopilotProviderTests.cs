using System.Net;
using Sobreiro.Core.Copilot;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Live;
using Sobreiro.Core.Polling;

namespace Sobreiro.Tests;

public class CopilotProviderTests
{
    private static readonly DateTimeOffset Now = new(2026, 11, 10, 12, 0, 0, TimeSpan.Zero);

    private const string Business = """
        {
          "copilot_plan": "business",
          "quota_reset_date_utc": "2026-12-01T00:00:00.000Z",
          "quota_snapshots": {
            "premium_interactions": {
              "entitlement": 15000, "remaining": 12355, "quota_remaining": 12355.8,
              "unlimited": false, "has_quota": true, "timestamp_utc": "2026-11-10T09:25:19.365Z"
            }
          }
        }
        """;

    private const string OkLog = "copilot Ok env=ok credential-manager=skipped gh=skipped";

    private static CopilotReading Live(DateTimeOffset at) => new(15000, 2645, Now.AddDays(18), at, CopilotSource.Live);

    private static CopilotReading Cache(DateTimeOffset at) => new(15000, 2645, Now.AddDays(18), at, CopilotSource.Cache);

    private static CopilotPollResult Poll(ProviderState state, CopilotReading? reading, string log) =>
        new(LiveSchedule.Start(Now), state, reading, log);

    private sealed class Rig
    {
        public Func<LiveSchedule, DateTimeOffset, CancellationToken, Task<CopilotPollResult>> PollFunc { get; set; } =
            (_, _, _) => Task.FromResult(Poll(ProviderState.Ok, Live(Now), OkLog));

        public Func<CopilotReading?> CacheFunc { get; set; } = () => null;
        public List<string> Logged { get; } = [];
        public CopilotProvider Provider { get; }

        public Rig() => Provider = new CopilotProvider((s, at, ct) => PollFunc(s, at, ct), () => CacheFunc(), Logged.Add);

        public void Returns(CopilotPollResult result, CopilotReading? cache = null)
        {
            PollFunc = (_, _, _) => Task.FromResult(result);
            CacheFunc = () => cache;
        }

        public async Task<ProviderPoll> Kept()
        {
            var poll = await Provider.PollAsync(LiveSchedule.Start(Now), Now, CancellationToken.None);
            poll.Keep!(Now);
            return poll;
        }
    }

    [Fact]
    public void It_carries_copilots_name() => Assert.Equal("GitHub Copilot", new Rig().Provider.Name);

    [Fact]
    public void Nothing_is_shown_before_a_poll()
    {
        var rig = new Rig();
        Assert.Null(rig.Provider.Shown);
        Assert.Empty(rig.Provider.Limits(Now));
    }

    [Fact]
    public async Task A_poll_passes_on_the_schedule_the_state_and_the_line()
    {
        var rig = new Rig();
        var schedule = LiveSchedule.Start(Now);
        rig.Returns(new CopilotPollResult(schedule, ProviderState.Ok, Live(Now), OkLog));
        var poll = await rig.Provider.PollAsync(LiveSchedule.Start(Now), Now, CancellationToken.None);
        Assert.Same(schedule, poll.Schedule);
        Assert.Equal(ProviderState.Ok, poll.State);
        Assert.Equal("copilot Ok env=ok credential-manager=skipped gh=skipped", poll.Log);
    }

    [Fact]
    public async Task The_reading_is_shown_only_after_keep()
    {
        var rig = new Rig();
        var live = Live(Now);
        rig.Returns(Poll(ProviderState.Ok, live, OkLog));
        var poll = await rig.Provider.PollAsync(LiveSchedule.Start(Now), Now, CancellationToken.None);
        Assert.Null(rig.Provider.Shown);
        poll.Keep!(Now);
        Assert.Same(live, rig.Provider.Shown);
        var limit = Assert.Single(rig.Provider.Limits(Now));
        Assert.Equal(LimitKind.Credits, limit.Kind);
    }

    [Fact]
    public async Task The_later_of_live_and_cache_is_shown()
    {
        var rig = new Rig();
        rig.Returns(Poll(ProviderState.Ok, Live(Now), OkLog), Cache(Now.AddMinutes(5)));
        await rig.Kept();
        Assert.Equal(CopilotSource.Cache, rig.Provider.Shown!.Source);

        var older = new Rig();
        older.Returns(Poll(ProviderState.Ok, Live(Now), OkLog), Cache(Now.AddMinutes(-5)));
        await older.Kept();
        Assert.Equal(CopilotSource.Live, older.Provider.Shown!.Source);
    }

    [Fact]
    public async Task The_line_says_how_the_cache_was_used()
    {
        var none = new Rig();
        none.Returns(Poll(ProviderState.Ok, Live(Now), OkLog));
        await none.Kept();
        Assert.EndsWith(" cache=none", Assert.Single(none.Logged));

        var older = new Rig();
        older.Returns(Poll(ProviderState.Ok, Live(Now), OkLog), Cache(Now.AddMinutes(-5)));
        await older.Kept();
        Assert.EndsWith(" cache=older", Assert.Single(older.Logged));

        var used = new Rig();
        used.Returns(Poll(ProviderState.Ok, Live(Now), OkLog), Cache(Now.AddMinutes(5)));
        await used.Kept();
        Assert.EndsWith(" cache=used", Assert.Single(used.Logged));
    }

    [Fact]
    public async Task The_line_is_logged_only_when_it_changes()
    {
        var rig = new Rig();
        rig.Returns(Poll(ProviderState.Ok, Live(Now), OkLog));
        await rig.Kept();
        await rig.Kept();
        Assert.Single(rig.Logged);

        rig.Returns(Poll(ProviderState.NoSignIn, null, "copilot NoSignIn env=none credential-manager=none gh=none"));
        await rig.Kept();
        Assert.Equal(2, rig.Logged.Count);
    }

    [Fact]
    public async Task A_poll_that_is_not_kept_logs_nothing()
    {
        var rig = new Rig();
        await rig.Provider.PollAsync(LiveSchedule.Start(Now), Now, CancellationToken.None);
        Assert.Empty(rig.Logged);
    }

    [Fact]
    public async Task A_failed_poll_keeps_the_live_reading()
    {
        var rig = new Rig();
        var live = Live(Now);
        rig.Returns(Poll(ProviderState.Ok, live, OkLog));
        await rig.Kept();
        rig.Returns(Poll(ProviderState.Offline, null, "copilot Offline env=unreachable credential-manager=skipped gh=skipped"));
        await rig.Kept();
        Assert.Same(live, rig.Provider.Shown);
    }

    [Fact]
    public async Task A_cache_that_cannot_be_read_keeps_the_one_before()
    {
        var rig = new Rig();
        var cache = Cache(Now);
        rig.Returns(Poll(ProviderState.Offline, null, "copilot Offline env=unreachable credential-manager=skipped gh=skipped"), cache);
        await rig.Kept();
        rig.CacheFunc = () => null;
        await rig.Kept();
        Assert.Same(cache, rig.Provider.Shown);
    }

    [Fact]
    public async Task The_early_cache_is_taken_only_when_none_is_held()
    {
        var rig = new Rig();
        rig.Provider.TakeEarly(Cache(Now));
        Assert.Equal(CopilotSource.Cache, rig.Provider.Shown!.Source);

        var held = new Rig();
        var newer = Cache(Now.AddMinutes(5));
        held.Returns(Poll(ProviderState.Offline, null, "copilot Offline env=unreachable credential-manager=skipped gh=skipped"), newer);
        await held.Kept();
        held.Provider.TakeEarly(Cache(Now.AddMinutes(-60)));
        Assert.Same(newer, held.Provider.Shown);
    }

    [Fact]
    public async Task A_poll_that_throws_counts_as_a_bad_answer()
    {
        var rig = new Rig();
        rig.PollFunc = (_, _, _) => throw new InvalidOperationException("ghp_SECRET");
        var poll = await rig.Provider.PollAsync(LiveSchedule.Start(Now), Now, CancellationToken.None);
        Assert.Equal(ProviderState.BadAnswer, poll.State);
        Assert.Equal("copilot BadAnswer Failed", poll.Log);
        Assert.Equal(Now.AddMinutes(2), poll.Schedule.NextDue);
        Assert.Equal(1, poll.Schedule.Misses);
        poll.Keep!(Now);
        Assert.Equal(["copilot BadAnswer Failed cache=none"], rig.Logged);
        Assert.DoesNotContain("ghp_SECRET", poll.Log);
        Assert.DoesNotContain(rig.Logged, line => line.Contains("ghp_SECRET"));
    }

    [Fact]
    public async Task A_cache_read_that_throws_counts_as_a_bad_answer()
    {
        var rig = new Rig();
        rig.CacheFunc = () => throw new IOException("ghp_SECRET");
        var poll = await rig.Provider.PollAsync(LiveSchedule.Start(Now), Now, CancellationToken.None);
        Assert.Equal(ProviderState.BadAnswer, poll.State);
        poll.Keep!(Now);
        Assert.Null(rig.Provider.Shown);
    }

    [Fact]
    public async Task Cancellation_passes_through()
    {
        var rig = new Rig();
        using var cts = new CancellationTokenSource();
        cts.Cancel();
        rig.PollFunc = (_, _, ct) => throw new OperationCanceledException(ct);
        await Assert.ThrowsAsync<OperationCanceledException>(
            () => rig.Provider.PollAsync(LiveSchedule.Start(Now), Now, cts.Token));
    }

    private sealed class FakeHandler(Func<HttpRequestMessage, HttpResponseMessage> respond) : HttpMessageHandler
    {
        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken ct) =>
            Task.FromResult(respond(request));
    }

    [Fact]
    public async Task The_public_constructor_asks_github_and_reads_the_cache_file()
    {
        var client = new CopilotUsageClient(
            new HttpClient(new FakeHandler(_ => new HttpResponseMessage(HttpStatusCode.OK) { Content = new StringContent(Business) })),
            "Sobreiro/0.7.0");
        var path = Path.Combine(Path.GetTempPath(), "sobreiro-tests-" + Guid.NewGuid().ToString("N"), "cache.json");
        var logged = new List<string>();
        var provider = new CopilotProvider(
            client, [new CopilotSignIn(CopilotSourceKind.Env, () => new[] { "ghp_SECRET" })], path, logged.Add);

        var poll = await provider.PollAsync(LiveSchedule.Start(Now), Now, CancellationToken.None);
        poll.Keep!(Now);

        Assert.Equal(CopilotSource.Live, provider.Shown!.Source);
        Assert.DoesNotContain("ghp_SECRET", poll.Log);
        Assert.DoesNotContain(logged, line => line.Contains("ghp_SECRET"));
        Assert.Null(provider.ReadCache());
    }
}
