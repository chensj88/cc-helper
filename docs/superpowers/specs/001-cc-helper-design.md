# cc-helper — Design Spec

**Date**: 2026-04-08
**Status**: Draft
**Author**: Claude

## Overview

cc-helper is a lightweight, cross-platform desktop companion for Claude Code. It runs as a system tray application, monitors Claude Code sessions via hook events, and provides native UI for permission confirmations, status indication, and session management.

### Problem

Claude Code operates in the terminal. When it needs user confirmation (tool permissions, AskUserQuestion), the user must interact with the terminal. There is no desktop-native way to:

- See at a glance whether Claude is working or idle
- Approve/deny tool execution through a native dialog
- Manage multiple concurrent Claude Code sessions

### Solution

A system tray application that receives Claude Code hook events, displays session status in the tray, and pops up native dialogs for permission requests. It communicates with Claude Code through the hook mechanism: a lightweight CLI (`cc-helper-hook`) forwards hook stdin data to the helper daemon via local socket, and returns user decisions via hook stdout.

### Non-goals

- Full chat interface (covered by cc-remote / terminal)
- Remote access (covered by cc-remote)
- Replacing Claude Code's built-in permission system

## Architecture

```
┌─────────────────────────────────────────────┐
│              cc-helper               │
│                                             │
│  ┌─────────┐  ┌──────────┐  ┌───────────┐  │
│  │  Tray    │  │ Session  │  │  Dialog   │  │
│  │  Icon    │  │ Manager  │  │  Engine   │  │
│  │ (status) │  │ (multi)  │  │ (native)  │  │
│  └────┬─────┘  └────┬─────┘  └─────┬─────┘  │
│       │             │              │         │
│  ┌────┴─────────────┴──────────────┴─────┐  │
│  │           Hook IPC Bridge             │  │
│  │    (Unix Socket / Named Pipe)         │  │
│  └────────────────┬──────────────────────┘  │
└───────────────────┼─────────────────────────┘
                    │
          ┌─────────┴─────────┐
          │   cc-helper-hook CLI   │  ← Claude Code hook invokes this
          │   (stdin→socket)   │
          │   (socket→stdout)  │
          └───────────────────┘
```

### Components

1. **Tray Icon** — system tray icon showing aggregate status of all sessions
2. **Session Manager** — tracks multiple Claude Code sessions by `session_id + cwd`
3. **Dialog Engine** — platform-native dialogs for permission requests
4. **Hook IPC Bridge** — listens on local socket, routes events to appropriate handler
5. **cc-helper-hook CLI** — standalone lightweight binary that bridges Claude Code hooks to the helper daemon

### Communication Flow

```
Claude Code ──(hook stdin)──→ cc-helper-hook ──(socket)──→ helper daemon
                                                           │
                                                     Native dialog popup
                                                           │
Claude Code ←─(hook stdout)── cc-helper-hook ←─(socket)── User decision
```

For non-blocking events (Stop, PostToolUse, etc.), the daemon responds immediately with an ack and the hook exits. Only `PermissionRequest` blocks until the user acts.

## Hook Integration

### Hook Events

| Hook Event | Helper Behavior | Priority |
|---|---|---|
| **PermissionRequest** | Show native permission dialog, return user decision via stdout | P0 |
| PostToolUse | Update session working status (Claude continues after tool use) | P1 |
| Notification | Capture notification types, update tray status | P1 |
| SessionStart | Register new session, tray icon to active | P1 |
| SessionEnd | Cleanup session state (NOTE: not currently emitted by Claude Code; handled for future compatibility) | P1* |
| Stop | Session ended, tray icon to idle, optional desktop notification | P1 |
| StopFailure | Session failed, tray icon to red, desktop notification with error summary | P1 |
| SubagentStop | Sub-agent completion notification (optional display) | P2 |
| TaskCompleted | Task completion notification | P2 |

