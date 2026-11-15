using Sobreiro.Core.Config;
using Sobreiro.Core.Connect;

namespace Sobreiro.Tests;

/// <summary>A status line that still runs the old openheadroom-bridge counts as this app's own.</summary>
public class LegacyConnectionTests : IDisposable
{
    private const string NewBridge = @"C:\Apps\Sobreiro\sobreiro-bridge.exe";
    private const string NewBridgeCmd = "\"C:/Apps/Sobreiro/sobreiro-bridge.exe\"";
    private const string OldBridgeCmd = "\"C:/Apps/OpenHeadroom/openheadroom-bridge.exe\"";
    private static readonly DateTimeOffset Now = new(2026, 11, 9, 14, 30, 0, TimeSpan.Zero);

    // What an in-app Connect with the old build left behind.
    private const string UserStatusLine = "{ \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 }";
    private const string LegacySettings =
        "{\n  \"model\": \"x\",\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/OpenHeadroom/openheadroom-bridge.exe\\\"\",\n    \"padding\": 1\n  },\n  \"theme\": \"dark\"\n}\n";

    private readonly string _dir = Path.Combine(Path.GetTempPath(), "sobreiro-tests-" + Guid.NewGuid().ToString("N"));
    private string SettingsPath => Path.Combine(_dir, "claude", "settings.json");
    private string ConfigPath => Path.Combine(_dir, "data", "config.json");

    public void Dispose()
    {
        if (Directory.Exists(_dir)) Directory.Delete(_dir, recursive: true);
    }

