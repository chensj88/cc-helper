// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod install;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("--install") | Some("-i") => {
            let hook_path = install::find_hook_binary();
            match install::install_with_path(&hook_path) {
                Ok(()) => {}
                Err(e) => eprintln!("{}", e),
            }
            return;
        }
        Some("--uninstall") | Some("-u") => {
            match install::uninstall() {
                Ok(()) => {}
                Err(e) => eprintln!("{}", e),
            }
            return;
        }
        _ => {}
    }

    cc_helper_lib::run()
}
