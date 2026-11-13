using System.Windows;
using System.Windows.Controls;
using System.Windows.Documents;
using System.Windows.Media;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Display;
using Brush = System.Windows.Media.Brush;
using Color = System.Windows.Media.Color;
using FontFamily = System.Windows.Media.FontFamily;
using HorizontalAlignment = System.Windows.HorizontalAlignment;
using Orientation = System.Windows.Controls.Orientation;
using Path = System.Windows.Shapes.Path;
using Point = System.Windows.Point;
using Size = System.Windows.Size;

namespace Sobreiro.Pill;

/// <summary>
/// Draws a <see cref="PillModel"/> for the taskbar: two right-aligned lines (label in Inter, figure
/// in Unbounded) and two concentric rings, in DIPs and in the taskbar's ink.
/// </summary>
public static class PillView
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

    private static readonly Uri Root = new("pack://application:,,,/");
    private static readonly FontFamily Inter = new(Root, "./Fonts/#Inter");
    private static readonly FontFamily Unbounded = new(Root, "./Fonts/#Unbounded");

    public static FrameworkElement Build(PillModel model, TaskbarColours colours)
    {
        var ink = colours.Ink;
        // Nearly clear at rest, so clicks land on this window and not on what is behind it.
        var ground = Solid(ink, 1);
        var hover = Solid(ink, HoverAlpha);

        var row = new StackPanel { Orientation = Orientation.Horizontal };
        if (model.ShowClaude)
        {
            var lines = new StackPanel { VerticalAlignment = VerticalAlignment.Center };
            if (model.ClaudeWords is { } words)
            {
                lines.Children.Add(Word(words.Top, ink));
                lines.Children.Add(Word(words.Bottom, ink));
            }
            else
            {
                lines.Children.Add(Line(model.Session, ink));
                lines.Children.Add(Line(model.Weekly, ink));
            }

            var rings = Rings(model.Rings, colours);
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
            Child = row,
        };
        face.MouseEnter += (_, _) => face.Background = hover;
        face.MouseLeave += (_, _) => face.Background = ground;
        return face;
    }

    // Two right-aligned lines (credits figure, reset date), then a level bar.
    private static StackPanel CopilotBlock(PillCopilot copilot, TaskbarColours colours, bool afterClaude)
    {
        var ink = colours.Ink;
        var lines = new StackPanel { VerticalAlignment = VerticalAlignment.Center };
        if (copilot.Words is { } words)
        {
            lines.Children.Add(Word(words.Top, ink));
            lines.Children.Add(Word(words.Bottom, ink));
        }
        else
        {
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
        }

        var bar = LevelBar(copilot, colours);
        bar.Margin = new Thickness(TextToRings, 0, 0, 0);

        var block = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Margin = new Thickness(afterClaude ? BlockGap : 0, 0, 0, 0),
        };
        block.Children.Add(lines);
        block.Children.Add(bar);
        return block;
    }

    // A rounded track filling from the bottom, clipped to its own shape.
    private static Canvas LevelBar(PillCopilot copilot, TaskbarColours colours)
    {
        var shape = new RectangleGeometry(new Rect(0, 0, BarWidth, BarHeight), BarRadius, BarRadius);
        var canvas = new Canvas
        {
            Width = BarWidth,
            Height = BarHeight,
            VerticalAlignment = VerticalAlignment.Center,
            Clip = shape,
        };
        canvas.Children.Add(new System.Windows.Shapes.Rectangle
        {
            Width = BarWidth,
            Height = BarHeight,
            Fill = Solid(colours.Ink, TaskbarColours.TrackAlpha),
        });
        var height = BarHeight * Math.Clamp(copilot.Fraction, 0, 1);
        if (height > 0)
        {
            var fill = new System.Windows.Shapes.Rectangle { Width = BarWidth, Height = height, Fill = Solid(colours.Fill(copilot.Level)) };
            Canvas.SetTop(fill, BarHeight - height);
            canvas.Children.Add(fill);
        }
        return canvas;
    }

    // One word of a stopped block, in the label's font and size.
    private static TextBlock Word(string text, Rgb ink) => new()
    {
        Text = text,
        Foreground = Solid(ink),
        FontFamily = Inter,
        FontSize = LabelSize,
        TextAlignment = TextAlignment.Right,
        HorizontalAlignment = HorizontalAlignment.Right,
    };

    private static TextBlock Line(PillLine line, Rgb ink)
    {
        var text = new TextBlock
        {
            Foreground = Solid(ink),
            TextAlignment = TextAlignment.Right,
            HorizontalAlignment = HorizontalAlignment.Right,
        };
        text.Inlines.Add(new Run(line.Label + " ") { FontFamily = Inter, FontSize = LabelSize });
        text.Inlines.Add(new Run(line.Figure) { FontFamily = Unbounded, FontWeight = FontWeights.Bold, FontSize = FigureSize });
        return text;
    }

    private static Canvas Rings(RingsModel model, TaskbarColours colours)
    {
        var canvas = new Canvas { Width = RingsSize, Height = RingsSize, VerticalAlignment = VerticalAlignment.Center };
        var outer = RingsSize / 2 - Stroke / 2;
        Ring(canvas, model.Outer, colours, outer);
        Ring(canvas, model.Inner, colours, outer - Stroke - RingGap);
        return canvas;
    }

    // The track as a full circle at the track alpha, then the arc clockwise from 12 o'clock.
    private static void Ring(Canvas canvas, Ring ring, TaskbarColours colours, double radius)
    {
        var centre = new Point(RingsSize / 2, RingsSize / 2);
        canvas.Children.Add(new Path
        {
            Data = new EllipseGeometry(centre, radius, radius),
            Stroke = Solid(colours.Ink, TaskbarColours.TrackAlpha),
            StrokeThickness = Stroke,
        });

        var sweep = ring.CappedSweep(radius, Stroke);
        if (sweep <= 0) return;
        var fill = Solid(colours.Fill(ring.Level));
        if (sweep >= 360)
        {
            canvas.Children.Add(new Path { Data = new EllipseGeometry(centre, radius, radius), Stroke = fill, StrokeThickness = Stroke });
            return;
        }

        var radians = sweep * Math.PI / 180;
        var end = new Point(centre.X + radius * Math.Sin(radians), centre.Y - radius * Math.Cos(radians));
        var figure = new PathFigure { StartPoint = new Point(centre.X, centre.Y - radius), IsClosed = false };
        figure.Segments.Add(new ArcSegment(end, new Size(radius, radius), 0, sweep > 180, SweepDirection.Clockwise, true));
        var geometry = new PathGeometry();
        geometry.Figures.Add(figure);
        canvas.Children.Add(new Path
        {
            Data = geometry,
            Stroke = fill,
            StrokeThickness = Stroke,
            StrokeStartLineCap = PenLineCap.Round,
            StrokeEndLineCap = PenLineCap.Round,
        });
    }

    private static Brush Solid(Rgb c, byte alpha = 0xFF) => new SolidColorBrush(Color.FromArgb(alpha, c.R, c.G, c.B));
}
