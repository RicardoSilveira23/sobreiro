using System.Text;
using Sobreiro.Core.Config;
using Sobreiro.Core.Connect;

namespace Sobreiro.Tests;

public class ConnectionTests : IDisposable
{
    private const string Bridge = @"C:\Apps\Sobreiro\sobreiro-bridge.exe";
    private const string BridgeCmd = "\"C:/Apps/Sobreiro/sobreiro-bridge.exe\"";
    private static readonly DateTimeOffset Now = new(2026, 11, 9, 14, 30, 0, TimeSpan.Zero);

    private readonly string _dir = Path.Combine(Path.GetTempPath(), "sobreiro-tests-" + Guid.NewGuid().ToString("N"));
    private string SettingsPath => Path.Combine(_dir, "claude", "settings.json");
    private string ConfigPath => Path.Combine(_dir, "data", "config.json");

    public void Dispose()
    {
        if (Directory.Exists(_dir)) Directory.Delete(_dir, recursive: true);
    }

    private void WriteSettings(string text, bool bom = false)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(SettingsPath)!);
        File.WriteAllBytes(SettingsPath, new UTF8Encoding(bom).GetPreamble().Concat(Encoding.UTF8.GetBytes(text)).ToArray());
    }

    private const string OldStatusLine = "{ \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 }";
    private const string Original = "{\n  \"model\": \"x\",\n  \"statusLine\": " + OldStatusLine + ",\n  \"theme\": \"dark\"\n}\n";

    private ConnectionPlan Connect(AppConfig? config = null) =>
        Connection.PlanConnect(Connection.ReadSettings(SettingsPath), config ?? AppConfig.Default, Bridge);

    [Fact]
    public void Connect_chains_the_old_command_and_saves_the_old_status_line()
    {
        WriteSettings(Original);
        var plan = Connect();

        Assert.True(plan.Connect);
        Assert.True(plan.Chains);
        Assert.Equal("~/.claude/statusline.sh", plan.NewConfig.ChainedCommand);
        Assert.Equal(OldStatusLine, plan.NewConfig.Previous!.RawJson);
        Assert.Equal(OldStatusLine, plan.Before);
        Assert.Equal(BridgeCommand.StatusLineJson(BridgeCmd, OldStatusLine), plan.After);

        var backup = Connection.Apply(plan, ConfigPath, Now);

        Assert.Equal(Path.Combine(_dir, "claude", "settings.json.sobreiro-20261109-143000.bak"), backup);
        Assert.Equal(Encoding.UTF8.GetBytes(Original), File.ReadAllBytes(backup!));
        var written = File.ReadAllText(SettingsPath);
        Assert.Equal(BridgeCmd, SettingsJson.CommandOf(SettingsJson.ReadStatusLine(written)));
        Assert.Contains("\"padding\": 1", written);
        Assert.StartsWith("{\n  \"model\": \"x\",\n  \"statusLine\": {", written);
        Assert.EndsWith(",\n  \"theme\": \"dark\"\n}\n", written);

        var config = AppConfig.Load(ConfigPath);
        Assert.Equal("~/.claude/statusline.sh", config.ChainedCommand);
        Assert.Equal(OldStatusLine, config.Previous!.RawJson);
    }

    [Fact]
    public void Connect_with_no_status_line_has_no_previous_and_does_not_chain()
    {
        WriteSettings("{\n  \"model\": \"x\"\n}\n");
        var plan = Connect();

        Assert.False(plan.Chains);
        Assert.Null(plan.Before);
        Assert.Null(plan.NewConfig.ChainedCommand);
        Assert.NotNull(plan.NewConfig.Previous);
        Assert.Null(plan.NewConfig.Previous!.RawJson);

        Connection.Apply(plan, ConfigPath, Now);
        Assert.Contains("\"previous_status_line\": null", File.ReadAllText(ConfigPath));
    }

    [Fact]
    public void Connect_without_a_settings_file_creates_it_and_makes_no_backup()
    {
        var settings = Connection.ReadSettings(SettingsPath);
        Assert.False(settings.Exists);
        Assert.Equal("", settings.Text);
        Assert.False(settings.Bom);

        var plan = Connection.PlanConnect(settings, AppConfig.Default, Bridge);
        Assert.Null(Connection.Apply(plan, ConfigPath, Now));

        Assert.True(File.Exists(SettingsPath));
        Assert.True(Connection.IsConnected(File.ReadAllText(SettingsPath)));
        Assert.Empty(Directory.GetFiles(Path.GetDirectoryName(SettingsPath)!, "*.bak"));
    }

    [Fact]
    public void Connect_when_already_pointing_at_a_bridge_keeps_the_saved_config_values()
    {
        WriteSettings("{\n  \"statusLine\": { \"type\": \"command\", \"command\": \"C:/old/sobreiro-bridge.exe\" }\n}\n");
        var saved = new AppConfig("keep-me", new PreviousStatusLine("{\"command\":\"keep\"}"),
            AppConfig.Default.Style, AppConfig.Default.BarColour, AppConfig.Default.Extra);

        var plan = Connect(saved);

        Assert.Equal("keep-me", plan.NewConfig.ChainedCommand);
        Assert.Equal("{\"command\":\"keep\"}", plan.NewConfig.Previous!.RawJson);
        Assert.Equal(BridgeCmd, SettingsJson.CommandOf(plan.After));
    }

    [Fact]
    public void Connect_when_already_a_bridge_with_empty_config_does_not_chain_to_it()
    {
        WriteSettings("{\"statusLine\": { \"type\": \"command\", \"command\": \"C:/old/sobreiro-bridge.exe\" }}");
        var plan = Connect();

        Assert.Null(plan.NewConfig.ChainedCommand);
        Assert.Null(plan.NewConfig.Previous);
        Assert.False(plan.Chains);
    }

    [Fact]
    public void Disconnect_restores_the_original_status_line_exactly_and_clears_the_config_keys()
    {
        WriteSettings(Original);
        Connection.Apply(Connect(), ConfigPath, Now);

        var settings = Connection.ReadSettings(SettingsPath);
        var plan = Connection.PlanDisconnect(settings, AppConfig.Load(ConfigPath));
        Assert.False(plan.Connect);
        Assert.Equal(OldStatusLine, plan.After);

        var backup = Connection.Apply(plan, ConfigPath, Now);

        Assert.Equal(Original, File.ReadAllText(SettingsPath));
        Assert.NotNull(backup);
        var config = AppConfig.Load(ConfigPath);
        Assert.Null(config.ChainedCommand);
        Assert.Null(config.Previous);
    }

    [Fact]
    public void Disconnect_after_a_connect_with_no_status_line_removes_it()
    {
        const string text = "{\n  \"model\": \"x\"\n}\n";
        WriteSettings(text);
        Connection.Apply(Connect(), ConfigPath, Now);

        var plan = Connection.PlanDisconnect(Connection.ReadSettings(SettingsPath), AppConfig.Load(ConfigPath));
        Connection.Apply(plan, ConfigPath, Now);

        Assert.Equal(text, File.ReadAllText(SettingsPath));
    }

    [Fact]
    public void Disconnect_of_a_manual_setup_removes_the_status_line()
    {
        WriteSettings("{\n  \"model\": \"x\",\n  \"statusLine\": { \"type\": \"command\", \"command\": \"C:/b/sobreiro-bridge.exe\" }\n}\n");
        var plan = Connection.PlanDisconnect(Connection.ReadSettings(SettingsPath), AppConfig.Default);

        Assert.Null(plan.After);
        Connection.Apply(plan, ConfigPath, Now);
        Assert.Equal("{\n  \"model\": \"x\"\n}\n", File.ReadAllText(SettingsPath));
    }

    [Fact]
    public void Disconnect_of_a_manual_setup_restores_the_chained_command()
    {
        WriteSettings("{\n  \"statusLine\": { \"type\": \"command\", \"command\": \"C:/b/sobreiro-bridge.exe\" }\n}\n");
        var config = AppConfig.Default with { ChainedCommand = "x" };
        var plan = Connection.PlanDisconnect(Connection.ReadSettings(SettingsPath), config);

        Connection.Apply(plan, ConfigPath, Now);

        var statusLine = SettingsJson.ReadStatusLine(File.ReadAllText(SettingsPath));
        Assert.Equal("x", SettingsJson.CommandOf(statusLine));
        Assert.Contains("\"type\": \"command\"", statusLine);
        Assert.Null(AppConfig.Load(ConfigPath).ChainedCommand);
    }

    [Fact]
    public void Disconnect_never_restores_a_chained_bridge_command()
    {
        WriteSettings("{\n  \"statusLine\": { \"type\": \"command\", \"command\": \"C:/b/sobreiro-bridge.exe\" }\n}\n");
        var config = AppConfig.Default with { ChainedCommand = "C:/other/sobreiro-bridge.exe" };
        Assert.Null(Connection.PlanDisconnect(Connection.ReadSettings(SettingsPath), config).After);
    }

    [Fact]
    public void Disconnect_of_a_manual_setup_keeps_the_other_status_line_properties()
    {
        WriteSettings("{\n  \"statusLine\": { \"type\": \"command\", \"command\": \"C:/b/sobreiro-bridge.exe\", \"padding\": 2 }\n}\n");
        var config = AppConfig.Default with { ChainedCommand = "x" };
        Connection.Apply(Connection.PlanDisconnect(Connection.ReadSettings(SettingsPath), config), ConfigPath, Now);

        var statusLine = SettingsJson.ReadStatusLine(File.ReadAllText(SettingsPath));
        Assert.Equal("x", SettingsJson.CommandOf(statusLine));
        Assert.Contains("\"padding\": 2", statusLine);
    }

    [Theory]
    [InlineData("")]
    [InlineData("   ")]
    public void Disconnect_with_a_blank_chained_command_removes_the_status_line(string command)
    {
        WriteSettings("{\n  \"statusLine\": { \"type\": \"command\", \"command\": \"C:/b/sobreiro-bridge.exe\" }\n}\n");
        var config = AppConfig.Default with { ChainedCommand = command };
        Assert.Null(Connection.PlanDisconnect(Connection.ReadSettings(SettingsPath), config).After);
    }

    [Theory]
    [InlineData("")]
    [InlineData("   ")]
    public void Connect_over_a_status_line_with_a_blank_command_does_not_chain(string command)
    {
        var old = "{ \"type\": \"command\", \"command\": \"" + command + "\" }";
        WriteSettings("{\n  \"statusLine\": " + old + "\n}\n");
        var plan = Connect();

        Assert.False(plan.Chains);
        Assert.Null(plan.NewConfig.ChainedCommand);
        Assert.Equal(old, plan.NewConfig.Previous!.RawJson);
    }

    [Fact]
    public void A_settings_file_that_is_a_symbolic_link_stays_a_link()
    {
        var target = Path.Combine(_dir, "dotfiles", "settings.json");
        Directory.CreateDirectory(Path.GetDirectoryName(target)!);
        File.WriteAllText(target, Original);
        Directory.CreateDirectory(Path.GetDirectoryName(SettingsPath)!);
        try
        {
            File.CreateSymbolicLink(SettingsPath, target);
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            return; // no privilege to create symbolic links here: nothing to test against
        }

        var plan = Connect();
        Assert.Equal(SettingsPath, plan.Settings.Path);
        Assert.Equal(OldStatusLine, plan.Before);
        var backup = Connection.Apply(plan, ConfigPath, Now);

        Assert.Equal(target, new FileInfo(SettingsPath).LinkTarget);
        Assert.Equal(BridgeCmd, SettingsJson.CommandOf(SettingsJson.ReadStatusLine(File.ReadAllText(target))));
        Assert.Equal(Path.GetDirectoryName(SettingsPath), Path.GetDirectoryName(backup));
        Assert.Null(new FileInfo(backup!).LinkTarget);
        Assert.Equal(Original, File.ReadAllText(backup!));

        var disconnect = Connection.PlanDisconnect(Connection.ReadSettings(SettingsPath), AppConfig.Load(ConfigPath));
        Connection.Apply(disconnect, ConfigPath, Now.AddSeconds(1));
        Assert.Equal(target, new FileInfo(SettingsPath).LinkTarget);
        Assert.Equal(Original, File.ReadAllText(target));
    }

    [Fact]
    public void PlanDisconnect_throws_when_not_connected()
    {
        WriteSettings(Original);
        Assert.Throws<InvalidOperationException>(() =>
            Connection.PlanDisconnect(Connection.ReadSettings(SettingsPath), AppConfig.Default));
    }

    [Fact]
    public void IsConnected_reads_the_command_and_is_false_for_invalid_json()
    {
        Assert.True(Connection.IsConnected("{\"statusLine\": {\"command\": \"\\\"C:/x/sobreiro-bridge.exe\\\"\"}}"));
        Assert.False(Connection.IsConnected(Original));
        Assert.False(Connection.IsConnected("{oops"));
        Assert.False(Connection.IsConnected(""));
    }

    [Fact]
    public void Apply_refuses_when_settings_changed_since_the_plan()
    {
        WriteSettings(Original);
        var plan = Connect();
        const string changed = "{\n  \"model\": \"other\"\n}\n";
        WriteSettings(changed);

        Assert.Throws<InvalidOperationException>(() => Connection.Apply(plan, ConfigPath, Now));

        Assert.Equal(changed, File.ReadAllText(SettingsPath));
        Assert.False(File.Exists(ConfigPath));
        Assert.Empty(Directory.GetFiles(Path.GetDirectoryName(SettingsPath)!, "*.bak"));
    }

    [Fact]
    public void Apply_refuses_when_the_file_appeared_since_the_plan()
    {
        var plan = Connect();
        WriteSettings(Original);

        Assert.Throws<InvalidOperationException>(() => Connection.Apply(plan, ConfigPath, Now));
        Assert.Equal(Original, File.ReadAllText(SettingsPath));
    }

    [Fact]
    public void A_second_backup_in_the_same_second_does_not_overwrite_the_first()
    {
        WriteSettings(Original);
        var first = Connection.Apply(Connect(), ConfigPath, Now);
        var second = Connection.Apply(
            Connection.PlanDisconnect(Connection.ReadSettings(SettingsPath), AppConfig.Load(ConfigPath)), ConfigPath, Now);

        Assert.NotEqual(first, second);
        Assert.EndsWith("-143000-1.bak", second);
        Assert.Equal(Encoding.UTF8.GetBytes(Original), File.ReadAllBytes(first!));
    }

    [Fact]
    public void A_file_with_a_bom_keeps_its_bom()
    {
        WriteSettings("{\n  \"model\": \"café\"\n}\n", bom: true);
        var settings = Connection.ReadSettings(SettingsPath);
        Assert.True(settings.Bom);
        Assert.StartsWith("{", settings.Text);

        Connection.Apply(Connection.PlanConnect(settings, AppConfig.Default, Bridge), ConfigPath, Now);

        var bytes = File.ReadAllBytes(SettingsPath);
        Assert.Equal(new byte[] { 0xEF, 0xBB, 0xBF }, bytes[..3]);
        Assert.Equal(0x7B, bytes[3]);
        Assert.Contains("café", File.ReadAllText(SettingsPath));
    }

    [Fact]
    public void Invalid_json_throws_FormatException_and_leaves_the_file_alone()
    {
        WriteSettings("{oops");
        Assert.Throws<FormatException>(() => Connect());
        Assert.Equal("{oops", File.ReadAllText(SettingsPath));
    }

    [Fact]
    public void A_comment_inside_the_old_status_line_stops_Connect_before_anything_is_written()
    {
        const string text = "{\n  \"statusLine\": { // x\n \"type\": \"command\", \"command\": \"a\" }\n}\n";
        WriteSettings(text);

        Assert.Throws<FormatException>(() => Connect());

        Assert.Equal(text, File.ReadAllText(SettingsPath));
        Assert.False(File.Exists(ConfigPath));
        Assert.Empty(Directory.GetFiles(Path.GetDirectoryName(SettingsPath)!, "*.bak"));
    }

    [Fact]
    public void A_trailing_comma_inside_the_old_status_line_stops_Connect()
    {
        WriteSettings("{\"statusLine\": { \"command\": \"a\", }}");
        Assert.Throws<FormatException>(() => Connect());
    }

    [Fact]
    public void A_config_that_cannot_be_serialised_leaves_no_backup()
    {
        WriteSettings(Original);
        var plan = Connect(AppConfig.Default) with
        {
            NewConfig = AppConfig.Default with { Previous = new PreviousStatusLine("{ // bad\n}") },
        };

        Assert.ThrowsAny<Exception>(() => Connection.Apply(plan, ConfigPath, Now));

        Assert.Equal(Original, File.ReadAllText(SettingsPath));
        Assert.Empty(Directory.GetFiles(Path.GetDirectoryName(SettingsPath)!, "*.bak"));
    }

    [Fact]
    public void A_file_that_is_not_valid_utf8_is_left_alone()
    {
        Directory.CreateDirectory(Path.GetDirectoryName(SettingsPath)!);
        var bytes = new byte[] { 0x7B, 0x22, 0x61, 0x22, 0x3A, 0x22, 0xE9, 0x22, 0x7D };
        File.WriteAllBytes(SettingsPath, bytes);

        Assert.Throws<FormatException>(() => Connection.ReadSettings(SettingsPath));
        Assert.Equal(bytes, File.ReadAllBytes(SettingsPath));
    }

    [Fact]
    public void Apply_refuses_when_a_bom_was_added_since_the_plan()
    {
        WriteSettings(Original);
        var plan = Connect();
        WriteSettings(Original, bom: true);
        var before = File.ReadAllBytes(SettingsPath);

        Assert.Throws<InvalidOperationException>(() => Connection.Apply(plan, ConfigPath, Now));

        Assert.Equal(before, File.ReadAllBytes(SettingsPath));
        Assert.False(File.Exists(ConfigPath));
    }
}
