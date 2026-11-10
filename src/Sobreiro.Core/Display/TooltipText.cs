using System.Globalization;
using Sobreiro.Core.Limits;

namespace Sobreiro.Core.Display;

public static class TooltipText
{
    /// <summary>The longest text a Windows tray tooltip can show.</summary>
    public const int MaxLength = 127;

    /// <summary>"AI credits 17% · resets Dec 1 ...", the credits figures and the age, cut to <see cref="MaxLength"/>.</summary>
    public static string RenderCopilot(Limit credits, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture)
    {
        var lines = new List<string> { Line(credits, now, zone, culture) };
        if (!credits.IsReset && CreditsFigures(credits) is { } figures) lines.Add(figures);
        lines.Add("as of " + When.Format(credits.ObservedAt, now, zone, culture));
        var text = string.Join("\n", lines);
        return text.Length <= MaxLength ? text : text[..MaxLength];
    }

    /// <summary>"2,644 of 15,000 credits", or null without both numbers.</summary>
    internal static string? CreditsFigures(Limit limit)
    {
        if (limit.UsedCredits is not { } used || limit.CreditBudget is not { } budget) return null;
        return $"{Math.Round(used).ToString("#,0", CultureInfo.InvariantCulture)} of {Math.Round(budget).ToString("#,0", CultureInfo.InvariantCulture)} credits";
    }

    public static string Render(IReadOnlyList<Limit> limits, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture, bool legacyConnection = false)
    {
        limits = limits.Where(l => l.Kind != LimitKind.Credits).ToList();
        if (limits.Count == 0)
            return legacyConnection
                ? "Sobreiro · no data yet\nStill on the old bridge: reconnect from the tray menu."
                : "Sobreiro · no data yet";

        var lines = new List<string>();
        foreach (var kind in new[] { LimitKind.Session, LimitKind.Weekly, LimitKind.Spend })
        {
            var limit = limits.FirstOrDefault(l => l.Kind == kind);
            if (limit is null)
            {
                // Session and weekly are always listed; spend only when present.
                if (kind != LimitKind.Spend) lines.Add($"{(kind == LimitKind.Session ? "Session" : "Weekly")} –");
                continue;
            }
            lines.Add(Line(limit, now, zone, culture));
        }
        lines.Add("as of " + When.Format(limits.Max(l => l.ObservedAt), now, zone, culture));

        var text = string.Join("\n", lines);
        return text.Length <= MaxLength ? text : text[..MaxLength];
    }

    private static string Line(Limit limit, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture)
    {
        var line = $"{limit.Label} {limit.DisplayPercent.ToString(CultureInfo.InvariantCulture)}%";
        if (limit.ResetsAt is { } at)
            line += $" · {(limit.IsReset ? "reset" : "resets")} {When.Format(at, now, zone, culture)}";
        return line;
    }
}
