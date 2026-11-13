using System.Windows;
using System.Windows.Media.Imaging;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Live;
using Sobreiro.Core.Media;
using Sobreiro.Flyout;
using Sobreiro.Pill;
using Sobreiro.Core.Polling;

namespace Sobreiro.Media;

/// <summary>The app's own views, built from a scene at the pinned clock, zone, culture and accent.</summary>
internal static class MediaViews
{
    /// <summary>The details at <paramref name="now"/> (the pinned clock unless the hero uses the plate's).</summary>
    public static FrameworkElement Details(Scene s, BarColourOption colour, DateTimeOffset? now = null)
    {
        var model = FlyoutModel.From(s.Limits, now ?? MediaPins.Now, MediaPins.Zone, MediaPins.Culture, ProviderState.Off,
            showClaude: SampleReadings.ShowClaude(s.Services));
        return FlyoutView.Build(model, s.Style, FlyoutColours.For(s.Dark, colour, MediaPins.Accent));
    }

    public static FrameworkElement Pill(Scene s) =>
        PillView.Build(PillModel.From(s.Limits, MediaPins.Zone, SampleReadings.ShowClaude(s.Services)), Taskbar(s.Dark));

    public static BitmapSource Icon(bool copilot, IReadOnlyList<Limit> l, bool dark, int px, int scale) =>
        IconBitmaps.Source(copilot ? RingsModel.Copilot(l) : RingsModel.From(l), Taskbar(dark), px, scale);

    // Dark is a dark taskbar, so the light flag of the colours is its opposite.
    private static TaskbarColours Taskbar(bool dark) => TaskbarColours.For(!dark, false, BarColourOption.Purple, null);
}
