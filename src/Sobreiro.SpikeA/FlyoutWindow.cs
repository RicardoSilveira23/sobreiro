using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Media;
using Avalonia.Threading;
using Sobreiro.Core.Display;

namespace Sobreiro.SpikeA;

/// <summary>
/// SPIKE (shell candidate A): the borderless toolkit window that holds the flyout card. It hides on Esc and
/// when it loses focus, and is placed by Core beside the point it was opened from.
/// </summary>
internal sealed class FlyoutWindow : Window
{
    private const long HideGraceMs = 300;
    private long? _hiddenAt;
    private int _cursorX;
    private int _cursorY;

    public FlyoutWindow()
    {
        Title = "Sobreiro";
        WindowDecorations = WindowDecorations.None;
        Background = Brushes.Transparent;
        TransparencyLevelHint = [WindowTransparencyLevel.Transparent];
        ShowInTaskbar = false;
        ShowActivated = true;
        Topmost = true;
        CanResize = false;
        SizeToContent = SizeToContent.WidthAndHeight;
        WindowStartupLocation = WindowStartupLocation.Manual;

        Deactivated += (_, _) =>
        {
            if (!IsVisible) return;
            Hide();
            _hiddenAt = Environment.TickCount64;
            Spike.Trace("flyout hidden: lost focus");
        };
        KeyDown += (_, e) =>
        {
            if (e.Key != Key.Escape) return;
            e.Handled = true;
            Hide();
        };
        ScalingChanged += (_, _) => Dispatcher.UIThread.Post(Place);
    }

    public bool HiddenJustNow => _hiddenAt is { } at && Environment.TickCount64 - at < HideGraceMs;

    public void Render(Control card)
    {
        Content = card;
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
            Position = new PixelPoint(-32000, -32000);
            Show();
            Native.AddExStyle(Handle, Native.WsExToolWindow);
        }
        UpdateLayout();
        Place();
        Activate();
        // Once more after the toolkit has settled its own size.
        Dispatcher.UIThread.Post(Place, DispatcherPriority.Loaded);
    }

    private IntPtr Handle => TryGetPlatformHandle()?.Handle ?? IntPtr.Zero;

    private void Place()
    {
        if (!IsVisible || Native.ScreenAt(_cursorX, _cursorY) is not { } screen || Native.WindowRect(Handle) is not { } own) return;
        var scale = screen.Dpi / 96.0;
        var (x, y) = FlyoutPlacement.Place(screen.Monitor, screen.Work, _cursorX, _cursorY, own.Width, own.Height, (int)Math.Round(12 * scale));
        Native.SetWindowPos(Handle, Native.HwndTopmost, x, y, 0, 0, Native.SwpNoSize | Native.SwpNoActivate);
        Spike.Trace($"flyout placed at {x},{y} size {own.Width}x{own.Height} scale {RenderScaling}");
    }
}
