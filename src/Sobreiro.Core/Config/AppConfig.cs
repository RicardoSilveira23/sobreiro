using System.Text;
using System.Text.Encodings.Web;
using System.Text.Json;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Display;
using Sobreiro.Core.IO;

namespace Sobreiro.Core.Config;

/// <summary>The status line the user had before Connect: the raw JSON object, or null when there was none.</summary>
public sealed record PreviousStatusLine(string? RawJson);

/// <summary>What the tray shows: the icon alone, or the icon and the taskbar pill.</summary>
public enum ShowOption { TrayAndPill, TrayOnly }

/// <summary>config.json. Unknown keys are kept as raw JSON and written back on save.</summary>
public sealed record AppConfig(
    string? ChainedCommand,
    PreviousStatusLine? Previous,
    FlyoutStyle Style,
    BarColourOption BarColour,
    IReadOnlyDictionary<string, string> Extra)
{
    private const string ChainedKey = "chained_command";
    private const string PreviousKey = "previous_status_line";
    private const string StyleKey = "style";
    private const string ColourKey = "bar_colour";
    private const string ShowKey = "show";

    private const string PaceKey = "pace";

    public ShowOption Show { get; init; } = ShowOption.TrayAndPill;

    public PaceDays PaceDays { get; init; } = PaceDays.EveryDay;

    public static readonly AppConfig Default = new(
        null, null, FlyoutStyle.Bars, BarColourOption.Purple, new Dictionary<string, string>());

    /// <summary>
    /// A missing file or folder, or an empty or blank file, gives <see cref="Default"/>. JSON that doesn't parse,
    /// or isn't an object, throws <see cref="FormatException"/>; any other IOException or
    /// UnauthorizedAccessException (a locked file, say) is rethrown. Either way a caller never saves over a
    /// config it could not read. A blank chained command counts as none.
    /// </summary>
    public static AppConfig Load(string path)
    {
        string text;
        try
        {
            text = File.ReadAllText(path);
        }
        catch (Exception e) when (e is FileNotFoundException or DirectoryNotFoundException)
        {
            return Default;
        }
        if (string.IsNullOrWhiteSpace(text)) return Default;

        try
        {
            using var doc = JsonDocument.Parse(text);
            if (doc.RootElement.ValueKind != JsonValueKind.Object) throw NotJson(null);

            string? chained = null;
            PreviousStatusLine? previous = null;
            var style = FlyoutStyle.Bars;
            var colour = BarColourOption.Purple;
            var show = ShowOption.TrayAndPill;
            var pace = PaceDays.EveryDay;
            var extra = new Dictionary<string, string>();

            foreach (var property in doc.RootElement.EnumerateObject())
            {
                var value = property.Value;
                switch (property.Name)
                {
                    case ChainedKey:
                        if (value.ValueKind == JsonValueKind.String && !string.IsNullOrWhiteSpace(value.GetString()))
                            chained = value.GetString();
                        break;
                    case PreviousKey:
                        previous = new PreviousStatusLine(value.ValueKind == JsonValueKind.Null ? null : value.GetRawText());
                        break;
                    case StyleKey:
                        style = ParseEnum(value, FlyoutStyle.Bars);
                        break;
                    case ColourKey:
                        colour = ParseEnum(value, BarColourOption.Purple);
                        break;
                    case ShowKey:
                        show = ParseEnum(value, ShowOption.TrayAndPill);
                        break;
                    case PaceKey:
                        pace = ParseEnum(value, PaceDays.EveryDay);
                        break;
                    default:
                        extra[property.Name] = value.GetRawText();
                        break;
                }
            }
            return new AppConfig(chained, previous, style, colour, extra) { Show = show, PaceDays = pace };
        }
        catch (JsonException e)
        {
            throw NotJson(e);
        }
    }

    private static FormatException NotJson(Exception? inner) =>
        new("config.json isn't valid JSON, so Sobreiro left it alone.", inner);

    public void Save(string path) => AtomicFile.WriteAllText(path, ToJson());

    /// <summary>The text Save writes. Throws when a raw value is not valid JSON, before anything touches the disk.</summary>
    public string ToJson()
    {
        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream, new JsonWriterOptions { Indented = true, Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping }))
        {
            writer.WriteStartObject();
            if (ChainedCommand is not null) writer.WriteString(ChainedKey, ChainedCommand);
            if (Previous is not null)
            {
                writer.WritePropertyName(PreviousKey);
                if (Previous.RawJson is null) writer.WriteNullValue();
                else writer.WriteRawValue(Previous.RawJson);
            }
            writer.WriteString(StyleKey, Style.ToString().ToLowerInvariant());
            writer.WriteString(ColourKey, BarColour.ToString().ToLowerInvariant());
            writer.WriteString(ShowKey, Show.ToString().ToLowerInvariant());
            writer.WriteString(PaceKey, PaceDays.ToString().ToLowerInvariant());
            foreach (var (key, raw) in Extra)
            {
                writer.WritePropertyName(key);
                writer.WriteRawValue(raw);
            }
            writer.WriteEndObject();
        }
        return Encoding.UTF8.GetString(stream.ToArray());
    }

    private static T ParseEnum<T>(JsonElement value, T fallback) where T : struct, Enum =>
        value.ValueKind == JsonValueKind.String
        && Enum.TryParse<T>(value.GetString(), ignoreCase: true, out var parsed)
        && Enum.GetNames<T>().Any(n => string.Equals(n, value.GetString(), StringComparison.OrdinalIgnoreCase))
            ? parsed
            : fallback;
}
