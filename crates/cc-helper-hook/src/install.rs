use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

const SETTINGS_PATH: &str = ".claude/settings.json";

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
}

fn hook_configs() -> Value {
    json!({
        "PermissionRequest": [
            { "matcher": "", "hooks": [{ "type": "command", "command": "cc-helper-hook permission-request" }] }
        ],
        "PostToolUse": [
            { "matcher": "", "hooks": [{ "type": "command", "command": "cc-helper-hook post-tool-use" }] }
        ],
        "Notification": [
            { "matcher": "", "hooks": [{ "type": "command", "command": "cc-helper-hook notification" }] }
        ],
        "Stop": [
            { "matcher": "", "hooks": [{ "type": "command", "command": "cc-helper-hook stop" }] }
        ],
        "StopFailure": [
            { "matcher": "", "hooks": [{ "type": "command", "command": "cc-helper-hook stop-failure" }] }
        ],
        "SessionStart": [
            { "matcher": "", "hooks": [{ "type": "command", "command": "cc-helper-hook session-start" }] }
        ],
        "SubagentStop": [
            { "matcher": "", "hooks": [{ "type": "command", "command": "cc-helper-hook subagent-stop" }] }
        ]
    })
}

const PILOT_MARKER: &str = "pilot-hook";
const OLD_HELPER_MARKER: &str = "helper-hook";
const HOOK_MARKER: &str = "cc-helper-hook";

pub fn install() -> Result<(), String> {
    let home = home_dir().ok_or("Cannot determine home directory")?;
    let settings_path = home.join(SETTINGS_PATH);
    let hook_configs = hook_configs();

    let mut settings: Value = if settings_path.exists() {
        let content =
            fs::read_to_string(&settings_path).map_err(|e| format!("Cannot read settings: {}", e))?;
        serde_json::from_str(&content).unwrap_or(json!({}))
    } else {
        json!({})
    };

    if settings.get("hooks").is_none() {
        settings["hooks"] = json!({});
    }

    for (event_name, new_entries) in hook_configs.as_object().unwrap() {
        if settings["hooks"].get(event_name).is_none() {
            settings["hooks"][event_name] = json!([]);
        }
        let existing = settings["hooks"][event_name].as_array_mut().unwrap();
        let has_hook = existing.iter().any(|entry| {
            entry
                .get("hooks")
                .and_then(|h| h.as_array())
                .map_or(false, |arr| {
                    arr.iter().any(|h| {
                        h.get("command")
                            .and_then(|c| c.as_str())
                            .map_or(false, |c| c.contains(HOOK_MARKER) || c.contains(OLD_HELPER_MARKER) || c.contains(PILOT_MARKER))
                    })
                })
        });
        if !has_hook {
            existing.push(new_entries.as_array().unwrap()[0].clone());
        }
    }

    let output =
        serde_json::to_string_pretty(&settings).map_err(|e| format!("Cannot serialize: {}", e))?;
    fs::write(&settings_path, output).map_err(|e| format!("Cannot write: {}", e))?;
    println!("cc-helper hooks installed to {}", settings_path.display());
    Ok(())
}

pub fn uninstall() -> Result<(), String> {
    let home = home_dir().ok_or("Cannot determine home directory")?;
    let settings_path = home.join(SETTINGS_PATH);
    if !settings_path.exists() {
        println!("No settings file found.");
        return Ok(());
    }

    let content = fs::read_to_string(&settings_path).map_err(|e| format!("Cannot read: {}", e))?;
    let mut settings: Value = serde_json::from_str(&content).unwrap_or(json!({}));

    if let Some(hooks) = settings.get_mut("hooks").and_then(|h| h.as_object_mut()) {
        for (_, entries) in hooks.iter_mut() {
            if let Some(arr) = entries.as_array_mut() {
                arr.retain(|entry| {
                    entry
                        .get("hooks")
                        .and_then(|h| h.as_array())
                        .map_or(true, |h| {
                            !h.iter().any(|hook| {
                                hook.get("command")
                                    .and_then(|c| c.as_str())
                                    .map_or(false, |c| c.contains(HOOK_MARKER) || c.contains(OLD_HELPER_MARKER) || c.contains(PILOT_MARKER))
                            })
                        })
                });
            }
        }
    }

    let output =
        serde_json::to_string_pretty(&settings).map_err(|e| format!("Cannot serialize: {}", e))?;
    fs::write(&settings_path, output).map_err(|e| format!("Cannot write: {}", e))?;
    println!(
        "cc-helper hooks removed from {}",
        settings_path.display()
    );
    Ok(())
}
