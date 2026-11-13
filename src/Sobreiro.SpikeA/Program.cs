using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Themes.Fluent;

namespace Sobreiro.SpikeA;

/// <summary>SPIKE (shell candidate A): throwaway switches, read once from the environment, for the comparisons.</summary>
internal static class Spike
{
    /// <summary>SOBREIRO_SPIKE_RENDER=gpu draws through the graphics card; anything else in software.</summary>
    public static readonly bool Gpu = Environment.GetEnvironmentVariable("SOBREIRO_SPIKE_RENDER") == "gpu";

    /// <summary>SOBREIRO_SPIKE_UIA=none never asks UI Automation where the tray icons are.</summary>
    public static readonly bool AskUia = Environment.GetEnvironmentVariable("SOBREIRO_SPIKE_UIA") != "none";

    private static readonly string? TraceFile = Environment.GetEnvironmentVariable("SOBREIRO_SPIKE_TRACE");
    private static string? _lastTrace;

    /// <summary>SOBREIRO_SPIKE_TRACE=file: appends a line when it differs from the last one. Never holds a token.</summary>
    public static void Trace(string line)
    {
        if (TraceFile is null || line == _lastTrace) return;
        _lastTrace = line;
        try { File.AppendAllText(TraceFile, $"{DateTime.Now:HH:mm:ss.fff} {line}" + Environment.NewLine); } catch (Exception) { }
    }
}

internal static class Program
{
    public static string[] Args = [];

    [STAThread]
    public static int Main(string[] args)
    {
        Args = args;
        // The spike never runs on an installed copy's data: without a scratch folder it leaves.
        var tray = !(args.Length > 0 && args[0] is "--probe" or "--snapshot");
        if (tray && string.IsNullOrEmpty(Environment.GetEnvironmentVariable("SOBREIRO_DATA_DIR"))) return 3;
        return AppBuilder.Configure<App>()
            .UseWin32()
            .UseSkia()
            .UseHarfBuzz()
            .With(new Win32PlatformOptions
            {
                RenderingMode = Spike.Gpu
                    ? [Win32RenderingMode.AngleEgl, Win32RenderingMode.Software]
                    : [Win32RenderingMode.Software],
            })
            .StartWithClassicDesktopLifetime(args, ShutdownMode.OnExplicitShutdown);
    }
}

internal sealed class App : Application
{
    private Mutex? _singleInstance;
    private TrayController? _tray;

    public override void Initialize() => Styles.Add(new FluentTheme());

    public override void OnFrameworkInitializationCompleted()
    {
        var lifetime = (IClassicDesktopStyleApplicationLifetime)ApplicationLifetime!;
        if (Program.Args.Length > 0 && Program.Args[0] == "--probe")
        {
            Probe.Run();
        }
        else if (Program.Args.Length == 2 && Program.Args[0] == "--snapshot")
        {
            Snapshot.Write(Program.Args[1]);
            Avalonia.Threading.Dispatcher.UIThread.Post(() => lifetime.Shutdown(0));
        }
        else
        {
            // One tray per data folder, as today.
            _singleInstance = new Mutex(true, $@"Local\{Sobreiro.Core.AppPaths.InstanceId()}.Tray", out var createdNew);
            if (!createdNew)
            {
                _singleInstance.Dispose();
                _singleInstance = null;
                Avalonia.Threading.Dispatcher.UIThread.Post(() => lifetime.Shutdown(0));
                base.OnFrameworkInitializationCompleted();
                return;
            }
            Spike.Trace($"start render={(Spike.Gpu ? "gpu" : "software")} uia={(Spike.AskUia ? "com" : "none")}");
            _tray = new TrayController();
            lifetime.Exit += (_, _) =>
            {
                _tray?.Dispose();
                _singleInstance?.ReleaseMutex();
                _singleInstance?.Dispose();
            };
        }
        base.OnFrameworkInitializationCompleted();
    }
}
