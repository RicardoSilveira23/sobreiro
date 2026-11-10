using Sobreiro.Core.Display;
using Sobreiro.Core.Usage;

namespace Sobreiro.Tests;

public class StatusLineTextTests
{
    [Fact]
    public void Both_windows() =>
        Assert.Equal("5h 8% · 7d 44%", StatusLineText.Render(new RateLimits(new LimitWindow(8.4, null), new LimitWindow(44, null), null)));

    [Fact]
    public void Only_weekly() =>
        Assert.Equal("5h – · 7d 44%", StatusLineText.Render(new RateLimits(null, new LimitWindow(44, null), null)));

    [Fact]
    public void Null_limits() => Assert.Equal("5h – · 7d –", StatusLineText.Render(null));

    [Fact]
    public void With_spend() =>
        Assert.Equal("5h 8% · 7d 44% · spend 62%", StatusLineText.Render(new RateLimits(
            new LimitWindow(8.4, null), new LimitWindow(44, null), new SpendLimit(62.8, null, null, null, null))));
}
