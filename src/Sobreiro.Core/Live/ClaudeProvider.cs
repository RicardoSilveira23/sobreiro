using Sobreiro.Core.Limits;
using Sobreiro.Core.Polling;
using Sobreiro.Core.Usage;

namespace Sobreiro.Core.Live;

/// <summary>
/// Claude Code's usage as a provider: one Live usage poll, and the keeping of its reading in usage.json, where the
/// icon, flyout and alerts read it. Nothing it traces or logs holds a token or an exception's text.
/// </summary>
public sealed class ClaudeProvider : IProvider
{
    private static readonly TimeSpan StoreRetryDelay = TimeSpan.FromMilliseconds(50);

    private readonly Func<LiveSchedule, DateTimeOffset, CancellationToken, Task<LivePollResult>> _poll;
    private readonly string _usagePath;
    private readonly Action<string>? _trace;
    private UsageSnapshot? _last;

    public ClaudeProvider(LiveUsageClient client, string credentialsPath, string usagePath, Action<string>? trace = null)
        : this((s, at, ct) => LivePoll.RunAsync(credentialsPath, client, s, at, ct), usagePath, trace)
    {
    }

    internal ClaudeProvider(
        Func<LiveSchedule, DateTimeOffset, CancellationToken, Task<LivePollResult>> poll,
        string usagePath, Action<string>? trace = null)
    {
        _poll = poll;
        _usagePath = usagePath;
        _trace = trace;
    }

    public string Name => Providers.Claude;

    public async Task<ProviderPoll> PollAsync(LiveSchedule schedule, DateTimeOffset now, CancellationToken ct)
    {
        var r = await _poll(schedule, now, ct).ConfigureAwait(false);
        Action<DateTimeOffset>? keep = r.Limits is { } limits ? at => Store(limits, at) : null;
        return new ProviderPoll(r.Schedule, r.State, "Live usage " + r.Outcome, keep);
    }

    /// <summary>Lets UsageFile.Read's exceptions through, as the tray's caller catches them.</summary>
    public IReadOnlyList<Limit> Limits(DateTimeOffset now)
    {
        _last = UsageFile.Read(_usagePath) ?? _last;
        return LimitList.From(_last, now);
    }

    // The bridge's merge, so the icon, flyout and alerts read one state, but authoritative: within a period this
    // reading replaces the stored one whatever its percentage. A usage.json the bridge holds is tried once more;
    // still held, this reading is skipped rather than written over what the file keeps (spend).
    private void Store(RateLimits limits, DateTimeOffset now)
    {
        try
        {
            if (!UsageFile.TryReadForMerge(_usagePath, out var stored))
            {
                Thread.Sleep(StoreRetryDelay);
                if (!UsageFile.TryReadForMerge(_usagePath, out stored))
                {
                    _trace?.Invoke("usage.json was busy, so this Live usage reading was not saved");
                    return;
                }
            }
            UsageFile.Write(_usagePath, UsageMerge.Merge(stored, limits, now, authoritative: true));
        }
        catch (Exception ex)
        {
            _trace?.Invoke("saving the Live usage reading failed: " + ex.GetType().Name);
        }
    }
}
