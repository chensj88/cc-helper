use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

const SETTINGS_PATH: &str = ".claude/settings.json";
const PILOT_MARKER: &str = "pilot-hook";
const OLD_HELPER_MARKER: &str = "helper-hook";
const HOOK_MARKER: &str = "cc-helper-hook";
const WRAPPER_DIR: &str = ".claude/bin";
const WRAPPER_NAME: &str = "cc-helper-hook";

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
}

fn helper_hooks(wrapper_path: &str) -> Value {
    // No quoting needed — wrapper path has no spaces
    json!({
        "PermissionRequest": [
            { "matcher": "", "hooks": [{ "type": "command", "command": format!("{wrapper_path} permission-request") }] }
        ],
        "PostToolUse": [
            { "matcher": "", "hooks": [{ "type": "command", "command": format!("{wrapper_path} post-tool-use") }] }
        ],
        "Notification": [
            { "matcher": "", "hooks": [{ "type": "command", "command": format!("{wrapper_path} notification") }] }
        ],
        "Stop": [
            { "matcher": "", "hooks": [{ "type": "command", "command": format!("{wrapper_path} stop") }] }
        ],
        "StopFailure": [
            { "matcher": "", "hooks": [{ "type": "command", "command": format!("{wrapper_path} stop-failure") }] }
        ],
        "SessionStart": [
            { "matcher": "", "hooks": [{ "type": "command", "command": format!("{wrapper_path} session-start") }] }
        ],
        "SubagentStop": [
            { "matcher": "", "hooks": [{ "type": "command", "command": format!("{wrapper_path} subagent-stop") }] }
        ]
    })
}

/// Create a wrapper script at ~/.claude/bin/cc-helper-hook that preserves stdin
/// This avoids quoting issues with paths containing spaces — Claude Code does not
/// use shell execution for hook commands, so single/double quotes in the command
/// string are treated as literal characters and break the path.
fn create_wrapper(hook_path: &str) -> Result<String, String> {
    let home = home_dir().ok_or("Cannot determine home directory")?;
    let wrapper_dir = home.join(WRAPPER_DIR);
    fs::create_dir_all(&wrapper_dir).map_err(|e| format!("Cannot create bin dir: {}", e))?;

    let wrapper_path = wrapper_dir.join(WRAPPER_NAME);
    let wrapper_content = if cfg!(target_os = "windows") {
        format!("@echo off\n\"{}\" %*\n", hook_path)
    } else {
        format!("#!/bin/bash\nstdin_file=\"/tmp/cc-helper-stdin-$.txt\"\ncat > \"$stdin_file\"\n'{}' \"$@\" < \"$stdin_file\"\nrm -f \"$stdin_file\"\n", hook_path)
    };

    fs::write(&wrapper_path, &wrapper_content).map_err(|e| format!("Cannot write wrapper: {}", e))?;

    // Make executable (Unix only)
    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&wrapper_path, fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Cannot set permissions: {}", e))?;
    }

    Ok(wrapper_path
        .to_str()
        .ok_or("Invalid wrapper path".to_string())
        .map(|s| s.to_string())?)
}

pub fn find_hook_binary() -> String {
    let exe_dir = std::env::current_exe()
        .expect("Cannot find executable path")
        .parent()
        .expect("Cannot determine executable directory")
        .to_path_buf();

    let hook_name = if cfg!(target_os = "windows") {
        "cc-helper-hook.exe"
    } else {
        "cc-helper-hook"
    };

    exe_dir
        .join(hook_name)
        .to_str()
        .expect("Invalid path")
        .to_string()
}

pub fn install_with_path(hook_path: &str) -> Result<(), String> {
    // Verify cc-helper-hook exists
    if !std::path::Path::new(hook_path).exists() {
        return Err(format!(
            "cc-helper-hook not found at: {}\nMake sure you are running from the installed application.",
            hook_path
        ));
    }

    // Create wrapper script first (avoids quoting issues with paths containing spaces)
    let wrapper_path = create_wrapper(hook_path)?;
    let helper_hooks = helper_hooks(&wrapper_path);

    let home = home_dir().ok_or("Cannot determine home directory")?;
    let settings_path = home.join(SETTINGS_PATH);

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

    for (event_name, new_entries) in helper_hooks.as_object().unwrap() {
        if settings["hooks"].get(event_name).is_none() {
            settings["hooks"][event_name] = json!([]);
        }
        let existing = settings["hooks"][event_name].as_array_mut().unwrap();
        // Remove any old cc-helper-hook, helper-hook, or pilot-hook entries (handles path changes and renames)
        existing.retain(|entry| {
            entry
                .get("hooks")
                .and_then(|h| h.as_array())
                .map_or(true, |arr| {
                    !arr.iter().any(|h| {
                        h.get("command")
                            .and_then(|c| c.as_str())
                            .map_or(false, |c| c.contains(HOOK_MARKER) || c.contains(OLD_HELPER_MARKER) || c.contains(PILOT_MARKER))
                    })
                })
        });
        existing.push(new_entries.as_array().unwrap()[0].clone());
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
        return Ok(())
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

    // Also remove the wrapper script
    let wrapper_path = home.join(WRAPPER_DIR).join(WRAPPER_NAME);
    if wrapper_path.exists() {
        fs::remove_file(&wrapper_path).map_err(|e| format!("Cannot remove wrapper: {}", e))?;
        println!("Wrapper script removed from {}", wrapper_path.display());
    }

    Ok(())
}
