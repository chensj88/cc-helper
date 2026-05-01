# cc-helper Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a cross-platform system tray companion for Claude Code that displays session status, shows native permission dialogs, and manages multiple sessions via hook events.

**Architecture:** Tauri v2 app with Rust backend (socket listener, session manager, dialog engine) + Web frontend (tray panel, permission dialog). A standalone `cc-helper-hook` CLI binary bridges Claude Code hook stdin to the daemon via Unix socket. Claude Code's `PermissionRequest` hook fires → cc-helper-hook forwards to daemon → native dialog → user choice → stdout back to Claude Code.

**Tech Stack:** Rust, Tauri v2, Vue 3, tokio (async socket), serde_json (IPC protocol)

**Spec:** `docs/superpowers/specs/001-cc-helper-design.md`

---

## File Structure

```
cc-helper/
├── Cargo.toml                          # Workspace root
├── crates/
│   ├── protocol/                       # Shared IPC types (used by both binaries)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs
│   └── cc-helper-hook/                     # Standalone CLI binary
│       ├── Cargo.toml
│       └── src/
│           └── main.rs
├── src-tauri/                          # Tauri app (daemon)
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   └── default.json
│   ├── icons/
│   └── src/
│       ├── main.rs                     # Tauri entry + tray setup
│       ├── ipc.rs                      # Unix socket listener + handler
│       ├── session.rs                  # Session manager
│       └── dialog.rs                   # Permission dialog window management
├── src/                                # Web frontend
│   ├── main.js                         # Vue app entry
│   ├── App.vue                         # Main layout (hidden main window)
│   ├── styles.css                      # Global styles
│   └── components/
│       ├── SessionPanel.vue            # Tray click panel with session list
│       └── PermissionDialog.vue        # Permission request popup
├── index.html                          # Entry HTML
├── package.json
├── vite.config.js
└── docs/
    └── superpowers/
        ├── specs/001-cc-helper-design.md
        └── plans/001-cc-helper-v01-v03.md
```

---

## Chunk 1: Dev Environment + Project Scaffold + Protocol + cc-helper-hook CLI

### Task 1: Install Rust and Tauri prerequisites

**Files:**
- N/A (system-level installation)

- [ ] **Step 1: Install Rust via rustup**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

- [ ] **Step 2: Verify installation**

Run: `rustc --version && cargo --version`
Expected: Rust 1.77+ and cargo

- [ ] **Step 3: Install Tauri system dependencies (Debian)**

```bash
sudo apt update
sudo apt install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libssl-dev libgtk-3-dev libayatana-appindicator3-dev
```

- [ ] **Step 4: Install Tauri CLI**

```bash
cargo install tauri-cli
```

- [ ] **Step 5: Commit (git ignore update)**

---

### Task 2: Initialize Tauri project with workspace

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `package.json`
- Create: `vite.config.js`
- Create: `index.html`
- Create: `src/main.js`
- Create: `src/App.vue`
- Create: `src/styles.css`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/capabilities/default.json`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/src/main.rs`

- [ ] **Step 1: Create Tauri project scaffold**

```bash
cd ~/projects/cc-helper
npm create tauri-app@latest -- --template vue-ts --manager npm .
```

If interactive, choose: Project name: `cc-helper`, Frontend: `Vue + TypeScript`, Package manager: `npm`.

If the command fails because the directory is not empty (has docs/), create in a temp dir and move:

```bash
cd /tmp && npm create tauri-app@latest cc-helper-init -- --template vue-ts --manager npm
cp -r /tmp/cc-helper-init/{package.json,vite.config.js,index.html,src,src-tauri} ~/projects/cc-helper/
rm -rf /tmp/cc-helper-init
```

- [ ] **Step 2: Convert to Cargo workspace**

Replace `Cargo.toml` at project root with workspace config:

```toml
[workspace]
members = ["crates/protocol", "crates/cc-helper-hook", "src-tauri"]
resolver = "2"
```

Remove the standalone `Cargo.toml` if `npm create` created one at root.

- [ ] **Step 3: Update src-tauri/Cargo.toml**

```toml
[package]
name = "cc-helper"
version = "0.1.0"
edition = "2021"

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-notification = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
helper-protocol = { path = "../crates/protocol" }

[build-dependencies]
tauri-build = { version = "2" }
```

- [ ] **Step 4: Create minimal src-tauri/src/main.rs**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: Verify it builds**

Run: `cd ~/projects/cc-helper && cargo build`
Expected: Compiles with warnings about unused code but no errors

- [ ] **Step 6: Configure Vite for multi-page build (needed for dialog.html)**

Update `vite.config.js`:

```javascript
import { defineConfig } from 'vite'
import { resolve } from 'path'

export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
        dialog: resolve(__dirname, 'src/dialog.html'),
      },
    },
  },
})
```

- [ ] **Step 7: Set Tauri identifier**

In `src-tauri/tauri.conf.json`, set the identifier:
```json
{
  "identifier": "com.cc-helper.app"
}
```

- [ ] **Step 8: Add notification permission to capabilities**

`src-tauri/capabilities/default.json`:
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "notification:default"
  ]
}
```

- [ ] **Step 9: Install @tauri-apps/api**

```bash
npm install @tauri-apps/api
```

- [ ] **Step 10: Commit**

```bash
git add -A
git commit -m "chore: initialize Tauri project with workspace"
```

---

### Task 3: Define IPC protocol types (shared crate)

**Files:**
- Create: `crates/protocol/Cargo.toml`
- Create: `crates/protocol/src/lib.rs`
- Test: `crates/protocol/src/lib.rs` (inline tests)

- [ ] **Step 1: Create protocol crate**

```bash
mkdir -p crates/protocol/src
```

`crates/protocol/Cargo.toml`:
```toml
[package]
name = "helper-protocol"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 2: Write protocol types**

