namespace Sobreiro.Core.Diagnostics;

/// <summary>
/// Caps how many times one kind of event is logged in a run, so an error that repeats
/// on every frame can't churn the disk. Safe to call from any thread.
/// </summary>
public sealed class LogBudget
{
    private readonly int _limit;
    private int _used;

    public LogBudget(int limit) => _limit = limit;

    /// <summary>The event to log, or null once the limit is spent. The last one allowed says later ones are dropped.</summary>
    public string? Next(string @event)
    {
        var n = Interlocked.Increment(ref _used);
        if (n < _limit) return @event;
        if (n == _limit) return @event + "; later ones not logged";
        return null;
    }
}
