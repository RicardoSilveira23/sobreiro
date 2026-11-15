using System.Windows;
using System.Windows.Controls;
using System.Windows.Documents;
using System.Windows.Media;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;
using Brush = System.Windows.Media.Brush;
using Color = System.Windows.Media.Color;
using FontFamily = System.Windows.Media.FontFamily;
using HorizontalAlignment = System.Windows.HorizontalAlignment;
using Orientation = System.Windows.Controls.Orientation;
using Path = System.Windows.Shapes.Path;
using Point = System.Windows.Point;
using Rectangle = System.Windows.Shapes.Rectangle;

namespace Sobreiro.Flyout;

/// <summary>
/// Draws a <see cref="FlyoutModel"/> as the Night card, in DIPs, to the measures of the confirmed
/// mockup. Everything it says and every colour choice comes from Core; this only lays it out.
/// </summary>
public static partial class FlyoutView
{
    private const double CardWidth = 330;
    private const double LineFactor = 1.45;
    private const double ColumnGap = 12;

    /// <summary>The card's width inside its border and padding: a bar row's width.</summary>
    internal const double Inner = CardWidth - 2 * 1 - 2 * 18;

    /// <summary>The width of a battery or thermometer column.</summary>
    internal const double ColumnWidth = (Inner - ColumnGap) / 2;

    private const double SoonOpacity = 0.35;

    private static readonly Uri Root = new("pack://application:,,,/");
    private static readonly FontFamily Inter = new(Root, "./Fonts/#Inter");
    private static readonly FontFamily Unbounded = new(Root, "./Fonts/#Unbounded");

    public static FrameworkElement Build(FlyoutModel model, FlyoutStyle style, FlyoutColours colours)
    {
        var p = new Paint(colours);
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
            if (model.Spend is { } spend)
                body.Children.Add(BarRow(spend, p));
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

    // Label left and figure right on one baseline, the meter with its tick, then the text lines.
    private static FrameworkElement BarRow(LimitView limit, Paint p, string? note = null)
    {
        var label = Text(limit.Label, 15, p.Ink, wrap: false);
        var (figure, unit) = Figure(limit.UsedFigure, "used", p);
        AlignBaselines(label, figure, unit);

        var head = new Grid();
        head.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        head.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        var right = Row(figure, unit);
        Grid.SetColumn(right, 1);
        head.Children.Add(label);
        head.Children.Add(right);

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

    // Battery and thermometer: session and weekly side by side, each centred in its column.
    private static FrameworkElement Columns(FlyoutModel model, FlyoutStyle style, Paint p)
    {
        var grid = new Grid();
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(ColumnGap) });
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        var session = Column(model.Session, style, p);
        var weekly = Column(model.Weekly, style, p);
        Grid.SetColumn(weekly, 2);
        grid.Children.Add(session);
        grid.Children.Add(weekly);
        return Ruled(grid, p.Line, new Thickness(0, 16, 0, 14));
    }

