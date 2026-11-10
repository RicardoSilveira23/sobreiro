using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;

namespace Sobreiro.Tests;

public class PaceTests
{
    private static readonly DateTimeOffset Now = DateTimeOffset.Parse("2026-11-09T10:00:00Z");

    private static Limit Make(double pct, TimeSpan? window, TimeSpan? resetsIn, DateTimeOffset? observed = null, bool isReset = false) =>
        new("Claude Code", LimitKind.Session, "Session", pct, resetsIn is { } r ? Now + r : null, observed ?? Now, isReset, window);

    [Fact]
    public void Session_on_track()
    {
        var p = Pace.Project(Make(27, TimeSpan.FromHours(5), TimeSpan.FromMinutes(191)));
        Assert.NotNull(p);
        Assert.Equal(74.31, p.ProjectedPercent, 0.01);
        Assert.Null(p.RunsOutBeforeReset);
        Assert.Null(p.RunsOutAt);
    }

    [Fact]
    public void Weekly_runs_out()
    {
        var p = Pace.Project(Make(74, TimeSpan.FromDays(7), TimeSpan.FromHours(85)));
        Assert.NotNull(p);
        Assert.Equal(149.78, p.ProjectedPercent, 0.01);
        Assert.Equal(55.84, p.RunsOutBeforeReset!.Value.TotalHours, 1.0 / 60);
        Assert.Equal(Now.AddHours(85) - p.RunsOutBeforeReset.Value, p.RunsOutAt);
    }

    [Fact]
    public void Too_early_is_null() =>
        Assert.Null(Pace.Project(Make(5, TimeSpan.FromHours(5), TimeSpan.FromHours(5) - TimeSpan.FromMinutes(12))));

    [Fact]
    public void Full_is_null() => Assert.Null(Pace.Project(Make(100, TimeSpan.FromHours(5), TimeSpan.FromHours(2))));

    [Fact]
    public void Reset_is_null() => Assert.Null(Pace.Project(Make(0, TimeSpan.FromHours(5), TimeSpan.FromHours(2), isReset: true)));

    [Fact]
    public void No_reset_time_is_null() => Assert.Null(Pace.Project(Make(30, TimeSpan.FromHours(5), null)));

    [Fact]
    public void No_window_is_null() => Assert.Null(Pace.Project(Make(30, null, TimeSpan.FromHours(2))));

    [Fact]
    public void Zero_percent_projects_zero()
    {
        var p = Pace.Project(Make(0, TimeSpan.FromHours(4), TimeSpan.FromHours(2)));
        Assert.Equal(0, p!.ProjectedPercent);
        Assert.Null(p.RunsOutBeforeReset);
    }

    [Fact]
    public void Uses_observed_time_not_now()
    {
        var p = Pace.Project(Make(27, TimeSpan.FromHours(5), TimeSpan.FromMinutes(131), Now.AddMinutes(-60)));
        Assert.Equal(74.31, p!.ProjectedPercent, 0.01);
    }
}
