using System.Text.Json;
using Sobreiro.Core.Usage;

namespace Sobreiro.Core.Live;

/// <summary>
/// Reads the reply of GET /api/oauth/usage. Prefers the "limits" list (group session or weekly, unscoped entries
/// only); a window the list lacks comes from the keyed five_hour and seven_day objects. Anything else is ignored.
/// Windows come back with ObservedAt null; the caller's merge stamps it.
/// </summary>
public static class UsageApiJson
{
    public static RateLimits? Parse(string json)
    {
        try
        {
            using var doc = JsonDocument.Parse(json);
            var root = doc.RootElement;
            if (root.ValueKind != JsonValueKind.Object) return null;

            var (five, seven) = FromList(root);
            if (five is null || seven is null)
            {
                var (keyedFive, keyedSeven) = FromKeys(root);
                five ??= keyedFive;
                seven ??= keyedSeven;
            }
            return five is null && seven is null ? null : new RateLimits(five, seven, null);
        }
        catch (JsonException)
        {
            return null;
        }
    }

    private static (LimitWindow?, LimitWindow?) FromList(JsonElement root)
    {
        LimitWindow? five = null, seven = null;
        if (!root.TryGetProperty("limits", out var list) || list.ValueKind != JsonValueKind.Array) return (null, null);

        foreach (var entry in list.EnumerateArray())
        {
            if (entry.ValueKind != JsonValueKind.Object) continue;
            if (entry.TryGetProperty("scope", out var scope) && scope.ValueKind != JsonValueKind.Null) continue;
            if (!entry.TryGetProperty("group", out var group) || group.ValueKind != JsonValueKind.String) continue;

            var window = Window(entry, "percent");
            if (window is null) continue;
            switch (group.GetString())
            {
                case "session": five ??= window; break;
                case "weekly": seven ??= window; break;
            }
        }
        return (five, seven);
    }

    private static (LimitWindow?, LimitWindow?) FromKeys(JsonElement root) =>
        (Keyed(root, "five_hour"), Keyed(root, "seven_day"));

    private static LimitWindow? Keyed(JsonElement root, string key) =>
        root.TryGetProperty(key, out var value) && value.ValueKind == JsonValueKind.Object
            ? Window(value, "utilization")
            : null;

    private static LimitWindow? Window(JsonElement element, string percentKey)
    {
        if (!element.TryGetProperty(percentKey, out var percent)
            || percent.ValueKind != JsonValueKind.Number
            || !percent.TryGetDouble(out var used))
            return null;

        DateTimeOffset? resetsAt = null;
        if (element.TryGetProperty("resets_at", out var reset)
            && reset.ValueKind == JsonValueKind.String
            && reset.TryGetDateTimeOffset(out var parsed))
            resetsAt = parsed;
        return new LimitWindow(used, resetsAt);
    }
}