    private static FrameworkElement Column(LimitView limit, FlyoutStyle style, Paint p)
    {
        var glyph = style == FlyoutStyle.Battery ? Battery(limit, p) : Thermometer(limit, p);
        glyph.HorizontalAlignment = HorizontalAlignment.Center;

        var (figure, unit) = Figure(limit.LeftFigure, "left", p);
        AlignBaselines(figure, unit);
        var figures = Row(figure, unit);
        figures.HorizontalAlignment = HorizontalAlignment.Center;
        figures.Margin = new Thickness(0, 4, 0, 0);

        var label = Text(limit.Label, 15, p.Ink, weight: FontWeights.Medium);
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

    // 128 × 40: an outline and nub, ten cells. Lit cells are what is left now; the faint ones are
    // what the pace will use by the reset.
    private static FrameworkElement Battery(LimitView limit, Paint p)
    {
        var fill = GlyphFill.From(limit);
        var colour = p.Level(limit.Level);

        var canvas = new Canvas { Width = 128, Height = 40 };
        canvas.Children.Add(Shape(new RectangleGeometry(new Rect(1, 1, 117, 38), 5, 5), null, p.Soft, 1.5));
        canvas.Children.Add(Shape(new RectangleGeometry(new Rect(120, 13, 5, 14), 1.5, 1.5), p.Soft));
        for (var i = 0; i < GlyphFill.Cells; i++)
        {
            var cell = new RectangleGeometry(new Rect(5 + 11 * i, 5, 8, 30), 1.5, 1.5);
            canvas.Children.Add(i < fill.LeftCells - fill.SoonCells ? Shape(cell, colour)
                : i < fill.LeftCells ? Shape(cell, colour, opacity: SoonOpacity)
                : Shape(cell, p.Track));
        }
        return canvas;
    }

    // 30 × 104: tube, the faint share the pace will use, the solid rest, the bulb; a 100% / 0% scale beside it.
    private static FrameworkElement Thermometer(LimitView limit, Paint p)
    {
        const double top = 4, height = 80, bottom = 90;
        var fill = GlyphFill.From(limit);
        var yLeft = top + height * (1 - fill.LeftPercent / 100);
        var ySoon = yLeft + height * fill.SoonPercent / 100;
        var colour = limit.HasData ? p.Level(limit.Level) : p.Track;

        var glyph = new Canvas { Width = 30, Height = 104 };
        glyph.Children.Add(Shape(new RectangleGeometry(new Rect(10, top, 10, 86), 5, 5), p.Track));
        glyph.Children.Add(Shape(new RectangleGeometry(new Rect(10, yLeft, 10, ySoon - yLeft)), colour, opacity: SoonOpacity));
        glyph.Children.Add(Shape(new RectangleGeometry(new Rect(10, ySoon, 10, bottom - ySoon)), colour));
        glyph.Children.Add(Shape(new EllipseGeometry(new Point(15, 93), 9, 9), colour));

        var full = Text("100%", 11, p.Soft, wrap: false);
        var empty = Text("0%", 11, p.Soft, wrap: false);
        empty.VerticalAlignment = VerticalAlignment.Bottom;
        var scale = new Grid { Height = 88, Margin = new Thickness(10, 0, 0, 0), VerticalAlignment = VerticalAlignment.Center };
        scale.Children.Add(full);
        scale.Children.Add(empty);

        return Row(glyph, scale);
    }

    // The figure in Unbounded and its unit in Inter, 4 px apart.
    private static (TextBlock Figure, TextBlock Unit) Figure(string figure, string unit, Paint p)
    {
        var big = Text(figure, 22, p.Ink, Unbounded, FontWeights.Bold, lineHeight: 22, wrap: false);
        Typography.SetNumeralAlignment(big, FontNumeralAlignment.Tabular);
        var small = Text(unit, 14, p.Soft, wrap: false);
        small.Margin = new Thickness(4, 0, 0, 0);
        return (big, small);
    }

    private static TextBlock Text(string text, double size, Brush brush, FontFamily? family = null,
        FontWeight? weight = null, double lineHeight = 0, bool wrap = true) => new()
    {
        Text = text,
        FontFamily = family ?? Inter,
        FontSize = size,
        FontWeight = weight ?? FontWeights.Normal,
        Foreground = brush,
        LineHeight = lineHeight > 0 ? lineHeight : size * LineFactor,
        LineStackingStrategy = LineStackingStrategy.BlockLineHeight,
        TextWrapping = wrap ? TextWrapping.Wrap : TextWrapping.NoWrap,
    };

    private static StackPanel Row(params UIElement[] children)
    {
        var row = new StackPanel { Orientation = Orientation.Horizontal };
        foreach (var child in children) row.Children.Add(child);
        return row;
    }

    private static void Centre(TextBlock block)
    {
        block.TextAlignment = TextAlignment.Center;
        block.HorizontalAlignment = HorizontalAlignment.Center;
    }

    // A hairline under the content: Ink under the title, Line under each limit.
    private static Border Ruled(UIElement child, Brush line, Thickness padding) => new()
    {
        BorderBrush = line,
        BorderThickness = new Thickness(0, 0, 0, 1),
        Padding = padding,
        Child = child,
    };

    private static Path Shape(Geometry geometry, Brush? fill, Brush? stroke = null, double thickness = 0, double opacity = 1) => new()
    {
        Data = geometry,
        Fill = fill,
        Stroke = stroke,
        StrokeThickness = thickness,
        Opacity = opacity,
    };

    private sealed class Paint(FlyoutColours colours)
    {
        public Brush Ground { get; } = Solid(colours.Palette.Ground);
        public Brush Ink { get; } = Solid(colours.Palette.Ink);
        public Brush Soft { get; } = Solid(colours.Palette.Soft);
        public Brush Line { get; } = Solid(colours.Palette.Line);
        public Brush Track { get; } = Solid(colours.Palette.Track);

        private readonly Dictionary<UsageLevel, Brush> _levels = Enum.GetValues<UsageLevel>()
            .ToDictionary(level => level, level => Solid(colours.Palette.LevelColour(level, colours.Fill)));

        private readonly Dictionary<StatusKind, (Brush Fill, Brush Text)> _badges = Enum.GetValues<StatusKind>()
            .ToDictionary(kind => kind, kind =>
            {
                var badge = colours.Palette.Badge(kind);
                return (Solid(badge.Fill), Solid(badge.Text));
            });

        /// <summary>The fill below 70%, amber from 70%, red from 90%.</summary>
        public Brush Level(UsageLevel level) => _levels[level];

        /// <summary>A status badge's fill and text, from <see cref="NightPalette.Badge"/>.</summary>
        public (Brush Fill, Brush Text) Badge(StatusKind kind) => _badges[kind];

        private static Brush Solid(Rgb c)
        {
            var brush = new SolidColorBrush(Color.FromRgb(c.R, c.G, c.B));
            brush.Freeze();
            return brush;
        }
    }
}
