using System.ComponentModel;
using System.Diagnostics;
using System.Text;

namespace Sobreiro.Core.Bridge;

public sealed record ChainResult(int ExitCode, string Output);

/// <summary>Runs the user's previous status line command with the same stdin.</summary>
public static class ChainRunner
{
    public static ProcessStartInfo StartInfo(string command, string? bashPath)
    {
        var utf8 = new UTF8Encoding(false); // a BOM on the child's stdin breaks JSON parsers such as jq
        var psi = new ProcessStartInfo
        {
            FileName = bashPath ?? "powershell.exe",
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            StandardInputEncoding = utf8,
            StandardOutputEncoding = utf8,
            UseShellExecute = false,
            CreateNoWindow = true,
        };
        // Lets a bridge started by the chained command see it is inside a chain and not chain again.
        psi.Environment["SOBREIRO_IN_CHAIN"] = "1";
        // stderr stays attached so Claude Code's --debug still sees it.
        if (bashPath is not null)
        {
            psi.ArgumentList.Add("-c");
            psi.ArgumentList.Add(command);
        }
        else
        {
            foreach (var arg in new[] { "-NoProfile", "-NonInteractive", "-Command", "[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); " + command })
                psi.ArgumentList.Add(arg);
        }
        return psi;
    }

    /// <summary>Null when the process cannot be started or does not finish within the timeout (default 5 s).</summary>
    public static ChainResult? Run(string command, string stdin, string? bashPath, TimeSpan? timeout = null)
    {
        try
        {
            using var process = Process.Start(StartInfo(command, bashPath));
            if (process is null) return null;
            var deadline = Stopwatch.StartNew();
            var limit = timeout ?? TimeSpan.FromSeconds(5);
            TimeSpan Remaining() => limit - deadline.Elapsed is { } left && left > TimeSpan.Zero ? left : TimeSpan.Zero;

            var output = process.StandardOutput.ReadToEndAsync();
            // Written on a task so a child that never reads stdin cannot block past the deadline.
            var input = Task.Run(() =>
            {
                try
                {
                    process.StandardInput.Write(stdin);
                    process.StandardInput.Close();
                }
                catch (Exception e) when (e is IOException or ObjectDisposedException or InvalidOperationException)
                {
                    // The child exited without reading all of stdin; its output still counts.
                }
            });

            // Waiting for the output bounds the rest too: a background child holding the pipe open cannot hang us.
            var finished = process.WaitForExit(Remaining()) && WaitFor(output, Remaining());
            if (!finished)
            {
                try { process.Kill(entireProcessTree: true); }
                catch (Exception e) when (e is InvalidOperationException or Win32Exception) { }
                // Observe the abandoned tasks so they cannot raise unobserved exceptions.
                output.ContinueWith(static t => _ = t.Exception, TaskContinuationOptions.OnlyOnFaulted);
                input.ContinueWith(static t => _ = t.Exception, TaskContinuationOptions.OnlyOnFaulted);
                return null;
            }
            return new ChainResult(process.ExitCode, output.GetAwaiter().GetResult());
        }
        catch (Exception e) when (e is Win32Exception or InvalidOperationException or FileNotFoundException)
        {
            return null;
        }
    }

    private static bool WaitFor(Task task, TimeSpan time)
    {
        try { return task.Wait(time); }
        catch (AggregateException) { return true; } // faulted: nothing more to wait for; GetResult rethrows
    }
}
