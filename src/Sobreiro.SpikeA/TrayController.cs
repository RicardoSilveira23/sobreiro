using System.Diagnostics;
using System.Globalization;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Threading;
using Sobreiro.Copilot;
using Sobreiro.Core;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Config;
using Sobreiro.Core.Connect;
using Sobreiro.Core.Copilot;
using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Live;
using Sobreiro.Core.Polling;

namespace Sobreiro.SpikeA;

/// <summary>
/// SPIKE (shell candidate A): today's tray controller cut down to what the spike shows. Core does the
/// reading, the polling and every word; this draws the two tray icons, the pill and the flyout.
/// Left out: Connect and Disconnect, the alerts, the settings in the menu other than Style, the first-run
/// migration, the bridge, the log, and the theme, time and display events (a redraw every 30 seconds stands in).
/// </summary>
internal sealed class TrayController : IDisposable
{
    private readonly TrayIcon _claudeIcon;
    private readonly TrayIcon _copilotIcon;
    private readonly PillHost _pill = new();
    private readonly ProviderHost _claude;
    private readonly ProviderHost _copilot;
    private readonly CopilotProvider _copilotProvider;
    private readonly FileSystemWatcher _watcher;
    private readonly DispatcherTimer _debounce;
    private readonly DispatcherTimer _tick;
    private readonly string _usagePath = AppPaths.UsageFile();
    private static readonly string? Collect = Environment.GetEnvironmentVariable("SOBREIRO_SPIKE_COLLECT");
    private FlyoutWindow? _flyout;
    private bool _copilotDrawn;
    private bool _disposed;

    public TrayController()
    {
        var dataDir = AppPaths.DataDirectory();
        Directory.CreateDirectory(dataDir);

        var claudeHttp = new HttpClient(LiveUsageClient.CreateHandler());
        _claude = new ProviderHost(claudeHttp, new ClaudeProvider(
            new LiveUsageClient(claudeHttp, ProviderHost.UserAgent()),
            ClaudePaths.CredentialsFile(Environment.GetEnvironmentVariable), _usagePath, _ => { }), () => Refresh());

        var copilotHttp = new HttpClient(LiveUsageClient.CreateHandler());
        IReadOnlyList<CopilotSignIn> signIns =
        [
            new(CopilotSourceKind.Env, () => new TokenSearch(CopilotToken.FromEnvironment(Environment.GetEnvironmentVariable))),
            new(CopilotSourceKind.CredentialManager, CredentialStore.CopilotTokens),
            new(CopilotSourceKind.Gh, () => new TokenSearch(GhCli.Token())),
        ];
        _copilotProvider = new CopilotProvider(
            new CopilotUsageClient(copilotHttp, ProviderHost.UserAgent()), signIns,
            CopilotCache.DefaultPath(Environment.GetEnvironmentVariable), _ => { });
        _copilot = new ProviderHost(copilotHttp, _copilotProvider, () => Refresh());

        _claudeIcon = new TrayIcon { Menu = Menu(), IsVisible = true };
        _claudeIcon.Clicked += OnIconClicked;
        _copilotIcon = new TrayIcon { Menu = Menu(), IsVisible = false };
        _copilotIcon.Clicked += OnIconClicked;
        TrayIcon.SetIcons(Application.Current!, [_claudeIcon, _copilotIcon]);

        _pill.LeftClicked += OnPillLeftClicked;
        _pill.RightClicked += () => Spike.Trace("pill right click: the toolkit's tray menu cannot be opened from here");

        _debounce = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(250) };
        _debounce.Tick += (_, _) =>
        {
            _debounce.Stop();
            Refresh();
        };
        _tick = new DispatcherTimer { Interval = TimeSpan.FromSeconds(30) };
        _tick.Tick += (_, _) =>
        {
            Guard("live usage", () => _claude.Tick(DateTimeOffset.Now));
            Guard("copilot usage", () => _copilot.Tick(DateTimeOffset.Now));
            Refresh();
            // The runtime's first collection comes only after tens of MB of garbage on a PC with a large
            // processor cache, and an idle tray makes about 0.7 MB a minute: until then private memory
            // just rises. SOBREIRO_SPIKE_COLLECT=gen0 or full collects here, every 30 seconds.
            switch (Collect)
            {
                case "gen0": GC.Collect(0, GCCollectionMode.Forced, blocking: true); break;
                case "full": GC.Collect(); break;
            }
            // Every 30 seconds, for the trace: how much of the process's memory is the garbage collector's.
            var info = GC.GetGCMemoryInfo();
            using var self = System.Diagnostics.Process.GetCurrentProcess();
            Spike.Trace($"memory private={self.PrivateMemorySize64 / 1024} KB gc-committed={info.TotalCommittedBytes / 1024} KB " +
                $"gc-heap={info.HeapSizeBytes / 1024} KB live={GC.GetTotalMemory(false) / 1024} KB gen0={GC.CollectionCount(0)} gen2={GC.CollectionCount(2)} asked={TaskbarProbe.Asked}");
        };

