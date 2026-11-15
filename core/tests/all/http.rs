//! The real sender, held against a server on this PC. No C# test file is behind this one: the
//! C# tests handed their client a stand-in and trusted the framework for the rest.
//!
//! The server listens on 127.0.0.1 only, on a port Windows picks, and is written with the
//! standard library. Nothing here leaves the PC, and every token is made up.
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use sobreiro_core::io::http::{NetworkSender, NoAnswer, Sender};
use sobreiro_core::live::live_usage_client;

/// What the server does with one connection, once it has read the request.
enum Reply {
    /// Sends these bytes and closes.
    Text(Vec<u8>),
    /// Says nothing, and keeps the connection open until the test is over.
    Silence,
    /// Sends these bytes, then says no more and keeps the connection open.
    TextThenSilence(Vec<u8>),
}

/// An answer with a status line, header lines and a body, as HTTP writes one.
fn reply(status: &str, headers: &[&str], body: &str) -> Reply {
    let mut text = format!("HTTP/1.1 {status}\r\n");
    for header in headers {
        text += header;
        text += "\r\n";
    }
    text += &format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    Reply::Text(text.into_bytes())
}

/// A server for one test. It gives each connection the next reply of its list, and keeps
/// every request it read, as text.
struct LocalServer {
    address: String,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl LocalServer {
    fn start(replies: Vec<Reply>) -> LocalServer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));

        let (kept, stopping) = (requests.clone(), stop.clone());
        let thread = std::thread::spawn(move || {
            // Connections that got no answer, held open until the test is over.
            let mut silent: Vec<TcpStream> = Vec::new();
            for reply in replies {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                if stopping.load(Ordering::SeqCst) {
                    break;
                }
                kept.lock().unwrap().push(read_request(&mut stream));
                match reply {
                    Reply::Text(bytes) => {
                        let _ = stream.write_all(&bytes);
                    }
                    Reply::Silence => silent.push(stream),
                    Reply::TextThenSilence(bytes) => {
                        let _ = stream.write_all(&bytes);
                        silent.push(stream);
                    }
                }
            }
            // Out of replies: wait for the test to end, so that a request nobody expected
            // still finds the port taken and is counted by `requests` staying as it is.
            while !stopping.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(10));
            }
        });

        LocalServer {
            address,
            requests,
            stop,
            thread: Some(thread),
        }
    }

    /// The address of a path on this server.
    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.address)
    }

    /// The requests read so far.
    fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for LocalServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wakes the server if it is still waiting for a connection.
        let _ = TcpStream::connect(self.address.trim_start_matches("http://"));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Reads a request up to the empty line that ends its headers.
fn read_request(stream: &mut TcpStream) -> String {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut request = Vec::new();
    let mut byte = [0u8; 1];
    while !request.ends_with(b"\r\n\r\n") {
        match stream.read(&mut byte) {
            Ok(1) => request.push(byte[0]),
            _ => break,
        }
    }
    String::from_utf8_lossy(&request).into_owned()
}

/// The header lines of a request, in small letters, sorted, without the first line.
fn header_lines(request: &str) -> Vec<String> {
    let mut lines: Vec<String> = request
        .lines()
        .skip(1)
        .filter(|line| !line.is_empty())
        .map(str::to_lowercase)
        .collect();
    lines.sort();
    lines
}

fn sender() -> NetworkSender {
    NetworkSender::new(Duration::from_secs(5))
}

#[test]
fn a_request_goes_out_as_it_was_asked_and_the_answer_comes_back() {
    let server = LocalServer::start(vec![reply("200 OK", &[], "{\"a\":1}")]);
    let host = server.address.trim_start_matches("http://").to_string();

    let answer = sender()
        .get(
            &server.url("/api/oauth/usage"),
            &[
                ("Authorization", "Bearer made-up"),
                ("anthropic-beta", "oauth-2025-04-20"),
                ("Accept", "application/json"),
                ("User-Agent", "Sobreiro/0.3.0"),
            ],
        )
        .ok()
        .unwrap();

    assert_eq!(answer.status, 200);
    assert_eq!(answer.body, b"{\"a\":1}");
    assert_eq!(answer.retry_after, None);

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert!(
        requests[0].starts_with("GET /api/oauth/usage HTTP/1.1\r\n"),
        "{}",
        requests[0]
    );
    // Every header on the wire: the four that were asked for and the one HTTP needs. Nothing
    // is added: no cookie, no name of the library, no word about the PC.
    assert_eq!(
        header_lines(&requests[0]),
        [
            "accept: application/json".to_string(),
            "anthropic-beta: oauth-2025-04-20".to_string(),
            "authorization: bearer made-up".to_string(),
            format!("host: {host}"),
            "user-agent: sobreiro/0.3.0".to_string(),
        ]
    );
}

