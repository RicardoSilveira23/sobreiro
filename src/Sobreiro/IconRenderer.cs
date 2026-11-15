using System.Drawing;
using System.Drawing.Drawing2D;
using System.Runtime.InteropServices;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Display;

namespace Sobreiro;

public static class IconRenderer
{
    public static Icon Render(RingsModel model, TaskbarColours colours, int size)
    {
        using var bitmap = Draw(model, colours, size);
        var hicon = bitmap.GetHicon();
        try
        {
            return (Icon)Icon.FromHandle(hicon).Clone();
        }
        finally
        {
            DestroyIcon(hicon);
        }
    }

    /// <summary>The icon's pixels (32 bpp ARGB); the caller disposes the bitmap.</summary>
    public static Bitmap Draw(RingsModel model, TaskbarColours colours, int size)
    {
        var bitmap = new Bitmap(size, size, System.Drawing.Imaging.PixelFormat.Format32bppArgb);
        try
        {
            using var g = Graphics.FromImage(bitmap);
            g.Clear(Color.Transparent);
            g.SmoothingMode = SmoothingMode.AntiAlias;
            g.PixelOffsetMode = PixelOffsetMode.Half;

            // Outer ring = session, inner ring = weekly (none for Copilot's single ring); each filling clockwise from 12 o'clock.
            var stroke = size * 5f / 32f;
            var gap = Math.Max(1, size / 16);
            var outer = size / 2f - stroke / 2f;
            DrawRing(g, model.Outer, colours, size / 2f, outer, stroke);
            if (model.HasInner)
                DrawRing(g, model.Inner, colours, size / 2f, outer - stroke - gap, stroke);
            return bitmap;
        }
        catch
        {
            bitmap.Dispose();
            throw;
        }
    }

    private static void DrawRing(Graphics g, Ring ring, TaskbarColours colours, float centre, float radius, float stroke)
    {
        var box = new RectangleF(centre - radius, centre - radius, radius * 2, radius * 2);
        var ink = colours.Ink;
        using (var track = new Pen(Color.FromArgb(TaskbarColours.IconTrackAlpha, ink.R, ink.G, ink.B), stroke))
            g.DrawEllipse(track, box);

        var sweep = ring.CappedSweep(radius, stroke);
        if (sweep <= 0) return;
        var fill = colours.Fill(ring.Level);
        using var pen = new Pen(Color.FromArgb(fill.R, fill.G, fill.B), stroke) { StartCap = LineCap.Round, EndCap = LineCap.Round };
        if (sweep >= 360)
            g.DrawEllipse(pen, box);
        else
            g.DrawArc(pen, box, -90f, (float)sweep);
    }

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool DestroyIcon(IntPtr handle);
}
