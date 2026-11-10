using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Interop;
using System.Windows.Media;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Display;
using Sobreiro.Core.Taskbar;
using Brushes = System.Windows.Media.Brushes;

namespace Sobreiro.Pill;

/// <summary>
/// The small borderless window over the taskbar that holds the pill. It is owned by the taskbar,
/// never takes focus, and is placed in physical pixels by the tracker.
/// </summary>
internal sealed class PillWindow : Window
{
    private const double OffScreen = -32000;
    private readonly IntPtr _owner;
    private bool _shown;
    private double _tooltipOffset;
    private bool _hasTooltipOffset;
    private PillModel? _lastModel;
    private TaskbarColours? _lastColours;
    private string? _lastTooltip;

    public PillWindow(IntPtr owner)
    {
        Title = "Sobreiro pill";
        WindowStyle = WindowStyle.None;
        AllowsTransparency = true;
        Background = Brushes.Transparent;
        ShowInTaskbar = false;
        ShowActivated = false;
        Topmost = true;
        Focusable = false;
        ResizeMode = ResizeMode.NoResize;
        SizeToContent = SizeToContent.WidthAndHeight;
        WindowStartupLocation = WindowStartupLocation.Manual;
        Left = OffScreen;
        Top = OffScreen;
        _owner = owner;
        new WindowInteropHelper(this).Owner = owner;

        MouseLeftButtonUp += (_, _) => LeftClicked?.Invoke();
        MouseRightButtonUp += (_, _) => RightClicked?.Invoke();
    }

    public IntPtr Handle => new WindowInteropHelper(this).Handle;

    public event Action? LeftClicked;

    public event Action? RightClicked;

    /// <summary>The window's DIP size at its own DPI, rounded up to whole pixels. Valid after the first <see cref="Render"/>.</summary>
    public (int Width, int Height) PixelSize
    {
        get
        {
            var dpi = VisualTreeHelper.GetDpi(this);
            return ((int)Math.Ceiling(ActualWidth * dpi.DpiScaleX), (int)Math.Ceiling(ActualHeight * dpi.DpiScaleY));
        }
    }

    /// <summary>
    /// Swaps the face and leaves the window laid out and ready to measure and move: the first call
    /// shows it at its off-screen start location, without activating it.
    /// </summary>
    public void Render(PillModel model, TaskbarColours colours, string tooltip)
    {
        // Same face as last time: keep it, so a hover highlight or an open tooltip is not reset.
        if (_shown && model == _lastModel && colours == _lastColours && tooltip == _lastTooltip) return;
        _lastModel = model;
        _lastColours = colours;
        _lastTooltip = tooltip;
        var face = PillView.Build(model, colours);
        face.ToolTip = tooltip;
        ApplyTooltipPlacement(face);
        Content = face;
        // Only the first time: after HideNow the tracker decides when it shows again.
        if (!_shown)
        {
            _shown = true;
            Show();
        }
        UpdateLayout();
    }

    /// <summary>
    /// Moves the window to (x, y) in screen pixels, above the taskbar, and shows it. The tooltip opens
    /// wholly above the taskbar band (<paramref name="bandTop"/>, screen pixels), where the taskbar
    /// cannot cover it. Valid after the first <see cref="Render"/>.
    /// </summary>
    public void MoveTo(int x, int y, int bandTop)
    {
        // Owned windows don't follow their owner while hidden: after a full-screen window closes, Explorer
        // has raised the taskbar past the hidden pill, which would show again under it. So a hidden pill, or
        // one below the taskbar, goes back to the top of the topmost band; SWP_NOOWNERZORDER leaves the
        // taskbar where it is. Otherwise no z-order change, and no call at all when nothing moves.
        var hwnd = Handle;
        var visible = IsWindowVisible(hwnd);
        var inPlace = GetWindowRect(hwnd, out var r) && r.Left == x && r.Top == y;
        var aboveTaskbar = ZOrder.IsAbove(hwnd, _owner, w => GetWindow(w, GwHwndPrev));
        switch (PillShow.Step(visible, aboveTaskbar, inPlace))
        {
            case PillShowStep.Raise:
                SetWindowPos(hwnd, HwndTopmost, x, y, 0, 0, SwpNoOwnerZOrder | SwpNoSize | SwpNoActivate | SwpShowWindow);
                break;
            case PillShowStep.Move:
                SetWindowPos(hwnd, IntPtr.Zero, x, y, 0, 0, SwpNoZOrder | SwpNoOwnerZOrder | SwpNoSize | SwpNoActivate | SwpShowWindow);
                break;
        }
        _tooltipOffset = -((y - bandTop) / VisualTreeHelper.GetDpi(this).DpiScaleY + 8);
        _hasTooltipOffset = true;
        if (Content is FrameworkElement face) ApplyTooltipPlacement(face);
    }

    private void ApplyTooltipPlacement(FrameworkElement face)
    {
        if (!_hasTooltipOffset) return;
        ToolTipService.SetPlacement(face, PlacementMode.Top);
        ToolTipService.SetPlacementTarget(face, face);
        ToolTipService.SetVerticalOffset(face, _tooltipOffset);
    }

    /// <summary>Hides the window. Valid after the first <see cref="Render"/>.</summary>
    public void HideNow() => ShowWindow(Handle, SwHide);

    protected override void OnSourceInitialized(EventArgs e)
    {
        base.OnSourceInitialized(e);
        var hwnd = Handle;
        var style = GetWindowLongPtr(hwnd, GwlExStyle).ToInt64();
        SetWindowLongPtr(hwnd, GwlExStyle, new IntPtr(style | WsExToolWindow | WsExNoActivate));
    }

    private const int GwlExStyle = -20;
    private const long WsExToolWindow = 0x80;
    private const long WsExNoActivate = 0x08000000;
    private const uint SwpNoSize = 0x1;
    private const uint SwpNoZOrder = 0x4;
    private const uint SwpNoActivate = 0x10;
    private const uint SwpNoOwnerZOrder = 0x200;
    private const uint SwpShowWindow = 0x40;
    private const int SwHide = 0;
    private const uint GwHwndPrev = 3;
    private static readonly IntPtr HwndTopmost = new(-1);

    [StructLayout(LayoutKind.Sequential)]
    private struct NativeRect
    {
        public int Left, Top, Right, Bottom;
    }

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindowVisible(IntPtr hwnd);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetWindowRect(IntPtr hwnd, out NativeRect rect);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetWindowPos(IntPtr hwnd, IntPtr insertAfter, int x, int y, int cx, int cy, uint flags);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ShowWindow(IntPtr hwnd, int command);

    [DllImport("user32.dll")]
    private static extern IntPtr GetWindow(IntPtr hwnd, uint command);

    [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW")]
    private static extern IntPtr GetWindowLongPtr(IntPtr hwnd, int index);

    [DllImport("user32.dll", EntryPoint = "SetWindowLongPtrW")]
    private static extern IntPtr SetWindowLongPtr(IntPtr hwnd, int index, IntPtr value);
}
