using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Windows.Media.Imaging;
using Sobreiro.Core.Media;

namespace Sobreiro.Media;

/// <summary>An animation as PNG frames plus the <c>timeline.json</c> the encoder reads.</summary>
internal static class AnimationWriter
{
    /// <summary>
    /// <c>animations/name/f000.png…</c> for the distinct frames (one shown twice is stored once) and
    /// <c>timeline.json</c>, with the canvas size every frame shares. Nothing is written to disk here.
    /// </summary>
    public static (List<(string Path, byte[] Bytes)> Files, int Width, int Height) Encode(
        AnimationSpec spec, Func<Scene, PointerSpot?, BitmapSource> render)
    {
        var folder = "animations/" + spec.Name;
        var frames = Timeline.Frames(spec.Beats);
        var files = new List<(string, byte[])>();
        var byContent = new Dictionary<string, string>();
        var listed = new List<object>();
        int width = 0, height = 0;
        foreach (var frame in frames)
        {
            var image = render(frame.Scene, frame.Pointer);
            width = image.PixelWidth;
            height = image.PixelHeight;
            var bytes = Raster.Png(image);
            var key = Convert.ToHexString(SHA256.HashData(bytes));
            if (!byContent.TryGetValue(key, out var name))
            {
                name = $"f{byContent.Count:000}.png";
                byContent[key] = name;
                files.Add(($"{folder}/{name}", bytes));
            }
            listed.Add(new { file = name, ms = frame.Ms });
        }

        var poster = Timeline.PosterIndex(spec.Beats, spec.PosterBeat);

        var timeline = JsonSerializer.Serialize(
            new { frames = listed, poster, grounds = spec.GifGrounds, budgetKb = spec.BudgetKb },
            new JsonSerializerOptions { WriteIndented = true });
        files.Add(($"{folder}/timeline.json", Encoding.UTF8.GetBytes(timeline)));
        return (files, width, height);
    }
}
