using Sobreiro.Core.Live;
using Sobreiro.Core.Polling;

namespace Sobreiro.Tests;

public class LiveStateTests
{
    [Theory]
    [InlineData(ProviderState.Expired, "Claude Code's sign-in has expired. Use Claude Code in a terminal and it renews it.")]
    [InlineData(ProviderState.NoSignIn, "Claude Code isn't signed in on this PC. Run claude in a terminal and sign in.")]
    [InlineData(ProviderState.Refused, "Anthropic didn't accept Claude Code's sign-in. Use Claude Code in a terminal so it gets a new one.")]
    [InlineData(ProviderState.RateLimited, "Anthropic asked Sobreiro to wait. Nothing to do: it asks again by itself.")]
    [InlineData(ProviderState.Offline, "Sobreiro can't reach Anthropic. Check the connection; it tries again every 2 minutes.")]
    [InlineData(ProviderState.BadAnswer, "Anthropic's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro.")]
    public void Each_problem_has_a_sentence(ProviderState state, string text) =>
        Assert.Equal(text, LiveStateText.Footer(state));

    [Theory]
    [InlineData(ProviderState.Off)]
    [InlineData(ProviderState.Ok)]
    public void Off_and_ok_have_none(ProviderState state) => Assert.Null(LiveStateText.Footer(state));

    [Fact]
    public void The_about_line_is_fixed() =>
        Assert.Equal(
            "It asks Anthropic for your usage every 5 minutes with Claude Code's sign-in, and makes no other network calls.",
            LiveUsageMessage.AboutLine);
}
