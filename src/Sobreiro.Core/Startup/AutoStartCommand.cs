namespace Sobreiro.Core.Startup;

/// <summary>The text of the Windows Run value that starts the app at sign-in.</summary>
public static class AutoStartCommand
{
    public const string ValueName = "Sobreiro";

    public static string For(string exePath) => "\"" + exePath + "\"";

    /// <summary>True when the value runs exactly this exe (quotes trimmed, case-insensitive).</summary>
    public static bool Matches(string? value, string exePath) =>
        value is not null
        && string.Equals(value.Trim().Trim('"'), exePath.Trim().Trim('"'), StringComparison.OrdinalIgnoreCase);
}
