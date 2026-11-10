using System.Windows.Automation;
using Sobreiro.Core.Display;
using Sobreiro.Core.Taskbar;

namespace Sobreiro.Pill;

/// <summary>Reads where the tray icons and the task buttons are, from UI Automation. Slow: call it off the UI thread.</summary>
internal static class TaskbarProbe
{
    private const string FrameClass = "Taskbar.TaskbarFrameAutomationPeer";

    /// <summary>Null when the taskbar cannot be read (auto-hidden, restarting, no tray items) or on any exception.</summary>
    public static TaskbarAnchors? Query(IntPtr taskbar)
    {
        try
        {
            if (TaskbarNative.WindowRect(taskbar) is not { } bounds) return null;
            var items = new List<(string ClassName, PixelRect Rect, int ProcessId)>();
            var buttons = new List<PixelRect>();
            var cache = new CacheRequest { TreeScope = TreeScope.Element | TreeScope.Children };
            cache.Add(AutomationElement.ClassNameProperty);
            cache.Add(AutomationElement.BoundingRectangleProperty);
            cache.Add(AutomationElement.ProcessIdProperty);
            using (cache.Activate())
            {
                var root = AutomationElement.FromHandle(taskbar);
                foreach (AutomationElement level1 in root.CachedChildren)
                {
                    foreach (AutomationElement child in level1.GetUpdatedCache(cache).CachedChildren)
                    {
                        var className = child.Cached.ClassName ?? "";
                        items.Add((className, ToPixels(child.Cached.BoundingRectangle), child.Cached.ProcessId));
                        if (className != FrameClass) continue;
                        foreach (AutomationElement button in child.GetUpdatedCache(cache).CachedChildren)
                            buttons.Add(ToPixels(button.Cached.BoundingRectangle));
                    }
                }
            }
            return TaskbarLayout.Anchors(bounds, items, buttons, Environment.ProcessId);
        }
        catch (Exception ex)
        {
            System.Diagnostics.Debug.WriteLine($"Sobreiro: pill taskbar probe failed: {ex.GetType().Name}");
            return null;
        }
    }

    private static PixelRect ToPixels(System.Windows.Rect r) =>
        r.IsEmpty || double.IsInfinity(r.Left)
            ? new PixelRect(0, 0, 0, 0)
            : new PixelRect((int)r.Left, (int)r.Top, (int)r.Right, (int)r.Bottom);
}
