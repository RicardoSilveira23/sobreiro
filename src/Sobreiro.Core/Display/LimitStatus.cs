using System.Globalization;
using Sobreiro.Core.Limits;

namespace Sobreiro.Core.Display;

public enum StatusKind { OnTrack, Ends, UsedUp, NotUsedYet, TooEarly, Resetting }

public sealed record LimitStatus(StatusKind Kind, string Text)
{
    /// <summary>Under this far from the reset, a limit that is not yet reset reads "Resetting".</summary>
    public static readonly TimeSpan ResettingWithin = TimeSpan.FromSeconds(30);

    /// <summary>Below this percentage the limit reads "Not used yet".</summary>
    public const double NotUsedBelow = 2;

    /// <summary>The badge for a limit, or null when it has no reset time or window length.</summary>
    public static LimitStatus? For(Limit limit, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture)
    {
        if (limit.ResetsAt is not { } resetsAt || limit.Window is not { } window || window <= TimeSpan.Zero)
            return null;
        if (!limit.IsReset && resetsAt - now < ResettingWithin) return new(StatusKind.Resetting, "Resetting");
        if (limit.IsReset || limit.Percent < NotUsedBelow) return new(StatusKind.NotUsedYet, "Not used yet");
        if (limit.Percent >= 100) return new(StatusKind.UsedUp, "Used up");

        var pace = Pace.Project(limit);
        if (pace is null) return new(StatusKind.TooEarly, "Too early");
        if (pace.RunsOutAt is { } at)
            return at <= now
                ? new(StatusKind.Ends, "Likely used up")
                : new(StatusKind.Ends, "Ends " + When.Format(at, now, zone, culture));
        return new(StatusKind.OnTrack, "On track");
    }
}
