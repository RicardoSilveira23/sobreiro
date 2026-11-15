using System.Globalization;
using System.Text;
using System.Text.Json;
using Sobreiro.Core.Config;
using Sobreiro.Core.IO;

namespace Sobreiro.Core.Connect;

/// <summary>
/// settings.json as read from disk: whether it exists, its text (BOM stripped) and whether it had a BOM.
/// <c>Path</c> is the path the user sees, even when it is a symbolic link.
/// </summary>
public sealed record SettingsSnapshot(string Path, bool Exists, string Text, bool Bom);

/// <summary>
/// What Connect or Disconnect would do. <see cref="Before"/> and <see cref="After"/> are the raw statusLine
/// value texts (null when absent or removed), for display.
/// </summary>
public sealed record ConnectionPlan(
    bool Connect,
    SettingsSnapshot Settings,
    string? Before,
    string? After,
    string NewSettingsText,
    AppConfig NewConfig,
    bool Chains,
    bool Reconnects = false);

/// <summary>
/// Legacy: the status line runs the old openheadroom-bridge. Elsewhere: it runs a sobreiro-bridge that isn't
/// this Sobreiro's (an older copy in another folder).
/// </summary>
public enum ConnectionState { NotConnected, Connected, Legacy, Elsewhere }

public static class Connection
{
    public static SettingsSnapshot ReadSettings(string path)
    {
        var target = Target(path);
        if (!File.Exists(target)) return new SettingsSnapshot(path, false, "", false);

        var bytes = File.ReadAllBytes(target);
        var bom = bytes.Length >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF;
        string text;
        try
        {
            text = new UTF8Encoding(false, throwOnInvalidBytes: true)
                .GetString(bytes, bom ? 3 : 0, bytes.Length - (bom ? 3 : 0));
        }
        catch (DecoderFallbackException e)
        {
            throw new FormatException("settings.json isn't valid UTF-8, so Sobreiro left it alone.", e);
        }
        return new SettingsSnapshot(path, true, text, bom);
    }

    /// <summary>
    /// The file a symbolic link finally points at, so it is read and written there and the link stays a link;
    /// any other path as it is.
    /// </summary>
    private static string Target(string path) =>
        new FileInfo(path).LinkTarget is not null && File.ResolveLinkTarget(path, returnFinalTarget: true) is { } final
            ? final.FullName
            : path;

    private static string? CommandOrNull(string? command) => string.IsNullOrWhiteSpace(command) ? null : command;

    /// <summary>True when the status line runs the bridge, this app's or the old OpenHeadroom one.</summary>
    public static bool IsConnected(string settingsText) => State(settingsText) != ConnectionState.NotConnected;

    /// <summary>
    /// Connected, connected through the old openheadroom-bridge (Legacy: Reconnect points it at this app's),
    /// or not connected. settings.json that can't be read counts as not connected.
    /// </summary>
    public static ConnectionState State(string settingsText) => State(settingsText, null);

    /// <summary>
    /// As <see cref="State(string)"/>, and with <paramref name="ownBridgePath"/> a sobreiro-bridge that runs from
    /// anywhere else is Elsewhere (Reconnect points it at this Sobreiro's). Null means can't tell: any
    /// sobreiro-bridge is Connected.
    /// </summary>
    public static ConnectionState State(string settingsText, string? ownBridgePath)
    {
        try
        {
            var command = SettingsJson.CommandOf(SettingsJson.ReadStatusLine(settingsText));
            if (BridgeCommand.IsLegacyBridge(command)) return ConnectionState.Legacy;
            if (!BridgeCommand.IsBridge(command)) return ConnectionState.NotConnected;
            return ownBridgePath is null || BridgeCommand.Runs(command, ownBridgePath)
                ? ConnectionState.Connected
                : ConnectionState.Elsewhere;
        }
        catch (FormatException)
        {
            return ConnectionState.NotConnected;
        }
    }

