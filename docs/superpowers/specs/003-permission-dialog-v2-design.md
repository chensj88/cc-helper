# Permission Dialog V2 Design

## Overview

Redesign the permission dialog to support all 15+ Claude Code tool types, with full rendering of AskUserQuestion options and user input. The dialog acts as a complete replacement for Claude Code's native permission UI.

## Approach

Single `PermissionDialog.vue` component with mode switching based on `tool_name`. One code path for all tool types, with tool-specific rendering sections.

## Data Flow

```
Claude Code → hook (stdin) → cc-helper-hook → Unix socket → Tauri daemon → WebviewWindow → Vue component
                                                                                              ↓ user action
Claude Code ← hook (stdout) ← cc-helper-hook ← socket response ← resolve() ← invoke ← __helperResolve
```

Protocol unchanged. The `tool_name` and `tool_input` fields in the request payload already contain everything needed.

## UI Modes

### Mode 1: AskUserQuestion (`ask_user_question`)

The most complex mode. Renders questions with selectable options.

**Layout:**
- Question text with header chip
- Option cards (2-4 per question): label + description
- "Other" text input for custom answers (always available)
- Multi-question support (1-4 questions, list layout)
- Submit / Cancel buttons

**Window size:** 600x450

**Interactions:**
- Single select: radio buttons / clickable cards
- Multi select: checkboxes
- Custom input: text field under "Other"
- All questions must be answered before Submit enables

**Response format:**
```json
{
  "behavior": "allow",
  "updatedInput": {
    "questions": [...],
    "answers": {
      "question text": "selected_label_or_custom_text"
    }
  }
}
```

Cancel response: `{"behavior": "deny", "message": "User cancelled"}`

### Mode 2: Command (`bash`, `powershell`)

**Layout:**
- Tool label ("Bash" / "PowerShell")
- Command preview in monospace block
- Optional description text
- Allow / Deny buttons

**Window size:** 500x300

### Mode 3: File Operation (`edit`, `write`, `read`, `glob`, `grep`)

**Layout:**
- Operation type label (Edit / Write / Read / Search)
- File path
- Allow / Deny buttons

**Window size:** 500x300

### Mode 4: Web Fetch (`web_fetch`)

**Layout:**
- URL display
- Allow / Deny buttons

**Window size:** 500x300

### Mode 5: Skill (`skill`)

**Layout:**
- Skill name + arguments
- Allow / Deny buttons

**Window size:** 500x300

### Mode 6: Fallback (all other tools)

**Layout:**
- Tool name
- JSON representation of tool_input (truncated)
- Allow / Deny buttons

**Window size:** 500x300

## Rust Changes

### Protocol (`crates/protocol/src/lib.rs`)

Add `allow_with_input()` method to `Response`:

```rust
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
                message: None,
                interrupt: None,
            },
        }),
    }
}
```

Add `updated_input` field to `PermissionDecision`:

```rust
pub struct PermissionDecision {
    pub behavior: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interrupt: Option<bool>,
}
```

### Dialog Manager (`src-tauri/src/dialog.rs`)

- Pass `tool_name` to window for size selection
- Store `tool_input` in pending map for response construction
- `resolve()` accepts optional `answers` parameter

### Command (`src-tauri/src/lib.rs`)

```rust
#[tauri::command]
fn resolve_permission(app: tauri::AppHandle, key: String, allowed: bool, answers: Option<serde_json::Value>) {
    let state = app.state::<DialogManager>();
    state.resolve(&app, &key, allowed, answers);
}
```

## Frontend Changes

### PermissionDialog.vue

Mode-switching component:

```vue
<template>
  <div class="permission-dialog">
    <AskUserQuestionRenderer v-if="isAskMode" ... />
    <CommandRenderer v-else-if="isCommandMode" ... />
    <FileRenderer v-else-if="isFileMode" ... />
    <WebFetchRenderer v-else-if="isWebFetchMode" ... />
    <SkillRenderer v-else-if="isSkillMode" ... />
    <FallbackRenderer v-else ... />
  </div>
</template>
```

### AskUserQuestionRenderer

- Parses `questions` array from `tool_input`
- Renders each question with option cards
- Handles "Other" text input
- Collects answers into `{ "question text": "answer" }` map
- On submit: `window.__helperResolve(true, answers)`
- On cancel: `window.__helperResolve(false, null)`

### dialog-main.ts

- Parse `tool_input` from URL params
- Update `__helperResolve` signature to accept optional answers

## Error Handling

- If `tool_input` parsing fails, fall back to FallbackRenderer
- If answers are missing for required questions, disable Submit
- All errors result in deny response (never block Claude Code)

## Window Management

- AskUserQuestion windows: 600x450
- Other tool windows: 500x300
- Always on top, centered, not resizable
- Close intercepted and hidden (not destroyed) to prevent app exit
- Old hidden windows destroyed after new window creation
