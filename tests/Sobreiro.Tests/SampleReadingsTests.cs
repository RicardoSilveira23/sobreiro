using System.Globalization;
using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Media;

namespace Sobreiro.Tests;

public class SampleReadingsTests
{
    private static FlyoutModel Model(Services s, IReadOnlyList<Limit>? limits = null) =>
        FlyoutModel.From(limits ?? SampleReadings.Calm(s, MediaPins.Now), MediaPins.Now, MediaPins.Zone, MediaPins.Culture,
            showClaude: SampleReadings.ShowClaude(s));

    [Fact] public void CalmClaudeReadsOnTrackOnBothRows()
    {
        var m = Model(Services.Claude);
        Assert.Equal(StatusKind.OnTrack, m.Session.Status!.Kind);
        Assert.Equal(StatusKind.OnTrack, m.Weekly.Status!.Kind);
        Assert.Null(m.Copilot);
    }

    [Fact] public void CopilotOnlyHasNoClaudeRows()
    {
        var m = Model(Services.Copilot);
        Assert.False(m.ShowClaude);
        Assert.NotNull(m.Copilot);
    }

    [Fact] public void BothHasClaudeAndCopilot()
    {
        var m = Model(Services.Both);
        Assert.True(m.Session.HasData);
        Assert.NotNull(m.Copilot);
    }

    [Fact] public void PinnedCultureWinsOverTheCurrentOne()
    {
        var before = CultureInfo.CurrentCulture;
        var baseline = Model(Services.Both).Copilot!.Credits;
        try
        {
            CultureInfo.CurrentCulture = CultureInfo.GetCultureInfo("pt-PT");
            var credits = Model(Services.Both).Copilot!.Credits;
            // The app shows the countdown to 1 Dec 00:00 GMT, not the date; the grouped figure is what culture changes.
            Assert.Equal("resets in 20d 10h", credits.ResetsIn);
            Assert.Equal("2,644 of 15,000 credits", credits.Detail);
            Assert.Equal(baseline.UsedFigure, credits.UsedFigure);
        }
        finally { CultureInfo.CurrentCulture = before; }
    }

    [Fact] public void RungZeroIsNoReadingAndRungFourIsUsedUp()
    {
        Assert.Empty(SampleReadings.Rung(Services.Both, 0, MediaPins.Now));
        var m = Model(Services.Both, SampleReadings.Rung(Services.Both, 4, MediaPins.Now));
        Assert.Equal("100%", m.Session.UsedFigure);
        Assert.Equal(1.0, m.Copilot!.Credits.UsedFraction, 6);
    }

    [Theory]
    [InlineData(1, "8%", "44%", 750)]
    [InlineData(2, "27%", "61%", 2644)]
    [InlineData(3, "74%", "80%", 9450)]
    public void RungsGiveTheirExactValues(int level, string session, string weekly, int credits)
    {
        var m = Model(Services.Both, SampleReadings.Rung(Services.Both, level, MediaPins.Now));
        Assert.Equal(session, m.Session.UsedFigure);
        Assert.Equal(weekly, m.Weekly.UsedFigure);
        Assert.Equal(credits / 15000.0, m.Copilot!.Credits.UsedFraction, 6);
    }

    [Fact] public void LevelNamesAreTheFiveRungs() =>
        Assert.Equal(["none", "low", "middle", "high", "full"], SampleReadings.LevelNames);

    [Fact] public void AtGivesTheValuesAsked()
    {
        var m = Model(Services.Both, SampleReadings.At(Services.Both, 13, 44, 2400, MediaPins.Now));
        Assert.Equal("13%", m.Session.UsedFigure);
        Assert.Equal("44%", m.Weekly.UsedFigure);
        Assert.Equal(2400 / 15000.0, m.Copilot!.Credits.UsedFraction, 6);
    }
}
