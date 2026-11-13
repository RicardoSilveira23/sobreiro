using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Polling;

namespace Sobreiro.Tests;

public class PillStaleTests
{
    private static readonly DateTimeOffset Now = DateTimeOffset.Parse("2026-11-13T12:00:00Z");

    private const int Two = PillStale.MissesNeeded;

    private static readonly ProviderState[] Stopped =
    [
        ProviderState.Expired, ProviderState.NoSignIn, ProviderState.Refused,
        ProviderState.Offline, ProviderState.BadAnswer, ProviderState.RateLimited,
    ];

    private static Limit Aged(LimitKind kind, TimeSpan age) => new("Claude Code", kind, kind.ToString(), 40, null, Now - age, false);

    private static readonly IReadOnlyList<Limit> Old =
        [Aged(LimitKind.Session, TimeSpan.FromDays(3)), Aged(LimitKind.Weekly, TimeSpan.FromDays(3))];

    [Theory]
    [InlineData(ProviderState.Expired)]
    [InlineData(ProviderState.NoSignIn)]
    [InlineData(ProviderState.Refused)]
    [InlineData(ProviderState.Offline)]
    [InlineData(ProviderState.BadAnswer)]
    [InlineData(ProviderState.RateLimited)]
    public void Every_stopped_state_marks_claude(ProviderState state) => Assert.True(PillStale.Claude(state, Old, Now, Two));

    [Theory]
    [InlineData(ProviderState.Off)]
    [InlineData(ProviderState.Ok)]
    public void Off_and_ok_are_never_stale(ProviderState state) => Assert.False(PillStale.Claude(state, Old, Now, Two));

    [Theory]
    [InlineData(0)]
    [InlineData(1)]
    public void One_missed_poll_is_not_stale(int misses)
    {
        foreach (var state in Stopped)
        {
            Assert.False(PillStale.Claude(state, Old, Now, misses));
            Assert.False(PillStale.Copilot(state, Credits(TimeSpan.FromDays(3)), Now, misses));
        }
    }

    [Fact]
    public void Two_missed_polls_are_needed() => Assert.Equal(2, PillStale.MissesNeeded);

    [Fact]
    public void Fresh_numbers_are_not_stale()
    {
        var edge = TimeSpan.FromMinutes(15);
        Assert.False(PillStale.Claude(ProviderState.Expired, [Aged(LimitKind.Session, edge), Aged(LimitKind.Weekly, edge)], Now, Two));
        var past = edge + TimeSpan.FromSeconds(1);
        Assert.True(PillStale.Claude(ProviderState.Expired, [Aged(LimitKind.Session, past), Aged(LimitKind.Weekly, past)], Now, Two));
    }

    [Fact]
    public void The_newest_of_the_two_readings_decides()
    {
        Assert.False(PillStale.Claude(ProviderState.Offline,
            [Aged(LimitKind.Session, TimeSpan.FromHours(3)), Aged(LimitKind.Weekly, TimeSpan.FromMinutes(1))], Now, Two));
        Assert.False(PillStale.Claude(ProviderState.Offline, [Aged(LimitKind.Weekly, TimeSpan.FromMinutes(1))], Now, Two));
    }

    [Fact]
    public void A_spend_reading_does_not_count()
    {
        var spend = Aged(LimitKind.Spend, TimeSpan.FromMinutes(1));
        Assert.True(PillStale.Claude(ProviderState.Offline, [spend, Aged(LimitKind.Session, TimeSpan.FromHours(3))], Now, Two));
        Assert.False(PillStale.Claude(ProviderState.Offline, [spend], Now, Two));
    }

    [Theory]
    [InlineData(ProviderState.Expired)]
    [InlineData(ProviderState.NoSignIn)]
    [InlineData(ProviderState.Refused)]
    [InlineData(ProviderState.Offline)]
    [InlineData(ProviderState.BadAnswer)]
    [InlineData(ProviderState.RateLimited)]
    public void Nothing_to_keep_is_not_stale(ProviderState state) => Assert.False(PillStale.Claude(state, [], Now, Two));

    [Fact]
    public void A_reading_from_the_future_is_fresh() =>
        Assert.False(PillStale.Claude(ProviderState.Offline, [Aged(LimitKind.Session, TimeSpan.FromMinutes(-10))], Now, Two));

    private static Limit Credits(TimeSpan age) =>
        new("GitHub Copilot", LimitKind.Credits, "AI credits", 20, null, Now - age, false);

    [Theory]
    [InlineData(ProviderState.NoSignIn)]
    [InlineData(ProviderState.Refused)]
    [InlineData(ProviderState.Offline)]
    [InlineData(ProviderState.BadAnswer)]
    [InlineData(ProviderState.RateLimited)]
    public void Every_stopped_state_marks_copilot(ProviderState state) =>
        Assert.True(PillStale.Copilot(state, Credits(TimeSpan.FromHours(25)), Now, Two));

    [Fact]
    public void Copilot_ok_is_not_stale() =>
        Assert.False(PillStale.Copilot(ProviderState.Ok, Credits(TimeSpan.FromHours(25)), Now, Two));

    [Fact]
    public void A_copilot_figure_from_today_is_not_stale()
    {
        Assert.False(PillStale.Copilot(ProviderState.NoSignIn, Credits(TimeSpan.FromHours(2)), Now, Two));
        Assert.False(PillStale.Copilot(ProviderState.NoSignIn, Credits(TimeSpan.FromHours(24)), Now, Two));
    }

    [Theory]
    [InlineData(ProviderState.Ok)]
    [InlineData(ProviderState.NoSignIn)]
    [InlineData(ProviderState.Refused)]
    [InlineData(ProviderState.Offline)]
    [InlineData(ProviderState.BadAnswer)]
    [InlineData(ProviderState.RateLimited)]
    public void No_reading_is_not_stale(ProviderState state) => Assert.False(PillStale.Copilot(state, null, Now, Two));
}
