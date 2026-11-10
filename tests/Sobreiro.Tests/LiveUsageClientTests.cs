using System.Net;
using Sobreiro.Core.Live;

namespace Sobreiro.Tests;

public class LiveUsageClientTests
{
    private static readonly DateTimeOffset Now = new(2026, 11, 9, 12, 0, 0, TimeSpan.Zero);
    private const string Secret = "t0k3n";
    private const string Body = "{\"limits\":[{\"group\":\"session\",\"percent\":27,\"resets_at\":\"2026-11-09T14:00:00Z\"}]}";

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

    private static HttpResponseMessage Reply(HttpStatusCode code, string body = "") =>
        new(code) { Content = new StringContent(body) };

    private static async Task<(LiveResult Result, FakeHandler Handler)> Run(Func<HttpRequestMessage, HttpResponseMessage> respond)
    {
        var handler = new FakeHandler(respond);
        var client = new LiveUsageClient(new HttpClient(handler), "Sobreiro/0.3.0");
        var result = await client.FetchAsync(new AccessToken(Secret, null), Now, CancellationToken.None);
        return (result, handler);
    }

    [Fact]
    public async Task A_200_gives_limits_and_sends_the_exact_request()
    {
        var (result, handler) = await Run(_ => Reply(HttpStatusCode.OK, Body));
        Assert.Equal(LiveOutcome.Ok, result.Outcome);
        Assert.Equal(27, result.Limits!.FiveHour!.UsedPercentage);

        var request = Assert.Single(handler.Seen);
        Assert.Equal(HttpMethod.Get, request.Method);
        Assert.Equal("https://api.anthropic.com/api/oauth/usage", request.RequestUri!.ToString());
        Assert.Equal("https://api.anthropic.com/api/oauth/usage", LiveUsageClient.Endpoint);
        Assert.Equal("Bearer", request.Headers.Authorization!.Scheme);
        Assert.Equal(Secret, request.Headers.Authorization.Parameter);
        Assert.Equal("oauth-2025-04-20", Assert.Single(request.Headers.GetValues("anthropic-beta")));
        Assert.Equal("Sobreiro/0.3.0", request.Headers.UserAgent.ToString());
        Assert.Contains(request.Headers.Accept, a => a.MediaType == "application/json");
    }

    [Theory]
    [InlineData(HttpStatusCode.Unauthorized)]
    [InlineData(HttpStatusCode.Forbidden)]
    public async Task A_401_or_403_is_unauthorized(HttpStatusCode code)
    {
        var (result, _) = await Run(_ => Reply(code));
        Assert.Equal(LiveOutcome.Unauthorized, result.Outcome);
        Assert.Null(result.Limits);
    }

    [Fact]
    public async Task A_429_with_seconds_gives_that_retry_after()
    {
        var (result, _) = await Run(_ =>
        {
            var r = Reply(HttpStatusCode.TooManyRequests);
            r.Headers.TryAddWithoutValidation("Retry-After", "120");
            return r;
        });
        Assert.Equal(LiveOutcome.RateLimited, result.Outcome);
        Assert.Equal(TimeSpan.FromMinutes(2), result.RetryAfter);
    }

    [Fact]
    public async Task A_429_with_an_http_date_gives_the_gap_from_now()
    {
        var (result, _) = await Run(_ =>
        {
            var r = Reply(HttpStatusCode.TooManyRequests);
            r.Headers.TryAddWithoutValidation("Retry-After", Now.AddMinutes(3).ToString("R"));
            return r;
        });
        Assert.Equal(LiveOutcome.RateLimited, result.Outcome);
        Assert.InRange(result.RetryAfter!.Value.TotalSeconds, 179, 181);
    }

    [Fact]
    public async Task A_429_without_retry_after_has_none()
    {
        var (result, _) = await Run(_ => Reply(HttpStatusCode.TooManyRequests));
        Assert.Equal(LiveOutcome.RateLimited, result.Outcome);
        Assert.Null(result.RetryAfter);
    }

    [Fact]
    public async Task A_500_is_failed()
    {
        var (result, _) = await Run(_ => Reply(HttpStatusCode.InternalServerError));
        Assert.Equal(LiveOutcome.Failed, result.Outcome);
    }

    [Fact]
    public async Task A_redirect_is_failed_and_not_followed()
    {
        var (result, handler) = await Run(_ =>
        {
            var r = Reply(HttpStatusCode.Found);
            r.Headers.Location = new Uri("https://evil.example/steal");
            return r;
        });
        Assert.Equal(LiveOutcome.Failed, result.Outcome);
        Assert.Single(handler.Seen);
    }

    [Theory]
    [InlineData("not json")]
    [InlineData("{}")]
    [InlineData("")]
    public async Task A_200_with_an_unparseable_body_is_failed(string body)
    {
        var (result, _) = await Run(_ => Reply(HttpStatusCode.OK, body));
        Assert.Equal(LiveOutcome.Failed, result.Outcome);
        Assert.Null(result.Limits);
    }

