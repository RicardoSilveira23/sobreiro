using Sobreiro.Core.Limits;

namespace Sobreiro.Core.Display;

public sealed record PaceProjection(double ProjectedPercent, TimeSpan? RunsOutBeforeReset, DateTimeOffset? RunsOutAt);

public static class Pace
{
    /// <summary>Below this share of the window elapsed, a projection is too noisy to show.</summary>
    public const double MinElapsedShare = 0.05;

    public static PaceProjection? Project(Limit limit) => Project(limit, PaceDays.EveryDay, TimeZoneInfo.Utc);

    /// <summary>
    /// With <see cref="PaceDays.Weekdays"/>, a window longer than a day that holds some working
    /// time is measured in Monday-to-Friday time in <paramref name="zone"/>; any other window
    /// keeps calendar time.
    /// </summary>
    public static PaceProjection? Project(Limit limit, PaceDays days, TimeZoneInfo zone)
    {
        if (limit.IsReset || limit.ResetsAt is not { } resetsAt || limit.Window is not { } window || window <= TimeSpan.Zero)
            return null;
        if (limit.Percent >= 100 || limit.ObservedAt > resetsAt) return null;

        var start = resetsAt - window;
        var workingWindow = days == PaceDays.Weekdays && window > TimeSpan.FromDays(1)
            ? WorkingTime.Between(start, resetsAt, zone)
            : TimeSpan.Zero;
        var weekdays = workingWindow > TimeSpan.Zero;

        var elapsed = weekdays ? WorkingTime.Between(start, limit.ObservedAt, zone) : limit.ObservedAt - start;
        var total = weekdays ? workingWindow : window;
        var share = elapsed / total;
        if (share < MinElapsedShare || share > 1) return null;

        var projected = limit.Percent / share;
        TimeSpan? runsOut = null;
        DateTimeOffset? runsOutAt = null;
        if (projected > 100)
        {
            var needed = elapsed * ((100 - limit.Percent) / limit.Percent);
            var runOutAt = weekdays ? WorkingTime.After(limit.ObservedAt, needed, zone) : limit.ObservedAt + needed;
            runsOut = resetsAt - runOutAt;
            runsOutAt = runOutAt;
        }
        return new PaceProjection(projected, runsOut, runsOutAt);
    }
}
