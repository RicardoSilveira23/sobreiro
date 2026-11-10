namespace Sobreiro.Core.Connect;

/// <summary>The words of the Connect and Disconnect flow: menu item, confirmation box and the box after.</summary>
public static class ConnectMessage
{
    public const string ReconnectTitle = "Reconnect to Claude Code";

    public static string Title(bool connect) => connect ? "Connect to Claude Code" : "Disconnect from Claude Code";

    /// <summary>
    /// The file, the statusLine before and after, whether the old status line keeps running, and the backup
    /// (only when there is a file to back up).
    /// </summary>
    public static string Confirm(ConnectionPlan plan)
    {
        var lines = new List<string>
        {
            plan.Settings.Path,
            "",
            "statusLine now: " + (plan.Before?.Trim() ?? "not set"),
            "statusLine after: " + (plan.After?.Trim() ?? "removed"),
        };
        if (plan.Reconnects)
            lines.AddRange(["", BridgeCommand.IsLegacyBridge(SettingsJson.CommandOf(plan.Before))
                ? "The status line still runs the old OpenHeadroom bridge. This points it at Sobreiro's."
                : "The status line runs a Sobreiro bridge from another folder. This points it at the one this Sobreiro uses."]);
        if (plan.Chains) lines.AddRange(["", "Your current status line keeps running: the bridge passes its output through."]);
        if (plan.Settings.Exists) lines.AddRange(["", "A backup is saved first."]);
        return string.Join("\n", lines);
    }

    /// <summary>What was done, and the backup's full path when <see cref="Connection.Apply"/> made one.</summary>
    public static string Done(bool connect, string? backupPath, bool reconnect = false)
    {
        var done = reconnect ? "Reconnected. The numbers arrive with Claude Code's next response."
            : connect ? "Connected. The numbers arrive with Claude Code's next response." : "Disconnected.";
        return backupPath is null ? done : done + "\n\nBackup: " + backupPath;
    }
}
