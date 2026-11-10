using System.Diagnostics;
using System.Drawing;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using System.Windows.Forms;
using System.Windows.Threading;
using Microsoft.Win32;
using Sobreiro.Core;
using Sobreiro.Core.Alerts;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Config;
using Sobreiro.Core.Connect;
using Sobreiro.Core.Copilot;
using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Usage;
using Sobreiro.Flyout;
using Sobreiro.Pill;

namespace Sobreiro;

internal sealed class TrayController : IDisposable
{
    private const int BalloonMs = 10_000;

    private readonly Dispatcher _dispatcher = Dispatcher.CurrentDispatcher;
    private readonly NotifyIcon _notifyIcon;
    private readonly NotifyIcon _copilotIcon;
    private readonly TrayMenu _menu;
    private readonly FlyoutWindow _flyout = new();
    private readonly PillHost _pill = new();
    private readonly LiveUsage _live;
    private readonly CopilotUsage _copilot;
    private readonly FileSystemWatcher _watcher;
    private readonly DispatcherTimer _debounce;
    private readonly DispatcherTimer _tick;
    private readonly string _usagePath = AppPaths.UsageFile();
    private readonly string _alertsPath = AppPaths.AlertsFile();
    private UsageSnapshot? _last;
    private AlertState _alerts;
    private Icon? _icon;
    private Icon? _copilotIconImage;
    private bool _disposed;

    public TrayController(Action<string>? log = null)
    {
        var dataDir = AppPaths.DataDirectory();
        Directory.CreateDirectory(dataDir);
        _alerts = AlertState.Load(_alertsPath);

        _live = new LiveUsage(_usagePath, () => Refresh());
        _copilot = new CopilotUsage(() => Refresh(), log ?? (_ => { }));
        _menu = new TrayMenu(() => Refresh());
        _notifyIcon = new NotifyIcon { ContextMenuStrip = _menu.Strip, Visible = true };
        _notifyIcon.MouseClick += OnMouseClick;
        _copilotIcon = new NotifyIcon { ContextMenuStrip = _menu.Strip, Visible = false };
        _copilotIcon.MouseClick += OnMouseClick;
        _pill.LeftClicked += OnPillLeftClicked;
        _pill.RightClicked += OnPillRightClicked;

        _debounce = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(250) };
        _debounce.Tick += OnDebounceTick;
        _tick = new DispatcherTimer { Interval = TimeSpan.FromSeconds(30) };
        _tick.Tick += OnTick;

        _watcher = new FileSystemWatcher(dataDir, "usage.json")
        {
            NotifyFilter = NotifyFilters.LastWrite | NotifyFilters.FileName | NotifyFilters.Size,
        };
        _watcher.Changed += OnFileEvent;
        _watcher.Created += OnFileEvent;
        _watcher.Renamed += OnFileEvent;
        _watcher.EnableRaisingEvents = true;

        SystemEvents.UserPreferenceChanged += OnUserPreferenceChanged;
        SystemEvents.TimeChanged += OnTimeChanged;
        SystemEvents.DisplaySettingsChanged += OnDisplaySettingsChanged;

