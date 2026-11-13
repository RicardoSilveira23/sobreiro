using Sobreiro.Core.Live;

namespace Sobreiro.Core.Polling;

/// <summary>A poll the host runs off its thread (RunAsync), then hands back on it (Done).</summary>
public sealed class PollRun
{
    private readonly ProviderPoller _poller;
    private readonly IProvider _provider;
    private readonly LiveSchedule _schedule;
    private readonly DateTimeOffset _started;
    private readonly CancellationTokenSource _cts;

    internal PollRun(
        ProviderPoller poller, IProvider provider, LiveSchedule schedule, DateTimeOffset started, CancellationTokenSource cts)
    {
        _poller = poller;
        _provider = provider;
        _schedule = schedule;
        _started = started;
        _cts = cts;
    }

    /// <summary>The poll itself, with the schedule and time it started with. Safe off the caller's thread.</summary>
    public Task<ProviderPoll> RunAsync() => _provider.PollAsync(_schedule, _started, _cts.Token);

    /// <summary>Hands the finished task back, on the caller's thread, with the time it is handled.</summary>
    public void Done(Task<ProviderPoll> task, DateTimeOffset now) => _poller.Complete(this, task, now);

    internal LiveSchedule Schedule => _schedule;
    internal DateTimeOffset Started => _started;
    internal CancellationTokenSource Cts => _cts;
}

/// <summary>
/// The timing of one provider's polls: when one is due, one at a time, and what an answer does. It starts no
/// thread: the host runs each <see cref="PollRun"/> and hands it back. Not thread-safe: every member but
/// <see cref="PollRun.RunAsync"/> is called on one thread.
/// </summary>
public sealed class ProviderPoller : IDisposable
{
    private readonly Action<PollRun> _run;
    private readonly Action _changed;
    private readonly Action<string>? _trace;
    private LiveSchedule? _schedule; // null while off
    private PollRun? _poll; // set while a poll runs, until its answer is handled
    private bool _disposed;

    public ProviderPoller(IProvider provider, Action<PollRun> run, Action changed, Action<string>? trace = null)
    {
        Provider = provider;
        _run = run;
        _changed = changed;
        _trace = trace;
    }

    public IProvider Provider { get; }

    /// <summary>Ok until the first poll answers.</summary>
    public ProviderState State { get; private set; } = ProviderState.Ok;

    /// <summary>The schedule's; 0 while off.</summary>
    public int Misses => _schedule?.Misses ?? 0;

    public void Start(DateTimeOffset now)
    {
        if (_disposed || _schedule is not null) return;
        _schedule = LiveSchedule.Start(now);
        Tick(now);
    }

    /// <summary>Drops a running poll's answer; the poll itself is cancelled.</summary>
    public void Stop()
    {
        _schedule = null;
        State = ProviderState.Off;
        _poll?.Cts.Cancel();
    }

    public void Tick(DateTimeOffset now)
    {
        if (_schedule is { } schedule && schedule.IsDue(now)) Poll(schedule, now);
    }

    public void FlyoutOpened(DateTimeOffset now)
    {
        if (_schedule is { } schedule && (schedule.IsDue(now) || schedule.FetchOnOpen(now))) Poll(schedule, now);
    }

    private void Poll(LiveSchedule schedule, DateTimeOffset now)
    {
        if (_disposed || _poll is not null) return;
        _poll = new PollRun(this, Provider, schedule, now, new CancellationTokenSource());
        _run(_poll);
    }

    internal void Complete(PollRun run, Task<ProviderPoll> task, DateTimeOffset now)
    {
        var dropped = run.Cts.IsCancellationRequested;
        run.Cts.Dispose();
        _poll = null;
        if (_disposed) return;
        if (dropped || _schedule is null)
        {
            // Stopped while it ran: the answer is dropped.
            Tick(now);
            return;
        }

        ProviderPoll poll;
        if (task.IsCompletedSuccessfully)
        {
            poll = task.Result;
        }
        else
        {
            // A provider throws only for cancellation; anything else counts as a failure. Its text stays out of the log.
            var type = task.Exception?.GetBaseException().GetType().Name ?? "nothing";
            poll = new ProviderPoll(
                run.Schedule.AfterFetch(new LiveResult(LiveOutcome.Failed, null, null), run.Started),
                ProviderState.BadAnswer,
                $"{Provider.Name} poll threw {type}");
        }

        _schedule = poll.Schedule;
        State = poll.State;
        poll.Keep?.Invoke(now);
        _trace?.Invoke(poll.Log);
        _changed();
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        Stop();
    }
}
