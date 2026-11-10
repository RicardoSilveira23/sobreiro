using System.Globalization;
using Sobreiro.Core.Limits;

namespace Sobreiro.Core.Display;

public sealed record PillLine(string Label, string Figure);

/// <summary>Copilot's block of the pill: the credits figure, the reset date and the bar.</summary>
public sealed record PillCopilot(PillLine Credits, string ResetDate, double Fraction, UsageLevel Level);

/// <summary>What the taskbar pill shows: a 5h line, a 7d line and the rings, plus Copilot's block when there is one.</summary>
public sealed record PillModel(PillLine Session, PillLine Weekly, RingsModel Rings, PillCopilot? Copilot = null, bool ShowClaude = true)
{
    private const string Dash = "–";

    public static PillModel From(IReadOnlyList<Limit> limits, TimeZoneInfo zone, bool showClaude) =>
        From(limits) with { Copilot = CopilotBlock(limits.FirstOrDefault(l => l.Kind == LimitKind.Credits), zone), ShowClaude = showClaude };

    private static PillCopilot? CopilotBlock(Limit? credits, TimeZoneInfo zone)
    {
        if (credits is null) return null;
        var date = credits.ResetsAt is { } at
            ? TimeZoneInfo.ConvertTime(at, zone).ToString("d MMM", CultureInfo.InvariantCulture)
            : Dash;
        return new PillCopilot(
            new PillLine("AI", credits.DisplayPercent.ToString(CultureInfo.InvariantCulture) + "%"),
            date,
            Math.Clamp(credits.Percent / 100, 0, 1),
            UsageLevels.For(credits.DisplayPercent));
    }

    public static PillModel From(IReadOnlyList<Limit> limits) => new(
        Line("5h", limits.FirstOrDefault(l => l.Kind == LimitKind.Session)),
        Line("7d", limits.FirstOrDefault(l => l.Kind == LimitKind.Weekly)),
        RingsModel.From(limits));

    private static PillLine Line(string label, Limit? limit) =>
        new(label, limit is null ? Dash : limit.DisplayPercent.ToString(CultureInfo.InvariantCulture) + "%");
}
