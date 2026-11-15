using Sobreiro.Core.Usage;

namespace Sobreiro.Tests;

public class UsageMergeTests
{
    private static readonly DateTimeOffset T = DateTimeOffset.Parse("2026-11-09T10:00:00Z");
    private static readonly DateTimeOffset SessionReset = T.AddHours(3);
    private static readonly DateTimeOffset WeeklyReset = T.AddDays(4);

    private static UsageSnapshot Stored(LimitWindow? five, LimitWindow? seven = null, SpendLimit? spend = null, DateTimeOffset? at = null) =>
        new(new RateLimits(five, seven, spend), at ?? T.AddMinutes(-5));

    private static RateLimits Incoming(LimitWindow? five, LimitWindow? seven = null, SpendLimit? spend = null) =>
        new(five, seven, spend);

    [Fact]
    public void Merge_with_nothing_stored_takes_reading_and_stamps_now()
    {
        var merged = UsageMerge.Merge(null,
            Incoming(new LimitWindow(10, SessionReset), new LimitWindow(20, WeeklyReset), new SpendLimit(30, WeeklyReset, 3, 10, "monthly")), T);

        Assert.Equal(T, merged.ObservedAt);
        Assert.Equal(T, merged.Limits.FiveHour!.ObservedAt);
        Assert.Equal(T, merged.Limits.SevenDay!.ObservedAt);
        Assert.Equal(T, merged.Limits.Spend!.ObservedAt);
    }

    [Fact]
    public void Merge_keeps_higher_reading_in_same_period()
    {
        var stored = Stored(new LimitWindow(60, SessionReset, T.AddMinutes(-5)));
        var merged = UsageMerge.Merge(stored, Incoming(new LimitWindow(40, SessionReset.AddSeconds(10))), T);

        Assert.Equal(new LimitWindow(60, SessionReset, T.AddMinutes(-5)), merged.Limits.FiveHour);
        Assert.Equal(T, merged.ObservedAt);
    }

    [Fact]
    public void Merge_takes_equal_or_higher_reading_in_same_period()
    {
        var stored = Stored(new LimitWindow(60, SessionReset, T.AddMinutes(-5)));
        var merged = UsageMerge.Merge(stored, Incoming(new LimitWindow(60, SessionReset)), T);

        Assert.Equal(new LimitWindow(60, SessionReset, T), merged.Limits.FiveHour);
    }

    [Fact]
    public void Merge_takes_lower_reading_from_a_later_period()
    {
        var stored = Stored(new LimitWindow(60, SessionReset, T.AddMinutes(-5)));
        var merged = UsageMerge.Merge(stored, Incoming(new LimitWindow(5, T.AddHours(8))), T);

        Assert.Equal(new LimitWindow(5, T.AddHours(8), T), merged.Limits.FiveHour);
    }

    [Fact]
    public void Merge_ignores_reading_from_an_earlier_period()
    {
        var stored = Stored(new LimitWindow(5, T.AddHours(8), T.AddMinutes(-5)));
        var merged = UsageMerge.Merge(stored, Incoming(new LimitWindow(70, SessionReset)), T);

        Assert.Equal(new LimitWindow(5, T.AddHours(8), T.AddMinutes(-5)), merged.Limits.FiveHour);
    }

    [Fact]
    public void Merge_takes_reading_when_either_reset_time_is_missing()
    {
        var withReset = Stored(new LimitWindow(60, SessionReset, T.AddMinutes(-5)));
        Assert.Equal(new LimitWindow(10, null, T), UsageMerge.Merge(withReset, Incoming(new LimitWindow(10, null)), T).Limits.FiveHour);

        var withoutReset = Stored(new LimitWindow(60, null, T.AddMinutes(-5)));
        Assert.Equal(new LimitWindow(10, SessionReset, T), UsageMerge.Merge(withoutReset, Incoming(new LimitWindow(10, SessionReset)), T).Limits.FiveHour);
    }

    [Fact]
    public void Merge_keeps_stored_window_missing_from_reading_until_its_reset_passes()
    {
        var stored = Stored(new LimitWindow(60, SessionReset, T.AddMinutes(-5)), new LimitWindow(30, WeeklyReset, T.AddMinutes(-5)));
        var merged = UsageMerge.Merge(stored, Incoming(new LimitWindow(61, SessionReset)), T);
        Assert.Equal(new LimitWindow(30, WeeklyReset, T.AddMinutes(-5)), merged.Limits.SevenDay);

        var expired = Stored(new LimitWindow(60, SessionReset, T.AddMinutes(-5)), new LimitWindow(30, T, T.AddMinutes(-5)));
        Assert.Null(UsageMerge.Merge(expired, Incoming(new LimitWindow(61, SessionReset)), T).Limits.SevenDay);
    }

    [Fact]
    public void Merge_fills_window_observed_at_from_old_snapshot()
    {
        var stored = Stored(new LimitWindow(60, SessionReset), new LimitWindow(30, WeeklyReset), at: T.AddHours(-1));
        var merged = UsageMerge.Merge(stored, Incoming(new LimitWindow(10, SessionReset)), T);

        Assert.Equal(T.AddHours(-1), merged.Limits.FiveHour!.ObservedAt); // kept (lower, same period)
        Assert.Equal(T.AddHours(-1), merged.Limits.SevenDay!.ObservedAt); // kept (absent from reading)
    }

