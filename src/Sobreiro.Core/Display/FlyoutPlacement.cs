namespace Sobreiro.Core.Display;

public readonly record struct PixelRect(int Left, int Top, int Right, int Bottom)
{
    public int Width => Right - Left;
    public int Height => Bottom - Top;
}

public enum TaskbarEdge { Bottom, Top, Left, Right }

public static class FlyoutPlacement
{
    /// <summary>
    /// The side the taskbar is on: where the work area is inset from the monitor. With no inset
    /// (auto-hide), the monitor edge nearest the cursor.
    /// </summary>
    public static TaskbarEdge Edge(PixelRect monitor, PixelRect workArea, int cursorX, int cursorY)
    {
        var bottom = monitor.Bottom - workArea.Bottom;
        var top = workArea.Top - monitor.Top;
        var left = workArea.Left - monitor.Left;
        var right = monitor.Right - workArea.Right;

        var max = Math.Max(Math.Max(bottom, top), Math.Max(left, right));
        if (max > 0)
            return bottom == max ? TaskbarEdge.Bottom
                : top == max ? TaskbarEdge.Top
                : left == max ? TaskbarEdge.Left
                : TaskbarEdge.Right;

        var dBottom = monitor.Bottom - cursorY;
        var dTop = cursorY - monitor.Top;
        var dLeft = cursorX - monitor.Left;
        var dRight = monitor.Right - cursorX;
        var min = Math.Min(Math.Min(dBottom, dTop), Math.Min(dLeft, dRight));
        return dBottom == min ? TaskbarEdge.Bottom
            : dTop == min ? TaskbarEdge.Top
            : dLeft == min ? TaskbarEdge.Left
            : TaskbarEdge.Right;
    }

    public static (int X, int Y) Place(
        PixelRect monitor, PixelRect workArea, int cursorX, int cursorY, int width, int height, int margin)
    {
        int ClampX(int x) => Clamp(x, workArea.Left + margin, workArea.Right - width - margin);
        int ClampY(int y) => Clamp(y, workArea.Top + margin, workArea.Bottom - height - margin);

        return Edge(monitor, workArea, cursorX, cursorY) switch
        {
            TaskbarEdge.Bottom => (ClampX(cursorX - width / 2), workArea.Bottom - height - margin),
            TaskbarEdge.Top => (ClampX(cursorX - width / 2), workArea.Top + margin),
            TaskbarEdge.Left => (workArea.Left + margin, ClampY(cursorY - height / 2)),
            _ => (workArea.Right - width - margin, ClampY(cursorY - height / 2)),
        };
    }

    // Math.Clamp throws when max < min (a flyout larger than the work area); keep the start on screen instead.
    private static int Clamp(int v, int min, int max) => max < min ? min : Math.Clamp(v, min, max);
}
