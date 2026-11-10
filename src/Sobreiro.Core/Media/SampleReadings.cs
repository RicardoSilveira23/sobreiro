using Sobreiro.Core.Copilot;
using Sobreiro.Core.Limits;

namespace Sobreiro.Core.Media;

/// <summary>Readings the pictures are drawn from. Claude and Copilot rows appear as the Services value says.</summary>
public static class SampleReadings
{
    public static readonly string[] LevelNames = ["none", "low", "middle", "high", "full"];

    private static readonly (double Session, double Weekly, double Credits)[] Rungs =
        [(0, 0, 0), (8, 44, 750), (27, 61, 2644), (74, 80, 9450), (100, 93, 15000)];

    public static bool ShowClaude(Services s) => s != Services.Copilot;

    public static IReadOnlyList<Limit> Calm(Services s, DateTimeOffset now) => Build(s, 15, 45, 2644, now);

    public static IReadOnlyList<Limit> Rung(Services s, int level, DateTimeOffset now)
    {
        if (level == 0) return [];
        var (session, weekly, credits) = Rungs[level];
        return At(s, (int)session, (int)weekly, credits, now);
    }

    /// <summary>A reading at the given session and weekly percentages and Copilot credits used.</summary>
    public static IReadOnlyList<Limit> At(Services s, int session, int weekly, double credits, DateTimeOffset now) =>
        Build(s, session, weekly, credits, now);

    private static List<Limit> Build(Services s, double session, double weekly, double credits, DateTimeOffset now)
    {
        var observed = now.AddMinutes(-1);
        var list = new List<Limit>();
        if (ShowClaude(s))
        {
            list.Add(new Limit(Providers.Claude, LimitKind.Session, "Session", session, now.AddMinutes(141),
                observed, false, TimeSpan.FromHours(5)));
            list.Add(new Limit(Providers.Claude, LimitKind.Weekly, "Weekly", weekly, now.AddHours(74),
                observed, false, TimeSpan.FromDays(7)));
        }
        if (s != Services.Claude)
        {
            var local = new DateTime(2026, 12, 1, 0, 0, 0);
            var first = new DateTimeOffset(local, MediaPins.Zone.GetUtcOffset(local));
            list.AddRange(LimitList.FromCopilot(new CopilotReading(15000, credits, first, observed, CopilotSource.Live), now));
        }
        return list;
    }
}
