using Sobreiro.Core.Usage;

namespace Sobreiro.Core.Display;

public static class StatusLineText
{
    /// <summary>The bridge's default line. Shows exactly what Claude Code just sent, so no reset logic.</summary>
    public static string Render(RateLimits? limits)
    {
        var line = $"5h {Percent(limits?.FiveHour?.UsedPercentage)} · 7d {Percent(limits?.SevenDay?.UsedPercentage)}";
        if (limits?.Spend is { } spend) line += $" · spend {Percent(spend.UsedPercentage)}";
        return line;
    }

    private static string Percent(double? used) =>
        used is { } u ? (int)Math.Clamp(Math.Floor(u), 0, 100) + "%" : "–";
}
