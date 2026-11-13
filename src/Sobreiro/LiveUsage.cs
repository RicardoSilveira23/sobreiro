using System.Diagnostics;
using System.Net.Http;
using System.Reflection;
using System.Windows.Threading;
using Sobreiro.Core.Connect;
using Sobreiro.Core.Live;
using Sobreiro.Core.Usage;
using Sobreiro.Core.Polling;

namespace Sobreiro;

/// <summary>
/// The Live usage poller: timing, threading, the merge into usage.json and the redraw. What one poll does, and
/// when it may send anything, is <see cref="LivePoll"/>. One poll at a time; all but the poll runs on the UI thread.
/// </summary>
internal sealed class LiveUsage : IDisposable
{
    private static readonly TimeSpan StoreRetryDelay = TimeSpan.FromMilliseconds(50);

    private readonly Dispatcher _dispatcher = Dispatcher.CurrentDispatcher;
    private readonly HttpClient _http = new(LiveUsageClient.CreateHandler());
    private readonly LiveUsageClient _client;
    private readonly string _usagePath;
    private readonly Action _changed;
    private LiveSchedule? _schedule; // null while off
    private CancellationTokenSource? _poll; // set while a poll runs, until its result is handled
    private bool _disposed;

    public LiveUsage(string usagePath, Action changed)
    {
        _client = new LiveUsageClient(_http, UserAgent());
        _usagePath = usagePath;
        _changed = changed;
    }

    private string CredentialsPath { get; } = ClaudePaths.CredentialsFile(Environment.GetEnvironmentVariable);

    /// <summary>Ok until the first poll answers.</summary>
    public ProviderState State { get; private set; } = ProviderState.Ok;

    public int Misses => _schedule?.Misses ?? 0;

    private bool IsOn => _schedule is not null;

    public void Start(DateTimeOffset now)
    {
        if (_disposed || IsOn) return;
        _schedule = LiveSchedule.Start(now);
        Tick(now);
    }

    /// <summary>Drops a running poll's result; the request itself is cancelled.</summary>
    public void Stop()
    {
        _schedule = null;
        State = ProviderState.Off;
        _poll?.Cancel();
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
        var cts = new CancellationTokenSource();
        _poll = cts;
        var path = CredentialsPath;
        Task.Run(() => LivePoll.RunAsync(path, _client, schedule, now, cts.Token))
            .ContinueWith(task =>
            {
                _ = task.Exception; // observed here, so a poll that ends after shutdown raises nothing
                _dispatcher.BeginInvoke(() => Done(task, cts, schedule, now));
            }, TaskScheduler.Default);
    }

    private void Done(Task<LivePollResult> task, CancellationTokenSource cts, LiveSchedule schedule, DateTimeOffset now)
    {
        var dropped = cts.IsCancellationRequested;
        cts.Dispose();
        _poll = null;
        if (_disposed) return;
        if (dropped || _schedule is null)
        {
            // Stopped while it ran: the result is dropped.
            Tick(DateTimeOffset.Now);
            return;
        }

        LivePollResult result;
        if (task.IsCompletedSuccessfully)
        {
            result = task.Result;
        }
        else
        {
            // LivePoll throws only for cancellation; anything else counts as a failure. Its text stays out of the log.
            Debug.WriteLine($"Sobreiro: Live usage poll threw {task.Exception?.InnerException?.GetType().Name}");
            result = new LivePollResult(
                schedule.AfterFetch(new LiveResult(LiveOutcome.Failed, null, null), now), ProviderState.BadAnswer, null, nameof(LiveOutcome.Failed));
        }

        _schedule = result.Schedule;
        State = result.State;
        Debug.WriteLine($"Sobreiro: Live usage {result.Outcome}");
        if (result.Limits is { } limits) Store(limits);
        _changed();
    }

    // The bridge's merge, so the icon, flyout and alerts read one state, but authoritative: within a period this
    // reading replaces the stored one whatever its percentage. A usage.json the bridge holds is tried once more;
    // still held, this reading is skipped rather than written over what the file keeps (spend).
    private void Store(RateLimits limits)
    {
        try
        {
            if (!UsageFile.TryReadForMerge(_usagePath, out var stored))
            {
                Thread.Sleep(StoreRetryDelay);
                if (!UsageFile.TryReadForMerge(_usagePath, out stored))
                {
                    Debug.WriteLine("Sobreiro: usage.json was busy, so this Live usage reading was not saved");
                    return;
                }
            }
            UsageFile.Write(_usagePath, UsageMerge.Merge(stored, limits, DateTimeOffset.Now, authoritative: true));
        }
        catch (Exception ex)
        {
            Debug.WriteLine($"Sobreiro: saving the Live usage reading failed: {ex.GetType().Name}");
        }
    }

    // Sobreiro/0.3.0: the informational version without its "+commit" part, else the assembly version.
    private static string UserAgent()
    {
        var assembly = typeof(LiveUsage).Assembly;
        var version = assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion.Split('+')[0];
        if (string.IsNullOrEmpty(version)) version = assembly.GetName().Version?.ToString(3) ?? "0";
        return "Sobreiro/" + version;
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        Stop();
        _http.Dispose();
    }
}
