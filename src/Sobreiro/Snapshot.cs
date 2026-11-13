using System.Globalization;
using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Copilot;
using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Live;
using Sobreiro.Flyout;
using Sobreiro.Pill;
using Brush = System.Windows.Media.Brush;
using Color = System.Windows.Media.Color;
using HorizontalAlignment = System.Windows.HorizontalAlignment;
using Image = System.Windows.Controls.Image;
using Orientation = System.Windows.Controls.Orientation;
using Size = System.Windows.Size;
using Sobreiro.Core.Polling;

namespace Sobreiro;

/// <summary>
/// <c>Sobreiro.exe --snapshot file.png</c>: one sheet with every flyout style in both themes and
/// both bar colours, from a sample reading; every status badge in both themes; and the tray icon at
/// each level and size. The accent columns carry Live usage's longest footer sentence.
/// </summary>
internal static class Snapshot
{
    private static readonly (bool Dark, BarColourOption Option, ProviderState Live)[] Columns =
    [
        (true, BarColourOption.Purple, ProviderState.Off), (true, BarColourOption.Accent, ProviderState.Refused),
        (false, BarColourOption.Purple, ProviderState.Off), (false, BarColourOption.Accent, ProviderState.Refused),
    ];

    // (Session, Weekly) readings; null for no data.
    private static readonly (int Session, int Weekly)?[] IconReadings = [null, (8, 44), (27, 74), (74, 27), (93, 61), (100, 100)];
    private static readonly (int Session, int Weekly)?[] PillReadings = [null, (37, 61), (74, 61), (93, 100)];
    private static readonly int[] IconSizes = [16, 20, 24, 32];
    private const int Zoom = 4;

    /// <summary>Renders the sheet to <paramref name="pngPath"/>; 0 on success, 1 on failure.</summary>
    public static int Write(string pngPath)
    {
        try
        {
            var sheet = Sheet(DateTimeOffset.Now);
            sheet.Measure(new Size(double.PositiveInfinity, double.PositiveInfinity));
            sheet.Arrange(new Rect(sheet.DesiredSize));
            sheet.UpdateLayout();

            var bitmap = new RenderTargetBitmap(
                (int)Math.Ceiling(sheet.ActualWidth), (int)Math.Ceiling(sheet.ActualHeight), 96, 96, PixelFormats.Pbgra32);
            bitmap.Render(sheet);
            var encoder = new PngBitmapEncoder();
            encoder.Frames.Add(BitmapFrame.Create(bitmap));
            // Encode in memory first, so a failed encode leaves no half-written file.
            using var png = new MemoryStream();
            encoder.Save(png);
            File.WriteAllBytes(pngPath, png.ToArray());
            return 0;
        }
        catch (Exception ex)
        {
            return ConsoleError.Fail($"Snapshot failed: {ex.GetType().Name}: {ex.Message}", 1);
        }
    }

    /// <summary>The wrong arguments after --snapshot: prints how to call it; 2.</summary>
    public static int Usage() => ConsoleError.Fail("Usage: Sobreiro.exe --snapshot <file.png>", 2);