    [Fact]
    public void Merge_applies_the_same_rule_to_spend()
    {
        var stored = Stored(null, null, new SpendLimit(60, WeeklyReset, 6, 10, "monthly", T.AddMinutes(-5)));
        var merged = UsageMerge.Merge(stored, Incoming(null, null, new SpendLimit(40, WeeklyReset, 4, 10, "weekly")), T);

        Assert.Equal(new SpendLimit(60, WeeklyReset, 6, 10, "monthly", T.AddMinutes(-5)), merged.Limits.Spend);
    }

    [Fact]
    public void Merge_takes_spend_whose_cap_was_raised_in_the_same_period()
    {
        var stored = Stored(null, null, new SpendLimit(90, WeeklyReset, 45, 50, "monthly", T.AddMinutes(-5)));
        var merged = UsageMerge.Merge(stored, Incoming(null, null, new SpendLimit(45, WeeklyReset, 45, 100, "monthly")), T);

        Assert.Equal(new SpendLimit(45, WeeklyReset, 45, 100, "monthly", T), merged.Limits.Spend);
    }

    [Fact]
    public void Merge_compares_spend_in_dollars_when_the_cap_is_the_same()
    {
        var stored = Stored(null, null, new SpendLimit(60, WeeklyReset, 30, 50, "monthly", T.AddMinutes(-5)));

        var fewerDollars = UsageMerge.Merge(stored, Incoming(null, null, new SpendLimit(61, WeeklyReset, 29, 50, "monthly")), T);
        Assert.Equal(stored.Limits.Spend, fewerDollars.Limits.Spend);

        var moreDollars = UsageMerge.Merge(stored, Incoming(null, null, new SpendLimit(59, WeeklyReset, 31, 50, "monthly")), T);
        Assert.Equal(new SpendLimit(59, WeeklyReset, 31, 50, "monthly", T), moreDollars.Limits.Spend);
    }

    [Fact]
    public void Merge_keeps_stored_spend_missing_from_reading_until_its_reset_passes()
    {
        var stored = Stored(null, null, new SpendLimit(90, WeeklyReset, 45, 50, "monthly", T.AddMinutes(-5)));
        Assert.Equal(stored.Limits.Spend, UsageMerge.Merge(stored, Incoming(new LimitWindow(10, SessionReset)), T).Limits.Spend);

        var expired = Stored(null, null, new SpendLimit(90, T, 45, 50, "monthly", T.AddMinutes(-5)));
        Assert.Null(UsageMerge.Merge(expired, Incoming(new LimitWindow(10, SessionReset)), T).Limits.Spend);
    }

    [Fact]
    public void Merge_takes_lower_spend_from_a_later_period()
    {
        var stored = Stored(null, null, new SpendLimit(90, WeeklyReset, 45, 50, "monthly", T.AddMinutes(-5)));
        var later = WeeklyReset.AddDays(30);
        var merged = UsageMerge.Merge(stored, Incoming(null, null, new SpendLimit(4, later, 2, 50, "monthly")), T);

        Assert.Equal(new SpendLimit(4, later, 2, 50, "monthly", T), merged.Limits.Spend);
    }

    [Fact]
    public void An_authoritative_reading_replaces_a_higher_one_in_the_same_period()
    {
        var stored = Stored(new LimitWindow(60, SessionReset, T.AddMinutes(-5)), new LimitWindow(40, WeeklyReset, T.AddMinutes(-5)));
        var incoming = Incoming(new LimitWindow(12, SessionReset.AddSeconds(10)), new LimitWindow(39, WeeklyReset));

        var live = UsageMerge.Merge(stored, incoming, T, authoritative: true);
        Assert.Equal(new LimitWindow(12, SessionReset.AddSeconds(10), T), live.Limits.FiveHour);
        Assert.Equal(new LimitWindow(39, WeeklyReset, T), live.Limits.SevenDay);

        var statusLine = UsageMerge.Merge(stored, incoming, T, authoritative: false);
        Assert.Equal(stored.Limits.FiveHour, statusLine.Limits.FiveHour);
        Assert.Equal(stored.Limits.SevenDay, statusLine.Limits.SevenDay);
    }

    [Fact]
    public void An_authoritative_reading_still_ignores_an_earlier_period()
    {
        var stored = Stored(new LimitWindow(5, T.AddHours(8), T.AddMinutes(-5)));
        var merged = UsageMerge.Merge(stored, Incoming(new LimitWindow(70, SessionReset)), T, authoritative: true);

        Assert.Equal(stored.Limits.FiveHour, merged.Limits.FiveHour);
    }

    [Fact]
    public void An_authoritative_reading_leaves_spend_to_its_own_rule()
    {
        var spend = new SpendLimit(60, WeeklyReset, 30, 50, "monthly", T.AddMinutes(-5));
        var stored = Stored(new LimitWindow(60, SessionReset, T.AddMinutes(-5)), null, spend);

        Assert.Equal(spend, UsageMerge.Merge(stored, Incoming(new LimitWindow(10, SessionReset)), T, authoritative: true).Limits.Spend);
        var fewerDollars = Incoming(new LimitWindow(10, SessionReset), null, new SpendLimit(50, WeeklyReset, 25, 50, "monthly"));
        Assert.Equal(spend, UsageMerge.Merge(stored, fewerDollars, T, authoritative: true).Limits.Spend);
    }
}