#[test]
fn a_request_with_no_headers_goes_out_with_none_but_the_one_http_needs() {
    let server = LocalServer::start(vec![reply("200 OK", &[], "")]);
    let host = server.address.trim_start_matches("http://").to_string();

    sender().get(&server.url("/usage"), &[]).ok().unwrap();

    // The library would otherwise name itself and say what kinds of answer it takes.
    assert_eq!(
        header_lines(&server.requests()[0]),
        [format!("host: {host}")]
    );
}

#[test]
fn a_redirect_comes_back_as_the_answer_and_is_not_followed() {
    for status in [
        "301 Moved Permanently",
        "302 Found",
        "303 See Other",
        "307 Temporary Redirect",
        "308 Permanent Redirect",
    ] {
        let server = LocalServer::start(vec![
            reply(status, &["Location: /steal"], ""),
            // What a sender that followed the redirect would be given.
            reply("200 OK", &[], "stolen"),
        ]);

        let answer = sender()
            .get(
                &server.url("/usage"),
                &[("Authorization", "Bearer made-up")],
            )
            .ok()
            .unwrap();

        let code: u16 = status[..3].parse().unwrap();
        assert_eq!(answer.status, code);
        assert_eq!(answer.body, b"");
        // The server was asked once, for the first address, and never for the second.
        let requests = server.requests();
        assert_eq!(requests.len(), 1, "{status}");
        assert!(requests[0].starts_with("GET /usage "), "{}", requests[0]);
    }
}

#[test]
fn a_redirect_to_another_server_is_not_followed_either() {
    let other = LocalServer::start(vec![reply("200 OK", &[], "stolen")]);
    let location = format!("Location: {}", other.url("/steal"));
    let server = LocalServer::start(vec![reply("302 Found", &[&location], "")]);

    let answer = sender()
        .get(
            &server.url("/usage"),
            &[("Authorization", "Bearer made-up")],
        )
        .ok()
        .unwrap();

    assert_eq!(answer.status, 302);
    assert_eq!(server.requests().len(), 1);
    // The token went nowhere else.
    assert_eq!(other.requests(), Vec::<String>::new());
}

#[test]
fn no_cookie_is_kept_from_one_request_to_the_next() {
    let server = LocalServer::start(vec![
        reply("200 OK", &["Set-Cookie: session=abc; Path=/"], "one"),
        reply("200 OK", &[], "two"),
    ]);
    let sender = sender();

    let first = sender.get(&server.url("/usage"), &[]).ok().unwrap();
    let second = sender.get(&server.url("/usage"), &[]).ok().unwrap();

    assert_eq!(first.body, b"one");
    assert_eq!(second.body, b"two");
    let requests = server.requests();
    assert_eq!(requests.len(), 2);
    assert!(
        !requests[1].to_lowercase().contains("cookie"),
        "{}",
        requests[1]
    );
}

#[test]
fn a_status_of_400_or_more_is_an_answer_and_its_body_is_not_read() {
    for status in [
        "401 Unauthorized",
        "403 Forbidden",
        "404 Not Found",
        "500 Internal Server Error",
    ] {
        let server = LocalServer::start(vec![reply(status, &[], "no")]);

        let answer = sender().get(&server.url("/usage"), &[]).ok().unwrap();

        let code: u16 = status[..3].parse().unwrap();
        assert_eq!(answer.status, code);
        assert_eq!(answer.body, b"", "{status}");
    }
}

#[test]
fn retry_after_comes_back_as_the_server_wrote_it() {
    let rows: &[(&[&str], Option<&str>)] = &[
        (&["Retry-After: 120"], Some("120")),
        (&["retry-after: 120"], Some("120")),
        (
            &["Retry-After: Mon, 09 Nov 2026 12:03:00 GMT"],
            Some("Mon, 09 Nov 2026 12:03:00 GMT"),
        ),
        // Written twice: the first one, as in the C#.
        (&["Retry-After: 120", "Retry-After: 130"], Some("120")),
        (&[], None),
    ];
    for (headers, expected) in rows {
        let server = LocalServer::start(vec![reply("429 Too Many Requests", headers, "")]);

        let answer = sender().get(&server.url("/usage"), &[]).ok().unwrap();

        assert_eq!(answer.status, 429);
        assert_eq!(answer.retry_after.as_deref(), *expected, "{headers:?}");
    }
}

