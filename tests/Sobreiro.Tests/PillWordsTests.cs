using Sobreiro.Core.Copilot;
using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Live;
using Sobreiro.Core.Polling;

namespace Sobreiro.Tests;

public class PillWordsTests
{
    private static readonly DateTimeOffset Now = DateTimeOffset.Parse("2026-11-13T12:00:00Z");

    private const int Two = PillWords.MissesNeeded;

    private static Limit Aged(LimitKind kind, TimeSpan age) => new("Claude Code", kind, kind.ToString(), 40, null, Now - age, false);

    [Theory]
    [InlineData(ProviderState.Expired, "sign-in", "expired")]
    [InlineData(ProviderState.NoSignIn, "no", "sign-in")]
    [InlineData(ProviderState.Refused, "sign-in", "refused")]
    [InlineData(ProviderState.Offline, "no", "network")]
    [InlineData(ProviderState.BadAnswer, "no", "answer")]
    [InlineData(ProviderState.RateLimited, "asked", "to wait")]
    public void Claude_words_per_state(ProviderState state, string top, string bottom) =>
        Assert.Equal(new PillWords(top, bottom), PillWords.ForClaude(state, [], Now, Two));

    [Theory]
    [InlineData(ProviderState.Off)]
    [InlineData(ProviderState.Ok)]
    public void Off_and_ok_never_have_words(ProviderState state)
    {
        Assert.Null(PillWords.ForClaude(state, [], Now, Two));
        Assert.Null(PillWords.ForClaude(state, [Aged(LimitKind.Session, TimeSpan.FromDays(3))], Now, Two));
    }

    [Theory]
    [InlineData(0)]
    [InlineData(1)]
    public void One_missed_poll_gives_no_words(int misses)
    {
        foreach (var state in new[] { ProviderState.Expired, ProviderState.NoSignIn, ProviderState.Refused, ProviderState.Offline, ProviderState.BadAnswer, ProviderState.RateLimited })
        {
            Assert.Null(PillWords.ForClaude(state, [], Now, misses));
            Assert.Null(PillWords.ForClaude(state, [Aged(LimitKind.Session, TimeSpan.FromDays(3))], Now, misses));
        }
        foreach (var state in new[] { ProviderState.NoSignIn, ProviderState.Refused, ProviderState.Offline, ProviderState.BadAnswer, ProviderState.RateLimited })
            Assert.Null(PillWords.ForCopilot(state, Credits(TimeSpan.FromDays(3)), Now, misses));
    }

    [Fact]
    public void Two_missed_polls_are_needed() => Assert.Equal(2, PillWords.MissesNeeded);

    [Fact]
    public void Fresh_numbers_keep_the_figures()
    {
        var edge = TimeSpan.FromMinutes(15);
        Assert.Null(PillWords.ForClaude(ProviderState.Expired, [Aged(LimitKind.Session, edge), Aged(LimitKind.Weekly, edge)], Now, Two));
        var past = edge + TimeSpan.FromSeconds(1);
        Assert.Equal(new PillWords("sign-in", "expired"),
            PillWords.ForClaude(ProviderState.Expired, [Aged(LimitKind.Session, past), Aged(LimitKind.Weekly, past)], Now, Two));
    }

    [Fact]
    public void The_newest_of_the_two_readings_decides()
    {
        Assert.Null(PillWords.ForClaude(ProviderState.Offline,
            [Aged(LimitKind.Session, TimeSpan.FromHours(3)), Aged(LimitKind.Weekly, TimeSpan.FromMinutes(1))], Now, Two));
        Assert.Null(PillWords.ForClaude(ProviderState.Offline, [Aged(LimitKind.Weekly, TimeSpan.FromMinutes(1))], Now, Two));
    }

    [Fact]
    public void A_spend_reading_does_not_count() =>
        Assert.Equal(new PillWords("no", "network"),
            PillWords.ForClaude(ProviderState.Offline, [Aged(LimitKind.Spend, TimeSpan.FromMinutes(1))], Now, Two));

    [Fact]
    public void A_reading_from_the_future_is_fresh() =>
        Assert.Null(PillWords.ForClaude(ProviderState.Offline, [Aged(LimitKind.Session, TimeSpan.FromMinutes(-10))], Now, Two));

    private static Limit Credits(TimeSpan age) =>
        new("GitHub Copilot", LimitKind.Credits, "AI credits", 20, null, Now - age, false);

    [Theory]
    [InlineData(ProviderState.NoSignIn, "no", "sign-in")]
    [InlineData(ProviderState.Refused, "sign-in", "refused")]
    [InlineData(ProviderState.Offline, "no", "network")]
    [InlineData(ProviderState.BadAnswer, "no", "answer")]
    [InlineData(ProviderState.RateLimited, "asked", "to wait")]
    public void Copilot_words_per_state(ProviderState state, string top, string bottom) =>
        Assert.Equal(new PillWords(top, bottom), PillWords.ForCopilot(state, Credits(TimeSpan.FromHours(25)), Now, Two));

    [Fact]
    public void Copilot_ok_has_no_words() =>
        Assert.Null(PillWords.ForCopilot(ProviderState.Ok, Credits(TimeSpan.FromHours(25)), Now, Two));

    [Fact]
    public void A_copilot_figure_from_today_stays()
    {
        Assert.Null(PillWords.ForCopilot(ProviderState.NoSignIn, Credits(TimeSpan.FromHours(2)), Now, Two));
        Assert.Null(PillWords.ForCopilot(ProviderState.NoSignIn, Credits(TimeSpan.FromHours(24)), Now, Two));
    }

    [Theory]
    [InlineData(ProviderState.Ok)]
    [InlineData(ProviderState.NoSignIn)]
    [InlineData(ProviderState.Refused)]
    [InlineData(ProviderState.Offline)]
    [InlineData(ProviderState.BadAnswer)]
    [InlineData(ProviderState.RateLimited)]
    public void No_reading_no_words(ProviderState state) => Assert.Null(PillWords.ForCopilot(state, null, Now, Two));
}
