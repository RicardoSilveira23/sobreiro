using System.Globalization;

namespace Sobreiro.Core.Baseline;

/// <summary>
/// One measured build: a row of docs/baseline.md. <see cref="Idle"/> is null when the tray was not
/// started at all (a bridge-only run); a pill time is null when the tray ran and no pill was seen.
/// </summary>
public sealed record BaselineRun(
    string Build,
    DateOnly Date,
    IdleFigures? Idle,
    TimeSpan? FirstPillFirstStart,
    TimeSpan? FirstPillLaterStarts,
    long? TrayExeBytes,
    long? BridgeExeBytes,
    TimeSpan? BridgeRun);

/// <summary>The tray after its idle run: memory at the end, processor time scaled to an hour.</summary>
public sealed record IdleFigures(long PrivateBytes, long WorkingSetBytes, double CpuSecondsPerHour);

public static class BaselineTable
{
    public const string Header =
        "| Build | Date | Private memory | Working set | CPU per idle hour | First pill, first start | First pill, later starts | Sobreiro.exe | Bridge exe | Bridge run |";

    public const string Divider = "|---|---|---|---|---|---|---|---|---|---|";

    private const double BytesPerMb = 1024 * 1024;
    private const string NotMeasured = "not measured";

    public static string Row(BaselineRun run)
    {
        var inv = CultureInfo.InvariantCulture;
        string[] cells =
        [
            run.Build,
            run.Date.ToString("yyyy-MM-dd", inv),
            run.Idle is { } i ? (i.PrivateBytes / BytesPerMb).ToString("0", inv) + " MB" : NotMeasured,
            run.Idle is { } j ? (j.WorkingSetBytes / BytesPerMb).ToString("0", inv) + " MB" : NotMeasured,
            run.Idle is { } k ? k.CpuSecondsPerHour.ToString("0.0", inv) + " s" : NotMeasured,
            Seconds(run.FirstPillFirstStart),
            Seconds(run.FirstPillLaterStarts),
            run.TrayExeBytes is { } tray ? (tray / BytesPerMb).ToString("0.0", inv) + " MB" : NotMeasured,
            run.BridgeExeBytes is { } bridge ? (bridge / BytesPerMb).ToString("0.0", inv) + " MB" : "none",
            run.BridgeRun is { } time ? time.TotalMilliseconds.ToString("0", inv) + " ms" : "none",
        ];
        return "| " + string.Join(" | ", cells) + " |";

        string Seconds(TimeSpan? time) =>
            time is { } t ? t.TotalSeconds.ToString("0.00", inv) + " s" : run.Idle is null ? NotMeasured : "not seen";
    }

    /// <summary>
    /// The document with the run's row in the first table under "## <paramref name="heading"/>": in
    /// place of the row for the same build, else after the last row. Null when that heading or its
    /// table isn't there.
    /// </summary>
    public static string? Add(string markdown, string heading, BaselineRun run)
    {
        var lines = markdown.Split('\n').ToList();
        var ending = markdown.Contains("\r\n") ? "\r" : "";
        var at = lines.FindIndex(l => l.TrimEnd() == "## " + heading);
        if (at < 0) return null;

        // The table: the first run of lines starting with "|" before the next heading.
        var first = at + 1;
        while (first < lines.Count && !lines[first].StartsWith('|'))
        {
            if (lines[first].StartsWith('#')) return null;
            first++;
        }
        var end = first;
        while (end < lines.Count && lines[end].StartsWith('|')) end++;
        // A header and a divider at least.
        if (end - first < 2) return null;

        var row = Row(run) + ending;
        var same = lines.FindIndex(first + 2, end - first - 2, l => FirstCell(l) == run.Build);
        if (same >= 0) lines[same] = row;
        else lines.Insert(end, row);
        return string.Join('\n', lines);
    }

    private static string FirstCell(string line)
    {
        var cells = line.Split('|');
        return cells.Length > 1 ? cells[1].Trim() : "";
    }
}
