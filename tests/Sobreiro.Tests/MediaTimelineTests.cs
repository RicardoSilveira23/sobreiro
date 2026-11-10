using Sobreiro.Core.Display;
using Sobreiro.Core.Media;

namespace Sobreiro.Tests;

public class MediaTimelineTests
{
    private static Scene S(bool open = false) =>
        new(Services.Claude, SampleReadings.Calm(Services.Claude, MediaPins.Now), FlyoutStyle.Bars, true, open);

    private static Scene SceneOf(Beat b) => b switch
    {
        Hold h => h.Scene,
        Glide g => g.Scene,
        _ => throw new InvalidOperationException(),
    };

    [Fact] public void AHoldIsOneFrame()
    {
        var f = Assert.Single(Timeline.Frames([new Hold(S(), 800)]));
        Assert.Equal(800, f.Ms);
        Assert.Null(f.Pointer);
    }

    [Fact] public void GlideDurationsSumToTheBeat()
    {
        var frames = Timeline.Frames([new Glide(S(), new(10, 10), new(0, 0), 350)]);
        Assert.Equal(21, frames.Count);
        Assert.Equal(350, frames.Sum(f => f.Ms));
        Assert.All(frames, f => Assert.InRange(f.Ms, 16, 17));
    }

    [Fact] public void GlideEndsExactlyOnItsTarget()
    {
        var to = new PointerSpot(3.3, -7.7);
        var frames = Timeline.Frames([new Glide(S(), new(150, -110), to, 450)]);
        Assert.Equal(to, frames[^1].Pointer);
    }

    [Fact] public void EaseIsZeroHalfAndOneAtTheEnds()
    {
        Assert.Equal(0, Timeline.Ease(0));
        Assert.Equal(0.5, Timeline.Ease(0.5), 9);
        Assert.Equal(1, Timeline.Ease(1), 9);
    }

    [Fact] public void RepeatedFramesMerge()
    {
        var s = S();
        var f = Assert.Single(Timeline.Frames([new Hold(s, 500), new Hold(s, 700)]));
        Assert.Equal(1200, f.Ms);
    }

    [Fact] public void PosterIndexOfABeatThatStartsANewFrame()
    {
        Beat[] beats = [new Hold(S(), 500), new Hold(S(true), 500), new Hold(S(), 500)];
        Assert.Equal(1, Timeline.PosterIndex(beats, 1));
        Assert.Equal(2, Timeline.PosterIndex(beats, 2));
    }

    [Fact] public void PosterIndexOfABeatMergedIntoThePreviousFrame()
    {
        var s = S();
        Beat[] beats = [new Hold(S(true), 500), new Hold(s, 500), new Hold(s, 500), new Hold(S(true), 500)];
        Assert.Equal(1, Timeline.PosterIndex(beats, 2));
        Assert.Equal(2, Timeline.PosterIndex(beats, 3));
    }

    [Fact] public void PosterIndexOfTheFirstBeatIsZero()
    {
        Beat[] beats = [new Glide(S(), new PointerSpot(0, 0), new PointerSpot(10, 10), 200), new Hold(S(true), 500)];
        Assert.Equal(0, Timeline.PosterIndex(beats, 0));
    }

    [Fact] public void EveryAnimationHasPositiveBeatsAndAUniqueName()
    {
        var all = MediaAnimations.All;
        Assert.Equal(all.Count, all.Select(a => a.Name).Distinct().Count());
        Assert.All(all, a =>
        {
            Assert.NotEmpty(a.Beats);
            Assert.All(a.Beats, b => Assert.True(b.Ms > 0));
            Assert.InRange(a.PosterBeat, 0, a.Beats.Count - 1);
        });
    }

    [Fact] public void OnlyTheHeroHasAPointer()
    {
        foreach (var a in MediaAnimations.All.Where(a => a.Kind != AnimationKind.Hero))
            Assert.All(a.Beats, b => Assert.Null(Assert.IsType<Hold>(b).Pointer));
        Assert.Contains(MediaAnimations.All, a => a.Kind == AnimationKind.Hero && a.Beats.Any(b => b is Glide));
    }