**Key distinction**: We use `PermissionRequest` (not `PreToolUse`) for permission dialogs. `PermissionRequest` fires only when Claude Code's own permission system would show a prompt — exactly when helper should show a native dialog. `PreToolUse` fires on every tool call (including auto-allowed ones like Read, Glob) and would create unnecessary noise and latency.

**NOTE on SessionEnd**: `SessionEnd` is not currently emitted by Claude Code. It is defined in the protocol and hook configuration for forward compatibility, but `Stop` is the actual event that signals session completion in current Claude Code versions. The SessionEnd hook entry in settings.json will not be triggered until Claude Code adds support for it.

**Out of scope for V1**: The following Claude Code hook events exist but are not used: `PreToolUse`, `UserPromptSubmit`, `PostToolUseFailure`, `PermissionDenied`, `SubagentStart`, `TeammateIdle`, `ConfigChange`, `CwdChanged`, `FileChanged`, `InstructionsLoaded`, `PreCompact`, `PostCompact`, `WorktreeCreate`, `WorktreeRemove`, `Elicitation`, `ElicitationResult`. These may be integrated in future versions.

### Settings Configuration

```json
{
  "hooks": {
    "PermissionRequest": [
      {
        "matcher": "",
        "hooks": [
          { "type": "command", "command": "cc-helper-hook permission-request" }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "",
        "hooks": [
          { "type": "command", "command": "cc-helper-hook post-tool-use" }
        ]
      }
    ],
    "Notification": [
      {
        "matcher": "",
        "hooks": [
          { "type": "command", "command": "cc-helper-hook notification" }
        ]
      }
    ],
    "Stop": [
      {
        "matcher": "",
        "hooks": [
          { "type": "command", "command": "cc-helper-hook stop" }
        ]
      }
    ],
    "StopFailure": [
      {
        "matcher": "",
        "hooks": [
          { "type": "command", "command": "cc-helper-hook stop-failure" }
        ]
      }
    ],
    "SessionStart": [
      {
        "matcher": "",
        "hooks": [
          { "type": "command", "command": "cc-helper-hook session-start" }
        ]
      }
    ],
    "SessionEnd": [
      {
        "matcher": "",
        "hooks": [
          { "type": "command", "command": "cc-helper-hook session-end" }
        ]
      }
    ]
    // NOTE: SessionEnd is not currently emitted by Claude Code.
    // This entry is included for forward compatibility and will not be triggered
    // until Claude Code adds support for it.
  }
}
```

### Permission Decision Flow

`PermissionRequest` hook fires when Claude Code is about to show a permission prompt. cc-helper-hook forwards the event to the daemon, which shows a native dialog. User choice is returned via stdout:

Allow:
```json
{
  "hookSpecificOutput": {
    "hookEventName": "PermissionRequest",
    "permissionDecision": "allow"
  }
}
```

Deny with reason:
```json
{
  "hookSpecificOutput": {
    "hookEventName": "PermissionRequest",
    "permissionDecision": "deny",
    "permissionDecisionReason": "User denied this operation"
  }
}
```

The `hookEventName` field is required in all `hookSpecificOutput` responses.

## IPC Protocol

### Socket Path

- Linux/macOS: `/tmp/cc-helper.sock` (Unix domain socket)
- Windows: `\\.\pipe\cc-helper` (Named Pipe)

### Request Format (cc-helper-hook → daemon)

`cc-helper-hook` extracts `session_id` and `cwd` from Claude Code's stdin JSON and forwards them:

```json
{
  "version": 1,
  "event": "permission_request",
  "session_id": "abc123",
  "cwd": "/home/user/projects/my-project",
  "payload": {
    "tool_name": "Bash",
    "tool_input": { "command": "npm test" },
    "tool_use_id": "toolu_01ABC"
  }
}
```

### Response Format (daemon → cc-helper-hook)

Blocking response (PermissionRequest):

