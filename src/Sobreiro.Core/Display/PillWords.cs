using Sobreiro.Core.Copilot;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Live;
using Sobreiro.Core.Polling;

namespace Sobreiro.Core.Display;

/// <summary>The two words a pill block shows in place of its figures once the numbers have stopped.</summary>
public sealed record PillWords(string Top, string Bottom)
{
    /// <summary>Polls in a row that must bring nothing before words show: one miss after a wake or a start is no reason.</summary>
    public const int MissesNeeded = 2;

    public static readonly TimeSpan ClaudeAfter = TimeSpan.FromMinutes(15);
    public static readonly TimeSpan CopilotAfter = TimeSpan.FromHours(24);

    public static PillWords? ForClaude(ProviderState state, IReadOnlyList<Limit> limits, DateTimeOffset now, int misses)
    {
        if (misses < MissesNeeded) return null;
        var words = Of(state);
        if (words is null) return null;

        DateTimeOffset? newest = null;
        foreach (var l in limits)
            if (l.Kind is LimitKind.Session or LimitKind.Weekly && (newest is null || l.ObservedAt > newest))
                newest = l.ObservedAt;
        return newest is null || now - newest > ClaudeAfter ? words : null;
    }

    public static PillWords? ForCopilot(ProviderState state, Limit? credits, DateTimeOffset now, int misses)
    {
        if (misses < MissesNeeded || credits is null || now - credits.ObservedAt <= CopilotAfter) return null;
        return Of(state);
    }

    private static PillWords? Of(ProviderState state) => state switch
    {
        ProviderState.Expired => new PillWords("sign-in", "expired"),
        ProviderState.NoSignIn => new PillWords("no", "sign-in"),
        ProviderState.Refused => new PillWords("sign-in", "refused"),
        ProviderState.Offline => new PillWords("no", "network"),
        ProviderState.BadAnswer => new PillWords("no", "answer"),
        ProviderState.RateLimited => new PillWords("asked", "to wait"),
        _ => null,
    };
}
