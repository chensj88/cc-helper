use helper_protocol::HookEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

static EVENT_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SessionStatus {
    Working,
    WaitingPermission,
    Idle,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub session_id: String,
    pub cwd: String,
    pub project_name: String,
    pub status: SessionStatus,
    pub last_event: String,
    pub timestamp: u64,
}

pub struct SessionManager {
    sessions: Mutex<HashMap<String, Session>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn handle_event(&self, event: &HookEvent, session_id: &str, cwd: &str) {
        if !Self::has_valid_session_identity(session_id, cwd)
            && !matches!(
                event,
                HookEvent::Stop
                    | HookEvent::SubagentStop
                    | HookEvent::StopFailure
                    | HookEvent::PostToolUse
                    | HookEvent::PreToolUse
            )
        {
            return;
        }

        let mut sessions = self.sessions.lock().unwrap();
        let key = Self::session_key(session_id, cwd);
        let existing_key = if sessions.contains_key(&key) {
            Some(key.clone())
        } else {
            Self::find_existing_key(&sessions, session_id)
        };
        let project_name = Self::project_name(cwd);

        let event_name = format!("{:?}", event);
        let now = Self::now_millis();

        match event {
            HookEvent::SessionStart => {
                sessions.insert(
                    key,
                    Session {
                        session_id: session_id.to_string(),
                        cwd: cwd.to_string(),
                        project_name,
                        status: SessionStatus::Working,
                        last_event: event_name,
                        timestamp: now,
                    },
                );
            }
            HookEvent::Stop | HookEvent::SubagentStop => {
                // Task finished but Claude Code may still be running
                // Keep session visible with Idle status
                if let Some(existing_key) = existing_key {
                    let s = sessions.get_mut(&existing_key).unwrap();
                    s.status = SessionStatus::Idle;
                    s.last_event = event_name;
                    s.timestamp = now;
                }
            }
            HookEvent::StopFailure => {
                if let Some(existing_key) = existing_key {
                    let s = sessions.get_mut(&existing_key).unwrap();
                    s.status = SessionStatus::Failed;
                    s.last_event = event_name;
                    s.timestamp = now;
                }
            }
            HookEvent::PermissionRequest => {
                if let Some(existing_key) = existing_key {
                    let s = sessions.get_mut(&existing_key).unwrap();
                    s.status = SessionStatus::WaitingPermission;
                    s.last_event = event_name;
                    s.timestamp = now;
                } else {
                    // Create session if PermissionRequest arrives before SessionStart.
                    sessions.insert(
                        key,
                        Session {
                            session_id: session_id.to_string(),
                            cwd: cwd.to_string(),
                            project_name,
                            status: SessionStatus::WaitingPermission,
                            last_event: event_name,
                            timestamp: now,
                        },
                    );
                }
            }
            HookEvent::PostToolUse | HookEvent::PreToolUse => {
                if let Some(existing_key) = existing_key {
                    let s = sessions.get_mut(&existing_key).unwrap();
                    s.status = SessionStatus::Working;
                    s.last_event = event_name;
                    s.timestamp = now;
                }
            }
            // Log all other events
            _ => {
                if let Some(existing_key) = existing_key {
                    let s = sessions.get_mut(&existing_key).unwrap();
                    s.last_event = event_name;
                    s.timestamp = now;
                }
            }
        }

        Self::cleanup_sessions(&mut sessions);
    }

    fn has_valid_session_identity(session_id: &str, cwd: &str) -> bool {
        !session_id.trim().is_empty() && session_id != "unknown" && !cwd.trim().is_empty()
    }

    fn normalized_cwd(cwd: &str) -> String {
        let trimmed = cwd.trim();
        if trimmed == "/" {
            return trimmed.to_string();
        }
        trimmed
            .trim_end_matches(std::path::MAIN_SEPARATOR)
            .to_string()
    }

    fn session_key(session_id: &str, cwd: &str) -> String {
        format!("{}\0{}", session_id, Self::normalized_cwd(cwd))
    }

    fn find_existing_key(sessions: &HashMap<String, Session>, session_id: &str) -> Option<String> {
        let mut matches = sessions
            .iter()
            .filter(|(_, s)| s.session_id == session_id)
            .map(|(key, _)| key.clone());
        let first = matches.next()?;
        if matches.next().is_none() {
            Some(first)
        } else {
            None
        }
    }

