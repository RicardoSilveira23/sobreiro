using Sobreiro.Core.Limits;

namespace Sobreiro.Core.Display;

public sealed record PaceProjection(double ProjectedPercent, TimeSpan? RunsOutBeforeReset, DateTimeOffset? RunsOutAt);

public static class Pace
{
    /// <summary>Below this share of the window elapsed, a projection is too noisy to show.</summary>
    public const double MinElapsedShare = 0.05;

    public static PaceProjection? Project(Limit limit)
    {
        if (limit.IsReset || limit.ResetsAt is not { } resetsAt || limit.Window is not { } window || window <= TimeSpan.Zero)
            return null;
        if (limit.Percent >= 100) return null;

        var elapsed = window - (resetsAt - limit.ObservedAt);
        var share = elapsed / window;
        if (share < MinElapsedShare || share > 1) return null;

        var projected = limit.Percent / share;
        TimeSpan? runsOut = null;
        DateTimeOffset? runsOutAt = null;
        if (projected > 100)
        {
            var runOutAt = limit.ObservedAt + elapsed * ((100 - limit.Percent) / limit.Percent);
            runsOut = resetsAt - runOutAt;
            runsOutAt = runOutAt;
        }
        return new PaceProjection(projected, runsOut, runsOutAt);
    }
}
