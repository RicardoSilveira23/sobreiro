using System.Diagnostics;
using Sobreiro.Core.Baseline;

namespace Sobreiro.Baseline;

internal sealed record IdleResult(long PrivateBytes, long WorkingSetBytes, double CpuSecondsPerHour);

/// <summary>The runs themselves. Every copy started here gets the scratch folders in <paramref name="scratch"/>.</summary>
internal sealed class Measure(string scratch, Action<string> say)
{
    private static readonly TimeSpan PillWait = TimeSpan.FromSeconds(60);
    private static readonly TimeSpan QuitWait = TimeSpan.FromSeconds(10);

    private const string StatusLine =
        """{"session_id":"baseline","rate_limits":{"five_hour":{"used_percentage":23.4,"resets_at":1999999999},"seven_day":{"used_percentage":41,"resets_at":1999999999}}}""";

    private string Folder(string name)
    {
        var path = Path.Combine(scratch, name);
        Directory.CreateDirectory(path);
        return path;
    }

    /// <summary>
    /// Starts the tray on its own data folder, so it never meets an installed copy. Signed out, it also
    /// gets an empty Claude config folder: repeated starts then make no repeated calls for Claude's usage.
    /// </summary>
    private Process StartTray(string exe, bool signedOut)
    {
        var info = new ProcessStartInfo(exe) { UseShellExecute = false, WorkingDirectory = scratch };
        info.Environment["SOBREIRO_DATA_DIR"] = Folder("data");
        info.Environment["DOTNET_BUNDLE_EXTRACT_BASE_DIR"] = Folder("extract");
        if (signedOut) info.Environment["CLAUDE_CONFIG_DIR"] = Folder("claude-config");
        return Process.Start(info) ?? throw new InvalidOperationException("The tray did not start.");
    }

    private static TimeSpan? WaitForPill(Process tray, Stopwatch sinceStart)
    {
        while (sinceStart.Elapsed < PillWait && !tray.HasExited)
        {
            if (Native.PillShown(tray.Id)) return sinceStart.Elapsed;
            Thread.Sleep(2);
        }
        return null;
    }

    /// <summary>What was there instead, when no pill was seen.</summary>
    private void SayWindows(Process tray)
    {
        if (tray.HasExited) return;
        say("  taskbar: " + (Native.Taskbar() is { } t ? $"{t.Handle:x} at {Edges(t.Rect)}" : "not found"));
        foreach (var (hwnd, facts) in Native.WindowsOf(tray.Id).Where(w => w.Facts.Rect.Width > 0))
            say($"  window {hwnd:x}: visible={facts.Visible} owner={facts.Owner:x} at {Edges(facts.Rect)}");

        static string Edges(Sobreiro.Core.Display.PixelRect r) => $"[{r.Left},{r.Top},{r.Right},{r.Bottom}]";
    }

    private void Stop(Process tray)
    {
        if (tray.HasExited) return;
        Native.AskToQuit(tray.Id);
        if (tray.WaitForExit(QuitWait)) return;
        say("  did not leave when asked: killed (its tray icon stays until the mouse passes over it)");
        tray.Kill();
        tray.WaitForExit();
    }

    /// <summary>Time from launch to a pill over the taskbar, once per start. The first start also unpacks the exe.</summary>
    public List<TimeSpan?> Starts(string exe, int count)
    {
        var times = new List<TimeSpan?>();
        for (var i = 1; i <= count; i++)
        {
            var clock = Stopwatch.StartNew();
            using var tray = StartTray(exe, signedOut: true);
            var pill = WaitForPill(tray, clock);
            times.Add(pill);
            say($"  start {i}: " + (pill is { } t ? $"pill after {t.TotalMilliseconds:0} ms" : "no pill seen"));
            if (pill is null) SayWindows(tray);
            Stop(tray);
            Thread.Sleep(1000);
        }
        return times;
    }

    /// <summary>
    /// One run with this PC's sign-ins, left alone for <paramref name="minutes"/>: memory at the end, and
    /// the processor time from the end of the first minute on, scaled to an hour.
    /// </summary>
    public IdleResult Idle(string exe, int minutes)
    {
        var clock = Stopwatch.StartNew();
        using var tray = StartTray(exe, signedOut: false);
        try
        {
            var pill = WaitForPill(tray, clock);
            say("  idle run: " + (pill is { } t ? $"pill after {t.TotalMilliseconds:0} ms" : "no pill seen"));
            if (pill is null) SayWindows(tray);
            var cpuAfterFirstMinute = TimeSpan.Zero;
            for (var minute = 1; minute <= minutes; minute++)
            {
                var left = TimeSpan.FromMinutes(minute) - clock.Elapsed;
                if (left > TimeSpan.Zero && tray.WaitForExit(left))
                    throw new InvalidOperationException($"The tray exited after {clock.Elapsed.TotalSeconds:0} s (code {tray.ExitCode}).");
                tray.Refresh();
                if (minute == 1) cpuAfterFirstMinute = tray.TotalProcessorTime;
                say($"  minute {minute}: private {Mb(tray.PrivateMemorySize64)} MB, working set {Mb(tray.WorkingSet64)} MB, " +
                    $"cpu {tray.TotalProcessorTime.TotalSeconds:0.00} s");
            }
            return new IdleResult(tray.PrivateMemorySize64, tray.WorkingSet64, BaselineMath.CpuSecondsPerHour(
                cpuAfterFirstMinute, tray.TotalProcessorTime, TimeSpan.FromMinutes(minutes - 1)));
        }
        finally
        {
            Stop(tray);
        }
    }

    /// <summary>The bridge's whole run, launch to exit, on one status line: the median of <paramref name="count"/>.</summary>
    public TimeSpan? Bridge(string exe, int count)
    {
        var times = new List<TimeSpan>();
        for (var i = 0; i < count; i++)
        {
            var info = new ProcessStartInfo(exe)
            {
                UseShellExecute = false, RedirectStandardInput = true, RedirectStandardOutput = true, WorkingDirectory = scratch,
            };
            info.Environment["SOBREIRO_DATA_DIR"] = Folder("bridge-data");
            var clock = Stopwatch.StartNew();
            using var bridge = Process.Start(info) ?? throw new InvalidOperationException("The bridge did not start.");
            bridge.StandardInput.Write(StatusLine);
            bridge.StandardInput.Close();
            bridge.StandardOutput.ReadToEnd();
            bridge.WaitForExit();
            times.Add(clock.Elapsed);
        }
        say("  bridge runs, ms: " + string.Join(" ", times.Select(t => t.TotalMilliseconds.ToString("0"))));
        return BaselineMath.Median(times);
    }

    private static string Mb(long bytes) => (bytes / (1024.0 * 1024)).ToString("0.0");
}
