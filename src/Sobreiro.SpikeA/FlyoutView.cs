using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Documents;
using Avalonia.Controls.Shapes;
using Avalonia.Layout;
using Avalonia.Media;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;

namespace Sobreiro.SpikeA;

/// <summary>
/// SPIKE (shell candidate A): the flyout card in the toolkit's controls, to the WPF card's measures. Not
/// polished: a figure and its unit share one text block instead of the WPF card's hand-set baselines.
/// </summary>
internal static class FlyoutView
{
    private const double CardWidth = 330;
    private const double LineFactor = 1.45;
    private const double ColumnGap = 12;
    private const double Inner = CardWidth - 2 * 1 - 2 * 18;
    private const double SoonOpacity = 0.35;
    private static readonly FontFeatureCollection Tabular = [FontFeature.Parse("tnum")];

    public static Control Build(FlyoutModel model, FlyoutStyle style, NightPalette palette, Rgb fill)
    {
        var p = new Paint(palette, fill);
        var body = new StackPanel();
        if (model.ShowClaude)
        {
            body.Children.Add(Ruled(Text(model.Title, 15, p.Soft), p.Ink, new Thickness(0, 0, 0, 12)));
            if (style == FlyoutStyle.Bars)
            {
                body.Children.Add(BarRow(model.Session, p));
                body.Children.Add(BarRow(model.Weekly, p));
            }
            else
            {
                body.Children.Add(Columns(model, style, p));
            }
            if (model.Spend is { } spend) body.Children.Add(BarRow(spend, p));
        }
        if (model.Copilot is { } copilot)
        {
            var heading = Ruled(Text("GitHub Copilot", 15, p.Soft), p.Ink, new Thickness(0, 0, 0, 12));
            if (model.ShowClaude) heading.Margin = new Thickness(0, 10, 0, 0);
            body.Children.Add(heading);
            body.Children.Add(BarRow(copilot.Credits, p, copilot.Note));
        }

        var footer = Text(model.Footer, 14, p.Soft);
        footer.Margin = new Thickness(0, 12, 0, 0);
        body.Children.Add(footer);

        return new Border
        {
            Width = CardWidth,
            Background = p.Ground,
            BorderBrush = p.Line,
            BorderThickness = new Thickness(1),
            CornerRadius = new CornerRadius(10),
            Padding = new Thickness(18, 18, 18, 14),
            UseLayoutRounding = true,
            Child = body,
        };
    }

    // Label left and figure right, the meter with its tick, then the text lines.
    private static Control BarRow(LimitView limit, Paint p, string? note = null)
    {
        var label = Text(limit.Label, 15, p.Ink, wrap: false);
        label.VerticalAlignment = VerticalAlignment.Bottom;
        var figure = Figure(limit.UsedFigure, "used", p);
        figure.VerticalAlignment = VerticalAlignment.Bottom;
        Grid.SetColumn(figure, 1);
        var head = new Grid { ColumnDefinitions = new ColumnDefinitions("*,Auto") };
        head.Children.Add(label);
        head.Children.Add(figure);

        var meter = new Canvas { Width = Inner, Height = 6, Background = p.Track, Margin = new Thickness(0, 6, 0, 0) };
        meter.Children.Add(new Rectangle { Width = Inner * limit.UsedFraction, Height = 6, Fill = p.Level(limit.Level) });
        if (limit.ProjectedFraction is double projected && projected < 1)
        {
            var tick = new Rectangle { Width = 2, Height = 12, Fill = p.Ink };
            Canvas.SetLeft(tick, Inner * projected - 1);
            Canvas.SetTop(tick, -3);
            meter.Children.Add(tick);
        }

        var row = new StackPanel();
        row.Children.Add(head);
        row.Children.Add(meter);
        var lines = Lines(limit, p, column: false);
        lines.Margin = new Thickness(0, 10, 0, 0);
        row.Children.Add(lines);
        if (note is not null)
        {
            var noteText = Text(note, 14, p.Soft);
            noteText.Margin = new Thickness(0, 4, 0, 0);
            row.Children.Add(noteText);
        }
        return Ruled(row, p.Line, new Thickness(0, 14, 0, 14));
    }

    private static Control Columns(FlyoutModel model, FlyoutStyle style, Paint p)
    {
        var grid = new Grid { ColumnDefinitions = new ColumnDefinitions($"*,{ColumnGap},*") };
        var session = Column(model.Session, style, p);
        var weekly = Column(model.Weekly, style, p);
        Grid.SetColumn(weekly, 2);
        grid.Children.Add(session);
        grid.Children.Add(weekly);
        return Ruled(grid, p.Line, new Thickness(0, 16, 0, 14));
    }

