using System.Net;
using Sobreiro.Core.Copilot;

namespace Sobreiro.Tests;

public class CopilotUsageClientTests
{
    private static readonly DateTimeOffset Now = new(2026, 11, 10, 12, 0, 0, TimeSpan.Zero);
    private const string Secret = "t0k3n";

    private const string Business = """
        {
          "copilot_plan": "business",
          "quota_reset_date_utc": "2026-12-01T00:00:00.000Z",
          "quota_snapshots": {
            "premium_interactions": {
              "entitlement": 15000, "remaining": 12355, "quota_remaining": 12355.8,
              "unlimited": false, "has_quota": true, "timestamp_utc": "2026-11-10T09:25:19.365Z"
            }
          }
        }
        """;

    private const string Free =
        "{\"quota_reset_at\":0,\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":0,\"has_quota\":false,\"remaining\":0}}}";

    private sealed class FakeHandler(Func<HttpRequestMessage, HttpResponseMessage> respond) : HttpMessageHandler
    {
        public List<HttpRequestMessage> Seen { get; } = [];

        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken ct)
        {
            Seen.Add(request);
            return Task.FromResult(respond(request));
        }
    }

    private sealed class ThrowingHandler(Exception e) : HttpMessageHandler
    {
        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken ct) =>
            throw e;
    }

    private sealed class SlowHandler : HttpMessageHandler
    {
        protected override async Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken ct)
        {
            await Task.Delay(TimeSpan.FromSeconds(30), ct);
            return new HttpResponseMessage(HttpStatusCode.OK);
        }
    }

    private static HttpResponseMessage Reply(HttpStatusCode code, string body = "") =>
        new(code) { Content = new StringContent(body) };

    private static async Task<(CopilotFetch Result, FakeHandler Handler)> Run(Func<HttpRequestMessage, HttpResponseMessage> respond)
    {
        var handler = new FakeHandler(respond);
        var client = new CopilotUsageClient(new HttpClient(handler), "Sobreiro/0.7.0");
        var result = await client.FetchAsync(Secret, Now, CancellationToken.None);
        return (result, handler);
    }

    [Fact]
    public async Task Sends_the_four_headers_and_nothing_else()
    {
        var (_, handler) = await Run(_ => Reply(HttpStatusCode.OK, Business));
        var request = Assert.Single(handler.Seen);
        Assert.Equal(HttpMethod.Get, request.Method);
        Assert.Equal("https://api.github.com/copilot_internal/user", request.RequestUri!.ToString());
        Assert.Equal(CopilotUsageClient.Endpoint, request.RequestUri.ToString());
        Assert.Equal("Bearer", request.Headers.Authorization!.Scheme);
        Assert.Equal(Secret, request.Headers.Authorization.Parameter);
        Assert.Contains(request.Headers.Accept, a => a.MediaType == "application/json");
        Assert.Equal("2025-04-01", Assert.Single(request.Headers.GetValues("X-GitHub-Api-Version")));
        Assert.Equal("Sobreiro/0.7.0", request.Headers.UserAgent.ToString());
        Assert.False(request.Headers.Contains("Editor-Version"));
        Assert.False(request.Headers.Contains("anthropic-beta"));
        Assert.Equal(
            ["Accept", "Authorization", "User-Agent", "X-GitHub-Api-Version"],
            request.Headers.Select(h => h.Key).Order().ToArray());
    }

    [Fact]
    public async Task Ok_with_a_budget_gives_the_reading()
    {
        var (result, _) = await Run(_ => Reply(HttpStatusCode.OK, Business));
        Assert.Equal(CopilotOutcome.Ok, result.Outcome);
        Assert.Equal(2644.2, result.Reading!.Used, 1);
        Assert.Null(result.RetryAfter);
    }

    [Fact]
    public async Task Ok_without_a_budget_is_NoBudget()
    {
        var (result, _) = await Run(_ => Reply(HttpStatusCode.OK, Free));
        Assert.Equal(CopilotOutcome.NoBudget, result.Outcome);
        Assert.Null(result.Reading);
    }

    [Theory]
    [InlineData(401)]
    [InlineData(403)]
    [InlineData(404)]
    public async Task Refusals(int code)
    {
        var (result, _) = await Run(_ => Reply((HttpStatusCode)code));
        Assert.Equal(CopilotOutcome.Refused, result.Outcome);
        Assert.Null(result.Reading);
    }

    [Fact]
    public async Task Too_many_requests_carries_retry_after()
    {
        var (result, _) = await Run(_ =>
        {
            var r = Reply(HttpStatusCode.TooManyRequests);
            r.Headers.TryAddWithoutValidation("Retry-After", "120");
            return r;
        });
        Assert.Equal(CopilotOutcome.RateLimited, result.Outcome);
        Assert.Equal(TimeSpan.FromMinutes(2), result.RetryAfter);
    }

    [Fact]
    public async Task Server_error_is_Failed()
    {
        var (result, _) = await Run(_ => Reply(HttpStatusCode.InternalServerError));
        Assert.Equal(CopilotOutcome.Failed, result.Outcome);
    }

    [Theory]
    [InlineData("{")]
    [InlineData("[1,2]")]
    [InlineData("")]
    public async Task Unparsable_200_is_Failed(string body)
    {
        var (result, _) = await Run(_ => Reply(HttpStatusCode.OK, body));
        Assert.Equal(CopilotOutcome.Failed, result.Outcome);
        Assert.Null(result.Reading);
    }

    [Fact]
    public async Task Redirect_is_Failed()
    {
        var (result, _) = await Run(_ =>
        {
            var r = Reply(HttpStatusCode.Found);
            r.Headers.Location = new Uri("https://elsewhere.example/");
            return r;
        });
        Assert.Equal(CopilotOutcome.Failed, result.Outcome);
    }

    [Fact]
    public async Task Network_error_is_Unreachable()
    {
        var client = new CopilotUsageClient(new HttpClient(new ThrowingHandler(new HttpRequestException("boom " + Secret))), "ua");
        var result = await client.FetchAsync(Secret, Now, CancellationToken.None);
        Assert.Equal(CopilotOutcome.Unreachable, result.Outcome);
        Assert.DoesNotContain(Secret, result.ToString());
    }

    [Fact]
    public async Task Timeout_is_Unreachable()
    {
        var client = new CopilotUsageClient(new HttpClient(new SlowHandler()), "ua", TimeSpan.FromMilliseconds(50));
        var result = await client.FetchAsync(Secret, Now, CancellationToken.None);
        Assert.Equal(CopilotOutcome.Unreachable, result.Outcome);
    }

    [Fact]
    public async Task Caller_cancellation_throws()
    {
        using var cts = new CancellationTokenSource();
        cts.Cancel();
        var client = new CopilotUsageClient(new HttpClient(new FakeHandler(_ => Reply(HttpStatusCode.OK, Business))), "ua");
        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => client.FetchAsync(Secret, Now, cts.Token));
    }

    [Fact]
    public async Task Token_never_appears_in_the_result()
    {
        foreach (var code in new[] { HttpStatusCode.OK, HttpStatusCode.Unauthorized, HttpStatusCode.TooManyRequests, HttpStatusCode.InternalServerError })
        {
            var (result, _) = await Run(_ => Reply(code, Business));
            Assert.DoesNotContain(Secret, result.ToString());
        }
    }
}
