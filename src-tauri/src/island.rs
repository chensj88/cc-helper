use helper_protocol::Response;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

const COLLAPSED_W: f64 = 160.0;
const COLLAPSED_H: f64 = 36.0;

struct PendingRequest {
    response_tx: tokio::sync::oneshot::Sender<Response>,
    tool_input: serde_json::Value,
}

pub struct IslandManager {
    pending: Mutex<HashMap<String, PendingRequest>>,
    last_payload: Mutex<IslandStatusPayload>,
    content_bounds: Mutex<Option<(i32, i32, i32, i32)>>,
    force_interactive: AtomicBool,
    #[cfg(not(target_os = "linux"))]
    polling_active: AtomicBool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IslandStatusPayload {
    status: String,
    session_count: usize,
    sessions: Vec<SessionInfo>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub session_id: String,
    pub project_name: String,
    pub status: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct IslandRequestPayload {
    key: String,
    tool_name: String,
    tool_input: serde_json::Value,
    project_name: String,
}

impl IslandManager {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            last_payload: Mutex::new(IslandStatusPayload {
                status: "idle".to_string(),
                session_count: 0,
                sessions: vec![],
            }),
            content_bounds: Mutex::new(None),
            force_interactive: AtomicBool::new(false),
            #[cfg(not(target_os = "linux"))]
            polling_active: AtomicBool::new(false),
        }
    }

    pub fn has_pending(&self, key: &str) -> bool {
        self.pending.lock().unwrap().contains_key(key)
    }

    pub fn create_window(&self, app: &tauri::AppHandle) {
        // Don't create if already exists
        if app.get_webview_window("island").is_some() {
            return;
        }

        let (pos_x, pos_y) = Self::load_position();
        let top_center = Self::current_screen_top_center_position(app);
        let (x, y) = if pos_x < 0.0
            || pos_y < 0.0
            || !Self::is_on_any_monitor(app, pos_x, pos_y)
        {
            top_center
        } else {
            (pos_x, pos_y)
        };

        #[cfg(debug_assertions)]
        let url = "/src/island.html";
        #[cfg(not(debug_assertions))]
        let url = "/src/island.html";

        let builder = WebviewWindowBuilder::new(app, "island", WebviewUrl::App(url.into()))
            .title("cc-helper")
            .inner_size(COLLAPSED_W, COLLAPSED_H)
            .position(x, y)
            .transparent(true)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .visible(true);

        match builder.build() {
            Ok(_) => println!("[island] window created at ({}, {})", x, y),
            Err(e) => eprintln!("[island] failed to create window: {}", e),
        }
    }

    fn current_screen_top_center_position(app: &tauri::AppHandle) -> (f64, f64) {
        // First startup: use primary monitor, fallback to first available
        let monitor = app
            .primary_monitor()
            .ok()
            .flatten()
            .or_else(|| app.available_monitors().ok().and_then(|m| m.into_iter().next()));

        if let Some(monitor) = monitor {
            let size = monitor.size();
            let scale = monitor.scale_factor();
            let origin = monitor.position();
            let mx = origin.x as f64 / scale;
            let my = origin.y as f64 / scale;
            let mw = size.width as f64 / scale;
            let x = mx + (mw - COLLAPSED_W) / 2.0;
            println!(
                "[island] primary monitor: origin=({},{}), size={}, scale={}, computed x={}",
                origin.x, origin.y, size.width, scale, x
            );
            (x, my + 8.0)
        } else {
            println!("[island] no monitor found, using fallback");
            (400.0, 8.0)
        }
    }

    fn is_on_any_monitor(app: &tauri::AppHandle, x: f64, y: f64) -> bool {
        let Ok(monitors) = app.available_monitors() else {
            return false;
        };
        for monitor in monitors {
            let size = monitor.size();
            let scale = monitor.scale_factor();
            let origin = monitor.position();
            let left = origin.x as f64 / scale;
            let top = origin.y as f64 / scale;
            let right = left + size.width as f64 / scale;
            let bottom = top + size.height as f64 / scale;
            if x >= left && x < right && y >= top && y < bottom {
                return true;
            }
        }
        false
    }

    pub fn ensure_window(&self, app: &tauri::AppHandle) {
        if app.get_webview_window("island").is_none() {
            self.create_window(app);
        }
    }

