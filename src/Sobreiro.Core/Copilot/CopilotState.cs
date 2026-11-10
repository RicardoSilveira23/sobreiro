using System.Globalization;
using Sobreiro.Core.Display;

namespace Sobreiro.Core.Copilot;

/// <summary>Where the Copilot poll stands, as the footer and the flyout name it.</summary>
public enum CopilotState { Ok, NoSignIn, Refused, RateLimited, Failing }

/// <summary>Which of the Live and the Copilot CLI's cached readings to show.</summary>
public static class CopilotShown
{
    /// <summary>The later ObservedAt wins; a tie goes to live.</summary>
    public static CopilotReading? Pick(CopilotReading? live, CopilotReading? cache)
    {
        if (live is null) return cache;
        if (cache is null) return live;
        return cache.ObservedAt > live.ObservedAt ? cache : live;
    }
}

/// <summary>The sentence under the Copilot figures: the problem first, then where the reading comes from.</summary>
public static class CopilotStateText
{
    private static readonly TimeSpan Stale = TimeSpan.FromMinutes(15);

    public static string? Line(
        CopilotState state, CopilotReading reading, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture)
    {
        var problem = state switch
        {
            CopilotState.NoSignIn => "No GitHub sign-in Sobreiro can use.",
            CopilotState.Refused => "GitHub didn't accept the sign-in.",
            CopilotState.Failing => "Can't reach GitHub.",
            CopilotState.RateLimited => "GitHub asked Sobreiro to wait.",
            _ => null,
        };
        string? source = null;
        if (reading.Source == CopilotSource.Cache)
            source = $"From the Copilot CLI, as of {When.Format(reading.ObservedAt, now, zone, culture)}.";
        else if (now - reading.ObservedAt > Stale)
            source = $"As of {When.Format(reading.ObservedAt, now, zone, culture)}.";

        if (problem is null) return source;
        return source is null ? problem : problem + " " + source;
    }
}
