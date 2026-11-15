using Sobreiro.Core.Limits;
using Sobreiro.Core.Live;

namespace Sobreiro.Core.Polling;

/// <summary>
/// One poll's answer. Log names outcomes only, never a token. Keep takes the reading in, on the caller's thread,
/// with the time it is called; a dropped poll's Keep is never called.
/// </summary>
public sealed record ProviderPoll(
    LiveSchedule Schedule, ProviderState State, string Log, Action<DateTimeOffset>? Keep = null);

/// <summary>A usage provider the tray polls: what one poll does, and the limits its kept polls bring.</summary>
public interface IProvider
{
    /// <summary>The name its limits carry.</summary>
    string Name { get; }

    /// <summary>One poll, safe to run off the caller's thread. Throws only an OperationCanceledException for ct.</summary>
    Task<ProviderPoll> PollAsync(LiveSchedule schedule, DateTimeOffset now, CancellationToken ct);

    /// <summary>The limits to show now, from what the kept polls brought. Called on the caller's thread.</summary>
    IReadOnlyList<Limit> Limits(DateTimeOffset now);
}
