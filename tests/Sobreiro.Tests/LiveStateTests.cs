using Sobreiro.Core.Live;

namespace Sobreiro.Tests;

public class LiveStateTests
{
    [Theory]
    [InlineData(LiveState.Expired, "Claude Code's sign-in has expired. Use Claude Code in a terminal and it renews it.")]
    [InlineData(LiveState.NoSignIn, "Claude Code isn't signed in on this PC. Run claude in a terminal and sign in.")]
    [InlineData(LiveState.Refused, "Anthropic didn't accept Claude Code's sign-in. Use Claude Code in a terminal so it gets a new one.")]
    [InlineData(LiveState.RateLimited, "Anthropic asked Sobreiro to wait. Nothing to do: it asks again by itself.")]
    [InlineData(LiveState.Offline, "Sobreiro can't reach Anthropic. Check the connection; it tries again every 2 minutes.")]
    [InlineData(LiveState.BadAnswer, "Anthropic's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro.")]
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