    private static Control Column(LimitView limit, FlyoutStyle style, Paint p)
    {
        var glyph = style == FlyoutStyle.Battery ? Battery(limit, p) : Thermometer(limit, p);
        glyph.HorizontalAlignment = HorizontalAlignment.Center;

        var figures = Figure(limit.LeftFigure, "left", p);
        figures.HorizontalAlignment = HorizontalAlignment.Center;
        figures.Margin = new Thickness(0, 4, 0, 0);

        var label = Text(limit.Label, 15, p.Ink, weight: FontWeight.Medium);
        label.Margin = new Thickness(0, 8, 0, 0);
        Centre(label);

        var column = new StackPanel();
        column.Children.Add(glyph);
        column.Children.Add(figures);
        column.Children.Add(label);
        var lines = Lines(limit, p, column: true);
        lines.Margin = new Thickness(0, 6, 0, 0);
        column.Children.Add(lines);
        return column;
    }

    // 128 x 40: an outline and nub, ten cells.
    private static Control Battery(LimitView limit, Paint p)
    {
        var fill = GlyphFill.From(limit);
        var colour = p.Level(limit.Level);
        var canvas = new Canvas { Width = 128, Height = 40 };
        canvas.Children.Add(Box(1, 1, 117, 38, 5, null, p.Soft, 1.5));
        canvas.Children.Add(Box(120, 13, 5, 14, 1.5, p.Soft));
        for (var i = 0; i < GlyphFill.Cells; i++)
        {
            var cell = i < fill.LeftCells - fill.SoonCells ? Box(5 + 11 * i, 5, 8, 30, 1.5, colour)
                : i < fill.LeftCells ? Box(5 + 11 * i, 5, 8, 30, 1.5, colour, opacity: SoonOpacity)
                : Box(5 + 11 * i, 5, 8, 30, 1.5, p.Track);
            canvas.Children.Add(cell);
        }
        return canvas;
    }

    // 30 x 104: tube, the faint share the pace will use, the solid rest, the bulb; a scale beside it.
    private static Control Thermometer(LimitView limit, Paint p)
    {
        const double top = 4, height = 80, bottom = 90;
        var fill = GlyphFill.From(limit);
        var yLeft = top + height * (1 - fill.LeftPercent / 100);
        var ySoon = yLeft + height * fill.SoonPercent / 100;
        var colour = limit.HasData ? p.Level(limit.Level) : p.Track;

        var glyph = new Canvas { Width = 30, Height = 104 };
        glyph.Children.Add(Box(10, top, 10, 86, 5, p.Track));
        glyph.Children.Add(Box(10, yLeft, 10, ySoon - yLeft, 0, colour, opacity: SoonOpacity));
        glyph.Children.Add(Box(10, ySoon, 10, bottom - ySoon, 0, colour));
        glyph.Children.Add(Box(6, 84, 18, 18, 9, colour));

        var full = Text("100%", 11, p.Soft, wrap: false);
        var empty = Text("0%", 11, p.Soft, wrap: false);
        empty.VerticalAlignment = VerticalAlignment.Bottom;
        var scale = new Grid { Height = 88, Margin = new Thickness(10, 0, 0, 0), VerticalAlignment = VerticalAlignment.Center };
        scale.Children.Add(full);
        scale.Children.Add(empty);

        var row = new StackPanel { Orientation = Orientation.Horizontal };
        row.Children.Add(glyph);
        row.Children.Add(scale);
        return row;
    }

