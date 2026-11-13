using System.Globalization;
using Sobreiro.Core.Copilot;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Live;
using Sobreiro.Core.Polling;

namespace Sobreiro.Core.Display;

public sealed record PillLine(string Label, string Figure);

/// <summary>Copilot's block of the pill: the credits figure, the reset date and the bar.</summary>
public sealed record PillCopilot(PillLine Credits, string ResetDate, double Fraction, UsageLevel Level, PillWords? Words = null);

/// <summary>What the taskbar pill shows: a 5h line, a 7d line and the rings, plus Copilot's block when there is one.</summary>
public sealed record PillModel(PillLine Session, PillLine Weekly, RingsModel Rings, PillCopilot? Copilot = null, bool ShowClaude = true, PillWords? ClaudeWords = null)
{
    private const string Dash = "–";

    public static PillModel From(IReadOnlyList<Limit> limits, TimeZoneInfo zone, bool showClaude) =>
        // Off and Ok never give words, so the placeholder time is never read.
        From(limits, zone, showClaude, ProviderState.Off, 0, ProviderState.Ok, 0, DateTimeOffset.UnixEpoch);

    /// <summary>As above, and a block whose numbers have stopped carries its two words and an empty ring or bar.</summary>
    public static PillModel From(IReadOnlyList<Limit> limits, TimeZoneInfo zone, bool showClaude,
        ProviderState live, int liveMisses, ProviderState copilot, int copilotMisses, DateTimeOffset now)
    {
        var claudeWords = PillWords.ForClaude(live, limits, now, liveMisses);
        var credits = limits.FirstOrDefault(l => l.Kind == LimitKind.Credits);
        return From(claudeWords is null ? limits : []) with
        {
            Copilot = CopilotBlock(credits, zone, PillWords.ForCopilot(copilot, credits, now, copilotMisses)),
            ShowClaude = showClaude,
            ClaudeWords = claudeWords,
        };
    }

    private static PillCopilot? CopilotBlock(Limit? credits, TimeZoneInfo zone, PillWords? words)
    {
        if (credits is null) return null;
        var date = credits.ResetsAt is { } at
            ? TimeZoneInfo.ConvertTime(at, zone).ToString("d MMM", CultureInfo.InvariantCulture)
            : Dash;
        return new PillCopilot(
            new PillLine("AI", credits.DisplayPercent.ToString(CultureInfo.InvariantCulture) + "%"),
            date,
            words is null ? Math.Clamp(credits.Percent / 100, 0, 1) : 0,
            UsageLevels.For(credits.DisplayPercent),
            words);
    }

    public static PillModel From(IReadOnlyList<Limit> limits) => new(
        Line("5h", limits.FirstOrDefault(l => l.Kind == LimitKind.Session)),
        Line("7d", limits.FirstOrDefault(l => l.Kind == LimitKind.Weekly)),
        RingsModel.From(limits));

    private static PillLine Line(string label, Limit? limit) =>
        new(label, limit is null ? Dash : limit.DisplayPercent.ToString(CultureInfo.InvariantCulture) + "%");
}
