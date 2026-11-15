using Sobreiro.Core.Limits;
using Sobreiro.Core.Polling;

namespace Sobreiro.Core.Display;

/// <summary>
/// Whether a pill block's numbers have stopped: it then keeps its last figures, each marked, and
/// its ring or bar turns grey. The flyout names the cause.
/// </summary>
public static class PillStale
{
    /// <summary>Polls in a row that must bring nothing before a block is marked: one miss after a wake or a start is no reason.</summary>
    public const int MissesNeeded = 2;

    public static readonly TimeSpan ClaudeAfter = TimeSpan.FromMinutes(15);
    public static readonly TimeSpan CopilotAfter = TimeSpan.FromHours(24);

    /// <summary>True when the poll has stopped and the newest session or weekly figure is old. With no such figure there is nothing to mark.</summary>
    public static bool Claude(ProviderState state, IReadOnlyList<Limit> limits, DateTimeOffset now, int misses)
    {
        if (misses < MissesNeeded || !Stopped(state)) return false;

        DateTimeOffset? newest = null;
        foreach (var l in limits)
            if (l.Kind is LimitKind.Session or LimitKind.Weekly && (newest is null || l.ObservedAt > newest))
                newest = l.ObservedAt;
        return newest is { } at && now - at > ClaudeAfter;
    }

    public static bool Copilot(ProviderState state, Limit? credits, DateTimeOffset now, int misses) =>
        misses >= MissesNeeded && Stopped(state) && credits is not null && now - credits.ObservedAt > CopilotAfter;

    private static bool Stopped(ProviderState state) => state
        is ProviderState.Expired or ProviderState.NoSignIn or ProviderState.Refused
        or ProviderState.Offline or ProviderState.BadAnswer or ProviderState.RateLimited;
}
