using System.Globalization;
using System.IO;
using System.Text.Json;
using System.Windows.Media.Imaging;
using Sobreiro.Core.Config;
using Sobreiro.Core.Display;
using Sobreiro.Core.Media;

namespace Sobreiro.Media;

/// <summary>
/// <c>Sobreiro.exe --media folder [--plate plate.json]</c>: renders every still picture from the
/// app's own views, at the pinned clock and with sample readings only.
/// </summary>
internal static class MediaMode
{
    /// <summary>Renders the stills and the animation frames under <paramref name="outFolder"/>; 0 on success, 1 on failure.</summary>
    public static int Write(string outFolder, string? platePath)
    {
        try
        {
            var culture = MediaPins.Culture;
            CultureInfo.CurrentCulture = culture;
            CultureInfo.CurrentUICulture = culture;
            CultureInfo.DefaultThreadCurrentCulture = culture;
            CultureInfo.DefaultThreadCurrentUICulture = culture;

            (TaskbarPlate Plate, BitmapSource Image)? plate = platePath is null ? null : LoadPlate(platePath);
            if (plate is null)
                ConsoleError.Write("No --plate given: the hero animations and the taskbar stills are skipped.");

            // Encode everything in memory first, so a failure leaves no half-written set.
            var files = new List<(string Path, byte[] Bytes)>();
            var manifest = new List<object>();
            foreach (var still in MediaStills.All)
            {
                switch (still)
                {
                    case DetailsStill d:
                        var scene = new Scene(d.Services, SampleReadings.Calm(d.Services, MediaPins.Now), d.Style, d.Dark, true);
                        files.Add((d.Path, Raster.Png(MediaViews.Details(scene, d.Colour), d.Scale)));
                        manifest.Add(new { path = d.Path, group = "details", dark = d.Dark });
                        break;
                    case PillStill p:
                        var pill = new Scene(p.Services, SampleReadings.Calm(p.Services, MediaPins.Now), default, p.Dark, false);
                        files.Add((p.Path, Raster.Png(MediaViews.Pill(pill), p.Scale)));
                        manifest.Add(new { path = p.Path, group = "pill", dark = p.Dark });
                        break;
                    case IconStill i:
                        var services = i.Copilot ? Services.Copilot : Services.Claude;
                        var icon = MediaViews.Icon(i.Copilot, SampleReadings.Rung(services, i.Level, MediaPins.Now), i.Dark, i.Px, i.Enlarge);
                        files.Add((i.Path, Raster.Png(icon)));
                        manifest.Add(new { path = i.Path, group = "icons", dark = i.Dark });
                        break;
                    case TaskbarStill t when plate is not null:
                        var shot = new Scene(t.Services, SampleReadings.Calm(t.Services, plate.Value.Plate.CapturedAt), default, plate.Value.Plate.Dark, false);
                        var withPill = t.Show != ShowOption.TrayOnly;
                        var onPlate = new PlateCompositor(plate.Value.Plate, plate.Value.Image, [shot], [], plate.Value.Plate.CapturedAt, false, withPill);
                        files.Add((t.Path, Raster.Png(onPlate.Still(shot, withPill))));
                        manifest.Add(new { path = t.Path, group = "taskbar", dark = plate.Value.Plate.Dark });
                        break;
                    case TaskbarStill:
                        break; // no plate: skipped, said once above
                    default:
                        throw new InvalidOperationException("Unknown still: " + still.Path);
                }
            }

            var animations = new List<object>();
            var specs = MediaAnimations.All.Where(a => a.Kind != AnimationKind.Hero).ToList();
            if (plate is not null) specs.AddRange(MediaAnimations.Hero(plate.Value.Plate.CapturedAt));
            foreach (var spec in specs)
            {
                var (animationFiles, width, height) = Encode(spec, plate);
                files.AddRange(animationFiles);
                animations.Add(new { name = spec.Name, kind = KindName(spec.Kind), scale = spec.Scale, folder = "animations/" + spec.Name, width, height });
            }

            foreach (var (path, bytes) in files)
            {
                var full = Path.Combine(outFolder, path);
                Directory.CreateDirectory(Path.GetDirectoryName(full)!);
                File.WriteAllBytes(full, bytes);
            }
            File.WriteAllText(Path.Combine(outFolder, "manifest.json"),
                JsonSerializer.Serialize(new { stills = manifest, animations }, new JsonSerializerOptions { WriteIndented = true }));
            return 0;
        }
        catch (Exception ex)
        {
            return ConsoleError.Fail($"Media failed: {ex.GetType().Name}: {ex.Message}", 1);
        }
    }

    public static int Usage() => ConsoleError.Fail("Usage: Sobreiro.exe --media <folder> [--plate <plate.json>]", 2);

    private static string KindName(AnimationKind kind) => kind switch
    {
        AnimationKind.Hero => "hero",
        AnimationKind.Styles => "styles",
        AnimationKind.Day => "day",
        AnimationKind.CopilotJoins => "copilot-joins",
        AnimationKind.Theme => "theme",
        _ => throw new InvalidOperationException("Unknown animation kind: " + kind),
    };

    private static (List<(string Path, byte[] Bytes)> Files, int Width, int Height) Encode(
        AnimationSpec spec, (TaskbarPlate Plate, BitmapSource Image)? plate)
    {
        var scenes = Timeline.Frames(spec.Beats).Select(f => f.Scene).Distinct(ReferenceEqualityComparer.Instance).Cast<Scene>().ToList();
        if (spec.Kind == AnimationKind.Hero)
        {
            var (p, image) = plate!.Value;
            var onPlate = new PlateCompositor(p, image, scenes, Timeline.Frames(spec.Beats).Where(f => f.Pointer is not null).Select(f => f.Pointer!.Value).Distinct().ToList(), p.CapturedAt, true, true);
            return AnimationWriter.Encode(spec, onPlate.Frame);
        }
        var group = new GroupCompositor(spec.Kind, spec.Scale, scenes);
        return AnimationWriter.Encode(spec, (scene, _) => group.Frame(scene));
    }

    // The plate JSON and its image (relative to the JSON's folder); a bad JSON throws a FormatException naming the field.
    private static (TaskbarPlate Plate, BitmapSource Image) LoadPlate(string jsonPath)
    {
        var plate = TaskbarPlate.Parse(File.ReadAllText(jsonPath));
        var imagePath = Path.Combine(Path.GetDirectoryName(Path.GetFullPath(jsonPath))!, plate.Image);
        var image = new PngBitmapDecoder(new Uri(imagePath), BitmapCreateOptions.PreservePixelFormat, BitmapCacheOption.OnLoad).Frames[0];
        image.Freeze();
        return (plate, image);
    }
}
