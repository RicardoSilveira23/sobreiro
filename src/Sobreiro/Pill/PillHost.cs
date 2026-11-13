using System.Diagnostics;
using System.Windows.Interop;
using System.Windows.Threading;
using Microsoft.Win32;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Display;
using Sobreiro.Core.Taskbar;

namespace Sobreiro.Pill;

/// <summary>
/// Keeps one <see cref="PillWindow"/> over the taskbar, just left of the tray icons: finds the taskbar,
/// follows it through auto-hide, DPI changes, display changes and Explorer restarts, and hides the pill
/// while a full-screen app runs. Everything here runs on the UI thread except the UI Automation query,
/// which is sent only when a <see cref="TaskbarQuerySchedule"/> says a tick should ask: on a sign of change,
/// and about every 16 seconds otherwise, not on every tick.
/// </summary>
internal sealed class PillHost : IDisposable
{
    private static readonly TimeSpan TickInterval = TimeSpan.FromSeconds(2);

    private readonly Dispatcher _dispatcher = Dispatcher.CurrentDispatcher;
    private readonly DispatcherTimer _timer;
    private readonly TaskbarNative.WinEventProc _locationProc;
    private readonly TaskbarNative.WinEventProc _foregroundProc;
    private readonly uint _taskbarCreated = TaskbarNative.RegisterMessage("TaskbarCreated");
    private readonly long _clockStart = Stopwatch.GetTimestamp();

