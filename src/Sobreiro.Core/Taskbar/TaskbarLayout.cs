using Sobreiro.Core.Display;

namespace Sobreiro.Core.Taskbar;

/// <summary>Offsets in physical pixels from the taskbar window's left (TrayLeft, ButtonsRight) and top (BandTop, BandBottom).</summary>
public readonly record struct TaskbarAnchors(int TrayLeft, int ButtonsRight, int BandTop, int BandBottom);

public static class TaskbarLayout
{
    public const string TrayClassPrefix = "SystemTray.";

    /// <summary>
    /// Turns UI Automation rectangles into anchors. <paramref name="items"/> are the children of the taskbar's
    /// InputSite pane, <paramref name="buttons"/> the children of Taskbar.TaskbarFrameAutomationPeer.
    /// Items of <paramref name="ownProcessId"/> are ignored. Null when there are no tray items.
    /// </summary>
    public static TaskbarAnchors? Anchors(PixelRect taskbar, IEnumerable<(string ClassName, PixelRect Rect, int ProcessId)> items, IEnumerable<PixelRect> buttons, int ownProcessId)
    {
        // Our own windows (the pill, which the taskbar owns, so UI Automation lists it) are never tray items or obstacles.
        var all = items.Where(i => i.ProcessId != ownProcessId).ToList();
        var tray = all
            .Where(i => i.ClassName.StartsWith(TrayClassPrefix, StringComparison.Ordinal) && i.Rect.Width > 0 && i.Rect.Height > 0)
            .Select(i => i.Rect)
            .ToList();
        if (tray.Count == 0) return null;

        var buttonsRight = 0;
        foreach (var b in buttons)
            if (b.Width > 0 && b.Height > 0)
                buttonsRight = Math.Max(buttonsRight, b.Right - taskbar.Left);

        // Other items (a Widgets button, an input indicator) left of the tray are obstacles too; panes that
        // reach past the tray's left edge (the whole-taskbar InputSite, the tray's own pane) are not.
        var trayLeft = tray.Min(r => r.Left);
        foreach (var (className, rect, _) in all)
            if (!className.StartsWith(TrayClassPrefix, StringComparison.Ordinal)
                && rect.Width > 0 && rect.Height > 0 && rect.Right <= trayLeft)
                buttonsRight = Math.Max(buttonsRight, rect.Right - taskbar.Left);

        return new TaskbarAnchors(
            trayLeft - taskbar.Left,
            buttonsRight,
            tray.Min(r => r.Top) - taskbar.Top,
            tray.Max(r => r.Bottom) - taskbar.Top);
    }
}