#[test]
fn an_answer_slower_than_the_timeout_is_unreachable() {
    let server = LocalServer::start(vec![Reply::Silence]);
    let impatient = NetworkSender::new(Duration::from_millis(300));
    let started = Instant::now();

    let answer = impatient.get(&server.url("/usage"), &[]);

    assert_eq!(answer.err(), Some(NoAnswer::Unreachable));
    assert!(started.elapsed() < Duration::from_secs(4));
    assert_eq!(server.requests().len(), 1);
}

#[test]
fn a_body_that_stalls_is_unreachable_within_the_timeout() {
    // The headers come, with five of the hundred bytes they promise, and then nothing: the
    // time given to a request covers its body too.
    let start = b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nhello".to_vec();
    let server = LocalServer::start(vec![Reply::TextThenSilence(start)]);
    let impatient = NetworkSender::new(Duration::from_millis(300));
    let started = Instant::now();

    let answer = impatient.get(&server.url("/usage"), &[]);

    assert_eq!(answer.err(), Some(NoAnswer::Unreachable));
    assert!(started.elapsed() < Duration::from_secs(4));
}

#[test]
fn an_answer_that_stops_half_way_is_unreachable() {
    // The headers promise a hundred bytes; five come, and then nothing.
    let half = b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nhello".to_vec();
    let server = LocalServer::start(vec![Reply::Text(half)]);

    let answer = sender().get(&server.url("/usage"), &[]);

    assert_eq!(answer.err(), Some(NoAnswer::Unreachable));
}

#[test]
fn nobody_listening_is_unreachable() {
    // A port that was free a moment ago: taken, then given back.
    let port = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };

    let answer = sender().get(&format!("http://127.0.0.1:{port}/usage"), &[]);

    assert_eq!(answer.err(), Some(NoAnswer::Unreachable));
}

#[test]
fn an_answer_that_is_not_http_is_unreachable() {
    let server = LocalServer::start(vec![Reply::Text(b"hello there\r\n\r\n".to_vec())]);

    let answer = sender().get(&server.url("/usage"), &[]);

    assert_eq!(answer.err(), Some(NoAnswer::Unreachable));
}

#[test]
fn a_body_of_a_megabyte_is_read_and_one_byte_more_is_failed() {
    let megabyte = "x".repeat(1 << 20);
    let server = LocalServer::start(vec![reply("200 OK", &[], &megabyte)]);
    let answer = sender().get(&server.url("/usage"), &[]).ok().unwrap();
    assert_eq!(answer.body.len(), 1 << 20);

    let more = megabyte + "x";
    let server = LocalServer::start(vec![reply("200 OK", &[], &more)]);
    let answer = sender().get(&server.url("/usage"), &[]);
    assert_eq!(answer.err(), Some(NoAnswer::Failed));
}

#[test]
fn an_address_or_a_header_that_cannot_be_is_failed_and_nothing_is_sent() {
    let server = LocalServer::start(vec![reply("200 OK", &[], "")]);

    let answer = sender().get("not an address", &[]);
    assert_eq!(answer.err(), Some(NoAnswer::Failed));

    let answer = sender().get(&server.url("/usage"), &[("Authorization", "a\nb")]);
    assert_eq!(answer.err(), Some(NoAnswer::Failed));

    assert_eq!(server.requests(), Vec::<String>::new());
}

#[test]
fn the_app_s_sender_waits_ten_seconds_and_no_more() {
    // `an_answer_slower_than_the_timeout_is_unreachable` shows that a sender keeps to the
    // time it was given. This is the time the app's own sender is given.
    assert_eq!(
        live_usage_client::network_sender().timeout(),
        Duration::from_secs(10)
    );
}

#[test]
fn plain_http_is_spoken_to_this_pc_and_to_nowhere_else() {
    // The tests above reach their server as `http://127.0.0.1`. The same server under its
    // name, `localhost`, is another address to the sender, and is refused before anything is
    // sent: outside this PC a request is HTTPS or it is not made.
    let server = LocalServer::start(vec![reply("200 OK", &[], "")]);
    let port = server.address.rsplit(':').next().unwrap().to_string();
    for url in [
        format!("http://localhost:{port}/usage"),
        // An address made to look as if it began with this PC's.
        format!("http://127.0.0.1:1@localhost:{port}/usage"),
        format!("ftp://127.0.0.1:{port}/usage"),
        format!("//127.0.0.1:{port}/usage"),
    ] {
        let answer = sender().get(&url, &[("Authorization", "Bearer made-up")]);

        assert_eq!(answer.err(), Some(NoAnswer::Failed), "{url}");
    }
    assert_eq!(server.requests(), Vec::<String>::new());
}

