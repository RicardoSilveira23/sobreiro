use super::{CopilotReading, CopilotSource, CopilotUserJson};
use chrono::DateTime;
use serde_json::Value;
use std::path::PathBuf;

/// Reads the Copilot CLI's own cache, %LOCALAPPDATA%\copilot\copilot-user-cache.json: a few "// ..." comment lines,
/// then {"copilotUserCache":{"v1:...":{"retrievedAt":"...","response":{...}}}}. Of the entries that hold a budget
/// (a free seat's entry does not), the one retrieved last wins. Never fails.
pub struct CopilotCache;

impl CopilotCache {
    pub fn default_path(get_env: &dyn Fn(&str) -> Option<String>) -> String {
        let local = get_env("LOCALAPPDATA").filter(|v| !v.is_empty()).unwrap_or_else(|| {
            // Environment.GetFolderPath(LocalApplicationData) without LOCALAPPDATA: under the profile.
            PathBuf::from(get_env("USERPROFILE").unwrap_or_default()).join("AppData").join("Local").to_string_lossy().into_owned()
        });
        PathBuf::from(local).join("copilot").join("copilot-user-cache.json").to_string_lossy().into_owned()
    }

    pub fn parse(text: &str) -> Option<CopilotReading> {
        let json: Vec<&str> = text.split('\n').filter(|line| !line.trim_start().starts_with("//")).collect();
        let doc: Value = serde_json::from_str(&json.join("\n")).ok()?;
        let entries = doc.as_object()?.get("copilotUserCache")?.as_object()?;

        let mut best: Option<(CopilotReading, crate::Time)> = None;
        for (_, entry) in entries {
            let Some(entry) = entry.as_object() else { continue };
            let Some(Value::String(at)) = entry.get("retrievedAt") else { continue };
            let Ok(retrieved_at) = DateTime::parse_from_rfc3339(at) else { continue };
            let Some(response) = entry.get("response") else { continue };
            let Some(reading) = CopilotUserJson::from_response(response, retrieved_at, CopilotSource::Cache) else { continue };
            if best.as_ref().is_none_or(|(_, best_at)| retrieved_at > *best_at) {
                best = Some((reading, retrieved_at));
            }
        }
        best.map(|(r, _)| r)
    }

    pub fn read(path: &str) -> Option<CopilotReading> {
        let bytes = std::fs::read(path).ok()?;
        Self::parse(&String::from_utf8_lossy(bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes)))
    }
}