    /// <summary>Throws <see cref="FormatException"/> when settings.json is not a JSON object.</summary>
    public static ConnectionPlan PlanConnect(SettingsSnapshot settings, AppConfig config, string bridgeExePath)
    {
        var before = SettingsJson.ReadStatusLine(settings.Text);
        var oldCommand = CommandOrNull(SettingsJson.CommandOf(before));
        var after = BridgeCommand.StatusLineJson(BridgeCommand.For(bridgeExePath), before);

        // Already pointing at a bridge, this one, an older copy elsewhere or the old OpenHeadroom one (a Reconnect): never chain to it,
        // and keep what the config already saved, so Disconnect still restores the user's own status line.
        if (!BridgeCommand.IsBridge(oldCommand) && before is not null && !IsStrictJson(before))
            throw new FormatException("statusLine has comments or trailing commas, so Sobreiro can't keep an exact copy to restore. Remove them and try again.");

        var newConfig = BridgeCommand.IsBridge(oldCommand)
            ? config
            : config with { ChainedCommand = oldCommand, Previous = new PreviousStatusLine(before) };

        return new ConnectionPlan(
            Connect: true,
            settings,
            before,
            after,
            SettingsJson.WithStatusLine(settings.Text, after),
            newConfig,
            Chains: newConfig.ChainedCommand is not null,
            Reconnects: BridgeCommand.IsBridge(oldCommand) && !BridgeCommand.Runs(oldCommand, bridgeExePath));
    }

    private static bool IsStrictJson(string json)
    {
        try
        {
            using var doc = JsonDocument.Parse(json);
            return true;
        }
        catch (JsonException)
        {
            return false;
        }
    }

    /// <summary>Throws <see cref="InvalidOperationException"/> when the status line is not the bridge.</summary>
    public static ConnectionPlan PlanDisconnect(SettingsSnapshot settings, AppConfig config)
    {
        var before = SettingsJson.ReadStatusLine(settings.Text);
        if (!BridgeCommand.IsBridge(SettingsJson.CommandOf(before)))
            throw new InvalidOperationException("The status line does not run the Sobreiro bridge.");

        // After an in-app Connect the saved statusLine comes back exactly. A bridge set up by hand gets the
        // chained command back in the current object (padding and the rest kept), or loses statusLine.
        string? after;
        if (config.Previous is not null) after = config.Previous.RawJson;
        else if (CommandOrNull(config.ChainedCommand) is { } chained && !BridgeCommand.IsBridge(chained))
            after = BridgeCommand.StatusLineJson(chained, before);
        else after = null;

        return new ConnectionPlan(
            Connect: false,
            settings,
            before,
            after,
            SettingsJson.WithStatusLine(settings.Text, after),
            config with { ChainedCommand = null, Previous = null },
            Chains: false);
    }

    /// <summary>
    /// Writes the plan after checking settings.json has not changed since it was read. Backs the old file up
    /// beside it first; returns the backup path, or null when there was no file. A symbolic link is written
    /// through, with the backup beside the link.
    /// </summary>
    public static string? Apply(ConnectionPlan plan, string configPath, DateTimeOffset localNow)
    {
        var path = plan.Settings.Path;
        var current = ReadSettings(path);
        var target = Target(path);
        if (current.Exists != plan.Settings.Exists || current.Text != plan.Settings.Text || current.Bom != plan.Settings.Bom)
            throw new InvalidOperationException("settings.json changed since the plan was made.");

        var configText = plan.NewConfig.ToJson(); // build everything before touching the disk

        string? backup = null;
        if (current.Exists)
        {
            backup = BackupPath(path, localNow);
            File.Copy(target, backup);
        }

        if (plan.Connect)
        {
            AtomicFile.WriteAllText(configPath, configText);
            AtomicFile.WriteAllText(target, plan.NewSettingsText, plan.Settings.Bom);
        }
        else
        {
            AtomicFile.WriteAllText(target, plan.NewSettingsText, plan.Settings.Bom);
            AtomicFile.WriteAllText(configPath, configText);
        }
        return backup;
    }

    private static string BackupPath(string settingsPath, DateTimeOffset localNow)
    {
        var stamp = localNow.ToString("yyyyMMdd-HHmmss", CultureInfo.InvariantCulture);
        var candidate = $"{settingsPath}.sobreiro-{stamp}.bak";
        for (var n = 1; File.Exists(candidate); n++)
            candidate = $"{settingsPath}.sobreiro-{stamp}-{n}.bak";
        return candidate;
    }
}
