using System.Runtime.InteropServices;
using Sobreiro.Core.Display;
using Sobreiro.Core.Taskbar;

namespace Sobreiro.Pill;

/// <summary>SPIKE (shell candidate F): throwaway switches, read once from the environment, for the comparisons.</summary>
internal static class Spike
{
    /// <summary>SOBREIRO_SPIKE_WINDOW=wpf keeps the WPF pill window; anything else is the plain one.</summary>
    public static readonly bool PlainWindow = Environment.GetEnvironmentVariable("SOBREIRO_SPIKE_WINDOW") != "wpf";

    /// <summary>SOBREIRO_SPIKE_UIA: "managed" (WPF's client), "none" (never ask), anything else the system's own interface.</summary>
    public static readonly string Uia = Environment.GetEnvironmentVariable("SOBREIRO_SPIKE_UIA") ?? "com";

    private static readonly string? TraceFile = Environment.GetEnvironmentVariable("SOBREIRO_SPIKE_TRACE");
    private static string? _lastTrace;

    /// <summary>SOBREIRO_SPIKE_TRACE=file: appends a line when it differs from the last one. Never holds a token.</summary>
    public static void Trace(string line)
    {
        if (TraceFile is null || line == _lastTrace) return;
        _lastTrace = line;
        try { System.IO.File.AppendAllText(TraceFile, $"{DateTime.Now:HH:mm:ss} {line}" + Environment.NewLine); } catch (Exception) { }
    }
}

/// <summary>
/// SPIKE (shell candidate F): <see cref="TaskbarProbe"/>'s question, put to UI Automation through the
/// system's own COM interface instead of WPF's managed client. Slow: call it off the UI thread.
/// </summary>
internal static class TaskbarProbeCom
{
    private const string FrameClass = "Taskbar.TaskbarFrameAutomationPeer";
    private const int BoundingRectangle = 30001, ProcessId = 30002, ClassNameProperty = 30012;
    private const int ElementAndChildren = 1 | 2;
    private static IUIAutomation? _automation;

    public static TaskbarAnchors? Query(IntPtr taskbar)
    {
        try
        {
            if (TaskbarNative.WindowRect(taskbar) is not { } bounds) return null;
            var automation = _automation ??= (IUIAutomation)Activator.CreateInstance(
                Type.GetTypeFromCLSID(new Guid("ff48dba4-60ef-4201-aa87-54103eef594e"))!)!;
            var cache = automation.CreateCacheRequest();
            cache.AddProperty(ClassNameProperty);
            cache.AddProperty(BoundingRectangle);
            cache.AddProperty(ProcessId);
            cache.SetTreeScope(ElementAndChildren);

            var items = new List<(string ClassName, PixelRect Rect, int ProcessId)>();
            var buttons = new List<PixelRect>();
            var root = automation.ElementFromHandleBuildCache(taskbar, cache);
            foreach (var level1 in Children(root))
            {
                foreach (var child in Children(level1.BuildUpdatedCache(cache)))
                {
                    var className = child.GetCachedPropertyValue(ClassNameProperty) as string ?? "";
                    items.Add((className, Rect(child), child.GetCachedPropertyValue(ProcessId) as int? ?? 0));
                    if (className != FrameClass) continue;
                    foreach (var button in Children(child.BuildUpdatedCache(cache))) buttons.Add(Rect(button));
                }
            }
            return TaskbarLayout.Anchors(bounds, items, buttons, Environment.ProcessId);
        }
        catch (Exception ex)
        {
            System.Diagnostics.Debug.WriteLine($"Sobreiro: pill taskbar probe (COM) failed: {ex.GetType().Name}");
            return null;
        }
    }

    private static IEnumerable<IUIAutomationElement> Children(IUIAutomationElement element)
    {
        var children = element.GetCachedChildren();
        if (children is null) yield break;
        var count = children.Length();
        for (var i = 0; i < count; i++) yield return children.GetElement(i);
    }

    // UI Automation gives left, top, width, height.
    private static PixelRect Rect(IUIAutomationElement element) =>
        element.GetCachedPropertyValue(BoundingRectangle) is double[] { Length: 4 } r && !double.IsInfinity(r[0])
            ? new PixelRect((int)r[0], (int)r[1], (int)(r[0] + r[2]), (int)(r[1] + r[3]))
            : new PixelRect(0, 0, 0, 0);

    // Only the methods used are named; the others hold their place in the interface's order.
    [ComImport, Guid("30cbe57d-d9d0-452a-ab13-7ac5ac4825ee"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IUIAutomation
    {
        void _1(); void _2(); void _3(); void _4(); void _5(); void _6(); void _7();
        IUIAutomationElement ElementFromHandleBuildCache(IntPtr hwnd, IUIAutomationCacheRequest cache);
        void _9(); void _10(); void _11(); void _12(); void _13(); void _14(); void _15(); void _16(); void _17();
        IUIAutomationCacheRequest CreateCacheRequest();
    }

    [ComImport, Guid("d22108aa-8ac5-49a5-837b-37bbb3d7591e"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IUIAutomationElement
    {
        void _1(); void _2(); void _3(); void _4(); void _5(); void _6();
        IUIAutomationElement BuildUpdatedCache(IUIAutomationCacheRequest cache);
        void _8(); void _9();
        [return: MarshalAs(UnmanagedType.Struct)] object? GetCachedPropertyValue(int propertyId);
        void _11(); void _12(); void _13(); void _14(); void _15(); void _16();
        IUIAutomationElementArray? GetCachedChildren();
    }

    [ComImport, Guid("b32a92b5-bc25-4078-9c08-d7ee95c48e03"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IUIAutomationCacheRequest
    {
        void AddProperty(int propertyId);
        void _2(); void _3(); void _4();
        void SetTreeScope(int scope);
    }

    [ComImport, Guid("14314595-b4bc-4055-95f2-58f2e42c9855"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IUIAutomationElementArray
    {
        int Length();
        IUIAutomationElement GetElement(int index);
    }
}