    [Fact] public void StylesCycleBarsBatteryThermometer()
    {
        var styles = MediaAnimations.All.Where(a => a.Kind == AnimationKind.Styles).ToList();
        Assert.NotEmpty(styles);
        Assert.All(styles, a => Assert.Equal(
            [FlyoutStyle.Bars, FlyoutStyle.Battery, FlyoutStyle.Thermometer], a.Beats.Select(b => SceneOf(b).Style)));
    }

    [Fact] public void CopilotJoinsStartsClaudeOnlyAndEndsWithBoth()
    {
        var joins = MediaAnimations.All.Where(a => a.Kind == AnimationKind.CopilotJoins).ToList();
        Assert.NotEmpty(joins);
        Assert.All(joins, a =>
        {
            Assert.Equal(Services.Claude, SceneOf(a.Beats[0]).Services);
            Assert.Equal(Services.Both, SceneOf(a.Beats[^1]).Services);
        });
    }

    [Fact] public void EachCutChangesOneThing()
    {
        var specs = MediaAnimations.All.Concat(MediaAnimations.Hero(MediaPins.Now.AddMinutes(7)));
        foreach (var a in specs)
            for (var i = 1; i < a.Beats.Count; i++)
            {
                Scene p = SceneOf(a.Beats[i - 1]), n = SceneOf(a.Beats[i]);
                var changes = (ReferenceEquals(p.Limits, n.Limits) && p.Services == n.Services ? 0 : 1)
                    + (p.Style != n.Style ? 1 : 0) + (p.Dark != n.Dark ? 1 : 0) + (p.DetailsOpen != n.DetailsOpen ? 1 : 0);
                Assert.True(changes <= 1, $"{a.Name} beat {i} changes {changes} things");
            }
    }

    [Fact] public void HeroLandsAReadingBeforeTheClick()
    {
        foreach (var hero in MediaAnimations.All.Where(a => a.Kind == AnimationKind.Hero))
        {
            var scenes = hero.Beats.Select(SceneOf).ToList();
            var firstOpen = scenes.FindIndex(sc => sc.DetailsOpen);
            Assert.True(firstOpen > 0);
            Assert.Contains(scenes.Take(firstOpen).Skip(1), sc => !ReferenceEquals(sc.Limits, scenes[0].Limits));
        }
    }

    [Fact] public void DayShowsNoDetails()
    {
        foreach (var day in MediaAnimations.All.Where(a => a.Kind == AnimationKind.Day))
            Assert.All(day.Beats, b => Assert.False(SceneOf(b).DetailsOpen));
    }

    [Fact] public void DayEndsWithAReset()
    {
        var days = MediaAnimations.All.Where(a => a.Kind == AnimationKind.Day).ToList();
        Assert.NotEmpty(days);
        foreach (var day in days)
        {
            var last = SceneOf(day.Beats[^1]);
            var m = FlyoutModel.From(last.Limits, MediaPins.Now, MediaPins.Zone, MediaPins.Culture,
                showClaude: SampleReadings.ShowClaude(last.Services));
            if (last.Services == Services.Copilot) Assert.Equal(0.0, m.Copilot!.Credits.UsedFraction, 6);
            else Assert.Equal("0%", m.Session.UsedFigure);
        }
    }

    private static readonly PointerSpot InDetails = new(110, -60);

    [Fact] public void HeroReadsWithThePointerOffThePill()
    {
        foreach (var hero in MediaAnimations.All.Where(a => a.Kind == AnimationKind.Hero))
        {
            var holds = hero.Beats.OfType<Hold>().ToList();
            var landing = holds.Where((h, i) => i > 0 && h.Scene.DetailsOpen && holds[i - 1].Scene.DetailsOpen
                && !ReferenceEquals(h.Scene.Limits, holds[i - 1].Scene.Limits)).ToList();
            Assert.NotEmpty(landing);
            Assert.All(landing, h => Assert.Equal(InDetails, h.Pointer));
        }
    }

    [Fact] public void HeroPosterIsOpenWithThePointerInTheDetails()
    {
        foreach (var hero in MediaAnimations.All.Where(a => a.Kind == AnimationKind.Hero))
        {
            var poster = Assert.IsType<Hold>(hero.Beats[hero.PosterBeat]);
            Assert.True(poster.Scene.DetailsOpen);
            Assert.Equal(InDetails, poster.Pointer);
        }
    }
}
