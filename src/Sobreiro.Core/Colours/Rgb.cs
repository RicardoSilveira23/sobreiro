using System.Globalization;

namespace Sobreiro.Core.Colours;

public readonly record struct Rgb(byte R, byte G, byte B)
{
    /// <summary>Parses "#RRGGBB" (the hash is optional).</summary>
    public static Rgb Parse(string hex)
    {
        var s = hex.AsSpan().TrimStart('#');
        if (s.Length != 6)
            throw new FormatException($"Expected #RRGGBB, got '{hex}'.");
        return new Rgb(
            byte.Parse(s[0..2], NumberStyles.HexNumber, CultureInfo.InvariantCulture),
            byte.Parse(s[2..4], NumberStyles.HexNumber, CultureInfo.InvariantCulture),
            byte.Parse(s[4..6], NumberStyles.HexNumber, CultureInfo.InvariantCulture));
    }

    public string ToHex() => $"#{R:X2}{G:X2}{B:X2}";
}
