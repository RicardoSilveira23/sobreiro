using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Media;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Display;
using Sobreiro.Core.Taskbar;

namespace Sobreiro.SpikeA;

/// <summary>
/// SPIKE (shell candidate A): the toolkit's window that holds the pill. Borderless, clear per pixel, owned by
/// the taskbar, never taking focus, placed in physical pixels by the tracker with the same native calls as today.
/// </summary>
internal sealed class PillWindow : Window
{
    private readonly IntPtr _owner;
    private bool _shown;
    private PillModel? _lastModel;
    private TaskbarColours? _lastColours;
    private string? _lastTooltip;

    public PillWindow(IntPtr owner)
    {
        Title = "Sobreiro pill";
        WindowDecorations = WindowDecorations.None;
        Background = Brushes.Transparent;
        TransparencyLevelHint = [WindowTransparencyLevel.Transparent];
        ShowInTaskbar = false;
        ShowActivated = false;
        Topmost = true;
        CanResize = false;
        Focusable = false;
        SizeToContent = SizeToContent.WidthAndHeight;
        WindowStartupLocation = WindowStartupLocation.Manual;
        Position = new PixelPoint(-32000, -32000);
        _owner = owner;

        PointerReleased += (_, e) =>
        {
            if (e.InitialPressMouseButton == MouseButton.Left) LeftClicked?.Invoke();
            else if (e.InitialPressMouseButton == MouseButton.Right) RightClicked?.Invoke();
        };
    }

    public IntPtr Handle => TryGetPlatformHandle()?.Handle ?? IntPtr.Zero;

    public event Action? LeftClicked;

    public event Action? RightClicked;

    /// <summary>The window's size in physical pixels, as Windows reports it.</summary>
    public (int Width, int Height) PixelSize => Native.WindowRect(Handle) is { } r ? (r.Width, r.Height) : (0, 0);

    /// <summary>Swaps the face. The first call shows the window at its off-screen start, without activating it.</summary>
    public void Render(PillModel model, TaskbarColours colours, string tooltip)
    {
        if (_shown && model == _lastModel && colours == _lastColours && tooltip == _lastTooltip) return;
        _lastModel = model;
        _lastColours = colours;
        _lastTooltip = tooltip;
        var face = PillView.Build(model, colours);
        ToolTip.SetTip(face, tooltip);
        ToolTip.SetPlacement(face, PlacementMode.Top);
        ToolTip.SetVerticalOffset(face, -12);
        Content = face;
        if (!_shown)
        {
            _shown = true;
            Show();
            // The toolkit sets the owner and the extended style itself when it shows a window: ours go after.
            var hwnd = Handle;
            Native.SetOwner(hwnd, _owner);
            Native.AddExStyle(hwnd, Native.WsExToolWindow | Native.WsExNoActivate);
            Spike.Trace($"pill window 0x{hwnd:x} shown, owner-is-taskbar={Native.GetWindow(hwnd, Native.GwOwner) == _owner} exstyle=0x{Native.ExStyle(hwnd):x} scale={RenderScaling}");
        }
        UpdateLayout();
    }

    /// <summary>Moves the window to (x, y) in screen pixels, above the taskbar, and shows it.</summary>
    public void MoveTo(int x, int y)
    {
        var hwnd = Handle;
        // The toolkit rewrites the extended style when it changes a window: put ours back if it did.
        const long ours = Native.WsExToolWindow | Native.WsExNoActivate;
        if ((Native.ExStyle(hwnd) & ours) != ours) Native.AddExStyle(hwnd, ours);
        if (Native.GetWindow(hwnd, Native.GwOwner) != _owner) Native.SetOwner(hwnd, _owner);

        var visible = Native.IsWindowVisible(hwnd);
        var inPlace = Native.GetWindowRect(hwnd, out var r) && r.Left == x && r.Top == y;
        var aboveTaskbar = ZOrder.IsAbove(hwnd, _owner, w => Native.GetWindow(w, Native.GwHwndPrev));
        switch (PillShow.Step(visible, aboveTaskbar, inPlace))
        {
            case PillShowStep.Raise:
                Native.SetWindowPos(hwnd, Native.HwndTopmost, x, y, 0, 0,
                    Native.SwpNoOwnerZOrder | Native.SwpNoSize | Native.SwpNoActivate | Native.SwpShowWindow);
                Spike.Trace($"pill raised to {x},{y} size {PixelSize.Width}x{PixelSize.Height} dpi {Native.GetDpiForWindow(hwnd)}");
                break;
            case PillShowStep.Move:
                Native.SetWindowPos(hwnd, IntPtr.Zero, x, y, 0, 0,
                    Native.SwpNoZOrder | Native.SwpNoOwnerZOrder | Native.SwpNoSize | Native.SwpNoActivate | Native.SwpShowWindow);
                Spike.Trace($"pill moved to {x},{y} size {PixelSize.Width}x{PixelSize.Height} dpi {Native.GetDpiForWindow(hwnd)}");
                break;
        }
    }

    /// <summary>Hides the window behind the toolkit's back, as the tracker does today.</summary>
    public void HideNow() => Native.ShowWindow(Handle, Native.SwHide);
}
