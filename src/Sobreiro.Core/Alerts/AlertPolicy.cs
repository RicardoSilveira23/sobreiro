using System.Globalization;
using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Usage;

namespace Sobreiro.Core.Alerts;

public sealed record Alert(LimitKind Kind, int Threshold, string Title, string Body);

/// <summary>Toast policy: one alert per window per reset period for the highest threshold crossed.</summary>
public static class AlertPolicy
{
    public static readonly IReadOnlyList<int> Thresholds = [80, 95];

    /// <summary>
    /// Returns the alerts to show and the state to keep. <c>Next</c> is the incoming
    /// <paramref name="state"/> itself when nothing changed, so callers can skip saving.
    /// </summary>
    public static (IReadOnlyList<Alert> Alerts, AlertState Next) Check(
        IReadOnlyList<Limit> limits, AlertState state, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture, PaceDays days = PaceDays.EveryDay)
    {
        var alerts = new List<Alert>();
        var marks = new Dictionary<LimitKind, AlertMark>(state.Marks);

        foreach (var limit in limits)
        {
            if (limit.IsReset) continue;
            var crossed = Thresholds.Where(t => limit.DisplayPercent >= t).DefaultIfEmpty(0).Max();
            if (crossed == 0) continue;

            if (state.Marks.TryGetValue(limit.Kind, out var mark)
                && ResetPeriod.Same(mark.ResetsAt, limit.ResetsAt)
                && mark.Threshold >= crossed)
                continue;

            alerts.Add(new Alert(limit.Kind, crossed,
                limit.Kind == LimitKind.Credits
                    ? $"Copilot {limit.Label} {limit.DisplayPercent}% used"
                    : $"{limit.Label} {limit.DisplayPercent}% used",
                string.Join(" · ", new[]
                {
                    LimitStatus.For(limit, now, zone, culture, days)?.Text,
                    FlyoutModel.ResetsIn(limit, now, zone, culture),
                }.Where(t => t is not null))));
            marks[limit.Kind] = new AlertMark(limit.ResetsAt, crossed);
        }

        if (alerts.Count == 0) return (alerts, state);
        var ordered = alerts.OrderByDescending(a => a.Threshold).ThenBy(a => a.Kind).ToList();
        return (ordered, new AlertState(marks));
    }
}
