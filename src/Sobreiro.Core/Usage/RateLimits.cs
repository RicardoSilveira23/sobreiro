namespace Sobreiro.Core.Usage;

public sealed record LimitWindow(double UsedPercentage, DateTimeOffset? ResetsAt, DateTimeOffset? ObservedAt = null);

public sealed record SpendLimit(
    double UsedPercentage,
    DateTimeOffset? ResetsAt,
    double? UsedUsd,
    double? LimitUsd,
    string? Period,
    DateTimeOffset? ObservedAt = null);

public sealed record RateLimits(LimitWindow? FiveHour, LimitWindow? SevenDay, SpendLimit? Spend);

public sealed record UsageSnapshot(RateLimits Limits, DateTimeOffset ObservedAt);
