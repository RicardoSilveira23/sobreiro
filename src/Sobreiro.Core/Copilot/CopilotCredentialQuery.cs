namespace Sobreiro.Core.Copilot;

/// <summary>What to do when Credential Manager answers a filter with an error.</summary>
public enum ListingAnswer
{
    /// <summary>There is no such entry.</summary>
    NoEntry,

    /// <summary>List every entry and keep the ones that are ours.</summary>
    ListEverything,

    /// <summary>The listing itself failed: not the same as finding nothing.</summary>
    Failed,
}

/// <summary>
/// How one Credential Manager entry is looked for: the filter to ask with, which target names are
/// ours, and whether a "not found" may be followed by a listing of every entry.
/// </summary>
public sealed record CopilotCredentialQuery(string Filter, Func<string?, bool> IsOurs, bool MayListEverything)
{
    /// <summary>ERROR_NOT_FOUND: no entry matches the filter.</summary>
    private const int ErrorNotFound = 1168;

    /// <summary>The Copilot CLI's entry. Older versions of the CLI may name it differently, so it may be searched for among all entries.</summary>
    public static readonly CopilotCredentialQuery Cli =
        new(CopilotCredentialName.CliFilter, CopilotCredentialName.IsCli, MayListEverything: true);

    /// <summary>The Copilot language server's entry: asked for by name and nothing else.</summary>
    public static readonly CopilotCredentialQuery LanguageServer =
        new(CopilotCredentialName.LanguageServerFilter, CopilotCredentialName.IsLanguageServer, MayListEverything: false);

    /// <summary>
    /// The answer to a failed listing. Every entry is listed only for a query that allows it, and only
    /// on a PC where the Copilot CLI has left its folder.
    /// </summary>
    public ListingAnswer AfterFailure(int win32Error, bool cliHasBeenUsed) =>
        win32Error != ErrorNotFound ? ListingAnswer.Failed
        : MayListEverything && cliHasBeenUsed ? ListingAnswer.ListEverything
        : ListingAnswer.NoEntry;
}
