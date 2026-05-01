use helper_protocol::HookEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const STALE_THRESHOLD_SECONDS: u64 = 5 * 60; // 5 min — no event means session exited
const IDLE_TIMEOUT_SECONDS: u64 = 24 * 60 * 60; // 24 hours
const FAILED_TIMEOUT_SECONDS: u64 = 60 * 60; // 1 hour
const MAX_SESSIONS: usize = 50;

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
        let mut sessions = self.sessions.lock().unwrap();
        let project_name = std::path::Path::new(cwd)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let event_name = format!("{:?}", event);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Ensure session exists for any event — don't rely on SessionStart alone
        if !sessions.contains_key(session_id) {
            sessions.insert(
                session_id.to_string(),
                Session {
                    session_id: session_id.to_string(),
                    cwd: cwd.to_string(),
                    project_name,
                    status: SessionStatus::Working,
                    last_event: event_name.clone(),
                    timestamp: now,
                },
            );
        }

        match event {
            HookEvent::SessionStart => {
                // Session already created above; just ensure Working status
                if let Some(s) = sessions.get_mut(session_id) {
                    s.status = SessionStatus::Working;
                    s.last_event = event_name;
                    s.timestamp = now;
                }
            }
            HookEvent::SessionEnd | HookEvent::Stop | HookEvent::SubagentStop => {
                // Task finished but Claude Code may still be running
                // Keep session visible with Idle status
                if let Some(s) = sessions.get_mut(session_id) {
                    s.status = SessionStatus::Idle;
                    s.last_event = event_name;
                    s.timestamp = now;
                }
            }
            HookEvent::StopFailure => {
                if let Some(s) = sessions.get_mut(session_id) {
                    s.status = SessionStatus::Failed;
                    s.last_event = event_name;
                    s.timestamp = now;
                }
            }
            HookEvent::PermissionRequest => {
                if let Some(s) = sessions.get_mut(session_id) {
                    s.status = SessionStatus::WaitingPermission;
                    s.last_event = event_name;
                    s.timestamp = now;
                }
            }
            HookEvent::PostToolUse | HookEvent::PreToolUse => {
                if let Some(s) = sessions.get_mut(session_id) {
                    s.status = SessionStatus::Working;
                    s.last_event = event_name;
                    s.timestamp = now;
                }
            }
            // Log all other events
            _ => {
                if let Some(s) = sessions.get_mut(session_id) {
                    s.last_event = event_name;
                    s.timestamp = now;
                }
            }
        }

        // Cleanup old sessions after handling event
        Self::cleanup_sessions(&mut sessions);
    }

    fn cleanup_sessions(sessions: &mut HashMap<String, Session>) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut degraded = false;

        // Degrade stale Working/WaitingPermission sessions to Idle
        // (Claude Code doesn't emit events on user exit)
        for s in sessions.values_mut() {
            let elapsed = now - s.timestamp;
            if matches!(s.status, SessionStatus::Working | SessionStatus::WaitingPermission)
                && elapsed >= STALE_THRESHOLD_SECONDS
            {
                s.status = SessionStatus::Idle;
                degraded = true;
            }
        }

        // Remove sessions past their timeout
        sessions.retain(|_, s| {
            let timeout = match s.status {
                SessionStatus::Idle => IDLE_TIMEOUT_SECONDS,
                SessionStatus::Failed => FAILED_TIMEOUT_SECONDS,
                _ => u64::MAX,
            };
            now - s.timestamp < timeout
        });

        // If still too many sessions, remove oldest ones
        if sessions.len() > MAX_SESSIONS {
            let mut session_list: Vec<_> = sessions.iter().collect();
            session_list.sort_by_key(|(_, s)| s.timestamp);

            let to_remove = sessions.len() - MAX_SESSIONS;
            let ids_to_remove: Vec<_> = session_list
                .iter()
                .take(to_remove)
                .map(|(id, _)| id.to_string())
                .collect();

            for session_id in ids_to_remove {
                sessions.remove(session_id.as_str());
            }
        }

        degraded
    }

    pub fn get_sessions(&self) -> Vec<Session> {
        self.sessions.lock().unwrap().values().cloned().collect()
    }

    /// Run staleness check and cleanup without requiring a new event.
    /// Called periodically by a timer so dead sessions get detected even
    /// when no hook events arrive (e.g. user exited Claude Code).
    /// Returns `true` if any session was degraded (Working/WaitingPermission → Idle).
    pub fn tick_cleanup(&self) -> bool {
        let mut sessions = self.sessions.lock().unwrap();
        Self::cleanup_sessions(&mut sessions)
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
}