`crates/protocol/src/lib.rs`:
```rust
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u32 = 1;

/// Hook event types that cc-helper-hook can forward to the daemon.
/// NOTE: SessionEnd is not currently emitted by Claude Code; defined for forward compatibility.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    PermissionRequest,
    PostToolUse,
    Notification,
    Stop,
    StopFailure,
    SessionStart,
    SessionEnd,
}

/// Request from cc-helper-hook CLI to the daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub version: u32,
    pub event: HookEvent,
    pub session_id: String,
    pub cwd: String,
    pub payload: serde_json::Value,
}

/// Response from the daemon back to cc-helper-hook.
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseType {
    Ack,
    Decision,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookSpecificOutput {
    pub hook_event_name: String,
    pub permission_decision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_decision_reason: Option<String>,
}

impl Request {
    pub fn from_stdin(input: &str) -> Result<Self, serde_json::Error> {
        let mut req: Self = serde_json::from_str(input)?;
        // Normalize event name from Claude Code's hook_event_name field
        Ok(req)
    }
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

    pub fn error(msg: &str) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            response_type: ResponseType::Error,
            message: Some(msg.to_string()),
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
                permission_decision: Some("allow".to_string()),
                permission_decision_reason: None,
            }),
        }
    }

    pub fn deny(reason: &str) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            response_type: ResponseType::Decision,
            message: None,
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "PermissionRequest".to_string(),
                permission_decision: Some("deny".to_string()),
                permission_decision_reason: Some(reason.to_string()),
            }),
        }
    }

    /// Serialize to a single line ending with newline for socket protocol.
    pub fn to_wire(&self) -> Vec<u8> {
        let mut json = serde_json::to_string(self).unwrap();
        json.push('\n');
        json.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_request() {
        let req = Request {
            version: 1,
            event: HookEvent::PermissionRequest,
            session_id: "abc123".to_string(),
            cwd: "/home/user/project".to_string(),
            payload: serde_json::json!({"tool_name": "Bash"}),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.event, HookEvent::PermissionRequest);
        assert_eq!(parsed.session_id, "abc123");
    }

    #[test]
    fn test_response_allow() {
        let resp = Response::allow();
        let wire = resp.to_wire();
        let parsed: serde_json::Value = serde_json::from_slice(&wire).unwrap();
        assert_eq!(parsed["type"], "decision");
        assert_eq!(parsed["hookSpecificOutput"]["permissionDecision"], "allow");
        assert_eq!(
            parsed["hookSpecificOutput"]["hookEventName"],
            "PermissionRequest"
        );
    }

    #[test]
    fn test_response_deny() {
        let resp = Response::deny("User denied");
        let wire = resp.to_wire();
        let parsed: serde_json::Value = serde_json::from_slice(&wire).unwrap();
        assert_eq!(parsed["type"], "decision");
        assert_eq!(
            parsed["hookSpecificOutput"]["permissionDecision"],
            "deny"
        );
        assert_eq!(
            parsed["hookSpecificOutput"]["permissionDecisionReason"],
            "User denied"
        );
    }

    #[test]
    fn test_response_ack() {
        let resp = Response::ack();
        assert_eq!(resp.response_type, ResponseType::Ack);
        assert!(resp.hook_specific_output.is_none());
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cd ~/projects/cc-helper && cargo test -p helper-protocol`
Expected: 4 tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/protocol/
git commit -m "feat: add shared IPC protocol types (helper-protocol crate)"
```

---

### Task 4: Implement cc-helper-hook CLI binary

**Files:**
- Create: `crates/cc-helper-hook/Cargo.toml`
- Create: `crates/cc-helper-hook/src/main.rs`

- [ ] **Step 1: Create cc-helper-hook crate**

```bash
mkdir -p crates/cc-helper-hook/src
```

`crates/cc-helper-hook/Cargo.toml`:
```toml
[package]
name = "cc-helper-hook"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "cc-helper-hook"
path = "src/main.rs"

[dependencies]
helper-protocol = { path = "../protocol" }
serde_json = "1"
```

- [ ] **Step 2: Implement cc-helper-hook main**

`crates/cc-helper-hook/src/main.rs`:
```rust
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

const SOCKET_PATH: &str = "/tmp/cc-helper.sock";
const CONNECT_TIMEOUT_SECS: u64 = 3;