```json
{
  "version": 1,
  "type": "decision",
  "hookSpecificOutput": {
    "hookEventName": "PermissionRequest",
    "permissionDecision": "allow"
  }
}
```

Non-blocking ack:

```json
{
  "version": 1,
  "type": "ack"
}
```

### Error Handling & Resilience

**Daemon unreachable**: `cc-helper-hook` connects with a 3-second timeout. If the daemon is not running:
- For blocking events (PermissionRequest): exit with code 0 and no stdout output, letting Claude Code fall through to its own terminal prompt
- For non-blocking events: exit with code 0 silently

**Daemon crash during blocking wait**: `cc-helper-hook` detects socket disconnection and exits with code 0 (no stdout), allowing Claude Code to proceed with its default behavior.

**Stale socket cleanup**: On startup, the daemon checks if the socket file already exists. If it does, the daemon attempts to connect to it — if the connection fails (stale from previous crash), the daemon removes the file and creates a new socket.

**Protocol versioning**: The daemon rejects requests with mismatched `version` fields, returning `{"version": 1, "type": "error", "message": "version mismatch"}`. `cc-helper-hook` then exits with code 0.

**Concurrent connections**: The daemon accepts multiple simultaneous connections (one per `cc-helper-hook` invocation). Each connection is handled independently in its own task/thread. This is necessary because multiple Claude Code sessions may trigger hooks concurrently.

**Performance target**: `cc-helper-hook` must complete in under 100ms for non-blocking events (daemon ack). Blocking events (PermissionRequest) block until user action with no hard timeout — Claude Code's own hook timeout (600s default) serves as the upper bound.

## System Tray & Session Management

### Tray Icon States

| State | Visual | Trigger |
|---|---|---|
| Idle | Gray/static icon | No active Claude sessions |
| Working | Green pulse/rotation animation | Session started, no Stop received |
| Waiting for permission | Yellow flash + desktop notification | PermissionRequest dialog shown, awaiting user |
| Failed | Red icon | StopFailure event |
| Multi-session | Badge showing active count | Multiple Claude instances running |

### Multi-session Display

Clicking the tray icon shows a session list panel:

```
┌─────────────────────────────────────┐
│ > project-a  [working]  abc123      │  ← /home/user/projects/my-project
│ > project-b  [idle]     def456      │  ← /home/user/projects/webapp
│ > project-a  [perm]     ghi789      │  ← /home/user/projects/my-project
└─────────────────────────────────────┘
```

Sessions are identified by `session_id` (unique key). Display shows `cwd` basename + `session_id` prefix to distinguish multiple sessions in the same directory.

### Tray Context Menu

- Session list with status (read-only)
- Settings (IPC path, notification preferences, etc.)
- Quit

### Desktop Notifications

Helper uses Tauri's notification API for desktop notifications:
- Linux: libnotify
- macOS: NSUserNotification
- Windows: Windows Toast Notifications

## Permission Dialog UI

Each platform follows its native design language. Interaction logic is identical across platforms.

**Implementation approach**: Tauri's built-in `tauri::api::dialog` supports simple message dialogs. For permission dialogs with structured tool-specific content, helper uses a **Tauri webview popup window** styled to match the platform's native look (macOS vibrancy, Windows Mica, GTK headerbar). This avoids platform-specific native toolkit dependencies while providing rich content display.

### macOS

- Window styled with macOS vibrancy backdrop
- Button order: Deny (left), Allow (right) — macOS convention

### Windows

- Window styled with WinUI-inspired theme
- Button order: Allow (left), Deny (right) — Windows convention
- Taskbar flash for attention

### Linux (Debian)

- GTK-inspired styling (follows system theme light/dark)
- Button order: Deny (left), Allow (right) — GNOME convention

### Tool Information Display

