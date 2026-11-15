using System.Text;
using System.Text.Json;

namespace Sobreiro.Core.Usage;

/// <summary>Reads the status line / usage.json shape and writes usage.json. No reflection.</summary>
public static class RateLimitsJson
{
    /// <summary>Parses Claude Code's status line stdin. Null when there is no usable rate_limits.</summary>
    public static RateLimits? ParseStatusLine(string json)
    {
        try
        {
            using var doc = JsonDocument.Parse(json);
            if (doc.RootElement.ValueKind != JsonValueKind.Object) return null;
            if (!doc.RootElement.TryGetProperty("rate_limits", out var rl)) return null;
            return ParseRateLimits(rl, readObservedAt: false);
        }
        catch (JsonException)
        {
            return null;
        }
    }

    /// <summary>Parses usage.json. Null without observed_at or usable rate_limits.</summary>
    public static UsageSnapshot? ParseUsageFile(string json)
    {
        try
        {
            using var doc = JsonDocument.Parse(json);
            var root = doc.RootElement;
            if (root.ValueKind != JsonValueKind.Object) return null;
            if (!root.TryGetProperty("observed_at", out var obs) || !TryUnix(obs, out var observedAt)) return null;
            if (!root.TryGetProperty("rate_limits", out var rl)) return null;
            var limits = ParseRateLimits(rl, readObservedAt: true);
            return limits is null ? null : new UsageSnapshot(limits, observedAt);
        }
        catch (JsonException)
        {
            return null;
        }
    }

    public static string WriteUsageFile(UsageSnapshot s)
    {
        using var stream = new MemoryStream();
        using (var w = new Utf8JsonWriter(stream))
        {
            w.WriteStartObject();
            w.WriteNumber("observed_at", s.ObservedAt.ToUnixTimeSeconds());
            w.WriteStartObject("rate_limits");
            if (s.Limits.FiveHour is { } five) WriteWindow(w, "five_hour", five.UsedPercentage, five.ResetsAt, five.ObservedAt);
            if (s.Limits.SevenDay is { } seven) WriteWindow(w, "seven_day", seven.UsedPercentage, seven.ResetsAt, seven.ObservedAt);
            if (s.Limits.Spend is { } spend)
            {
                w.WriteStartObject("spend_limit");
                w.WriteNumber("used_percentage", spend.UsedPercentage);
                if (spend.ResetsAt is { } r) w.WriteNumber("resets_at", r.ToUnixTimeSeconds());
                if (spend.UsedUsd is { } used) w.WriteNumber("used_usd", used);
                if (spend.LimitUsd is { } limit) w.WriteNumber("limit_usd", limit);
                if (spend.Period is { } period) w.WriteString("period", period);
                if (spend.ObservedAt is { } spendAt) w.WriteNumber("observed_at", spendAt.ToUnixTimeSeconds());
                w.WriteEndObject();
            }
            w.WriteEndObject();
            w.WriteEndObject();
        }
        return Encoding.UTF8.GetString(stream.ToArray());
    }

    private static void WriteWindow(Utf8JsonWriter w, string name, double pct, DateTimeOffset? resetsAt, DateTimeOffset? observedAt)
    {
        w.WriteStartObject(name);
        w.WriteNumber("used_percentage", pct);
        if (resetsAt is { } r) w.WriteNumber("resets_at", r.ToUnixTimeSeconds());
        if (observedAt is { } o) w.WriteNumber("observed_at", o.ToUnixTimeSeconds());
        w.WriteEndObject();
    }

    private static RateLimits? ParseRateLimits(JsonElement rl, bool readObservedAt)
    {
        if (rl.ValueKind != JsonValueKind.Object) return null;
        var five = ParseWindow(rl, "five_hour", readObservedAt);
        var seven = ParseWindow(rl, "seven_day", readObservedAt);
        var spend = ParseSpend(rl, readObservedAt);
        return five is null && seven is null && spend is null ? null : new RateLimits(five, seven, spend);
    }

    private static LimitWindow? ParseWindow(JsonElement parent, string name, bool readObservedAt)
    {
        if (!TryObject(parent, name, out var el) || !TryNumber(el, "used_percentage", out var pct)) return null;
        return new LimitWindow(pct, ResetsAt(el), readObservedAt ? ObservedAt(el) : null);
    }

    private static SpendLimit? ParseSpend(JsonElement parent, bool readObservedAt)
    {
        if (!TryObject(parent, "spend_limit", out var el) || !TryNumber(el, "used_percentage", out var pct)) return null;
        double? usedUsd = TryNumber(el, "used_usd", out var u) ? u : null;
        double? limitUsd = TryNumber(el, "limit_usd", out var l) ? l : null;
        string? period = null;
        if (el.TryGetProperty("period", out var p) && p.ValueKind == JsonValueKind.String)
        {
            var text = p.GetString();
            if (text is "daily" or "weekly" or "monthly") period = text;
        }
        return new SpendLimit(pct, ResetsAt(el), usedUsd, limitUsd, period, readObservedAt ? ObservedAt(el) : null);
    }

    private static DateTimeOffset? ResetsAt(JsonElement el) =>
        el.TryGetProperty("resets_at", out var r) && TryUnix(r, out var t) ? t : null;

    private static DateTimeOffset? ObservedAt(JsonElement el) =>
        el.TryGetProperty("observed_at", out var r) && TryUnix(r, out var t) ? t : null;

    private static bool TryObject(JsonElement parent, string name, out JsonElement el) =>
        parent.TryGetProperty(name, out el) && el.ValueKind == JsonValueKind.Object;

    private static bool TryNumber(JsonElement parent, string name, out double value)
    {
        value = 0;
        return parent.TryGetProperty(name, out var el)
            && el.ValueKind == JsonValueKind.Number
            && el.TryGetDouble(out value);
    }

    private static bool TryUnix(JsonElement el, out DateTimeOffset value)
    {
        value = default;
        if (el.ValueKind != JsonValueKind.Number || !el.TryGetDouble(out var d)) return false;
        try
        {
            value = DateTimeOffset.FromUnixTimeSeconds((long)d);
            return true;
        }
        catch (ArgumentOutOfRangeException)
        {
            return false;
        }
    }
}
