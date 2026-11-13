using Sobreiro.Core.Copilot;

namespace Sobreiro.Tests;

public class CopilotCredentialQueryTests
{
    private const int NotFound = 1168;
    private const int NoSuchLogonSession = 1312;

    [Fact]
    public void Each_query_asks_for_its_own_entry()
    {
        Assert.Equal(CopilotCredentialName.CliFilter, CopilotCredentialQuery.Cli.Filter);
        Assert.True(CopilotCredentialQuery.Cli.IsOurs("copilot-cli/https://github.com:someone"));
        Assert.False(CopilotCredentialQuery.Cli.IsOurs("copilot-language-server/oauth-token-key"));

        Assert.Equal(CopilotCredentialName.LanguageServerFilter, CopilotCredentialQuery.LanguageServer.Filter);
        Assert.True(CopilotCredentialQuery.LanguageServer.IsOurs("copilot-language-server/oauth-token-key"));
        Assert.False(CopilotCredentialQuery.LanguageServer.IsOurs("copilot-cli/https://github.com:someone"));
    }

    [Fact]
    public void Not_found_is_no_entry()
    {
        Assert.Equal(ListingAnswer.NoEntry, CopilotCredentialQuery.Cli.AfterFailure(NotFound, cliHasBeenUsed: false));
        Assert.Equal(ListingAnswer.NoEntry, CopilotCredentialQuery.LanguageServer.AfterFailure(NotFound, cliHasBeenUsed: false));
    }

    [Fact]
    public void Only_the_cli_entry_may_list_everything_and_only_where_the_cli_has_been_used() =>
        Assert.Equal(ListingAnswer.ListEverything, CopilotCredentialQuery.Cli.AfterFailure(NotFound, cliHasBeenUsed: true));

    [Fact]
    public void The_language_server_entry_never_lists_everything() =>
        Assert.Equal(ListingAnswer.NoEntry, CopilotCredentialQuery.LanguageServer.AfterFailure(NotFound, cliHasBeenUsed: true));

    [Theory]
    [InlineData(true)]
    [InlineData(false)]
    public void Any_other_error_is_a_failure_not_an_empty_answer(bool cliHasBeenUsed)
    {
        Assert.Equal(ListingAnswer.Failed, CopilotCredentialQuery.Cli.AfterFailure(NoSuchLogonSession, cliHasBeenUsed));
        Assert.Equal(ListingAnswer.Failed, CopilotCredentialQuery.LanguageServer.AfterFailure(NoSuchLogonSession, cliHasBeenUsed));
    }
}
