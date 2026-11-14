using System.Diagnostics;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using Avalonia.Threading;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Display;
using Sobreiro.Core.Taskbar;

namespace Sobreiro.SpikeA;

/// <summary>
/// SPIKE (shell candidate A): today's tracker on the toolkit's dispatcher and timer. Same schedule, same
/// placement from Core. Left out: the message that says the taskbar was made again and the display-change
/// event (the 2-second tick and the taskbar's own DPI catch both, later).
/// </summary>
internal sealed unsafe class PillHost : IDisposable
{
    private static readonly TimeSpan TickInterval = TimeSpan.FromSeconds(2);
    private static PillHost? _current;

    private readonly DispatcherTimer _timer;
    private readonly long _clockStart = Stopwatch.GetTimestamp();

    private PillWindow? _window;
    private IntPtr _taskbar;
    private IntPtr _trayNotify;
    private IntPtr _locationHook;
    private IntPtr _foregroundHook;
    private TaskbarAnchors? _anchors;
    private int _anchorsDpi;
    private bool _querying;
    private bool _requery;
    private int _generation;
    private TaskbarQuerySchedule _schedule;
    private bool _enabled;
    private bool _disposed;

    private PillModel? _model;
    private TaskbarColours? _colours;
    private string _tooltip = "";

    public PillHost()
    {
        _current = this;
        _timer = new DispatcherTimer { Interval = TickInterval };
        _timer.Tick += (_, _) => Guard("tick", Tick);
    }

    public event Action? LeftClicked;

    public event Action? RightClicked;

    public void SetEnabled(bool enabled)
    {
        if (_disposed || enabled == _enabled) return;
        _enabled = enabled;
        if (enabled) Guard("start", Start);
        else Guard("stop", Stop);
    }

    public void Update(PillModel model, TaskbarColours colours, string tooltip)
    {
        if (_disposed) return;
        _model = model;
        _colours = colours;
        _tooltip = tooltip;
        if (!_enabled || _taskbar == IntPtr.Zero) return;
        Guard("update", () =>
        {
            if (_window is not null && !WindowAlive(_window)) CloseWindow();
            if (_window is null) CreateWindow();
            else _window.Render(model, colours, tooltip);
            Place();
        });
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        _enabled = false;
        Guard("stop", Stop);
    }

    private bool Active => _enabled && !_disposed;

    private TimeSpan Now => Stopwatch.GetElapsedTime(_clockStart);

    private void Start()
    {
        _foregroundHook = Native.SetWinEventHook(Native.EventSystemForeground, Native.EventSystemForeground, IntPtr.Zero,
            (IntPtr)(delegate* unmanaged[Stdcall]<IntPtr, uint, IntPtr, int, int, uint, uint, void>)&OnForeground, 0, 0,
            Native.WinEventOutOfContext | Native.WinEventSkipOwnProcess);
        _timer.Start();
        Tick();
    }

    private void Stop()
    {
        _timer.Stop();
        Unhook(ref _locationHook);
        Unhook(ref _foregroundHook);
        CloseWindow();
        _anchors = null;
        _generation++;
        _schedule = default;
        _taskbar = IntPtr.Zero;
        _trayNotify = IntPtr.Zero;
        _requery = false;
    }

    private void Tick()
    {
        if (!Active) return;
        var taskbar = Native.FindTaskbar();
        if (taskbar == IntPtr.Zero)
        {
            HideWindow();
            return;
        }
        if (taskbar != _taskbar) AttachTo(taskbar);
        if (_window is not null && !WindowAlive(_window)) CloseWindow();
        if (_window is null) CreateWindow();
        if (_schedule.Due(Now)) AskUiAutomation();
        Place();
    }

    private void AttachTo(IntPtr taskbar)
    {
        Unhook(ref _locationHook);
        CloseWindow();
        _anchors = null;
        _generation++;
        _schedule = default;
        _taskbar = taskbar;
        _trayNotify = Native.FindTrayNotify(taskbar);
        var thread = Native.GetWindowThreadProcessId(taskbar, out var pid);
        _locationHook = Native.SetWinEventHook(Native.EventObjectLocationChange, Native.EventObjectLocationChange, IntPtr.Zero,
            (IntPtr)(delegate* unmanaged[Stdcall]<IntPtr, uint, IntPtr, int, int, uint, uint, void>)&OnLocationChange, pid, thread,
            Native.WinEventOutOfContext);
    }