    private static FrameworkElement Sheet(DateTimeOffset now)
    {
        var limits = SampleLimits(now);
        var accent = WindowsTheme.Accent() ?? AccentPalette.WindowsBlue;
        FlyoutStyle[] styles = [FlyoutStyle.Bars, FlyoutStyle.Battery, FlyoutStyle.Thermometer];

        var grid = new Grid { UseLayoutRounding = true };
        foreach (var _ in Columns) grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        for (var row = 0; row < styles.Length + 3; row++) grid.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });

        for (var row = 0; row < styles.Length; row++)
        {
            for (var col = 0; col < Columns.Length; col++)
            {
                var (dark, option, live) = Columns[col];
                var model = FlyoutModel.From(limits, now, TimeZoneInfo.Local, CultureInfo.CurrentCulture, live);
                var card = FlyoutView.Build(model, styles[row], FlyoutColours.For(dark, option, accent));
                card.VerticalAlignment = VerticalAlignment.Top;
                var cell = Cell(dark, card);
                Grid.SetRow(cell, row);
                Grid.SetColumn(cell, col);
                grid.Children.Add(cell);
            }
        }

        // The badge row reads at 10:00 today, so "Ends today" and "Ends another day" keep their
        // wording whatever time the sheet is made.
        var morning = new DateTimeOffset(DateTime.Today.AddHours(10));
        foreach (var dark in new[] { true, false })
        {
            var badges = Cell(dark, Badges(morning, dark));
            Grid.SetRow(badges, styles.Length);
            Grid.SetColumn(badges, dark ? 0 : 2);
            Grid.SetColumnSpan(badges, 2);
            grid.Children.Add(badges);
        }

        var icons = new StackPanel();
        icons.Children.Add(IconStrip(now, light: false));
        icons.Children.Add(IconStrip(now, light: true));
        icons.Children.Add(PillStrip(now, light: false));
        icons.Children.Add(PillStrip(now, light: true));
        Grid.SetRow(icons, styles.Length + 1);
        Grid.SetColumnSpan(icons, Columns.Length);
        grid.Children.Add(icons);

        var copilot = CopilotRow(now, accent);
        Grid.SetRow(copilot, styles.Length + 2);
        Grid.SetColumnSpan(copilot, Columns.Length);
        grid.Children.Add(copilot);
        return grid;
    }

    // The Copilot row: both-services and Copilot-only pills, both flyouts (Bars) and the one-ring icon, in each theme.
    private static FrameworkElement CopilotRow(DateTimeOffset now, AccentPalette? accent)
    {
        var credits = LimitList.FromCopilot(CopilotSample(now), now);
        var both = SampleLimits(now).Concat(credits).ToList();
        var stack = new StackPanel();

        var flyouts = new StackPanel { Orientation = Orientation.Horizontal };
        foreach (var dark in new[] { true, false })
        {
            var model = FlyoutModel.From(both, now, TimeZoneInfo.Local, CultureInfo.CurrentCulture, ProviderState.Off);
            var card = FlyoutView.Build(model, FlyoutStyle.Bars, FlyoutColours.For(dark, BarColourOption.Purple, accent));
            card.VerticalAlignment = VerticalAlignment.Top;
            flyouts.Children.Add(Cell(dark, card));
        }
        var only = FlyoutModel.From(credits, now, TimeZoneInfo.Local, CultureInfo.CurrentCulture, ProviderState.Off,
            copilotNote: "From the Copilot CLI, as of 09:25.", showClaude: false);
        var onlyCard = FlyoutView.Build(only, FlyoutStyle.Bars, FlyoutColours.For(true, BarColourOption.Purple, accent));
        onlyCard.VerticalAlignment = VerticalAlignment.Top;
        flyouts.Children.Add(Cell(true, onlyCard));
        stack.Children.Add(flyouts);

        foreach (var light in new[] { false, true })
        {
            var colours = TaskbarColours.For(light, false, BarColourOption.Purple, null);
            var pills = new StackPanel { Orientation = Orientation.Horizontal };
            foreach (var (limits, showClaude) in new[] { (both, true), (credits.ToList(), false) })
            {
                var pill = PillView.Build(PillModel.From(limits, TimeZoneInfo.Local, showClaude), colours);
                pill.Margin = new Thickness(0, 0, 24, 0);
                pills.Children.Add(pill);
            }
            foreach (var size in IconSizes)
            {
                var pair = new StackPanel { Orientation = Orientation.Horizontal, Margin = new Thickness(0, 0, 24, 0) };
                var model = RingsModel.Copilot(credits);
                pair.Children.Add(Pixels(model, light, size, 1));
                var zoomed = Pixels(model, light, size, Zoom);
                zoomed.Margin = new Thickness(8, 0, 0, 0);
                pair.Children.Add(zoomed);
                pills.Children.Add(pair);
            }
            stack.Children.Add(new Border { Background = Solid(light ? "#F0F0F0" : "#1C1C1C"), Padding = new Thickness(16, 8, 16, 8), Child = pills });
        }
        return stack;
    }

    // 2,644 of 15,000 AI credits, resetting on the 1st of next month.
    private static CopilotReading CopilotSample(DateTimeOffset now)
    {
        var next = now.AddMonths(1);
        return new(15000, 2644, new DateTimeOffset(next.Year, next.Month, 1, 0, 0, 0, now.Offset), now.AddMinutes(-3), CopilotSource.Live);
    }

    // Session 27% resetting in 191 min, weekly 74% resetting in 85 h, read 3 min ago.
    private static IReadOnlyList<Limit> SampleLimits(DateTimeOffset now)
    {
        var observed = now.AddMinutes(-3);
        return
        [
            new Limit("Claude Code", LimitKind.Session, "Session", 27, now.AddMinutes(191), observed, false, TimeSpan.FromHours(5)),
            new Limit("Claude Code", LimitKind.Weekly, "Weekly", 74, now.AddHours(85), observed, false, TimeSpan.FromDays(7)),
        ];
    }

    // A card per theme listing every badge: on the left as a bar row draws it, on the right as a
    // column does, between hairlines at the column's width. Spend is only ever a bar row.
    private static FrameworkElement Badges(DateTimeOffset now, bool dark)
    {
        var colours = FlyoutColours.For(dark, BarColourOption.Purple, null);
        var palette = colours.Palette;
        var list = new StackPanel();
        foreach (var view in BadgeViews(now))
        {
            var line = new Grid { Margin = new Thickness(0, 0, 0, 12) };
            line.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(FlyoutView.Inner) });
            line.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(24) });
            line.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            line.Children.Add(FlyoutView.Lines(view, colours, column: false));
            if (view.Kind != LimitKind.Spend)
            {
                var column = new Border
                {
                    Width = FlyoutView.ColumnWidth + 2,
                    BorderBrush = Solid(palette.Line),
                    BorderThickness = new Thickness(1, 0, 1, 0),
                    Child = FlyoutView.Lines(view, colours, column: true),
                };
                Grid.SetColumn(column, 2);
                line.Children.Add(column);
            }
            list.Children.Add(line);
        }
        return new Border
        {
            Background = Solid(palette.Ground),
            BorderBrush = Solid(palette.Line),
            BorderThickness = new Thickness(1),
            CornerRadius = new CornerRadius(10),
            Padding = new Thickness(18, 18, 18, 6),
            HorizontalAlignment = HorizontalAlignment.Left,
            UseLayoutRounding = true,
            Child = list,
        };
    }

    // One limit per badge kind and wording, an Ends in en-US (the widest), spend, and no reading.
    private static IEnumerable<LimitView> BadgeViews(DateTimeOffset now)
    {
        var culture = CultureInfo.CurrentCulture;
        var us = CultureInfo.GetCultureInfo("en-US");
        var observed = now.AddMinutes(-3);
        var window = TimeSpan.FromHours(5);
        Limit Session(double percent, TimeSpan resetsIn, DateTimeOffset? seen = null) =>
            new("Claude Code", LimitKind.Session, "Session", percent, now + resetsIn, seen ?? observed, false, window);
        var weekly = new Limit("Claude Code", LimitKind.Weekly, "Weekly", 74, now.AddHours(85), observed, false, TimeSpan.FromDays(7));

        yield return View(Session(27, TimeSpan.FromMinutes(191)), now, culture);                       // On track
        yield return View(Session(60, TimeSpan.FromMinutes(150)), now, culture);                       // Ends today
        yield return View(weekly, now, culture);                                                       // Ends another day
        yield return View(weekly, now, us);                                                            // Ends, 12-hour clock
        yield return View(Session(95, TimeSpan.FromMinutes(60), now.AddMinutes(-90)), now, culture);   // Likely used up
        yield return View(Session(100, TimeSpan.FromMinutes(42)), now, culture);                       // Used up
        yield return View(Session(1, TimeSpan.FromHours(4)), now, culture);                            // Not used yet
        yield return View(Session(10, TimeSpan.FromMinutes(295), now.AddMinutes(-1)), now, culture);   // Too early
        yield return View(Session(80, TimeSpan.FromSeconds(20), now.AddMinutes(-1)), now, culture);    // Resetting
        yield return View(new Limit("Claude Code", LimitKind.Spend, "Spend", 25, now.AddDays(12), observed, false,
            UsedUsd: 12.5, LimitUsd: 50, Period: "monthly"), now, culture);                             // Spend: no badge
        yield return FlyoutModel.From([], now, TimeZoneInfo.Local, culture).Session;                    // No reading
    }

    private static LimitView View(Limit limit, DateTimeOffset now, CultureInfo culture)
    {
        var model = FlyoutModel.From([limit], now, TimeZoneInfo.Local, culture);
        return limit.Kind switch
        {
            LimitKind.Weekly => model.Weekly,
            LimitKind.Spend => model.Spend ?? throw new InvalidOperationException("The spend sample gave no spend row."),
            LimitKind.Credits => model.Copilot?.Credits ?? throw new InvalidOperationException("The credits sample gave no Copilot row."),
            _ => model.Session,
        };
    }

    // A sheet cell: a desktop-like ground behind the card, so its edge shows.
    private static Border Cell(bool dark, FrameworkElement card) => new()
    {
        Background = Solid(dark ? "#2B3542" : "#C9D3DE"),
        Padding = new Thickness(16),
        Child = card,
    };

    // One row per size; each reading at 1× and at ×4 (nearest neighbour) on a taskbar-coloured strip.
    private static FrameworkElement IconStrip(DateTimeOffset now, bool light)
    {
        var rows = new StackPanel();
        foreach (var size in IconSizes)
        {
            var row = new StackPanel { Orientation = Orientation.Horizontal, Margin = new Thickness(0, 0, 0, 12) };
            foreach (var reading in IconReadings)
            {
                var model = RingsModel.From(reading is var (session, weekly)
                    ? [Reading(LimitKind.Session, session, now), Reading(LimitKind.Weekly, weekly, now)]
                    : []);
                var pair = new StackPanel { Orientation = Orientation.Horizontal, Margin = new Thickness(0, 0, 24, 0) };
                pair.Children.Add(Pixels(model, light, size, 1));
                var zoomed = Pixels(model, light, size, Zoom);
                zoomed.Margin = new Thickness(8, 0, 0, 0);
                pair.Children.Add(zoomed);
                row.Children.Add(pair);
            }
            rows.Children.Add(row);
        }
        return new Border
        {
            Background = Solid(light ? "#F0F0F0" : "#1C1C1C"),
            Padding = new Thickness(16, 16, 16, 4),
            Child = rows,
        };
    }

    // The taskbar pill with no data and three readings, then with numbers that have stopped, on a taskbar-coloured strip.
    private static FrameworkElement PillStrip(DateTimeOffset now, bool light)
    {
        var colours = TaskbarColours.For(light, false, BarColourOption.Purple, null);
        var row = new StackPanel { Orientation = Orientation.Horizontal };
        foreach (var reading in PillReadings)
        {
            var limits = reading is var (session, weekly)
                ? new[] { Reading(LimitKind.Session, session, now), Reading(LimitKind.Weekly, weekly, now) }
                : [];
            var pill = PillView.Build(PillModel.From(limits), colours);
            pill.Margin = new Thickness(0, 0, 24, 0);
            row.Children.Add(pill);
        }

        // Second row, numbers that have stopped: Claude's block; both blocks; Copilot's beside a fresh
        // Claude block; both figures at 100%, the widest; and a block with no figures to keep.
        var stopped = new StackPanel { Orientation = Orientation.Horizontal, Margin = new Thickness(0, 8, 0, 0) };
        var old = now.AddDays(-3);
        var oldReadings = new[] { Reading(LimitKind.Session, 37, old), Reading(LimitKind.Weekly, 78, old) };
        var oldCredits = LimitList.FromCopilot(CopilotSample(now) with { ObservedAt = old }, now);
        var fresh = new[] { Reading(LimitKind.Session, 37, now), Reading(LimitKind.Weekly, 78, now) };
        var full = new[] { Reading(LimitKind.Session, 100, old), Reading(LimitKind.Weekly, 100, old) };
        stopped.Children.Add(StoppedPill(oldReadings, ProviderState.Expired, ProviderState.Ok, colours, now));
        stopped.Children.Add(StoppedPill(oldReadings.Concat(oldCredits).ToList(), ProviderState.Refused, ProviderState.Offline, colours, now));
        stopped.Children.Add(StoppedPill(fresh.Concat(oldCredits).ToList(), ProviderState.Ok, ProviderState.NoSignIn, colours, now));
        stopped.Children.Add(StoppedPill(full, ProviderState.BadAnswer, ProviderState.Ok, colours, now));
        stopped.Children.Add(StoppedPill([], ProviderState.Expired, ProviderState.Ok, colours, now));

        var rows = new StackPanel();
        rows.Children.Add(row);
        rows.Children.Add(stopped);
        return new Border { Background = Solid(light ? "#F0F0F0" : "#1C1C1C"), Padding = new Thickness(16, 8, 16, 8), Child = rows };
    }

    private static FrameworkElement StoppedPill(
        IReadOnlyList<Limit> limits, ProviderState live, ProviderState copilot, TaskbarColours colours, DateTimeOffset now)
    {
        var pill = PillView.Build(PillModel.From(limits, TimeZoneInfo.Local, true, live, PillStale.MissesNeeded, copilot, PillStale.MissesNeeded, now), colours);
        pill.Margin = new Thickness(0, 0, 24, 0);
        return pill;
    }

    private static Limit Reading(LimitKind kind, int percent, DateTimeOffset now) =>
        new("Claude Code", kind, kind.ToString(), percent, now.AddHours(1), now, false);

    // The renderer's bitmap, each pixel repeated scale x scale times.
    private static Image Pixels(RingsModel model, bool light, int size, int scale) => new()
    {
        Source = IconBitmaps.Source(model, TaskbarColours.For(light, false, BarColourOption.Purple, null), size, scale),
        Width = size * scale,
        Height = size * scale,
        Stretch = Stretch.None,
        VerticalAlignment = VerticalAlignment.Center,
    };

    private static Brush Solid(string hex) => Solid(Rgb.Parse(hex));

    private static Brush Solid(Rgb c) => new SolidColorBrush(Color.FromRgb(c.R, c.G, c.B));
}
