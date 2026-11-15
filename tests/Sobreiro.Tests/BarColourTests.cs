using Sobreiro.Core.Colours;

namespace Sobreiro.Tests;

public class BarColourTests
{
    private static Rgb C(string hex) => Rgb.Parse(hex);

    private static readonly AccentPalette Blue = new(
        C("#99EBFF"), C("#4CC2FF"), C("#0091F8"), C("#0078D4"), C("#0067C0"), C("#003E92"), C("#001A68"));

    private static readonly AccentPalette Storm = new(
        Light3: C("#B5B3B1"), Light2: C("#8F8D8B"), Light1: C("#6B6967"), Accent: C("#4C4A48"),
        Dark1: C("#3A3836"), Dark2: C("#2A2826"), Dark3: C("#1A1816"));

    private static readonly AccentPalette Yellow = new(
        C("#FFFF00"), C("#FFFF00"), C("#FFFF00"), C("#FFFF00"), C("#FFFF00"), C("#FFFF00"), C("#FFFF00"));

    [Theory]
    [InlineData(true, "#A594FF")]
    [InlineData(false, "#5A3DD1")]
    public void PurpleOptionIgnoresTheAccent(bool dark, string hex)
    {
        var pick = BarColour.Pick(BarColourOption.Purple, Blue, dark);
        Assert.Equal(hex, pick.Colour.ToHex());
        Assert.Equal(BarColourSource.Purple, pick.Source);
    }

    [Theory]
    [InlineData(true)]
    [InlineData(false)]
    public void BlueAccentIsUsedInBothThemes(bool dark)
    {
        var pick = BarColour.Pick(BarColourOption.Accent, Blue, dark);
        Assert.Equal("#0078D4", pick.Colour.ToHex());
        Assert.Equal(BarColourSource.Accent, pick.Source);
    }

    [Fact]
    public void StormOnDarkFallsToLight2()
    {
        var pick = BarColour.Pick(BarColourOption.Accent, Storm, dark: true);
        Assert.Equal("#8F8D8B", pick.Colour.ToHex());
        Assert.Equal(BarColourSource.AccentShade, pick.Source);
    }

    [Fact]
    public void StormOnLightKeepsTheAccent()
    {
        var pick = BarColour.Pick(BarColourOption.Accent, Storm, dark: false);
        Assert.Equal("#4C4A48", pick.Colour.ToHex());
        Assert.Equal(BarColourSource.Accent, pick.Source);
    }

    [Fact]
    public void YellowOnLightFallsBackToPurple()
    {
        var pick = BarColour.Pick(BarColourOption.Accent, Yellow, dark: false);
        Assert.Equal("#5A3DD1", pick.Colour.ToHex());
        Assert.Equal(BarColourSource.PurpleFallback, pick.Source);
    }

    [Fact]
    public void YellowOnDarkIsFine()
    {
        var pick = BarColour.Pick(BarColourOption.Accent, Yellow, dark: true);
        Assert.Equal("#FFFF00", pick.Colour.ToHex());
        Assert.Equal(BarColourSource.Accent, pick.Source);
    }

    [Theory]
    [InlineData(true, "#A594FF")]
    [InlineData(false, "#5A3DD1")]
    public void AccentOptionWithoutPaletteFallsBackToPurple(bool dark, string hex)
    {
        var pick = BarColour.Pick(BarColourOption.Accent, null, dark);
        Assert.Equal(hex, pick.Colour.ToHex());
        Assert.Equal(BarColourSource.PurpleFallback, pick.Source);
    }

    [Fact]
    public void LevelColourMapsWarningAndError()
    {
        var n = NightPalette.Dark;
        var normal = C("#123456");
        Assert.Equal(n.Warning, n.LevelColour(Sobreiro.Core.Limits.UsageLevel.Warning, normal));
        Assert.Equal(n.Error, n.LevelColour(Sobreiro.Core.Limits.UsageLevel.Error, normal));
        Assert.Equal(normal, n.LevelColour(Sobreiro.Core.Limits.UsageLevel.Normal, normal));
    }
}
