using Sobreiro.Core.Diagnostics;

namespace Sobreiro.Tests;

public class LogEntryTests
{
    private static readonly DateTimeOffset At = new(2026, 11, 9, 14, 3, 12, 345, TimeSpan.FromHours(1));

    private static Exception Thrown(Exception e)
    {
        try { throw e; }
        catch (Exception caught) { return caught; }
    }

    [Fact]
    public void Format_starts_with_iso_time_pid_and_event() =>
        Assert.Equal(
            "2026-11-09T14:03:12.345+01:00 pid=1234 start version=0.2.0 instance=Sobreiro\n",
            LogEntry.Format(At, 1234, "start version=0.2.0 instance=Sobreiro"));

    [Fact]
    public void Format_flattens_line_breaks_in_the_event() =>
        Assert.StartsWith("2026-11-09T14:03:12.345+01:00 pid=1234 a  b\n", LogEntry.Format(At, 1234, "a\r\nb"));

    [Fact]
    public void Format_adds_the_type_and_stack_trace()
    {
        var text = LogEntry.Format(At, 1, "e", Thrown(new InvalidOperationException()));
        var lines = text.Split('\n');
        var i = Array.IndexOf(lines, "  System.InvalidOperationException");
        Assert.True(i >= 0);
        Assert.StartsWith("    at ", lines[i + 1]);
    }

    [Fact]
    public void Format_never_includes_the_message()
    {
        var ex = new InvalidOperationException(@"secret C:\Users\x token=abc");
        ex.Data["k"] = "secret";
        var text = LogEntry.Format(At, 1, "e", Thrown(ex));
        Assert.DoesNotContain("secret", text);
        Assert.DoesNotContain("token=abc", text);
        Assert.DoesNotContain(@"C:\Users\x", text);
    }

    [Fact]
    public void Format_walks_inner_and_aggregate_exceptions()
    {
        var ex = new AggregateException(new IOException(), new InvalidOperationException("m", new ArgumentException()));
        var text = LogEntry.Format(At, 1, "e", ex);
        var order = new[]
        {
            "  System.AggregateException", "  inner System.IO.IOException",
            "  inner System.InvalidOperationException", "  inner System.ArgumentException",
        }.Select(s => text.IndexOf(s + "\n", StringComparison.Ordinal)).ToArray();
        Assert.All(order, o => Assert.True(o >= 0));
        Assert.Equal(order.OrderBy(o => o), order);
    }

    [Fact]
    public void Format_stops_after_eight_exceptions()
    {
        Exception ex = new Exception();
        for (var i = 0; i < 11; i++) ex = new Exception("x", ex);
        var text = LogEntry.Format(At, 1, "e", ex);
        var typeLines = text.Split('\n').Count(l => l.StartsWith("  System.") || l.StartsWith("  inner System."));
        Assert.Equal(8, typeLines);
    }

    [Fact]
    public void Format_records_a_non_exception_by_type_only()
    {
        var text = LogEntry.Format(At, 1, "unhandled-exception terminating=true", "boom");
        Assert.Contains("  System.String\n", text);
        Assert.DoesNotContain("boom", text);
    }

    [Fact]
    public void Format_marks_a_missing_stack_trace() =>
        Assert.Contains("    (no stack trace)\n", LogEntry.Format(At, 1, "e", new Exception()));

    [Fact]
    public void Format_cuts_an_entry_over_the_cap()
    {
        var text = LogEntry.Format(At, 1, new string('x', 20000));
        Assert.True(text.Length <= LogEntry.MaxChars + 10);
        Assert.EndsWith("  (cut)\n", text);
    }

    [Fact]
    public void StartupFailed_names_the_type_and_the_log() =>
        Assert.Equal(
            "Sobreiro couldn't start (InvalidOperationException).\n\nThe details are in C:\\x\\log.txt",
            DiagnosticsMessage.StartupFailed("InvalidOperationException", @"C:\x\log.txt"));

    [Fact]
    public void StartupFailedUnlogged_names_the_type_and_the_folder() =>
        Assert.Equal(
            "Sobreiro couldn't start (IOException).\n\nIt couldn't write its log either. Check that this folder can be created and written to: C:\\x",
            DiagnosticsMessage.StartupFailedUnlogged("IOException", @"C:\x"));
}
