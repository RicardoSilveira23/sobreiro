using System.Globalization;
using Sobreiro.Core.Copilot;
using Sobreiro.Core.Polling;

namespace Sobreiro.Tests;

public class CopilotStateTests
{
    private static readonly DateTimeOffset Now = new(2026, 11, 10, 9, 45, 0, TimeSpan.Zero);
    private static readonly DateTimeOffset Seen = new(2026, 11, 10, 9, 25, 0, TimeSpan.Zero);
    private static readonly CultureInfo Culture = CultureInfo.GetCultureInfo("en-GB");

    private static CopilotReading Reading(DateTimeOffset observed, CopilotSource source = CopilotSource.Live) =>
        new(15000, 2644.2, null, observed, source);

    private static string? Line(ProviderState state, CopilotReading reading) =>
        CopilotStateText.Line(state, reading, Now, TimeZoneInfo.Utc, Culture);

    [Fact]
    public void Fresh_live_ok_has_no_line() =>
        Assert.Null(Line(ProviderState.Ok, Reading(Now.AddMinutes(-15))));

    [Fact]
    public void Old_live_ok_says_as_of() =>
        Assert.Equal("As of 09:25.", Line(ProviderState.Ok, Reading(Seen)));

    [Fact]
    public void Cache_says_from_the_copilot_cli() =>
        Assert.Equal("From the Copilot CLI, as of 09:25.", Line(ProviderState.Ok, Reading(Seen, CopilotSource.Cache)));

    [Fact]
    public void Problem_comes_before_source() =>
        Assert.Equal(
            "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes. From the Copilot CLI, as of 09:25.",
            Line(ProviderState.Offline, Reading(Seen, CopilotSource.Cache)));

    [Theory]
    [InlineData(ProviderState.NoSignIn, "No GitHub sign-in Sobreiro can use. Sign in with the Copilot CLI or gh.")]
    [InlineData(ProviderState.Refused, "GitHub didn't accept the sign-in. Sign in again with the Copilot CLI or gh.")]
    [InlineData(ProviderState.RateLimited, "GitHub asked Sobreiro to wait. Nothing to do: it asks again by itself.")]
    [InlineData(ProviderState.Offline, "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes.")]
    [InlineData(ProviderState.BadAnswer, "GitHub's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro.")]
    public void Each_problem_has_its_sentence(ProviderState state, string sentence) =>
        Assert.Equal(sentence, Line(state, Reading(Now)));

    [Fact]
    public void Pick_takes_the_newer_and_live_on_a_tie()
    {
        var live = Reading(Seen);
        var newerCache = Reading(Seen.AddMinutes(1), CopilotSource.Cache);
        var tiedCache = Reading(Seen, CopilotSource.Cache);
        Assert.Same(newerCache, CopilotShown.Pick(live, newerCache));
        Assert.Same(live, CopilotShown.Pick(live, tiedCache));
        Assert.Same(live, CopilotShown.Pick(live, Reading(Seen.AddMinutes(-1), CopilotSource.Cache)));
        Assert.Same(live, CopilotShown.Pick(live, null));
        Assert.Same(tiedCache, CopilotShown.Pick(null, tiedCache));
        Assert.Null(CopilotShown.Pick(null, null));
    }
}
