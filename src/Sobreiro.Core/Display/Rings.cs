using Sobreiro.Core.Limits;

namespace Sobreiro.Core.Display;

/// <summary>One taskbar/tray ring: how full it is and what colour level it takes.</summary>
public readonly record struct Ring(int Percent, UsageLevel Level, bool HasData)
{
    public static Ring None { get; } = new(0, UsageLevel.Normal, false);

    /// <summary>Clockwise sweep from 12 o'clock; 0 without data.</summary>
    public double SweepDegrees => HasData ? Percent * 360.0 / 100 : 0;

    /// <summary>
    /// Degrees to draw with round caps so the visible arc spans the ring's percentage: the caps add
    /// stroke / radius radians. 0 without data or at 0%, 360 (a full circle) at 100%, never under 0.5.
    /// </summary>
    public double CappedSweep(double radius, double stroke)
    {
        if (!HasData || Percent <= 0) return 0;
        if (Percent >= 100) return 360;
        return Math.Max(0.5, SweepDegrees - stroke / radius * 180 / Math.PI);
    }

    public static Ring From(Limit? limit) =>
        limit is null ? None : new Ring(limit.DisplayPercent, UsageLevels.For(limit.DisplayPercent), true);
}

/// <summary>Outer ring is the session (5h) window, inner ring the weekly (7d). Spend is not drawn.</summary>
public sealed record RingsModel(Ring Outer, Ring Inner, bool HasInner = true)
{
    /// <summary>Copilot's credits fill one ring.</summary>
    public static RingsModel Copilot(IReadOnlyList<Limit> limits) =>
        new(Ring.From(limits.FirstOrDefault(l => l.Kind == LimitKind.Credits)), Ring.None, HasInner: false);

    public static RingsModel From(IReadOnlyList<Limit> limits) => new(
        Ring.From(limits.FirstOrDefault(l => l.Kind == LimitKind.Session)),
        Ring.From(limits.FirstOrDefault(l => l.Kind == LimitKind.Weekly)));
}
