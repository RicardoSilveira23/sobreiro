using Sobreiro.Core.Limits;
using Sobreiro.Core.Live;
using Sobreiro.Core.Polling;

namespace Sobreiro.Tests;

public class ProviderPollerTests
{
    private static readonly DateTimeOffset Now = new(2026, 11, 13, 12, 0, 0, TimeSpan.Zero);

    private sealed class FakeProvider : IProvider
    {
        public string Name => "Fake";
        public Func<LiveSchedule, DateTimeOffset, CancellationToken, Task<ProviderPoll>> Answer { get; set; } =
            (_, _, _) => Task.FromResult(Ok(Now));
        public List<CancellationToken> Tokens { get; } = [];
        public List<(LiveSchedule Schedule, DateTimeOffset Now)> Seen { get; } = [];

        public Task<ProviderPoll> PollAsync(LiveSchedule schedule, DateTimeOffset now, CancellationToken ct)
        {
            Tokens.Add(ct);
            Seen.Add((schedule, now));
            return Answer(schedule, now, ct);
        }

        public IReadOnlyList<Limit> Limits(DateTimeOffset now) => [];
    }

    private readonly FakeProvider _provider = new();
    private readonly List<PollRun> _runs = [];
    private readonly List<string> _traced = [];
    private readonly List<string> _order = [];
    private readonly List<(ProviderState State, int Misses)> _atRedraw = [];
    private int _changed;
    private readonly ProviderPoller _poller;

    public ProviderPollerTests()
    {
        _poller = new ProviderPoller(
            _provider, _runs.Add, () => { _changed++; _order.Add("changed"); _atRedraw.Add((_poller!.State, _poller.Misses)); }, _traced.Add);
    }

    private static ProviderPoll Ok(DateTimeOffset at, Action<DateTimeOffset>? keep = null) =>
        new(LiveSchedule.Start(at).AfterFetch(new LiveResult(LiveOutcome.Ok, null, null), at), ProviderState.Ok, "fake Ok", keep);

    private static ProviderPoll WithSchedule(LiveSchedule schedule) => new(schedule, ProviderState.Ok, "fake Ok");

    [Fact]
    public void Before_start_it_is_ok_with_no_misses()
    {
        Assert.Equal(ProviderState.Ok, _poller.State);
        Assert.Equal(0, _poller.Misses);
        _poller.Tick(Now);
        _poller.FlyoutOpened(Now);
        Assert.Empty(_runs);
    }

    [Fact]
    public void Start_polls_at_once_and_only_once()
    {
        _poller.Start(Now);
        Assert.Single(_runs);
        _poller.Start(Now);
        Assert.Single(_runs);
    }

    [Fact]
    public async Task The_poll_gets_the_schedule_and_time_it_started_with()
    {
        _poller.Start(Now);
        await _runs[0].RunAsync();
        var seen = Assert.Single(_provider.Seen);
        Assert.Equal(Now, seen.Now);
        Assert.Equal(Now, seen.Schedule.NextDue);
    }

    [Fact]
    public void One_poll_at_a_time()
    {
        _poller.Start(Now);
        _poller.Tick(Now.AddMinutes(10));
        _poller.FlyoutOpened(Now.AddMinutes(10));
        Assert.Single(_runs);
    }

    [Fact]
    public void An_answer_sets_the_state_keeps_the_reading_and_redraws()
    {
        var kept = new List<DateTimeOffset>();
        _poller.Start(Now);
        var poll = new ProviderPoll(
            LiveSchedule.Start(Now).AfterNoCredentials(Now), ProviderState.RateLimited, "fake Ok")
        {
            Keep = at => { kept.Add(at); _order.Add("keep"); },
        };
        _runs[0].Done(Task.FromResult(poll), Now.AddSeconds(3));

        Assert.Equal(ProviderState.RateLimited, _poller.State);
        Assert.Equal([(ProviderState.RateLimited, 1)], _atRedraw);
        Assert.Equal([Now.AddSeconds(3)], kept);
        Assert.Equal(1, _changed);
        Assert.Equal(["fake Ok"], _traced);
        Assert.Equal(["keep", "changed"], _order);
    }

    [Fact]
    public void Misses_follow_the_schedule()
    {
        _poller.Start(Now);
        _runs[0].Done(Task.FromResult(WithSchedule(LiveSchedule.Start(Now).AfterNoCredentials(Now))), Now);
        Assert.Equal(1, _poller.Misses);

        _poller.Tick(Now.AddMinutes(1));
        Assert.Equal(2, _runs.Count);
        _runs[1].Done(Task.FromResult(Ok(Now.AddMinutes(1))), Now.AddMinutes(1));
        Assert.Equal(0, _poller.Misses);
    }

    [Fact]
    public void Tick_waits_for_the_schedule()
    {
        _poller.Start(Now);
        _runs[0].Done(Task.FromResult(Ok(Now)), Now);
        _poller.Tick(Now.AddMinutes(4).AddSeconds(59));
        Assert.Single(_runs);
        _poller.Tick(Now.AddMinutes(5));
        Assert.Equal(2, _runs.Count);
    }

