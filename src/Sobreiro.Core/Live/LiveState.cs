namespace Sobreiro.Core.Live;

public enum LiveState { Off, Ok, Expired, NoSignIn, Refused, RateLimited, Failing }

public static class LiveStateText
{
    /// <summary>The flyout footer sentence for a Live usage that is on but not working; null for Off and Ok.</summary>
    public static string? Footer(LiveState state) => state switch
    {
        LiveState.Expired => "Live usage is paused until Claude Code renews its sign-in.",
        LiveState.NoSignIn => "Live usage needs Claude Code to be signed in.",
        LiveState.Refused => "Anthropic didn't accept Claude Code's sign-in. Live usage waits for a new one.",
        LiveState.RateLimited => "Anthropic asked Live usage to wait.",
        LiveState.Failing => "Live usage can't reach Anthropic.",
        _ => null,
    };
}
