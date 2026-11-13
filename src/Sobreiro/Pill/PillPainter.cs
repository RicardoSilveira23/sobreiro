using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Imaging;
using System.Drawing.Text;
using System.Runtime.InteropServices;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;

namespace Sobreiro.Pill;

/// <summary>
/// SPIKE (shell candidate F): the pill of <see cref="PillView"/> drawn by hand with GDI+ into a
/// premultiplied 32-bit bitmap, in physical pixels at a given scale. Same measures as PillView.
/// </summary>
internal static class PillPainter
{
    private const float LabelSize = 12, FigureSize = 11, ResetSize = 11;
    private const float RingsSize = 26, Stroke = 3, RingGap = 2, TextToRings = 8, BlockGap = 12;
    private const float BarWidth = 5, BarHeight = 26, BarRadius = 2.5f;
    private const float PadX = 8, PadY = 4, Corner = 4;
    private const byte HoverAlpha = 0x1A, ResetAlpha = 0x99;

    private static readonly PrivateFontCollection Fonts = new();
    private static readonly FontFamily Inter = Load("Inter-Regular.ttf");
    private static readonly FontFamily Unbounded = Load("Unbounded-Bold.ttf");
    private static readonly StringFormat Format = TypographicFormat();
    private static readonly Bitmap MeasureSurface = new(1, 1, PixelFormat.Format32bppPArgb);

    private readonly record struct Seg(string Text, Font Font, Color Colour, float Width);

    private sealed record TextLine(List<Seg> Segs, float Width, float Ascent, float Height);

    public static Bitmap Draw(PillModel model, TaskbarColours colours, double scaleFactor, bool hover)
    {
        var s = (float)scaleFactor;
        var ink = colours.Ink;
        var fonts = new List<Font>();
        try
        {
            using var measure = Graphics.FromImage(MeasureSurface);
            measure.TextRenderingHint = TextRenderingHint.AntiAlias;
            var label = Make(fonts, Inter, LabelSize * s);
            var figure = Make(fonts, Unbounded, FigureSize * s);
            var reset = Make(fonts, Inter, ResetSize * s);

            TextLine[]? claude = model.ShowClaude
                ? [Line(measure, model.Session, label, figure, ink), Line(measure, model.Weekly, label, figure, ink)]
                : null;
            TextLine[]? copilot = model.Copilot is { } c
                ? [Line(measure, c.Credits, label, figure, ink), Plain(measure, c.ResetDate, reset, Color.FromArgb(ResetAlpha, ink.R, ink.G, ink.B))]
                : null;

            float ColumnWidth(TextLine[] lines) => MathF.Ceiling(lines.Max(l => l.Width));
            float ColumnHeight(TextLine[] lines) => MathF.Ceiling(lines.Sum(l => l.Height));

            float rowWidth = 0, rowHeight = 0;
            if (claude is not null)
            {
                rowWidth += ColumnWidth(claude) + MathF.Round((TextToRings + RingsSize) * s);
                rowHeight = MathF.Max(ColumnHeight(claude), MathF.Round(RingsSize * s));
            }
            if (copilot is not null)
            {
                rowWidth += (claude is null ? 0 : MathF.Round(BlockGap * s)) + ColumnWidth(copilot) + MathF.Round((TextToRings + BarWidth) * s);
                rowHeight = MathF.Max(rowHeight, MathF.Max(ColumnHeight(copilot), MathF.Round(BarHeight * s)));
            }
            var padX = MathF.Round(PadX * s);
            var padY = MathF.Round(PadY * s);
            var width = Math.Max(1, (int)(rowWidth + 2 * padX));
            var height = Math.Max(1, (int)(rowHeight + 2 * padY));

            var bitmap = new Bitmap(width, height, PixelFormat.Format32bppPArgb);
            using var g = Graphics.FromImage(bitmap);
            g.Clear(Color.Transparent);
            g.SmoothingMode = SmoothingMode.AntiAlias;
            g.PixelOffsetMode = PixelOffsetMode.Half;
            g.TextRenderingHint = TextRenderingHint.AntiAlias;

            // Nearly clear at rest, so clicks land on this window and not on what is behind it.
            using (var face = Rounded(new RectangleF(0, 0, width, height), Corner * s))
            using (var ground = new SolidBrush(Color.FromArgb(hover ? HoverAlpha : (byte)1, ink.R, ink.G, ink.B)))
                g.FillPath(ground, face);

            var x = padX;
            if (claude is not null)
            {
                var w = ColumnWidth(claude);
                DrawColumn(g, claude, x, padY + MathF.Round((rowHeight - ColumnHeight(claude)) / 2), w);
                x += w + MathF.Round(TextToRings * s);
                var size = MathF.Round(RingsSize * s);
                Rings(g, model.Rings, colours, model.ClaudeStale, x, padY + MathF.Round((rowHeight - size) / 2), s);
                x += size;
            }
            if (copilot is not null && model.Copilot is { } block)
            {
                if (claude is not null) x += MathF.Round(BlockGap * s);
                var w = ColumnWidth(copilot);
                DrawColumn(g, copilot, x, padY + MathF.Round((rowHeight - ColumnHeight(copilot)) / 2), w);
                x += w + MathF.Round(TextToRings * s);
                var barHeight = MathF.Round(BarHeight * s);
                LevelBar(g, block, colours, x, padY + MathF.Round((rowHeight - barHeight) / 2), MathF.Round(BarWidth * s), barHeight, BarRadius * s);
            }
            return bitmap;
        }
        finally
        {
            foreach (var font in fonts) font.Dispose();
        }
    }

