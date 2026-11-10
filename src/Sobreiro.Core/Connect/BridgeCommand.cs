using System.Text;
using System.Text.Encodings.Web;
using System.Text.Json;

namespace Sobreiro.Core.Connect;

/// <summary>The status line command that runs the bridge, and the statusLine object around it.</summary>
public static class BridgeCommand
{
    private const string BridgeName = "sobreiro-bridge";

    /// <summary>Forward slashes and quoted, so Git Bash runs it as one path.</summary>
    public static string For(string bridgeExePath) => "\"" + bridgeExePath.Replace('\\', '/') + "\"";

    /// <summary>
    /// True when <paramref name="command"/> runs exactly <paramref name="bridgeExePath"/>: whitespace and one pair of
    /// double quotes trimmed, / read as a backslash on both sides, paths made full (.. resolved), case ignored. Anything after the path (a pipe, arguments) makes it false.
    /// </summary>
    public static bool Runs(string? command, string bridgeExePath)
    {
        if (command is null) return false;
        var text = command.Trim();
        if (text.Length >= 2 && text[0] == '"' && text[^1] == '"') text = text[1..^1];
        try
        {
            return string.Equals(Normalise(text), Normalise(bridgeExePath), StringComparison.OrdinalIgnoreCase);
        }
        catch (Exception ex) when (ex is ArgumentException or NotSupportedException or PathTooLongException)
        {
            return false;
        }
    }

    private static string Normalise(string path) => Path.GetFullPath(path.Replace('/', '\\'));

    /// <summary>The bridge the app was called before 0.5.0 (OpenHeadroom).</summary>
    private const string LegacyBridgeName = "openheadroom-bridge";

    /// <summary>True for a command running this app's bridge, or the old one: both are our own.</summary>
    public static bool IsBridge(string? command) =>
        command is not null && command.Contains(BridgeName, StringComparison.OrdinalIgnoreCase) || IsLegacyBridge(command);

    /// <summary>True for a command running the old openheadroom-bridge: connected, but to the old name.</summary>
    public static bool IsLegacyBridge(string? command) =>
        command is not null
        && command.Contains(LegacyBridgeName, StringComparison.OrdinalIgnoreCase)
        && !command.Contains(BridgeName, StringComparison.OrdinalIgnoreCase);

    /// <summary>
    /// A statusLine object running <paramref name="command"/>, laid out for a top-level property (inner keys
    /// at 4 spaces, closing brace at 2). Other properties of the existing object (padding, say) are kept.
    /// </summary>
    public static string StatusLineJson(string command, string? existingStatusLineJson)
    {
        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream, new JsonWriterOptions
        {
            Indented = true,
            IndentSize = 4,
            NewLine = "\n",
            Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
        }))
        {
            writer.WriteStartObject();
            writer.WriteString("type", "command");
            writer.WriteString("command", command);
            if (existingStatusLineJson is not null) WriteOthers(writer, existingStatusLineJson);
            writer.WriteEndObject();
        }
        var text = Encoding.UTF8.GetString(stream.ToArray());
        return text[..^1] + "  }";
    }

    private static void WriteOthers(Utf8JsonWriter writer, string existingJson)
    {
        JsonDocument doc;
        try
        {
            doc = JsonDocument.Parse(existingJson, new JsonDocumentOptions
            {
                CommentHandling = JsonCommentHandling.Skip,
                AllowTrailingCommas = true,
            });
        }
        catch (JsonException)
        {
            return;
        }
        using (doc)
        {
            if (doc.RootElement.ValueKind != JsonValueKind.Object) return;
            foreach (var property in doc.RootElement.EnumerateObject())
            {
                if (property.Name is "type" or "command") continue;
                writer.WritePropertyName(property.Name);
                property.Value.WriteTo(writer);
            }
        }
    }
}
