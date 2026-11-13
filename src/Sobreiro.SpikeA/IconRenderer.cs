using Avalonia;
using Avalonia.Controls;
using Avalonia.Media;
using Avalonia.Media.Imaging;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Display;

namespace Sobreiro.SpikeA;

/// <summary>SPIKE (shell candidate A): the tray icon's rings, drawn with the toolkit instead of GDI+.</summary>
internal static class IconRenderer
{
    public static WindowIcon Render(RingsModel model, TaskbarColours colours, int size)
    {
        using var bitmap = new RenderTargetBitmap(new PixelSize(size, size));
        using (var context = bitmap.CreateDrawingContext())
        {
            var stroke = size * 5.0 / 32.0;
            var gap = Math.Max(1, size / 16);
            var outer = size / 2.0 - stroke / 2.0;
            DrawRing(context, model.Outer, colours, size / 2.0, outer, stroke);
            if (model.HasInner)
                DrawRing(context, model.Inner, colours, size / 2.0, outer - stroke - gap, stroke);
        }
        return new WindowIcon(bitmap);
    }

    private static void DrawRing(DrawingContext context, Ring ring, TaskbarColours colours, double centre, double radius, double stroke)
    {
        var middle = new Point(centre, centre);
        context.DrawEllipse(null, new Pen(PillView.Solid(colours.Ink, TaskbarColours.IconTrackAlpha), stroke), middle, radius, radius);
        var sweep = ring.CappedSweep(radius, stroke);
        if (sweep <= 0) return;
        var pen = new Pen(PillView.Solid(colours.Fill(ring.Level)), stroke, lineCap: PenLineCap.Round);
        if (sweep >= 360) context.DrawEllipse(null, pen, middle, radius, radius);
        else context.DrawGeometry(null, pen, PillView.Arc(middle, radius, sweep));
    }
}