fn main() {
    // Read hook data from stdin (Claude Code passes JSON via stdin)
    let mut stdin_data = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut stdin_data) {
        eprintln!("cc-helper-hook: failed to read stdin: {}", e);
        std::process::exit(0); // exit 0 to not block Claude Code
    }

    if stdin_data.trim().is_empty() {
        std::process::exit(0);
    }

    // Parse the sub-command to determine event type
    let args: Vec<String> = std::env::args().collect();
    let event = match args.get(1).map(|s| s.as_str()) {
        Some("permission-request") => helper_protocol::HookEvent::PermissionRequest,
        Some("post-tool-use") => helper_protocol::HookEvent::PostToolUse,
        Some("notification") => helper_protocol::HookEvent::Notification,
        Some("stop") => helper_protocol::HookEvent::Stop,
        Some("stop-failure") => helper_protocol::HookEvent::StopFailure,
        Some("session-start") => helper_protocol::HookEvent::SessionStart,
        Some("session-end") => helper_protocol::HookEvent::SessionEnd,  // NOTE: not currently emitted by Claude Code
        _ => {
            eprintln!("cc-helper-hook: unknown event type");
            std::process::exit(0);
        }
    };

    // Extract session_id and cwd from stdin JSON
    let stdin_json: serde_json::Value = match serde_json::from_str(&stdin_data) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("cc-helper-hook: failed to parse stdin JSON: {}", e);
            std::process::exit(0);
        }
    };

    let session_id = stdin_json["session_id"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let cwd = stdin_json["cwd"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let request = helper_protocol::Request {
        version: helper_protocol::PROTOCOL_VERSION,
        event,
        session_id,
        cwd,
        payload: stdin_json.clone(),
    };

    // Connect to daemon with timeout
    let stream = match UnixStream::connect(SOCKET_PATH) {
        Ok(s) => s,
        Err(_) => {
            // Daemon not running — exit 0, let Claude Code proceed with defaults
            std::process::exit(0);
        }
    };

    stream
        .set_read_timeout(Some(Duration::from_secs(600)))
        .ok();
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .ok();

    let mut stream = stream;

    // Send request
    let mut wire = serde_json::to_string(&request).unwrap();
    wire.push('\n');
    if let Err(_) = stream.write_all(wire.as_bytes()) {
        std::process::exit(0);
    }

    // Read response (newline-delimited JSON)
    let mut response_buf = String::new();
    let mut byte = [0u8; 1];
    loop {
        match stream.read(&mut byte) {
            Ok(0) => break, // connection closed
            Ok(_) => {
                response_buf.push(byte[0] as char);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    if response_buf.trim().is_empty() {
        // No response (daemon crashed or disconnected)
        std::process::exit(0);
    }

    // Parse response — if it has hookSpecificOutput, write it to stdout
    let response: serde_json::Value = match serde_json::from_str(&response_buf) {
        Ok(v) => v,
        Err(_) => std::process::exit(0),
    };

    if let Some(hso) = response.get("hookSpecificOutput") {
        // Write hookSpecificOutput to stdout for Claude Code to read
        let output = serde_json::to_string(hso).unwrap();
        print!("{}", output);
    }

    // Check for version mismatch
    if response.get("type").and_then(|t| t.as_str()) == Some("error") {
        eprintln!("cc-helper-hook: daemon error");
    }
}
```

- [ ] **Step 3: Build cc-helper-hook**

Run: `cd ~/projects/cc-helper && cargo build -p cc-helper-hook --release`
Expected: Compiles successfully, binary at `target/release/cc-helper-hook`

- [ ] **Step 4: Test cc-helper-hook with no daemon (should exit 0 silently)**

Run: `echo '{"session_id":"test","cwd":"/tmp"}' | cargo run -p cc-helper-hook -- permission-request; echo "exit: $?"`
Expected: exit: 0 (daemon not running, graceful fallback)

- [ ] **Step 5: Commit**

```bash
git add crates/cc-helper-hook/
git commit -m "feat: implement cc-helper-hook CLI binary (stdin→socket→stdout bridge)"
```

---

## Chunk 2: IPC Bridge + System Tray

### Task 5: Implement socket listener in Tauri backend

**Files:**
- Modify: `src-tauri/Cargo.toml` (add tokio)
- Create: `src-tauri/src/ipc.rs`

- [ ] **Step 1: Create the IPC socket listener**

`src-tauri/src/ipc.rs`:
```rust
use helper_protocol::{Request, Response, ResponseType, PROTOCOL_VERSION};
use std::path::Path;
use std::sync::mpsc::Sender;
use tokio::net::UnixListener;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const SOCKET_PATH: &str = "/tmp/cc-helper.sock";

/// Messages sent from IPC listener to the main app logic.
#[derive(Debug)]
pub enum IpcMessage {
    Request {
        request: Request,
        response_tx: tokio::sync::oneshot::Sender<Response>,
    },
}

/// Clean up stale socket file if it exists.
fn cleanup_stale_socket() {
    let path = Path::new(SOCKET_PATH);
    if !path.exists() {
        return;
    }
    // Try to connect — if it works, another daemon is running
    if std::os::unix::net::UnixStream::connect(SOCKET_PATH).is_ok() {
        eprintln!("ipc: another daemon is already running on {}", SOCKET_PATH);
        std::process::exit(1);
    }
    // Stale socket — remove it
    std::fs::remove_file(SOCKET_PATH).ok();
}

/// Start the Unix socket listener. Returns a receiver for incoming messages.
pub fn start_ipc_listener() -> tokio::sync::mpsc::Receiver<IpcMessage> {
    cleanup_stale_socket();

    let (tx, rx) = tokio::sync::mpsc::channel::<IpcMessage>(32);

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .expect("Failed to create tokio runtime for IPC");

        rt.block_on(async move {
            let listener = match UnixListener::bind(SOCKET_PATH) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("ipc: failed to bind socket: {}", e);
                    return;
                }
            };

            loop {
                match listener.accept().await {
                    Ok((stream, _addr)) => {
                        let tx = tx.clone();
                        tokio::spawn(async move {
                            handle_connection(stream, tx).await;
                        });
                    }
                    Err(e) => {
                        eprintln!("ipc: accept error: {}", e);
                    }
                }
            }
        });
    });

    rx
}

