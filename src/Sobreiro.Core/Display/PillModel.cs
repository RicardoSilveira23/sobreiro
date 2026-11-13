using System.Globalization;
using Sobreiro.Core.Copilot;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Live;
using Sobreiro.Core.Polling;

namespace Sobreiro.Core.Display;

/// <summary>A label and its figure; <paramref name="Stale"/> when the figure is a last reading that is no longer updated.</summary>
public sealed record PillLine(string Label, string Figure, bool Stale = false);

/// <summary>Copilot's block of the pill: the credits figure, the reset date and the bar.</summary>
public sealed record PillCopilot(PillLine Credits, string ResetDate, double Fraction, UsageLevel Level, bool Stale = false);

/// <summary>What the taskbar pill shows: a 5h line, a 7d line and the rings, plus Copilot's block when there is one.</summary>
public sealed record PillModel(PillLine Session, PillLine Weekly, RingsModel Rings, PillCopilot? Copilot = null, bool ShowClaude = true, bool ClaudeStale = false)
{
    private const string Dash = "–";

    public static PillModel From(IReadOnlyList<Limit> limits, TimeZoneInfo zone, bool showClaude) =>
        // Off and Ok are never stale, so the placeholder time is never read.
        From(limits, zone, showClaude, ProviderState.Off, 0, ProviderState.Ok, 0, DateTimeOffset.UnixEpoch);

    /// <summary>As above, and a block whose numbers have stopped keeps its figures and its fill, marked stale.</summary>
    public static PillModel From(IReadOnlyList<Limit> limits, TimeZoneInfo zone, bool showClaude,
        ProviderState live, int liveMisses, ProviderState copilot, int copilotMisses, DateTimeOffset now)
    {
        var claudeStale = PillStale.Claude(live, limits, now, liveMisses);
        var credits = limits.FirstOrDefault(l => l.Kind == LimitKind.Credits);
        return From(limits, claudeStale) with
        {
            Copilot = CopilotBlock(credits, zone, PillStale.Copilot(copilot, credits, now, copilotMisses)),
            ShowClaude = showClaude,
        };
    }

    private static PillCopilot? CopilotBlock(Limit? credits, TimeZoneInfo zone, bool stale)
    {
        if (credits is null) return null;
        var date = credits.ResetsAt is { } at
            ? TimeZoneInfo.ConvertTime(at, zone).ToString("d MMM", CultureInfo.InvariantCulture)
            : Dash;
        return new PillCopilot(
            Line("AI", credits, stale),
            date,
            Math.Clamp(credits.Percent / 100, 0, 1),
            UsageLevels.For(credits.DisplayPercent),
            stale);
    }

    public static PillModel From(IReadOnlyList<Limit> limits) => From(limits, false);

    private static PillModel From(IReadOnlyList<Limit> limits, bool claudeStale) => new(
        Line("5h", limits.FirstOrDefault(l => l.Kind == LimitKind.Session), claudeStale),
        Line("7d", limits.FirstOrDefault(l => l.Kind == LimitKind.Weekly), claudeStale),
        RingsModel.From(limits),
        ClaudeStale: claudeStale);

    // A line with no reading is a dash and never stale: there is no figure to mark.
    private static PillLine Line(string label, Limit? limit, bool stale) => limit is null
        ? new(label, Dash)
        : new(label, limit.DisplayPercent.ToString(CultureInfo.InvariantCulture) + "%", stale);
}