/// Keeps what the libraries of the build hand to a logger, at every level.
struct KeptLines(Mutex<Vec<String>>);

impl log::Log for KeptLines {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        // A test that fails while it holds the list must not take the other tests with it.
        let mut lines = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        lines.push(record.args().to_string());
    }

    fn flush(&self) {}
}

static KEPT_LINES: KeptLines = KeptLines(Mutex::new(Vec::new()));

#[test]
fn a_logger_set_to_hear_everything_is_not_told_the_token() {
    // Sobreiro installs no logger. Should one ever be installed, by the app or by a library,
    // this is what it would hear at its most detailed level. The HTTP library can write a
    // request out byte by byte, sixteen to a line, so the made-up token is one pair of letters
    // over and over: any sixteen bytes of it would show.
    let token = "ZQ".repeat(40);
    log::set_logger(&KEPT_LINES).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
    let server = LocalServer::start(vec![reply("200 OK", &[], "{}")]);

    let answer = sender()
        .get(
            &server.url("/usage"),
            &[("Authorization", &format!("Bearer {token}"))],
        )
        .ok()
        .unwrap();

    assert_eq!(answer.status, 200);
    // The request did go out with the token in it.
    assert!(server.requests()[0].contains(&token));
    let kept = KEPT_LINES.0.lock().unwrap().clone();
    let told: Vec<&String> = kept.iter().filter(|line| line.contains("ZQZQ")).collect();
    assert!(
        told.is_empty(),
        "{} lines, the first: {}",
        told.len(),
        told[0]
    );
}

/// Every `.rs` file under a folder, and under the folders inside it.
fn rust_files(folder: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    files
}

#[test]
fn one_file_of_the_workspace_reaches_the_network_and_no_other() {
    // The rule is "no other network calls" than the two Sobreiro is known for. Both go
    // through the sender in `core/src/io/http.rs`. So that file, and no other file of Core,
    // of the bridge or of the shell, may name the HTTP library or the network types of the
    // standard library. A new place that reaches the network fails here until someone has
    // looked at it.
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let names = ["ureq", "std::net", "TcpStream", "TcpListener", "UdpSocket"];
    let mut found = Vec::new();
    for folder in ["core/src", "bridge/src", "shell/windows/src"] {
        for file in rust_files(&workspace.join(folder)) {
            let text = std::fs::read_to_string(&file).unwrap();
            if names.iter().any(|name| text.contains(name)) {
                let inside = file.strip_prefix(workspace).unwrap();
                found.push(
                    inside
                        .to_string_lossy()
                        .replace(std::path::MAIN_SEPARATOR, "/"),
                );
            }
        }
    }
    assert_eq!(found, ["core/src/io/http.rs"]);

    // And only Core is built with the HTTP library.
    for manifest in ["bridge/Cargo.toml", "shell/windows/Cargo.toml"] {
        let text = std::fs::read_to_string(workspace.join(manifest)).unwrap();
        assert!(!text.contains("ureq"), "{manifest}");
    }
}

#[test]
fn two_addresses_are_written_in_the_code_and_no_other() {
    // Each token goes to one address, and each address is a constant of its client. So no
    // line of code in Core, in the bridge or in the shell may hold another address. A comment
    // may. A new address fails here until someone has looked at it.
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut found = Vec::new();
    for folder in ["core/src", "bridge/src", "shell/windows/src"] {
        for file in rust_files(&workspace.join(folder)) {
            let text = std::fs::read_to_string(&file).unwrap();
            for line in text.lines().map(str::trim_start) {
                let is_comment = line.starts_with("//");
                if !is_comment && (line.contains("http://") || line.contains("https://")) {
                    found.push(line.to_string());
                }
            }
        }
    }
    found.sort();
    assert_eq!(
        found,
        [
            "pub const ENDPOINT: &str = \"https://api.anthropic.com/api/oauth/usage\";",
            "pub const ENDPOINT: &str = \"https://api.github.com/copilot_internal/user\";",
        ]
    );
}
