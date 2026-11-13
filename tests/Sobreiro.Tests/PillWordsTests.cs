using Sobreiro.Core.Copilot;
using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;
using Sobreiro.Core.Live;

namespace Sobreiro.Tests;

public class PillWordsTests
{
    private static readonly DateTimeOffset Now = DateTimeOffset.Parse("2026-11-13T12:00:00Z");

    private static Limit Aged(LimitKind kind, TimeSpan age) => new("Claude Code", kind, kind.ToString(), 40, null, Now - age, false);

    [Theory]
    [InlineData(LiveState.Expired, "sign-in", "expired")]
    [InlineData(LiveState.NoSignIn, "no", "sign-in")]
    [InlineData(LiveState.Refused, "sign-in", "refused")]
    [InlineData(LiveState.Offline, "no", "network")]
    [InlineData(LiveState.BadAnswer, "no", "answer")]
    [InlineData(LiveState.RateLimited, "asked", "to wait")]
    public void Claude_words_per_state(LiveState state, string top, string bottom) =>
        Assert.Equal(new PillWords(top, bottom), PillWords.ForClaude(state, [], Now));

    [Theory]
    [InlineData(LiveState.Off)]
    [InlineData(LiveState.Ok)]
    public void Off_and_ok_never_have_words(LiveState state)
    {
        Assert.Null(PillWords.ForClaude(state, [], Now));
        Assert.Null(PillWords.ForClaude(state, [Aged(LimitKind.Session, TimeSpan.FromDays(3))], Now));
    }

    [Fact]
    public void Fresh_numbers_keep_the_figures()
    {
        var edge = TimeSpan.FromMinutes(15);
        Assert.Null(PillWords.ForClaude(LiveState.Expired, [Aged(LimitKind.Session, edge), Aged(LimitKind.Weekly, edge)], Now));
        var past = edge + TimeSpan.FromSeconds(1);
        Assert.Equal(new PillWords("sign-in", "expired"),
            PillWords.ForClaude(LiveState.Expired, [Aged(LimitKind.Session, past), Aged(LimitKind.Weekly, past)], Now));
    }

    [Fact]
    public void The_newest_of_the_two_readings_decides()
    {
        Assert.Null(PillWords.ForClaude(LiveState.Offline,
            [Aged(LimitKind.Session, TimeSpan.FromHours(3)), Aged(LimitKind.Weekly, TimeSpan.FromMinutes(1))], Now));
        Assert.Null(PillWords.ForClaude(LiveState.Offline, [Aged(LimitKind.Weekly, TimeSpan.FromMinutes(1))], Now));
    }

    [Fact]
    public void A_spend_reading_does_not_count() =>
        Assert.Equal(new PillWords("no", "network"),
            PillWords.ForClaude(LiveState.Offline, [Aged(LimitKind.Spend, TimeSpan.FromMinutes(1))], Now));

    [Fact]
    public void A_reading_from_the_future_is_fresh() =>
        Assert.Null(PillWords.ForClaude(LiveState.Offline, [Aged(LimitKind.Session, TimeSpan.FromMinutes(-10))], Now));

    private static Limit Credits(TimeSpan age) =>
        new("GitHub Copilot", LimitKind.Credits, "AI credits", 20, null, Now - age, false);

    [Theory]
    [InlineData(CopilotState.NoSignIn, "no", "sign-in")]
    [InlineData(CopilotState.Refused, "sign-in", "refused")]
    [InlineData(CopilotState.Offline, "no", "network")]
    [InlineData(CopilotState.BadAnswer, "no", "answer")]
    [InlineData(CopilotState.RateLimited, "asked", "to wait")]
    public void Copilot_words_per_state(CopilotState state, string top, string bottom) =>
        Assert.Equal(new PillWords(top, bottom), PillWords.ForCopilot(state, Credits(TimeSpan.FromHours(25)), Now));

    [Fact]
    public void Copilot_ok_has_no_words() =>
        Assert.Null(PillWords.ForCopilot(CopilotState.Ok, Credits(TimeSpan.FromHours(25)), Now));

    [Fact]
    public void A_copilot_figure_from_today_stays()
    {
        Assert.Null(PillWords.ForCopilot(CopilotState.NoSignIn, Credits(TimeSpan.FromHours(2)), Now));
        Assert.Null(PillWords.ForCopilot(CopilotState.NoSignIn, Credits(TimeSpan.FromHours(24)), Now));
    }

    [Theory]
    [InlineData(CopilotState.Ok)]
    [InlineData(CopilotState.NoSignIn)]
    [InlineData(CopilotState.Refused)]
    [InlineData(CopilotState.Offline)]
    [InlineData(CopilotState.BadAnswer)]
    [InlineData(CopilotState.RateLimited)]
    public void No_reading_no_words(CopilotState state) => Assert.Null(PillWords.ForCopilot(state, null, Now));
}