| Tool | Display |
|---|---|
| Bash | Command + description |
| Edit | File path + old_string summary |
| Write | File path |
| MCP tools | Tool name + key parameters |
| Other | Tool name + parameter JSON summary |

## Tech Stack

**Primary**: Tauri (Rust backend + Web frontend)

**Fallback**: If Rust learning curve is too steep, per-platform native (macOS Swift/SwiftUI, Windows C#/WPF, Linux Python/PyQt). This is a last resort, not a parallel implementation.

### Project Structure

```
cc-helper/
├── src-tauri/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs           # Tauri entry
│       ├── tray.rs           # System tray
│       ├── ipc.rs            # Socket listener + protocol
│       ├── session.rs        # Session management
│       └── dialog.rs         # Dialog window management
├── src/                      # Web frontend (tray panel + dialog UI)
│   ├── App.vue
│   ├── dialogs/
│   │   └── Permission.vue    # Permission dialog component
│   └── main.js
├── cc-helper-hook/               # Standalone CLI binary
│   └── main.rs               # stdin → socket → stdout bridge
├── package.json
└── tauri.conf.json
```

`cc-helper-hook` is a standalone lightweight binary with no Tauri runtime dependency. It does one thing: read stdin JSON, forward to helper daemon via socket, wait for response, write to stdout. Compiled size ~1MB for fast startup when invoked by Claude Code hooks.

### Build Artifacts

| Platform | Artifact | cc-helper-hook |
|---|---|---|
| macOS | `cc-helper.app` | Embedded, symlinked to `/usr/local/bin` |
| Windows | `cc-helper.exe` | Embedded, added to PATH |
| Linux | AppImage / deb | Embedded, symlinked to `/usr/local/bin` |

### Installation

```bash
# 1. Install helper
dpkg -i cc-helper_0.1.0_amd64.deb    # Linux
# or drag to Applications                       # macOS

# 2. Auto-configure Claude Code hooks
helper install
```

`helper install` **merges** hook configuration into `~/.claude/settings.json`. It reads existing hooks, appends helper's hooks to each event's array (preserving user's existing hooks), and writes back. `helper uninstall` removes only helper's hooks, leaving user's hooks intact.

## V2 Roadmap

### AskUserQuestion Support (PTY Wrapper)

Upgrade from hook-only mode to PTY wrapper mode:

```
helper spawn Claude Code (PTY)
    │
    ├── Parse stdout → detect AskUserQuestion pattern
    │   (option list, text input, "Other" option)
    │
    ├── Show native dialog
    │
    └── User selection → write to PTY stdin
```

Challenge: Claude Code's terminal output is ANSI-formatted. AskUserQuestion option lists lack structured markers, requiring regex/pattern matching that may break across Claude Code version updates.

**Alternative V2 approach**: Use `PreToolUse` hook with `permissionDecision: "defer"` when running Claude Code in non-interactive mode (`-p`). This provides a structured mechanism for deferred tool approval without needing PTY wrapping.

### cc-remote Integration (Optional)

Helper reserves an optional config for cc-remote server connection:

```json
{
  "cc_remote": {
    "enabled": false,
    "server_url": "http://localhost:3000",
    "token": "xxx"
  }
}
```

When enabled, helper reports local events (permission decisions, session status changes) to cc-remote for unified visibility in the web interface. Helper operates fully independently when disabled.

## Development Roadmap

| Phase | Content | Platform |
|---|---|---|
| V0.1 | Tauri skeleton + system tray + cc-helper-hook CLI + IPC | Linux first |
| V0.2 | PermissionRequest dialog + PostToolUse status | Linux |
| V0.3 | Multi-session + Stop/StopFailure notifications | Linux |
| V0.4 | macOS adaptation + signing | macOS |
| V0.5 | Windows adaptation | Windows |
| V1.0 | `helper install` auto-config + settings panel | All platforms |
| V2.0 | PTY wrapper / defer mode + AskUserQuestion + cc-remote integration | All platforms |
