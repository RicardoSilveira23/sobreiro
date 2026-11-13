using System.Globalization;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Media.Imaging;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Config;
using Sobreiro.Core.Copilot;
using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Polling;

namespace Sobreiro.SpikeA;

/// <summary>
/// SPIKE (shell candidate A): <c>--snapshot folder</c> renders the pill at 100%, 150% and 200% in both
/// taskbar themes, and the flyout, to pictures, from made-up figures. No tray, no network.
/// </summary>
internal static class Snapshot
{
    public static void Write(string folder)
    {
        Directory.CreateDirectory(folder);
        var now = DateTimeOffset.Now;
        var observed = now.AddMinutes(-3);
        IReadOnlyList<Limit> claude =
        [
            new Limit("Claude Code", LimitKind.Session, "Session", 27, now.AddMinutes(191), observed, false, TimeSpan.FromHours(5)),
            new Limit("Claude Code", LimitKind.Weekly, "Weekly", 74, now.AddHours(85), observed, false, TimeSpan.FromDays(7)),
        ];
        var next = now.AddMonths(1);
        var reading = new CopilotReading(15000, 2644, new DateTimeOffset(next.Year, next.Month, 1, 0, 0, 0, now.Offset), observed, CopilotSource.Live);
        var both = claude.Concat(LimitList.FromCopilot(reading, now)).ToList();
        var old = now.AddDays(-3);
        var stale = claude.Select(l => l with { ObservedAt = old })
            .Concat(LimitList.FromCopilot(reading with { ObservedAt = old }, now)).ToList();

        foreach (var light in new[] { false, true })
        {
            var colours = TaskbarColours.For(light, false, BarColourOption.Purple, null);
            var row = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 24 };
            row.Children.Add(PillView.Build(PillModel.From([], TimeZoneInfo.Local, true), colours));
            row.Children.Add(PillView.Build(PillModel.From(claude, TimeZoneInfo.Local, true), colours));
            row.Children.Add(PillView.Build(PillModel.From(both, TimeZoneInfo.Local, true), colours));
            row.Children.Add(PillView.Build(PillModel.From(
                stale, TimeZoneInfo.Local, true, ProviderState.Refused, PillStale.MissesNeeded, ProviderState.Offline, PillStale.MissesNeeded, now), colours));
            var sheet = new Border
            {
                Background = new SolidColorBrush(light ? Color.Parse("#F0F0F0") : Color.Parse("#1C1C1C")),
                Padding = new Thickness(16, 7, 16, 7),
                Child = row,
            };
            foreach (var scale in new[] { 1.0, 1.5, 2.0 })
                Save(sheet, scale, Path.Combine(folder, $"pill-{(light ? "light" : "dark")}-{scale * 100:0}.png"));
        }

        foreach (var style in new[] { FlyoutStyle.Bars, FlyoutStyle.Battery })
        {
            var model = FlyoutModel.From(both, now, TimeZoneInfo.Local, CultureInfo.CurrentCulture, ProviderState.Ok);
            var card = FlyoutView.Build(model, style, NightPalette.For(true), BarColour.Pick(BarColourOption.Purple, null, true).Colour);
            Save(card, 1.0, Path.Combine(folder, $"flyout-{style.ToString().ToLowerInvariant()}.png"));
        }
    }

    private static void Save(Control control, double scale, string file)
    {
        // A control can sit in one tree only: each picture gets its own holder.
        var holder = new Decorator { Child = control };
        holder.Measure(Size.Infinity);
        holder.Arrange(new Rect(holder.DesiredSize));
        var size = holder.DesiredSize;
        using var bitmap = new RenderTargetBitmap(
            new PixelSize((int)Math.Ceiling(size.Width * scale), (int)Math.Ceiling(size.Height * scale)), new Vector(96 * scale, 96 * scale));
        bitmap.Render(holder);
        bitmap.Save(file);
        holder.Child = null;
        Console.WriteLine($"{Path.GetFileName(file)}: {size.Width}x{size.Height} DIP, {bitmap.PixelSize.Width}x{bitmap.PixelSize.Height} px");
    }
}
