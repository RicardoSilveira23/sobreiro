using Sobreiro.Core.Polling;

namespace Sobreiro.Core.Live;

public static class LiveStateText
{
    /// <summary>The flyout footer sentence for a Live usage that is on but not working; null for Off and Ok.</summary>
    public static string? Footer(ProviderState state) => state switch
    {
        ProviderState.Expired => "Claude Code's sign-in has expired. Use Claude Code in a terminal and it renews it.",
        ProviderState.NoSignIn => "Claude Code isn't signed in on this PC. Run claude in a terminal and sign in.",
        ProviderState.Refused => "Anthropic didn't accept Claude Code's sign-in. Use Claude Code in a terminal so it gets a new one.",
        ProviderState.RateLimited => "Anthropic asked Sobreiro to wait. Nothing to do: it asks again by itself.",
        ProviderState.Offline => "Sobreiro can't reach Anthropic. Check the connection; it tries again every 2 minutes.",
        ProviderState.BadAnswer => "Anthropic's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro.",
        _ => null,
    };
}
