using System.Windows;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Display;
using Sobreiro.Core.Media;

namespace Sobreiro.Media;

/// <summary>
/// Draws Sobreiro's pill, tray icons, details and the pointer onto a real capture of the taskbar.
/// Everything is in image pixels (screen pixels minus the plate's origin), at 1x.
/// </summary>
internal sealed class PlateCompositor
{
    private const int DetailsMargin = 12, RoomSpare = 4, CropMargin = 24;

    private readonly TaskbarPlate _plate;
    private readonly DateTimeOffset _now;
    private readonly BitmapSource _ground;
    private readonly int _plateWidth, _plateHeight, _room, _cropX;
    private readonly Dictionary<Scene, (BitmapSource Pill, BitmapSource? Details)> _views = new(ReferenceEqualityComparer.Instance);
    private (BitmapSource Image, int HotX, int HotY)? _arrow;

    /// <summary>
    /// <paramref name="scenes"/> are every scene that will be drawn: the widest pill among them is
    /// cleared from the plate (only the captured pill's rectangle when <paramref name="pillShown"/> is
    /// false), and with <paramref name="room"/> the canvas gets the height the tallest details need
    /// above the taskbar.
    /// </summary>
    public PlateCompositor(TaskbarPlate plate, BitmapSource plateImage, IReadOnlyList<Scene> scenes, IReadOnlyList<PointerSpot> pointers, DateTimeOffset now, bool room, bool pillShown)
    {
        _plate = plate;
        _now = now;
        _plateWidth = plateImage.PixelWidth;
        _plateHeight = plateImage.PixelHeight;
        var widest = plate.Pill.Width;
        var tallest = 0;
        var drawn = new List<PixelRect>();
        foreach (var scene in scenes)
        {
            var views = Views(scene);
            if (pillShown) widest = Math.Max(widest, views.Pill.PixelWidth);
            if (views.Details is not null)
            {
                tallest = Math.Max(tallest, views.Details.PixelHeight);
                var (x, y) = plate.DetailsAt(views.Details.PixelWidth, views.Details.PixelHeight, DetailsMargin);
                drawn.Add(new PixelRect(x, y, x + views.Details.PixelWidth, y + views.Details.PixelHeight));
            }
        }
        drawn.Add(plate.ClearedPill(widest));
        if (pointers.Count > 0)
        {
            var arrow = _arrow ??= ArrowCursor.Load();
            var ink = Ink(arrow.Image);
            foreach (var spot in pointers)
            {
                var (x, y) = ArrowAt(spot, arrow);
                drawn.Add(new PixelRect(x + ink.X, y + ink.Y, x + ink.X + ink.Width, y + ink.Y + ink.Height));
            }
        }
        // The canvas keeps only what is drawn; the rest of the capture stays as the ground source.
        _cropX = plate.CropLeft(drawn, CropMargin) - plate.OriginX;
        _room = room ? tallest + DetailsMargin + RoomSpare : 0;
        if (room) CheckFit(scenes, pointers);
        _ground = Cleared(plateImage, widest);
    }

    // Details and pointer must lie inside the canvas, or a real plate would clip them silently.
    private void CheckFit(IReadOnlyList<Scene> scenes, IReadOnlyList<PointerSpot> pointers)
    {
        var canvas = new Rect(0, 0, Width, Height);
        void Inside(Rect r, string what)
        {
            if (!canvas.Contains(r))
                throw new InvalidOperationException(
                    $"{what} falls outside the {canvas.Width}x{canvas.Height} canvas ({r}); capture more of the taskbar.");
        }
        foreach (var scene in scenes)
        {
            if (scene.DetailsOpen && Views(scene).Details is { } d)
            {
                var (x, y) = _plate.DetailsAt(d.PixelWidth, d.PixelHeight, DetailsMargin);
                Inside(new Rect(x - _plate.OriginX - _cropX, y - _plate.OriginY + _room, d.PixelWidth, d.PixelHeight), "The details");
            }
        }
        var arrow = _arrow ??= ArrowCursor.Load();
        var ink = Ink(arrow.Image);
        foreach (var spot in pointers)
        {
            var (x, y) = ArrowAt(spot, arrow);
            // Only the drawn part of the arrow counts: its image has a transparent margin.
            Inside(new Rect(x - _plate.OriginX - _cropX + ink.X, y - _plate.OriginY + _room + ink.Y, ink.Width, ink.Height), "The pointer");
        }
    }

    private static Int32Rect Ink(BitmapSource image)
    {
        var converted = new FormatConvertedBitmap(image, PixelFormats.Bgra32, null, 0);
        var w = converted.PixelWidth;
        var h = converted.PixelHeight;
        var px = new byte[w * h * 4];
        converted.CopyPixels(px, w * 4, 0);
        int minX = w, minY = h, maxX = -1, maxY = -1;
        for (var y = 0; y < h; y++)
            for (var x = 0; x < w; x++)
                if (px[(y * w + x) * 4 + 3] != 0)
                {
                    minX = Math.Min(minX, x); maxX = Math.Max(maxX, x);
                    minY = Math.Min(minY, y); maxY = Math.Max(maxY, y);
                }
        return maxX < 0 ? new Int32Rect(0, 0, 0, 0) : new Int32Rect(minX, minY, maxX - minX + 1, maxY - minY + 1);
    }

