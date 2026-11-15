namespace Sobreiro.Core.Connect;

/// <summary>Where Claude Code keeps its settings.</summary>
public static class ClaudePaths
{
    private const string ConfigDirVariable = "CLAUDE_CONFIG_DIR";

    /// <summary>CLAUDE_CONFIG_DIR when set and fully qualified, else USERPROFILE\.claude.</summary>
    public static string ConfigDirectory(Func<string, string?> getEnv)
    {
        var overridden = getEnv(ConfigDirVariable);
        if (!string.IsNullOrEmpty(overridden) && Path.IsPathFullyQualified(overridden)) return overridden;

        var profile = getEnv("USERPROFILE");
        if (string.IsNullOrEmpty(profile)) profile = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        return Path.Combine(profile, ".claude");
    }

    /// <summary>Claude Code's sign-in file. Only Live usage reads it.</summary>
    public static string CredentialsFile(Func<string, string?> getEnv) =>
        Path.Combine(ConfigDirectory(getEnv), ".credentials.json");

    public static string SettingsFile(Func<string, string?> getEnv) =>
        Path.Combine(ConfigDirectory(getEnv), "settings.json");
}