    [Fact]
    public async Task Network_errors_and_timeouts_are_unreachable_without_the_token_in_the_text()
    {
        foreach (var e in new Exception[]
                 {
                     new HttpRequestException("boom " + Secret),
                     new TaskCanceledException("timed out " + Secret),
                     new IOException("io " + Secret),
                 })
        {
            var client = new LiveUsageClient(new HttpClient(new ThrowingHandler(e)), "Sobreiro/0.3.0");
            var result = await client.FetchAsync(new AccessToken(Secret, null), Now, CancellationToken.None);
            Assert.Equal(LiveOutcome.Unreachable, result.Outcome);
            Assert.DoesNotContain(Secret, result.ToString());
        }
    }

    [Fact]
    public async Task Cancelling_by_the_caller_propagates()
    {
        using var cts = new CancellationTokenSource();
        cts.Cancel();
        var client = new LiveUsageClient(new HttpClient(new FakeHandler(_ => Reply(HttpStatusCode.OK, Body))), "ua");
        await Assert.ThrowsAnyAsync<OperationCanceledException>(() =>
            client.FetchAsync(new AccessToken(Secret, null), Now, cts.Token));
    }

    [Fact]
    public async Task No_result_text_contains_the_token()
    {
        foreach (var code in new[] { HttpStatusCode.OK, HttpStatusCode.Unauthorized, HttpStatusCode.TooManyRequests, HttpStatusCode.Found, HttpStatusCode.InternalServerError })
        {
            var (result, _) = await Run(_ => Reply(code, Body));
            Assert.DoesNotContain(Secret, result.ToString());
        }
    }

    [Fact]
    public void The_timeout_is_ten_seconds() => Assert.Equal(TimeSpan.FromSeconds(10), LiveUsageClient.Timeout);

    private sealed class ExplodingContent : HttpContent
    {
        protected override Task SerializeToStreamAsync(Stream stream, TransportContext? context) =>
            throw new InvalidOperationException("cannot read body " + Secret);

        protected override bool TryComputeLength(out long length)
        {
            length = 0;
            return false;
        }
    }

    [Fact]
    public async Task A_non_network_exception_while_reading_the_body_is_failed_not_thrown()
    {
        var (result, _) = await Run(_ => new HttpResponseMessage(HttpStatusCode.OK) { Content = new ExplodingContent() });
        Assert.Equal(LiveOutcome.Failed, result.Outcome);
        Assert.DoesNotContain(Secret, result.ToString());
    }

    [Fact]
    public async Task A_bad_charset_in_a_200_never_throws()
    {
        var (result, _) = await Run(_ =>
        {
            var r = new HttpResponseMessage(HttpStatusCode.OK) { Content = new StringContent(Body) };
            r.Content.Headers.TryAddWithoutValidation("Content-Type", "application/json; charset=nope");
            return r;
        });
        Assert.True(result.Outcome is LiveOutcome.Ok or LiveOutcome.Failed);
        Assert.DoesNotContain(Secret, result.ToString());
    }

    [Fact]
    public void The_handler_neither_follows_redirects_nor_uses_cookies()
    {
        using var handler = Assert.IsType<SocketsHttpHandler>(LiveUsageClient.CreateHandler());
        Assert.False(handler.AllowAutoRedirect);
        Assert.False(handler.UseCookies);
    }

    [Fact]
    public async Task A_far_future_retry_after_is_capped_at_six_hours()
    {
        var (result, _) = await Run(_ =>
        {
            var r = Reply(HttpStatusCode.TooManyRequests);
            r.Headers.TryAddWithoutValidation("Retry-After", "Fri, 31 Dec 9999 23:59:59 GMT");
            return r;
        });
        Assert.Equal(TimeSpan.FromHours(6), result.RetryAfter);

        var (seconds, _) = await Run(_ =>
        {
            var r = Reply(HttpStatusCode.TooManyRequests);
            r.Headers.TryAddWithoutValidation("Retry-After", "999999999");
            return r;
        });
        Assert.Equal(TimeSpan.FromHours(6), seconds.RetryAfter);
    }

    private sealed class SlowHandler : HttpMessageHandler
    {
        protected override async Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken ct)
        {
            await Task.Delay(TimeSpan.FromSeconds(30), ct);
            return new HttpResponseMessage(HttpStatusCode.OK);
        }
    }

    [Fact]
    public async Task A_reply_slower_than_the_timeout_is_unreachable()
    {
        var client = new LiveUsageClient(new HttpClient(new SlowHandler()), "ua", TimeSpan.FromMilliseconds(100));
        var result = await client.FetchAsync(new AccessToken(Secret, null), Now, CancellationToken.None);
        Assert.Equal(LiveOutcome.Unreachable, result.Outcome);
    }
}