async fn handle_connection(
    stream: tokio::net::UnixStream,
    tx: tokio::sync::mpsc::Sender<IpcMessage>,
) {
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    match reader.read_line(&mut line).await {
        Ok(0) => return, // connection closed
        Ok(_) => {}
        Err(e) => {
            eprintln!("ipc: read error: {}", e);
            return;
        }
    }

    let request: Request = match serde_json::from_str(&line) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ipc: parse error: {}", e);
            let err = Response::error(&format!("parse error: {}", e));
            let _ = writer.write_all(&err.to_wire()).await;
            return;
        }
    };

    // Version check
    if request.version != PROTOCOL_VERSION {
        let err = Response::error("version mismatch");
        let _ = writer.write_all(&err.to_wire()).await;
        return;
    }

    // For non-blocking events, send ack immediately
    if !matches!(request.event, helper_protocol::HookEvent::PermissionRequest) {
        let ack = Response::ack();
        let _ = writer.write_all(&ack.to_wire()).await;
        // Still forward to app for status tracking
        // Create a dummy oneshot channel — the receiver will be dropped immediately,
        // so any send attempt on tx will harmlessly fail
        let (dummy_tx, _dummy_rx) = tokio::sync::oneshot::channel::<Response>();
        let _ = tx.try_send(IpcMessage::Request {
            request,
            response_tx: dummy_tx,
        });
        return;
    }

    // Blocking event: create oneshot channel for response
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel::<Response>();

    if tx.try_send(IpcMessage::Request {
        request,
        response_tx: resp_tx,
    })
    .is_err()
    {
        // App not handling — fall through
        return;
    }

    // Wait for app to provide response (user clicks Allow/Deny)
    match resp_rx.await {
        Ok(response) => {
            let _ = writer.write_all(&response.to_wire()).await;
        }
        Err(_) => {
            // App dropped the sender (crashed or shutdown)
        }
    }
}
```

- [ ] **Step 2: Wire IPC into main.rs**

`src-tauri/src/main.rs`:
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ipc;

use ipc::IpcMessage;

fn main() {
    let mut ipc_rx = ipc::start_ipc_listener();

    tauri::Builder::default()
        .setup(move |_app| {
            // Handle incoming IPC messages
            std::thread::spawn(move || {
                while let Some(msg) = ipc_rx.blocking_recv() {
                    match msg {
                        IpcMessage::Request { request, response_tx } => {
                            println!(
                                "Received: {:?} from session {} ({})",
                                request.event, request.session_id, request.cwd
                            );
                            // For now, auto-allow everything (will be replaced by dialog in V0.2)
                            if matches!(request.event, helper_protocol::HookEvent::PermissionRequest) {
                                let _ = response_tx.send(helper_protocol::Response::allow());
                            }
                        }
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Build**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 4: Test end-to-end IPC**

Terminal 1:
```bash
cd ~/projects/cc-helper && cargo run
```

Terminal 2:
```bash
echo '{"session_id":"test1","cwd":"/tmp/test"}' | cargo run -p cc-helper-hook -- session-start; echo "exit: $?"
echo '{"session_id":"test1","cwd":"/tmp/test","tool_name":"Bash","tool_input":{"command":"ls"}}' | cargo run -p cc-helper-hook -- permission-request; echo "exit: $?"
```

Expected:
- Terminal 1 prints: "Received: SessionStart from session test1 (/tmp/test)"
- Terminal 2: exit 0 for both commands
- Permission-request auto-responds with allow (V0.2 will show dialog)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/
git commit -m "feat: implement IPC socket listener with concurrent connection handling"
```

---

### Task 6: Add system tray with idle state

**Files:**
- Modify: `src-tauri/src/main.rs` (add tray setup)
- Create: `src-tauri/icons/icon.png` (tray icon)

- [ ] **Step 1: Create a minimal tray icon**

Generate a simple 32x32 PNG icon (or use a placeholder). For development, a solid-color square works:

```bash
# Create a simple 32x32 gray PNG icon using Python
python3 -c "
from PIL import Image
img = Image.new('RGBA', (32, 32), (128, 128, 128, 255))
img.save('src-tauri/icons/icon.png')
" 2>/dev/null || python3 -c "
# Fallback: create minimal valid PNG manually
import struct, zlib
def create_png(w, h, r, g, b, a=255):
    def chunk(ctype, data):
        c = ctype + data
        return struct.pack('>I', len(data)) + c + struct.pack('>I', zlib.crc32(c) & 0xffffffff)
    raw = b''
    for y in range(h):
        raw += b'\x00' + bytes([r, g, b, a]) * w
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(raw)) + chunk(b'IEND', b'')
with open('src-tauri/icons/icon.png', 'wb') as f:
    f.write(create_png(32, 32, 128, 128, 128))
"
```

- [ ] **Step 2: Update main.rs with tray icon**

Replace `src-tauri/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ipc;

use ipc::IpcMessage;
use std::sync::Mutex;

struct AppState {
    status: Mutex<String>,
}

fn main() {
    let mut ipc_rx = ipc::start_ipc_listener();

    tauri::Builder::default()
        .manage(AppState {
            status: Mutex::new("idle".to_string()),
        })
        .setup(move |app| {
            // System tray
            let quit = tauri::menu::MenuBuilder::new(app)
                .text("quit", "Quit")
                .build()?;

            let tray = tauri::tray::TrayIconBuilder::new()
                .icon(app.default_window_icon().cloned().unwrap())
                .tooltip("cc-helper")
                .menu(&quit)
                .on_menu_event(move |app, event| {
                    if event.id() == "quit" {
                        app.exit(0);
                    }
                })
                .build(app)?;

            // Store tray handle in app state for later icon updates
            app.manage(tray);

            // Handle incoming IPC messages
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                while let Some(msg) = ipc_rx.blocking_recv() {
                    match msg {
                        IpcMessage::Request { request, response_tx } => {
                            println!(
                                "[{}] {:?} ({})",
                                request.session_id, request.event, request.cwd
                            );

                            match request.event {
                                helper_protocol::HookEvent::SessionStart => {
                                    // TODO: register session, update tray status
                                    let state = handle.state::<AppState>();
                                    *state.status.lock().unwrap() = "working".to_string();
                                }
                                helper_protocol::HookEvent::Stop => {
                                    let state = handle.state::<AppState>();
                                    *state.status.lock().unwrap() = "idle".to_string();
                                }
                                _ => {}
                            }

                            if matches!(request.event, helper_protocol::HookEvent::PermissionRequest) {
                                let _ = response_tx.send(helper_protocol::Response::allow());
                            }
                        }
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Update tauri.conf.json to allow tray**

Ensure `src-tauri/tauri.conf.json` has tray permissions. Add to the `"app"` section if not present:

```json
{
  "app": {
    "windows": [],
    "trayIcon": {
      "iconPath": "icons/icon.png",
      "iconAsTemplate": false
    }
  }
}
```

- [ ] **Step 4: Build and verify tray appears**

Run: `cargo run`
Expected: System tray icon appears (gray square), right-click shows "Quit"

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: add system tray with idle state and quit menu"
```

---

## Chunk 3: Permission Dialog

### Task 7: Create PermissionDialog.vue component

