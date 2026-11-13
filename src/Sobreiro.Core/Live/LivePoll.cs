using Sobreiro.Core.Usage;

namespace Sobreiro.Core.Live;

/// <summary>
/// One poll's answer: the next schedule, the footer state, the reading to merge (only on success) and the outcome's
/// name for the debug log. Nothing in it holds the token.
/// </summary>
public sealed record LivePollResult(LiveSchedule Schedule, LiveState State, RateLimits? Limits, string Outcome);

/// <summary>
/// One Live usage poll: read the sign-in, then ask Anthropic, but only with a token that is there, not expired and
/// not the one Anthropic last refused.
/// </summary>
public static class LivePoll
{
    /// <summary>Throws only an <see cref="OperationCanceledException"/> for <paramref name="ct"/>.</summary>
    public static Task<LivePollResult> RunAsync(
        string credentialsPath, LiveUsageClient client, LiveSchedule schedule, DateTimeOffset now, CancellationToken ct) =>
        RunAsync(at => ClaudeCredentials.Read(credentialsPath, at), client, schedule, now, ct);

    internal static async Task<LivePollResult> RunAsync(
        Func<DateTimeOffset, CredentialRead> readCredentials, LiveUsageClient client, LiveSchedule schedule,
        DateTimeOffset now, CancellationToken ct)
    {
        var read = readCredentials(now);
        if (read.Status != CredentialStatus.Ok || read.Token is not { } token)
        {
            // No request without a token that is there and not expired.
            var expired = read.Status == CredentialStatus.Expired;
            var name = read.Status == CredentialStatus.Ok ? nameof(CredentialStatus.Unreadable) : read.Status.ToString();
            return new LivePollResult(
                schedule.AfterNoCredentials(now), expired ? LiveState.Expired : LiveState.NoSignIn, null, name);
        }

        // The same expiry means the same sign-in Anthropic refused: wait for Claude Code to save a new one.
        if (schedule.Refuses(token.ExpiresAt))
            return new LivePollResult(schedule.AfterNoCredentials(now), LiveState.Refused, null, nameof(LiveState.Refused));

        var result = await client.FetchAsync(token, now, ct).ConfigureAwait(false);
        if (result.Outcome == LiveOutcome.Unauthorized)
        {
            return new LivePollResult(
                schedule.AfterRefused(token.ExpiresAt, now), LiveState.Refused, null, nameof(LiveOutcome.Unauthorized));
        }

        var state = result.Outcome switch
        {
            LiveOutcome.Ok => LiveState.Ok,
            LiveOutcome.RateLimited => LiveState.RateLimited,
            LiveOutcome.Unreachable => LiveState.Offline,
            _ => LiveState.BadAnswer,
        };
        return new LivePollResult(
            schedule.AfterFetch(result, now), state, result.Outcome == LiveOutcome.Ok ? result.Limits : null, result.Outcome.ToString());
    }
}
