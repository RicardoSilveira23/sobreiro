using Sobreiro.Core.Display;

namespace Sobreiro.Core.Taskbar;

public enum PillHidden { NoAnchors, NotAtBottom, NoRoom, OffScreen }

public readonly record struct PillPlace(int X, int Y, PillHidden? Hidden, int BandTop = 0)
{
    public bool IsShown => Hidden is null;
}

public static class PillPlacement
{
    /// <summary>Where the pill goes, just left of the tray icons, or why it is hidden.</summary>
    public static PillPlace Place(PixelRect monitor, PixelRect taskbar, TaskbarAnchors? anchors, int width, int height, int gap)
    {
        if (anchors is not { } a) return Hide(PillHidden.NoAnchors);
        if (taskbar.Width <= taskbar.Height || taskbar.Top < monitor.Top + monitor.Height / 2)
            return Hide(PillHidden.NotAtBottom);

        var x = taskbar.Left + a.TrayLeft - gap - width;
        if (x < taskbar.Left + a.ButtonsRight + gap) return Hide(PillHidden.NoRoom);

        if (height > a.BandBottom - a.BandTop) return Hide(PillHidden.NoRoom);

        var y = taskbar.Top + a.BandTop + (a.BandBottom - a.BandTop - height) / 2;
        if (x < monitor.Left || y < monitor.Top || x + width > monitor.Right || y + height > monitor.Bottom)
            return Hide(PillHidden.OffScreen);

        return new PillPlace(x, y, null, taskbar.Top + a.BandTop);
    }

    private static PillPlace Hide(PillHidden why) => new(0, 0, why);
}