    private (int X, int Y) ArrowAt(PointerSpot spot, (BitmapSource Image, int HotX, int HotY) arrow) =>
        ((int)Math.Round((_plate.Pill.Left + _plate.Pill.Right) / 2.0 + spot.Dx) - arrow.HotX,
         (int)Math.Round((_plate.Pill.Top + _plate.Pill.Bottom) / 2.0 + spot.Dy) - arrow.HotY);

    public int Width => _plateWidth - _cropX;
    public int Height => _plateHeight + _room;

    /// <summary>The hero frame: the pill, the icons, the details when open, the pointer.</summary>
    public BitmapSource Frame(Scene scene, PointerSpot? pointer) => Compose(scene, true, pointer);

    /// <summary>A still of the taskbar: the pill only when <paramref name="withPill"/>.</summary>
    public BitmapSource Still(Scene scene, bool withPill) => Compose(scene, withPill, null);

    private BitmapSource Compose(Scene scene, bool withPill, PointerSpot? pointer)
    {
        var views = Views(scene);
        var visual = new DrawingVisual();
        RenderOptions.SetBitmapScalingMode(visual, BitmapScalingMode.NearestNeighbor);
        using (var dc = visual.RenderOpen())
        {
            dc.DrawImage(_ground, new Rect(-_cropX, _room, _plateWidth, _plateHeight));
            if (withPill)
            {
                var (sx, sy) = _plate.PillAt(views.Pill.PixelWidth, views.Pill.PixelHeight);
                Draw(dc, views.Pill, sx, sy);
            }
            // Only the slots the capture has: an icon the plate lacks is not invented.
            if (scene.Services != Services.Copilot && _plate.ClaudeIcon is { } claude)
                DrawIcon(dc, MediaViews.Icon(false, scene.Limits, scene.Dark, _plate.IconPx, 1), claude);
            if (scene.Services != Services.Claude && _plate.CopilotIcon is { } copilot)
                DrawIcon(dc, MediaViews.Icon(true, scene.Limits, scene.Dark, _plate.IconPx, 1), copilot);
            if (scene.DetailsOpen && views.Details is { } details)
            {
                var (dx, dy) = _plate.DetailsAt(details.PixelWidth, details.PixelHeight, DetailsMargin);
                Draw(dc, details, dx, dy);
            }
            if (pointer is { } spot)
            {
                var arrow = _arrow ??= ArrowCursor.Load();
                var (ax, ay) = ArrowAt(spot, arrow);
                Draw(dc, arrow.Image, ax, ay);
            }
        }
        var target = new RenderTargetBitmap(Width, Height, 96, 96, PixelFormats.Pbgra32);
        target.Render(visual);
        target.Freeze();
        return target;
    }

    // The icon at its real size (the app draws it at the small-icon size), centred in its slot.
    private void DrawIcon(DrawingContext dc, BitmapSource icon, PixelRect slot) =>
        Draw(dc, icon, slot.Left + (slot.Width - icon.PixelWidth) / 2, slot.Top + (slot.Height - icon.PixelHeight) / 2);

    // Screen pixels to canvas pixels, the bitmap at its own pixel size.
    private void Draw(DrawingContext dc, BitmapSource image, int screenX, int screenY) =>
        dc.DrawImage(image, new Rect(screenX - _plate.OriginX - _cropX, screenY - _plate.OriginY + _room, image.PixelWidth, image.PixelHeight));

    private (BitmapSource Pill, BitmapSource? Details) Views(Scene scene)
    {
        if (_views.TryGetValue(scene, out var v)) return v;
        var pill = Raster.Bitmap(MediaViews.Pill(scene), 1);
        var details = scene.DetailsOpen ? Raster.Bitmap(MediaViews.Details(scene, BarColourOption.Purple, _now), 1) : null;
        return _views[scene] = (pill, details);
    }

    // The plate with the pill's room and the icon slots painted over by tiling the ground columns.
    private BitmapSource Cleared(BitmapSource plateImage, int widestPill)
    {
        var converted = new FormatConvertedBitmap(plateImage, PixelFormats.Pbgra32, null, 0);
        var stride = _plateWidth * 4;
        var pixels = new byte[stride * _plateHeight];
        converted.CopyPixels(pixels, stride, 0);

        var original = (byte[])pixels.Clone();

        void Clear(PixelRect r)
        {
            for (var x = r.Left; x < r.Right; x++)
            {
                var ix = x - _plate.OriginX;
                if (ix < 0 || ix >= _plateWidth) continue;
                var gx = _plate.GroundColumn(x - r.Left + _plate.GroundLeft) - _plate.OriginX;
                if (gx < 0 || gx >= _plateWidth) throw new FormatException("Plate JSON \"ground\" lies outside the image.");
                for (var y = Math.Max(r.Top - _plate.OriginY, 0); y < Math.Min(r.Bottom - _plate.OriginY, _plateHeight); y++)
                    Array.Copy(original, y * stride + gx * 4, pixels, y * stride + ix * 4, 4);
            }
        }

        Clear(_plate.ClearedPill(widestPill));
        if (_plate.ClaudeIcon is { } claude) Clear(claude);
        if (_plate.CopilotIcon is { } copilot) Clear(copilot);

        var cleared = BitmapSource.Create(_plateWidth, _plateHeight, 96, 96, PixelFormats.Pbgra32, null, pixels, stride);
        cleared.Freeze();
        return cleared;
    }
}