**Files:**
- Create: `src/components/PermissionDialog.vue`
- Modify: `src/styles.css`

- [ ] **Step 1: Create the permission dialog component**

`src/components/PermissionDialog.vue`:
```vue
<template>
  <div class="permission-dialog" :class="platform">
    <div class="dialog-header">
      <span class="icon">&#x1F512;</span>
      <span class="title">Permission Request</span>
    </div>
    <div class="dialog-body">
      <div class="field">
        <span class="label">Tool</span>
        <span class="value">{{ toolName }}</span>
      </div>
      <div class="field" v-if="toolDetail">
        <span class="label">{{ detailLabel }}</span>
        <pre class="code">{{ toolDetail }}</pre>
      </div>
      <div class="field">
        <span class="label">Project</span>
        <span class="value">{{ projectName }}</span>
      </div>
    </div>
    <div class="dialog-footer">
      <button class="btn deny" @click="deny">Deny</button>
      <button class="btn allow" @click="allow">Allow</button>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'

const props = defineProps({
  toolName: { type: String, default: '' },
  toolInput: { type: Object, default: () => ({}) },
  projectName: { type: String, default: '' },
  platform: { type: String, default: 'linux' },
})

const emit = defineEmits(['allow', 'deny'])

const toolDetail = computed(() => {
  const input = props.toolInput
  switch (props.toolName) {
    case 'Bash':
      return input.command || ''
    case 'Edit':
      return input.file_path || ''
    case 'Write':
      return input.file_path || ''
    default:
      return JSON.stringify(input, null, 2).slice(0, 200)
  }
})

const detailLabel = computed(() => {
  switch (props.toolName) {
    case 'Bash': return 'Command'
    case 'Edit': return 'File'
    case 'Write': return 'File'
    default: return 'Input'
  }
})

function allow() { emit('allow') }
function deny() { emit('deny') }
</script>
```

- [ ] **Step 2: Add dialog styles**

`src/styles.css`:
```css
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  overflow: hidden;
  user-select: none;
}

.permission-dialog {
  padding: 20px;
  max-width: 480px;
}

.dialog-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 16px;
  font-size: 16px;
  font-weight: 600;
}

.dialog-body {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 20px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.label {
  font-size: 12px;
  color: #888;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.value {
  font-size: 14px;
  color: #333;
}

.code {
  font-family: 'SF Mono', 'Fira Code', monospace;
  font-size: 13px;
  background: #f5f5f5;
  border: 1px solid #e0e0e0;
  border-radius: 6px;
  padding: 8px 12px;
  max-height: 120px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-all;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}

.btn {
  padding: 8px 24px;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: opacity 0.15s;
}

.btn:hover {
  opacity: 0.85;
}

.btn.deny {
  background: #f5f5f5;
  color: #333;
}

.btn.allow {
  background: #22c55e;
  color: white;
}

/* Dark mode */
@media (prefers-color-scheme: dark) {
  .value { color: #e0e0e0; }
  .code {
    background: #2a2a2a;
    border-color: #404040;
    color: #e0e0e0;
  }
  .btn.deny {
    background: #404040;
    color: #e0e0e0;
  }
}
```

- [ ] **Step 3: Commit**

```bash
git add src/components/PermissionDialog.vue src/styles.css
git commit -m "feat: add PermissionDialog Vue component with tool-specific display"
```

---

### Task 8: Implement PermissionRequest dialog window

**Files:**
- Create: `src-tauri/src/dialog.rs`
- Modify: `src-tauri/src/main.rs` (wire dialog into PermissionRequest handler)

- [ ] **Step 0: Add urlencoding dependency to src-tauri/Cargo.toml**

```toml
urlencoding = "2"
```

Add this to `src-tauri/Cargo.toml` under `[dependencies]` before proceeding.

- [ ] **Step 1: Create dialog window manager**

`src-tauri/src/dialog.rs`:
```rust
use helper_protocol::Response;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// Manages permission dialog windows. Each pending PermissionRequest gets
/// a popup window. When the user clicks Allow/Deny, the response is sent
/// back through the stored oneshot channel.
pub struct DialogManager {
    /// Pending responses keyed by session_id + tool_use_id
    pending: Mutex<HashMap<String, tokio::sync::oneshot::Sender<Response>>>,
}

impl DialogManager {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
        }
    }

    /// Show a permission dialog for the given request. Returns the response
    /// when the user acts.
    pub async fn show_permission_dialog(
        &self,
        app: &AppHandle,
        request: &helper_protocol::Request,
        response_tx: tokio::sync::oneshot::Sender<Response>,
    ) {
        let tool_name = request.payload["tool_name"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        let tool_input = request.payload["tool_input"].clone();
        let project_name = std::path::Path::new(&request.cwd)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let session_id = request.session_id.clone();
        let key = format!(
            "{}_{}",
            session_id,
            request.payload["tool_use_id"].as_str().unwrap_or("unknown")
        );

        // Store the response channel
        {
            let mut pending = self.pending.lock().unwrap();
            pending.insert(key.clone(), response_tx);
        }

        // Create a new webview window for the dialog
        let window_label = format!("perm_{}", key);
        let url = format!(
            "/dialog.html?tool_name={}&project={}&key={}&input={}",
            urlencoding::encode(&tool_name),
            urlencoding::encode(&project_name),
            urlencoding::encode(&key),
            urlencoding::encode(&serde_json::to_string(&tool_input).unwrap_or_default()),
        );

        let _ = WebviewWindowBuilder::new(app, &window_label, WebviewUrl::App(url.into()))
            .title("Permission Request")
            .inner_size(500.0, 300.0)
            .center()
            .always_on_top(true)
            .resizable(false)
            .skip_taskbar(true)
            .build();
    }

    /// Called from the frontend when user clicks Allow/Deny.
    pub fn resolve(&self, key: &str, allowed: bool) {
        let mut pending = self.pending.lock().unwrap();
        if let Some(tx) = pending.remove(key) {
            let response = if allowed {
                Response::allow()
            } else {
                Response::deny("User denied this operation")
            };
            let _ = tx.send(response);
        }
    }
}
```

