using System.Diagnostics;
using System.Net.Http;
using System.Reflection;
using System.Windows.Threading;
using Sobreiro.Copilot;
using Sobreiro.Core.Copilot;
using Sobreiro.Core.Live;
using Sobreiro.Core.Polling;

namespace Sobreiro;

/// <summary>
/// The Copilot poller: timing, threading and the redraw. What one poll does is <see cref="CopilotPoll"/>. One poll at
/// a time; the poll and the cache read run off the UI thread, and the result is handled on it. Tokens are read on
/// every poll and never kept.
/// </summary>
internal sealed class CopilotUsage : IDisposable
{
    private readonly Dispatcher _dispatcher = Dispatcher.CurrentDispatcher;
    private readonly HttpClient _http = new(LiveUsageClient.CreateHandler());
    private readonly CopilotUsageClient _client;
    private readonly Action _changed;
    private readonly Action<string> _log;
    private readonly string _cachePath = CopilotCache.DefaultPath(Environment.GetEnvironmentVariable);
    private readonly IReadOnlyList<CopilotSignIn> _signIns =
    [
        new(CopilotSourceKind.Env, () => new TokenSearch(CopilotToken.FromEnvironment(Environment.GetEnvironmentVariable))),
        new(CopilotSourceKind.CredentialManager, CredentialStore.CopilotTokens),
        new(CopilotSourceKind.Gh, () => new TokenSearch(GhCli.Token())),
    ];
    private LiveSchedule? _schedule; // null until Start
    private CancellationTokenSource? _poll;
    private CopilotReading? _live;
    private CopilotReading? _cache;
    private string? _lastLogged;
    private bool _disposed;

    public CopilotUsage(Action changed, Action<string> log)
    {
        _client = new CopilotUsageClient(_http, UserAgent());
        _changed = changed;
        _log = log;
    }

    /// <summary>Ok until the first poll answers.</summary>
    public ProviderState State { get; private set; } = ProviderState.Ok;

    public int Misses => _schedule?.Misses ?? 0;

    public CopilotReading? Shown => CopilotShown.Pick(_live, _cache);

    public void Start(DateTimeOffset now)
    {
        if (_disposed || _schedule is not null) return;
        _schedule = LiveSchedule.Start(now);
        // Off the UI thread; the first poll reads the cache again, so this only shows it sooner.
        Task.Run(() => CopilotCache.Read(_cachePath))
            .ContinueWith(task =>
            {
                _ = task.Exception; // observed: a fault here only means no early cache
                if (task.IsCompletedSuccessfully && task.Result is { } cache)
                    _dispatcher.BeginInvoke(() =>
                    {
                        if (_disposed) return;
                        _cache ??= cache;
                        _changed();
                    });
            }, TaskScheduler.Default);
        Tick(now);
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
        Task.Run(async () =>
            {
                var result = await CopilotPoll.RunAsync(_signIns, _client, schedule, now, cts.Token).ConfigureAwait(false);
                return (Result: result, Cache: CopilotCache.Read(_cachePath));
            })
            .ContinueWith(task =>
            {
                _ = task.Exception; // observed here, so a poll that ends after shutdown raises nothing
                _dispatcher.BeginInvoke(() => Done(task, cts, schedule, now));
            }, TaskScheduler.Default);
    }

    private void Done(
        Task<(CopilotPollResult Result, CopilotReading? Cache)> task, CancellationTokenSource cts,
        LiveSchedule schedule, DateTimeOffset now)
    {
        var dropped = cts.IsCancellationRequested;
        cts.Dispose();
        _poll = null;
        if (_disposed) return;
        if (dropped)
        {
            Tick(DateTimeOffset.Now);
            return;
        }

        CopilotPollResult result;
        CopilotReading? cache = null;
        if (task.IsCompletedSuccessfully)
        {
            (result, cache) = task.Result;
        }
        else
        {
            // CopilotPoll throws only for cancellation; anything else counts as a failure. Its text stays out of the log.
            if (task.Exception?.GetBaseException() is { } fault) Debug.WriteLine($"Sobreiro: Copilot poll failed: {fault.GetType().Name}");
            result = new CopilotPollResult(
                schedule.AfterFetch(new LiveResult(LiveOutcome.Failed, null, null), now), ProviderState.BadAnswer, null,
                "copilot BadAnswer " + nameof(LiveOutcome.Failed));
        }

        _schedule = result.Schedule;
        State = result.State;
        if (result.Reading is { } reading) _live = reading;
        _cache = cache ?? _cache; // a read that found nothing keeps the previous one

        var line = result.Log + " cache=" + (Shown?.Source == CopilotSource.Cache ? "used" : _cache is null ? "none" : "older");
        if (line != _lastLogged)
        {
            _lastLogged = line;
            _log(line); // result.Log already starts with "copilot"
        }
        _changed();
    }

    // Sobreiro/0.3.0, built as the Live poller builds it.
    private static string UserAgent()
    {
        var assembly = typeof(CopilotUsage).Assembly;
        var version = assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion.Split('+')[0];
        if (string.IsNullOrEmpty(version)) version = assembly.GetName().Version?.ToString(3) ?? "0";
        return "Sobreiro/" + version;
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        _schedule = null;
        _poll?.Cancel();
        _http.Dispose();
    }
}
