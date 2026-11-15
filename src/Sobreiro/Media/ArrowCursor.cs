using System.IO;
using System.Windows.Media.Imaging;

namespace Sobreiro.Media;

/// <summary>Windows' default arrow from its own file: the 32 px image and its hotspot, no shadow.</summary>
internal static class ArrowCursor
{
    public static (BitmapSource Image, int HotX, int HotY) Load()
    {
        var path = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.Windows), "Cursors", "aero_arrow.cur");
        return Parse(File.ReadAllBytes(path));
    }

    private static (BitmapSource Image, int HotX, int HotY) Parse(byte[] cur)
    {
        if (cur.Length < 6 || BitConverter.ToUInt16(cur, 2) != 2)
            throw new InvalidDataException("aero_arrow.cur is not a cursor file.");
        var count = BitConverter.ToUInt16(cur, 4);
        for (var i = 0; i < count; i++)
        {
            var e = 6 + i * 16;
            if (cur[e] != 32 || cur[e + 1] != 32) continue;
            var hotX = BitConverter.ToUInt16(cur, e + 4); // the planes field, in a cursor
            var hotY = BitConverter.ToUInt16(cur, e + 6); // the bit-count field, in a cursor
            var size = BitConverter.ToInt32(cur, e + 8);
            var offset = BitConverter.ToInt32(cur, e + 12);
            return (Decode(cur.AsSpan(offset, size).ToArray()), hotX, hotY);
        }
        throw new InvalidDataException("aero_arrow.cur has no 32 px image.");
    }

    private static BitmapSource Decode(byte[] data)
    {
        BitmapSource frame;
        if (data.Length > 8 && data[0] == 0x89 && data[1] == 'P' && data[2] == 'N' && data[3] == 'G')
        {
            frame = new PngBitmapDecoder(new MemoryStream(data), BitmapCreateOptions.PreservePixelFormat, BitmapCacheOption.OnLoad).Frames[0];
        }
        else
        {
            // A DIB: wrap it as a one-entry icon (type 1, one plane, 32 bits) and let the icon decoder read it.
            var ico = new byte[22 + data.Length];
            BitConverter.GetBytes((ushort)1).CopyTo(ico, 2); // type: icon
            BitConverter.GetBytes((ushort)1).CopyTo(ico, 4); // one entry
            ico[6] = 32;
            ico[7] = 32;
            BitConverter.GetBytes((ushort)1).CopyTo(ico, 10); // planes
            BitConverter.GetBytes((ushort)32).CopyTo(ico, 12); // bit count
            BitConverter.GetBytes(data.Length).CopyTo(ico, 14);
            BitConverter.GetBytes(22).CopyTo(ico, 18);
            data.CopyTo(ico, 22);
            frame = new IconBitmapDecoder(new MemoryStream(ico), BitmapCreateOptions.PreservePixelFormat, BitmapCacheOption.OnLoad).Frames[0];
        }
        frame.Freeze();
        return frame;
    }
}
