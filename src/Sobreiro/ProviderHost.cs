using System.Diagnostics;
using System.Net.Http;
using System.Reflection;
using System.Windows.Threading;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Polling;

namespace Sobreiro;

/// <summary>
/// One provider's poller on the UI thread: the HttpClient, the hop off the thread and back, and the redraw. The
/// timing is <see cref="ProviderPoller"/>; what a poll does is the provider's.
/// </summary>
internal sealed class ProviderHost : IDisposable
{
    private readonly Dispatcher _dispatcher = Dispatcher.CurrentDispatcher;
    private readonly HttpClient _http;
    private readonly ProviderPoller _poller;
    private bool _disposed;

    public ProviderHost(HttpClient http, IProvider provider, Action changed)
    {
        _http = http;
        _poller = new ProviderPoller(provider, Run, changed, line => Debug.WriteLine("Sobreiro: " + line));
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
            _ = task.Exception; // observed here, so a poll that ends after shutdown raises nothing
            _dispatcher.BeginInvoke(() => run.Done(task, DateTimeOffset.Now));
        }, TaskScheduler.Default);

    // Sobreiro/0.3.0: the informational version without its "+commit" part, else the assembly version.
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
