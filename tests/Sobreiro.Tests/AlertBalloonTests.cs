using Sobreiro.Core.Alerts;
using Sobreiro.Core.Limits;

namespace Sobreiro.Tests;

public class AlertBalloonTests
{
    private static Alert Make(LimitKind kind, int threshold, string title, string body) => new(kind, threshold, title, body);

    [Fact]
    public void No_alerts_give_no_balloon() => Assert.Null(AlertBalloon.For([]));

    [Fact]
    public void One_alert_gives_its_title_and_body()
    {
        var balloon = AlertBalloon.For([Make(LimitKind.Session, 80, "Session limit at 82%", "Resets 1:00 PM · in 3h")]);
        Assert.Equal(("Session limit at 82%", "Resets 1:00 PM · in 3h"), balloon);
    }

    [Fact]
    public void Other_alerts_add_their_titles_on_following_lines()
    {
        var balloon = AlertBalloon.For([
            Make(LimitKind.Session, 95, "Session limit at 96%", "Resets 1:00 PM · in 3h"),
            Make(LimitKind.Weekly, 80, "Weekly limit at 81%", "Resets Fri 12:00 AM · in 4d 14h"),
            Make(LimitKind.Spend, 80, "Spend limit at 80%", ""),
        ]);
        Assert.Equal(("Session limit at 96%", "Resets 1:00 PM · in 3h\nWeekly limit at 81%\nSpend limit at 80%"), balloon);
    }

    [Fact]
    public void An_empty_first_body_is_left_out()
    {
        var balloon = AlertBalloon.For([
            Make(LimitKind.Spend, 95, "Spend limit at 95%", ""),
            Make(LimitKind.Session, 80, "Session limit at 80%", "Resets 1:00 PM · in 3h"),
        ]);
        Assert.Equal(("Spend limit at 95%", "Session limit at 80%"), balloon);
    }

    [Fact]
    public void A_lone_alert_without_a_body_still_has_some_text()
    {
        var balloon = AlertBalloon.For([Make(LimitKind.Spend, 80, "Spend limit at 80%", "")]);
        Assert.Equal(("Spend limit at 80%", " "), balloon);
    }
}
