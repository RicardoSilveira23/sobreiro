using Sobreiro.Core.Live;

namespace Sobreiro.Tests;

public class LiveStateTests
{
    [Theory]
    [InlineData(LiveState.Expired, "Live usage is paused until Claude Code renews its sign-in.")]
    [InlineData(LiveState.NoSignIn, "Live usage needs Claude Code to be signed in.")]
    [InlineData(LiveState.Refused, "Anthropic didn't accept Claude Code's sign-in. Live usage waits for a new one.")]
    [InlineData(LiveState.RateLimited, "Anthropic asked Live usage to wait.")]
    [InlineData(LiveState.Failing, "Live usage can't reach Anthropic.")]
    public void Each_problem_has_a_sentence(LiveState state, string text) =>
        Assert.Equal(text, LiveStateText.Footer(state));

    [Theory]
    [InlineData(LiveState.Off)]
    [InlineData(LiveState.Ok)]
    public void Off_and_ok_have_none(LiveState state) => Assert.Null(LiveStateText.Footer(state));

    [Fact]
    public void The_about_line_is_fixed() =>
        Assert.Equal(
            "It asks Anthropic for your usage every 5 minutes with Claude Code's sign-in, and makes no other network calls.",
            LiveUsageMessage.AboutLine);
}
