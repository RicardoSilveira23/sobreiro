using Sobreiro.Core.Live;
using Sobreiro.Core.Polling;

namespace Sobreiro.Core.Copilot;

/// <summary>One poll's answer. <see cref="Log"/> names outcomes only; nothing in it holds a token.</summary>
public sealed record CopilotPollResult(LiveSchedule Schedule, ProviderState State, CopilotReading? Reading, string Log);

/// <summary>
/// One Copilot poll: go through the sign-ins in order and ask GitHub with each token until one is accepted. A
/// refusal or a plan without a budget goes on to the next token; a 429 or a failure stops the chain.
/// </summary>
public static class CopilotPoll
{
    /// <summary>Throws only an <see cref="OperationCanceledException"/> for <paramref name="ct"/>.</summary>
    public static async Task<CopilotPollResult> RunAsync(
        IReadOnlyList<CopilotSignIn> signIns, CopilotUsageClient client, LiveSchedule schedule,
        DateTimeOffset now, CancellationToken ct)
    {
        var results = new Dictionary<CopilotSourceKind, string>();
        var anyToken = false;

        foreach (var signIn in signIns)
        {
            ct.ThrowIfCancellationRequested();
            TokenSearch search;
            try
            {
                search = signIn.Tokens();
            }
            catch (Exception) when (!ct.IsCancellationRequested)
            {
                results[signIn.Kind] = "error"; // never the exception: its text could hold a token
                continue;
            }

            if (search.Tokens.Count == 0) results[signIn.Kind] = search.EntryFound ? "no-token" : "none";
            foreach (var token in search.Tokens)
            {
                anyToken = true;
                var fetch = await client.FetchAsync(token, now, ct).ConfigureAwait(false);
                switch (fetch.Outcome)
                {
                    case CopilotOutcome.Ok when fetch.Reading is { } reading:
                        results[signIn.Kind] = "ok";
                        return Done(
                            schedule.AfterFetch(new LiveResult(LiveOutcome.Ok, null, null), now),
                            ProviderState.Ok, reading, results);
                    case CopilotOutcome.Refused:
                        results[signIn.Kind] = "refused";
                        break;
                    case CopilotOutcome.NoBudget:
                        results[signIn.Kind] = "no-budget";
                        break;
                    case CopilotOutcome.RateLimited:
                        results[signIn.Kind] = "rate-limited";
                        return Done(
                            schedule.AfterFetch(new LiveResult(LiveOutcome.RateLimited, null, fetch.RetryAfter), now),
                            ProviderState.RateLimited, null, results);
                    case CopilotOutcome.Unreachable:
                        results[signIn.Kind] = "unreachable";
                        return Done(
                            schedule.AfterFetch(new LiveResult(LiveOutcome.Unreachable, null, null), now),
                            ProviderState.Offline, null, results);
                    default:
                        results[signIn.Kind] = "failed";
                        return Done(
                            schedule.AfterFetch(new LiveResult(LiveOutcome.Failed, null, null), now),
                            ProviderState.BadAnswer, null, results);
                }
            }
        }

        return Done(
            schedule.AfterNoReading(now), anyToken ? ProviderState.Refused : ProviderState.NoSignIn, null, results);
    }

    private static CopilotPollResult Done(
        LiveSchedule schedule, ProviderState state, CopilotReading? reading, Dictionary<CopilotSourceKind, string> results)
    {
        string Of(CopilotSourceKind kind) => results.GetValueOrDefault(kind, "skipped");
        var log = $"copilot {state} env={Of(CopilotSourceKind.Env)} " +
                  $"credential-manager={Of(CopilotSourceKind.CredentialManager)} gh={Of(CopilotSourceKind.Gh)}";
        return new CopilotPollResult(schedule, state, reading, log);
    }
}
