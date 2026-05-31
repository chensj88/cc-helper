//! Linux backend selection module
//!
//! Intelligently chooses between Wayland and XWayland based on:
//! 1. Wayland availability
//! 2. Window manager compatibility detection
//! 3. User preference via environment variable
//!
//! Strategy: Try Wayland first, fall back to XWayland if needed

#[cfg(target_os = "linux")]
use std::env;

/// Display server type
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DisplayServer {
    Wayland,
    XWayland,
    X11,
    Unknown,
}

/// Detect if Wayland is available
#[cfg(target_os = "linux")]
fn is_wayland_available() -> bool {
    // Check WAYLAND_DISPLAY environment variable
    if env::var("WAYLAND_DISPLAY").is_ok() {
        return true;
    }

    // Check if wayland-0 or wayland-1 exists in XDG_RUNTIME_DIR
    if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
        let wayland_0 = std::path::PathBuf::from(&runtime_dir).join("wayland-0");
        let wayland_1 = std::path::PathBuf::from(&runtime_dir).join("wayland-1");
        if wayland_0.exists() || wayland_1.exists() {
            return true;
        }
    }

    false
}

/// Check if we're already running under XWayland
#[cfg(target_os = "linux")]
fn is_running_under_xwayland() -> bool {
    // Check if GDK_BACKEND is already set to x11
    if env::var("GDK_BACKEND").as_deref() == Ok(&"x11".to_string()) {
        return true;
    }

    false
}

/// Detect the current Wayland compositor
#[cfg(target_os = "linux")]
fn detect_wayland_compositor() -> Option<String> {
    // Try to get compositor from XDG_CURRENT_DESKTOP
    if let Ok(desktop) = env::var("XDG_CURRENT_DESKTOP") {
        for name in desktop.split(':') {
            match name.to_lowercase().as_str() {
                "gnome" => return Some("gnome".to_string()),
                "kde" => return Some("kde".to_string()),
                "sway" => return Some("sway".to_string()),
                "hyprland" => return Some("hyprland".to_string()),
                "wlroots" => return Some("wlroots".to_string()),
                _ => continue,
            }
        }
    }

    // Try to detect from process
    use std::process::Command;
    if let Ok(output) = Command::new("ps")
        .args(&["-e", "-o", "comm="])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let comm = line.trim();
            match comm {
                "gnome-shell" => return Some("gnome".to_string()),
                "plasmashell" => return Some("kde".to_string()),
                "sway" => return Some("sway".to_string()),
                "Hyprland" => return Some("hyprland".to_string()),
                _ => continue,
            }
        }
    }

    None
}

/// Check if compositor is known to have poor always_on_top support on Wayland
#[cfg(target_os = "linux")]
fn has_poor_wayland_support(compositor: &str) -> bool {
    // GNOME on Wayland (Mutter) has poor always_on_top support
    // KDE on Wayland (KWin) has better support but may still have issues
    matches!(compositor, "gnome")
}

/// Check if user has explicitly set preference via environment variable
#[cfg(target_os = "linux")]
fn get_user_preference() -> Option<bool> {
    // CC_HELPER_BACKEND=x11 to force X11, wayland to force Wayland
    if let Ok(backend) = env::var("CC_HELPER_BACKEND") {
        return Some(backend.to_lowercase() == "x11");
    }

    None
}

/// Get the current display server type
#[cfg(target_os = "linux")]
pub fn get_display_server_type() -> DisplayServer {
    if is_running_under_xwayland() {
        return DisplayServer::XWayland;
    }

    if is_wayland_available() {
        return DisplayServer::Wayland;
    }

    if env::var("DISPLAY").is_ok() {
        return DisplayServer::X11;
    }

    DisplayServer::Unknown
}

/// Select the best backend for the current environment
/// Returns true if XWayland should be forced
#[cfg(target_os = "linux")]
pub fn select_backend() -> bool {
    // Check user preference first
    if let Some(force_x11) = get_user_preference() {
        if force_x11 {
            println!("[backend] User preference: forcing X11 backend");
            env::set_var("GDK_BACKEND", "x11");
            return true;
        } else {
            println!("[backend] User preference: forcing Wayland backend");
            return false;
        }
    }

    let display_server = get_display_server_type();

    match display_server {
        DisplayServer::Wayland => {
            // Detect compositor
            if let Some(compositor) = detect_wayland_compositor() {
                println!("[backend] Wayland compositor detected: {}", compositor);

                if has_poor_wayland_support(&compositor) {
                    println!("[backend] {} has poor Wayland support, falling back to X11", compositor);
                    env::set_var("GDK_BACKEND", "x11");
                    return true;
                }

                println!("[backend] {} has good Wayland support, using native Wayland", compositor);
                return false;
            }

            // Unknown compositor, try Wayland native first
            println!("[backend] Unknown Wayland compositor, trying native Wayland (set CC_HELPER_BACKEND=x11 if issues occur)");
            false
        }
        DisplayServer::XWayland => {
            println!("[backend] Already using XWayland");
            true
        }
        DisplayServer::X11 => {
            println!("[backend] X11 session detected, using X11 backend");
            false
        }
        DisplayServer::Unknown => {
            println!("[backend] Unknown display server, defaulting to X11");
            env::set_var("GDK_BACKEND", "x11");
            true
        }
    }
}

/// Initialize backend selection
/// Should be called early in app startup
#[cfg(target_os = "linux")]
pub fn init_backend() {
    select_backend();
}

/// No-op for non-Linux platforms
#[cfg(not(target_os = "linux"))]
pub fn init_backend() {
    // No backend selection needed on other platforms
}
