using Sobreiro.Core.Colours;

namespace Sobreiro.Tests;

public class AccentPaletteTests
{
    private static readonly byte[] Blue =
    [
        0x99, 0xEB, 0xFF, 0x00, 0x4C, 0xC2, 0xFF, 0x00, 0x00, 0x91, 0xF8, 0x00, 0x00, 0x78, 0xD4, 0x00,
        0x00, 0x67, 0xC0, 0x00, 0x00, 0x3E, 0x92, 0x00, 0x00, 0x1A, 0x68, 0x00, 0xF7, 0x63, 0x0C, 0x00,
    ];

    [Fact]
    public void ParsesSevenQuadsInOrder()
    {
        var p = AccentPalette.FromRegistry(Blue)!;
        Assert.Equal("#99EBFF", p.Light3.ToHex());
        Assert.Equal("#4CC2FF", p.Light2.ToHex());
        Assert.Equal("#0091F8", p.Light1.ToHex());
        Assert.Equal("#0078D4", p.Accent.ToHex());
        Assert.Equal("#0067C0", p.Dark1.ToHex());
        Assert.Equal("#003E92", p.Dark2.ToHex());
        Assert.Equal("#001A68", p.Dark3.ToHex());
    }

    [Fact]
    public void WindowsBlueIsTheDefaultBluePalette() =>
        Assert.Equal(AccentPalette.FromRegistry(Blue), AccentPalette.WindowsBlue);

    [Fact]
    public void NullOrShortBytesGiveNull()
    {
        Assert.Null(AccentPalette.FromRegistry(null));
        Assert.Null(AccentPalette.FromRegistry(new byte[27]));
    }

    [Fact]
    public void ExactlyTwentyEightBytesIsEnough() =>
        Assert.NotNull(AccentPalette.FromRegistry(Blue[..28]));
}
