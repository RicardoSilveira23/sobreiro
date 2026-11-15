using System.Runtime.InteropServices;
using Sobreiro.Core.Display;

namespace Sobreiro.Pill;

/// <summary>The Win32 calls the tracker needs. <see cref="PillWindow"/> keeps its own.</summary>
internal static class TaskbarNative
{
    public delegate void WinEventProc(IntPtr hook, uint eventId, IntPtr hwnd, int idObject, int idChild, uint thread, uint time);

    public const uint EventSystemForeground = 0x0003;
    public const uint EventObjectLocationChange = 0x800B;
    public const uint WinEventOutOfContext = 0;
    public const uint WinEventSkipOwnProcess = 2;
    private const uint MonitorDefaultToNearest = 2;
    private const int ShqueryOk = 0;

    [StructLayout(LayoutKind.Sequential)]
    private struct Rect
    {
        public int Left, Top, Right, Bottom;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct MonitorInfo
    {
        public int Size;
        public Rect Monitor;
        public Rect Work;
        public uint Flags;
    }

    public static IntPtr FindTaskbar() => FindWindow("Shell_TrayWnd", null);

    public static IntPtr FindTrayNotify(IntPtr taskbar) => FindWindowEx(taskbar, IntPtr.Zero, "TrayNotifyWnd", null);

    public static PixelRect? WindowRect(IntPtr hwnd) =>
        GetWindowRect(hwnd, out var r) ? new PixelRect(r.Left, r.Top, r.Right, r.Bottom) : null;

    public static PixelRect? MonitorRect(IntPtr hwnd)
    {
        var monitor = MonitorFromWindow(hwnd, MonitorDefaultToNearest);
        var info = new MonitorInfo { Size = Marshal.SizeOf<MonitorInfo>() };
        return monitor != IntPtr.Zero && GetMonitorInfo(monitor, ref info)
            ? new PixelRect(info.Monitor.Left, info.Monitor.Top, info.Monitor.Right, info.Monitor.Bottom)
            : null;
    }

    /// <summary>The shell's notification state, or null when the query fails.</summary>
    public static int? NotificationState() => SHQueryUserNotificationState(out var state) == ShqueryOk ? state : null;

    public static (uint ProcessId, uint ThreadId) ProcessAndThread(IntPtr hwnd)
    {
        var thread = GetWindowThreadProcessId(hwnd, out var pid);
        return (pid, thread);
    }

    public static IntPtr HookEvents(uint eventId, WinEventProc proc, uint processId, uint threadId, uint flags) =>
        SetWinEventHook(eventId, eventId, IntPtr.Zero, proc, processId, threadId, flags);

    public static uint RegisterMessage(string name) => RegisterWindowMessage(name);

    public static int DpiOf(IntPtr hwnd) => (int)GetDpiForWindow(hwnd);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr FindWindow(string? className, string? windowName);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr FindWindowEx(IntPtr parent, IntPtr after, string? className, string? windowName);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetWindowRect(IntPtr hwnd, out Rect rect);

    [DllImport("user32.dll")]
    private static extern uint GetDpiForWindow(IntPtr hwnd);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool IsWindow(IntPtr hwnd);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern uint RegisterWindowMessage(string name);

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint processId);

    [DllImport("user32.dll")]
    private static extern IntPtr SetWinEventHook(uint min, uint max, IntPtr module, WinEventProc proc, uint processId, uint threadId, uint flags);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool UnhookWinEvent(IntPtr hook);

    [DllImport("user32.dll")]
    private static extern IntPtr MonitorFromWindow(IntPtr hwnd, uint flags);

    [DllImport("user32.dll", EntryPoint = "GetMonitorInfoW")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetMonitorInfo(IntPtr monitor, ref MonitorInfo info);

    [DllImport("shell32.dll")]
    private static extern int SHQueryUserNotificationState(out int state);
}
