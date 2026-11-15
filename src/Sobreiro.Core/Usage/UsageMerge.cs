namespace Sobreiro.Core.Usage;

/// <summary>
/// Merges a reading into the stored snapshot per window, so an idle session cannot overwrite fresher numbers from
/// another one.
/// </summary>
public static class UsageMerge
{
    /// <summary>
    /// <paramref name="authoritative"/> is for Live usage's readings, which come from Anthropic itself: within a
    /// period they replace the stored session and weekly windows whatever their percentage. Status line readings
    /// keep the higher one. Spend follows its own rule either way.
    /// </summary>
    public static UsageSnapshot Merge(UsageSnapshot? stored, RateLimits incoming, DateTimeOffset now, bool authoritative = false)
    {
        var old = stored?.Limits;
        var oldAt = stored?.ObservedAt ?? now;
        Func<LimitWindow, LimitWindow, bool> windowFresher = authoritative ? (_, _) => true : WindowFresher;

        var limits = new RateLimits(
            Pick(Resolve(old?.FiveHour, oldAt), incoming.FiveHour, now, windowFresher, w => w.ResetsAt, (w, at) => w with { ObservedAt = at }),
            Pick(Resolve(old?.SevenDay, oldAt), incoming.SevenDay, now, windowFresher, w => w.ResetsAt, (w, at) => w with { ObservedAt = at }),
            Pick(Resolve(old?.Spend, oldAt), incoming.Spend, now, SpendFresher, s => s.ResetsAt, (s, at) => s with { ObservedAt = at }));
        return new UsageSnapshot(limits, now);
    }

    // Within a period the used percentage only rises, so of two status line readings the higher is the fresher.
    private static bool WindowFresher(LimitWindow stored, LimitWindow incoming) => incoming.UsedPercentage >= stored.UsedPercentage;

    // Not so for spend: raising the cap lowers the percentage, so a new cap is always taken, and with the same
    // cap the dollars decide.
    private static bool SpendFresher(SpendLimit stored, SpendLimit incoming)
    {
        if (incoming.LimitUsd != stored.LimitUsd) return true;
        return incoming.UsedUsd is { } used && stored.UsedUsd is { } before
            ? used >= before
            : incoming.UsedPercentage >= stored.UsedPercentage;
    }

    private static LimitWindow? Resolve(LimitWindow? w, DateTimeOffset snapshotAt) => w is null ? null : w with { ObservedAt = w.ObservedAt ?? snapshotAt };

    private static SpendLimit? Resolve(SpendLimit? s, DateTimeOffset snapshotAt) => s is null ? null : s with { ObservedAt = s.ObservedAt ?? snapshotAt };

    /// <summary>Stored must already have its ObservedAt resolved. The result always carries an ObservedAt.</summary>
    private static T? Pick<T>(T? stored, T? incoming, DateTimeOffset now,
        Func<T, T, bool> fresher, Func<T, DateTimeOffset?> resetsAt, Func<T, DateTimeOffset, T> stamp)
        where T : class
    {
        if (incoming is null)
        {
            // A window the reading does not carry stays until its own reset passes.
            return stored is not null && !(resetsAt(stored) is { } expiry && expiry <= now) ? stored : null;
        }
        if (stored is null) return stamp(incoming, now);

        var oldReset = resetsAt(stored);
        var newReset = resetsAt(incoming);
        var takeIncoming = oldReset is null || newReset is null
            || newReset.Value - oldReset.Value > ResetPeriod.Tolerance
            || (ResetPeriod.Same(oldReset, newReset) && fresher(stored, incoming));
        return takeIncoming ? stamp(incoming, now) : stored;
    }
}
