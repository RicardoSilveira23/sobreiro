namespace Sobreiro.Core.Copilot;

/// <summary>What to do when Credential Manager answers a listing with an error.</summary>
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
/// How the Copilot CLI's sign-in is looked for in Windows Credential Manager: the filter to ask
/// with, which target names are its entry, and what a failed listing means. It is the only entry
/// Sobreiro reads there.
/// </summary>
public static class CopilotCliCredential
{
    private const string Name = "copilot-cli";

    /// <summary>ERROR_NOT_FOUND: no entry matches the filter.</summary>
    private const int ErrorNotFound = 1168;

    /// <summary>What Credential Manager is asked for: target names that start with the entry's name.</summary>
    public const string Filter = Name + "*";

    /// <summary>The entry has its name anywhere in the target name: older versions of the CLI may put text before it.</summary>
    public static bool IsOurs(string? targetName) =>
        targetName is not null && targetName.Contains(Name, StringComparison.OrdinalIgnoreCase);

    /// <summary>
    /// The answer to a failed listing by filter. Every entry is listed only when nothing matched
    /// the filter, and only on a PC where the Copilot CLI has left its folder.
    /// </summary>
    public static ListingAnswer AfterFailure(int win32Error, bool cliHasBeenUsed) =>
        win32Error != ErrorNotFound ? ListingAnswer.Failed
        : cliHasBeenUsed ? ListingAnswer.ListEverything
        : ListingAnswer.NoEntry;

    /// <summary>The answer to a failed listing of every entry: an empty Credential Manager is no entry, anything else a failure.</summary>
    public static ListingAnswer AfterListingEverythingFailed(int win32Error) =>
        win32Error == ErrorNotFound ? ListingAnswer.NoEntry : ListingAnswer.Failed;
}
