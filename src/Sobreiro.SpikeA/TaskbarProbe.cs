using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Sobreiro.Core.Display;
using Sobreiro.Core.Taskbar;

namespace Sobreiro.SpikeA;

/// <summary>
/// SPIKE (shell candidate A): where the tray icons and the task buttons are, asked of UI Automation through
/// the system's own COM interface. The interfaces are generated at build time, because Native AOT has no
/// built-in COM. Slow: call it off the UI thread.
/// </summary>
internal static unsafe partial class TaskbarProbe
{
    private const string FrameClass = "Taskbar.TaskbarFrameAutomationPeer";
    private const int BoundingRectangle = 30001, ProcessId = 30002, ClassNameProperty = 30012;
    private const int ElementAndChildren = 1 | 2;
    private static readonly Guid AutomationClass = new("ff48dba4-60ef-4201-aa87-54103eef594e");
    private static readonly Guid AutomationInterface = new("30cbe57d-d9d0-452a-ab13-7ac5ac4825ee");
    private static IUIAutomation? _automation;
    [ThreadStatic] private static bool _comReady;

    /// <summary>SOBREIRO_SPIKE_RELEASE=gc leaves the answer's COM objects to the garbage collector, as first measured.</summary>
    private static readonly bool ReleaseAtOnce = Environment.GetEnvironmentVariable("SOBREIRO_SPIKE_RELEASE") != "gc";

    private static readonly System.Collections.Concurrent.BlockingCollection<(IntPtr Taskbar, Action<TaskbarAnchors?> Done)> Queue = new();
    private static Thread? _thread;

    /// <summary>
    /// Puts the question on one thread that lives as long as the app and belongs to COM's multithreaded
    /// apartment. SOBREIRO_SPIKE_THREAD=pool asks from a thread-pool thread instead, as first measured.
    /// </summary>
    public static void Ask(IntPtr taskbar, Action<TaskbarAnchors?> done)
    {
        if (Environment.GetEnvironmentVariable("SOBREIRO_SPIKE_THREAD") == "pool")
        {
            _ = Task.Run(() => done(Query(taskbar)));
            return;
        }
        if (_thread is null)
        {
            _thread = new Thread(() =>
            {
                foreach (var (handle, answer) in Queue.GetConsumingEnumerable()) answer(Query(handle));
            }) { IsBackground = true, Name = "taskbar probe" };
            _thread.SetApartmentState(ApartmentState.MTA);
            _thread.Start();
        }
        Queue.Add((taskbar, done));
    }

    /// <summary>How many questions have been put, for the trace.</summary>
    public static int Asked;

    public static TaskbarAnchors? Query(IntPtr taskbar)
    {
        Interlocked.Increment(ref Asked);
        try
        {
            if (Native.WindowRect(taskbar) is not { } bounds) return null;
            if (!_comReady)
            {
                CoInitializeEx(IntPtr.Zero, 0); // multithreaded; "already done" is fine
                _comReady = true;
            }
            var automation = _automation ??= Create();
            var cache = automation.CreateCacheRequest();
            cache.AddProperty(ClassNameProperty);
            cache.AddProperty(BoundingRectangle);
            cache.AddProperty(ProcessId);
            cache.SetTreeScope(ElementAndChildren);

            var items = new List<(string ClassName, PixelRect Rect, int ProcessId)>();
            var buttons = new List<PixelRect>();
            var made = new List<object> { cache };
            try
            {
                var root = Keep(made, automation.ElementFromHandleBuildCache(taskbar, cache));
                foreach (var level1 in Children(root, made))
                {
                    foreach (var child in Children(Keep(made, level1.BuildUpdatedCache(cache)), made))
                    {
                        var className = Text(child);
                        items.Add((className, Rect(child), Number(child)));
                        if (className != FrameClass) continue;
                        foreach (var button in Children(Keep(made, child.BuildUpdatedCache(cache)), made)) buttons.Add(Rect(button));
                    }
                }
            }
            finally
            {
                // Each element holds its cached answer in this process until it is released: waiting for
                // the garbage collector let them pile up, about 1 MB a minute.
                if (ReleaseAtOnce)
                    foreach (var one in made) (one as ComObject)?.FinalRelease();
            }
            return TaskbarLayout.Anchors(bounds, items, buttons, Environment.ProcessId);
        }
        catch (Exception ex)
        {
            Spike.Trace($"taskbar probe failed: {ex.GetType().Name} 0x{ex.HResult:x}");
            return null;
        }
    }

    private static IUIAutomation Create()
    {
        Marshal.ThrowExceptionForHR(CoCreateInstance(AutomationClass, IntPtr.Zero, 1, AutomationInterface, out var pointer));
        try
        {
            return ComInterfaceMarshaller<IUIAutomation>.ConvertToManaged((void*)pointer)!;
        }
        finally
        {
            Marshal.Release(pointer);
        }
    }