    private void CreateWindow()
    {
        if (_model is not { } model || _colours is not { } colours || _taskbar == IntPtr.Zero) return;
        var window = new PillWindow(_taskbar);
        window.LeftClicked += () => Guard("click", () => LeftClicked?.Invoke());
        window.RightClicked += () => Guard("click", () => RightClicked?.Invoke());
        window.ScalingChanged += (_, _) => Dispatcher.UIThread.Post(() => Guard("dpi", Place));
        _window = window;
        window.Render(model, colours, _tooltip);
    }

    private void CloseWindow()
    {
        var window = _window;
        _window = null;
        if (window is null) return;
        try
        {
            window.Close();
        }
        catch (InvalidOperationException)
        {
            // Already closing or closed.
        }
    }

    private static bool WindowAlive(PillWindow window) => Native.IsWindow(window.Handle);

    /// <summary>One UI Automation query at a time, off the UI thread; the answer comes back through the dispatcher.</summary>
    private void AskUiAutomation()
    {
        if (!Active || _taskbar == IntPtr.Zero) return;
        if (_querying)
        {
            _requery = true;
            return;
        }
        _querying = true;
        _schedule = _schedule.Asked(Now);
        var taskbar = _taskbar;
        var generation = _generation;
        void Answer(TaskbarAnchors? answer)
        {
            Spike.Trace(answer is null ? "anchors: none" : $"anchors: {answer}");
            Dispatcher.UIThread.Post(() => Guard("anchors", () => OnProbed(taskbar, generation, answer)));
        }
        if (Spike.AskUia) TaskbarProbe.Ask(taskbar, Answer);
        else Answer(null);
    }

    private void OnProbed(IntPtr taskbar, int generation, TaskbarAnchors? answer)
    {
        _querying = false;
        if (!Active) return;
        if (taskbar == _taskbar && generation == _generation)
        {
            _schedule = _schedule.Answered(answer);
            if (answer is not null)
            {
                _anchors = answer;
                _anchorsDpi = (int)Native.GetDpiForWindow(taskbar);
            }
            Place();
        }
        if (_requery)
        {
            _requery = false;
            AskUiAutomation();
        }
    }

    private void Place()
    {
        if (!Active || _window is not { } window) return;
        if (!WindowAlive(window))
        {
            CloseWindow();
            return;
        }
        if (FullScreenApp()
            || Native.WindowRect(_taskbar) is not { } taskbarRect
            || Native.MonitorRect(_taskbar) is not { } monitor)
        {
            window.HideNow();
            return;
        }
        var dpi = (int)Native.GetDpiForWindow(_taskbar);
        if (_anchors is not null && dpi != _anchorsDpi)
        {
            _anchors = null;
            _generation++;
            _schedule = default;
            window.HideNow();
            AskUiAutomation();
            return;
        }
        var (width, height) = window.PixelSize;
        var gap = (int)Math.Round(4.0 * dpi / 96);
        var place = PillPlacement.Place(monitor, taskbarRect, _anchors, width, height, gap);
        if (place.IsShown) window.MoveTo(place.X, place.Y);
        else window.HideNow();
    }

    private static bool FullScreenApp() =>
        Native.NotificationState() is { } state && FullScreen.IsFullScreen(state);

    private void HideWindow()
    {
        if (_window is { } window && WindowAlive(window)) window.HideNow();
    }

    // Out-of-context events arrive on the thread that set the hook, through its message loop: the UI thread.
    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvStdcall)])]
    private static void OnLocationChange(IntPtr hook, uint eventId, IntPtr hwnd, int idObject, int idChild, uint thread, uint time)
    {
        if (_current is not { } host) return;
        Guard("location", () =>
        {
            if (!host.Active || idObject != 0) return;
            if (hwnd == host._taskbar)
            {
                host.Place();
                if (host._anchors is null) host.AskUiAutomation();
            }
            else if (host._trayNotify != IntPtr.Zero && hwnd == host._trayNotify)
            {
                host.AskUiAutomation();
            }
        });
    }

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvStdcall)])]
    private static void OnForeground(IntPtr hook, uint eventId, IntPtr hwnd, int idObject, int idChild, uint thread, uint time)
    {
        if (_current is not { } host) return;
        Guard("foreground", () =>
        {
            if (!host.Active) return;
            host._schedule = host._schedule.ForegroundSwitched();
            host.Place();
        });
    }

    private static void Unhook(ref IntPtr hook)
    {
        if (hook == IntPtr.Zero) return;
        Native.UnhookWinEvent(hook);
        hook = IntPtr.Zero;
    }

    private static void Guard(string part, Action action)
    {
        try
        {
            action();
        }
        catch (Exception ex)
        {
            Spike.Trace($"pill {part} failed: {ex.GetType().Name}: {ex.Message}");
        }
    }
}
