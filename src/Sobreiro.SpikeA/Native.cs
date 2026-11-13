using System.Runtime.InteropServices;
using Sobreiro.Core.Display;

namespace Sobreiro.SpikeA;

/// <summary>SPIKE (shell candidate A): every Win32 call the spike makes, in one place so they can be counted.</summary>
internal static partial class Native
{
    public delegate void WinEventProc(IntPtr hook, uint eventId, IntPtr hwnd, int idObject, int idChild, uint thread, uint time);

    public const uint EventSystemForeground = 0x0003;
    public const uint EventObjectLocationChange = 0x800B;
    public const uint WinEventOutOfContext = 0;
    public const uint WinEventSkipOwnProcess = 2;
    public const int GwlpHwndParent = -8;
    public const int GwlExStyle = -20;
    public const long WsExToolWindow = 0x80;
    public const long WsExNoActivate = 0x08000000;
    public const long WsExLayered = 0x00080000;
    public const long WsExNoRedirectionBitmap = 0x00200000;
    public const uint SwpNoSize = 0x1;
    public const uint SwpNoZOrder = 0x4;
    public const uint SwpNoActivate = 0x10;
    public const uint SwpShowWindow = 0x40;
    public const uint SwpNoOwnerZOrder = 0x200;
    public const int SwHide = 0;
    public const uint GwHwndPrev = 3;
    public const uint GwOwner = 4;
    public const uint WmDisplayChange = 0x007E;
    public const uint WmLButtonUp = 0x0202;
    public const uint WmRButtonUp = 0x0205;
    public static readonly IntPtr HwndTopmost = new(-1);
    private const uint MonitorDefaultToPrimary = 1;
    private const uint MonitorDefaultToNearest = 2;

    [StructLayout(LayoutKind.Sequential)]
    public struct Rect
    {
        public int Left, Top, Right, Bottom;
        public readonly PixelRect ToPixelRect() => new(Left, Top, Right, Bottom);
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct Point32
    {
        public int X, Y;
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

    public static PixelRect? WindowRect(IntPtr hwnd) => GetWindowRect(hwnd, out var r) ? r.ToPixelRect() : null;

    public static PixelRect? MonitorRect(IntPtr hwnd)
    {
        var monitor = MonitorFromWindow(hwnd, MonitorDefaultToNearest);
        var info = new MonitorInfo { Size = Marshal.SizeOf<MonitorInfo>() };
        return monitor != IntPtr.Zero && GetMonitorInfo(monitor, ref info) ? info.Monitor.ToPixelRect() : null;
    }

    /// <summary>The monitor under a point (else the primary one): its bounds, its work area and its DPI.</summary>
    public static (PixelRect Monitor, PixelRect Work, double Dpi)? ScreenAt(int x, int y)
    {
        foreach (var fallback in new[] { MonitorDefaultToNearest, MonitorDefaultToPrimary })
        {
            var monitor = MonitorFromPoint(new Point32 { X = x, Y = y }, fallback);
            var info = new MonitorInfo { Size = Marshal.SizeOf<MonitorInfo>() };
            if (monitor == IntPtr.Zero || !GetMonitorInfo(monitor, ref info)) continue;
            var dpi = GetDpiForMonitor(monitor, 0, out var dpiX, out _) == 0 ? dpiX : 96u;
            return (info.Monitor.ToPixelRect(), info.Work.ToPixelRect(), dpi);
        }
        return null;
    }

    public static int? NotificationState() => SHQueryUserNotificationState(out var state) == 0 ? state : null;

    public static long Style(IntPtr hwnd) => GetWindowLongPtr(hwnd, -16).ToInt64();

    public static long ExStyle(IntPtr hwnd) => GetWindowLongPtr(hwnd, GwlExStyle).ToInt64();

    public static void AddExStyle(IntPtr hwnd, long bits) => SetWindowLongPtr(hwnd, GwlExStyle, new IntPtr(ExStyle(hwnd) | bits));

    /// <summary>Makes <paramref name="owner"/> the owner of a top-level window that already exists.</summary>
    public static void SetOwner(IntPtr hwnd, IntPtr owner) => SetWindowLongPtr(hwnd, GwlpHwndParent, owner);

    public static Point32 Cursor() => GetCursorPos(out var p) ? p : default;

    public static int SmallIconSize() => GetSystemMetrics(49) is > 0 and var size ? size : 16;

    [LibraryImport("user32.dll")]
    private static partial int GetSystemMetrics(int index);

    [LibraryImport("user32.dll", EntryPoint = "FindWindowW", StringMarshalling = StringMarshalling.Utf16)]
    private static partial IntPtr FindWindow(string? className, string? windowName);

    [LibraryImport("user32.dll", EntryPoint = "FindWindowExW", StringMarshalling = StringMarshalling.Utf16)]
    private static partial IntPtr FindWindowEx(IntPtr parent, IntPtr after, string? className, string? windowName);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool GetWindowRect(IntPtr hwnd, out Rect rect);

    [LibraryImport("user32.dll")]
    public static partial uint GetDpiForWindow(IntPtr hwnd);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool IsWindow(IntPtr hwnd);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool IsWindowVisible(IntPtr hwnd);

    [LibraryImport("user32.dll", EntryPoint = "RegisterWindowMessageW", StringMarshalling = StringMarshalling.Utf16)]
    public static partial uint RegisterWindowMessage(string name);

    [LibraryImport("user32.dll")]
    public static partial uint GetWindowThreadProcessId(IntPtr hwnd, out uint processId);

    [LibraryImport("user32.dll")]
    public static partial IntPtr SetWinEventHook(uint min, uint max, IntPtr module, IntPtr proc, uint processId, uint threadId, uint flags);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool UnhookWinEvent(IntPtr hook);

    [LibraryImport("user32.dll")]
    private static partial IntPtr MonitorFromWindow(IntPtr hwnd, uint flags);

    [LibraryImport("user32.dll")]
    private static partial IntPtr MonitorFromPoint(Point32 point, uint flags);

    [LibraryImport("user32.dll", EntryPoint = "GetMonitorInfoW")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool GetMonitorInfo(IntPtr monitor, ref MonitorInfo info);

    [LibraryImport("shcore.dll")]
    private static partial int GetDpiForMonitor(IntPtr monitor, int dpiType, out uint dpiX, out uint dpiY);

    [LibraryImport("shell32.dll")]
    private static partial int SHQueryUserNotificationState(out int state);

    [LibraryImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool SetWindowPos(IntPtr hwnd, IntPtr insertAfter, int x, int y, int cx, int cy, uint flags);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static partial bool ShowWindow(IntPtr hwnd, int command);

    [LibraryImport("user32.dll")]
    public static partial IntPtr GetWindow(IntPtr hwnd, uint command);

    [LibraryImport("user32.dll", EntryPoint = "GetWindowLongPtrW")]
    private static partial IntPtr GetWindowLongPtr(IntPtr hwnd, int index);

    [LibraryImport("user32.dll", EntryPoint = "SetWindowLongPtrW")]
    private static partial IntPtr SetWindowLongPtr(IntPtr hwnd, int index, IntPtr value);

    [LibraryImport("user32.dll")]
    public static partial IntPtr GetForegroundWindow();

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool GetCursorPos(out Point32 point);
}
