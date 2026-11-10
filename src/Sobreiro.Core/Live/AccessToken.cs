namespace Sobreiro.Core.Live;

/// <summary>
/// Claude Code's OAuth access token, held in memory for one request. A class, not a record, so nothing
/// prints it: <see cref="ToString"/> is redacted.
/// </summary>
public sealed class AccessToken(string value, DateTimeOffset? expiresAt)
{
    public string Value { get; } = value;

    public DateTimeOffset? ExpiresAt { get; } = expiresAt;

    public override string ToString() => "AccessToken(redacted)";
}
