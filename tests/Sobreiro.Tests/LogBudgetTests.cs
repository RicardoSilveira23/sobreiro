using Sobreiro.Core.Diagnostics;

namespace Sobreiro.Tests;

public class LogBudgetTests
{
    [Fact]
    public void LogBudget_lets_the_limit_through_then_stays_quiet()
    {
        var budget = new LogBudget(3);
        Assert.Equal("e", budget.Next("e"));
        Assert.Equal("e", budget.Next("e"));
        Assert.Equal("e; later ones not logged", budget.Next("e"));
        Assert.Null(budget.Next("e"));
        Assert.Null(budget.Next("e"));
    }
}