        Refresh();
        _tick.Start();
        _live.Start(DateTimeOffset.Now);
        _copilot.Start(DateTimeOffset.Now);
    }

    /// <summary>
    /// config.json for drawing. A config that can't be read, or isn't valid JSON, draws as the default and
    /// is never saved from here.
    /// </summary>
    public static AppConfig ConfigForDisplay()
    {
        try
        {
            return AppConfig.Load(AppPaths.ConfigFile());
        }
        catch (Exception ex)
        {
            Debug.WriteLine($"Sobreiro: reading config.json failed: {ex.GetType().Name}");
            return AppConfig.Default;
        }
    }

    /// <summary>Opens the flyout by the tray: the bottom-right corner of the primary work area.</summary>
    public void ShowFlyoutByTray()
    {
        if (_disposed) return;
        var at = Screen.PrimaryScreen is { } primary
            ? new Point(primary.WorkingArea.Right - 1, primary.WorkingArea.Bottom - 1)
            : Cursor.Position;
        OpenFlyout(at);
    }

    // A Live usage poll this starts redraws the open flyout when it answers.
    private void OpenFlyout(Point at)
    {
        Guard("live usage", () => _live.FlyoutOpened(DateTimeOffset.Now));
        Guard("copilot usage", () => _copilot.FlyoutOpened(DateTimeOffset.Now));
        Refresh(at);
    }

    private void OnMouseClick(object? sender, MouseEventArgs e)
    {
        if (e.Button != MouseButtons.Left) return;
        if (_flyout.IsVisible)
        {
            _flyout.Hide();
            return;
        }
        // The click that took the focus away already closed it.
        if (_flyout.HiddenJustNow) return;
        OpenFlyout(Cursor.Position);
    }

    private void OnPillLeftClicked()
    {
        if (_disposed) return;
        if (_flyout.IsVisible) _flyout.Hide();
        else OpenFlyout(Cursor.Position);
    }

    // The strip must own the foreground to close on Esc or on a click elsewhere.
    private void OnPillRightClicked()
    {
        if (_disposed) return;
        SetForegroundWindow(_menu.Strip.Handle);
        _menu.Strip.Show(Cursor.Position);
    }

    [DllImport("user32.dll")]
    private static extern bool SetForegroundWindow(IntPtr hWnd);

    private void OnDebounceTick(object? sender, EventArgs e)
    {
        _debounce.Stop();
        Refresh();
    }

    private void OnTick(object? sender, EventArgs e)
    {
        Guard("live usage", () =>
        {
            _live.Tick(DateTimeOffset.Now);
        });
        Guard("copilot usage", () => _copilot.Tick(DateTimeOffset.Now));
        Refresh();
    }

    // Watcher events arrive on a thread-pool thread: hop to the UI thread and debounce there.
    private void OnFileEvent(object? sender, FileSystemEventArgs e) => OnDispatcher(() =>
    {
        _debounce.Stop();
        _debounce.Start();
    });

    // Theme (light or dark, accent), visual style and regional format changes.
    private void OnUserPreferenceChanged(object? sender, UserPreferenceChangedEventArgs e)
    {
        if (e.Category is not (UserPreferenceCategory.General or UserPreferenceCategory.Color
            or UserPreferenceCategory.VisualStyle or UserPreferenceCategory.Locale))
            return;
        var locale = e.Category == UserPreferenceCategory.Locale;
        OnDispatcher(() =>
        {
            if (locale) CultureInfo.CurrentCulture.ClearCachedData();
            Refresh();
        });
    }

    private void OnTimeChanged(object? sender, EventArgs e) => OnDispatcher(() =>
    {
        TimeZoneInfo.ClearCachedData();
        Refresh();
    });

    private void OnDisplaySettingsChanged(object? sender, EventArgs e) => OnDispatcher(() => Refresh());

    private void OnDispatcher(Action action) => _dispatcher.BeginInvoke(() =>
    {
        if (!_disposed) action();
    });

    /// <summary>
    /// Reads usage.json once and feeds the tooltip and icon, the flyout (when it is open, or opens it at
    /// <paramref name="showAt"/>) and the alerts. Each part fails on its own, keeping what it last drew.
    /// </summary>
    private void Refresh(Point? showAt = null)
    {
        if (_disposed) return;
        IReadOnlyList<Limit> claude;
        DateTimeOffset now;
        try
        {
            // A read that loses a race with the bridge returns null: keep the previous snapshot.
            _last = UsageFile.Read(_usagePath) ?? _last;
            now = DateTimeOffset.Now;
            claude = LimitList.From(_last, now);
        }
        catch (Exception ex)
        {
            Debug.WriteLine($"Sobreiro: reading usage failed: {ex.GetType().Name}");
            return;
        }
        // Copilot fails on its own: with no reading, Claude draws as it always did.
        IReadOnlyList<Limit> copilot = [];
        Guard("copilot limits", () => copilot = LimitList.FromCopilot(_copilot.Shown, now));
        var showClaude = true;
        var legacy = false;
        Guard("connection", () =>
        {
            var credentialsExist = File.Exists(ClaudePaths.CredentialsFile(Environment.GetEnvironmentVariable));
            // settings.json is read only when it can matter: with numbers, Claude is on this PC and not legacy.
            var connection = claude.Count == 0 ? ConnectFlow.CurrentState() : ConnectionState.NotConnected;
            showClaude = LimitList.ShowClaude(
                LimitList.ClaudeOnPc(credentialsExist, claude.Count, connection),
                copilot.Count > 0);
            // Only with no numbers: the old bridge keeps writing to the old folder until Reconnect.
            legacy = claude.Count == 0 && connection == ConnectionState.Legacy;
        });
        var zone = TimeZoneInfo.Local;
        var culture = CultureInfo.CurrentCulture;
        var config = ConfigForDisplay();
        var shown = showClaude ? claude.Concat(copilot).ToList() : copilot.ToList();

        // Once per refresh: it reads the registry.
        TaskbarColours? taskbarColours = null;
        Guard("colours", () => taskbarColours = TaskbarColours.For(
            WindowsTheme.IsLight(), WindowsTheme.AccentTaskbar(), config.BarColour, WindowsTheme.Accent()));
        if (taskbarColours is { } colours)
        {
            Guard("icon", () => UpdateIcon(claude, showClaude, now, zone, culture, colours, legacy));
            Guard("copilot icon", () => UpdateCopilotIcon(copilot, now, zone, culture, colours));
            Guard("pill", () => UpdatePill(shown, showClaude, now, zone, culture, config, colours, legacy));
        }
        Guard("icon visibility", () => UpdateIconVisibility(showClaude, copilot.Count > 0));
        Guard("flyout", () => UpdateFlyout(shown, showClaude, now, zone, culture, config, showAt, legacy));
        Guard("alerts", () => CheckAlerts(shown, now, zone, culture));
    }

    // A failing part (GDI, registry, an unexpected Core error) must not take the tray down.
    private static void Guard(string part, Action action)
    {
        try
        {
            action();
        }
        catch (Exception ex)
        {
            Debug.WriteLine($"Sobreiro: {part} failed: {ex.GetType().Name}");
        }
    }

    // Everything is computed before anything is swapped, so the last good icon and tooltip stay.
    private void UpdateIcon(
        IReadOnlyList<Limit> claude, bool showClaude, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture,
        TaskbarColours colours, bool legacy)
    {
        if (showClaude)
        {
            var text = TooltipText.Render(claude, now, zone, culture, legacy);
            var newIcon = IconRenderer.Render(RingsModel.From(claude), colours, SystemInformation.SmallIconSize.Width);
            Swap(_notifyIcon, ref _icon, newIcon, text);
        }
    }

    // Never both hidden: the tray icon is the way to quit. Claude's hides only once Copilot's is drawn.
    private void UpdateIconVisibility(bool showClaude, bool copilotShown)
    {
        var copilotVisible = copilotShown && _copilotIconImage is not null;
        _copilotIcon.Visible = copilotVisible;
        _notifyIcon.Visible = showClaude || !copilotVisible;
    }

    private void UpdateCopilotIcon(
        IReadOnlyList<Limit> copilot, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture, TaskbarColours colours)
    {
        if (copilot.Count > 0)
        {
            var text = TooltipText.RenderCopilot(copilot[0], now, zone, culture);
            var newIcon = IconRenderer.Render(RingsModel.Copilot(copilot), colours, SystemInformation.SmallIconSize.Width);
            Swap(_copilotIcon, ref _copilotIconImage, newIcon, text);
        }
    }

    private static void Swap(NotifyIcon target, ref Icon? current, Icon newIcon, string text)
    {
        var old = current;
        try
        {
            target.Icon = newIcon;
        }
        catch
        {
            newIcon.Dispose();
            throw;
        }
        current = newIcon;
        old?.Dispose();
        target.Text = text;
    }

    private void UpdatePill(
        IReadOnlyList<Limit> limits, bool showClaude, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture,
        AppConfig config, TaskbarColours colours, bool legacy)
    {
        var enabled = config.Show == ShowOption.TrayAndPill;
        _pill.SetEnabled(enabled);
        if (!enabled) return;
        var parts = new List<string>();
        if (showClaude) parts.Add(TooltipText.Render(limits, now, zone, culture, legacy));
        if (limits.FirstOrDefault(l => l.Kind == LimitKind.Credits) is { } credits)
            parts.Add(TooltipText.RenderCopilot(credits, now, zone, culture));
        _pill.Update(PillModel.From(limits, zone, showClaude), colours, string.Join("\n", parts));
    }

    private void UpdateFlyout(
        IReadOnlyList<Limit> limits, bool showClaude, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture,
        AppConfig config, Point? showAt, bool legacy)
    {
        if (showAt is null && !_flyout.IsVisible) return;
        var colours = FlyoutColours.For(!WindowsTheme.IsLight(), config.BarColour, WindowsTheme.Accent());
        var note = _copilot.Shown is { } reading
            ? CopilotStateText.Line(_copilot.State, reading, now, zone, culture)
            : null;
        _flyout.Render(
            FlyoutModel.From(limits, now, zone, culture, _live.State, legacy, note, showClaude), config.Style, colours);
        if (showAt is { } at) _flyout.ShowNear(at.X, at.Y);
    }

    // The new state is kept even when saving it fails, so an alert never repeats every 30 seconds.
    private void CheckAlerts(IReadOnlyList<Limit> limits, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture)
    {
        var (alerts, next) = AlertPolicy.Check(limits, _alerts, now, zone, culture);
        if (ReferenceEquals(next, _alerts)) return;
        _alerts = next;
        if (AlertBalloon.For(alerts.Where(a => a.Kind != LimitKind.Credits).ToList()) is { } claude)
            Guard("balloon", () => ShowBalloon(_notifyIcon, claude.Title, claude.Text));
        if (AlertBalloon.For(alerts.Where(a => a.Kind == LimitKind.Credits).ToList()) is { } credits)
            Guard("copilot balloon", () => ShowBalloon(_copilotIcon, credits.Title, credits.Text));
        next.Save(_alertsPath);
    }

    private static void ShowBalloon(NotifyIcon icon, string title, string text) =>
        icon.ShowBalloonTip(BalloonMs, title, text, ToolTipIcon.None);

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        SystemEvents.UserPreferenceChanged -= OnUserPreferenceChanged;
        SystemEvents.TimeChanged -= OnTimeChanged;
        SystemEvents.DisplaySettingsChanged -= OnDisplaySettingsChanged;

        _tick.Stop();
        _tick.Tick -= OnTick;
        _live.Dispose();
        _copilot.Dispose();
        _debounce.Stop();
        _debounce.Tick -= OnDebounceTick;

        _watcher.EnableRaisingEvents = false;
        _watcher.Changed -= OnFileEvent;
        _watcher.Created -= OnFileEvent;
        _watcher.Renamed -= OnFileEvent;
        _watcher.Dispose();

        _pill.LeftClicked -= OnPillLeftClicked;
        _pill.RightClicked -= OnPillRightClicked;
        _pill.Dispose();

        try
        {
            _flyout.Close();
        }
        catch (InvalidOperationException)
        {
            // The application's shutdown closed it already.
        }

        _notifyIcon.MouseClick -= OnMouseClick;
        _notifyIcon.Visible = false;
        _notifyIcon.ContextMenuStrip = null;
        _notifyIcon.Dispose();
        _copilotIcon.MouseClick -= OnMouseClick;
        _copilotIcon.Visible = false;
        _copilotIcon.ContextMenuStrip = null;
        _copilotIcon.Dispose();
        _menu.Dispose();
        _icon?.Dispose();
        _copilotIconImage?.Dispose();
    }
}
