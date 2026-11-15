using Sobreiro.Core.Connect;

namespace Sobreiro.Tests;

public class ClaudePathsTests
{
    private static Func<string, string?> Env(string? configDir, string? profile = @"C:\Users\ana") =>
        name => name switch
        {
            "CLAUDE_CONFIG_DIR" => configDir,
            "USERPROFILE" => profile,
            _ => null,
        };

    [Fact]
    public void Credentials_file_sits_in_the_config_dir()
    {
        Assert.Equal(Path.Combine(@"D:\cfg\claude", ".credentials.json"), ClaudePaths.CredentialsFile(Env(@"D:\cfg\claude")));
        Assert.Equal(Path.Combine(@"C:\Users\ana", ".claude", ".credentials.json"), ClaudePaths.CredentialsFile(Env(null)));
    }

    [Fact]
    public void Config_dir_variable_is_used_when_fully_qualified()
    {
        Assert.Equal(@"D:\cfg\claude", ClaudePaths.ConfigDirectory(Env(@"D:\cfg\claude")));
        Assert.Equal(Path.Combine(@"D:\cfg\claude", "settings.json"), ClaudePaths.SettingsFile(Env(@"D:\cfg\claude")));
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData(@"relative\dir")]
    public void Relative_or_empty_config_dir_is_ignored(string? value)
    {
        Assert.Equal(Path.Combine(@"C:\Users\ana", ".claude"), ClaudePaths.ConfigDirectory(Env(value)));
        Assert.Equal(Path.Combine(@"C:\Users\ana", ".claude", "settings.json"), ClaudePaths.SettingsFile(Env(value)));
    }
}