    fn project_name(cwd: &str) -> String {
        std::path::Path::new(cwd)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| cwd.to_string())
    }

    fn now_millis() -> u64 {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        millis * 1_000 + EVENT_COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    fn cleanup_sessions(sessions: &mut HashMap<String, Session>) {
        const MAX_SESSIONS: usize = 50;
        const IDLE_TIMEOUT_MILLIS: u64 = 24 * 60 * 60 * 1_000; // 24 hours
        const FAILED_TIMEOUT_MILLIS: u64 = 60 * 60 * 1_000; // 1 hour

        let now = Self::now_millis();

        // Remove sessions that are Idle for more than 24 hours or Failed for more than 1 hour
        sessions.retain(|_, s| match s.status {
            SessionStatus::Idle => now - s.timestamp < IDLE_TIMEOUT_MILLIS,
            SessionStatus::Failed => now - s.timestamp < FAILED_TIMEOUT_MILLIS,
            _ => true,
        });

        // If still too many sessions, remove oldest ones
        if sessions.len() > MAX_SESSIONS {
            let mut session_list: Vec<_> = sessions.iter().collect();
            session_list.sort_by_key(|(_, s)| s.timestamp);

            let to_remove = sessions.len() - MAX_SESSIONS;
            let ids_to_remove: Vec<String> = session_list
                .into_iter()
                .take(to_remove)
                .map(|(id, _)| id.to_string())
                .collect();

            for session_id in ids_to_remove {
                sessions.remove(session_id.as_str());
            }
        }
    }

    pub fn get_sessions(&self) -> Vec<Session> {
        let mut sessions: Vec<_> = self.sessions.lock().unwrap().values().cloned().collect();
        sessions.sort_by(|a, b| {
            Self::status_rank(&a.status)
                .cmp(&Self::status_rank(&b.status))
                .then_with(|| b.timestamp.cmp(&a.timestamp))
                .then_with(|| a.project_name.cmp(&b.project_name))
                .then_with(|| a.session_id.cmp(&b.session_id))
        });
        sessions
    }

    pub fn active_session_count(&self) -> usize {
        self.sessions
            .lock()
            .unwrap()
            .values()
            .filter(|s| !matches!(s.status, SessionStatus::Idle))
            .count()
    }

    fn status_rank(status: &SessionStatus) -> u8 {
        match status {
            SessionStatus::WaitingPermission => 0,
            SessionStatus::Working => 1,
            SessionStatus::Failed => 2,
            SessionStatus::Idle => 3,
        }
    }

    pub fn aggregate_status(&self) -> SessionStatus {
        let sessions = self.sessions.lock().unwrap();
        if sessions.is_empty() {
            return SessionStatus::Idle;
        }
        if sessions
            .values()
            .any(|s| matches!(s.status, SessionStatus::WaitingPermission))
        {
            return SessionStatus::WaitingPermission;
        }
        if sessions
            .values()
            .any(|s| matches!(s.status, SessionStatus::Failed))
        {
            return SessionStatus::Failed;
        }
        if sessions
            .values()
            .any(|s| matches!(s.status, SessionStatus::Working))
        {
            return SessionStatus::Working;
        }
        SessionStatus::Idle
    }

    /// Downgrade stale Working sessions to Idle (no events for 5 minutes).
    /// Returns true if any session was actually downgraded.
    pub fn check_staleness(&self) -> bool {
        const STALE_THRESHOLD_MILLIS: u64 = 5 * 60 * 1_000; // 5 minutes

        let mut sessions = self.sessions.lock().unwrap();
        let now = Self::now_millis();

        let mut changed = false;
        for s in sessions.values_mut() {
            if matches!(s.status, SessionStatus::Working)
                && now - s.timestamp > STALE_THRESHOLD_MILLIS
            {
                s.status = SessionStatus::Idle;
                s.last_event = "StaleDowngrade".to_string();
                s.timestamp = now;
                changed = true;
            }
        }

        if changed {
            Self::cleanup_sessions(&mut sessions);
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session_projects(manager: &SessionManager) -> Vec<String> {
        manager
            .get_sessions()
            .into_iter()
            .map(|s| s.project_name)
            .collect()
    }

    #[test]
    fn unknown_stop_events_do_not_create_tmp_sessions() {
        let manager = SessionManager::new();

        manager.handle_event(&HookEvent::Stop, "missing", "/tmp/claude-code");
        manager.handle_event(&HookEvent::SubagentStop, "subagent", "/tmp/subagent-work");

        assert!(manager.get_sessions().is_empty());
    }

    #[test]
    fn session_key_includes_cwd_to_avoid_cross_project_overwrite() {
        let manager = SessionManager::new();

        manager.handle_event(&HookEvent::SessionStart, "same-id", "/work/project-a");
        manager.handle_event(&HookEvent::SessionStart, "same-id", "/work/project-b");

        assert_eq!(session_projects(&manager), vec!["project-b", "project-a"]);
    }

    #[test]
    fn active_session_count_excludes_idle_history() {
        let manager = SessionManager::new();

        manager.handle_event(&HookEvent::SessionStart, "active", "/work/active");
        manager.handle_event(&HookEvent::SessionStart, "finished", "/work/finished");
        manager.handle_event(&HookEvent::Stop, "finished", "/work/finished");

        assert_eq!(manager.active_session_count(), 1);
    }

    #[test]
    fn get_sessions_returns_active_recent_sessions_first() {
        let manager = SessionManager::new();

        manager.handle_event(&HookEvent::SessionStart, "old", "/work/old");
        manager.handle_event(&HookEvent::Stop, "old", "/work/old");
        manager.handle_event(&HookEvent::SessionStart, "new", "/work/new");
        manager.handle_event(&HookEvent::PermissionRequest, "waiting", "/work/waiting");

        let sessions = manager.get_sessions();

        assert_eq!(
            sessions
                .iter()
                .map(|s| s.project_name.as_str())
                .collect::<Vec<_>>(),
            vec!["waiting", "new", "old"]
        );
    }
}