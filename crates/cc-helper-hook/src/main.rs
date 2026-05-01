mod install;

use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::process::exit;
use std::time::Duration;

fn main() {
    // Collect args early to check for install/uninstall subcommands.
    let args: Vec<String> = std::env::args().collect();

    // Handle install/uninstall subcommands (no stdin needed).
    match args.get(1).map(|s| s.as_str()) {
        Some("install") => {
            match install::install() {
                Ok(()) => exit(0),
                Err(e) => {
                    eprintln!("{}", e);
                    exit(1);
                }
            }
        }
        Some("uninstall") => {
            match install::uninstall() {
                Ok(()) => exit(0),
                Err(e) => {
                    eprintln!("{}", e);
                    exit(1);
                }
            }
        }
        _ => {}
    }

    // Read stdin to String. Exit 0 on error or empty — never block Claude Code.
    let mut stdin_str = String::new();
    if std::io::stdin().read_to_string(&mut stdin_str).is_err() || stdin_str.trim().is_empty() {
        exit(0);
    }
    let hook_event = match args.get(1) {
        Some(arg) => match helper_protocol::HookEvent::from_cli_arg(arg) {
            Some(e) => e,
            None => exit(0),
        },
        None => exit(0),
    };

    // Extract session_id and cwd from stdin JSON.
    let stdin_json: serde_json::Value = match serde_json::from_str(&stdin_str) {
        Ok(v) => v,
        Err(_) => exit(0),
    };

    let session_id = stdin_json
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let cwd = stdin_json
        .get("cwd")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Build the protocol Request.
    let request = helper_protocol::Request {
        version: helper_protocol::PROTOCOL_VERSION,
        event: hook_event,
        session_id,
        cwd,
        payload: stdin_json.clone(),
    };

    let request_json = match serde_json::to_string(&request) {
        Ok(j) => j,
        Err(_) => exit(0),
    };

    // Connect to Unix socket. Exit 0 if fails — daemon not running.
    let socket_path = "/tmp/cc-helper.sock";
    let mut stream = match UnixStream::connect(socket_path) {
        Ok(s) => s,
        Err(_) => exit(0),
    };

    // Set timeouts.
    if stream
        .set_read_timeout(Some(Duration::from_secs(600)))
        .is_err()
    {
        exit(0);
    }
    if stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .is_err()
    {
        exit(0);
    }

    // Send request as JSON + newline.
    let wire = format!("{}\n", request_json);
    if stream.write_all(wire.as_bytes()).is_err() {
        exit(0);
    }
    if stream.flush().is_err() {
        exit(0);
    }

    // Read response byte-by-byte until newline.
    let mut response_bytes = Vec::new();
    let mut buf = [0u8; 1];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break, // EOF
            Ok(n) => {
                // buf is 1 byte, so n is 1 in practice, but handle any size
                let mut found_newline = false;
                for &byte in &buf[..n] {
                    if byte == b'\n' {
                        found_newline = true;
                        break;
                    }
                    response_bytes.push(byte);
                }
                if found_newline {
                    break;
                }
            }
            Err(_) => exit(0),
        }
    }

    // Shutdown the write side.
    let _ = stream.shutdown(Shutdown::Both);

    // Parse response JSON.
    let response_str = match String::from_utf8(response_bytes) {
        Ok(s) => s,
        Err(_) => exit(0),
    };

    let response: serde_json::Value = match serde_json::from_str(&response_str) {
        Ok(v) => v,
        Err(_) => exit(0),
    };

    // Output the hookSpecificOutput wrapped in the format Claude Code expects
    if let Some(output) = response.get("hookSpecificOutput") {
        let wrapped = serde_json::json!({
            "hookSpecificOutput": output
        });
        if let Ok(output_str) = serde_json::to_string(&wrapped) {
            println!("{}", output_str);
        }
    }

    exit(0);
}
