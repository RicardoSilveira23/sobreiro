using System.Text.Json;

namespace Sobreiro.Core.Copilot;

/// <summary>
/// Reads the Copilot CLI's own cache, %LOCALAPPDATA%\copilot\copilot-user-cache.json: a few "// ..." comment lines,
/// then {"copilotUserCache":{"v1:...":{"retrievedAt":"...","response":{...}}}}. Of the entries that hold a budget
/// (a free seat's entry does not), the one retrieved last wins. Never throws.
/// </summary>
public static class CopilotCache
{
    public static string DefaultPath(Func<string, string?> getEnv)
    {
        var local = getEnv("LOCALAPPDATA");
        if (string.IsNullOrEmpty(local)) local = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        return Path.Combine(local, "copilot", "copilot-user-cache.json");
    }

    public static CopilotReading? Parse(string text)
    {
        try
        {
            var json = string.Join('\n', text.Split('\n').Where(line => !line.TrimStart().StartsWith("//", StringComparison.Ordinal)));
            using var doc = JsonDocument.Parse(json);
            if (doc.RootElement.ValueKind != JsonValueKind.Object
                || !doc.RootElement.TryGetProperty("copilotUserCache", out var entries)
                || entries.ValueKind != JsonValueKind.Object)
                return null;

            CopilotReading? best = null;
            DateTimeOffset bestRetrievedAt = default;
            foreach (var entry in entries.EnumerateObject())
            {
                if (entry.Value.ValueKind != JsonValueKind.Object) continue;
                if (!entry.Value.TryGetProperty("retrievedAt", out var at) || at.ValueKind != JsonValueKind.String
                    || !at.TryGetDateTimeOffset(out var retrievedAt)) continue;
                if (!entry.Value.TryGetProperty("response", out var response)) continue;
                if (CopilotUserJson.FromResponse(response, retrievedAt, CopilotSource.Cache) is not { } reading) continue;
                if (best is null || retrievedAt > bestRetrievedAt)
                {
                    best = reading;
                    bestRetrievedAt = retrievedAt;
                }
            }
            return best;
        }
        catch (JsonException)
        {
            return null;
        }
    }

    public static CopilotReading? Read(string path)
    {
        try
        {
            return Parse(File.ReadAllText(path));
        }
        catch (Exception)
        {
            return null;
        }
    }
}
