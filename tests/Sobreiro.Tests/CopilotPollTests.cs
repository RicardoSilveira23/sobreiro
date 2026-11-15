using System.Net;
using Sobreiro.Core.Copilot;
using Sobreiro.Core.Live;
using Sobreiro.Core.Polling;

namespace Sobreiro.Tests;

public class CopilotPollTests
{
    private static readonly DateTimeOffset Now = new(2026, 11, 10, 12, 0, 0, TimeSpan.Zero);

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
        public List<string> Tokens { get; } = [];

        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken ct)
        {
            Tokens.Add(request.Headers.Authorization!.Parameter!);
            return Task.FromResult(respond(request));
        }
    }

    private static HttpResponseMessage Reply(HttpStatusCode code, string body = "") =>
        new(code) { Content = new StringContent(body) };

    private static HttpResponseMessage Answer(Dictionary<string, (HttpStatusCode, string)> byToken, HttpRequestMessage r)
    {
        var (code, body) = byToken[r.Headers.Authorization!.Parameter!];
        return Reply(code, body);
    }

    private static CopilotSignIn Source(CopilotSourceKind kind, params string[] tokens) => new(kind, () => tokens);

    private static CopilotSignIn EntryWithoutToken(CopilotSourceKind kind) => new(kind, () => new TokenSearch([], true));

    private static CopilotSignIn Throwing(CopilotSourceKind kind) =>
        new(kind, () => throw new InvalidOperationException("boom gho_secret"));

    private static async Task<(CopilotPollResult Result, FakeHandler Handler)> Run(
        Func<HttpRequestMessage, HttpResponseMessage> respond, params CopilotSignIn[] signIns)
    {
        var handler = new FakeHandler(respond);
        var client = new CopilotUsageClient(new HttpClient(handler), "Sobreiro/0.7.0");
        var result = await CopilotPoll.RunAsync(signIns, client, LiveSchedule.Start(Now), Now, CancellationToken.None);
        return (result, handler);
    }

    [Fact]
    public async Task First_accepted_token_wins_and_later_sources_are_not_called()
    {
        var (result, handler) = await Run(
            _ => Reply(HttpStatusCode.OK, Business),
            Source(CopilotSourceKind.Env, "A"),
            Throwing(CopilotSourceKind.Gh));
        Assert.Equal(ProviderState.Ok, result.State);
        Assert.NotNull(result.Reading);
        Assert.Equal("copilot Ok env=ok credential-manager=skipped gh=skipped", result.Log);
        Assert.Equal(Now, result.Schedule.LastSuccess);
        Assert.Equal(["A"], handler.Tokens);
    }

    [Fact]
    public async Task Refused_and_no_budget_fall_through()
    {
        var answers = new Dictionary<string, (HttpStatusCode, string)>
        {
            ["A"] = (HttpStatusCode.Unauthorized, ""),
            ["B"] = (HttpStatusCode.OK, Free),
            ["C"] = (HttpStatusCode.OK, Business),
        };
        var (result, handler) = await Run(
            r => Answer(answers, r),
            Source(CopilotSourceKind.Env, "A"),
            Source(CopilotSourceKind.CredentialManager, "B"),
            Source(CopilotSourceKind.Gh, "C"));
        Assert.Equal(ProviderState.Ok, result.State);
        Assert.Equal("copilot Ok env=refused credential-manager=no-budget gh=ok", result.Log);
        Assert.Equal(["A", "B", "C"], handler.Tokens);
    }

    [Fact]
    public async Task All_refused_is_Refused_with_a_five_minute_wait()
    {
        var (result, _) = await Run(
            _ => Reply(HttpStatusCode.Unauthorized),
            Source(CopilotSourceKind.Env, "A"),
            Source(CopilotSourceKind.Gh, "C"));
        Assert.Equal(ProviderState.Refused, result.State);
        Assert.Null(result.Reading);
        Assert.Equal("copilot Refused env=refused credential-manager=skipped gh=refused", result.Log);
        Assert.Equal(Now.AddMinutes(5), result.Schedule.NextDue);
        Assert.False(result.Schedule.FetchOnOpen(Now.AddMinutes(2)));
    }

    [Fact]
    public async Task No_tokens_anywhere_is_NoSignIn()
    {
        var (result, handler) = await Run(
            _ => Reply(HttpStatusCode.OK, Business),
            Source(CopilotSourceKind.Env),
            Source(CopilotSourceKind.CredentialManager),
            Source(CopilotSourceKind.Gh));
        Assert.Equal(ProviderState.NoSignIn, result.State);
        Assert.Equal("copilot NoSignIn env=none credential-manager=none gh=none", result.Log);
        Assert.Empty(handler.Tokens);
        Assert.Equal(Now.AddMinutes(5), result.Schedule.NextDue);
    }

    [Fact]
    public async Task Entry_without_a_token_is_logged_as_no_token()
    {
        var (result, handler) = await Run(
            _ => Reply(HttpStatusCode.OK, Business),
            Source(CopilotSourceKind.Env),
            EntryWithoutToken(CopilotSourceKind.CredentialManager),
            Source(CopilotSourceKind.Gh));
        Assert.Equal(ProviderState.NoSignIn, result.State);
        Assert.Equal("copilot NoSignIn env=none credential-manager=no-token gh=none", result.Log);
        Assert.Empty(handler.Tokens);
    }

    [Fact]
    public async Task Throwing_source_is_error_and_skipped()
    {
        var (result, handler) = await Run(
            _ => Reply(HttpStatusCode.OK, Business),
            Source(CopilotSourceKind.Env),
            Throwing(CopilotSourceKind.CredentialManager),
            Source(CopilotSourceKind.Gh, "C"));
        Assert.Equal(ProviderState.Ok, result.State);
        Assert.Equal("copilot Ok env=none credential-manager=error gh=ok", result.Log);
        Assert.DoesNotContain("boom", result.Log);
        Assert.Equal(["C"], handler.Tokens);
    }

    [Fact]
    public async Task Only_a_throwing_source_is_NoSignIn()
    {
        var (result, _) = await Run(_ => Reply(HttpStatusCode.OK, Business), Throwing(CopilotSourceKind.Gh));
        Assert.Equal(ProviderState.NoSignIn, result.State);
        Assert.Equal("copilot NoSignIn env=skipped credential-manager=skipped gh=error", result.Log);
    }

    [Fact]
    public async Task Rate_limit_stops_the_chain()
    {
        var (result, handler) = await Run(
            _ =>
            {
                var r = Reply(HttpStatusCode.TooManyRequests);
                r.Headers.TryAddWithoutValidation("Retry-After", "600");
                return r;
            },
            Source(CopilotSourceKind.Env, "A"),
            Throwing(CopilotSourceKind.Gh));
        Assert.Equal(ProviderState.RateLimited, result.State);
        Assert.Equal("copilot RateLimited env=rate-limited credential-manager=skipped gh=skipped", result.Log);
        Assert.Equal(Now.AddMinutes(10), result.Schedule.NextDue);
        Assert.Single(handler.Tokens);
    }

    [Fact]
    public async Task A_500_is_a_bad_answer_and_stops_the_chain()
    {
        var (result, handler) = await Run(
            _ => Reply(HttpStatusCode.InternalServerError),
            Source(CopilotSourceKind.Env, "A"),
            Throwing(CopilotSourceKind.Gh));
        Assert.Equal(ProviderState.BadAnswer, result.State);
        Assert.Equal("copilot BadAnswer env=failed credential-manager=skipped gh=skipped", result.Log);
        Assert.Equal(Now.AddMinutes(2), result.Schedule.NextDue);
        Assert.Single(handler.Tokens);
    }

    [Fact]
    public async Task Network_failure_is_offline_and_stops_the_chain()
    {
        var (result, handler) = await Run(
            _ => throw new HttpRequestException("down"),
            Source(CopilotSourceKind.Env, "A"),
            Throwing(CopilotSourceKind.Gh));
        Assert.Equal(ProviderState.Offline, result.State);
        Assert.Equal("copilot Offline env=unreachable credential-manager=skipped gh=skipped", result.Log);
        Assert.Equal(Now.AddMinutes(2), result.Schedule.NextDue);
        Assert.Single(handler.Tokens);
    }

    [Fact]
    public async Task Several_tokens_in_one_source_log_the_last_result()
    {
        var answers = new Dictionary<string, (HttpStatusCode, string)>
        {
            ["A"] = (HttpStatusCode.Unauthorized, ""),
            ["B"] = (HttpStatusCode.OK, Free),
        };
        var (result, _) = await Run(r => Answer(answers, r), Source(CopilotSourceKind.Gh, "A", "B"));
        Assert.Equal(ProviderState.Refused, result.State);
        Assert.Equal("copilot Refused env=skipped credential-manager=skipped gh=no-budget", result.Log);
    }

    [Fact]
    public async Task Log_never_holds_a_token()
    {
        var (result, _) = await Run(
            _ => Reply(HttpStatusCode.Unauthorized),
            Source(CopilotSourceKind.Env, "gho_made_up_one"),
            Source(CopilotSourceKind.Gh, "gho_made_up_two"));
        Assert.DoesNotContain("gho_", result.Log);
    }

    [Fact]
    public async Task Cancellation_is_the_only_exception()
    {
        var handler = new FakeHandler(_ => Reply(HttpStatusCode.OK, Business));
        var client = new CopilotUsageClient(new HttpClient(handler), "Sobreiro/0.7.0");
        using var cts = new CancellationTokenSource();
        await cts.CancelAsync();
        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => CopilotPoll.RunAsync(
            [Source(CopilotSourceKind.Env, "A")], client, LiveSchedule.Start(Now), Now, cts.Token));
    }
}
