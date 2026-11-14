pub struct LiveUsageMessage;

impl LiveUsageMessage {
    pub const ABOUT_LINE: &'static str =
        "It asks Anthropic for your usage every 5 minutes with Claude Code's sign-in, and makes no other network calls.";
}
