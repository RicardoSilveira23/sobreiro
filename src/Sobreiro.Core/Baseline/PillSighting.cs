using Sobreiro.Core.Display;

namespace Sobreiro.Core.Baseline;

/// <summary>What the baseline tool reads about one top-level window of the app it measures.</summary>
public readonly record struct WindowFacts(bool Visible, nint Owner, PixelRect Rect);

public static class PillSighting
{
    /// <summary>
    /// True when the window is a pill someone can see: visible, owned by the taskbar, not empty and
    /// wholly inside the taskbar. A pill still parked off screen, waiting to be placed, does not count,
    /// and neither does a menu or flyout that opens from the taskbar and reaches above it.
    /// </summary>
    public static bool IsPill(WindowFacts window, nint taskbar, PixelRect taskbarRect) =>
        taskbar != 0
        && window.Visible
        && window.Owner == taskbar
        && window.Rect.Width > 0 && window.Rect.Height > 0
        && window.Rect.Left >= taskbarRect.Left && window.Rect.Right <= taskbarRect.Right
        && window.Rect.Top >= taskbarRect.Top && window.Rect.Bottom <= taskbarRect.Bottom;
}
