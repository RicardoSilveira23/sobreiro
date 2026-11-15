using System.Text.Json;

namespace Sobreiro.Core.Bridge;

/// <summary>config.json: the user's previous status line command, which the bridge chains to.</summary>
public sealed record BridgeConfig(string? ChainedCommand)
{
    /// <summary>Missing, corrupt, wrong-typed or blank all give a null command.</summary>
    public static BridgeConfig Load(string path)
    {
        try
        {
            using var doc = JsonDocument.Parse(File.ReadAllText(path));
            if (doc.RootElement.ValueKind == JsonValueKind.Object
                && doc.RootElement.TryGetProperty("chained_command", out var value)
                && value.ValueKind == JsonValueKind.String
                && value.GetString() is { } command
                && !string.IsNullOrWhiteSpace(command))
                return new BridgeConfig(command);
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException or JsonException)
        {
        }
        return new BridgeConfig((string?)null);
    }
}
