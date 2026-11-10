using System.Globalization;
using System.Text;

namespace Sobreiro.Core.Diagnostics;

/// <summary>
/// Formats one diagnostics log entry: a header line, then the exception types and stack traces.
/// Exception messages, data and ToString() are never written, since they can hold paths or tokens.
/// </summary>
public static class LogEntry
{
    public const int MaxChars = 16384;
    public const int MaxExceptions = 8;

    public static string Format(DateTimeOffset at, int pid, string @event, object? exception = null)
    {
        var sb = new StringBuilder();
        sb.Append(at.ToString("yyyy-MM-dd'T'HH:mm:ss.fffzzz", CultureInfo.InvariantCulture))
          .Append(" pid=").Append(pid.ToString(CultureInfo.InvariantCulture))
          .Append(' ').Append(@event.Replace('\r', ' ').Replace('\n', ' ')).Append('\n');

        if (exception is Exception root) AppendExceptions(sb, root);
        else if (exception is not null) sb.Append("  ").Append(exception.GetType().FullName).Append('\n');

        if (sb.Length > MaxChars)
        {
            sb.Length = MaxChars;
            sb.Append("\n  (cut)\n");
        }
        return sb.ToString();
    }

    private static void AppendExceptions(StringBuilder sb, Exception root)
    {
        var queue = new Queue<Exception>();
        queue.Enqueue(root);
        for (var count = 0; count < MaxExceptions && queue.Count > 0; count++)
        {
            var current = queue.Dequeue();
            sb.Append("  ").Append(count == 0 ? "" : "inner ").Append(current.GetType().FullName).Append('\n');
            AppendStack(sb, current.StackTrace);

            if (current is AggregateException aggregate)
            {
                foreach (var inner in aggregate.InnerExceptions) queue.Enqueue(inner);
            }
            else if (current.InnerException is { } single)
            {
                queue.Enqueue(single);
            }
        }
    }

    private static void AppendStack(StringBuilder sb, string? stack)
    {
        var any = false;
        if (stack is not null)
        {
            foreach (var raw in stack.Split(["\r\n", "\n", "\r"], StringSplitOptions.None))
            {
                var line = raw.Trim();
                if (line.Length == 0) continue;
                sb.Append("    ").Append(line).Append('\n');
                any = true;
            }
        }
        if (!any) sb.Append("    (no stack trace)\n");
    }
}
