using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Threading;

namespace Sobreiro.SpikeA;

/// <summary>
/// SPIKE step 1, before any drawing: does the taskbar take a toolkit window? Borderless, transparent per
/// pixel, never taking focus, owned by the taskbar, placed inside it. Writes what it finds to the trace
/// file and leaves after 12 seconds.
/// </summary>
internal static class Probe
{
    public static void Run()
    {
        var taskbar = Native.FindTaskbar();
        var foregroundBefore = Native.GetForegroundWindow();
        var window = new Window
        {
            Title = "Sobreiro pill",
            WindowDecorations = WindowDecorations.None,
            Background = Brushes.Transparent,
            TransparencyLevelHint = [WindowTransparencyLevel.Transparent],
            ShowInTaskbar = false,
            ShowActivated = false,
            Topmost = true,
            CanResize = false,
            Focusable = false,
            SizeToContent = SizeToContent.WidthAndHeight,
            WindowStartupLocation = WindowStartupLocation.Manual,
            Position = new PixelPoint(-32000, -32000),
            // Half-clear red with a solid white dot: the taskbar must show through the red and around the corners.
            Content = new Border
            {
                Width = 140,
                Height = 36,
                CornerRadius = new CornerRadius(18),
                Background = new SolidColorBrush(Color.FromArgb(0x80, 0xE0, 0x30, 0x30)),
                Child = new Border
                {
                    Width = 16, Height = 16, CornerRadius = new CornerRadius(8), Background = Brushes.White,
                    HorizontalAlignment = HorizontalAlignment.Center, VerticalAlignment = VerticalAlignment.Center,
                },
            },
        };
        var hwnd = window.TryGetPlatformHandle()!.Handle;
        Spike.Trace($"probe render={(Spike.Gpu ? "gpu" : "software")} taskbar=0x{taskbar:x} hwnd=0x{hwnd:x} exstyle-at-birth=0x{Native.ExStyle(hwnd):x}");
        // The toolkit sets the owner and the extended style itself when it shows the window: ours go after.
        window.Show();
        Native.SetOwner(hwnd, taskbar);
        Native.AddExStyle(hwnd, Native.WsExToolWindow | Native.WsExNoActivate);

        DispatcherTimer.RunOnce(() =>
        {
            if (Native.WindowRect(taskbar) is not { } bar || Native.WindowRect(hwnd) is not { } own) return;
            var x = bar.Right - 760;
            var y = bar.Top + (bar.Height - own.Height) / 2;
            var moved = Native.SetWindowPos(hwnd, Native.HwndTopmost, x, y, 0, 0,
                Native.SwpNoOwnerZOrder | Native.SwpNoSize | Native.SwpNoActivate | Native.SwpShowWindow);
            Spike.Trace($"moved={moved} to {x},{y}; taskbar [{bar.Left},{bar.Top},{bar.Right},{bar.Bottom}]");
        }, TimeSpan.FromMilliseconds(500));

        DispatcherTimer.RunOnce(() =>
        {
            var own = Native.WindowRect(hwnd);
            var bar = Native.WindowRect(taskbar);
            var style = Native.ExStyle(hwnd);
            var inside = own is { } o && bar is { } b && o.Left >= b.Left && o.Right <= b.Right && o.Top >= b.Top && o.Bottom <= b.Bottom;
            var above = false;
            for (var w = Native.GetWindow(taskbar, Native.GwHwndPrev); w != IntPtr.Zero; w = Native.GetWindow(w, Native.GwHwndPrev))
                if (w == hwnd) above = true;
            Spike.Trace(
                $"owner-is-taskbar={Native.GetWindow(hwnd, Native.GwOwner) == taskbar} visible={Native.IsWindowVisible(hwnd)} " +
                $"rect=[{own?.Left},{own?.Top},{own?.Right},{own?.Bottom}] size={own?.Width}x{own?.Height} inside-taskbar={inside} above-taskbar={above} " +
                $"foreground-unchanged={Native.GetForegroundWindow() == foregroundBefore} " +
                $"style=0x{Native.Style(hwnd):x} exstyle=0x{style:x} layered={(style & Native.WsExLayered) != 0} no-redirection-bitmap={(style & Native.WsExNoRedirectionBitmap) != 0} " +
                $"toolwindow={(style & Native.WsExToolWindow) != 0} noactivate={(style & Native.WsExNoActivate) != 0} " +
                $"window-dpi={Native.GetDpiForWindow(hwnd)} taskbar-dpi={Native.GetDpiForWindow(taskbar)} avalonia-scale={window.RenderScaling}");
        }, TimeSpan.FromMilliseconds(2000));

        DispatcherTimer.RunOnce(
            () => ((IClassicDesktopStyleApplicationLifetime)Application.Current!.ApplicationLifetime!).Shutdown(),
            TimeSpan.FromSeconds(12));
    }
}
