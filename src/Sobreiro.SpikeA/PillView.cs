using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Documents;
using Avalonia.Controls.Shapes;
using Avalonia.Layout;
using Avalonia.Media;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;
using Path = Avalonia.Controls.Shapes.Path;

namespace Sobreiro.SpikeA;

/// <summary>
/// SPIKE (shell candidate A): the pill's face in the toolkit's controls, to the same measures as the WPF
/// face: two right-aligned lines, two rings, Copilot's block and bar, the stale mark.
/// </summary>
internal static class PillView
{
    private const double LabelSize = 12;
    private const double FigureSize = 11;
    private const double RingsSize = 26;
    private const double Stroke = 3;
    private const double RingGap = 2;
    private const double TextToRings = 8;
    private const byte HoverAlpha = 0x1A;
    private const double BlockGap = 12;
    private const double ResetSize = 11;
    private const byte ResetAlpha = 0x99;
    private const double BarWidth = 5;
    private const double BarHeight = 26;
    private const double BarRadius = 2.5;

    public static readonly FontFamily Inter = new("avares://Sobreiro/Fonts#Inter");
    public static readonly FontFamily Unbounded = new("avares://Sobreiro/Fonts#Unbounded");

    public static Control Build(PillModel model, TaskbarColours colours)
    {
        var ink = colours.Ink;
        // Nearly clear at rest, as in the WPF face.
        var ground = Solid(ink, 1);
        var hover = Solid(ink, HoverAlpha);

        var row = new StackPanel { Orientation = Orientation.Horizontal };
        if (model.ShowClaude)
        {
            var lines = new StackPanel { VerticalAlignment = VerticalAlignment.Center };
            lines.Children.Add(Line(model.Session, ink));
            lines.Children.Add(Line(model.Weekly, ink));

            var rings = Rings(model.Rings, colours, model.ClaudeStale);
            rings.Margin = new Thickness(TextToRings, 0, 0, 0);

            row.Children.Add(lines);
            row.Children.Add(rings);
        }
        if (model.Copilot is { } copilot)
            row.Children.Add(CopilotBlock(copilot, colours, model.ShowClaude));

        var face = new Border
        {
            Background = ground,
            CornerRadius = new CornerRadius(4),
            Padding = new Thickness(8, 4, 8, 4),
            UseLayoutRounding = true,
            VerticalAlignment = VerticalAlignment.Center,
            Child = row,
        };
        face.PointerEntered += (_, _) => face.Background = hover;
        face.PointerExited += (_, _) => face.Background = ground;
        return face;
    }

    private static StackPanel CopilotBlock(PillCopilot copilot, TaskbarColours colours, bool afterClaude)
    {
        var ink = colours.Ink;
        var lines = new StackPanel { VerticalAlignment = VerticalAlignment.Center };
        lines.Children.Add(Line(copilot.Credits, ink));
        lines.Children.Add(new TextBlock
        {
            Text = copilot.ResetDate,
            Foreground = Solid(ink, ResetAlpha),
            FontFamily = Inter,
            FontSize = ResetSize,
            TextAlignment = TextAlignment.Right,
            HorizontalAlignment = HorizontalAlignment.Right,
        });

        // A rounded track filling from the bottom: the border clips its child to its own corners.
        var height = BarHeight * Math.Clamp(copilot.Fraction, 0, 1);
        var bar = new Border
        {
            Width = BarWidth,
            Height = BarHeight,
            CornerRadius = new CornerRadius(BarRadius),
            Background = Solid(ink, TaskbarColours.TrackAlpha),
            ClipToBounds = true,
            VerticalAlignment = VerticalAlignment.Center,
            Margin = new Thickness(TextToRings, 0, 0, 0),
            Child = new Rectangle
            {
                Width = BarWidth,
                Height = height,
                VerticalAlignment = VerticalAlignment.Bottom,
                Fill = Fill(colours, copilot.Level, copilot.Stale),
            },
        };

        var block = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Margin = new Thickness(afterClaude ? BlockGap : 0, 0, 0, 0),
        };
        block.Children.Add(lines);
        block.Children.Add(bar);
        return block;
    }

    private static IBrush Fill(TaskbarColours colours, UsageLevel level, bool stale) =>
        stale ? Solid(colours.Ink, TaskbarColours.StaleAlpha) : Solid(colours.Fill(level));

    // The label, the figure, and "*" in the label's font after a figure that is no longer updated.
    private static TextBlock Line(PillLine line, Rgb ink)
    {
        var text = new TextBlock
        {
            Foreground = Solid(ink),
            TextAlignment = TextAlignment.Right,
            HorizontalAlignment = HorizontalAlignment.Right,
            // The WPF face's line: without it the toolkit gives the figure's font 19 px a line.
            LineHeight = 15,
        };
        text.Inlines!.Add(new Run(line.Label + " ") { FontFamily = Inter, FontSize = LabelSize });
        text.Inlines.Add(new Run(line.Figure) { FontFamily = Unbounded, FontWeight = FontWeight.Bold, FontSize = FigureSize });
        if (line.Stale) text.Inlines.Add(new Run("*") { FontFamily = Inter, FontSize = LabelSize });
        return text;
    }

    private static Canvas Rings(RingsModel model, TaskbarColours colours, bool stale)
    {
        var canvas = new Canvas { Width = RingsSize, Height = RingsSize, VerticalAlignment = VerticalAlignment.Center };
        var outer = RingsSize / 2 - Stroke / 2;
        Ring(canvas, model.Outer, colours, outer, stale);
        Ring(canvas, model.Inner, colours, outer - Stroke - RingGap, stale);
        return canvas;
    }

    // The track as a full circle at the track alpha, then the arc clockwise from 12 o'clock.
    private static void Ring(Canvas canvas, Ring ring, TaskbarColours colours, double radius, bool stale)
    {
        var centre = new Point(RingsSize / 2, RingsSize / 2);
        canvas.Children.Add(new Path
        {
            Data = new EllipseGeometry { Center = centre, RadiusX = radius, RadiusY = radius },
            Stroke = Solid(colours.Ink, TaskbarColours.TrackAlpha),
            StrokeThickness = Stroke,
        });

        var sweep = ring.CappedSweep(radius, Stroke);
        if (sweep <= 0) return;
        var fill = Fill(colours, ring.Level, stale);
        if (sweep >= 360)
        {
            canvas.Children.Add(new Path { Data = new EllipseGeometry { Center = centre, RadiusX = radius, RadiusY = radius }, Stroke = fill, StrokeThickness = Stroke });
            return;
        }
        canvas.Children.Add(new Path
        {
            Data = Arc(centre, radius, sweep),
            Stroke = fill,
            StrokeThickness = Stroke,
            StrokeLineCap = PenLineCap.Round,
        });
    }

    public static Geometry Arc(Point centre, double radius, double sweep)
    {
        var radians = sweep * Math.PI / 180;
        var end = new Point(centre.X + radius * Math.Sin(radians), centre.Y - radius * Math.Cos(radians));
        var geometry = new StreamGeometry();
        using (var context = geometry.Open())
        {
            context.BeginFigure(new Point(centre.X, centre.Y - radius), false);
            context.ArcTo(end, new Size(radius, radius), 0, sweep > 180, SweepDirection.Clockwise);
            context.EndFigure(false);
        }
        return geometry;
    }

    public static IBrush Solid(Rgb c, byte alpha = 0xFF) => new SolidColorBrush(Color.FromArgb(alpha, c.R, c.G, c.B));
}
