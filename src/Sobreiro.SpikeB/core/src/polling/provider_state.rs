/// Where a provider's poll stands: Off and Ok, or the reason its numbers stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderState {
    Off,
    Ok,
    Expired,
    NoSignIn,
    Refused,
    RateLimited,
    Offline,
    BadAnswer,
}
