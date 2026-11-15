using System.Text;
using System.Text.Json;
using Sobreiro.Core.IO;
using Sobreiro.Core.Limits;

namespace Sobreiro.Core.Alerts;

/// <summary>The highest threshold already alerted for a limit, and the reset period it was for.</summary>
public sealed record AlertMark(DateTimeOffset? ResetsAt, int Threshold);

/// <summary>alerts.json: which alerts were already shown, so each fires once per window per reset period.</summary>
public sealed record AlertState(IReadOnlyDictionary<LimitKind, AlertMark> Marks)
{
    public static readonly AlertState Empty = new(new Dictionary<LimitKind, AlertMark>());

    private static readonly LimitKind[] Kinds = [LimitKind.Session, LimitKind.Weekly, LimitKind.Spend, LimitKind.Credits];

    private static string Key(LimitKind kind) => kind switch
    {
        LimitKind.Session => "session",
        LimitKind.Weekly => "weekly",
        LimitKind.Credits => "copilot_credits",
        _ => "spend",
    };

    /// <summary>Missing, corrupt or wrong-shaped files give <see cref="Empty"/>.</summary>
    public static AlertState Load(string path)
    {
        try
        {
            using var doc = JsonDocument.Parse(File.ReadAllText(path));
            if (doc.RootElement.ValueKind != JsonValueKind.Object) return Empty;

            var marks = new Dictionary<LimitKind, AlertMark>();
            foreach (var kind in Kinds)
            {
                if (!doc.RootElement.TryGetProperty(Key(kind), out var entry)
                    || entry.ValueKind != JsonValueKind.Object
                    || !entry.TryGetProperty("threshold", out var threshold)
                    || threshold.ValueKind != JsonValueKind.Number
                    || !threshold.TryGetInt32(out var level))
                    continue;

                DateTimeOffset? resetsAt = null;
                if (entry.TryGetProperty("resets_at", out var at) && at.ValueKind == JsonValueKind.Number && at.TryGetInt64(out var seconds))
                    resetsAt = DateTimeOffset.FromUnixTimeSeconds(seconds);
                marks[kind] = new AlertMark(resetsAt, level);
            }
            return marks.Count == 0 ? Empty : new AlertState(marks);
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException or JsonException or ArgumentOutOfRangeException)
        {
            return Empty;
        }
    }

    public void Save(string path)
    {
        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream, new JsonWriterOptions { Indented = true }))
        {
            writer.WriteStartObject();
            foreach (var kind in Kinds)
            {
                if (!Marks.TryGetValue(kind, out var mark)) continue;
                writer.WriteStartObject(Key(kind));
                if (mark.ResetsAt is { } at) writer.WriteNumber("resets_at", at.ToUnixTimeSeconds());
                else writer.WriteNull("resets_at");
                writer.WriteNumber("threshold", mark.Threshold);
                writer.WriteEndObject();
            }
            writer.WriteEndObject();
        }
        AtomicFile.WriteAllText(path, Encoding.UTF8.GetString(stream.ToArray()));
    }
}
