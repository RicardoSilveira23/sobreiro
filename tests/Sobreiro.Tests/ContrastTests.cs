using Sobreiro.Core.Colours;

namespace Sobreiro.Tests;

public class ContrastTests
{
    [Fact]
    public void BlackOnWhiteIsTwentyOne() =>
        Assert.Equal(21.0, Contrast.Ratio(Rgb.Parse("#000000"), Rgb.Parse("#FFFFFF")), 2);

    [Fact]
    public void Grey767676OnWhiteIs454() =>
        Assert.Equal(4.54, Contrast.Ratio(Rgb.Parse("#767676"), Rgb.Parse("#FFFFFF")), 0.005);

    [Fact]
    public void RatioIsSymmetric()
    {
        var a = Rgb.Parse("#7B62FF");
        var b = Rgb.Parse("#0B0B0B");
        Assert.Equal(Contrast.Ratio(a, b), Contrast.Ratio(b, a));
    }

    [Fact]
    public void ParseAndToHexRoundTripInUpperCase() =>
        Assert.Equal("#7B62FF", Rgb.Parse("#7b62ff").ToHex());
}
