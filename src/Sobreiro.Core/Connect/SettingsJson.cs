using System.Text;
using System.Text.Json;

namespace Sobreiro.Core.Connect;

/// <summary>
/// Reads and splices the top-level statusLine value of settings.json. Works on the UTF-8 bytes and only
/// ever cuts and pastes them, so everything outside that one property stays byte for byte.
/// </summary>
public static class SettingsJson
{
    private const string Key = "statusLine";

    private readonly record struct Prop(string Name, int NameStart, int ValueStart, int ValueEnd);

    private readonly record struct Layout(List<Prop> Props, int Open, int Close);

    /// <summary>Raw text of the top-level statusLine value, null when absent. Blank text counts as {}.</summary>
    public static string? ReadStatusLine(string text)
    {
        if (string.IsNullOrWhiteSpace(text)) return null;
        var bytes = Encoding.UTF8.GetBytes(text);
        var layout = Parse(bytes);
        var index = layout.Props.FindLastIndex(p => p.Name == Key);
        if (index < 0) return null;
        var p = layout.Props[index];
        return Encoding.UTF8.GetString(bytes, p.ValueStart, p.ValueEnd - p.ValueStart);
    }

    /// <summary>The "command" string of a statusLine object, or null.</summary>
    public static string? CommandOf(string? statusLineJson)
    {
        if (statusLineJson is null) return null;
        try
        {
            using var doc = JsonDocument.Parse(statusLineJson, new JsonDocumentOptions
            {
                CommentHandling = JsonCommentHandling.Skip,
                AllowTrailingCommas = true,
            });
            return doc.RootElement.ValueKind == JsonValueKind.Object
                && doc.RootElement.TryGetProperty("command", out var command)
                && command.ValueKind == JsonValueKind.String
                    ? command.GetString()
                    : null;
        }
        catch (JsonException)
        {
            return null;
        }
    }

    /// <summary>
    /// Replaces the statusLine value, removes the property (and its comma) when <paramref name="valueJson"/>
    /// is null, or appends it as the last property when absent.
    /// </summary>
    public static string WithStatusLine(string text, string? valueJson)
    {
        if (string.IsNullOrWhiteSpace(text))
            return valueJson is null ? text : "{\n  \"" + Key + "\": " + valueJson + "\n}\n";

        var bytes = Encoding.UTF8.GetBytes(text);
        var layout = Parse(bytes);
        var index = layout.Props.FindLastIndex(p => p.Name == Key);

        if (index >= 0)
        {
            var p = layout.Props[index];
            return valueJson is null
                ? Decode(Remove(bytes, layout, index))
                : Decode(Splice(bytes, p.ValueStart, p.ValueEnd, Encoding.UTF8.GetBytes(valueJson)));
        }
        return valueJson is null ? text : Decode(Insert(bytes, text, layout, valueJson));
    }

    private static Layout Parse(byte[] bytes)
    {
        var props = new List<Prop>();
        var open = 0;
        var close = 0;
        try
        {
            var reader = new Utf8JsonReader(bytes, new JsonReaderOptions
            {
                CommentHandling = JsonCommentHandling.Skip,
                AllowTrailingCommas = true,
            });
            if (!reader.Read() || reader.TokenType != JsonTokenType.StartObject)
                throw new FormatException("settings.json isn't a JSON object, so Sobreiro left it alone.");
            open = (int)reader.TokenStartIndex;
            while (reader.Read())
            {
                if (reader.TokenType == JsonTokenType.EndObject)
                {
                    close = (int)reader.TokenStartIndex;
                    break;
                }
                var name = reader.GetString()!;
                var nameStart = (int)reader.TokenStartIndex;
                reader.Read();
                var valueStart = (int)reader.TokenStartIndex;
                reader.Skip();
                props.Add(new Prop(name, nameStart, valueStart, (int)reader.BytesConsumed));
            }
            reader.Read(); // anything after the object is an error
        }
        catch (JsonException e)
        {
            throw new FormatException("settings.json isn't valid JSON, so Sobreiro left it alone.", e);
        }
        return new Layout(props, open, close);
    }

    private static bool IsBlank(byte c) => c == ' ' || c == '\t';