- [ ] **Step 2: Create dialog.html entry point**

`src-tauri/dialog.html` (at web root — configure vite to build it):
Create `src/dialog.html`:
```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Permission Request</title>
  <link rel="stylesheet" href="/src/styles.css" />
</head>
<body>
  <div id="app"></div>
  <script type="module" src="/src/dialog-main.js"></script>
</body>
</html>
```

`src/dialog-main.js`:
```javascript
import { createApp } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import PermissionDialog from './components/PermissionDialog.vue'

const params = new URLSearchParams(window.location.search)
const toolName = params.get('tool_name') || 'unknown'
const project = params.get('project') || ''
const key = params.get('key') || ''
const inputStr = params.get('input') || '{}'
let toolInput = {}
try { toolInput = JSON.parse(inputStr) } catch (e) {}

const platform = navigator.platform.toLowerCase().includes('mac') ? 'macos'
  : navigator.platform.toLowerCase().includes('win') ? 'windows'
  : 'linux'

const app = createApp(PermissionDialog, {
  toolName,
  toolInput,
  projectName: project,
  platform,
})

app.config.globalProperties.$key = key

app.mount('#app')

// Expose resolve function for Tauri invoke
window.__helperResolve = (allowed) => {
  invoke('resolve_permission', { key, allowed })
}
```

Update `PermissionDialog.vue` emits to call `window.__helperResolve`:

Add to `<script setup>` in `PermissionDialog.vue`:
```javascript
function allow() {
  if (window.__helperResolve) window.__helperResolve(true)
  emit('allow')
}
function deny() {
  if (window.__helperResolve) window.__helperResolve(false)
  emit('deny')
}
```

- [ ] **Step 3: Add Tauri command for resolve_permission**

Update `src-tauri/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod dialog;
mod ipc;

use dialog::DialogManager;
use ipc::IpcMessage;
use std::sync::Mutex;

struct AppState {
    status: Mutex<String>,
    dialog_manager: DialogManager,
}

#[tauri::command]
fn resolve_permission(app: tauri::AppHandle, key: String, allowed: bool) {
    let state = app.state::<AppState>();
    state.dialog_manager.resolve(&key, allowed);
    // Close the dialog window
    let label = format!("perm_{}", key);
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.close();
    }
}

fn main() {
    let mut ipc_rx = ipc::start_ipc_listener();

    tauri::Builder::default()
        .manage(AppState {
            status: Mutex::new("idle".to_string()),
            dialog_manager: DialogManager::new(),
        })
        .invoke_handler(tauri::generate_handler![resolve_permission])
        .setup(move |app| {
            // System tray
            let quit = tauri::menu::MenuBuilder::new(app)
                .text("quit", "Quit")
                .build()?;

            let icon = app.default_window_icon().cloned()
                .expect("Failed to load tray icon — ensure icons/icon.png exists in tauri.conf.json");
            let _tray = tauri::tray::TrayIconBuilder::new()
                .icon(icon)
                .tooltip("cc-helper")
                .menu(&quit)
                .on_menu_event(move |app, event| {
                    if event.id() == "quit" {
                        app.exit(0);
                    }
                })
                .build(app)?;

            // Handle IPC messages
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();

                rt.block_on(async move {
                    while let Some(msg) = ipc_rx.recv().await {
                        match msg {
                            IpcMessage::Request { request, response_tx } => {
                                println!("[{}] {:?}", request.session_id, request.event);

                                match request.event {
                                    helper_protocol::HookEvent::PermissionRequest => {
                                        let dialog_mgr = &handle.state::<AppState>().dialog_manager;
                                        dialog_mgr.show_permission_dialog(
                                            &handle,
                                            &request,
                                            response_tx,
                                        ).await;
                                    }
                                    helper_protocol::HookEvent::SessionStart => {
                                        let state = handle.state::<AppState>();
                                        *state.status.lock().unwrap() = "working".to_string();
                                        // response_tx dropped = no response needed for non-blocking
                                    }
                                    helper_protocol::HookEvent::Stop => {
                                        let state = handle.state::<AppState>();
                                        *state.status.lock().unwrap() = "idle".to_string();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                });
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Add `urlencoding` to `src-tauri/Cargo.toml`:
```toml
urlencoding = "2"
```
*(Note: This was already added in Step 0 above — skip if already present.)*

- [ ] **Step 4: Build and test**

Run: `cargo build`
Expected: Compiles

- [ ] **Step 5: Test permission dialog flow**

Terminal 1: `cargo run` (helper daemon starts)
Terminal 2:
```bash
echo '{"session_id":"test1","cwd":"/tmp/test","tool_name":"Bash","tool_input":{"command":"rm -rf /tmp/test"},"tool_use_id":"toolu_001"}' | ./target/debug/cc-helper-hook permission-request
```

Expected:
- A popup window appears showing "Bash / rm -rf /tmp/test"
- Click "Allow" → cc-helper-hook exits 0, stdout has allow JSON
- Click "Deny" → cc-helper-hook exits 0, stdout has deny JSON

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: implement permission dialog popup with Allow/Deny"
```

---

## Chunk 4: Session Management + Notifications

### Task 9: Implement session manager

**Files:**
- Create: `src-tauri/src/session.rs`
- Modify: `src-tauri/src/main.rs` (use SessionManager)

- [ ] **Step 1: Create session manager**

