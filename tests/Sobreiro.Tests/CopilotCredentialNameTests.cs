using Sobreiro.Core.Copilot;

namespace Sobreiro.Tests;

public class CopilotCredentialNameTests
{
    [Fact]
    public void The_filters_ask_for_each_entry_by_name()
    {
        Assert.Equal("copilot-cli*", CopilotCredentialName.CliFilter);
        Assert.Equal("copilot-language-server/oauth-token-key*", CopilotCredentialName.LanguageServerFilter);
    }

    [Theory]
    [InlineData("copilot-cli/https://github.com:someone")]
    [InlineData("Legacy copilot-cli token")]
    [InlineData("COPILOT-CLI")]
    public void The_cli_entry_has_its_name_anywhere(string name) => Assert.True(CopilotCredentialName.IsCli(name));

    [Theory]
    [InlineData("copilot-language-server/oauth-token-key")]
    [InlineData("git:https://github.com")]
    [InlineData("")]
    [InlineData(null)]
    public void Other_entries_are_not_the_cli(string? name) => Assert.False(CopilotCredentialName.IsCli(name));

    [Theory]
    [InlineData("copilot-language-server/oauth-token-key")]
    [InlineData("Copilot-Language-Server/OAuth-Token-Key")]
    [InlineData("copilot-language-server/oauth-token-key/github.com")]
    public void The_language_server_entry_starts_with_its_name(string name) =>
        Assert.True(CopilotCredentialName.IsLanguageServer(name));

    [Theory]
    [InlineData("copilot-language-server/other")]
    [InlineData("copilot-language-server")]
    [InlineData("copilot-language-server-x/oauth-token-key")]
    [InlineData("my-copilot-language-server/oauth-token-key")]
    [InlineData("copilot-cli/https://github.com:someone")]
    [InlineData("")]
    [InlineData(null)]
    public void Lookalikes_are_not_the_language_server(string? name) =>
        Assert.False(CopilotCredentialName.IsLanguageServer(name));
}
