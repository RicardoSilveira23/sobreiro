using Sobreiro.Core.Limits;
using Sobreiro.Core.Live;
using Sobreiro.Core.Polling;

namespace Sobreiro.Core.Copilot;

/// <summary>
/// GitHub Copilot's credits as a provider: one poll of GitHub plus a read of the Copilot CLI's cache file, and the
/// keeping of both readings in memory. Nothing it logs holds a token or an exception's text.
/// </summary>
public sealed class CopilotProvider : IProvider
{
    private readonly Func<LiveSchedule, DateTimeOffset, CancellationToken, Task<CopilotPollResult>> _poll;
    private readonly Func<CopilotReading?> _readCache;
    private readonly Action<string> _log;
    private CopilotReading? _live;
    private CopilotReading? _cache;
    private string? _lastLogged;

    public CopilotProvider(
        CopilotUsageClient client, IReadOnlyList<CopilotSignIn> signIns, string cachePath, Action<string> log)
        : this((s, at, ct) => CopilotPoll.RunAsync(signIns, client, s, at, ct), () => CopilotCache.Read(cachePath), log)
    {
    }

    internal CopilotProvider(
        Func<LiveSchedule, DateTimeOffset, CancellationToken, Task<CopilotPollResult>> poll,
        Func<CopilotReading?> readCache, Action<string> log)
    {
        _poll = poll;
        _readCache = readCache;
        _log = log;
    }

    public string Name => Providers.Copilot;

    /// <summary>The later of the live and the cached reading. Read on the caller's thread.</summary>
    public CopilotReading? Shown => CopilotShown.Pick(_live, _cache);

    /// <summary>The cache file, safe off the caller's thread.</summary>
    public CopilotReading? ReadCache() => _readCache();

    /// <summary>Shows a cache read ahead of the first poll, only while no cached reading is held.</summary>
    public void TakeEarly(CopilotReading cache) => _cache ??= cache;

    public async Task<ProviderPoll> PollAsync(LiveSchedule schedule, DateTimeOffset now, CancellationToken ct)
    {
        CopilotPollResult result;
        CopilotReading? cache = null;
        try
        {
            result = await _poll(schedule, now, ct).ConfigureAwait(false);
            cache = _readCache();
        }
        catch (Exception) when (!ct.IsCancellationRequested)
        {
            // Anything but cancellation counts as a bad answer; the exception's text could hold a token.
            result = new CopilotPollResult(
                schedule.AfterFetch(new LiveResult(LiveOutcome.Failed, null, null), now), ProviderState.BadAnswer, null,
                "copilot BadAnswer " + nameof(LiveOutcome.Failed));
            cache = null;
        }

        return new ProviderPoll(result.Schedule, result.State, result.Log, _ => Keep(result, cache));
    }

    public IReadOnlyList<Limit> Limits(DateTimeOffset now) => LimitList.FromCopilot(Shown, now);

    private void Keep(CopilotPollResult result, CopilotReading? cache)
    {
        if (result.Reading is { } reading) _live = reading;
        _cache = cache ?? _cache; // a read that found nothing keeps the previous one

        var line = result.Log + " cache=" + (Shown?.Source == CopilotSource.Cache ? "used" : _cache is null ? "none" : "older");
        if (line == _lastLogged) return;
        _lastLogged = line;
        _log(line);
    }
}
