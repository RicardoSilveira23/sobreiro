//! The step Sobreiro takes at every start: bring the bridge in the data folder up to date.
use std::io::{self, Read};
use std::path::Path;

use super::bridge_install::{ensure, should_refresh};
use super::connection::read_settings;

/// What the start's step did. Each outcome has one word for the diagnostics log, so the log
/// never holds a path or the text of an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeRefresh {
    /// This exe does not carry the bridge, so there is nothing to write.
    NotCarried,
    /// The status line does not run the data folder's copy, so it is left alone.
    NotUsed,
    /// The copy was out of date or gone, and has been written.
    Written,
    /// The copy already held the same bytes.
    Current,
    /// The copy could not be checked or written.
    Failed,
}

impl BridgeRefresh {
    /// The word for the log.
    pub fn word(self) -> &'static str {
        match self {
            BridgeRefresh::NotCarried => "not-carried",
            BridgeRefresh::NotUsed => "not-used",
            BridgeRefresh::Written => "written",
            BridgeRefresh::Current => "current",
            BridgeRefresh::Failed => "failed",
        }
    }
}

/// Brings the bridge at `installed_path` up to date, when `settings.json` runs it.
///
/// `open_carried` opens the bridge the exe carries (see `bridge_install::ensure`), or is `None`
/// when it carries none. A status line that runs something else, or no `settings.json` at all,
/// means the copy is not in use: nothing is written, and a copy that is there is not touched.
/// `settings.json` is only read here, never written.
pub fn refresh<Source: Read>(
    open_carried: Option<impl Fn() -> io::Result<Source>>,
    settings_path: &Path,
    installed_path: &Path,
) -> BridgeRefresh {
    let Some(open_carried) = open_carried else {
        return BridgeRefresh::NotCarried;
    };
    let Ok(settings) = read_settings(settings_path) else {
        return BridgeRefresh::Failed;
    };
    if !settings.exists || !should_refresh(&settings.text, installed_path) {
        return BridgeRefresh::NotUsed;
    }
    match ensure(open_carried, installed_path) {
        Ok(true) => BridgeRefresh::Written,
        Ok(false) => BridgeRefresh::Current,
        Err(_) => BridgeRefresh::Failed,
    }
}
