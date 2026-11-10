namespace Sobreiro.Core.Copilot;

/// <summary>Where a Copilot reading came from: the endpoint just now, or the Copilot CLI's cache file.</summary>
public enum CopilotSource { Live, Cache }

/// <summary>A month's AI credits: the budget, what is used (past the budget when over), and when the month resets.</summary>
public sealed record CopilotReading(
    double Budget,
    double Used,
    DateTimeOffset? ResetsAt,
    DateTimeOffset ObservedAt,
    CopilotSource Source)
{
    public double Percent => Used / Budget * 100;
}