    pub fn emit_status(
        &self,
        app: &tauri::AppHandle,
        status: &str,
        sessions: &[crate::session::Session],
    ) {
        self.ensure_window(app);
        let session_infos: Vec<SessionInfo> = sessions
            .iter()
            .map(|s| {
                let short_id: String = s.session_id.chars().take(6).collect();
                SessionInfo {
                    session_id: s.session_id.clone(),
                    project_name: format!("{} · {}", s.project_name, short_id),
                    status: format!("{:?}", s.status).to_lowercase(),
                }
            })
            .collect();
        let payload = IslandStatusPayload {
            status: status.to_string(),
            session_count: sessions.len(),
            sessions: session_infos,
        };
        *self.last_payload.lock().unwrap() = payload.clone();
        let _ = app.emit("island:update-status", &payload);
    }

    pub fn emit_request(
        &self,
        app: &tauri::AppHandle,
        request: &helper_protocol::Request,
        response_tx: tokio::sync::oneshot::Sender<Response>,
    ) {
        self.ensure_window(app);

        let tool_name = request.payload["tool_name"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        let tool_input = request.payload["tool_input"].clone();
        let project_name = std::path::Path::new(&request.cwd)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let counter = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let key = format!("{}_{}", request.session_id, counter);

        self.pending.lock().unwrap().insert(
            key.clone(),
            PendingRequest {
                response_tx,
                tool_input: tool_input.clone(),
            },
        );

        let payload = IslandRequestPayload {
            key,
            tool_name,
            tool_input,
            project_name,
        };
        let _ = app.emit("island:show-request", &payload);
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
        // Emit resolve to collapse the island UI
        let _ = app.emit("island:resolve", ());
    }

    fn position_file() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home)
            .join(".config")
            .join("cc-helper")
            .join("island-position.json")
    }

    fn load_position() -> (f64, f64) {
        let path = Self::position_file();
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(pos) = serde_json::from_str::<(f64, f64)>(&data) {
                return pos;
            }
        }
        (-1.0, -1.0) // sentinel: compute center
    }

    pub fn save_position(x: f64, y: f64) {
        let path = Self::position_file();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, serde_json::to_string(&(x, y)).unwrap_or_default());
    }
}

#[tauri::command]
pub fn island_resize(app: tauri::AppHandle, width: f64, height: f64) {
    if let Some(win) = app.get_webview_window("island") {
        let _ = win.set_size(tauri::LogicalSize::new(width, height));
    }
}