    // The label, the figure, and "*" in the label's font after a figure that is no longer updated.
    private static TextLine Line(Graphics g, PillLine line, Font label, Font figure, Rgb ink)
    {
        var colour = Color.FromArgb(ink.R, ink.G, ink.B);
        var segs = new List<Seg> { Measure(g, line.Label + " ", label, colour), Measure(g, line.Figure, figure, colour) };
        if (line.Stale) segs.Add(Measure(g, "*", label, colour));
        return Lay(segs);
    }

    private static TextLine Plain(Graphics g, string text, Font font, Color colour) => Lay([Measure(g, text, font, colour)]);

    private static Seg Measure(Graphics g, string text, Font font, Color colour) =>
        new(text, font, colour, g.MeasureString(text, font, PointF.Empty, Format).Width);

    // A line is as tall as its tallest font, and every run sits on one baseline.
    private static TextLine Lay(List<Seg> segs) =>
        new(segs, segs.Sum(x => x.Width), segs.Max(x => Ascent(x.Font)), segs.Max(x => Ascent(x.Font)) + segs.Max(x => Descent(x.Font)));

    private static void DrawColumn(Graphics g, TextLine[] lines, float left, float top, float width)
    {
        var y = top;
        foreach (var line in lines)
        {
            var x = left + width - line.Width;
            foreach (var seg in line.Segs)
            {
                using var brush = new SolidBrush(seg.Colour);
                g.DrawString(seg.Text, seg.Font, brush, x, y + line.Ascent - Ascent(seg.Font), Format);
                x += seg.Width;
            }
            y += line.Height;
        }
    }

    private static void Rings(Graphics g, RingsModel model, TaskbarColours colours, bool stale, float left, float top, float s)
    {
        var centre = new PointF(left + RingsSize * s / 2, top + RingsSize * s / 2);
        var outer = RingsSize / 2 - Stroke / 2;
        Ring(g, model.Outer, colours, stale, centre, outer, s);
        Ring(g, model.Inner, colours, stale, centre, outer - Stroke - RingGap, s);
    }

    // The track as a full circle at the track alpha, then the arc clockwise from 12 o'clock.
    private static void Ring(Graphics g, Ring ring, TaskbarColours colours, bool stale, PointF centre, float radius, float s)
    {
        var box = new RectangleF(centre.X - radius * s, centre.Y - radius * s, radius * 2 * s, radius * 2 * s);
        var ink = colours.Ink;
        using (var track = new Pen(Color.FromArgb(TaskbarColours.TrackAlpha, ink.R, ink.G, ink.B), Stroke * s))
            g.DrawEllipse(track, box);

        var sweep = ring.CappedSweep(radius, Stroke);
        if (sweep <= 0) return;
        using var pen = new Pen(Fill(colours, ring.Level, stale), Stroke * s) { StartCap = LineCap.Round, EndCap = LineCap.Round };
        if (sweep >= 360) g.DrawEllipse(pen, box);
        else g.DrawArc(pen, box, -90f, (float)sweep);
    }

