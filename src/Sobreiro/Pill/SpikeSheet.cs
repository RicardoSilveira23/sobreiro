using System.Drawing;
using System.Drawing.Imaging;
using System.IO;
using System.Windows;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Config;
using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;
using Color = System.Drawing.Color;
using PixelFormat = System.Drawing.Imaging.PixelFormat;
using Size = System.Windows.Size;

namespace Sobreiro.Pill;

/// <summary>SPIKE: sample pills drawn by WPF (left) and by hand (right) at three scales, to compare by eye.</summary>
internal static class SpikeSheet
{
    public static int Write(string folder)
    {
        try
        {
            Directory.CreateDirectory(folder);
            var report = new List<string>();
            foreach (var light in new[] { false, true })
            {
                var colours = TaskbarColours.For(light, false, BarColourOption.Purple, null);
                foreach (var scale in new[] { 1.0, 1.5, 2.0 })
                {
                    var pairs = Models().Select(m => (Wpf: Wpf(m, colours, scale), Hand: PillPainter.Draw(m, colours, scale, false))).ToList();
                    var rowHeight = pairs.Max(p => Math.Max(p.Wpf.Height, p.Hand.Height)) + 8;
                    var column = pairs.Max(p => p.Wpf.Width) + 24;
                    using var sheet = new Bitmap(column + pairs.Max(p => p.Hand.Width) + 16, rowHeight * pairs.Count + 8, PixelFormat.Format32bppPArgb);
                    using (var g = Graphics.FromImage(sheet))
                    {
                        g.Clear(light ? Color.FromArgb(0xF0, 0xF0, 0xF0) : Color.FromArgb(0x1C, 0x1C, 0x1C));
                        for (var i = 0; i < pairs.Count; i++)
                        {
                            g.DrawImageUnscaled(pairs[i].Wpf, 8, 8 + i * rowHeight);
                            g.DrawImageUnscaled(pairs[i].Hand, column, 8 + i * rowHeight);
                            report.Add($"{(light ? "light" : "dark")} {scale:0.0} row {i}: wpf {pairs[i].Wpf.Width}x{pairs[i].Wpf.Height}, hand {pairs[i].Hand.Width}x{pairs[i].Hand.Height}");
                        }
                    }
                    sheet.Save(Path.Combine(folder, $"pill-{(light ? "light" : "dark")}-{(int)(scale * 100)}.png"), ImageFormat.Png);
                }
            }
            File.WriteAllLines(Path.Combine(folder, "sizes.txt"), report);
            return 0;
        }
        catch (Exception ex)
        {
            File.WriteAllText(Path.Combine(folder, "error.txt"), ex.ToString());
            return 1;
        }
    }

    private static IEnumerable<PillModel> Models()
    {
        static Ring R(int p) => new(p, UsageLevels.For(p), true);
        yield return new(new("5h", "37%"), new("7d", "61%"), new RingsModel(R(37), R(61)));
        yield return new(new("5h", "93%"), new("7d", "100%"), new RingsModel(R(93), R(100)));
        yield return new(new("5h", "–"), new("7d", "–"), new RingsModel(Ring.None, Ring.None));
        yield return new(new("5h", "74%"), new("7d", "27%"), new RingsModel(R(74), R(27)),
            new PillCopilot(new("AI", "42%"), "1 Dec", 0.42, UsageLevels.For(42)));
        yield return new(new("5h", "37%", true), new("7d", "78%", true), new RingsModel(R(37), R(78)),
            new PillCopilot(new("AI", "88%", true), "1 Dec", 0.88, UsageLevels.For(88), true), ClaudeStale: true);
        yield return new(new("5h", "–"), new("7d", "–"), new RingsModel(Ring.None, Ring.None),
            new PillCopilot(new("AI", "9%"), "28 Nov", 0.09, UsageLevels.For(9)), ShowClaude: false);
    }

    private static Bitmap Wpf(PillModel model, TaskbarColours colours, double scale)
    {
        var face = PillView.Build(model, colours);
        face.Measure(new Size(double.PositiveInfinity, double.PositiveInfinity));
        face.Arrange(new Rect(face.DesiredSize));
        face.UpdateLayout();
        int w = (int)Math.Ceiling(face.ActualWidth * scale), h = (int)Math.Ceiling(face.ActualHeight * scale);
        var target = new RenderTargetBitmap(w, h, 96 * scale, 96 * scale, PixelFormats.Pbgra32);
        target.Render(face);
        var bitmap = new Bitmap(w, h, PixelFormat.Format32bppPArgb);
        var data = bitmap.LockBits(new Rectangle(0, 0, w, h), ImageLockMode.WriteOnly, PixelFormat.Format32bppPArgb);
        target.CopyPixels(new Int32Rect(0, 0, w, h), data.Scan0, data.Stride * h, data.Stride);
        bitmap.UnlockBits(data);
        return bitmap;
    }
}
