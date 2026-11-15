using Sobreiro.Core.Live;

namespace Sobreiro.Tests;

public class LiveScheduleTests
{
    private static readonly DateTimeOffset T0 = new(2026, 11, 9, 12, 0, 0, TimeSpan.Zero);
    private static LiveResult Ok => new(LiveOutcome.Ok, null, null);
    private static LiveResult Limited(TimeSpan? after = null) => new(LiveOutcome.RateLimited, null, after);

    [Fact]
    public void Start_is_due_now()
    {
        var s = LiveSchedule.Start(T0);
        Assert.True(s.IsDue(T0));
        Assert.True(s.FetchOnOpen(T0));
    }

    [Fact]
    public void After_ok_the_next_is_five_minutes_on_and_opening_fetches_after_one_minute()
    {
        var s = LiveSchedule.Start(T0).AfterFetch(Ok, T0);
        Assert.Equal(T0.AddMinutes(5), s.NextDue);
        Assert.Equal(T0, s.LastSuccess);
        Assert.False(s.IsDue(T0.AddMinutes(4.9)));
        Assert.True(s.IsDue(T0.AddMinutes(5)));
        Assert.False(s.FetchOnOpen(T0));
        Assert.False(s.FetchOnOpen(T0.AddMinutes(1)));
        Assert.True(s.FetchOnOpen(T0.AddMinutes(1).AddSeconds(1)));
    }

    [Theory]
    [InlineData(LiveOutcome.Failed)]
    [InlineData(LiveOutcome.Unreachable)]
    public void After_a_failure_the_next_is_two_minutes_on(LiveOutcome outcome)
    {
        var s = LiveSchedule.Start(T0).AfterFetch(new LiveResult(outcome, null, null), T0);
        Assert.Equal(T0.AddMinutes(2), s.NextDue);
        Assert.Null(s.LastSuccess);
    }

    [Fact]
    public void A_failure_keeps_the_last_success()
    {
        var s = LiveSchedule.Start(T0).AfterFetch(Ok, T0)
            .AfterFetch(new LiveResult(LiveOutcome.Failed, null, null), T0.AddMinutes(5));
        Assert.Equal(T0, s.LastSuccess);
        Assert.False(s.FetchOnOpen(T0.AddMinutes(5)));
        Assert.True(s.FetchOnOpen(T0.AddMinutes(7)));
    }

    // A failing request is not resent on every flyout open: opening waits out the 2-minute retry like a 429 wait.
    [Theory]
    [InlineData(LiveOutcome.Failed)]
    [InlineData(LiveOutcome.Unreachable)]
    public void After_a_failure_opening_waits_for_the_retry(LiveOutcome outcome)
    {
        var s = LiveSchedule.Start(T0).AfterFetch(new LiveResult(outcome, null, null), T0);
        Assert.False(s.FetchOnOpen(T0));
        Assert.False(s.FetchOnOpen(T0.AddMinutes(2).AddSeconds(-1)));
        Assert.False(s.IsDue(T0.AddMinutes(2).AddSeconds(-1)));
        Assert.True(s.IsDue(T0.AddMinutes(2)));
        Assert.True(s.FetchOnOpen(T0.AddMinutes(2)));
    }

    [Fact]
    public void No_credentials_leave_opening_free_to_check_again()
    {
        var s = LiveSchedule.Start(T0).AfterFetch(new LiveResult(LiveOutcome.Failed, null, null), T0)
            .AfterNoCredentials(T0.AddSeconds(30));
        Assert.True(s.FetchOnOpen(T0.AddSeconds(31)));
    }

    [Fact]
    public void A_refusal_remembers_the_expiry_and_reads_the_file_again_in_a_minute()
    {
        var expiry = T0.AddHours(2);
        var s = LiveSchedule.Start(T0).AfterRefused(expiry, T0);
        Assert.Equal(new RefusedSignIn(expiry), s.Refused);
        Assert.Equal(T0.AddMinutes(1), s.NextDue);
        Assert.Null(s.WaitUntil);
        Assert.True(s.FetchOnOpen(T0)); // opening only reads the file: the poll sends nothing while it refuses
        Assert.True(s.Refuses(expiry));
        Assert.False(s.Refuses(expiry.AddHours(8)));
        Assert.False(s.Refuses(null));
    }

    [Fact]
    public void A_refusal_without_an_expiry_refuses_only_a_file_without_one()
    {
        var s = LiveSchedule.Start(T0).AfterRefused(null, T0);
        Assert.True(s.Refuses(null));
        Assert.False(s.Refuses(T0.AddHours(2)));
        Assert.False(LiveSchedule.Start(T0).Refuses(null));
    }

    [Fact]
    public void A_refusal_lasts_through_rereads_and_a_missing_file()
    {
        var expiry = T0.AddHours(2);
        var s = LiveSchedule.Start(T0).AfterRefused(expiry, T0).AfterNoCredentials(T0.AddMinutes(1)).AfterNoCredentials(T0.AddMinutes(2));
        Assert.True(s.Refuses(expiry));
        Assert.Equal(T0.AddMinutes(3), s.NextDue);
    }

    [Theory]
    [InlineData(LiveOutcome.Ok)]
    [InlineData(LiveOutcome.RateLimited)]
    [InlineData(LiveOutcome.Failed)]
    [InlineData(LiveOutcome.Unreachable)]
    public void A_fetch_with_another_sign_in_forgets_the_refusal(LiveOutcome outcome)
    {
        var s = LiveSchedule.Start(T0).AfterRefused(T0.AddHours(2), T0).AfterFetch(new LiveResult(outcome, null, null), T0.AddMinutes(1));
        Assert.Null(s.Refused);
    }

