using System.Runtime.InteropServices;
using Sobreiro.Core.Baseline;
using Sobreiro.Core.Display;

namespace Sobreiro.Baseline;

/// <summary>The few window calls the tool needs: it only looks, except for <see cref="AskToQuit"/>.</summary>
internal static class Native
{
    private const uint GwOwner = 4;
    private const uint WmQueryEndSession = 0x0011;

    public static (nint Handle, PixelRect Rect)? Taskbar()
    {
        var taskbar = FindWindowW("Shell_TrayWnd", null);
        return taskbar != 0 && GetWindowRect(taskbar, out var r) ? (taskbar, new PixelRect(r.Left, r.Top, r.Right, r.Bottom)) : null;
    }

    /// <summary>Every top-level window, hidden ones included, with the process it belongs to.</summary>
    private static List<(nint Handle, int ProcessId, WindowFacts Facts)> Windows()
    {
        var found = new List<(nint, int, WindowFacts)>();
        EnumWindows((hwnd, _) =>
        {
            GetWindowThreadProcessId(hwnd, out var processId);
            if (GetWindowRect(hwnd, out var r))
                found.Add((hwnd, (int)processId, new WindowFacts(IsWindowVisible(hwnd), GetWindow(hwnd, GwOwner), new PixelRect(r.Left, r.Top, r.Right, r.Bottom))));
            return true;
        }, 0);
        return found;
    }

    public static List<(nint Handle, WindowFacts Facts)> WindowsOf(int processId) =>
        Windows().Where(w => w.ProcessId == processId).Select(w => (w.Handle, w.Facts)).ToList();

    public static bool PillShown(int processId) =>
        Taskbar() is { } taskbar && WindowsOf(processId).Any(w => PillSighting.IsPill(w.Facts, taskbar.Handle, taskbar.Rect));

    /// <summary>
    /// The process of a pill that is already in the taskbar, if any: one that isn't Explorer's own. A
    /// second pill finds no room beside it and stays hidden, so nothing can be timed while it is there.
    /// </summary>
    public static int? PillOfAnotherApp()
    {
        if (Taskbar() is not { } taskbar) return null;
        GetWindowThreadProcessId(taskbar.Handle, out var explorer);
        foreach (var w in Windows())
            if (w.ProcessId != (int)explorer && PillSighting.IsPill(w.Facts, taskbar.Handle, taskbar.Rect)) return w.ProcessId;
        return null;
    }

    /// <summary>
    /// Asks the app to leave the way Windows does when a session ends, so it takes its tray icon
    /// with it. An app that doesn't answer is killed by the caller.
    /// </summary>
    public static void AskToQuit(int processId)
    {
        foreach (var (hwnd, _) in WindowsOf(processId)) PostMessageW(hwnd, WmQueryEndSession, 0, 0);
    }

    /// <summary>Sleeps are a millisecond long, not the default 15, while the returned value lives.</summary>
    public static IDisposable FineTimer()
    {
        timeBeginPeriod(1);
        return new TimerPeriod();
    }

    private sealed class TimerPeriod : IDisposable
    {
        public void Dispose() => timeEndPeriod(1);
    }

    private delegate bool EnumProc(nint hwnd, nint lParam);

    [StructLayout(LayoutKind.Sequential)]
    private struct NativeRect
    {
        public int Left, Top, Right, Bottom;
    }

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern nint FindWindowW(string className, string? windowName);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool EnumWindows(EnumProc callback, nint lParam);

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(nint hwnd, out uint processId);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindowVisible(nint hwnd);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetWindowRect(nint hwnd, out NativeRect rect);

    [DllImport("user32.dll")]
    private static extern nint GetWindow(nint hwnd, uint command);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool PostMessageW(nint hwnd, uint message, nint wParam, nint lParam);

    [DllImport("winmm.dll")]
    private static extern uint timeBeginPeriod(uint period);

    [DllImport("winmm.dll")]
    private static extern uint timeEndPeriod(uint period);
}