`src-tauri/src/session.rs`:
```rust
use helper_protocol::HookEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Working,
    WaitingPermission,
    Idle,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub cwd: String,
    pub project_name: String,
    pub status: SessionStatus,
    pub last_event: String,
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

        match event {
            HookEvent::SessionStart => {
                sessions.insert(
                    session_id.to_string(),
                    Session {
                        session_id: session_id.to_string(),
                        cwd: cwd.to_string(),
                        project_name,
                        status: SessionStatus::Working,
                        last_event: "SessionStart".to_string(),
                    },
                );
            }
            HookEvent::SessionEnd | HookEvent::Stop => {  // NOTE: SessionEnd is not currently emitted by Claude Code
                if let Some(s) = sessions.get_mut(session_id) {
                    s.status = SessionStatus::Idle;
                    s.last_event = format!("{:?}", event);
                }
            }
            HookEvent::StopFailure => {
                if let Some(s) = sessions.get_mut(session_id) {
                    s.status = SessionStatus::Failed;
                    s.last_event = "StopFailure".to_string();
                }
            }
            HookEvent::PermissionRequest => {
                if let Some(s) = sessions.get_mut(session_id) {
                    s.status = SessionStatus::WaitingPermission;
                    s.last_event = "PermissionRequest".to_string();
                }
            }
            HookEvent::PostToolUse => {
                // Tool completed but Claude continues working
                if let Some(s) = sessions.get_mut(session_id) {
                    s.status = SessionStatus::Working;
                    s.last_event = "PostToolUse".to_string();
                }
            }
            _ => {}
        }
    }

    pub fn get_sessions(&self) -> Vec<Session> {
        let sessions = self.sessions.lock().unwrap();
        sessions.values().cloned().collect()
    }

    /// Compute aggregate tray status from all sessions.
    pub fn aggregate_status(&self) -> SessionStatus {
        let sessions = self.sessions.lock().unwrap();
        if sessions.is_empty() {
            return SessionStatus::Idle;
        }
        if sessions.values().any(|s| matches!(s.status, SessionStatus::WaitingPermission)) {
            return SessionStatus::WaitingPermission;
        }
        if sessions.values().any(|s| matches!(s.status, SessionStatus::Failed)) {
            return SessionStatus::Failed;
        }
        if sessions.values().any(|s| matches!(s.status, SessionStatus::Working)) {
            return SessionStatus::Working;
        }
        SessionStatus::Idle
    }
}
```

- [ ] **Step 2: Add Tauri command to list sessions**

In `src-tauri/src/main.rs`, add:

```rust
use session::SessionManager;

#[tauri::command]
fn get_sessions(app: tauri::AppHandle) -> Vec<session::Session> {
    let state = app.state::<SessionManager>();
    state.get_sessions()
}
```

And register in `.invoke_handler`:
```rust
.invoke_handler(tauri::generate_handler![resolve_permission, get_sessions])
```

And manage SessionManager:
```rust
.manage(SessionManager::new())
```

Update the IPC handler to call `session_manager.handle_event()` for each event.

- [ ] **Step 3: Build**

Run: `cargo build`
Expected: Compiles

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/session.rs src-tauri/src/main.rs
git commit -m "feat: implement session manager with aggregate status tracking"
```

---

### Task 10: Multi-session tray display

**Files:**
- Create: `src/components/SessionPanel.vue`
- Modify: `src/App.vue`

- [ ] **Step 1: Create SessionPanel component**

`src/components/SessionPanel.vue`:
```vue
<template>
  <div class="session-panel">
    <div class="panel-header">Claude Code Sessions</div>
    <div v-if="sessions.length === 0" class="empty">No active sessions</div>
    <div
      v-for="session in sessions"
      :key="session.session_id"
      class="session-item"
    >
      <span class="status-dot" :class="statusClass(session.status)"></span>
      <span class="project">{{ session.project_name }}</span>
      <span class="status-text">{{ statusText(session.status) }}</span>
      <span class="sid">{{ session.session_id.slice(0, 6) }}</span>
    </div>
  </div>
</template>

<script setup>
defineProps({ sessions: { type: Array, default: () => [] } })

function statusClass(status) {
  return {
    working: status === 'Working',
    waiting: status === 'WaitingPermission',
    failed: status === 'Failed',
    idle: status === 'Idle',
  }
}

function statusText(status) {
  const map = { Working: 'working', WaitingPermission: 'perm', Failed: 'failed', Idle: 'idle' }
  return map[status] || status
}
</script>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/SessionPanel.vue
git commit -m "feat: add session panel component for tray display"
```

---

### Task 11: Desktop notifications for Stop/StopFailure

**Files:**
- Modify: `src-tauri/Cargo.toml` (add notification plugin)
- Modify: `src-tauri/src/main.rs` (send notifications)

- [ ] **Step 1: Add notification plugin**

Add to `src-tauri/Cargo.toml`:
```toml
tauri-plugin-notification = "2"
```

- [ ] **Step 2: Register notification plugin and send notifications**

In `main.rs`, add to builder chain:
```rust
.plugin(tauri_plugin_notification::init())
```

In the IPC handler for `Stop` and `StopFailure` events, add:
```rust
use tauri_plugin_notification::NotificationExt;

// In Stop handler:
let _ = app.notification()
    .builder()
    .title("Claude Code: Task Complete")
    .body(&format!("{} finished", project_name))
    .show();

// In StopFailure handler:
let _ = app.notification()
    .builder()
    .title("Claude Code: Task Failed")
    .body(&format!("{} encountered an error", project_name))
    .show();
```

- [ ] **Step 3: Build and test**

Run: `cargo build`
Expected: Compiles

- [ ] **Step 4: Test notifications**

Terminal 1: `cargo run`
Terminal 2:
```bash
echo '{"session_id":"test1","cwd":"/tmp/test","last_assistant_message":"Done"}' | ./target/debug/cc-helper-hook stop
echo '{"session_id":"test1","cwd":"/tmp/test","last_assistant_message":"Error: timeout"}' | ./target/debug/cc-helper-hook stop-failure
```

Expected: Desktop notifications appear for both events

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: add desktop notifications for Stop and StopFailure events"
```

---

### Task 12: Implement `helper install` command

