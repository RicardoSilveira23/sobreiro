using Sobreiro.Core.Config;
using Sobreiro.Core.Connect;

namespace Sobreiro.Tests;

public class ConnectMessageTests
{
    private const string Path = @"C:\Users\me\.claude\settings.json";
    private const string Bridge = "{\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/sobreiro-bridge.exe\\\"\"\n  }";
    private const string Old = "{ \"type\": \"command\", \"command\": \"bash ~/.claude/line.sh\" }";

    private static ConnectionPlan Plan(bool connect, string? before, string? after, bool chains = false, bool exists = true) =>
        new(connect, new SettingsSnapshot(Path, exists, exists ? "{}" : "", false), before, after, "{}", AppConfig.Default, chains);

    [Fact]
    public void Titles_name_the_action()
    {
        Assert.Equal("Connect to Claude Code", ConnectMessage.Title(true));
        Assert.Equal("Disconnect from Claude Code", ConnectMessage.Title(false));
    }

    [Fact]
    public void Connect_without_a_status_line_says_not_set_and_shows_the_bridge()
    {
        var text = ConnectMessage.Confirm(Plan(true, null, Bridge));
        Assert.Equal(
            Path + "\n\nstatusLine now: not set\nstatusLine after: " + Bridge + "\n\nA backup is saved first.",
            text);
    }

    [Fact]
    public void Connect_over_a_status_line_says_it_keeps_running()
    {
        var text = ConnectMessage.Confirm(Plan(true, "  " + Old + "\n", Bridge, chains: true));
        Assert.Equal(
            Path + "\n\nstatusLine now: " + Old + "\nstatusLine after: " + Bridge
            + "\n\nYour current status line keeps running: the bridge passes its output through.\n\nA backup is saved first.",
            text);
    }

    [Fact]
    public void Disconnect_without_a_previous_status_line_says_removed()
    {
        var text = ConnectMessage.Confirm(Plan(false, Bridge, null));
        Assert.Equal(
            Path + "\n\nstatusLine now: " + Bridge + "\nstatusLine after: removed\n\nA backup is saved first.",
            text);
    }

    [Fact]
    public void No_backup_line_when_there_is_no_file_to_back_up()
    {
        var text = ConnectMessage.Confirm(Plan(true, null, Bridge, exists: false));
        Assert.DoesNotContain("backup", text);
    }

    [Fact]
    public void Done_texts()
    {
        Assert.Equal("Connected. The numbers arrive with Claude Code's next response.", ConnectMessage.Done(true, null));
        Assert.Equal("Disconnected.", ConnectMessage.Done(false, null));
    }

    [Fact]
    public void Done_names_the_backup_when_there_is_one()
    {
        const string backup = Path + ".sobreiro-20261109-143000.bak";
        Assert.Equal(
            "Connected. The numbers arrive with Claude Code's next response.\n\nBackup: " + backup,
            ConnectMessage.Done(true, backup));
        Assert.Equal("Disconnected.\n\nBackup: " + backup, ConnectMessage.Done(false, backup));
    }

    [Fact]
    public void Confirm_for_a_reconnect_from_another_folder_says_so()
    {
        var text = ConnectMessage.Confirm(LegacyConnectionTests.ElsewherePlan());
        Assert.Contains("The status line runs a Sobreiro bridge from another folder. This points it at the one this Sobreiro uses.", text);
        Assert.DoesNotContain("OpenHeadroom", text);
    }
}
