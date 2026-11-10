using System.Net;
using System.Net.Http.Headers;
using Sobreiro.Core.Usage;

namespace Sobreiro.Core.Live;

public enum LiveOutcome { Ok, Unauthorized, RateLimited, Failed, Unreachable }

public sealed record LiveResult(LiveOutcome Outcome, RateLimits? Limits, TimeSpan? RetryAfter);

/// <summary>
/// The app's one network call: GET /api/oauth/usage with Claude Code's access token. Redirects are never followed
/// by this class (a 3xx is Failed; the caller's handler must not auto-redirect either). The token goes only into
/// the Authorization header, never into a result or an exception.
/// </summary>
public sealed class LiveUsageClient
{
    public const string Endpoint = "https://api.anthropic.com/api/oauth/usage";

    public static readonly TimeSpan Timeout = TimeSpan.FromSeconds(10);

    private static readonly TimeSpan MaxRetryAfter = TimeSpan.FromHours(6);

    private readonly HttpClient _http;
    private readonly string _userAgent;
    private readonly TimeSpan _timeout;

    public LiveUsageClient(HttpClient http, string userAgent) : this(http, userAgent, Timeout)
    {
    }

    internal LiveUsageClient(HttpClient http, string userAgent, TimeSpan timeout)
    {
        _http = http;
        _userAgent = userAgent;
        _timeout = timeout;
    }

    /// <summary>The handler the app's HttpClient must be built from: no redirects (the token must not follow one) and no cookies.</summary>
    public static HttpMessageHandler CreateHandler() => new SocketsHttpHandler { AllowAutoRedirect = false, UseCookies = false };

    private static LiveResult Of(LiveOutcome outcome) => new(outcome, null, null);

    public async Task<LiveResult> FetchAsync(AccessToken token, DateTimeOffset now, CancellationToken ct)
    {
        ct.ThrowIfCancellationRequested();

        // HttpRequestMessage.ToString() prints the Authorization header, so the request must never leave FetchAsync.
        using var request = new HttpRequestMessage(HttpMethod.Get, Endpoint);
        try
        {
            request.Headers.Authorization = new AuthenticationHeaderValue("Bearer", token.Value);
            request.Headers.TryAddWithoutValidation("anthropic-beta", "oauth-2025-04-20");
            request.Headers.Accept.Add(new MediaTypeWithQualityHeaderValue("application/json"));
            request.Headers.TryAddWithoutValidation("User-Agent", _userAgent);
        }
        catch (FormatException)
        {
            // Its message could quote the header value, so it goes no further.
            return Of(LiveOutcome.Failed);
        }

        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(ct);
        timeout.CancelAfter(_timeout);
        try
        {
            using var response = await _http.SendAsync(request, timeout.Token).ConfigureAwait(false);
            return await Interpret(response, now, timeout.Token).ConfigureAwait(false);
        }
        catch (OperationCanceledException) when (!ct.IsCancellationRequested)
        {
            return Of(LiveOutcome.Unreachable);
        }
        catch (Exception e) when (e is HttpRequestException or IOException)
        {
            return Of(LiveOutcome.Unreachable);
        }
        catch (Exception) when (!ct.IsCancellationRequested)
        {
            // Only the caller's cancellation may leave this method; the exception text is dropped on purpose.
            return Of(LiveOutcome.Failed);
        }
    }

    private static async Task<LiveResult> Interpret(HttpResponseMessage response, DateTimeOffset now, CancellationToken ct)
    {
        switch (response.StatusCode)
        {
            case HttpStatusCode.Unauthorized:
            case HttpStatusCode.Forbidden:
                return Of(LiveOutcome.Unauthorized);
            case HttpStatusCode.TooManyRequests:
                return new LiveResult(LiveOutcome.RateLimited, null, RetryAfter(response, now));
        }
        if (!response.IsSuccessStatusCode) return Of(LiveOutcome.Failed);

        var body = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
        var limits = UsageApiJson.Parse(body);
        return limits is null ? Of(LiveOutcome.Failed) : new LiveResult(LiveOutcome.Ok, limits, null);
    }

    internal static TimeSpan? RetryAfter(HttpResponseMessage response, DateTimeOffset now)
    {
        var header = response.Headers.RetryAfter;
        TimeSpan? wait = null;
        if (header?.Delta is { } delta) wait = delta;
        else if (header?.Date is { } date) wait = date - now;
        if (wait is not { } w) return null;
        return w < TimeSpan.Zero ? TimeSpan.Zero : w > MaxRetryAfter ? MaxRetryAfter : w;
    }
}
