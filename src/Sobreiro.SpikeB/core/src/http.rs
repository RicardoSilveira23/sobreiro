//! The one blocking HTTP GET both clients use: no redirects, no cookies, Windows' own TLS, a total timeout.
//! The error text of a failed request is dropped on purpose: a header could be quoted in it.

use crate::{Span, Time};
use std::time::Duration;
use ureq::config::Config;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

/// What came back. The body is read only for a 2xx.
pub(crate) struct Answer {
    pub status: u16,
    pub retry_after: Option<String>,
    pub body: Option<String>,
}

pub(crate) enum Failure {
    /// Could not reach the server, or it timed out.
    Unreachable,
    /// The request could not be made or the answer made no sense.
    Failed,
}

/// The agent: redirects are never followed (a 3xx comes back as an answer, so the token never follows one), a
/// status of 400 or more is an answer and not an error, and there is no cookie store (the crate's cookie feature
/// is off). TLS is the platform's (schannel).
pub(crate) fn agent(timeout: Duration) -> ureq::Agent {
    let tls = TlsConfig::builder().provider(TlsProvider::NativeTls).root_certs(RootCerts::PlatformVerifier).build();
    let config = Config::builder()
        .http_status_as_error(false)
        .max_redirects(0)
        .timeout_global(Some(timeout))
        .tls_config(tls)
        .build();
    ureq::Agent::new_with_config(config)
}

pub(crate) fn get(agent: &ureq::Agent, url: &str, headers: &[(&str, &str)]) -> Result<Answer, Failure> {
    let mut request = agent.get(url);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let mut response = request.call().map_err(map_error)?;
    let status = response.status().as_u16();
    let retry_after = response.headers().get("retry-after").and_then(|v| v.to_str().ok()).map(String::from);
    let body = if (200..300).contains(&status) {
        Some(response.body_mut().read_to_string().map_err(map_error)?)
    } else {
        None
    };
    Ok(Answer { status, retry_after, body })
}

fn map_error(e: ureq::Error) -> Failure {
    use ureq::Error::*;
    match e {
        Io(_) | Timeout(_) | HostNotFound | ConnectionFailed | Tls(_) => Failure::Unreachable,
        _ => Failure::Failed,
    }
}

const MAX_RETRY_AFTER_HOURS: i64 = 6;

/// A Retry-After header: whole seconds or an HTTP date. Never negative, never over six hours.
pub(crate) fn retry_after(value: Option<&str>, now: Time) -> Option<Span> {
    let v = value?.trim();
    let wait = match v.parse::<i64>() {
        Ok(secs) => Span::try_seconds(secs)?,
        Err(_) => chrono::DateTime::parse_from_rfc2822(v).ok()? - now,
    };
    Some(wait.clamp(Span::zero(), Span::hours(MAX_RETRY_AFTER_HOURS)))
}
