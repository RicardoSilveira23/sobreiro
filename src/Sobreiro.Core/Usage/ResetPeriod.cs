namespace Sobreiro.Core.Usage;

/// <summary>Decides whether two reset times belong to the same limit period.</summary>
public static class ResetPeriod
{
    public static readonly TimeSpan Tolerance = TimeSpan.FromHours(1);

    public static bool Same(DateTimeOffset? a, DateTimeOffset? b)
    {
        if (a is null && b is null) return true;
        if (a is null || b is null) return false;
        return (a.Value - b.Value).Duration() <= Tolerance;
    }
}
