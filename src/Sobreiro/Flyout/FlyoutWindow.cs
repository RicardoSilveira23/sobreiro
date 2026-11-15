using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Input;
using System.Windows.Interop;
using System.Windows.Media;
using System.Windows.Threading;
using Sobreiro.Core.Display;
using Brushes = System.Windows.Media.Brushes;
using KeyEventArgs = System.Windows.Input.KeyEventArgs;

namespace Sobreiro.Flyout;

/// <summary>
/// The borderless window that holds the flyout card above the tray. It hides on Esc and when it
/// loses focus; it never shows in the taskbar or in Alt+Tab.
/// </summary>
public sealed class FlyoutWindow : Window
{
    private const long HideGraceMs = 300;
    private const double OffScreen = -32000;
    private long? _hiddenAt;
    private int _cursorX;
    private int _cursorY;

    public FlyoutWindow()
    {
        WindowStyle = WindowStyle.None;
        AllowsTransparency = true;
        Background = Brushes.Transparent;
        ShowInTaskbar = false;
        ShowActivated = false;
        Topmost = true;
        ResizeMode = ResizeMode.NoResize;
        SizeToContent = SizeToContent.WidthAndHeight;
        WindowStartupLocation = WindowStartupLocation.Manual;
        Title = "Sobreiro";

        Deactivated += (_, _) =>
        {
            if (!IsVisible) return;
            Hide();
            _hiddenAt = Environment.TickCount64;
        };
        DpiChanged += (_, _) => Dispatcher.InvokeAsync(Place, DispatcherPriority.Loaded);
    }

    /// <summary>
    /// True for 300 ms after the window hid itself on losing focus, so a click on the tray icon that
    /// took the focus away closes the flyout instead of opening it again.
    /// </summary>
    public bool HiddenJustNow => _hiddenAt is { } at && Environment.TickCount64 - at < HideGraceMs;

    public void Render(FlyoutModel model, FlyoutStyle style, FlyoutColours colours)
    {
        Content = FlyoutView.Build(model, style, colours);
        if (!IsVisible) return;
        UpdateLayout();
        Place();
    }

    /// <summary>Shows the window next to the tray, on the monitor under the cursor (screen pixels).</summary>
    public void ShowNear(int cursorX, int cursorY)
    {
        _cursorX = cursorX;
        _cursorY = cursorY;
        if (!IsVisible)
        {
            Left = OffScreen;
            Top = OffScreen;
            Show();
        }
        UpdateLayout();
        Place();
        Activate();
    }

    protected override void OnSourceInitialized(EventArgs e)
    {
        base.OnSourceInitialized(e);
        var hwnd = new WindowInteropHelper(this).Handle;
        SetExStyle(hwnd, GetExStyle(hwnd) | WsExToolWindow);
    }

    protected override void OnKeyDown(KeyEventArgs e)
    {
        base.OnKeyDown(e);
        if (e.Key != Key.Escape) return;
        e.Handled = true;
        Hide();
    }

    // The DIP size in pixels at the DPI of the monitor under the cursor, placed by Core.
    private void Place()
    {
        if (!IsVisible) return;
        var (monitor, work, dpi) = Screen();
        var scale = dpi / 96.0;
        var (x, y) = FlyoutPlacement.Place(
            monitor, work, _cursorX, _cursorY,
            (int)Math.Ceiling(ActualWidth * scale), (int)Math.Ceiling(ActualHeight * scale),
            (int)Math.Round(12 * scale));

        if (SetWindowPos(new WindowInteropHelper(this).Handle, HwndTopmost, x, y, 0, 0, SwpNoSize | SwpNoActivate))
            return;
        // Let WPF move it instead, in DIPs at the window's current DPI.
        var current = VisualTreeHelper.GetDpi(this);
        Left = x / current.DpiScaleX;
        Top = y / current.DpiScaleY;
    }

    // The monitor under the cursor, else the primary one, else WPF's primary work area at the
    // window's DPI: the flyout always lands on a screen.
    private (PixelRect Monitor, PixelRect Work, double Dpi) Screen()
    {
        var point = new Point32 { X = _cursorX, Y = _cursorY };
        foreach (var fallback in new[] { MonitorDefaultToNearest, MonitorDefaultToPrimary })
        {
            var monitor = MonitorFromPoint(point, fallback);
            var info = new MonitorInfo { Size = Marshal.SizeOf<MonitorInfo>() };
            if (monitor == IntPtr.Zero || !GetMonitorInfo(monitor, ref info)) continue;
            var dpi = GetDpiForMonitor(monitor, MdtEffectiveDpi, out var dpiX, out _) == 0 ? dpiX : 96u;
            return (info.Monitor.ToPixelRect(), info.Work.ToPixelRect(), dpi);
        }

        var scale = VisualTreeHelper.GetDpi(this);
        var area = SystemParameters.WorkArea;
        var work = new PixelRect(
            (int)Math.Round(area.Left * scale.DpiScaleX), (int)Math.Round(area.Top * scale.DpiScaleY),
            (int)Math.Round(area.Right * scale.DpiScaleX), (int)Math.Round(area.Bottom * scale.DpiScaleY));
        return (work, work, scale.PixelsPerInchX);
    }

    private const int GwlExStyle = -20;
    private const long WsExToolWindow = 0x80;
    private const uint MonitorDefaultToPrimary = 1;
    private const uint MonitorDefaultToNearest = 2;
    private const int MdtEffectiveDpi = 0;
    private const uint SwpNoSize = 0x1;
    private const uint SwpNoActivate = 0x10;
    private static readonly IntPtr HwndTopmost = new(-1);

    private static long GetExStyle(IntPtr hwnd) =>
        IntPtr.Size == 8 ? GetWindowLongPtr(hwnd, GwlExStyle).ToInt64() : GetWindowLong(hwnd, GwlExStyle);

    private static void SetExStyle(IntPtr hwnd, long style)
    {
        if (IntPtr.Size == 8) SetWindowLongPtr(hwnd, GwlExStyle, new IntPtr(style));
        else SetWindowLong(hwnd, GwlExStyle, (int)style);
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Point32
    {
        public int X;
        public int Y;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Rect32
    {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;

        public readonly PixelRect ToPixelRect() => new(Left, Top, Right, Bottom);
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct MonitorInfo
    {
        public int Size;
        public Rect32 Monitor;
        public Rect32 Work;
        public uint Flags;
    }

    [DllImport("user32.dll")]
    private static extern IntPtr MonitorFromPoint(Point32 point, uint flags);

    [DllImport("user32.dll", EntryPoint = "GetMonitorInfoW")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetMonitorInfo(IntPtr monitor, ref MonitorInfo info);

    [DllImport("shcore.dll")]
    private static extern int GetDpiForMonitor(IntPtr monitor, int dpiType, out uint dpiX, out uint dpiY);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetWindowPos(IntPtr hwnd, IntPtr insertAfter, int x, int y, int cx, int cy, uint flags);

    [DllImport("user32.dll", EntryPoint = "GetWindowLongW")]
    private static extern int GetWindowLong(IntPtr hwnd, int index);

    [DllImport("user32.dll", EntryPoint = "SetWindowLongW")]
    private static extern int SetWindowLong(IntPtr hwnd, int index, int value);

    [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW")]
    private static extern IntPtr GetWindowLongPtr(IntPtr hwnd, int index);

    [DllImport("user32.dll", EntryPoint = "SetWindowLongPtrW")]
    private static extern IntPtr SetWindowLongPtr(IntPtr hwnd, int index, IntPtr value);
}
