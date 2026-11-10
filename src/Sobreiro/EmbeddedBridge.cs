using System.IO;
using System.Reflection;
using Sobreiro.Core;
using Sobreiro.Core.Config;
using Sobreiro.Core.Connect;

namespace Sobreiro;

/// <summary>The bridge carried inside Sobreiro.exe, and where it lives once written to the data folder.</summary>
internal static class EmbeddedBridge
{
    private static readonly Assembly Self = typeof(EmbeddedBridge).Assembly;

    /// <summary>True when this build has sobreiro-bridge.exe embedded.</summary>
    public static bool Carried => Self.GetManifestResourceInfo(BridgeInstall.ResourceName) is not null;

    /// <summary>The embedded bridge's bytes.</summary>
    public static Stream Open() =>
        Self.GetManifestResourceStream(BridgeInstall.ResourceName)
        ?? throw new InvalidOperationException("This build doesn't carry the bridge.");

    /// <summary>The bridge Sobreiro owns: the installed copy when carried, else one next to the app.</summary>
    public static string? OwnPath() =>
        BridgeLocator.Own(Carried, AppPaths.BridgeFile(), AppContext.BaseDirectory, File.Exists);

    /// <summary>
    /// At start: when the status line already runs our bridge, bring the installed copy up to date.
    /// Returns the word for the log; never throws.
    /// </summary>
    public static string RefreshAtStart()
    {
        try
        {
            if (!Carried) return "not-carried";
            var installed = AppPaths.BridgeFile();
            var settings = Connection.ReadSettings(ClaudePaths.SettingsFile(Environment.GetEnvironmentVariable));
            if (!settings.Exists || !BridgeInstall.ShouldRefresh(settings.Text, installed)) return "not-used";
            return BridgeInstall.Ensure(Open, installed) ? "written" : "current";
        }
        catch (Exception ex)
        {
            System.Diagnostics.Debug.WriteLine($"Sobreiro: refreshing the bridge failed: {ex.GetType().Name}");
            return "failed:" + ex.GetType().Name;
        }
    }
}
