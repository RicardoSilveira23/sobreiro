using Sobreiro.Core.Colours;
using Sobreiro.Core.Config;
using Sobreiro.Core.Display;
using Sobreiro.Core.Media;

namespace Sobreiro.Tests;

public class MediaStillsTests
{
    [Fact] public void DetailsCoverEveryServiceStyleAndThemeAtBothScales()
        => Assert.Equal(48, MediaStills.All.OfType<DetailsStill>().Count()); // 18 purple + 6 accent (Both), x2 scales

    [Fact] public void AccentIsOnlyForBoth()
        => Assert.All(MediaStills.All.OfType<DetailsStill>().Where(d => d.Colour == BarColourOption.Accent),
            d => Assert.Equal(Services.Both, d.Services));

    [Fact] public void CountsPerSurface()
    {
        Assert.Equal(12, MediaStills.All.OfType<PillStill>().Count());
        Assert.Equal(100, MediaStills.All.OfType<IconStill>().Count()); // 2 rings x 5 levels x 2 themes x (4 px + one x8)
        Assert.Equal(4, MediaStills.All.OfType<TaskbarStill>().Count()); // 2 shows x (Claude, Both)
    }

    [Fact] public void PathsAreUniqueAndLowerCase()
    {
        var paths = MediaStills.All.Select(s => s.Path).ToList();
        Assert.Equal(paths.Count, paths.Distinct().Count());
        Assert.All(paths, p => Assert.Matches(@"^stills/(details|pill|icons|taskbar)/[a-z0-9@\-]+\.png$", p));
    }

    [Fact] public void NamesSayWhatTheyShow()
    {
        Assert.Equal("stills/details/both-battery-light-accent@2x.png",
            new DetailsStill(Services.Both, FlyoutStyle.Battery, false, BarColourOption.Accent, 2).Path);
        Assert.Equal("stills/icons/copilot-full-dark-16-x8.png", new IconStill(true, 4, true, 16, 8).Path);
        Assert.Equal("stills/taskbar/tray-and-pill-both.png", new TaskbarStill(ShowOption.TrayAndPill, Services.Both).Path);
    }
}
