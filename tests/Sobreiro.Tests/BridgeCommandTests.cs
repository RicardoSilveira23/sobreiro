using Sobreiro.Core.Connect;

namespace Sobreiro.Tests;

public class BridgeCommandTests
{
    [Fact]
    public void For_uses_forward_slashes_and_quotes()
    {
        Assert.Equal("\"C:/Dev/my sobreiro/sobreiro-bridge.exe\"",
            BridgeCommand.For(@"C:\Dev\my sobreiro\sobreiro-bridge.exe"));
    }

    [Theory]
    [InlineData("\"C:/x/Sobreiro-Bridge.exe\"", true)]
    [InlineData("~/bin/sobreiro-bridge", true)]
    [InlineData("~/.claude/statusline.sh", false)]
    [InlineData(null, false)]
    public void IsBridge_recognises_the_bridge(string? command, bool expected) =>
        Assert.Equal(expected, BridgeCommand.IsBridge(command));

    [Fact]
    public void StatusLineJson_builds_an_indented_object_for_a_top_level_property()
    {
        var json = BridgeCommand.StatusLineJson("\"C:/x/sobreiro-bridge.exe\"", null);
        Assert.Equal(
            "{\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/x/sobreiro-bridge.exe\\\"\"\n  }",
            json);
    }

    [Fact]
    public void StatusLineJson_keeps_other_properties_after_command()
    {
        var json = BridgeCommand.StatusLineJson("\"C:/x/sobreiro-bridge.exe\"",
            "{\"type\":\"command\",\"command\":\"old\",\"padding\":2}");
        Assert.Equal(
            "{\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/x/sobreiro-bridge.exe\\\"\",\n    \"padding\": 2\n  }",
            json);
    }

    [Theory]
    [InlineData("\"C:/Users/a/AppData/Local/Sobreiro/sobreiro-bridge.exe\"", true)]
    [InlineData("C:\\Users\\a\\AppData\\Local\\Sobreiro\\sobreiro-bridge.exe", true)]
    [InlineData("  \"c:/users/A/appdata/local/sobreiro/SOBREIRO-BRIDGE.EXE\" ", true)]
    [InlineData("\"C:/Users/a/Downloads/Sobreiro/sobreiro-bridge.exe\"", false)]
    [InlineData("\"C:/Users/a/AppData/Local/Sobreiro/sobreiro-bridge.exe\" | tee x", false)]
    [InlineData(null, false)]
    public void Runs_matches_the_path_however_it_is_written(string? command, bool expected) =>
        Assert.Equal(expected, BridgeCommand.Runs(command, @"C:\Users\a\AppData\Local\Sobreiro\sobreiro-bridge.exe"));

    [Theory]
    [InlineData("C:/data/sobreiro-bridge.exe", "\"C:/data/sobreiro-bridge.exe\"", true)]
    [InlineData("C:/data\\sobreiro-bridge.exe", "\"C:/data/sobreiro-bridge.exe\"", true)]
    [InlineData("C:/data/x/../sobreiro-bridge.exe", "\"C:/data/sobreiro-bridge.exe\"", true)]
    [InlineData(@"C:\data\sobreiro-bridge.exe", "\"C:/data/y/../sobreiro-bridge.exe\"", true)]
    [InlineData("C:/data/sobreiro-bridge.exe", "\"C:/other/sobreiro-bridge.exe\"", false)]
    public void Runs_normalises_both_sides_the_same_way(string bridgeExePath, string command, bool expected) =>
        Assert.Equal(expected, BridgeCommand.Runs(command, bridgeExePath));
}
