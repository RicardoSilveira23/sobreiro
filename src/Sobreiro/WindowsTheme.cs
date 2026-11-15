using Microsoft.Win32;
using Sobreiro.Core.Colours;

namespace Sobreiro;

internal static class WindowsTheme
{
    public static bool IsLight()
    {
        using var key = Registry.CurrentUser.OpenSubKey(@"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
        return key?.GetValue("SystemUsesLightTheme") is int value && value != 0;
    }

    /// <summary>True when the taskbar takes the accent colour (Personalize's ColorPrevalence).</summary>
    public static bool AccentTaskbar()
    {
        using var key = Registry.CurrentUser.OpenSubKey(@"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
        return key?.GetValue("ColorPrevalence") is int value && value != 0;
    }

    /// <summary>The Windows accent palette, or null when the registry holds none.</summary>
    public static AccentPalette? Accent()
    {
        using var key = Registry.CurrentUser.OpenSubKey(@"Software\Microsoft\Windows\CurrentVersion\Explorer\Accent");
        return AccentPalette.FromRegistry(key?.GetValue("AccentPalette") as byte[]);
    }
}
