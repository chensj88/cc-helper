use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
    PreCompact,
    Notification,
    PermissionRequest,
    SessionStart,
    SessionEnd,
    Stop,
    StopFailure,
    SubagentStop,
    TaskCompleted,
}

impl HookEvent {
    /// Parse from the CLI argument string used by Claude Code hooks.
    pub fn from_cli_arg(s: &str) -> Option<Self> {
        match s {
            "pre-tool-use" => Some(Self::PreToolUse),
            "post-tool-use" => Some(Self::PostToolUse),
            "pre-compact" => Some(Self::PreCompact),
            "notification" => Some(Self::Notification),
            "permission-request" => Some(Self::PermissionRequest),
            "session-start" => Some(Self::SessionStart),
            "session-end" => Some(Self::SessionEnd),
            "stop" => Some(Self::Stop),
            "stop-failure" => Some(Self::StopFailure),
            "subagent-stop" => Some(Self::SubagentStop),
            "task-completed" => Some(Self::TaskCompleted),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub version: u32,
    pub event: HookEvent,
    pub session_id: String,
    pub cwd: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub version: u32,
    #[serde(rename = "type")]
    pub response_type: ResponseType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook_specific_output: Option<HookSpecificOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseType {
    Ack,
    Decision,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionDecision {
    pub behavior: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_permissions: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interrupt: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookSpecificOutput {
    pub hook_event_name: String,
    pub decision: PermissionDecision,
}

impl Response {
    pub fn ack() -> Self {
        Self {
            version: PROTOCOL_VERSION,
            response_type: ResponseType::Ack,
            message: None,
            hook_specific_output: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            response_type: ResponseType::Error,
            message: Some(msg.into()),
            hook_specific_output: None,
        }
    }

    pub fn allow() -> Self {
        Self {
            version: PROTOCOL_VERSION,
            response_type: ResponseType::Decision,
            message: None,
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "PermissionRequest".to_string(),
                decision: PermissionDecision {
                    behavior: "allow".to_string(),
                    updated_input: None,
                    updated_permissions: None,
                    message: None,
                    interrupt: None,
                },
            }),
        }
    }

    pub fn always_allow(permission_suggestions: Vec<serde_json::Value>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            response_type: ResponseType::Decision,
            message: None,
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "PermissionRequest".to_string(),
                decision: PermissionDecision {
                    behavior: "allow".to_string(),
                    updated_input: None,
                    updated_permissions: Some(permission_suggestions),
                    message: None,
                    interrupt: None,
                },
            }),
        }
    }

    pub fn allow_with_input(tool_input: &serde_json::Value, answers: &serde_json::Value) -> Self {
        let mut updated = tool_input.clone();
        if let Some(obj) = updated.as_object_mut() {
            obj.insert("answers".to_string(), answers.clone());
        }
        Self {
            version: PROTOCOL_VERSION,
            response_type: ResponseType::Decision,
            message: None,
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "PermissionRequest".to_string(),
                decision: PermissionDecision {
                    behavior: "allow".to_string(),
                    updated_input: Some(updated),
                    updated_permissions: None,
                    message: None,
                    interrupt: None,
                },
            }),
        }
    }

    pub fn always_allow_with_input(
        tool_input: &serde_json::Value,
        answers: &serde_json::Value,
        permission_suggestions: Vec<serde_json::Value>,
    ) -> Self {
        let mut updated = tool_input.clone();
        if let Some(obj) = updated.as_object_mut() {
            obj.insert("answers".to_string(), answers.clone());
        }
        Self {
            version: PROTOCOL_VERSION,
            response_type: ResponseType::Decision,
            message: None,
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "PermissionRequest".to_string(),
                decision: PermissionDecision {
                    behavior: "allow".to_string(),
                    updated_input: Some(updated),
                    updated_permissions: Some(permission_suggestions),
                    message: None,
                    interrupt: None,
                },
            }),
        }
    }

    pub fn deny(reason: impl Into<String>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            response_type: ResponseType::Decision,
            message: None,
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "PermissionRequest".to_string(),
                decision: PermissionDecision {
                    behavior: "deny".to_string(),
                    updated_input: None,
                    updated_permissions: None,
                    message: Some(reason.into()),
                    interrupt: None,
                },
            }),
        }
    }

    /// Serialize to JSON followed by a newline, suitable for wire transport.
    pub fn to_wire(&self) -> String {
        let mut json = serde_json::to_string(self).unwrap_or_default();
        json.push('\n');
        json
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_response_format() {
        let resp = Response::allow();
        let wire = resp.to_wire();
        let json: serde_json::Value = serde_json::from_str(wire.trim()).unwrap();
        assert_eq!(json["type"], "decision");
        let output = &json["hookSpecificOutput"];
        assert_eq!(output["hookEventName"], "PermissionRequest");
        assert_eq!(output["decision"]["behavior"], "allow");
    }

    #[test]
    fn allow_with_input_format() {
        let tool_input = serde_json::json!({"questions": [{"question": "Which?"}]});
        let answers = serde_json::json!({"Which?": "React"});
        let resp = Response::allow_with_input(&tool_input, &answers);
        let wire = resp.to_wire();
        let json: serde_json::Value = serde_json::from_str(wire.trim()).unwrap();
        let output = &json["hookSpecificOutput"];
        assert_eq!(output["decision"]["behavior"], "allow");
        let updated = &output["decision"]["updatedInput"];
        assert_eq!(updated["answers"]["Which?"], "React");
    }

    #[test]
    fn deny_response_format() {
        let resp = Response::deny("dangerous command");
        let wire = resp.to_wire();
        let json: serde_json::Value = serde_json::from_str(wire.trim()).unwrap();
        let output = &json["hookSpecificOutput"];
        assert_eq!(output["decision"]["behavior"], "deny");
        assert_eq!(output["decision"]["message"], "dangerous command");
    }

    #[test]
    fn always_allow_response_format() {
        let suggestions = vec![serde_json::json!({
            "type": "addRules",
            "rules": [{"toolName": "Bash", "ruleContent": "npm test"}],
            "behavior": "allow",
            "destination": "localSettings"
        })];
        let resp = Response::always_allow(suggestions);
        let wire = resp.to_wire();
        let json: serde_json::Value = serde_json::from_str(wire.trim()).unwrap();
        let output = &json["hookSpecificOutput"];
        assert_eq!(output["decision"]["behavior"], "allow");
        let perms = &output["decision"]["updatedPermissions"];
        assert_eq!(perms[0]["type"], "addRules");
        assert_eq!(perms[0]["rules"][0]["ruleContent"], "npm test");
    }

    #[test]
    fn always_allow_with_input_format() {
        let tool_input = serde_json::json!({"questions": [{"question": "Which?"}]});
        let answers = serde_json::json!({"Which?": "React"});
        let suggestions = vec![serde_json::json!({
            "type": "addRules",
            "rules": [{"toolName": "Bash", "ruleContent": "npm test"}],
            "behavior": "allow",
            "destination": "localSettings"
        })];
        let resp = Response::always_allow_with_input(&tool_input, &answers, suggestions);
        let wire = resp.to_wire();
        let json: serde_json::Value = serde_json::from_str(wire.trim()).unwrap();
        let output = &json["hookSpecificOutput"];
        assert_eq!(output["decision"]["behavior"], "allow");
        assert_eq!(output["decision"]["updatedInput"]["answers"]["Which?"], "React");
        let perms = &output["decision"]["updatedPermissions"];
        assert_eq!(perms[0]["type"], "addRules");
    }
}
