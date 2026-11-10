using System.Drawing.Imaging;
using System.Runtime.InteropServices;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Display;
using PixelFormat = System.Drawing.Imaging.PixelFormat;

namespace Sobreiro;

/// <summary>The tray icon renderer's bitmap as a WPF image, each pixel repeated scale × scale times.</summary>
internal static class IconBitmaps
{
    public static BitmapSource Source(RingsModel model, TaskbarColours colours, int size, int scale)
    {
        byte[] source;
        int stride;
        using (var bitmap = IconRenderer.Draw(model, colours, size))
        {
            var data = bitmap.LockBits(new System.Drawing.Rectangle(0, 0, size, size), ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
            try
            {
                stride = data.Stride;
                source = new byte[stride * size];
                Marshal.Copy(data.Scan0, source, 0, source.Length);
            }
            finally
            {
                bitmap.UnlockBits(data);
            }
        }

        var side = size * scale;
        var pixels = new byte[side * side * 4];
        for (var y = 0; y < side; y++)
            for (var x = 0; x < side; x++)
                Array.Copy(source, (y / scale) * stride + (x / scale) * 4, pixels, (y * side + x) * 4, 4);
        return BitmapSource.Create(side, side, 96, 96, PixelFormats.Bgra32, null, pixels, side * 4);
    }
}
