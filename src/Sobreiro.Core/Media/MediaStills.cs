using Sobreiro.Core.Colours;
using Sobreiro.Core.Config;
using Sobreiro.Core.Display;

namespace Sobreiro.Core.Media;

public abstract record MediaStill
{
    /// <summary>File path under the media folder, forward slashes, lower case.</summary>
    public abstract string Path { get; }

    protected static string Slug(Services s) => s.ToString().ToLowerInvariant();
    protected static string Theme(bool dark) => dark ? "dark" : "light";
}

public sealed record DetailsStill(Services Services, FlyoutStyle Style, bool Dark, BarColourOption Colour, int Scale) : MediaStill
{
    public override string Path =>
        $"stills/details/{Slug(Services)}-{Style.ToString().ToLowerInvariant()}-{Theme(Dark)}{(Colour == BarColourOption.Accent ? "-accent" : "")}@{Scale}x.png";
}

public sealed record PillStill(Services Services, bool Dark, int Scale) : MediaStill
{
    public override string Path => $"stills/pill/{Slug(Services)}-{Theme(Dark)}@{Scale}x.png";
}

/// <summary>A tray icon; Dark is a dark taskbar. Enlarge is 1, or 8 for the magnified copy.</summary>
public sealed record IconStill(bool Copilot, int Level, bool Dark, int Px, int Enlarge) : MediaStill
{
    public override string Path =>
        $"stills/icons/{(Copilot ? "copilot" : "claude")}-{SampleReadings.LevelNames[Level]}-{Theme(Dark)}-{Px}{(Enlarge == 8 ? "-x8" : "")}.png";
}

public sealed record TaskbarStill(ShowOption Show, Services Services) : MediaStill
{
    public override string Path =>
        $"stills/taskbar/{(Show == ShowOption.TrayOnly ? "tray-only" : "tray-and-pill")}-{Slug(Services)}.png";
}

public static class MediaStills
{
    public static IReadOnlyList<MediaStill> All { get; } = Build();

    private static List<MediaStill> Build()
    {
        var all = new List<MediaStill>();
        foreach (var scale in new[] { 1, 2 })
        foreach (var services in Enum.GetValues<Services>())
        foreach (var style in Enum.GetValues<FlyoutStyle>())
        foreach (var dark in new[] { true, false })
        {
            all.Add(new DetailsStill(services, style, dark, BarColourOption.Purple, scale));
            if (services == Services.Both)
                all.Add(new DetailsStill(services, style, dark, BarColourOption.Accent, scale));
        }
        foreach (var services in Enum.GetValues<Services>())
        foreach (var dark in new[] { true, false })
        foreach (var scale in new[] { 1, 2 })
            all.Add(new PillStill(services, dark, scale));
        foreach (var copilot in new[] { false, true })
        foreach (var level in Enumerable.Range(0, SampleReadings.LevelNames.Length))
        foreach (var dark in new[] { true, false })
        {
            foreach (var px in new[] { 16, 20, 24, 32 })
                all.Add(new IconStill(copilot, level, dark, px, 1));
            all.Add(new IconStill(copilot, level, dark, 16, 8));
        }
        foreach (var show in new[] { ShowOption.TrayOnly, ShowOption.TrayAndPill })
        foreach (var services in new[] { Services.Claude, Services.Both })
            all.Add(new TaskbarStill(show, services));
        return all;
    }
}
