using System.IO;
using System.Windows.Forms;
using Sobreiro.Core;
using Sobreiro.Core.Config;
using Sobreiro.Core.Connect;

namespace Sobreiro;

/// <summary>Connect to Claude Code and Disconnect from Claude Code: plan, confirm, apply, say so.</summary>
internal static class ConnectFlow
{
    private const string NoBridge = "Can't find sobreiro-bridge.exe next to Sobreiro.exe or in ..\\bridge.";

    /// <summary>
    /// Where the status line stands now. A settings.json that can't be read counts as not connected.
    /// </summary>
    public static ConnectionState CurrentState()
    {
        try
        {
            return Connection.State(
                Connection.ReadSettings(ClaudePaths.SettingsFile(Environment.GetEnvironmentVariable)).Text, EmbeddedBridge.OwnPath());
        }
        catch (Exception ex)
        {
            System.Diagnostics.Debug.WriteLine($"Sobreiro: reading settings.json failed: {ex.GetType().Name}");
            return ConnectionState.NotConnected;
        }
    }

    /// <summary>
    /// Shows what changes and asks first, then says what was done and where the backup is. Every failure
    /// before the files are written ends in an error box and leaves them as they were: the settings and
    /// config.json parse errors (FormatException) carry their own words.
    /// </summary>
    public static void Run(bool connect)
    {
        string? backup;
        var reconnect = false;
        try
        {
            var settingsPath = ClaudePaths.SettingsFile(Environment.GetEnvironmentVariable);
            var configPath = AppPaths.ConfigFile();

            string? bridge = null;
            if (connect)
            {
                bridge = EmbeddedBridge.OwnPath();
                if (bridge is null)
                {
                    TrayMenu.ShowError(NoBridge);
                    return;
                }
            }

            var settings = Connection.ReadSettings(settingsPath);
            var config = AppConfig.Load(configPath);
            var plan = bridge is not null
                ? Connection.PlanConnect(settings, config, bridge)
                : Connection.PlanDisconnect(settings, config);

            reconnect = plan.Reconnects;
            var answer = MessageBox.Show(
                ConnectMessage.Confirm(plan), Title(connect, reconnect), MessageBoxButtons.YesNo, MessageBoxIcon.None);
            if (answer != DialogResult.Yes) return;

            if (bridge is not null && EmbeddedBridge.Carried) BridgeInstall.Ensure(EmbeddedBridge.Open, bridge);
            backup = Connection.Apply(plan, configPath, DateTimeOffset.Now);
        }
        catch (Exception ex)
        {
            TrayMenu.ShowError(ex.Message);
            return;
        }
        MessageBox.Show(ConnectMessage.Done(connect, backup, reconnect), Title(connect, reconnect), MessageBoxButtons.OK, MessageBoxIcon.None);
    }

    private static string Title(bool connect, bool reconnect) =>
        reconnect ? ConnectMessage.ReconnectTitle : ConnectMessage.Title(connect);
}
