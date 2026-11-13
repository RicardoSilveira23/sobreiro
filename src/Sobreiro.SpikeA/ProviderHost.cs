using System.Reflection;
using Avalonia.Threading;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Polling;

namespace Sobreiro.SpikeA;

/// <summary>SPIKE (shell candidate A): today's provider host, on the toolkit's dispatcher.</summary>
internal sealed class ProviderHost : IDisposable
{
    private readonly HttpClient _http;
    private readonly ProviderPoller _poller;
    private bool _disposed;

    public ProviderHost(HttpClient http, IProvider provider, Action changed)
    {
        _http = http;
        _poller = new ProviderPoller(provider, Run, changed, _ => { });
    }

    public ProviderState State => _poller.State;

    public int Misses => _poller.Misses;

    public IReadOnlyList<Limit> Limits(DateTimeOffset now) => _poller.Provider.Limits(now);

    public void Start(DateTimeOffset now) => _poller.Start(now);

    public void Tick(DateTimeOffset now) => _poller.Tick(now);

    public void FlyoutOpened(DateTimeOffset now) => _poller.FlyoutOpened(now);

    // The poll runs off the UI thread; its answer is handled on it.
    private void Run(PollRun run) =>
        Task.Run(run.RunAsync).ContinueWith(task =>
        {
            _ = task.Exception;
            Dispatcher.UIThread.Post(() => run.Done(task, DateTimeOffset.Now));
        }, TaskScheduler.Default);

    public static string UserAgent()
    {
        var assembly = typeof(ProviderHost).Assembly;
        var version = assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion.Split('+')[0];
        if (string.IsNullOrEmpty(version)) version = assembly.GetName().Version?.ToString(3) ?? "0";
        return "Sobreiro/" + version;
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        _poller.Dispose();
        _http.Dispose();
    }
}