    private static byte[] Remove(byte[] b, Layout layout, int index)
    {
        var p = layout.Props[index];
        var start = p.NameStart;
        var end = p.ValueEnd;
        var afterValue = SkipTrivia(b, end);
        var hasComma = afterValue < b.Length && b[afterValue] == ',';
        if (hasComma) end = afterValue + 1;

        if (layout.Props.Count == 1
            && AllWhitespace(b, layout.Open + 1, start)
            && AllWhitespace(b, end, layout.Close))
            return Splice(b, layout.Open + 1, layout.Close, []);

        var last = index == layout.Props.Count - 1;
        var previousComma = -1;
        if (last && !hasComma && index > 0)
        {
            var i = SkipTrivia(b, layout.Props[index - 1].ValueEnd);
            if (i < b.Length && b[i] == ',') previousComma = i;
        }

        var lineStart = start;
        while (lineStart > 0 && IsBlank(b[lineStart - 1])) lineStart--;
        var lineEnd = end;
        while (lineEnd < b.Length && IsBlank(b[lineEnd])) lineEnd++;
        var ownLine = (lineStart == 0 || b[lineStart - 1] == '\n')
            && lineEnd < b.Length && (b[lineEnd] == '\n' || b[lineEnd] == '\r');

        if (ownLine)
        {
            if (b[lineEnd] == '\r') lineEnd++;
            if (lineEnd < b.Length && b[lineEnd] == '\n') lineEnd++;
            var result = Splice(b, lineStart, lineEnd, []);
            // the previous comma sits before lineStart, so its index is unchanged
            return previousComma >= 0 ? Splice(result, previousComma, previousComma + 1, []) : result;
        }

        if (previousComma >= 0) return Splice(b, previousComma, p.ValueEnd, []);
        return Splice(b, start, lineEnd, []);
    }

    private static byte[] Insert(byte[] b, string text, Layout layout, string valueJson)
    {
        var newline = text.Contains("\r\n", StringComparison.Ordinal) ? "\r\n" : "\n";
        var property = "\"" + Key + "\": " + valueJson;

        if (layout.Props.Count == 0)
        {
            if (AllWhitespace(b, layout.Open + 1, layout.Close))
                return Splice(b, layout.Open + 1, layout.Close, Encoding.UTF8.GetBytes(newline + "  " + property + newline));
            return Splice(b, layout.Open + 1, layout.Open + 1, Encoding.UTF8.GetBytes(newline + "  " + property));
        }

        var last = layout.Props[^1];
        var lineStart = last.NameStart;
        while (lineStart > 0 && IsBlank(b[lineStart - 1])) lineStart--;
        var indent = lineStart == 0 || b[lineStart - 1] == '\n'
            ? Encoding.UTF8.GetString(b, lineStart, last.NameStart - lineStart)
            : "  ";

        var after = SkipTrivia(b, last.ValueEnd);
        if (after < b.Length && b[after] == ',')
            return Splice(b, after + 1, after + 1, Encoding.UTF8.GetBytes(newline + indent + property + ","));
        return Splice(b, last.ValueEnd, last.ValueEnd, Encoding.UTF8.GetBytes("," + newline + indent + property));
    }

    private static string Decode(byte[] b) => Encoding.UTF8.GetString(b);

    private static byte[] Splice(byte[] b, int start, int end, byte[] insert)
    {
        var result = new byte[b.Length - (end - start) + insert.Length];
        Buffer.BlockCopy(b, 0, result, 0, start);
        Buffer.BlockCopy(insert, 0, result, start, insert.Length);
        Buffer.BlockCopy(b, end, result, start + insert.Length, b.Length - end);
        return result;
    }

    private static bool AllWhitespace(byte[] b, int start, int end)
    {
        for (var i = start; i < end; i++)
            if (!(b[i] == ' ' || b[i] == '\t' || b[i] == '\r' || b[i] == '\n')) return false;
        return true;
    }

    /// <summary>Index of the next byte that is not whitespace or part of a comment.</summary>
    private static int SkipTrivia(byte[] b, int i)
    {
        while (i < b.Length)
        {
            if (b[i] == ' ' || b[i] == '\t' || b[i] == '\r' || b[i] == '\n') i++;
            else if (b[i] == '/' && i + 1 < b.Length && b[i + 1] == '/')
            {
                while (i < b.Length && b[i] != '\n') i++;
            }
            else if (b[i] == '/' && i + 1 < b.Length && b[i + 1] == '*')
            {
                i += 2;
                while (i + 1 < b.Length && !(b[i] == '*' && b[i + 1] == '/')) i++;
                i = Math.Min(i + 2, b.Length);
            }
            else break;
        }
        return i;
    }
}