        _watcher = new FileSystemWatcher(dataDir, "usage.json")
        {
            NotifyFilter = NotifyFilters.LastWrite | NotifyFilters.FileName | NotifyFilters.Size,
        };
        _watcher.Changed += OnFileEvent;
        _watcher.Created += OnFileEvent;
        _watcher.Renamed += OnFileEvent;
        _watcher.EnableRaisingEvents = true;

        Refresh();
        _tick.Start();
        _claude.Start(DateTimeOffset.Now);
        _copilot.Start(DateTimeOffset.Now);
        Task.Run(_copilotProvider.ReadCache).ContinueWith(task =>
        {
            _ = task.Exception;
            if (task.IsCompletedSuccessfully && task.Result is { } cache)
                Dispatcher.UIThread.Post(() =>
                {
                    if (_disposed) return;
                    _copilotProvider.TakeEarly(cache);
                    Refresh();
                });
        }, TaskScheduler.Default);
    }

    // The toolkit's own menu type: it draws the menu itself, in a window of its own.
    private NativeMenu Menu()
    {
        var style = new NativeMenu();
        foreach (var option in new[] { FlyoutStyle.Bars, FlyoutStyle.Battery, FlyoutStyle.Thermometer })
        {
            var item = new NativeMenuItem(option.ToString());
            item.Click += (_, _) => Guard("style", () =>
            {
                var path = AppPaths.ConfigFile();
                (AppConfig.Load(path) with { Style = option }).Save(path);
                Refresh();
            });
            style.Add(item);
        }
        var open = new NativeMenuItem("Open data folder");
        open.Click += (_, _) => Guard("open", () => Process.Start(new ProcessStartInfo(AppPaths.DataDirectory()) { UseShellExecute = true }));
        var quit = new NativeMenuItem("Quit");
        quit.Click += (_, _) => ((IClassicDesktopStyleApplicationLifetime)Application.Current!.ApplicationLifetime!).Shutdown();
        return
        [
            new NativeMenuItem("Sobreiro, spike of shell A") { IsEnabled = false },
            new NativeMenuItem("Style") { Menu = style },
            new NativeMenuItemSeparator(),
            open,
            quit,
        ];
    }

    private static AppConfig ConfigForDisplay()
    {
        try
        {
            return AppConfig.Load(AppPaths.ConfigFile());
        }
        catch (Exception)
        {
            return AppConfig.Default;
        }
    }

    private void OnIconClicked(object? sender, EventArgs e)
    {
        if (_flyout is { IsVisible: true })
        {
            _flyout.Hide();
            return;
        }
        if (_flyout is { HiddenJustNow: true }) return;
        var at = Native.Cursor();
        OpenFlyout(at.X, at.Y);
    }

    private void OnPillLeftClicked()
    {
        if (_disposed) return;
        if (_flyout is { IsVisible: true })
        {
            _flyout.Hide();
            return;
        }
        // The toolkit gives a clicked window the keyboard focus, so the press already closed the flyout.
        if (_flyout is { HiddenJustNow: true }) return;
        var at = Native.Cursor();
        OpenFlyout(at.X, at.Y);
    }

    private void OpenFlyout(int x, int y)
    {
        Guard("live usage", () => _claude.FlyoutOpened(DateTimeOffset.Now));
        Guard("copilot usage", () => _copilot.FlyoutOpened(DateTimeOffset.Now));
        Refresh((x, y));
    }

    private void OnFileEvent(object? sender, FileSystemEventArgs e) => Dispatcher.UIThread.Post(() =>
    {
        if (_disposed) return;
        _debounce.Stop();
        _debounce.Start();
    });

    private void Refresh((int X, int Y)? showAt = null)
    {
        if (_disposed) return;
        IReadOnlyList<Limit> claude;
        var now = DateTimeOffset.Now;
        try
        {
            claude = _claude.Limits(now);
        }
        catch (Exception)
        {
            return;
        }
        IReadOnlyList<Limit> copilot = [];
        Guard("copilot limits", () => copilot = _copilot.Limits(now));
        var showClaude = true;
        Guard("connection", () =>
        {
            var credentialsExist = File.Exists(ClaudePaths.CredentialsFile(Environment.GetEnvironmentVariable));
            // The spike never reads settings.json: with no numbers it counts as not connected.
            showClaude = LimitList.ShowClaude(
                LimitList.ClaudeOnPc(credentialsExist, claude.Count, ConnectionState.NotConnected), copilot.Count > 0);
        });
        var zone = TimeZoneInfo.Local;
        var culture = CultureInfo.CurrentCulture;
        var config = ConfigForDisplay();
        var shown = showClaude ? claude.Concat(copilot).ToList() : copilot.ToList();

        TaskbarColours? taskbarColours = null;
        Guard("colours", () => taskbarColours = TaskbarColours.For(
            WindowsTheme.IsLight(), WindowsTheme.AccentTaskbar(), config.BarColour, WindowsTheme.Accent()));
        if (taskbarColours is { } colours)
        {
            var iconSize = Native.SmallIconSize();
            Guard("icon", () =>
            {
                if (!showClaude) return;
                _claudeIcon.Icon = IconRenderer.Render(RingsModel.From(claude), colours, iconSize);
                _claudeIcon.ToolTipText = TooltipText.Render(claude, now, zone, culture, false);
            });
            Guard("copilot icon", () =>
            {
                if (copilot.Count == 0) return;
                _copilotIcon.Icon = IconRenderer.Render(RingsModel.Copilot(copilot), colours, iconSize);
                _copilotIcon.ToolTipText = TooltipText.RenderCopilot(copilot[0], now, zone, culture);
                _copilotDrawn = true;
            });
            Guard("pill", () => UpdatePill(shown, showClaude, now, zone, culture, config, colours));
        }
        Guard("icon visibility", () =>
        {
            var copilotVisible = copilot.Count > 0 && _copilotDrawn;
            _copilotIcon.IsVisible = copilotVisible;
            _claudeIcon.IsVisible = showClaude || !copilotVisible;
        });
        Guard("flyout", () => UpdateFlyout(shown, showClaude, now, zone, culture, config, showAt));
    }

    private void UpdatePill(
        IReadOnlyList<Limit> limits, bool showClaude, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture,
        AppConfig config, TaskbarColours colours)
    {
        var enabled = config.Show == ShowOption.TrayAndPill;
        _pill.SetEnabled(enabled);
        if (!enabled) return;
        var parts = new List<string>();
        if (showClaude) parts.Add(TooltipText.Render(limits, now, zone, culture, false));
        if (limits.FirstOrDefault(l => l.Kind == LimitKind.Credits) is { } credits)
            parts.Add(TooltipText.RenderCopilot(credits, now, zone, culture));
        _pill.Update(
            PillModel.From(limits, zone, showClaude, _claude.State, _claude.Misses, _copilot.State, _copilot.Misses, now),
            colours, string.Join("\n", parts));
    }

    // The flyout's window is made the first time it is opened, not at start.
    private void UpdateFlyout(
        IReadOnlyList<Limit> limits, bool showClaude, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture,
        AppConfig config, (int X, int Y)? showAt)
    {
        if (showAt is null && _flyout is not { IsVisible: true }) return;
        var dark = !WindowsTheme.IsLight();
        var fill = BarColour.Pick(config.BarColour, WindowsTheme.Accent(), dark).Colour;
        var note = _copilotProvider.Shown is { } reading
            ? CopilotStateText.Line(_copilot.State, reading, now, zone, culture)
            : null;
        var model = FlyoutModel.From(limits, now, zone, culture, _claude.State, false, note, showClaude, config.PaceDays);
        _flyout ??= new FlyoutWindow();
        _flyout.Render(FlyoutView.Build(model, config.Style, NightPalette.For(dark), fill));
        if (showAt is { } at) _flyout.ShowNear(at.X, at.Y);
    }

    private static void Guard(string part, Action action)
    {
        try
        {
            action();
        }
        catch (Exception ex)
        {
            Spike.Trace($"{part} failed: {ex.GetType().Name}");
        }
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        _tick.Stop();
        _debounce.Stop();
        _claude.Dispose();
        _copilot.Dispose();
        _watcher.EnableRaisingEvents = false;
        _watcher.Dispose();
        _pill.Dispose();
        Guard("flyout", () => _flyout?.Close());
        _claudeIcon.Dispose();
        _copilotIcon.Dispose();
    }
}
