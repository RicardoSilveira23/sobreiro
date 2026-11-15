using System.Diagnostics;

namespace Sobreiro.Core.Copilot;

/// <summary>Asks the GitHub CLI for its token (<c>gh auth token</c>). Never throws; never prints or logs what it read.</summary>
public static class GhCli
{
    public static IReadOnlyList<string> Token() =>
        Find(Environment.GetEnvironmentVariable("PATH")) is { } exe
            ? Run(exe, "auth token --hostname github.com", TimeSpan.FromSeconds(5))
            : [];

    /// <summary>The full path of gh.exe in the first absolute PATH entry that holds one; empty and relative entries are skipped.</summary>
    internal static string? Find(string? path)
    {
        foreach (var entry in (path ?? "").Split(Path.PathSeparator, StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
        {
            try
            {
                if (!Path.IsPathFullyQualified(entry)) continue;
                var candidate = Path.Combine(entry, "gh.exe");
                if (File.Exists(candidate)) return candidate;
            }
            catch (ArgumentException) { }
        }
        return null;
    }

    internal static IReadOnlyList<string> Run(string exe, string arguments, TimeSpan timeout)
    {
        try
        {
            var info = new ProcessStartInfo(exe, arguments)
            {
                UseShellExecute = false,
                CreateNoWindow = true,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
            };
            using var process = Process.Start(info);
            if (process is null) return [];

            // Read both pipes asynchronously before waiting, so a full pipe cannot block the child.
            var stdout = process.StandardOutput.ReadToEndAsync();
            _ = process.StandardError.ReadToEndAsync();

            var clock = Stopwatch.StartNew();
            if (!process.WaitForExit(timeout))
            {
                try { process.Kill(entireProcessTree: true); } catch { }
                return [];
            }

            if (process.ExitCode != 0) return [];

            // A grandchild may have inherited the pipe and keep it open: the drain is bounded by the same timeout.
            var remaining = timeout - clock.Elapsed;
            if (remaining < TimeSpan.Zero) remaining = TimeSpan.Zero;
            if (!stdout.Wait(remaining)) return [];
            var token = CopilotToken.Find(stdout.Result);
            return token is null ? [] : [token];
        }
        catch (Exception)
        {
            return [];
        }
    }
}
