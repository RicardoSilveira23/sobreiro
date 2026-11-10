using System.Security.Cryptography;
using System.Text.Json;

namespace Sobreiro.Core.Live;

public enum CredentialStatus { Ok, Missing, Unreadable, Expired }

/// <summary>The token is non-null only when the status is <see cref="CredentialStatus.Ok"/>.</summary>
public sealed record CredentialRead(CredentialStatus Status, AccessToken? Token);

/// <summary>
/// Reads claudeAiOauth.accessToken and claudeAiOauth.expiresAt from Claude Code's .credentials.json, into memory
/// only. Nothing else in the file is looked at, and the token never goes into a message.
/// </summary>
public static class ClaudeCredentials
{
    // A sign-in file is a few hundred bytes; anything far bigger is not one.
    private const int MaxBytes = 1 << 20;

    public static CredentialRead Read(string path, DateTimeOffset now)
    {
        byte[]? bytes = null;
        try
        {
            int length;
            // Shared for writing and deleting, so Claude Code can always replace the file while it refreshes; no
            // FileStream buffer, so the bytes below are the only copy of the file to wipe.
            using (var stream = new FileStream(
                path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete, bufferSize: 0))
            {
                if (stream.Length > MaxBytes) return Unreadable;
                bytes = new byte[stream.Length];
                length = stream.ReadAtLeast(bytes, bytes.Length, throwOnEndOfStream: false);
            }
            return Parse(bytes.AsMemory(0, length), now);
        }
        catch (Exception e) when (e is FileNotFoundException or DirectoryNotFoundException)
        {
            return new CredentialRead(CredentialStatus.Missing, null);
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            return Unreadable;
        }
        finally
        {
            if (bytes is not null) CryptographicOperations.ZeroMemory(bytes);
        }
    }

    private static CredentialRead Parse(ReadOnlyMemory<byte> json, DateTimeOffset now)
    {
        if (json.Span.StartsWith("﻿"u8)) json = json[3..];
        try
        {
            using var doc = JsonDocument.Parse(json);
            if (doc.RootElement.ValueKind != JsonValueKind.Object
                || !doc.RootElement.TryGetProperty("claudeAiOauth", out var oauth)
                || oauth.ValueKind != JsonValueKind.Object
                || !oauth.TryGetProperty("accessToken", out var tokenElement)
                || tokenElement.ValueKind != JsonValueKind.String)
                return Unreadable;

            var value = tokenElement.GetString();
            if (string.IsNullOrEmpty(value)) return Unreadable;

            DateTimeOffset? expiresAt = null;
            if (oauth.TryGetProperty("expiresAt", out var expiry) && expiry.ValueKind != JsonValueKind.Null)
            {
                if (expiry.ValueKind != JsonValueKind.Number || !expiry.TryGetInt64(out var millis)) return Unreadable;
                try
                {
                    expiresAt = DateTimeOffset.FromUnixTimeMilliseconds(millis);
                }
                catch (ArgumentOutOfRangeException)
                {
                    return Unreadable;
                }
            }

            if (expiresAt is { } at && at <= now) return new CredentialRead(CredentialStatus.Expired, null);
            return new CredentialRead(CredentialStatus.Ok, new AccessToken(value, expiresAt));
        }
        catch (JsonException)
        {
            return Unreadable;
        }
    }

    private static CredentialRead Unreadable => new(CredentialStatus.Unreadable, null);
}
