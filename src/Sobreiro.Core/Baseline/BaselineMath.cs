namespace Sobreiro.Core.Baseline;

public static class BaselineMath
{
    /// <summary>The processor time used over <paramref name="window"/>, scaled to one hour, in seconds.</summary>
    public static double CpuSecondsPerHour(TimeSpan cpuBefore, TimeSpan cpuAfter, TimeSpan window)
    {
        if (window <= TimeSpan.Zero) throw new ArgumentOutOfRangeException(nameof(window));
        return (cpuAfter - cpuBefore).TotalSeconds * 3600 / window.TotalSeconds;
    }

    /// <summary>The middle value, or the mean of the middle two; null for an empty list.</summary>
    public static TimeSpan? Median(IReadOnlyList<TimeSpan> values)
    {
        if (values.Count == 0) return null;
        var sorted = values.Order().ToList();
        var mid = sorted.Count / 2;
        return sorted.Count % 2 == 1 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
    }
}