#[tauri::command]
pub fn island_set_input_region(app: tauri::AppHandle, x: i32, y: i32, width: i32, height: i32) {
    let mgr = app.state::<IslandManager>();

    if width > 0 && height > 0 {
        *mgr.content_bounds.lock().unwrap() = Some((x, y, width, height));
    } else {
        *mgr.content_bounds.lock().unwrap() = None;
    }

    #[cfg(target_os = "linux")]
    if !mgr.force_interactive.load(Ordering::Acquire) {
        use gtk::prelude::*;
        let Some(win) = app.get_webview_window("island") else { return };
        let Ok(gtk_win) = win.gtk_window() else { return };

        if width > 0 && height > 0 {
            let scale = gtk_win.scale_factor();
            let rect = cairo::RectangleInt::new(
                x * scale,
                y * scale,
                width * scale,
                height * scale,
            );
            let region = cairo::Region::create_rectangle(&rect);
            if let Some(gdk_win) = gtk_win.window() {
                gdk_win.input_shape_combine_region(&region, 0, 0);
            }
        } else {
            gtk_win.input_shape_combine_region(None);
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        mgr.ensure_cursor_polling(&app);
    }
}

#[tauri::command]
pub fn island_set_interactive(app: tauri::AppHandle, interactive: bool) {
    let mgr = app.state::<IslandManager>();
    mgr.force_interactive.store(interactive, Ordering::Release);

    if interactive {
        #[cfg(not(target_os = "linux"))]
        {
            if let Some(win) = app.get_webview_window("island") {
                let _ = win.set_ignore_cursor_events(false);
            }
        }

        #[cfg(target_os = "linux")]
        {
            use gtk::prelude::*;
            if let Some(win) = app.get_webview_window("island") {
                if let Ok(gtk_win) = win.gtk_window() {
                    gtk_win.input_shape_combine_region(None);
                }
            }
        }
    } else {
        let bounds = mgr.content_bounds.lock().unwrap().clone();
        if let Some((x, y, w, h)) = bounds {
            island_set_input_region(app, x, y, w, h);
        }
    }
}

#[cfg(not(target_os = "linux"))]
impl IslandManager {
    fn ensure_cursor_polling(&self, app: &tauri::AppHandle) {
        if self.polling_active.swap(true, Ordering::AcqRel) {
            return;
        }
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let mut was_ignoring = false;

            loop {
                tokio::time::sleep(std::time::Duration::from_millis(40)).await;

                let Some(win) = app.get_webview_window("island") else {
                    break;
                };

                let mgr = app.state::<IslandManager>();

                if mgr.force_interactive.load(Ordering::Acquire) {
                    if was_ignoring {
                        let _ = win.set_ignore_cursor_events(false);
                        was_ignoring = false;
                    }
                    continue;
                }

                let bounds = mgr.content_bounds.lock().unwrap().clone();
                let Some((bx, by, bw, bh)) = bounds else {
                    if was_ignoring {
                        let _ = win.set_ignore_cursor_events(false);
                        was_ignoring = false;
                    }
                    continue;
                };

                let Ok(cursor) = app.cursor_position() else { continue };
                let Ok(win_pos) = win.outer_position() else { continue };
                let scale = win.scale_factor().unwrap_or(1.0);

                let rel_x = cursor.x - win_pos.x as f64;
                let rel_y = cursor.y - win_pos.y as f64;

                let cx = bx as f64 * scale;
                let cy = by as f64 * scale;
                let cw = bw as f64 * scale;
                let ch = bh as f64 * scale;

                let in_content =
                    rel_x >= cx && rel_x < cx + cw && rel_y >= cy && rel_y < cy + ch;

                let should_ignore = !in_content;
                if should_ignore != was_ignoring {
                    let _ = win.set_ignore_cursor_events(should_ignore);
                    was_ignoring = should_ignore;
                }
            }

            let mgr = app.state::<IslandManager>();
            mgr.polling_active.store(false, Ordering::Release);
        });
    }
}

#[tauri::command]
pub fn island_save_position(x: f64, y: f64) {
    IslandManager::save_position(x, y);
}

#[tauri::command]
pub fn island_ensure_visible(app: tauri::AppHandle, width: f64, height: f64) {
    if let Some(win) = app.get_webview_window("island") {
        let Ok(pos) = win.outer_position() else { return };
        let Ok(scale) = win.scale_factor() else { return };
        let logical_x = pos.x as f64 / scale;
        let logical_y = pos.y as f64 / scale;

        let Ok(Some(monitor)) = win.current_monitor() else { return };
        let m_size = monitor.size();
        let m_origin = monitor.position();
        let m_scale = monitor.scale_factor();
        let mx = m_origin.x as f64 / m_scale;
        let my = m_origin.y as f64 / m_scale;
        let mw = m_size.width as f64 / m_scale;
        let mh = m_size.height as f64 / m_scale;

        let mut new_x = logical_x;
        let mut new_y = logical_y;

        // Ensure right edge doesn't go off screen
        if new_x + width > mx + mw {
            new_x = mx + mw - width - 8.0;
        }
        // Ensure left edge doesn't go off screen
        if new_x < mx {
            new_x = mx + 8.0;
        }
        // Ensure bottom edge doesn't go off screen
        if new_y + height > my + mh {
            new_y = my + mh - height - 8.0;
        }
        // Ensure top edge doesn't go off screen
        if new_y < my {
            new_y = my + 8.0;
        }

        if (new_x - logical_x).abs() > 0.5 || (new_y - logical_y).abs() > 0.5 {
            let _ = win.set_position(tauri::LogicalPosition::new(new_x, new_y));
            println!("[island] repositioned to ({}, {})", new_x, new_y);
        }
    }
}

#[tauri::command]
pub fn get_island_status(app: tauri::AppHandle) -> IslandStatusPayload {
    let mgr = app.state::<IslandManager>();
    let payload = mgr.last_payload.lock().unwrap().clone();
    payload
}
