using Sobreiro.Core.Bridge;

namespace Sobreiro.Tests;

public class BridgeConfigTests
{
    private static string Temp(string? content)
    {
        var path = Path.Combine(Path.GetTempPath(), Guid.NewGuid() + ".json");
        if (content is not null) File.WriteAllText(path, content);
        return path;
    }

    [Fact]
    public void Load_reads_chained_command() =>
        Assert.Equal("npx ccusage statusline", BridgeConfig.Load(Temp("""{"chained_command": "npx ccusage statusline"}""")).ChainedCommand);

    [Fact]
    public void Load_missing_file_is_empty() => Assert.Null(BridgeConfig.Load(Temp(null)).ChainedCommand);

    [Theory]
    [InlineData("""{"chained_command": 5}""")]
    [InlineData("not json")]
    [InlineData("[]")]
    public void Load_corrupt_or_wrong_type_is_empty(string content) => Assert.Null(BridgeConfig.Load(Temp(content)).ChainedCommand);

    [Fact]
    public void Load_blank_command_is_null() => Assert.Null(BridgeConfig.Load(Temp("""{"chained_command": "   "}""")).ChainedCommand);
}
