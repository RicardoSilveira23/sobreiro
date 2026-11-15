//! Where a provider's polls stand.

/// Where a provider's polls stand: `Off` and `Ok`, or the reason its numbers stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderState {
    /// The provider is switched off.
    Off,
    /// The last poll brought a reading. Also the state before the first poll has answered.
    Ok,
    /// The sign-in has run out.
    Expired,
    /// There is no sign-in on this PC, or none that can be read.
    NoSignIn,
    /// The service did not accept the sign-in.
    Refused,
    /// The service asked Sobreiro to wait.
    RateLimited,
    /// The service could not be reached.
    Offline,
    /// The service answered in a way that is no use.
    BadAnswer,
}
