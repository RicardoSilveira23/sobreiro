using System.Globalization;
using Sobreiro.Core.Display;
using Sobreiro.Core.Polling;

namespace Sobreiro.Core.Copilot;

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
        ProviderState state, CopilotReading reading, DateTimeOffset now, TimeZoneInfo zone, CultureInfo culture)
    {
        var problem = state switch
        {
            ProviderState.NoSignIn => "No GitHub sign-in Sobreiro can use. Sign in with the Copilot CLI or gh.",
            ProviderState.Refused => "GitHub didn't accept the sign-in. Sign in again with the Copilot CLI or gh.",
            ProviderState.RateLimited => "GitHub asked Sobreiro to wait. Nothing to do: it asks again by itself.",
            ProviderState.Offline => "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes.",
            ProviderState.BadAnswer => "GitHub's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro.",
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
