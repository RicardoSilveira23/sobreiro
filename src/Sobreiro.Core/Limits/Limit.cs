using Sobreiro.Core.Connect;
using Sobreiro.Core.Copilot;
using Sobreiro.Core.Usage;

namespace Sobreiro.Core.Limits;

public enum LimitKind { Session, Weekly, Spend, Credits }

public sealed record Limit(
    string Provider,
    LimitKind Kind,
    string Label,
    double Percent,
    DateTimeOffset? ResetsAt,
    DateTimeOffset ObservedAt,
    bool IsReset,
    TimeSpan? Window = null,
    double? UsedUsd = null,
    double? LimitUsd = null,
    string? Period = null,
    double? UsedCredits = null,
    double? CreditBudget = null)
{
    /// <summary>Floor of <see cref="Percent"/>, clamped to 0-100.</summary>
    public int DisplayPercent => (int)Math.Clamp(Math.Floor(Percent), 0, 100);
}

public static class LimitList
{
    public static IReadOnlyList<Limit> From(UsageSnapshot? snapshot, DateTimeOffset now)
    {
        var list = new List<Limit>();
        if (snapshot is null) return list;

        var l = snapshot.Limits;
        if (l.FiveHour is { } five)
            list.Add(Make(LimitKind.Session, "Session", five.UsedPercentage, five.ResetsAt, five.ObservedAt ?? snapshot.ObservedAt, now, TimeSpan.FromHours(5)));
        if (l.SevenDay is { } seven)
            list.Add(Make(LimitKind.Weekly, "Weekly", seven.UsedPercentage, seven.ResetsAt, seven.ObservedAt ?? snapshot.ObservedAt, now, TimeSpan.FromDays(7)));
        if (l.Spend is { } spend)
            list.Add(Make(LimitKind.Spend, "Spend", spend.UsedPercentage, spend.ResetsAt, spend.ObservedAt ?? snapshot.ObservedAt, now,
                SpendWindow(spend.Period, spend.ResetsAt), spend.UsedUsd, spend.LimitUsd, spend.Period));
        return list;
    }

    public static IReadOnlyList<Limit> FromCopilot(CopilotReading? reading, DateTimeOffset now)
    {
        if (reading is null) return [];
        var resetsAt = reading.ResetsAt;
        var isReset = resetsAt is { } r && r <= now;
        TimeSpan? window = resetsAt is { } end ? end - end.AddMonths(-1) : null;
        return [new Limit(Providers.Copilot, LimitKind.Credits, "AI credits", isReset ? 0 : reading.Percent, resetsAt,
            reading.ObservedAt, isReset, window, UsedCredits: reading.Used, CreditBudget: reading.Budget)];
    }

    /// <summary>Claude is shown when it is on this PC, or when nothing else is shown.</summary>
    public static bool ShowClaude(bool claudeOnPc, bool copilotShown) => claudeOnPc || !copilotShown;

    public static bool ClaudeOnPc(bool credentialsFileExists, int claudeLimitCount, ConnectionState connection) =>
        credentialsFileExists || claudeLimitCount > 0 || connection != ConnectionState.NotConnected;

    private static Limit Make(LimitKind kind, string label, double percent, DateTimeOffset? resetsAt,
        DateTimeOffset observedAt, DateTimeOffset now, TimeSpan? window = null,
        double? usedUsd = null, double? limitUsd = null, string? period = null)
    {
        var isReset = resetsAt is { } r && r <= now;
        return new Limit(Providers.Claude, kind, label, isReset ? 0 : percent, resetsAt, observedAt, isReset, window, usedUsd, limitUsd, period);
    }

    private static TimeSpan? SpendWindow(string? period, DateTimeOffset? resetsAt) => period switch
    {
        "daily" => TimeSpan.FromDays(1),
        "weekly" => TimeSpan.FromDays(7),
        "monthly" when resetsAt is { } r => r - r.AddMonths(-1),
        _ => null,
    };
}
