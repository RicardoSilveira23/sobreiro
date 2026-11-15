using System.Text.Json;

namespace Sobreiro.Core.Copilot;

/// <summary>
/// Reads the reply of GET https://api.github.com/copilot_internal/user. The AI credits sit in
/// quota_snapshots.premium_models when that is an object, else premium_interactions. Every field is optional and a
/// field of the wrong type counts as absent; a reply with no usable budget and usage gives null. Never throws.
/// </summary>
public static class CopilotUserJson
{
    public static CopilotReading? Parse(string body, DateTimeOffset now)
    {
        try
        {
            using var doc = JsonDocument.Parse(body);
            return FromResponse(doc.RootElement, now, CopilotSource.Live);
        }
        catch (JsonException)
        {
            return null;
        }
    }

    internal static CopilotReading? FromResponse(JsonElement response, DateTimeOffset fallbackObservedAt, CopilotSource source)
    {
        if (response.ValueKind != JsonValueKind.Object) return null;
        if (!response.TryGetProperty("quota_snapshots", out var snapshots) || snapshots.ValueKind != JsonValueKind.Object)
            return null;

        var snapshot = Snapshot(snapshots, "premium_models") ?? Snapshot(snapshots, "premium_interactions");
        if (snapshot is not { } s) return null;

        if (s.TryGetProperty("unlimited", out var unlimited) && unlimited.ValueKind == JsonValueKind.True) return null;
        if (Number(s, "entitlement") is not { } budget || budget <= 0) return null;

        if (Used(s, budget) is not { } used) return null;
        if (Number(s, "overage_count") is { } overage && overage > 0 && used <= budget) used = budget + overage;

        var observedAt = Date(s, "timestamp_utc") ?? fallbackObservedAt;
        // A reset date far from the reading is nonsense: treat it as absent.
        var resetsAt = Date(response, "quota_reset_date_utc");
        if (resetsAt is { } reset && (reset - observedAt).Duration() > TimeSpan.FromDays(400)) resetsAt = null;

        return new CopilotReading(budget, used, resetsAt, observedAt, source);
    }

    private static JsonElement? Snapshot(JsonElement snapshots, string key) =>
        snapshots.TryGetProperty(key, out var value) && value.ValueKind == JsonValueKind.Object ? value : null;

    private static double? Used(JsonElement s, double budget)
    {
        if (Number(s, "quota_remaining") is { } quotaRemaining) return budget - quotaRemaining;
        if (Number(s, "remaining") is { } remaining) return budget - remaining;
        if (Number(s, "percent_remaining") is { } percentRemaining) return budget * (100 - percentRemaining) / 100;
        return Number(s, "credits_used");
    }

    private static double? Number(JsonElement element, string key) =>
        element.TryGetProperty(key, out var value) && value.ValueKind == JsonValueKind.Number && value.TryGetDouble(out var number)
            ? number
            : null;

    private static DateTimeOffset? Date(JsonElement element, string key) =>
        element.TryGetProperty(key, out var value) && value.ValueKind == JsonValueKind.String && value.TryGetDateTimeOffset(out var date)
            ? date
            : null;
}
