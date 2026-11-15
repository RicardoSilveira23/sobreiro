using System.Net;
using System.Net.Http.Headers;
using System.Text.Json;
using Sobreiro.Core.Live;

namespace Sobreiro.Core.Copilot;

public enum CopilotOutcome { Ok, NoBudget, Refused, RateLimited, Failed, Unreachable }

public sealed record CopilotFetch(CopilotOutcome Outcome, CopilotReading? Reading, TimeSpan? RetryAfter);

/// <summary>
/// GET https://api.github.com/copilot_internal/user with one GitHub token. Redirects are never followed (a 3xx is
/// Failed; build the HttpClient from <see cref="LiveUsageClient.CreateHandler"/>). The token goes only into the
/// Authorization header, never into a result or an exception.
/// </summary>
public sealed class CopilotUsageClient
{
    public const string Endpoint = "https://api.github.com/copilot_internal/user";

    private readonly HttpClient _http;
    private readonly string _userAgent;
    private readonly TimeSpan _timeout;

    public CopilotUsageClient(HttpClient http, string userAgent) : this(http, userAgent, LiveUsageClient.Timeout)
    {
    }

    internal CopilotUsageClient(HttpClient http, string userAgent, TimeSpan timeout)
    {
        _http = http;
        _userAgent = userAgent;
        _timeout = timeout;
    }

    private static CopilotFetch Of(CopilotOutcome outcome) => new(outcome, null, null);

    public async Task<CopilotFetch> FetchAsync(string token, DateTimeOffset now, CancellationToken ct)
    {
        ct.ThrowIfCancellationRequested();

        // HttpRequestMessage.ToString() prints the Authorization header, so the request must never leave FetchAsync.
        using var request = new HttpRequestMessage(HttpMethod.Get, Endpoint);
        try
        {
            request.Headers.Authorization = new AuthenticationHeaderValue("Bearer", token);
            request.Headers.Accept.Add(new MediaTypeWithQualityHeaderValue("application/json"));
            request.Headers.TryAddWithoutValidation("X-GitHub-Api-Version", "2025-04-01");
            request.Headers.TryAddWithoutValidation("User-Agent", _userAgent);
        }
        catch (FormatException)
        {
            // Its message could quote the header value, so it goes no further.
            return Of(CopilotOutcome.Failed);
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
            return Of(CopilotOutcome.Unreachable);
        }
        catch (Exception e) when (e is HttpRequestException or IOException)
        {
            return Of(CopilotOutcome.Unreachable);
        }
        catch (Exception) when (!ct.IsCancellationRequested)
        {
            // Only the caller's cancellation may leave this method; the exception text is dropped on purpose.
            return Of(CopilotOutcome.Failed);
        }
    }

    private static async Task<CopilotFetch> Interpret(HttpResponseMessage response, DateTimeOffset now, CancellationToken ct)
    {
        switch (response.StatusCode)
        {
            case HttpStatusCode.Unauthorized:
            case HttpStatusCode.Forbidden:
            case HttpStatusCode.NotFound:
                return Of(CopilotOutcome.Refused);
            case HttpStatusCode.TooManyRequests:
                return new CopilotFetch(CopilotOutcome.RateLimited, null, LiveUsageClient.RetryAfter(response, now));
        }
        if (!response.IsSuccessStatusCode) return Of(CopilotOutcome.Failed);

        var body = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
        var reading = CopilotUserJson.Parse(body, now);
        if (reading is not null) return new CopilotFetch(CopilotOutcome.Ok, reading, null);
        return Of(IsJsonObject(body) ? CopilotOutcome.NoBudget : CopilotOutcome.Failed);
    }

    private static bool IsJsonObject(string body)
    {
        try
        {
            using var doc = JsonDocument.Parse(body);
            return doc.RootElement.ValueKind == JsonValueKind.Object;
        }
        catch (JsonException)
        {
            return false;
        }
    }
}
