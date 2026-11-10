using System.Text;
using System.Text.RegularExpressions;

namespace Sobreiro.Core.Copilot;

public enum CopilotSourceKind { Env, CredentialManager, Gh }

/// <summary>What one source found. <c>EntryFound</c> is for a source that saw a sign-in entry but no token in it.</summary>
public sealed record TokenSearch(IReadOnlyList<string> Tokens, bool EntryFound = false)
{
    public static implicit operator TokenSearch(string[] tokens) => new((IReadOnlyList<string>)tokens);
    public static implicit operator TokenSearch(List<string> tokens) => new((IReadOnlyList<string>)tokens);
}

/// <summary>One place to look for a GitHub token. <c>Tokens</c> runs only when the poll reaches this source and may throw (the poll treats that as no token).</summary>
public sealed record CopilotSignIn(CopilotSourceKind Kind, Func<TokenSearch> Tokens);

public static partial class CopilotToken
{
    [GeneratedRegex("(gho|ghu|ghp|github_pat)_[A-Za-z0-9_]+")]
    private static partial Regex Pattern();

    /// <summary>The first GitHub token in the text, or null.</summary>
    public static string? Find(string? text)
    {
        if (string.IsNullOrEmpty(text)) return null;
        var m = Pattern().Match(text);
        return m.Success ? m.Value : null;
    }

    /// <summary>Decodes a credential blob as UTF-8, then as UTF-16LE (BOMs ignored), and returns the first token found.</summary>
    public static string? FromBlob(ReadOnlySpan<byte> blob) =>
        Find(Encoding.UTF8.GetString(blob).TrimStart('﻿')) ?? Find(Encoding.Unicode.GetString(blob).TrimStart('﻿'));

    /// <summary>COPILOT_GITHUB_TOKEN, GH_TOKEN, GITHUB_TOKEN in that order: trimmed, non-empty, distinct, as-is.</summary>
    public static IReadOnlyList<string> FromEnvironment(Func<string, string?> getEnv)
    {
        var tokens = new List<string>();
        foreach (var name in new[] { "COPILOT_GITHUB_TOKEN", "GH_TOKEN", "GITHUB_TOKEN" })
        {
            var value = getEnv(name)?.Trim();
            if (!string.IsNullOrEmpty(value) && !tokens.Contains(value)) tokens.Add(value);
        }
        return tokens;
    }
}
