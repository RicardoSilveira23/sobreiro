//! One HTTP request and its answer: the only way Core reaches the network.
//!
//! Core asks for two things over the network and nothing else: Claude Code's usage, and
//! Copilot's. Each has a client of its own, which knows its one address and what to make of
//! the answer. Both send through a [`Sender`], so that a test can hand a client a stand-in and
//! no test has to reach the real address.
//!
//! The real sender is [`NetworkSender`]. It follows no redirect, keeps no cookie, adds no
//! header of its own, speaks plain HTTP to this PC only, and gives up after the time it was
//! told.
use std::time::Duration;

use ureq::config::{AutoHeaderValue, Config};
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

/// What came back from a request.
///
/// There is no place in it for the address a redirect names, nor for a cookie: a client that
/// is handed an answer cannot act on either.
pub struct Answer {
    /// The status, such as 200 or 429.
    pub status: u16,
    /// The `Retry-After` header as the server wrote it, when there is one.
    pub retry_after: Option<String>,
    /// The body, read only when the status is one of success (200 to 299). Empty otherwise.
    pub body: Vec<u8>,
}

/// Why there is no answer. Neither carries any words: the text of an error could quote a
/// header, and the `Authorization` header holds the token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoAnswer {
    /// The server could not be reached, did not answer in time, or broke off.
    Unreachable,
    /// The request could not be made, or the answer was more than Sobreiro reads.
    Failed,
}

/// Something that sends one GET and brings back the answer.
///
/// `Send + Sync` says that a sender may be shared between threads: a request is made away
/// from the thread that draws the window.
pub trait Sender: Send + Sync {
    /// Asks `url` with these headers and nothing more. The headers are `(name, value)`.
    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<Answer, NoAnswer>;
}

/// The largest body that is read. The answers Sobreiro asks for are a few thousand bytes.
const MAX_BODY_BYTES: u64 = 1 << 20;

/// The sender that really uses the network.
pub struct NetworkSender {
    agent: ureq::Agent,
    timeout: Duration,
}

impl NetworkSender {
    /// A sender that gives up when `timeout` has passed since a request began, wherever the
    /// request has got to by then.
    pub fn new(timeout: Duration) -> NetworkSender {
        // Windows' own TLS, and the certificates Windows trusts.
        let tls = TlsConfig::builder()
            .provider(TlsProvider::NativeTls)
            .root_certs(RootCerts::PlatformVerifier)
            .build();
        let config = Config::builder()
            // A redirect is never followed: the answer that names another address comes back
            // as it is, so the token cannot follow it there.
            .max_redirects(0)
            // A status of 400 or more is an answer to read, not an error.
            .http_status_as_error(false)
            .timeout_global(Some(timeout))
            .tls_config(tls)
            // No proxy is looked up in the environment. Core asks the system for nothing by
            // itself; a proxy would have to be handed in.
            .proxy(None)
            // The library adds no header of its own: the request holds what it was given.
            .user_agent(AutoHeaderValue::None)
            .accept(AutoHeaderValue::None)
            .accept_encoding(AutoHeaderValue::None)
            .build();
        // The library is built without its cookie feature: there is no cookie code in the
        // program to switch off.
        NetworkSender {
            agent: ureq::Agent::new_with_config(config),
            timeout,
        }
    }

    /// The time after which a request is given up.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

impl Sender for NetworkSender {
    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<Answer, NoAnswer> {
        if !may_be_asked(url) {
            return Err(NoAnswer::Failed);
        }
        let mut request = self.agent.get(url);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        // From here on an error is turned into one of two names and its own text is dropped.
        let mut response = request.call().map_err(no_answer)?;

        let status = response.status().as_u16();
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let body = if (200..300).contains(&status) {
            response
                .body_mut()
                .with_config()
                .limit(MAX_BODY_BYTES)
                .read_to_vec()
                .map_err(no_answer)?
        } else {
            Vec::new()
        };
        Ok(Answer {
            status,
            retry_after,
            body,
        })
    }
}

/// Whether a request may go to `url`. Over HTTPS it may go anywhere; which address that is, is
/// for the client to say, and each client has one. Over plain HTTP, where anyone on the way
/// could read a token, it may go to this PC itself and nowhere else: `127.0.0.1`, which is
/// where the tests keep their server.
fn may_be_asked(url: &str) -> bool {
    let Ok(address) = url.parse::<ureq::http::Uri>() else {
        return false;
    };
    matches!(
        (address.scheme_str(), address.host()),
        (Some("https"), Some(_)) | (Some("http"), Some("127.0.0.1"))
    )
}

/// Which of the two names an error of the library gets. Its text goes no further.
fn no_answer(error: ureq::Error) -> NoAnswer {
    match error {
        // The request itself was wrong (an address or a header that cannot be), or the body
        // was over the limit.
        ureq::Error::BadUri(_) | ureq::Error::Http(_) | ureq::Error::BodyExceedsLimit(_) => {
            NoAnswer::Failed
        }
        // Everything else is the network: no connection, no name, no trust in the
        // certificate, no answer in time, an answer that is not HTTP or stops half way.
        _ => NoAnswer::Unreachable,
    }
}
