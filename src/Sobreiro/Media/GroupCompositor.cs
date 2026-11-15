using System.Windows;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Display;
using Sobreiro.Core.Media;

namespace Sobreiro.Media;

/// <summary>
/// The non-hero animations: a few views on a transparent canvas, the same size for every frame,
/// the group anchored to the bottom edge and centred (the details grow upwards, as in the app).
/// </summary>
internal sealed class GroupCompositor
{
    private sealed record Part(BitmapSource Bitmap, int X, int Y);
    private sealed record Group(IReadOnlyList<Part> Parts, int Width, int Height);

    private const int Gap = 8;
    private readonly Dictionary<Scene, Group> _groups = new(ReferenceEqualityComparer.Instance);

    public GroupCompositor(AnimationKind kind, int scale, IEnumerable<Scene> scenes)
    {
        foreach (var scene in scenes)
            _groups[scene] = Layout(kind, scale, scene);
        Width = _groups.Values.Max(g => g.Width);
        Height = _groups.Values.Max(g => g.Height);
    }

    public int Width { get; }
    public int Height { get; }

    public BitmapSource Frame(Scene scene)
    {
        var group = _groups[scene];
        var ox = (Width - group.Width) / 2;
        var oy = Height - group.Height;
        var visual = new DrawingVisual();
        RenderOptions.SetBitmapScalingMode(visual, BitmapScalingMode.NearestNeighbor);
        using (var dc = visual.RenderOpen())
            foreach (var part in group.Parts)
                dc.DrawImage(part.Bitmap, new Rect(ox + part.X, oy + part.Y, part.Bitmap.PixelWidth, part.Bitmap.PixelHeight));
        var target = new RenderTargetBitmap(Width, Height, 96, 96, PixelFormats.Pbgra32);
        target.Render(visual);
        target.Freeze();
        return target;
    }

    private static Group Layout(AnimationKind kind, int scale, Scene s)
    {
        switch (kind)
        {
            case AnimationKind.Day:
            {
                // The pill and the tray icons side by side, bottom-aligned.
                var pill = Raster.Bitmap(MediaViews.Pill(s), scale);
                var icons = new List<BitmapSource>();
                if (s.Services != Services.Copilot) icons.Add(MediaViews.Icon(false, s.Limits, s.Dark, 16, scale));
                if (s.Services != Services.Claude) icons.Add(MediaViews.Icon(true, s.Limits, s.Dark, 16, scale));
                var height = Math.Max(pill.PixelHeight, icons.Max(i => i.PixelHeight));
                var parts = new List<Part> { new(pill, 0, height - pill.PixelHeight) };
                var x = pill.PixelWidth + Gap * scale;
                foreach (var icon in icons)
                {
                    parts.Add(new Part(icon, x, height - icon.PixelHeight));
                    x += icon.PixelWidth + Gap * scale / 2;
                }
                return new Group(parts, x - Gap * scale / 2, height);
            }
            case AnimationKind.CopilotJoins:
            {
                // The details above the pill, the pill under the details' bottom-right.
                var details = Raster.Bitmap(MediaViews.Details(s, BarColourOption.Purple), scale);
                var pill = Raster.Bitmap(MediaViews.Pill(s), scale);
                var width = Math.Max(details.PixelWidth, pill.PixelWidth);
                var below = details.PixelHeight + Gap * scale;
                return new Group([new Part(details, width - details.PixelWidth, 0), new Part(pill, width - pill.PixelWidth, below)],
                    width, below + pill.PixelHeight);
            }
            default:
            {
                var details = Raster.Bitmap(MediaViews.Details(s, BarColourOption.Purple), scale);
                return new Group([new Part(details, 0, 0)], details.PixelWidth, details.PixelHeight);
            }
        }
    }
}