    private void WriteSettings(string text)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(SettingsPath)!);
        File.WriteAllText(SettingsPath, text);
    }

    private static AppConfig MigratedConfig() =>
        AppConfig.Default with { ChainedCommand = "~/.claude/statusline.sh", Previous = new PreviousStatusLine(UserStatusLine) };

    [Theory]
    [InlineData("\"C:/x/openheadroom-bridge.exe\"", true)]
    [InlineData("~/bin/OpenHeadroom-Bridge", true)]
    [InlineData("\"C:/x/sobreiro-bridge.exe\"", false)]
    [InlineData("~/.claude/statusline.sh", false)]
    [InlineData(null, false)]
    public void IsLegacyBridge_recognises_only_the_old_bridge(string? command, bool expected) =>
        Assert.Equal(expected, BridgeCommand.IsLegacyBridge(command));

    [Theory]
    [InlineData("\"C:/x/openheadroom-bridge.exe\"", true)]
    [InlineData("\"C:/x/sobreiro-bridge.exe\"", true)]
    [InlineData("~/.claude/statusline.sh", false)]
    public void IsBridge_counts_the_old_bridge_as_our_own(string command, bool expected) =>
        Assert.Equal(expected, BridgeCommand.IsBridge(command));

    [Fact]
    public void State_tells_not_connected_connected_and_legacy_apart()
    {
        Assert.Equal(ConnectionState.NotConnected, Connection.State("{\"statusLine\":{\"command\":\"~/x.sh\"}}"));
        Assert.Equal(ConnectionState.NotConnected, Connection.State("{}"));
        Assert.Equal(ConnectionState.Connected, Connection.State("{\"statusLine\":{\"command\":" + "\"" + "C:/a/sobreiro-bridge.exe\"}}"));
        Assert.Equal(ConnectionState.Legacy, Connection.State(LegacySettings));
        Assert.Equal(ConnectionState.NotConnected, Connection.State("not json"));
    }

    [Fact]
    public void IsConnected_is_true_for_a_legacy_connection()
    {
        Assert.True(Connection.IsConnected(LegacySettings));
    }

    [Fact]
    public void Reconnect_points_the_status_line_at_the_new_bridge_and_keeps_the_original_backup()
    {
        WriteSettings(LegacySettings);
        var config = MigratedConfig();

        var plan = Connection.PlanConnect(Connection.ReadSettings(SettingsPath), config, NewBridge);

        Assert.True(plan.Connect);
        Assert.True(plan.Reconnects);
        Assert.True(plan.Chains);
        Assert.Equal(config, plan.NewConfig);
        Assert.Equal(UserStatusLine, plan.NewConfig.Previous!.RawJson);
        Assert.Equal(NewBridgeCmd, SettingsJson.CommandOf(plan.After));
        Assert.DoesNotContain(OldBridgeCmd, plan.NewConfig.ToJson());

        var backup = Connection.Apply(plan, ConfigPath, Now);

        Assert.Equal(LegacySettings, File.ReadAllText(backup!)); // the file as it was, before the change
        var written = File.ReadAllText(SettingsPath);
        Assert.Equal(NewBridgeCmd, SettingsJson.CommandOf(SettingsJson.ReadStatusLine(written)));
        Assert.Contains("\"padding\": 1", written);
        Assert.Contains("\"theme\": \"dark\"", written);
        var saved = AppConfig.Load(ConfigPath);
        Assert.Equal(UserStatusLine, saved.Previous!.RawJson);
        Assert.Equal("~/.claude/statusline.sh", saved.ChainedCommand);
    }

    [Fact]
    public void Reconnect_without_a_saved_config_never_saves_the_old_bridge_as_the_users_own()
    {
        WriteSettings(LegacySettings);

        var plan = Connection.PlanConnect(Connection.ReadSettings(SettingsPath), AppConfig.Default, NewBridge);

        Assert.True(plan.Reconnects);
        Assert.Null(plan.NewConfig.ChainedCommand);
        Assert.Null(plan.NewConfig.Previous);
        Assert.False(plan.Chains);
    }

    [Fact]
    public void A_fresh_connect_is_not_a_reconnect()
    {
        WriteSettings("{\"statusLine\":" + UserStatusLine + "}");

        Assert.False(Connection.PlanConnect(Connection.ReadSettings(SettingsPath), AppConfig.Default, NewBridge).Reconnects);
    }

    [Fact]
    public void Disconnect_works_on_a_legacy_connection_and_restores_the_original_status_line()
    {
        WriteSettings(LegacySettings);

        var plan = Connection.PlanDisconnect(Connection.ReadSettings(SettingsPath), MigratedConfig());

        Assert.False(plan.Connect);
        Assert.Equal(UserStatusLine, plan.After);
        Connection.Apply(plan, ConfigPath, Now);
        Assert.Equal("{\n  \"model\": \"x\",\n  \"statusLine\": " + UserStatusLine + ",\n  \"theme\": \"dark\"\n}\n", File.ReadAllText(SettingsPath));
        Assert.Null(AppConfig.Load(ConfigPath).Previous);
    }

    [Fact]
    public void Disconnect_of_a_legacy_connection_with_no_saved_config_removes_the_status_line()
    {
        WriteSettings(LegacySettings);

        var plan = Connection.PlanDisconnect(Connection.ReadSettings(SettingsPath), AppConfig.Default);

        Assert.Null(plan.After);
    }

    [Fact]
    public void Reconnect_then_Disconnect_still_restores_the_original()
    {
        WriteSettings(LegacySettings);
        Connection.Apply(Connection.PlanConnect(Connection.ReadSettings(SettingsPath), MigratedConfig(), NewBridge), ConfigPath, Now);

        var plan = Connection.PlanDisconnect(Connection.ReadSettings(SettingsPath), AppConfig.Load(ConfigPath));

        Assert.Equal(UserStatusLine, plan.After);
    }

    private static string Settings(string command) =>
        "{\"statusLine\":{\"type\":\"command\",\"command\":" + System.Text.Json.JsonSerializer.Serialize(command) + "}}";

    [Fact]
    public void State_with_an_own_bridge_tells_connected_from_elsewhere()
    {
        const string own = @"C:\Data\sobreiro-bridge.exe";
        Assert.Equal(ConnectionState.Connected, Connection.State(Settings("\"C:/Data/sobreiro-bridge.exe\""), own));
        Assert.Equal(ConnectionState.Elsewhere, Connection.State(Settings("\"C:/Users/a/Downloads/Sobreiro/sobreiro-bridge.exe\""), own));
        Assert.Equal(ConnectionState.Legacy, Connection.State(LegacySettings, own));
        Assert.Equal(ConnectionState.NotConnected, Connection.State(Settings("~/x.sh"), own));
        Assert.Equal(ConnectionState.Connected, Connection.State(Settings("\"C:/Users/a/Downloads/Sobreiro/sobreiro-bridge.exe\""), null));
    }

    [Fact]
    public void PlanConnect_from_another_sobreiro_bridge_is_a_reconnect_that_keeps_the_config()
    {
        var plan = ElsewherePlan();

        Assert.True(plan.Reconnects);
        Assert.Equal(MigratedConfig(), plan.NewConfig);
        Assert.Equal("\"C:/Data/sobreiro-bridge.exe\"", SettingsJson.CommandOf(plan.After));
    }

    internal static ConnectionPlan ElsewherePlan()
    {
        var settings = new SettingsSnapshot(@"C:\c\settings.json", true, Settings("\"C:/Old/sobreiro-bridge.exe\""), false);
        return Connection.PlanConnect(settings, MigratedConfig(), @"C:\Data\sobreiro-bridge.exe");
    }
}
