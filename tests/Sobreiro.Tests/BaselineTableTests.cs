using Sobreiro.Core.Baseline;

namespace Sobreiro.Tests;

public class BaselineTableTests
{
    private const long Mb = 1024 * 1024;

    private static readonly BaselineRun Run = new(
        "0.10.0", new DateOnly(2026, 11, 13), 171 * Mb + 300_000, 301 * Mb, 4.24,
        TimeSpan.FromMilliseconds(2314), TimeSpan.FromMilliseconds(1118), 80 * Mb + 100_000, 17 * Mb + 800_000,
        TimeSpan.FromMilliseconds(61.6));

    private const string RunRow =
        "| 0.10.0 | 2026-11-13 | 171 MB | 301 MB | 4.2 s | 2.31 s | 1.12 s | 80.1 MB | 17.8 MB | 62 ms |";

    private static string Doc(params string[] releaseRows) => string.Join("\n",
    [
        "# Baseline", "", "## Releases", "", BaselineTable.Header, BaselineTable.Divider, .. releaseRows, "",
        "## Experiments", "", BaselineTable.Header, BaselineTable.Divider, "| 0.10.0, other | x |", "",
    ]);

    [Fact]
    public void Row_has_one_cell_per_header_column()
    {
        Assert.Equal(RunRow, BaselineTable.Row(Run));
        Assert.Equal(BaselineTable.Header.Count(c => c == '|'), RunRow.Count(c => c == '|'));
        Assert.Equal(BaselineTable.Header.Count(c => c == '|'), BaselineTable.Divider.Count(c => c == '|'));
    }

    [Fact]
    public void Row_says_what_was_not_measured()
    {
        var row = BaselineTable.Row(Run with { FirstPillFirstStart = null, FirstPillLaterStarts = null, BridgeExeBytes = null, BridgeRun = null });
        Assert.Equal("| 0.10.0 | 2026-11-13 | 171 MB | 301 MB | 4.2 s | not seen | not seen | 80.1 MB | none | none |", row);
    }

    [Fact]
    public void Row_is_the_same_in_any_culture()
    {
        var old = Thread.CurrentThread.CurrentCulture;
        try
        {
            Thread.CurrentThread.CurrentCulture = new System.Globalization.CultureInfo("pt-PT");
            Assert.Equal(RunRow, BaselineTable.Row(Run));
        }
        finally
        {
            Thread.CurrentThread.CurrentCulture = old;
        }
    }

    [Fact]
    public void Add_appends_to_the_table_under_the_heading() =>
        Assert.Equal(Doc("| 0.9.0 | x |", RunRow), BaselineTable.Add(Doc("| 0.9.0 | x |"), "Releases", Run));

    [Fact]
    public void Add_fills_an_empty_table() =>
        Assert.Equal(Doc(RunRow), BaselineTable.Add(Doc(), "Releases", Run));

    [Fact]
    public void Add_replaces_the_row_of_the_same_build() =>
        Assert.Equal(Doc("| 0.9.0 | x |", RunRow, "| 0.11.0 | x |"),
            BaselineTable.Add(Doc("| 0.9.0 | x |", "| 0.10.0 | old |", "| 0.11.0 | x |"), "Releases", Run));

    [Fact]
    public void Add_leaves_a_build_whose_name_only_starts_the_same()
    {
        var added = BaselineTable.Add(Doc(), "Experiments", Run);
        Assert.NotNull(added);
        Assert.Contains("| 0.10.0, other | x |\n" + RunRow + "\n", added);
    }

    [Fact]
    public void Add_keeps_windows_line_endings()
    {
        var added = BaselineTable.Add(Doc("| 0.9.0 | x |").Replace("\n", "\r\n"), "Releases", Run);
        Assert.Equal(Doc("| 0.9.0 | x |", RunRow).Replace("\n", "\r\n"), added);
    }

    [Fact]
    public void Add_without_the_heading_or_its_table_changes_nothing()
    {
        Assert.Null(BaselineTable.Add(Doc(), "Nightlies", Run));
        Assert.Null(BaselineTable.Add("# Baseline\n\n## Releases\n\nNo table yet.\n", "Releases", Run));
    }
}
