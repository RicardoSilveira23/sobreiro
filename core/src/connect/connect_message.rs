//! The words of the Connect and Disconnect flow: the menu item, the box that asks and the box
//! after.
use std::path::Path;

use super::bridge_command::is_legacy_bridge;
use super::connection::ConnectionPlan;
use super::settings_json::command_of;

pub const RECONNECT_TITLE: &str = "Reconnect to Claude Code";

pub fn title(connect: bool) -> &'static str {
    if connect {
        "Connect to Claude Code"
    } else {
        "Disconnect from Claude Code"
    }
}

/// What the box that asks says: the file, the `statusLine` before and after, whether the old
/// status line keeps running, and the backup (only when there is a file to back up).
pub fn confirm(plan: &ConnectionPlan) -> String {
    let now = plan.before.as_deref().map_or("not set", str::trim);
    let after = plan.after.as_deref().map_or("removed", str::trim);
    let mut lines = vec![
        plan.settings.path.display().to_string(),
        String::new(),
        format!("statusLine now: {now}"),
        format!("statusLine after: {after}"),
    ];
    // Each further paragraph is an empty line and the paragraph.
    let mut paragraph = |text: &str| {
        lines.push(String::new());
        lines.push(text.to_string());
    };
    if plan.reconnects {
        paragraph(
            if is_legacy_bridge(command_of(plan.before.as_deref()).as_deref()) {
                "The status line still runs the old OpenHeadroom bridge. This points it at Sobreiro's."
            } else {
                "The status line runs a Sobreiro bridge from another folder. This points it at the one this Sobreiro uses."
            },
        );
    }
    if plan.chains {
        paragraph("Your current status line keeps running: the bridge passes its output through.");
    }
    if plan.settings.exists {
        paragraph("A backup is saved first.");
    }
    lines.join("\n")
}

/// What the box after says: what was done, and the backup's full path when there is one.
pub fn done(connect: bool, backup_path: Option<&Path>, reconnect: bool) -> String {
    let done = if reconnect {
        "Reconnected. The numbers arrive with Claude Code's next response."
    } else if connect {
        "Connected. The numbers arrive with Claude Code's next response."
    } else {
        "Disconnected."
    };
    match backup_path {
        Some(backup) => format!("{done}\n\nBackup: {}", backup.display()),
        None => done.to_string(),
    }
}
