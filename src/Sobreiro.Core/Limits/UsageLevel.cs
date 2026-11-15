namespace Sobreiro.Core.Limits;

public enum UsageLevel { Normal, Warning, Error }

public static class UsageLevels
{
    public const int WarningFrom = 70;
    public const int ErrorFrom = 90;

    public static UsageLevel For(int displayPercent) =>
        displayPercent >= ErrorFrom ? UsageLevel.Error
        : displayPercent >= WarningFrom ? UsageLevel.Warning
        : UsageLevel.Normal;

    public static UsageLevel Highest(IReadOnlyList<Limit> limits) =>
        limits.Count == 0 ? UsageLevel.Normal : limits.Max(l => For(l.DisplayPercent));
}
