namespace Sobreiro.Core.Alerts;

/// <summary>The one balloon for a refresh's alerts, so none is dropped silently.</summary>
public static class AlertBalloon
{
    /// <summary>
    /// The first alert's title; its body, then the other alerts' titles, one per line. Null when there are
    /// no alerts. The text is never empty (Windows refuses an empty balloon): a single space at least.
    /// </summary>
    public static (string Title, string Text)? For(IReadOnlyList<Alert> alerts)
    {
        if (alerts.Count == 0) return null;

        var lines = new List<string>();
        if (alerts[0].Body.Length > 0) lines.Add(alerts[0].Body);
        lines.AddRange(alerts.Skip(1).Select(a => a.Title));
        return (alerts[0].Title, lines.Count == 0 ? " " : string.Join("\n", lines));
    }
}
