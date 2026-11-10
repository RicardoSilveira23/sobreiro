namespace Sobreiro.Core.Live;

/// <summary>The sign-in Anthropic refused, known by its expiry (not secret; null when the file had none).</summary>
public sealed record RefusedSignIn(DateTimeOffset? ExpiresAt);

/// <summary>
/// When the next Live usage fetch is due. Immutable: each event gives the next schedule. After a 429 the wait is
/// max(Retry-After, step) with step 5, 10, 20, 30, 30... minutes; a success resets the step. WaitUntil is set while
/// a wait after a 429 or a failure runs; opening the flyout does not cut it short. Refused holds the sign-in a
/// 401 or 403 turned down: it is not sent again, and the file is read every minute until it gives a new one.
/// </summary>
public sealed record LiveSchedule(
    DateTimeOffset NextDue, DateTimeOffset? LastSuccess, int RateLimitStep, DateTimeOffset? WaitUntil, RefusedSignIn? Refused = null)
{
    private static readonly TimeSpan Interval = TimeSpan.FromMinutes(5);
    private static readonly TimeSpan RetryInterval = TimeSpan.FromMinutes(2);
    private static readonly TimeSpan NoCredentialsInterval = TimeSpan.FromMinutes(1);
    private static readonly TimeSpan OpenFreshness = TimeSpan.FromMinutes(1);
    private static readonly int[] StepMinutes = [5, 10, 20, 30];

    public static LiveSchedule Start(DateTimeOffset now) => new(now, null, 0, null);

    public bool IsDue(DateTimeOffset now) => now >= NextDue;

    /// <summary>
    /// Opening the flyout fetches early when the last success is over a minute old, unless a wait after a 429 or a
    /// failure is running (so a failing request is not resent on every open).
    /// </summary>
    public bool FetchOnOpen(DateTimeOffset now) =>
        !(WaitUntil is { } wait && now < wait)
        && (LastSuccess is not { } last || now - last > OpenFreshness);

    /// <summary>True when the file still gives the sign-in Anthropic refused: send nothing.</summary>
    public bool Refuses(DateTimeOffset? expiresAt) => Refused is { } refused && refused.ExpiresAt == expiresAt;

    /// <summary>Any outcome but a refusal, which needs the token's expiry: <see cref="AfterRefused"/>.</summary>
    public LiveSchedule AfterFetch(LiveResult result, DateTimeOffset now)
    {
        switch (result.Outcome)
        {
            case LiveOutcome.Ok:
                return new LiveSchedule(now + Interval, now, 0, null);
            case LiveOutcome.Unauthorized:
                throw new ArgumentException("A refused sign-in goes through AfterRefused.", nameof(result));
            case LiveOutcome.RateLimited:
                var step = TimeSpan.FromMinutes(StepMinutes[Math.Min(RateLimitStep, StepMinutes.Length - 1)]);
                var wait = result.RetryAfter is { } after && after > step ? after : step;
                return this with
                {
                    NextDue = now + wait,
                    WaitUntil = now + wait,
                    RateLimitStep = Math.Min(RateLimitStep + 1, StepMinutes.Length),
                    Refused = null,
                };
            default:
                return this with { NextDue = now + RetryInterval, WaitUntil = now + RetryInterval, Refused = null };
        }
    }

    /// <summary>A 401 or 403 for the token with this expiry: nothing is sent until the file gives another one.</summary>
    public LiveSchedule AfterRefused(DateTimeOffset? expiresAt, DateTimeOffset now) =>
        AfterNoCredentials(now) with { Refused = new RefusedSignIn(expiresAt) };

    /// <summary>No sign-in to send (none, expired, or the refused one): read the file again in a minute.</summary>
    public LiveSchedule AfterNoCredentials(DateTimeOffset now) =>
        this with { NextDue = now + NoCredentialsInterval, WaitUntil = null };

    /// <summary>
    /// A Copilot poll that found nothing to show (no sign-in, or none GitHub accepted): ask again in five minutes,
    /// and opening the flyout does not ask sooner.
    /// </summary>
    public LiveSchedule AfterNoReading(DateTimeOffset now) =>
        this with { NextDue = now + Interval, WaitUntil = now + Interval, Refused = null };
}