    // A rounded track filling from the bottom, clipped to its own shape.
    private static void LevelBar(Graphics g, PillCopilot copilot, TaskbarColours colours, float left, float top, float width, float height, float radius)
    {
        using var shape = Rounded(new RectangleF(left, top, width, height), radius);
        var saved = g.Save();
        g.SetClip(shape, CombineMode.Intersect);
        var ink = colours.Ink;
        using (var track = new SolidBrush(Color.FromArgb(TaskbarColours.TrackAlpha, ink.R, ink.G, ink.B)))
            g.FillRectangle(track, left, top, width, height);
        var filled = height * (float)Math.Clamp(copilot.Fraction, 0, 1);
        if (filled > 0)
        {
            using var fill = new SolidBrush(Fill(colours, copilot.Level, copilot.Stale));
            g.FillRectangle(fill, left, top + height - filled, width, filled);
        }
        g.Restore(saved);
    }

    // A stopped block's fill is the ink at half strength: the level's colour goes, the amount stays.
    private static Color Fill(TaskbarColours colours, UsageLevel level, bool stale)
    {
        if (stale) return Color.FromArgb(TaskbarColours.StaleAlpha, colours.Ink.R, colours.Ink.G, colours.Ink.B);
        var fill = colours.Fill(level);
        return Color.FromArgb(fill.R, fill.G, fill.B);
    }

    private static GraphicsPath Rounded(RectangleF r, float radius)
    {
        var d = radius * 2;
        var path = new GraphicsPath();
        path.AddArc(r.Left, r.Top, d, d, 180, 90);
        path.AddArc(r.Right - d, r.Top, d, d, 270, 90);
        path.AddArc(r.Right - d, r.Bottom - d, d, d, 0, 90);
        path.AddArc(r.Left, r.Bottom - d, d, d, 90, 90);
        path.CloseFigure();
        return path;
    }

    private static Font Make(List<Font> made, FontFamily family, float pixels)
    {
        var font = new Font(family, pixels, StyleOf(family), GraphicsUnit.Pixel);
        made.Add(font);
        return font;
    }

    private static FontStyle StyleOf(FontFamily family) =>
        family.IsStyleAvailable(FontStyle.Regular) ? FontStyle.Regular : FontStyle.Bold;

    private static float Ascent(Font f) => f.Size * f.FontFamily.GetCellAscent(f.Style) / f.FontFamily.GetEmHeight(f.Style);

    private static float Descent(Font f) =>
        f.Size * (f.FontFamily.GetLineSpacing(f.Style) - f.FontFamily.GetCellAscent(f.Style)) / f.FontFamily.GetEmHeight(f.Style);

    private static StringFormat TypographicFormat() =>
        new(StringFormat.GenericTypographic) { FormatFlags = StringFormat.GenericTypographic.FormatFlags | StringFormatFlags.MeasureTrailingSpaces };

    // The app's own font files, from the exe. GDI+ reads the bytes for as long as the collection lives, so they are never freed.
    private static FontFamily Load(string file)
    {
        using var stream = typeof(PillPainter).Assembly.GetManifestResourceStream("font/" + file)
            ?? throw new InvalidOperationException("Font not in the exe: " + file);
        var bytes = new byte[stream.Length];
        stream.ReadExactly(bytes);
        var memory = Marshal.AllocCoTaskMem(bytes.Length);
        Marshal.Copy(bytes, 0, memory, bytes.Length);
        var before = Fonts.Families.Length;
        Fonts.AddMemoryFont(memory, bytes.Length);
        var families = Fonts.Families;
        if (families.Length <= before) throw new InvalidOperationException("Font not loaded: " + file);
        var wanted = file.StartsWith("Inter", StringComparison.Ordinal) ? "Inter" : "Unbounded";
        return families.First(f => f.Name.StartsWith(wanted, StringComparison.Ordinal));
    }
}
