mod dialog;
mod install;
mod ipc;
mod island;
mod session;

use dialog::DialogManager;
use ipc::IpcMessage;
use island::IslandManager;
use session::SessionManager;
use std::sync::Mutex;
use tauri::Manager;
use tauri_plugin_notification::NotificationExt;

struct AppState {
    status: Mutex<String>,
}

#[tauri::command]
fn resolve_permission(
    app: tauri::AppHandle,
    key: String,
    allowed: bool,
    always: Option<bool>,
    answers: Option<serde_json::Value>,
) {
    let always = always.unwrap_or(false);
    // Try island first, fallback to dialog manager
    let island_mgr = app.state::<IslandManager>();
    if island_mgr.has_pending(&key) {
        island_mgr.resolve(&app, &key, allowed, always, answers);
    } else {
        let state = app.state::<DialogManager>();
        state.resolve(&app, &key, allowed, always, answers);
    }
}

#[tauri::command]
fn get_sessions(app: tauri::AppHandle) -> Vec<session::Session> {
    app.state::<SessionManager>().get_sessions()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut ipc_rx = ipc::start_ipc_listener();

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .manage(AppState {
            status: Mutex::new("idle".to_string()),
        })
        .manage(DialogManager::new())
        .manage(IslandManager::new())
        .manage(SessionManager::new())
        .invoke_handler(tauri::generate_handler![
            resolve_permission,
            get_sessions,
            island::island_resize,
            island::island_save_position,
            island::island_ensure_visible,
            island::island_set_input_region,
            island::island_set_interactive,
            island::get_island_status,
        ])
        .setup(move |app| {
            let menu = tauri::menu::MenuBuilder::new(app)
                .text("install", "Install Hooks")
                .text("uninstall", "Uninstall Hooks")
                .separator()
                .text("quit", "Quit")
                .build()?;

            let icon = app
                .default_window_icon()
                .cloned()
                .expect("Failed to load tray icon");

            let _tray = tauri::tray::TrayIconBuilder::new()
                .icon(icon)
                .tooltip("cc-helper")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "install" => {
                            let hook_path = install::find_hook_binary();
                            match install::install_with_path(&hook_path) {
                                Ok(()) => {
                                    let _ = app.notification()
                                        .builder()
                                        .title("cc-helper")
                                        .body("Hooks installed successfully")
                                        .show();
                                }
                                Err(e) => {
                                    let _ = app.notification()
                                        .builder()
                                        .title("cc-helper")
                                        .body(&format!("Install failed: {}", e))
                                        .show();
                                }
                            }
                        }
                        "uninstall" => {
                            match install::uninstall() {
                                Ok(()) => {
                                    let _ = app.notification()
                                        .builder()
                                        .title("cc-helper")
                                        .body("Hooks uninstalled")
                                        .show();
                                }
                                Err(e) => {
                                    let _ = app.notification()
                                        .builder()
                                        .title("cc-helper")
                                        .body(&format!("Uninstall failed: {}", e))
                                        .show();
                                }
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // Create the Dynamic Island window
            let island_mgr = app.state::<IslandManager>();
            island_mgr.create_window(&app.handle().clone());

            let handle = app.handle().clone();
            let handle2 = app.handle().clone();

            // Periodic staleness check — degrade sessions stuck in Working/WaitingPermission
            // when no hook events arrive (user exited Claude Code without triggering Stop).
            // Only emit UI when a degradation actually occurred, avoiding unconditional refresh.
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                    let session_mgr = handle.state::<SessionManager>();
                    if session_mgr.tick_cleanup() {
                        let agg_status = session_mgr.aggregate_status();
                        let sessions = session_mgr.get_sessions();
                        let status_str = match agg_status {
                            session::SessionStatus::Working => "working",
                            session::SessionStatus::WaitingPermission => "permission",
                            session::SessionStatus::Failed => "failed",
                            session::SessionStatus::Idle => "idle",
                        };
                        let island_mgr = handle.state::<IslandManager>();
                        island_mgr.emit_status(&handle, status_str, &sessions);
                    }
                }
            });

            std::thread::spawn(move || {
                while let Some(msg) = ipc_rx.blocking_recv() {
                    let IpcMessage::Request {
                        request,
                        response_tx,
                    } = msg;
                    {
                        let project_name = std::path::Path::new(&request.cwd)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();

                        // Update session manager for ALL events
                        let session_mgr = handle2.state::<SessionManager>();
                        session_mgr.handle_event(&request.event, &request.session_id, &request.cwd);

                        // Compute aggregate status for island
                        let agg_status = session_mgr.aggregate_status();
                        let sessions = session_mgr.get_sessions();
                        let status_str = match agg_status {
                            session::SessionStatus::Working => "working",
                            session::SessionStatus::WaitingPermission => "permission",
                            session::SessionStatus::Failed => "failed",
                            session::SessionStatus::Idle => "idle",
                        };

                        // Update island status on every event
                        let island_mgr = handle2.state::<IslandManager>();
                        island_mgr.emit_status(&handle2, status_str, &sessions);

                        // Legacy status tracking
                        match request.event {
                            helper_protocol::HookEvent::SessionStart => {
                                let state = handle2.state::<AppState>();
                                *state.status.lock().unwrap() = "working".to_string();
                            }
                            helper_protocol::HookEvent::Stop => {
                                let state = handle2.state::<AppState>();
                                *state.status.lock().unwrap() = "idle".to_string();
                                let _ = handle2.notification()
                                    .builder()
                                    .title("Claude Code: Task Complete")
                                    .body(&format!("{} finished", project_name))
                                    .show();
                            }
                            helper_protocol::HookEvent::StopFailure => {
                                let _ = handle2.notification()
                                    .builder()
                                    .title("Claude Code: Task Failed")
                                    .body(&format!("{} encountered an error", project_name))
                                    .show();
                            }
                            _ => {}
                        }

                        if matches!(
                            request.event,
                            helper_protocol::HookEvent::PermissionRequest
                        ) {
                            // Route through island (replaces dialog popup)
                            let island_mgr = handle2.state::<IslandManager>();
                            island_mgr.emit_request(&handle2, &request, response_tx);
                            continue;
                        }
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let label = window.label();
                if label == "island" || label.starts_with("perm_") {
                    api.prevent_close();
                    if label.starts_with("perm_") {
                        let _ = window.hide();
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
