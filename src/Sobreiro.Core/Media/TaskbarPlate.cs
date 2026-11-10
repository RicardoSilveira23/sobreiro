using System.Text.Json;
using Sobreiro.Core.Display;

namespace Sobreiro.Core.Media;

/// <summary>
/// One real capture of the right part of the taskbar and where things sit on it. Every rect,
/// band and ground value is in screen pixels; the image's top-left pixel is at (OriginX, OriginY).
/// </summary>
public sealed record TaskbarPlate(
    string Image,
    DateTimeOffset CapturedAt,
    bool Dark,
    int OriginX,
    int OriginY,
    PixelRect Monitor,
    PixelRect WorkArea,
    PixelRect Taskbar,
    int BandTop,
    int BandBottom,
    PixelRect Pill,
    PixelRect? ClaudeIcon,
    PixelRect? CopilotIcon,
    int GroundLeft,
    int GroundRight,
    int IconPx = 16)
{
    public static TaskbarPlate Parse(string json)
    {
        JsonElement root;
        try { root = JsonDocument.Parse(json).RootElement; }
        catch (JsonException e) { throw new FormatException("Plate JSON is not valid: " + e.Message, e); }

        var origin = Field(root, "origin", "origin");
        var band = Field(root, "band", "band");
        var icons = Field(root, "icons", "icons");
        var ground = Field(root, "ground", "ground");

        return new TaskbarPlate(
            Image: Field(root, "image", "image").GetString() ?? throw Missing("image"),
            CapturedAt: Field(root, "capturedAt", "capturedAt").GetDateTimeOffset(),
            Dark: Field(root, "dark", "dark").GetBoolean(),
            OriginX: Field(origin, "x", "origin.x").GetInt32(),
            OriginY: Field(origin, "y", "origin.y").GetInt32(),
            Monitor: Rect(root, "monitor", "monitor"),
            WorkArea: Rect(root, "workArea", "workArea"),
            Taskbar: Rect(root, "taskbar", "taskbar"),
            BandTop: Field(band, "top", "band.top").GetInt32(),
            BandBottom: Field(band, "bottom", "band.bottom").GetInt32(),
            Pill: Rect(root, "pill", "pill"),
            ClaudeIcon: OptionalRect(icons, "claude", "icons.claude"),
            CopilotIcon: OptionalRect(icons, "copilot", "icons.copilot"),
            GroundLeft: Field(ground, "left", "ground.left").GetInt32(),
            GroundRight: Field(ground, "right", "ground.right").GetInt32(),
            IconPx: root.TryGetProperty("iconPx", out var px) && px.ValueKind != JsonValueKind.Null ? px.GetInt32() : 16);
    }

    /// <summary>
    /// The screen x of the ground column that fills <paramref name="screenX"/> when a stretch is
    /// cleared from the stretch's left edge: the ground tiled by modulo over GroundLeft..GroundRight-1.
    /// </summary>
    public int GroundColumn(int screenX)
    {
        var width = GroundRight - GroundLeft;
        if (width <= 0) throw new FormatException("Plate JSON \"ground\" has no width.");
        return GroundLeft + ((screenX - GroundLeft) % width + width) % width;
    }

    /// <summary>
    /// The screen x of the output's left edge: <paramref name="margin"/> left of the leftmost drawn
    /// rect, never left of the capture (<see cref="OriginX"/>). With nothing drawn, the capture's edge.
    /// </summary>
    public int CropLeft(IEnumerable<PixelRect> drawn, int margin)
    {
        var left = int.MaxValue;
        foreach (var r in drawn) left = Math.Min(left, r.Left);
        return left == int.MaxValue ? OriginX : Math.Max(OriginX, left - margin);
    }

    /// <summary><see cref="Pill"/> widened leftwards to <paramref name="widestWidth"/>, right edge kept; never narrower.</summary>
    public PixelRect ClearedPill(int widestWidth) =>
        widestWidth <= Pill.Width ? Pill : Pill with { Left = Pill.Right - widestWidth };

    /// <summary>Right edge on the captured pill's, vertically centred in the band (as <c>PillPlacement.Place</c>).</summary>
    public (int X, int Y) PillAt(int width, int height) =>
        (Pill.Right - width, BandTop + (BandBottom - BandTop - height) / 2);

    /// <summary>Where the details open for a click on the pill; <paramref name="margin"/> is the flyout's.</summary>
    public (int X, int Y) DetailsAt(int width, int height, int margin) =>
        FlyoutPlacement.Place(Monitor, WorkArea, (Pill.Left + Pill.Right) / 2, (Pill.Top + Pill.Bottom) / 2, width, height, margin);

    private static FormatException Missing(string name) => new($"Plate JSON is missing \"{name}\".");

    private static JsonElement Field(JsonElement parent, string name, string path)
    {
        if (parent.ValueKind == JsonValueKind.Object
            && parent.TryGetProperty(name, out var v) && v.ValueKind != JsonValueKind.Null)
            return v;
        throw Missing(path);
    }

    private static PixelRect Rect(JsonElement parent, string name, string path) => ToRect(Field(parent, name, path), path);

    private static PixelRect? OptionalRect(JsonElement parent, string name, string path) =>
        parent.TryGetProperty(name, out var v) && v.ValueKind != JsonValueKind.Null ? ToRect(v, path) : null;

    private static PixelRect ToRect(JsonElement e, string path) => new(
        Field(e, "left", path + ".left").GetInt32(),
        Field(e, "top", path + ".top").GetInt32(),
        Field(e, "right", path + ".right").GetInt32(),
        Field(e, "bottom", path + ".bottom").GetInt32());
}