**Files:**
- Modify: `crates/cc-helper-hook/src/main.rs` (add `install` subcommand)
- Create: `crates/cc-helper-hook/src/install.rs`

- [ ] **Step 1: Create install module**

`crates/cc-helper-hook/src/install.rs`:
```rust
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

/// Helper's hook definitions to merge into settings.json
fn helper_hooks() -> Value {
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
        "SessionEnd": [
            { "matcher": "", "hooks": [{ "type": "command", "command": "cc-helper-hook session-end" }] }  // NOTE: not currently emitted by Claude Code
        ]
    })
}

/// Marker to identify helper-managed hooks
const HELPER_MARKER: &str = "cc-helper-hook";

pub fn install() -> Result<(), String> {
    let home = home_dir().ok_or("Cannot determine home directory")?;
    let settings_path = home.join(SETTINGS_PATH);
    let helper_hooks = helper_hooks();

    // Read existing settings or create empty
    let mut settings: Value = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)
            .map_err(|e| format!("Cannot read settings: {}", e))?;
        serde_json::from_str(&content).unwrap_or(json!({}))
    } else {
        json!({})
    };

    // Get or create hooks object
    if settings.get("hooks").is_none() {
        settings["hooks"] = json!({});
    }

    // Merge helper hooks: append to existing arrays, skip duplicates
    for (event_name, new_entries) in helper_hooks.as_object().unwrap() {
        if settings["hooks"].get(event_name).is_none() {
            settings["hooks"][event_name] = json!([]);
        }

        let existing = settings["hooks"][event_name].as_array_mut().unwrap();
        for new_entry in new_entries.as_array().unwrap() {
            // Check if this event already has a cc-helper-hook entry
            let has_helper = existing.iter().any(|entry| {
                let hooks = entry.get("hooks");
                hooks.and_then(|h| h.as_array()).map_or(false, |arr| {
                    arr.iter().any(|h| {
                        h.get("command")
                            .and_then(|c| c.as_str())
                            .map_or(false, |c| c.contains(HELPER_MARKER))
                    })
                })
            });

            if !has_helper {
                existing.push(new_entry.clone());
            }
        }
    }

    // Write back
    let output = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Cannot serialize settings: {}", e))?;
    fs::write(&settings_path, output)
        .map_err(|e| format!("Cannot write settings: {}", e))?;

    println!("Helper hooks installed to {}", settings_path.display());
    Ok(())
}

pub fn uninstall() -> Result<(), String> {
    let home = home_dir().ok_or("Cannot determine home directory")?;
    let settings_path = home.join(SETTINGS_PATH);

    if !settings_path.exists() {
        println!("No settings file found, nothing to uninstall.");
        return Ok(());
    }

    let content = fs::read_to_string(&settings_path)
        .map_err(|e| format!("Cannot read settings: {}", e))?;
    let mut settings: Value = serde_json::from_str(&content).unwrap_or(json!({}));

    if let Some(hooks) = settings.get_mut("hooks").and_then(|h| h.as_object_mut()) {
        for (_event_name, entries) in hooks.iter_mut() {
            if let Some(arr) = entries.as_array_mut() {
                arr.retain(|entry| {
                    // Remove entries that contain cc-helper-hook in any of their hooks
                    let hooks_arr = entry.get("hooks");
                    hooks_arr.and_then(|h| h.as_array()).map_or(true, |h| {
                        !h.iter().any(|hook| {
                            hook.get("command")
                                .and_then(|c| c.as_str())
                                .map_or(false, |c| c.contains(HELPER_MARKER))
                        })
                    })
                });
            }
        }
    }

    let output = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Cannot serialize settings: {}", e))?;
    fs::write(&settings_path, output)
        .map_err(|e| format!("Cannot write settings: {}", e))?;

    println!("Helper hooks removed from {}", settings_path.display());
    Ok(())
}
```

- [ ] **Step 2: Update cc-helper-hook main to handle install/uninstall**

In `crates/cc-helper-hook/src/main.rs`, add to the args match:

```rust
Some("install") => {
    match install::install() {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            eprintln!("install failed: {}", e);
            std::process::exit(1);
        }
    }
}
Some("uninstall") => {
    match install::uninstall() {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            eprintln!("uninstall failed: {}", e);
            std::process::exit(1);
        }
    }
}
```

And add `mod install;` at the top.

- [ ] **Step 3: Test install**

Run: `cargo run -p cc-helper-hook -- install`
Expected: "Helper hooks installed to ~/.claude/settings.json"

Verify: `cat ~/.claude/settings.json | python3 -m json.tool | grep cc-helper-hook`
Expected: Multiple cc-helper-hook entries in hooks section

- [ ] **Step 4: Test uninstall**

Run: `cargo run -p cc-helper-hook -- uninstall`
Expected: "Helper hooks removed from ~/.claude/settings.json"

- [ ] **Step 5: Test idempotent install (run twice)**

Run: `cargo run -p cc-helper-hook -- install && cargo run -p cc-helper-hook -- install`
Expected: No duplicate entries

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: implement helper install/uninstall with settings.json merge"
```

---

### Task 13: End-to-end integration test

**Files:**
- N/A (manual test)

- [ ] **Step 1: Install helper hooks**

```bash
cargo run -p cc-helper-hook -- install
```

- [ ] **Step 2: Start helper daemon**

```bash
cargo run
```

- [ ] **Step 3: Start a Claude Code session in another terminal**

```bash
cd /tmp/test-project && claude
```

Expected behavior:
1. Claude starts → helper tray icon changes to "working"
2. Claude tries a tool needing permission → helper shows native dialog popup
3. User clicks Allow/Deny in dialog → Claude Code receives the decision
4. Claude finishes → helper tray icon returns to idle, desktop notification sent

- [ ] **Step 4: Test multi-session**

Open another terminal:
```bash
cd /tmp/test-project2 && claude
```

Expected: helper tray shows both sessions, badge shows count 2

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "chore: V0.1-V0.3 complete, integration test verified"
```