    [Fact]
    public void Opening_the_flyout_asks_early()
    {
        _poller.Start(Now);
        _runs[0].Done(Task.FromResult(WithSchedule(LiveSchedule.Start(Now).AfterNoCredentials(Now))), Now);
        _poller.Tick(Now.AddSeconds(10));
        Assert.Single(_runs);
        _poller.FlyoutOpened(Now.AddSeconds(10));
        Assert.Equal(2, _runs.Count);
    }

    [Fact]
    public void Opening_the_flyout_respects_a_wait()
    {
        var failed = LiveSchedule.Start(Now).AfterFetch(new LiveResult(LiveOutcome.Failed, null, null), Now);
        _poller.Start(Now);
        _runs[0].Done(Task.FromResult(WithSchedule(failed)), Now);
        _poller.FlyoutOpened(Now.AddSeconds(10));
        Assert.Single(_runs);
    }

    [Fact]
    public void A_poll_that_threw_counts_as_a_bad_answer()
    {
        _poller.Start(Now);
        _runs[0].Done(Task.FromException<ProviderPoll>(new InvalidOperationException("secret")), Now.AddSeconds(1));

        Assert.Equal(ProviderState.BadAnswer, _poller.State);
        Assert.Equal(1, _poller.Misses);
        Assert.Equal(1, _changed);
        Assert.Equal(["Fake poll threw InvalidOperationException"], _traced);
        _poller.Tick(Now.AddMinutes(1).AddSeconds(59));
        Assert.Single(_runs);
        _poller.Tick(Now.AddMinutes(2));
        Assert.Equal(2, _runs.Count);
    }

    [Fact]
    public void A_cancelled_task_without_stop_counts_as_a_bad_answer()
    {
        _poller.Start(Now);
        _runs[0].Done(Task.FromCanceled<ProviderPoll>(new CancellationToken(true)), Now.AddSeconds(1));
        Assert.Equal(ProviderState.BadAnswer, _poller.State);
    }

    [Fact]
    public async Task Stop_cancels_the_poll_and_turns_it_off()
    {
        _poller.Start(Now);
        await _runs[0].RunAsync();
        _poller.Stop();
        Assert.True(_provider.Tokens[0].IsCancellationRequested);
        Assert.Equal(ProviderState.Off, _poller.State);
        Assert.Equal(0, _poller.Misses);
    }

    [Fact]
    public void A_poll_stopped_while_it_ran_is_dropped()
    {
        var kept = 0;
        _poller.Start(Now);
        _poller.Stop();
        _runs[0].Done(Task.FromResult(Ok(Now, _ => kept++)), Now.AddSeconds(1));

        Assert.Equal(0, kept);
        Assert.Equal(0, _changed);
        Assert.Equal(ProviderState.Off, _poller.State);
        Assert.Single(_runs);
    }

    [Fact]
    public void Stop_then_start_while_a_poll_runs_waits_for_it()
    {
        var kept = 0;
        _poller.Start(Now);
        _poller.Stop();
        _poller.Start(Now.AddSeconds(1));
        Assert.Single(_runs);

        _runs[0].Done(Task.FromResult(Ok(Now, _ => kept++)), Now.AddSeconds(2));
        Assert.Equal(0, kept);
        Assert.Equal(2, _runs.Count);

        _runs[1].Done(Task.FromResult(Ok(Now.AddSeconds(2)) with { State = ProviderState.Expired }), Now.AddSeconds(3));
        Assert.Equal(ProviderState.Expired, _poller.State);
    }

    [Fact]
    public void A_run_handed_back_twice_does_nothing_the_second_time()
    {
        var kept = 0;
        _poller.Start(Now);
        _runs[0].Done(Task.FromResult(Ok(Now)), Now);
        _poller.Tick(Now.AddMinutes(5));
        Assert.Equal(2, _runs.Count);
        Assert.Equal(1, _changed);

        _runs[0].Done(Task.FromResult(Ok(Now) with { State = ProviderState.RateLimited, Keep = _ => kept++ }), Now.AddMinutes(5));

        Assert.Equal(ProviderState.Ok, _poller.State);
        Assert.Equal(0, kept);
        Assert.Equal(1, _changed);
        _poller.Tick(Now.AddHours(5));
        Assert.Equal(2, _runs.Count);

        _runs[1].Done(Task.FromResult(Ok(Now) with { State = ProviderState.Expired }), Now.AddHours(5));
        Assert.Equal(ProviderState.Expired, _poller.State);
        Assert.Equal(2, _changed);
    }

    [Fact]
    public async Task After_dispose_nothing_happens()
    {
        var kept = 0;
        _poller.Start(Now);
        await _runs[0].RunAsync();
        _poller.Dispose();
        Assert.True(_provider.Tokens[0].IsCancellationRequested);
        _runs[0].Done(Task.FromResult(Ok(Now, _ => kept++)), Now.AddSeconds(1));

        Assert.Equal(0, kept);
        Assert.Equal(0, _changed);
        _poller.Start(Now.AddMinutes(10));
        _poller.Tick(Now.AddMinutes(10));
        _poller.FlyoutOpened(Now.AddMinutes(10));
        Assert.Single(_runs);
        _poller.Dispose();
    }
}
