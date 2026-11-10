namespace Sobreiro.Core.Display;

/// <summary>
/// What the battery and thermometer show for a limit: what is left now, and how much of that the
/// pace will use by the reset (drawn faint). The battery counts tenths, rounded half away from zero.
/// </summary>
public sealed record GlyphFill(int LeftCells, int SoonCells, double LeftPercent, double SoonPercent)
{
    public const int Cells = 10;

    public static GlyphFill From(LimitView view)
    {
        if (!view.HasData) return new GlyphFill(0, 0, 0, 0);

        var used = view.UsedFraction * 100;
        var left = Cells - Round(used / 10);
        var leftPercent = 100 - used;
        if (view.ProjectedFraction is not double fraction)
            return new GlyphFill(left, 0, leftPercent, 0);

        var gain = fraction * 100 - used;
        return new GlyphFill(
            left,
            Math.Min(left, Math.Max(0, Round(gain / 10))),
            leftPercent,
            Math.Clamp(gain, 0, leftPercent));
    }

    private static int Round(double v) => (int)Math.Round(v, MidpointRounding.AwayFromZero);
}
