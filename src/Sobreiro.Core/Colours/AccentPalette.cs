namespace Sobreiro.Core.Colours;

/// <summary>
/// The Windows accent palette. The registry value Explorer\Accent\AccentPalette holds eight RGBA quads:
/// Light3, Light2, Light1, Accent, Dark1, Dark2, Dark3, unused. This type only parses bytes.
/// </summary>
public sealed record AccentPalette(Rgb Light3, Rgb Light2, Rgb Light1, Rgb Accent, Rgb Dark1, Rgb Dark2, Rgb Dark3)
{
    private const int Quads = 7;

    /// <summary>Windows' default blue palette, for when the registry holds none.</summary>
    public static AccentPalette WindowsBlue { get; } = new(
        Rgb.Parse("#99EBFF"), Rgb.Parse("#4CC2FF"), Rgb.Parse("#0091F8"), Rgb.Parse("#0078D4"),
        Rgb.Parse("#0067C0"), Rgb.Parse("#003E92"), Rgb.Parse("#001A68"));

    public static AccentPalette? FromRegistry(byte[]? bytes)
    {
        if (bytes is null || bytes.Length < Quads * 4)
            return null;
        Rgb At(int i) => new(bytes[i * 4], bytes[i * 4 + 1], bytes[i * 4 + 2]);
        return new AccentPalette(At(0), At(1), At(2), At(3), At(4), At(5), At(6));
    }
}
