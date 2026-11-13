namespace Sobreiro.Core.Copilot;

/// <summary>
/// The two Windows Credential Manager entries Sobreiro reads a GitHub sign-in from, by target name:
/// the Copilot CLI's and the Copilot language server's. No other entry is kept.
/// </summary>
public static class CopilotCredentialName
{
    private const string Cli = "copilot-cli";
    private const string LanguageServer = "copilot-language-server/oauth-token-key";

    /// <summary>What Credential Manager is asked for: target names that start with the entry's name.</summary>
    public const string CliFilter = Cli + "*";
    public const string LanguageServerFilter = LanguageServer + "*";

    /// <summary>The Copilot CLI's entry has its name anywhere in the target name: older versions of the CLI may put text before it.</summary>
    public static bool IsCli(string? targetName) =>
        targetName is not null && targetName.Contains(Cli, StringComparison.OrdinalIgnoreCase);

    /// <summary>The language server's entry starts with its full name, key included: one entry is known, so one is read.</summary>
    public static bool IsLanguageServer(string? targetName) =>
        targetName is not null && targetName.StartsWith(LanguageServer, StringComparison.OrdinalIgnoreCase);
}
