using System.Windows;
using System.Windows.Controls;
using System.Windows.Documents;
using System.Windows.Media;
using Sobreiro.Core.Display;
using HorizontalAlignment = System.Windows.HorizontalAlignment;
using Size = System.Windows.Size;

namespace Sobreiro.Flyout;

// The lines under each limit's meter or glyph: the status badge, the resets-in text and the spend
// detail, and the baseline alignment they and the figures share.
public static partial class FlyoutView
{
    // A badge's line box: with Inter's baseline at 0.8 of it, 16 px centres a 13 px capital in the pill.
    private const double BadgeLine = 16;
    private const double BadgePadY = 2;
    private const double BadgeToResets = 8;

    // Half the pill's height (10), not 999: WPF scales an oversized radius to half the width by half
    // the height, which draws an ellipse.
    private const double BadgeRadius = (BadgeLine + 2 * BadgePadY) / 2;

    /// <summary>A limit's badge, resets-in text and spend detail, as <see cref="Build"/> draws them.</summary>
    internal static FrameworkElement Lines(LimitView limit, FlyoutColours colours, bool column) =>
        Lines(limit, new Paint(colours), column);

    // In a bar row the badge and the resets-in text share a line, 8 px apart, on one baseline; when
    // the two are wider than the row, the resets-in text goes under the badge. In a column each is
    // centred on its own line, the resets-in text a size smaller. The spend detail
    // goes under them. With no reading there is no badge, only "No reading yet".
    private static StackPanel Lines(LimitView limit, Paint p, bool column)
    {
        var panel = new StackPanel();
        var badge = limit.HasData && limit.Status is { } status ? Badge(status, p) : null;
        var resets = limit.ResetsIn is { } text ? Text(text, column ? 13 : 14, p.Soft, wrap: false) : null;
        if (resets is not null) Typography.SetNumeralAlignment(resets, FontNumeralAlignment.Tabular);

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
            resets.Margin = new Thickness(BadgeToResets, 0, 0, 0);
            AlignBaselines((badge, badge.Padding.Top + Baseline((TextBlock)badge.Child)), (resets, Baseline(resets)));
            panel.Children.Add(Row(badge, resets));
        }
        else if (((FrameworkElement?)badge ?? resets) is { } only)
        {
            panel.Children.Add(only);
        }

        if (limit.Detail is { } detail)
        {
            var line = Text(detail, 14, p.Soft);
            Typography.SetNumeralAlignment(line, FontNumeralAlignment.Tabular);
            if (panel.Children.Count > 0) line.Margin = new Thickness(0, 4, 0, 0);
            if (column) Centre(line);
            panel.Children.Add(line);
        }
        return panel;
    }

    // Whether a bar row is wide enough for the badge, the gap and the resets-in text. A far Ends
    // badge beside "resets in 17d 10h" is not, and a row does not wrap or shrink by itself.
    private static bool FitOnOneLine(FrameworkElement badge, FrameworkElement resets)
    {
        var any = new Size(double.PositiveInfinity, double.PositiveInfinity);
        badge.Measure(any);
        resets.Measure(any);
        return badge.DesiredSize.Width + BadgeToResets + resets.DesiredSize.Width <= Inner;
    }

    // A filled pill: Inter Medium 13 px in the badge's text colour on its fill, no outline or shadow.
    private static Border Badge(LimitStatus status, Paint p)
    {
        var (fill, ink) = p.Badge(status.Kind);
        var text = Text(status.Text, 13, ink, weight: FontWeights.Medium, lineHeight: BadgeLine, wrap: false);
        Typography.SetNumeralAlignment(text, FontNumeralAlignment.Tabular);
        return new Border
        {
            Background = fill,
            CornerRadius = new CornerRadius(BadgeRadius),
            Padding = new Thickness(10, BadgePadY, 10, BadgePadY),
            HorizontalAlignment = HorizontalAlignment.Left,
            Child = text,
        };
    }

    // With BlockLineHeight, WPF puts a line's baseline at LineHeight × Baseline / LineSpacing of its
    // font. Each element moves down until its baseline meets the lowest one.
    private static double Baseline(TextBlock t) => t.LineHeight * t.FontFamily.Baseline / t.FontFamily.LineSpacing;

    private static void AlignBaselines(params TextBlock[] blocks) =>
        AlignBaselines(blocks.Select(b => ((FrameworkElement)b, Baseline(b))).ToArray());

    private static void AlignBaselines(params (FrameworkElement Element, double Baseline)[] items)
    {
        var lowest = items.Max(item => item.Baseline);
        foreach (var (element, baseline) in items)
        {
            element.VerticalAlignment = VerticalAlignment.Top;
            var margin = element.Margin;
            margin.Top += lowest - baseline;
            element.Margin = margin;
        }
    }
}
