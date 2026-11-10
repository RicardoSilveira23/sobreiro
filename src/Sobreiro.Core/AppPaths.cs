using System.Security.Cryptography;
using System.Text;

namespace Sobreiro.Core;

public static class AppPaths
{
    private const string DataDirVariable = "SOBREIRO_DATA_DIR";

    private static string? Override()
    {
        var overridden = Environment.GetEnvironmentVariable(DataDirVariable);
        return string.IsNullOrEmpty(overridden) || !Path.IsPathFullyQualified(overridden) ? null : overridden;
    }

    /// <summary>%LocalAppData%\Sobreiro, whatever the override says.</summary>
    public static string DefaultDirectory() =>
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Sobreiro");

    /// <summary>%LocalAppData%\OpenHeadroom, where the app kept its data before 0.5.0.</summary>
    public static string LegacyDirectory() =>
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "OpenHeadroom");

    /// <summary>True when the data directory is the default folder: no override, or one that names it.</summary>
    public static bool IsDefaultDataDirectory() =>
        Override() is not { } dir || Normalise(dir) == Normalise(DefaultDirectory());

    public static string DataDirectory() => Override() ?? DefaultDirectory();

    /// <summary>The bridge copied out of the Sobreiro exe: the one the status line runs.</summary>
    public static string BridgeFile() => Path.Combine(DataDirectory(), "sobreiro-bridge.exe");

    public static string UsageFile() => Path.Combine(DataDirectory(), "usage.json");

    public static string ConfigFile() => Path.Combine(DataDirectory(), "config.json");

    public static string AlertsFile() => Path.Combine(DataDirectory(), "alerts.json");

    public static string LogFile() => Path.Combine(DataDirectory(), "log.txt");

    /// <summary>
    /// "Sobreiro" normally, and when the data directory override names the default folder; with any
    /// other override, "Sobreiro." plus 8 hex digits of its case-insensitive hash, so scratch instances
    /// get their own single-instance name and two trays never share one folder.
    /// </summary>
    public static string InstanceId()
    {
        if (Override() is not { } dir) return "Sobreiro";
        var key = Normalise(dir);
        if (key == Normalise(DefaultDirectory())) return "Sobreiro";
        var hash = SHA256.HashData(Encoding.UTF8.GetBytes(key));
        return "Sobreiro." + Convert.ToHexString(hash, 0, 4).ToLowerInvariant();
    }

    private static string Normalise(string dir) =>
        Path.TrimEndingDirectorySeparator(Path.GetFullPath(dir)).ToUpperInvariant();
}
