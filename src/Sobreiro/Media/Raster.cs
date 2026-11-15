using System.IO;
using System.Windows;
using System.Windows.Media;
using System.Windows.Media.Imaging;

namespace Sobreiro.Media;

/// <summary>Turns a view or a bitmap into PNG bytes, in memory.</summary>
internal static class Raster
{
    /// <summary>The element at its natural size, rendered at 96·scale dpi on a transparent ground.</summary>
    public static byte[] Png(FrameworkElement e, int scale) => Png(Bitmap(e, scale));

    /// <summary>The element at its natural size, as a bitmap at 96·scale dpi (its pixels are its size times scale).</summary>
    public static BitmapSource Bitmap(FrameworkElement e, int scale)
    {
        e.Measure(new System.Windows.Size(double.PositiveInfinity, double.PositiveInfinity));
        e.Arrange(new Rect(e.DesiredSize));
        e.UpdateLayout();
        var bitmap = new RenderTargetBitmap(
            (int)Math.Ceiling(e.ActualWidth * scale), (int)Math.Ceiling(e.ActualHeight * scale),
            96 * scale, 96 * scale, PixelFormats.Pbgra32);
        bitmap.Render(e);
        bitmap.Freeze();
        return bitmap;
    }

    public static byte[] Png(BitmapSource b)
    {
        var encoder = new PngBitmapEncoder();
        encoder.Frames.Add(BitmapFrame.Create(b));
        using var png = new MemoryStream();
        encoder.Save(png);
        return png.ToArray();
    }
}
