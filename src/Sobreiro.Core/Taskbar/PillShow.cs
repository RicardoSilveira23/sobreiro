namespace Sobreiro.Core.Taskbar;

/// <summary>What placing the pill does to its window: nothing, a move, or a raise to the top of the topmost band.</summary>
public enum PillShowStep { None, Move, Raise }

public static class PillShow
{
    /// <summary>
    /// A pill that is hidden, or shown below the taskbar, is raised: while it is hidden Explorer can raise
    /// the taskbar past it (it does when a full-screen window closes), and a window shown again keeps its old
    /// place in the z-order, under the taskbar. Otherwise it only moves, and not at all when it is in place.
    /// </summary>
    public static PillShowStep Step(bool visible, bool aboveTaskbar, bool inPlace) =>
        !visible || !aboveTaskbar ? PillShowStep.Raise
        : inPlace ? PillShowStep.None
        : PillShowStep.Move;
}

public static class ZOrder
{
    private const int MaxSteps = 4096;

    /// <summary>
    /// True when <paramref name="window"/> is above <paramref name="other"/>: walks up from <paramref name="other"/>
    /// with <paramref name="above"/> (the next window up, 0 at the top) looking for it. The walk is bounded, as
    /// the z-order can change while it runs.
    /// </summary>
    public static bool IsAbove(nint window, nint other, Func<nint, nint> above)
    {
        var current = other;
        for (var i = 0; i < MaxSteps; i++)
        {
            current = above(current);
            if (current == 0) return false;
            if (current == window) return true;
        }
        return false;
    }
}
