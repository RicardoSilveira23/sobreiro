use crate::Time;
use crate::secret::Secret;

/// Claude Code's OAuth access token, held in memory for one request. No `Debug`, no `Display`: nothing prints it.
pub struct AccessToken {
    value: Secret,
    expires_at: Option<Time>,
}

impl AccessToken {
    pub fn new(value: String, expires_at: Option<Time>) -> AccessToken {
        AccessToken { value: Secret::new(value), expires_at }
    }

    pub fn expires_at(&self) -> Option<Time> {
        self.expires_at
    }

    /// For the one request that sends it.
    pub(crate) fn value(&self) -> &str {
        self.value.expose()
    }
}
