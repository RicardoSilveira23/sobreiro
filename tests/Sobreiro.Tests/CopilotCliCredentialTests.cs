using Sobreiro.Core.Copilot;

namespace Sobreiro.Tests;

public class CopilotCliCredentialTests
{
    private const int NotFound = 1168;
    private const int NoSuchLogonSession = 1312;

    [Fact]
    public void The_filter_asks_for_the_cli_entry_by_name() => Assert.Equal("copilot-cli*", CopilotCliCredential.Filter);

    [Theory]
    [InlineData("copilot-cli/https://github.com:someone")]
    [InlineData("Legacy copilot-cli token")]
    [InlineData("COPILOT-CLI")]
    public void The_entry_has_its_name_anywhere(string name) => Assert.True(CopilotCliCredential.IsOurs(name));

    // The second is a key another Copilot product keeps there, not a sign-in: it must never be read.
    [Theory]
    [InlineData("git:https://github.com")]
    [InlineData("copilot-language-server/oauth-token-key")]
    [InlineData("copilot")]
    [InlineData("")]
    [InlineData(null)]
    public void Other_entries_are_not_ours(string? name) => Assert.False(CopilotCliCredential.IsOurs(name));

    [Fact]
    public void Not_found_is_no_entry_where_the_cli_has_not_been_used() =>
        Assert.Equal(ListingAnswer.NoEntry, CopilotCliCredential.AfterFailure(NotFound, cliHasBeenUsed: false));

    [Fact]
    public void Not_found_lists_everything_only_where_the_cli_has_been_used() =>
        Assert.Equal(ListingAnswer.ListEverything, CopilotCliCredential.AfterFailure(NotFound, cliHasBeenUsed: true));

    [Theory]
    [InlineData(true)]
    [InlineData(false)]
    public void Any_other_error_is_a_failure_not_an_empty_answer(bool cliHasBeenUsed) =>
        Assert.Equal(ListingAnswer.Failed, CopilotCliCredential.AfterFailure(NoSuchLogonSession, cliHasBeenUsed));

    [Fact]
    public void A_failed_listing_of_everything_is_judged_the_same_way()
    {
        Assert.Equal(ListingAnswer.NoEntry, CopilotCliCredential.AfterListingEverythingFailed(NotFound));
        Assert.Equal(ListingAnswer.Failed, CopilotCliCredential.AfterListingEverythingFailed(NoSuchLogonSession));
    }
}
