using System.Globalization;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Live;

namespace Sobreiro.Core.Display;

public sealed record LimitView(
    LimitKind Kind,
    string Label,
    bool HasData,
    string UsedFigure,
    string LeftFigure,
    double UsedFraction,
    double? ProjectedFraction,
    UsageLevel Level,
    LimitStatus? Status,
    string? ResetsIn,
    string? Detail);

public sealed record CopilotSection(LimitView Credits, string? Note);

public sealed record FlyoutModel(string Title, LimitView Session, LimitView Weekly, LimitView? Spend, string Footer,
    CopilotSection? Copilot = null, bool ShowClaude = true)
{
    public static FlyoutModel From(IReadOnlyList<Limit> all, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture, LiveState live = LiveState.Off, bool legacyConnection = false,
        string? copilotNote = null, bool showClaude = true)
    {
        var credits = all.FirstOrDefault(l => l.Kind == LimitKind.Credits);
        var limits = all.Where(l => l.Kind != LimitKind.Credits).ToList();
        var session = limits.FirstOrDefault(l => l.Kind == LimitKind.Session);
        var weekly = limits.FirstOrDefault(l => l.Kind == LimitKind.Weekly);
        var spend = limits.FirstOrDefault(l => l.Kind == LimitKind.Spend);
        return new FlyoutModel(
            Providers.Claude,
            View(session, LimitKind.Session, "Session", now, zone, culture),
            View(weekly, LimitKind.Weekly, "Weekly", now, zone, culture),
            spend is null ? null : View(spend, LimitKind.Spend, "Spend", now, zone, culture),
            FooterText(showClaude ? limits : credits is null ? [] : [credits], now, zone, culture, live, legacyConnection, showClaude),
            credits is null ? null : new CopilotSection(View(credits, LimitKind.Credits, "AI credits", now, zone, culture), copilotNote),
            showClaude);
    }

    /// <summary>"resets in 3d 13h", "reset at 9:00 AM", or null when there is no reset time.</summary>
    public static string? ResetsIn(Limit limit, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture)
    {
        if (limit.ResetsAt is not { } at) return null;
        return limit.IsReset
            ? "reset at " + When.Format(at, now, zone, culture)
            : "resets in " + Duration.Format(at - now);
    }

    private static LimitView View(Limit? limit, LimitKind kind, string label, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture)
    {
        if (limit is null)
            return new LimitView(kind, label, false, "–", "–", 0, null, UsageLevel.Normal, null, "No reading yet", null);

        var pace = Pace.Project(limit);

        var used = limit.DisplayPercent;
        return new LimitView(
            limit.Kind, limit.Label, true,
            used.ToString(CultureInfo.InvariantCulture) + "%",
            (100 - used).ToString(CultureInfo.InvariantCulture) + "%",
            Math.Clamp(limit.Percent / 100, 0, 1),
            pace is null ? null : Math.Min(pace.ProjectedPercent, 100) / 100,
            UsageLevels.For(used),
            LimitStatus.For(limit, now, zone, culture),
            ResetsIn(limit, now, zone, culture),
            kind == LimitKind.Spend ? SpendDetail(limit)
                : kind == LimitKind.Credits ? CreditsDetail(limit) : null);
    }

    private static string? CreditsDetail(Limit limit) =>
        limit.IsReset ? null : TooltipText.CreditsFigures(limit);

    private static string? SpendDetail(Limit limit)
    {
        if (limit.UsedUsd is not { } used) return null;
        var period = limit.Period switch { "daily" => " today", "weekly" => " this week", "monthly" => " this month", _ => "" };
        return limit.LimitUsd is { } cap
            ? $"{Money(used)} of {Money(cap)}{period}"
            : $"{Money(used)} spent{period}";
    }

    private static string Money(double v) => "$" + v.ToString("#,0.00", CultureInfo.InvariantCulture);

    private static string FooterText(IReadOnlyList<Limit> limits, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture, LiveState live, bool legacyConnection, bool showClaude)
    {
        if (showClaude && limits.Count == 0 && legacyConnection)
            return "No numbers yet. The status line still runs the old OpenHeadroom bridge: choose Reconnect in the tray menu.";
        var sentence = showClaude ? LiveStateText.Footer(live) : null;
        // Only Claude's numbers wait for Claude Code's next response.
        var age = AgeText(limits, now, zone, culture, showClaude ? live : LiveState.Ok);
        if (sentence is null) return age;
        return age + (age.EndsWith('.') ? " " : ". ") + sentence;
    }

    // With Live usage on, the numbers may come from Anthropic before Claude Code's next response.
    private static string AgeText(IReadOnlyList<Limit> limits, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture, LiveState live)
    {
        if (limits.Count == 0)
            return live == LiveState.Off ? "No numbers yet. They arrive with Claude Code's next response." : "No numbers yet.";
        var newest = limits.Max(l => l.ObservedAt);
        var age = now - newest;
        if (age < TimeSpan.FromMinutes(1)) return "Updated just now";
        if (age < TimeSpan.FromMinutes(60)) return $"Updated {((int)age.TotalMinutes).ToString(CultureInfo.InvariantCulture)} min ago";
        if (age < TimeSpan.FromHours(24)) return $"Updated {Duration.Format(age)} ago";
        return "Updated " + When.Format(newest, now, zone, culture);
    }
}