    [Fact]
    public void A_refusal_needs_the_expiry_so_after_fetch_turns_it_away() =>
        Assert.Throws<ArgumentException>(() => LiveSchedule.Start(T0).AfterFetch(new LiveResult(LiveOutcome.Unauthorized, null, null), T0));

    [Fact]
    public void Rate_limits_back_off_five_ten_twenty_thirty_thirty()
    {
        var s = LiveSchedule.Start(T0);
        var now = T0;
        foreach (var minutes in new[] { 5, 10, 20, 30, 30 })
        {
            s = s.AfterFetch(Limited(), now);
            Assert.Equal(now.AddMinutes(minutes), s.NextDue);
            Assert.Equal(now.AddMinutes(minutes), s.WaitUntil);
            Assert.False(s.FetchOnOpen(now.AddMinutes(minutes - 1)));
            now = now.AddMinutes(minutes);
        }
    }

    [Fact]
    public void A_long_retry_after_wins_over_the_step()
    {
        var s = LiveSchedule.Start(T0).AfterFetch(Limited(TimeSpan.FromMinutes(45)), T0);
        Assert.Equal(T0.AddMinutes(45), s.NextDue);
        Assert.Equal(T0.AddMinutes(45), s.WaitUntil);
    }

    [Fact]
    public void A_short_retry_after_loses_to_the_step()
    {
        var s = LiveSchedule.Start(T0).AfterFetch(Limited(TimeSpan.FromSeconds(30)), T0);
        Assert.Equal(T0.AddMinutes(5), s.NextDue);
    }

    [Fact]
    public void An_ok_resets_the_step_and_the_wait()
    {
        var s = LiveSchedule.Start(T0).AfterFetch(Limited(), T0).AfterFetch(Limited(), T0.AddMinutes(5))
            .AfterFetch(Ok, T0.AddMinutes(15));
        Assert.Equal(0, s.RateLimitStep);
        Assert.Null(s.WaitUntil);
        Assert.Equal(T0.AddMinutes(20), s.NextDue);
        s = s.AfterFetch(Limited(), T0.AddMinutes(20));
        Assert.Equal(T0.AddMinutes(25), s.NextDue);
    }

    [Fact]
    public void A_non_rate_limit_failure_does_not_reset_the_step()
    {
        var s = LiveSchedule.Start(T0).AfterFetch(Limited(), T0)
            .AfterFetch(new LiveResult(LiveOutcome.Failed, null, null), T0.AddMinutes(5))
            .AfterFetch(Limited(), T0.AddMinutes(7));
        Assert.Equal(T0.AddMinutes(17), s.NextDue);
    }

    [Fact]
    public void No_credentials_check_again_in_a_minute()
    {
        var s = LiveSchedule.Start(T0).AfterNoCredentials(T0);
        Assert.Equal(T0.AddMinutes(1), s.NextDue);
        Assert.False(s.IsDue(T0.AddSeconds(59)));
        Assert.True(s.IsDue(T0.AddMinutes(1)));
    }

    [Fact]
    public void After_no_reading_waits_five_minutes_and_blocks_open()
    {
        var s = LiveSchedule.Start(T0).AfterNoReading(T0);
        Assert.Equal(T0.AddMinutes(5), s.NextDue);
        Assert.False(s.IsDue(T0.AddMinutes(4.9)));
        Assert.True(s.IsDue(T0.AddMinutes(5)));
        Assert.False(s.FetchOnOpen(T0.AddMinutes(2)));
        Assert.True(s.FetchOnOpen(T0.AddMinutes(5)));
    }

    [Fact]
    public void Start_has_no_misses() => Assert.Equal(0, LiveSchedule.Start(T0).Misses);

    [Fact]
    public void A_success_after_misses_gives_zero()
    {
        var s = LiveSchedule.Start(T0).AfterNoReading(T0).AfterNoReading(T0.AddMinutes(5)).AfterFetch(Ok, T0.AddMinutes(10));
        Assert.Equal(0, s.Misses);
    }

    [Theory]
    [InlineData(LiveOutcome.RateLimited)]
    [InlineData(LiveOutcome.Failed)]
    [InlineData(LiveOutcome.Unreachable)]
    public void Each_failed_outcome_adds_a_miss(LiveOutcome outcome) =>
        Assert.Equal(1, LiveSchedule.Start(T0).AfterFetch(new LiveResult(outcome, null, null), T0).Misses);

    [Fact]
    public void No_credentials_refused_and_no_reading_each_add_a_miss()
    {
        Assert.Equal(1, LiveSchedule.Start(T0).AfterNoCredentials(T0).Misses);
        Assert.Equal(1, LiveSchedule.Start(T0).AfterRefused(T0.AddHours(1), T0).Misses);
        Assert.Equal(1, LiveSchedule.Start(T0).AfterNoReading(T0).Misses);
    }

    [Fact]
    public void Three_failures_in_a_row_give_three()
    {
        var failed = new LiveResult(LiveOutcome.Failed, null, null);
        var s = LiveSchedule.Start(T0).AfterFetch(failed, T0).AfterFetch(failed, T0.AddMinutes(2)).AfterFetch(failed, T0.AddMinutes(4));
        Assert.Equal(3, s.Misses);
    }
}