    // The badge, the resets-in text and the spend detail under a meter or a glyph.
    private static StackPanel Lines(LimitView limit, Paint p, bool column)
    {
        var panel = new StackPanel();
        var badge = limit.HasData && limit.Status is { } status ? Badge(status, p) : null;
        var resets = limit.ResetsIn is { } text ? Text(text, column ? 13 : 14, p.Soft, wrap: false) : null;
        if (resets is not null) resets.FontFeatures = Tabular;

        if (column)
        {
            if (badge is not null)
            {
                badge.HorizontalAlignment = HorizontalAlignment.Center;
                panel.Children.Add(badge);
            }
            if (resets is not null)
            {
                Centre(resets);
                if (badge is not null) resets.Margin = new Thickness(0, 4, 0, 0);
                panel.Children.Add(resets);
            }
        }
        else if (badge is not null && resets is not null && !FitOnOneLine(badge, resets))
        {
            resets.Margin = new Thickness(0, 4, 0, 0);
            panel.Children.Add(badge);
            panel.Children.Add(resets);
        }
        else if (badge is not null && resets is not null)
        {
            resets.Margin = new Thickness(8, 0, 0, 0);
            resets.VerticalAlignment = VerticalAlignment.Center;
            var row = new StackPanel { Orientation = Orientation.Horizontal };
            row.Children.Add(badge);
            row.Children.Add(resets);
            panel.Children.Add(row);
        }
        else if (((Control?)badge ?? resets) is { } only)
        {
            panel.Children.Add(only);
        }

        if (limit.Detail is { } detail)
        {
            var line = Text(detail, 14, p.Soft);
            line.FontFeatures = Tabular;
            if (panel.Children.Count > 0) line.Margin = new Thickness(0, 4, 0, 0);
            if (column) Centre(line);
            panel.Children.Add(line);
        }
        return panel;
    }

    private static bool FitOnOneLine(Control badge, Control resets)
    {
        badge.Measure(Size.Infinity);
        resets.Measure(Size.Infinity);
        return badge.DesiredSize.Width + 8 + resets.DesiredSize.Width <= Inner;
    }

    private static Border Badge(LimitStatus status, Paint p)
    {
        var (fill, ink) = p.Badge(status.Kind);
        var text = Text(status.Text, 13, ink, weight: FontWeight.Medium, lineHeight: 16, wrap: false);
        text.FontFeatures = Tabular;
        return new Border
        {
            Background = fill,
            CornerRadius = new CornerRadius(10),
            Padding = new Thickness(10, 2, 10, 2),
            HorizontalAlignment = HorizontalAlignment.Left,
            Child = text,
        };
    }

    // The figure in Unbounded and its unit in Inter, on one baseline because they are one line of text.
    private static TextBlock Figure(string figure, string unit, Paint p)
    {
        var block = new TextBlock { FontFeatures = Tabular, TextWrapping = TextWrapping.NoWrap };
        block.Inlines!.Add(new Run(figure) { FontFamily = PillView.Unbounded, FontWeight = FontWeight.Bold, FontSize = 22, Foreground = p.Ink });
        block.Inlines.Add(new Run(" " + unit) { FontFamily = PillView.Inter, FontSize = 14, Foreground = p.Soft });
        return block;
    }

    private static TextBlock Text(string text, double size, IBrush brush, FontWeight weight = FontWeight.Normal,
        double lineHeight = 0, bool wrap = true) => new()
    {
        Text = text,
        FontFamily = PillView.Inter,
        FontSize = size,
        FontWeight = weight,
        Foreground = brush,
        LineHeight = lineHeight > 0 ? lineHeight : size * LineFactor,
        TextWrapping = wrap ? TextWrapping.Wrap : TextWrapping.NoWrap,
    };

    private static void Centre(TextBlock block)
    {
        block.TextAlignment = TextAlignment.Center;
        block.HorizontalAlignment = HorizontalAlignment.Center;
    }

    private static Border Ruled(Control child, IBrush line, Thickness padding) => new()
    {
        BorderBrush = line,
        BorderThickness = new Thickness(0, 0, 0, 1),
        Padding = padding,
        Child = child,
    };

    private static Rectangle Box(double x, double y, double width, double height, double radius, IBrush? fill,
        IBrush? stroke = null, double thickness = 0, double opacity = 1)
    {
        var box = new Rectangle
        {
            Width = width, Height = Math.Max(0, height), RadiusX = radius, RadiusY = radius,
            Fill = fill, Stroke = stroke, StrokeThickness = thickness, Opacity = opacity,
        };
        Canvas.SetLeft(box, x);
        Canvas.SetTop(box, y);
        return box;
    }

    private sealed class Paint(NightPalette palette, Rgb fill)
    {
        public IBrush Ground { get; } = PillView.Solid(palette.Ground);
        public IBrush Ink { get; } = PillView.Solid(palette.Ink);
        public IBrush Soft { get; } = PillView.Solid(palette.Soft);
        public IBrush Line { get; } = PillView.Solid(palette.Line);
        public IBrush Track { get; } = PillView.Solid(palette.Track);

        public IBrush Level(UsageLevel level) => PillView.Solid(palette.LevelColour(level, fill));

        public (IBrush Fill, IBrush Text) Badge(StatusKind kind)
        {
            var badge = palette.Badge(kind);
            return (PillView.Solid(badge.Fill), PillView.Solid(badge.Text));
        }
    }
}
