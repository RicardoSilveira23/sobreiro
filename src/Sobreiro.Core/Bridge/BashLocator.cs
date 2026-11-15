namespace Sobreiro.Core.Bridge;

/// <summary>Finds Git Bash the way Claude Code does on Windows.</summary>
public static class BashLocator
{
    public static string? Find(Func<string, string?> getEnv, Func<string, bool> fileExists)
    {
        if (getEnv("CLAUDE_CODE_GIT_BASH_PATH") is { Length: > 0 } configured && fileExists(configured))
            return configured;

        var systemRoot = getEnv("SystemRoot");
        var skipped = new List<string>();
        if (!string.IsNullOrEmpty(systemRoot))
        {
            skipped.Add(Normalise(Path.Combine(systemRoot, "System32")));
            skipped.Add(Normalise(Path.Combine(systemRoot, "SysWOW64")));
        }

        var directories = (getEnv("PATH") ?? "")
            .Split(';', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
            .Select(Normalise)
            .ToList();

        foreach (var dir in directories)
        {
            if (skipped.Contains(dir, StringComparer.OrdinalIgnoreCase)) continue;
            if (string.Equals(Path.GetFileName(dir), "WindowsApps", StringComparison.OrdinalIgnoreCase)) continue;
            var bash = Path.Combine(dir, "bash.exe");
            if (fileExists(bash)) return bash;
        }

        foreach (var dir in directories)
        {
            if (!fileExists(Path.Combine(dir, "git.exe"))) continue;
            var bash = Path.GetFullPath(Path.Combine(dir, "..", "bin", "bash.exe"));
            if (fileExists(bash)) return bash;
        }

        if (getEnv("ProgramFiles") is { Length: > 0 } programFiles)
        {
            var bash = Path.GetFullPath(Path.Combine(programFiles, "Git", "bin", "bash.exe"));
            if (fileExists(bash)) return bash;
        }
        return null;
    }

    private static string Normalise(string directory) =>
        Path.GetFullPath(directory).TrimEnd('\\', '/');
}
