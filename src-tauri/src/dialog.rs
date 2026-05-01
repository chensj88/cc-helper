use helper_protocol::Response;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

static DIALOG_COUNTER: AtomicU64 = AtomicU64::new(0);

struct PendingRequest {
    response_tx: tokio::sync::oneshot::Sender<Response>,
    tool_input: serde_json::Value,
}

pub struct DialogManager {
    pending: Mutex<HashMap<String, PendingRequest>>,
}

impl DialogManager {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
        }
    }

    pub fn show_permission_dialog(
        &self,
        app: &tauri::AppHandle,
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

        let counter = DIALOG_COUNTER.fetch_add(1, Ordering::Relaxed);
        let key = format!("{}_{}", request.session_id, counter);

        {
            self.pending.lock().unwrap().insert(
                key.clone(),
                PendingRequest {
                    response_tx,
                    tool_input: tool_input.clone(),
                },
            );
        }

        let window_label = format!("perm_{}", key);

        // Dynamic window size based on tool type
        let (w, h) = if tool_name == "ask_user_question" {
            (600.0, 500.0)
        } else {
            (500.0, 300.0)
        };

        #[cfg(debug_assertions)]
        let url = format!(
            "/src/dialog.html?tool_name={}&project={}&key={}&input={}",
            urlencoding::encode(&tool_name),
            urlencoding::encode(&project_name),
            urlencoding::encode(&key),
            urlencoding::encode(&serde_json::to_string(&tool_input).unwrap_or_default()),
        );
        #[cfg(not(debug_assertions))]
        let url = format!(
            "/src/dialog.html?tool_name={}&project={}&key={}&input={}",
            urlencoding::encode(&tool_name),
            urlencoding::encode(&project_name),
            urlencoding::encode(&key),
            urlencoding::encode(&serde_json::to_string(&tool_input).unwrap_or_default()),
        );

        match WebviewWindowBuilder::new(app, &window_label, WebviewUrl::App(url.into()))
            .title("Permission Request")
            .inner_size(w, h)
            .center()
            .always_on_top(true)
            .resizable(false)
            .skip_taskbar(true)
            .build()
        {
            Ok(_) => {
                // Destroy old hidden windows
                let labels_to_cleanup: Vec<String> = app
                    .webview_windows()
                    .keys()
                    .filter(|l| l.starts_with("perm_") && l.as_str() != window_label)
                    .cloned()
                    .collect();
                for label in labels_to_cleanup {
                    if let Some(win) = app.get_webview_window(&label) {
                        let _ = win.destroy();
                    }
                }
            }
            Err(e) => eprintln!("[dialog] failed to create window {}: {}", window_label, e),
        }
    }

    pub fn resolve(
        &self,
        app: &tauri::AppHandle,
        key: &str,
        allowed: bool,
        answers: Option<serde_json::Value>,
    ) {
        if let Some(pending) = self.pending.lock().unwrap().remove(key) {
            let response = if allowed {
                if let Some(answers) = answers {
                    Response::allow_with_input(&pending.tool_input, &answers)
                } else {
                    Response::allow()
                }
            } else {
                Response::deny("User denied")
            };
            let _ = pending.response_tx.send(response);
        }
        // Close (will be intercepted and hidden by on_window_event)
        let label = format!("perm_{}", key);
        if let Some(win) = app.get_webview_window(&label) {
            let _ = win.close();
        }
    }
}
