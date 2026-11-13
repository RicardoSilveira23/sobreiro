using System.Runtime.InteropServices;
using System.Windows;
using Sobreiro.Core;
using Sobreiro.Core.Diagnostics;
using Sobreiro.Core.Migration;
using Application = System.Windows.Application;

namespace Sobreiro;

public partial class App : Application
{
    private const int AsfwAny = -1;
    private const int RepeatLimit = 20;

    private Mutex? _singleInstance;
    private EventWaitHandle? _showEvent;
    private RegisteredWaitHandle? _showWait;
    private TrayController? _tray;
    private DiagnosticsLog? _log;
    private readonly LogBudget _dispatcherBudget = new(RepeatLimit);
    private readonly LogBudget _unobservedBudget = new(RepeatLimit);

    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        // --snapshot renders the look sheet and exits: no tray icon, no mutex.
        if (e.Args.Length > 0 && e.Args[0] == "--snapshot")
        {
            Shutdown(e.Args.Length == 2 ? Snapshot.Write(e.Args[1]) : Snapshot.Usage());
            return;
        }

        // SPIKE: --spike-pill <folder> draws sample pills with WPF and by hand, side by side, and exits.
        if (e.Args.Length == 2 && e.Args[0] == "--spike-pill")
        {
            Shutdown(Pill.SpikeSheet.Write(e.Args[1]));
            return;
        }

        // --media renders the product pictures and exits: no tray icon, no mutex, no network.
        if (e.Args.Length > 0 && e.Args[0] == "--media")
        {
            var ok = e.Args.Length == 2 || (e.Args.Length == 4 && e.Args[2] == "--plate");
            Shutdown(ok ? Media.MediaMode.Write(e.Args[1], e.Args.Length == 4 ? e.Args[3] : null) : Media.MediaMode.Usage());
            return;
        }

        _log = new DiagnosticsLog(AppPaths.LogFile());
        var id = AppPaths.InstanceId();
        AppDomain.CurrentDomain.UnhandledException += OnUnhandledException;
        TaskScheduler.UnobservedTaskException += OnUnobservedTaskException;

        // One tray per data folder. Every launch creates or opens the show event before it looks at the
        // mutex, so the event exists whenever a first tray does: a second launch that comes while the first
        // is still starting leaves it signalled, and the first opens its flyout once it is ready.
        var show = CreateShowEvent($@"Local\{id}.Show");
        _singleInstance = new Mutex(true, $@"Local\{id}.Tray", out var createdNew);
        if (!createdNew)
        {
            _singleInstance.Dispose();
            _singleInstance = null;
            AskFirstToShow(show);
            show?.Dispose();
            _log.Write("already-running");
            Shutdown(0);
            return;
        }

        // First run after the rename from OpenHeadroom. It runs before anything is written to the data
        // folder (the log included), so the old folder can be moved to the new name.
        var migration = LegacyMigrationHost.Run();
        if (migration.OldAppRunning)
        {
            show?.Dispose();
            _singleInstance.ReleaseMutex();
            _singleInstance.Dispose();
            _singleInstance = null;
            _log.Write("old-app-running");
            ShowMessage(LegacyMigration.StillRunningMessage, System.Windows.Forms.MessageBoxIcon.Information);
            Shutdown(0);
            return;
        }
        _log.Write($"start version={typeof(App).Assembly.GetName().Version?.ToString(3)} instance={id}");
        _log.Write($"migration data-folder={migration.DataFolder} not-copied={migration.FilesNotCopied} run-value={migration.RunValue}");
        // Off the UI thread: it may compare and write ~17 MB and retry for about a second.
        var log = _log;
        Task.Run(() => log.Write("bridge " + EmbeddedBridge.RefreshAtStart()));
        _showEvent = show;
        try
        {
            _tray = new TrayController(e => _log?.Write(e));
        }
        catch (Exception ex)
        {
            var logged = _log.Write("startup-failed", ex);
            // No icon means no way to quit: release the mutex so a later launch can try again.
            _showEvent?.Dispose();
            _showEvent = null;
            _singleInstance.ReleaseMutex();
            _singleInstance.Dispose();
            _singleInstance = null;
            var type = ex.GetType().Name;
            ShowMessage(
                logged
                    ? DiagnosticsMessage.StartupFailed(type, _log.FilePath)
                    : DiagnosticsMessage.StartupFailedUnlogged(type, AppPaths.DataDirectory()),
                System.Windows.Forms.MessageBoxIcon.Error);
            Shutdown(1);
            return;
        }
        // The wait runs on the thread pool: open the flyout on the UI thread.
        if (_showEvent is not null)
            _showWait = ThreadPool.RegisterWaitForSingleObject(
                _showEvent, (_, _) => Dispatcher.BeginInvoke(() => _tray?.ShowFlyoutByTray()), null, Timeout.Infinite, false);
        // Last line of defence: a stray exception on a UI callback must not kill the tray.
        DispatcherUnhandledException += (_, args) =>
        {
            args.Handled = true;
            if (_dispatcherBudget.Next("dispatcher-exception handled") is { } entry) _log.Write(entry, args.Exception);
        };
    }

    private static void ShowMessage(string text, System.Windows.Forms.MessageBoxIcon icon)
    {
        try
        {
            System.Windows.Forms.MessageBox.Show(text, "Sobreiro", System.Windows.Forms.MessageBoxButtons.OK, icon);
        }
        catch (Exception)
        {
            // No desktop to show it on: the exit code still says what happened.
        }
    }

    // Without the event the tray still runs; a second launch then just leaves.
    private EventWaitHandle? CreateShowEvent(string name)
    {
        try
        {
            return new EventWaitHandle(false, EventResetMode.AutoReset, name);
        }
        catch (Exception ex)
        {
            _log?.Write("show-event-failed", ex);
            return null;
        }
    }

    private void AskFirstToShow(EventWaitHandle? show)
    {
        if (show is null) return;
        try
        {
            // This launch may bring a window to the front; let the first tray do it instead.
            AllowSetForegroundWindow(AsfwAny);
            show.Set();
        }
        catch (Exception ex)
        {
            _log?.Write("show-request-failed", ex);
        }
    }

    private void OnUnhandledException(object sender, UnhandledExceptionEventArgs args) =>
        _log?.Write($"unhandled-exception terminating={(args.IsTerminating ? "true" : "false")}", args.ExceptionObject);

    private void OnUnobservedTaskException(object? sender, UnobservedTaskExceptionEventArgs args)
    {
        if (_unobservedBudget.Next("unobserved-task-exception") is { } entry) _log?.Write(entry, args.Exception);
    }

    // WPF quits at WM_QUERYENDSESSION itself, so this is the only session event that arrives.
    protected override void OnSessionEnding(SessionEndingCancelEventArgs e)
    {
        _log?.Write($"session-ending reason={e.ReasonSessionEnding}");
        base.OnSessionEnding(e);
    }

    protected override void OnExit(ExitEventArgs e)
    {
        _log?.Write($"exit code={e.ApplicationExitCode}");
        _showWait?.Unregister(null);
        _tray?.Dispose();
        _showEvent?.Dispose();
        if (_singleInstance is not null)
        {
            _singleInstance.ReleaseMutex();
            _singleInstance.Dispose();
        }
        base.OnExit(e);
    }

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool AllowSetForegroundWindow(int processId);
}
