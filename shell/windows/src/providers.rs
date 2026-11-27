//! Makes Core's two providers for the app, with the places they read and the one sender that
//! reaches the network. Nothing else in the shell asks the network for anything. Tokens are read
//! by the providers alone, for one request each, and are in no line this shell writes.
use crate::child::{self, CREATE_NO_WINDOW};
use crate::diagnostics::Diagnostics;
use crate::win::job::Job;
use crate::win::{credentials, known_folders};
use sobreiro_core::app_paths::{AppPaths, DATA_DIR_VARIABLE};
use sobreiro_core::connect::claude_paths::{
    CONFIG_DIR_VARIABLE, ClaudePaths, USER_PROFILE_VARIABLE,
};
use sobreiro_core::copilot::copilot_cache;
use sobreiro_core::copilot::copilot_provider::CopilotProvider;
use sobreiro_core::copilot::copilot_sign_in::{
    CopilotSignIn, CopilotSourceKind, SignInFailed, TokenSearch,
};
use sobreiro_core::copilot::copilot_token;
use sobreiro_core::copilot::copilot_usage_client::CopilotUsageClient;
use sobreiro_core::copilot::gh_cli;
use sobreiro_core::io::http::Sender;
use sobreiro_core::live::claude_provider::ClaudeProvider;
use sobreiro_core::live::live_usage_client::{LiveUsageClient, network_sender};
use sobreiro_core::polling::Trace;
use std::io::Read;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

/// Where Sobreiro's files and Claude Code's are.
pub struct Locations {
    pub app: AppPaths,
    pub claude: ClaudePaths,
    local_app_data: PathBuf,
    user_profile: PathBuf,
}

impl Locations {
    /// The folders, as Windows and the environment name them. None when Windows names none.
    pub fn find() -> Option<Locations> {
        let local_app_data = known_folders::local_app_data()?;
        let user_profile = known_folders::user_profile()?;
        let app = AppPaths::new(
            std::env::var_os(DATA_DIR_VARIABLE).as_deref(),
            &local_app_data,
        );
        let claude = ClaudePaths::new(
            std::env::var_os(CONFIG_DIR_VARIABLE).as_deref(),
            std::env::var_os(USER_PROFILE_VARIABLE).as_deref(),
            &user_profile,
        );
        Some(Locations {
            app,
            claude,
            local_app_data,
            user_profile,
        })
    }
}

/// Claude Code's and Copilot's providers. The Copilot one is also kept as itself, for the read of
/// its cache ahead of the first poll.
pub struct Providers {
    pub claude: Arc<ClaudeProvider>,
    pub copilot: Arc<CopilotProvider>,
}

/// Sobreiro's name and version, sent as the `User-Agent`.
fn user_agent() -> String {
    format!("Sobreiro/{}", env!("CARGO_PKG_VERSION"))
}

pub fn make(places: &Locations, diagnostics: &Arc<Diagnostics>) -> Providers {
    let sender: Arc<dyn Sender> = Arc::new(network_sender());
    let trace: Trace = {
        let diagnostics = diagnostics.clone();
        Arc::new(move |line: &str| diagnostics.say(line))
    };
    let claude = ClaudeProvider::new(
        LiveUsageClient::new(sender.clone(), &user_agent()),
        &places.claude.credentials_file(),
        &places.app.usage_file(),
        Some(trace.clone()),
    );
    let copilot = CopilotProvider::new(
        CopilotUsageClient::new(sender, &user_agent()),
        sign_ins(places),
        &copilot_cache::default_path(|name| std::env::var(name).ok(), &places.local_app_data),
        trace,
    );
    Providers {
        claude: Arc::new(claude),
        copilot: Arc::new(copilot),
    }
}

/// The three places a GitHub token is looked for, in order.
fn sign_ins(places: &Locations) -> Vec<CopilotSignIn> {
    let local_app_data = places.local_app_data.clone();
    let user_profile = places.user_profile.clone();
    vec![
        CopilotSignIn {
            kind: CopilotSourceKind::Env,
            tokens: Box::new(|| {
                Ok(TokenSearch {
                    tokens: copilot_token::from_environment(|name| std::env::var(name).ok()),
                    entry_found: false,
                })
            }),
        },
        CopilotSignIn {
            kind: CopilotSourceKind::CredentialManager,
            tokens: Box::new(move || {
                credentials::copilot_tokens(cli_has_been_used(&local_app_data, &user_profile))
            }),
        },
        CopilotSignIn {
            kind: CopilotSourceKind::Gh,
            tokens: Box::new(gh_tokens),
        },
    ]
}

/// Whether the Copilot CLI has left its folder on this PC.
fn cli_has_been_used(local_app_data: &Path, user_profile: &Path) -> bool {
    local_app_data.join("copilot").is_dir() || user_profile.join(".copilot").is_dir()
}

/// `gh auth token`, when `gh` is on the PATH. A run that fails or takes longer than Core's limit
/// gives no token.
fn gh_tokens() -> Result<TokenSearch, SignInFailed> {
    let none = || TokenSearch {
        tokens: Vec::new(),
        entry_found: false,
    };
    let path = std::env::var("PATH").ok();
    let Some(exe) = gh_cli::find(path.as_deref()) else {
        return Ok(none());
    };
    let Some((code, mut output)) = run_gh(&exe) else {
        return Ok(none());
    };
    let tokens = gh_cli::tokens_from_run(code, &output);
    output.fill(0);
    std::hint::black_box(&output);
    Ok(TokenSearch {
        tokens,
        entry_found: false,
    })
}

/// The most of `gh`'s output that is read: a token is far shorter.
const OUTPUT_CAP: usize = 16 * 1024;

/// Runs `gh` and gives its exit code and what it printed. None when it cannot be started or does
/// not end and answer within Core's limit. `gh` runs in a job object, so that on a timeout, and
/// when this returns, it and anything it started are ended, and no one keeps the pipe open.
fn run_gh(exe: &Path) -> Option<(Option<i32>, Vec<u8>)> {
    let mut child = Command::new(exe)
        .args(gh_cli::ARGUMENTS)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .ok()?;
    let job = Job::kill_on_close();
    if let Some(job) = &job {
        let _ = job.assign(&child);
    }
    let mut stdout = child.stdout.take()?;
    let (sender, receiver) = mpsc::channel::<Vec<u8>>();
    let reader = std::thread::Builder::new()
        .name("gh-output".to_string())
        .spawn(move || {
            // Sized once and never grown: what is read is at most the cap.
            let mut buffer = vec![0u8; OUTPUT_CAP];
            let mut length = 0;
            while length < OUTPUT_CAP {
                match stdout.read(&mut buffer[length..]) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => length += read,
                }
            }
            buffer.truncate(length);
            // Nobody is waiting any more: the bytes are zeroed and dropped here.
            if let Err(mpsc::SendError(mut late)) = sender.send(buffer) {
                late.fill(0);
            }
        });
    let deadline = Instant::now() + gh_cli::TIMEOUT;
    let status = reader
        .ok()
        .and_then(|_| child::wait_until(&mut child, deadline, Duration::from_millis(25)));
    let output = status.and_then(|status| {
        let left = deadline.saturating_duration_since(Instant::now());
        receiver
            .recv_timeout(left)
            .ok()
            .map(|bytes| (status.code(), bytes))
    });
    // Ended in every case: a finished `gh` has nothing left to end, and one that did not answer
    // is stopped with what it started.
    let _ = child.kill();
    drop(job);
    let _ = child.wait();
    output
}