    private HwndSource? _messages;
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
        _locationProc = OnLocationChange;
        _foregroundProc = OnForeground;
        _timer = new DispatcherTimer(DispatcherPriority.Normal, _dispatcher) { Interval = TickInterval };
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
        Guard("messages", () =>
        {
            _messages?.RemoveHook(WndProc);
            _messages?.Dispose();
            _messages = null;
        });
    }

    private bool Active => _enabled && !_disposed;

    // Monotonic, so a change of the wall clock cannot make a question early or late.
    private TimeSpan Now => Stopwatch.GetElapsedTime(_clockStart);

    private void Start()
    {
        if (_messages is null)
        {
            // A top-level window, not message-only: broadcasts such as TaskbarCreated skip message-only windows.
            _messages = new HwndSource(new HwndSourceParameters("SobreiroPillMessages") { WindowStyle = 0 });
            _messages.AddHook(WndProc);
        }
        _foregroundHook = TaskbarNative.HookEvents(TaskbarNative.EventSystemForeground, _foregroundProc, 0, 0,
            TaskbarNative.WinEventOutOfContext | TaskbarNative.WinEventSkipOwnProcess);
        SystemEvents.DisplaySettingsChanged += OnDisplaySettingsChanged;
        _timer.Start();
        Tick();
    }

    /// <summary>Stops everything and closes the window. Safe to call twice.</summary>
    private void Stop()
    {
        _timer.Stop();
        Unhook(ref _locationHook);
        Unhook(ref _foregroundHook);
        SystemEvents.DisplaySettingsChanged -= OnDisplaySettingsChanged;
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
        var taskbar = TaskbarNative.FindTaskbar();
        if (taskbar == IntPtr.Zero)
        {
            HideWindow();
            return;
        }
        if (taskbar != _taskbar) AttachTo(taskbar);
        if (_window is not null && !WindowAlive(_window)) CloseWindow();
        if (_window is null) CreateWindow();
        // Conditional: asking UI Automation every tick (1,800 times an hour) was most of the idle processor time.
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
        _trayNotify = TaskbarNative.FindTrayNotify(taskbar);
        var (pid, thread) = TaskbarNative.ProcessAndThread(taskbar);
        _locationHook = TaskbarNative.HookEvents(TaskbarNative.EventObjectLocationChange, _locationProc, pid, thread,
            TaskbarNative.WinEventOutOfContext);
    }

    private void CreateWindow()
    {
        if (_model is not { } model || _colours is not { } colours || _taskbar == IntPtr.Zero) return;
        var window = new PillWindow(_taskbar);
        window.LeftClicked += () => Guard("click", () => LeftClicked?.Invoke());
        window.RightClicked += () => Guard("click", () => RightClicked?.Invoke());
        window.DpiChanged += (_, _) => _dispatcher.BeginInvoke(() => Guard("dpi", Place));
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

    private static bool WindowAlive(PillWindow window) => TaskbarNative.IsWindow(window.Handle);

    /// <summary>One UI Automation query at a time, on a thread-pool thread; the answer comes back through the dispatcher.</summary>
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
        _ = Task.Run(() =>
        {
            var answer = TaskbarProbe.Query(taskbar);
            _dispatcher.BeginInvoke(() => Guard("anchors", () => OnProbed(taskbar, generation, answer)));
        });
    }

    private void OnProbed(IntPtr taskbar, int generation, TaskbarAnchors? answer)
    {
        _querying = false;
        if (!Active) return;
        // Taskbar changed or anchors were dropped while the query ran: the answer is stale, so ignore it.
        // A null answer (auto-hidden taskbar, restarting) keeps the last good anchors.
        if (taskbar == _taskbar && generation == _generation)
        {
            _schedule = _schedule.Answered(answer);
            if (answer is not null)
            {
                _anchors = answer;
                _anchorsDpi = TaskbarNative.DpiOf(taskbar);
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
            // Dropped here; the next tick (or Update) makes a new one.
            CloseWindow();
            return;
        }
        if (FullScreenApp()
            || TaskbarNative.WindowRect(_taskbar) is not { } taskbarRect
            || TaskbarNative.MonitorRect(_taskbar) is not { } monitor)
        {
            window.HideNow();
            return;
        }
        var dpi = TaskbarNative.DpiOf(_taskbar);
        if (_anchors is not null && dpi != _anchorsDpi)
        {
            // The taskbar's DPI changed (its location event can beat the display-change event): the
            // anchors are in the old scale, so drop them and ask again.
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
        if (place.IsShown) window.MoveTo(place.X, place.Y, place.BandTop);
        else window.HideNow();
    }

    private static bool FullScreenApp() =>
        TaskbarNative.NotificationState() is { } state && FullScreen.IsFullScreen(state);

    private void HideWindow()
    {
        if (_window is { } window && WindowAlive(window)) window.HideNow();
    }

    private void OnLocationChange(IntPtr hook, uint eventId, IntPtr hwnd, int idObject, int idChild, uint thread, uint time) =>
        Guard("location", () =>
        {
            if (!Active || idObject != 0) return;
            if (hwnd == _taskbar)
            {
                Place();
                if (_anchors is null) AskUiAutomation();
            }
            else if (_trayNotify != IntPtr.Zero && hwnd == _trayNotify)
            {
                AskUiAutomation();
            }
        });

    private void OnForeground(IntPtr hook, uint eventId, IntPtr hwnd, int idObject, int idChild, uint thread, uint time) =>
        Guard("foreground", () =>
        {
            if (!Active) return;
            // No question sent here: the schedule notes the switch and the next due tick asks.
            _schedule = _schedule.ForegroundSwitched();
            Place();
        });

    // Raised on another thread: marshal to the dispatcher.
    private void OnDisplaySettingsChanged(object? sender, EventArgs e) =>
        _dispatcher.BeginInvoke(() => Guard("display", () =>
        {
            if (!Active) return;
            _anchors = null;
            _generation++;
            _schedule = default;
            AskUiAutomation();
            Place();
        }));

    private IntPtr WndProc(IntPtr hwnd, int msg, IntPtr wParam, IntPtr lParam, ref bool handled)
    {
        if ((uint)msg == _taskbarCreated) Guard("taskbar created", Tick);
        return IntPtr.Zero;
    }

    private static void Unhook(ref IntPtr hook)
    {
        if (hook == IntPtr.Zero) return;
        TaskbarNative.UnhookWinEvent(hook);
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
            Debug.WriteLine($"Sobreiro: pill {part} failed: {ex.GetType().Name}");
        }
    }
}
