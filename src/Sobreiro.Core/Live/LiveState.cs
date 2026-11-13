namespace Sobreiro.Core.Live;

public enum LiveState { Off, Ok, Expired, NoSignIn, Refused, RateLimited, Offline, BadAnswer }

public static class LiveStateText
{
    /// <summary>The flyout footer sentence for a Live usage that is on but not working; null for Off and Ok.</summary>
    public static string? Footer(LiveState state) => state switch
    {
        LiveState.Expired => "Claude Code's sign-in has expired. Use Claude Code in a terminal and it renews it.",
        LiveState.NoSignIn => "Claude Code isn't signed in on this PC. Run claude in a terminal and sign in.",
        LiveState.Refused => "Anthropic didn't accept Claude Code's sign-in. Use Claude Code in a terminal so it gets a new one.",
        LiveState.RateLimited => "Anthropic asked Sobreiro to wait. Nothing to do: it asks again by itself.",
        LiveState.Offline => "Sobreiro can't reach Anthropic. Check the connection; it tries again every 2 minutes.",
        LiveState.BadAnswer => "Anthropic's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro.",
        _ => null,
    };
}