    private static T Keep<T>(List<object> made, T value) where T : class
    {
        made.Add(value);
        return value;
    }

    private static List<IUIAutomationElement> Children(IUIAutomationElement element, List<object> made)
    {
        var found = new List<IUIAutomationElement>();
        var children = element.GetCachedChildren();
        if (children is null) return found;
        made.Add(children);
        var count = children.Length();
        for (var i = 0; i < count; i++) found.Add(Keep(made, children.GetElement(i)));
        return found;
    }

    private static string Text(IUIAutomationElement element)
    {
        ComVariant value = default;
        if (element.GetCachedPropertyValue(ClassNameProperty, &value) < 0) return "";
        try { return value.VarType == VarEnum.VT_BSTR ? value.As<string>() ?? "" : ""; }
        finally { value.Dispose(); }
    }

    private static int Number(IUIAutomationElement element)
    {
        ComVariant value = default;
        if (element.GetCachedPropertyValue(ProcessId, &value) < 0) return 0;
        try { return value.VarType == VarEnum.VT_I4 ? value.As<int>() : 0; }
        finally { value.Dispose(); }
    }

    // UI Automation gives left, top, width, height as an array of four doubles.
    private static PixelRect Rect(IUIAutomationElement element)
    {
        ComVariant value = default;
        if (element.GetCachedPropertyValue(BoundingRectangle, &value) < 0) return new PixelRect(0, 0, 0, 0);
        try
        {
            if (value.VarType != (VarEnum.VT_ARRAY | VarEnum.VT_R8)) return new PixelRect(0, 0, 0, 0);
            var array = (SafeArray*)value.GetRawDataRef<nint>();
            if (array is null || array->Dimensions != 1 || array->Count < 4 || array->Data is null) return new PixelRect(0, 0, 0, 0);
            var r = new ReadOnlySpan<double>(array->Data, 4);
            return double.IsInfinity(r[0])
                ? new PixelRect(0, 0, 0, 0)
                : new PixelRect((int)r[0], (int)r[1], (int)(r[0] + r[2]), (int)(r[1] + r[3]));
        }
        finally { value.Dispose(); }
    }

    // The system's array layout on 64-bit Windows, one dimension.
    [StructLayout(LayoutKind.Sequential)]
    private struct SafeArray
    {
        public ushort Dimensions;
        public ushort Features;
        public uint ElementSize;
        public uint Locks;
        public void* Data;
        public uint Count;
        public int LowerBound;
    }

    [LibraryImport("ole32.dll")]
    private static partial int CoInitializeEx(IntPtr reserved, uint model);

    [LibraryImport("ole32.dll")]
    private static partial int CoCreateInstance(in Guid classId, IntPtr outer, uint context, in Guid interfaceId, out IntPtr instance);
}

// Only the methods used are named; the others hold their place in the interface's order.
[GeneratedComInterface, Guid("30cbe57d-d9d0-452a-ab13-7ac5ac4825ee")]
internal partial interface IUIAutomation
{
    void _1(); void _2(); void _3(); void _4(); void _5(); void _6(); void _7();
    IUIAutomationElement ElementFromHandleBuildCache(IntPtr hwnd, IUIAutomationCacheRequest cache);
    void _9(); void _10(); void _11(); void _12(); void _13(); void _14(); void _15(); void _16(); void _17();
    IUIAutomationCacheRequest CreateCacheRequest();
}

[GeneratedComInterface, Guid("d22108aa-8ac5-49a5-837b-37bbb3d7591e")]
internal unsafe partial interface IUIAutomationElement
{
    void _1(); void _2(); void _3(); void _4(); void _5(); void _6();
    IUIAutomationElement BuildUpdatedCache(IUIAutomationCacheRequest cache);
    void _8(); void _9();
    [PreserveSig] int GetCachedPropertyValue(int propertyId, ComVariant* value);
    void _11(); void _12(); void _13(); void _14(); void _15(); void _16();
    IUIAutomationElementArray? GetCachedChildren();
}

[GeneratedComInterface, Guid("b32a92b5-bc25-4078-9c08-d7ee95c48e03")]
internal partial interface IUIAutomationCacheRequest
{
    void AddProperty(int propertyId);
    void _2(); void _3(); void _4();
    void SetTreeScope(int scope);
}

[GeneratedComInterface, Guid("14314595-b4bc-4055-95f2-58f2e42c9855")]
internal partial interface IUIAutomationElementArray
{
    int Length();
    IUIAutomationElement GetElement(int index);
}
