using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;

namespace Sobreiro.Core.Media;

public sealed record AnimationSpec(string Name, AnimationKind Kind, int Scale, IReadOnlyList<Beat> Beats,
    IReadOnlyList<string> GifGrounds, int BudgetKb, int PosterBeat);

/// <summary>The animations Sobreiro draws: every change is a cut; only the hero's pointer moves.</summary>
public static class MediaAnimations
{
    private const int ReadingMs = 1500, StyleMs = 1600, RungMs = 1000, GlideInMs = 450,
        OpenMs = 2000, ClickMs = 600, MoveUpMs = 400, ReadMs = 1400, GlideOutMs = 350, ClosedMs = 800;
    private static readonly PointerSpot Away = new(190, -60);
    private static readonly PointerSpot InDetails = new(110, -60);
    private static readonly PointerSpot OnPill = new(0, 0);

    public static IReadOnlyList<AnimationSpec> All { get; } = Build();

    private static string Theme(bool dark) => dark ? "dark" : "light";
    private static string Key(Services s) => s switch
    {
        Services.Claude => "claude",
        Services.Copilot => "copilot",
        _ => "both",
    };

    private static List<AnimationSpec> Build()
    {
        var now = MediaPins.Now;
        var all = new List<AnimationSpec>();
        all.AddRange(Hero(now));
        foreach (var s in Enum.GetValues<Services>())
            foreach (var dark in new[] { true, false })
                foreach (var scale in new[] { 1, 2 })
                {
                    var calm = SampleReadings.Calm(s, now);
                    var beats = Enum.GetValues<FlyoutStyle>()
                        .Select(st => (Beat)new Hold(new Scene(s, calm, st, dark, true), StyleMs)).ToList();
                    all.Add(new($"styles-{Key(s)}-{Theme(dark)}@{scale}x", AnimationKind.Styles, scale, beats,
                        scale == 1 ? [Theme(dark)] : [], 300, 0));
                }
        foreach (var s in Enum.GetValues<Services>())
            foreach (var dark in new[] { true, false })
            {
                var beats = Enumerable.Range(1, 4)
                    .Select(l => (Beat)new Hold(new Scene(s, SampleReadings.Rung(s, l, now), FlyoutStyle.Bars, dark, false), RungMs))
                    .ToList();
                // The reset: the session's 5-hour window for Claude, the monthly credits for Copilot alone.
                var reset = s == Services.Copilot ? SampleReadings.At(s, 0, 93, 0, now) : SampleReadings.At(s, 0, 93, 15000, now);
                beats.Add(new Hold(new Scene(s, reset, FlyoutStyle.Bars, dark, false), RungMs));
                all.Add(new($"day-{Key(s)}-{Theme(dark)}@1x", AnimationKind.Day, 1, beats, [Theme(dark)], 300, 2));
            }
        foreach (var dark in new[] { true, false })
        {
            var only = new Scene(Services.Claude, SampleReadings.Calm(Services.Claude, now), FlyoutStyle.Bars, dark, true);
            var both = new Scene(Services.Both, SampleReadings.Calm(Services.Both, now), FlyoutStyle.Bars, dark, true);
            all.Add(new($"copilot-joins-{Theme(dark)}@1x", AnimationKind.CopilotJoins, 1,
                [new Hold(only, ReadingMs), new Hold(both, ReadingMs)], [Theme(dark)], 300, 1));
        }
        foreach (var s in Enum.GetValues<Services>())
        {
            var calm = SampleReadings.Calm(s, now);
            all.Add(new($"theme-{Key(s)}@1x", AnimationKind.Theme, 1,
                [new Hold(new Scene(s, calm, FlyoutStyle.Bars, true, true), ReadingMs),
                 new Hold(new Scene(s, calm, FlyoutStyle.Bars, false, true), ReadingMs)], ["dark"], 300, 0));
        }
        return all;
    }

    /// <summary>The hero: a reading lands on the closed pill, the pointer clicks it open and moves up into the details, a reading lands while open, the pointer clicks the desktop to close.</summary>
    public static IReadOnlyList<AnimationSpec> Hero(DateTimeOffset now) =>
        [HeroFor(Services.Claude, now), HeroFor(Services.Both, now)];

    private static AnimationSpec HeroFor(Services s, DateTimeOffset now)
    {
        var a = SampleReadings.At(s, 13, 44, 2400, now);
        var b = SampleReadings.At(s, 15, 45, 2644, now);
        var c = SampleReadings.At(s, 18, 45, 2644, now);
        Scene Of(IReadOnlyList<Limit> limits, bool open) => new(s, limits, FlyoutStyle.Bars, true, open);
        var closedA = Of(a, false);
        var closedB = Of(b, false);
        var openB = Of(b, true);
        var openC = Of(c, true);
        var closedC = Of(c, false);
        IReadOnlyList<Beat> beats =
        [
            new Hold(closedA, ReadingMs, Away),
            new Hold(closedB, ReadingMs, Away),
            new Glide(closedB, Away, OnPill, GlideInMs),
            new Hold(openB, ClickMs, OnPill),
            new Glide(openB, OnPill, InDetails, MoveUpMs),
            new Hold(openB, ReadMs, InDetails),
            new Hold(openC, OpenMs, InDetails),
            new Glide(openC, InDetails, Away, GlideOutMs),
            new Hold(closedC, ClosedMs, Away),
        ];
        return new($"hero-{Key(s)}@1x", AnimationKind.Hero, 1, beats, ["dark", "light"], 500, 5);
    }
}
