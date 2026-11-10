using System.Net;
using Sobreiro.Core.Live;

namespace Sobreiro.Tests;

public class LivePollTests : IDisposable
{
    private static readonly DateTimeOffset Now = new(2026, 11, 9, 12, 0, 0, TimeSpan.Zero);
    private const string Secret = "sk-ant-oat01-SECRETVALUE";
    private const string Body = "{\"limits\":[{\"group\":\"session\",\"percent\":27,\"resets_at\":\"2026-11-09T14:00:00Z\"}]}";

    private readonly string _dir = Path.Combine(Path.GetTempPath(), "sobreiro-tests-" + Guid.NewGuid().ToString("N"));
    private readonly LiveSchedule _start = LiveSchedule.Start(Now);

    public void Dispose()
    {
        if (Directory.Exists(_dir)) Directory.Delete(_dir, recursive: true);
    }

    private sealed class CountingHandler(Func<HttpResponseMessage> respond) : HttpMessageHandler
    {
        public List<HttpRequestMessage> Seen { get; } = [];

        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken ct)
        {
            Seen.Add(request);
            return Task.FromResult(respond());
        }
    }

    // FetchAsync turns any handler exception into a result, so "fails if called" is checked by the count too.
    private static CountingHandler MustNotBeCalled() =>
        new(() => throw new Xunit.Sdk.XunitException("Live usage sent a request without a usable token."));

    private string CredentialsPath => Path.Combine(_dir, ".credentials.json");

    private string WriteCredentials(string text)
    {
        Directory.CreateDirectory(_dir);
        File.WriteAllText(CredentialsPath, text);
        return CredentialsPath;
    }

    private string ValidCredentials() => CredentialsExpiring(Now.AddHours(2));

    private string CredentialsExpiring(DateTimeOffset? expiresAt) =>
        WriteCredentials("{\"claudeAiOauth\":{\"accessToken\":\"" + Secret + "\",\"refreshToken\":\"r\""
            + (expiresAt is { } at ? ",\"expiresAt\":" + at.ToUnixTimeMilliseconds() : "") + "}}");

    private static Task<LivePollResult> Run(string path, HttpMessageHandler handler, LiveSchedule schedule, DateTimeOffset? now = null) =>
        LivePoll.RunAsync(path, new LiveUsageClient(new HttpClient(handler), "Sobreiro/0.3.0"), schedule, now ?? Now, CancellationToken.None);

    private static HttpResponseMessage Reply(HttpStatusCode code, string body = "") =>
        new(code) { Content = new StringContent(body) };

    [Fact]
    public async Task Missing_credentials_send_nothing_and_say_no_sign_in()
    {
        var handler = MustNotBeCalled();
        var result = await Run(CredentialsPath, handler, _start);

        Assert.Empty(handler.Seen);
        Assert.Equal(LiveState.NoSignIn, result.State);
        Assert.Null(result.Limits);
        Assert.Equal(_start.AfterNoCredentials(Now), result.Schedule);
        Assert.Equal("Missing", result.Outcome);
    }

    [Theory]
    [InlineData("not json")]
    [InlineData("{\"claudeAiOauth\":{}}")]
    [InlineData("{\"claudeAiOauth\":{\"accessToken\":\"\"}}")]
    public async Task Unreadable_credentials_send_nothing_and_say_no_sign_in(string text)
    {
        var handler = MustNotBeCalled();
        var result = await Run(WriteCredentials(text), handler, _start);

        Assert.Empty(handler.Seen);
        Assert.Equal(LiveState.NoSignIn, result.State);
        Assert.Null(result.Limits);
        Assert.Equal(_start.AfterNoCredentials(Now), result.Schedule);
        Assert.Equal("Unreadable", result.Outcome);
    }

    [Fact]
    public async Task Expired_credentials_send_nothing_and_say_expired()
    {
        var handler = MustNotBeCalled();
        var path = WriteCredentials("{\"claudeAiOauth\":{\"accessToken\":\"" + Secret + "\",\"expiresAt\":"
            + Now.AddMinutes(-1).ToUnixTimeMilliseconds() + "}}");
        var result = await Run(path, handler, _start);

        Assert.Empty(handler.Seen);
        Assert.Equal(LiveState.Expired, result.State);
        Assert.Null(result.Limits);
        Assert.Equal(_start.AfterNoCredentials(Now), result.Schedule);
        Assert.Equal("Expired", result.Outcome);
    }

    [Fact]
    public async Task An_ok_read_without_a_token_sends_nothing()
    {
        var handler = MustNotBeCalled();
        var client = new LiveUsageClient(new HttpClient(handler), "Sobreiro/0.3.0");
        var result = await LivePoll.RunAsync(_ => new CredentialRead(CredentialStatus.Ok, null), client, _start, Now, CancellationToken.None);

        Assert.Empty(handler.Seen);
        Assert.Equal(LiveState.NoSignIn, result.State);
        Assert.Null(result.Limits);
        Assert.Equal(_start.AfterNoCredentials(Now), result.Schedule);
        Assert.Equal("Unreadable", result.Outcome);
    }

    [Fact]
    public async Task The_read_is_asked_with_the_poll_time()
    {
        DateTimeOffset? asked = null;
        var client = new LiveUsageClient(new HttpClient(MustNotBeCalled()), "Sobreiro/0.3.0");
        await LivePoll.RunAsync(at => { asked = at; return new CredentialRead(CredentialStatus.Missing, null); }, client, _start, Now, CancellationToken.None);
        Assert.Equal(Now, asked);
    }

    [Fact]
    public async Task A_good_reply_gives_the_limits_and_state_ok()
    {
        var handler = new CountingHandler(() => Reply(HttpStatusCode.OK, Body));
        var result = await Run(ValidCredentials(), handler, _start);

        var request = Assert.Single(handler.Seen);
        Assert.Equal(Secret, request.Headers.Authorization!.Parameter);
        Assert.Equal(LiveState.Ok, result.State);
        Assert.Equal(27, result.Limits!.FiveHour!.UsedPercentage);
        Assert.Equal(Now, result.Schedule.LastSuccess);
        Assert.Equal(Now.AddMinutes(5), result.Schedule.NextDue);
        Assert.Equal("Ok", result.Outcome);
    }

    [Fact]
    public async Task A_429_is_rate_limited_and_waits()
    {
        var handler = new CountingHandler(() =>
        {
            var r = Reply(HttpStatusCode.TooManyRequests);
            r.Headers.TryAddWithoutValidation("Retry-After", "600");
            return r;
        });
        var result = await Run(ValidCredentials(), handler, _start);

        Assert.Equal(LiveState.RateLimited, result.State);
        Assert.Null(result.Limits);
        Assert.Equal(Now.AddMinutes(10), result.Schedule.NextDue);
        Assert.Equal(Now.AddMinutes(10), result.Schedule.WaitUntil);
        Assert.Equal("RateLimited", result.Outcome);
    }

    [Theory]
    [InlineData(HttpStatusCode.InternalServerError)]
    [InlineData(HttpStatusCode.Found)]
    public async Task Other_replies_are_failing_and_retry_in_two_minutes(HttpStatusCode code)
    {
        var path = ValidCredentials();
        var handler = new CountingHandler(() => Reply(code));
        var result = await Run(path, handler, _start);

        Assert.Single(handler.Seen);
        Assert.Equal(LiveState.Failing, result.State);
        Assert.Equal("Live usage can't reach Anthropic.", LiveStateText.Footer(result.State));
        Assert.Null(result.Limits);
        Assert.Equal(Now.AddMinutes(2), result.Schedule.NextDue);
        Assert.Equal("Failed", result.Outcome);

        // The same sign-in is tried again once the retry is due.
        var retry = new CountingHandler(() => Reply(code));
        await Run(path, retry, result.Schedule, Now.AddMinutes(2));
        Assert.Single(retry.Seen);
    }

    [Fact]
    public async Task An_unreachable_host_is_failing()
    {
        var path = ValidCredentials();
        var handler = new CountingHandler(() => throw new HttpRequestException("no route"));
        var result = await Run(path, handler, _start);

        Assert.Equal(LiveState.Failing, result.State);
        Assert.Equal("Live usage can't reach Anthropic.", LiveStateText.Footer(result.State));
        Assert.Null(result.Limits);
        Assert.Equal(Now.AddMinutes(2), result.Schedule.NextDue);
        Assert.Equal("Unreachable", result.Outcome);

        var retry = new CountingHandler(() => throw new HttpRequestException("no route"));
        await Run(path, retry, result.Schedule, Now.AddMinutes(2));
        Assert.Single(retry.Seen);
    }

    [Theory]
    [InlineData(HttpStatusCode.Unauthorized)]
    [InlineData(HttpStatusCode.Forbidden)]
    public async Task A_refused_sign_in_is_not_sent_again_until_the_file_gives_a_new_one(HttpStatusCode code)
    {
        var path = CredentialsExpiring(Now.AddHours(2));
        var refusing = new CountingHandler(() => Reply(code));
        var result = await Run(path, refusing, _start);

        Assert.Single(refusing.Seen);
        Assert.Equal(LiveState.Refused, result.State);
        Assert.Null(result.Limits);
        Assert.Equal(new RefusedSignIn(Now.AddHours(2)), result.Schedule.Refused);
        Assert.Equal(Now.AddMinutes(1), result.Schedule.NextDue);
        Assert.Equal("Unauthorized", result.Outcome);

        // Each due tick reads the file again but sends nothing while it gives the same expiry.
        var schedule = result.Schedule;
        foreach (var minutes in new[] { 1, 2, 3 })
        {
            var at = Now.AddMinutes(minutes);
            Assert.True(schedule.IsDue(at));
            var silent = MustNotBeCalled();
            result = await Run(path, silent, schedule, at);

            Assert.Empty(silent.Seen);
            Assert.Equal(LiveState.Refused, result.State);
            Assert.Equal("Refused", result.Outcome);
            Assert.Equal(at.AddMinutes(1), result.Schedule.NextDue);
            schedule = result.Schedule;
        }

        // Claude Code saved a new sign-in: it is sent at once.
        CredentialsExpiring(Now.AddHours(10));
        var ok = new CountingHandler(() => Reply(HttpStatusCode.OK, Body));
        result = await Run(path, ok, schedule, Now.AddMinutes(4));

        Assert.Single(ok.Seen);
        Assert.Equal(LiveState.Ok, result.State);
        Assert.Null(result.Schedule.Refused);
    }

    [Fact]
    public async Task A_refused_sign_in_without_an_expiry_waits_for_a_file_with_one()
    {
        var path = CredentialsExpiring(null);
        var result = await Run(path, new CountingHandler(() => Reply(HttpStatusCode.Unauthorized)), _start);
        Assert.Equal(new RefusedSignIn(null), result.Schedule.Refused);

        var silent = MustNotBeCalled();
        result = await Run(path, silent, result.Schedule, Now.AddMinutes(1));
        Assert.Empty(silent.Seen);
        Assert.Equal(LiveState.Refused, result.State);

        CredentialsExpiring(Now.AddHours(2));
        var ok = new CountingHandler(() => Reply(HttpStatusCode.OK, Body));
        result = await Run(path, ok, result.Schedule, Now.AddMinutes(2));
        Assert.Single(ok.Seen);
        Assert.Equal(LiveState.Ok, result.State);
    }

    [Fact]
    public async Task A_success_after_a_429_resets_the_wait()
    {
        var waiting = _start with { RateLimitStep = 2, WaitUntil = Now.AddMinutes(-1) };
        var result = await Run(ValidCredentials(), new CountingHandler(() => Reply(HttpStatusCode.OK, Body)), waiting);

        Assert.Equal(0, result.Schedule.RateLimitStep);
        Assert.Null(result.Schedule.WaitUntil);
    }

    [Fact]
    public async Task The_token_never_appears_in_a_result()
    {
        foreach (var code in new[] { HttpStatusCode.OK, HttpStatusCode.Unauthorized, HttpStatusCode.TooManyRequests })
        {
            var result = await Run(ValidCredentials(), new CountingHandler(() => Reply(code, Body)), _start);
            Assert.DoesNotContain(Secret, result.ToString());
        }
    }

    [Fact]
    public async Task The_callers_cancellation_leaves_as_a_cancellation()
    {
        using var cts = new CancellationTokenSource();
        await cts.CancelAsync();
        var client = new LiveUsageClient(new HttpClient(new CountingHandler(() => Reply(HttpStatusCode.OK, Body))), "Sobreiro/0.3.0");

        await Assert.ThrowsAnyAsync<OperationCanceledException>(
            () => LivePoll.RunAsync(ValidCredentials(), client, _start, Now, cts.Token));
    }
}
