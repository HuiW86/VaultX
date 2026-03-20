mod commands;
pub mod crypto;
pub mod db;
mod state;

use std::sync::Mutex;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

const QUICK_ACCESS_LABEL: &str = "quickaccess";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_dir = db::connection::data_dir().expect("Cannot determine data directory");

    tauri::Builder::default()
        .manage(Mutex::new(state::AppState::new(data_dir)))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    let _ = shortcut; // We only register one shortcut
                    if let tauri_plugin_global_shortcut::ShortcutState::Pressed = event.state {
                        toggle_quick_access(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Register Cmd+Shift+Space
            use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
            let shortcut = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Space);
            app.global_shortcut().register(shortcut)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Auth
            commands::auth::get_app_status,
            commands::auth::setup_vault,
            commands::auth::unlock,
            commands::auth::lock,
            // Entries
            commands::entries::create_entry,
            commands::entries::get_entry,
            commands::entries::list_entries,
            commands::entries::update_entry,
            commands::entries::trash_entry,
            commands::entries::get_category_counts,
            commands::entries::generate_password,
            commands::entries::copy_to_clipboard,
            commands::entries::list_vaults,
            // Search
            commands::search::search_entries,
            commands::search::recent_entries,
        ])
        .on_window_event(|window, event| {
            // Auto-hide Quick Access when it loses focus
            if window.label() == QUICK_ACCESS_LABEL {
                if let tauri::WindowEvent::Focused(false) = event {
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn toggle_quick_access(handle: &tauri::AppHandle) {
    if let Some(window) = handle.get_webview_window(QUICK_ACCESS_LABEL) {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
            let _ = window.emit("quickaccess:show", ());
        }
    } else {
        let url = if cfg!(debug_assertions) {
            WebviewUrl::External("http://localhost:5173/quick-access.html".parse().unwrap())
        } else {
            WebviewUrl::App("quick-access.html".into())
        };

        let builder = WebviewWindowBuilder::new(handle, QUICK_ACCESS_LABEL, url)
            .title("Quick Access")
            .inner_size(680.0, 420.0)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .center()
            .skip_taskbar(true)
            .visible(true)
            .focused(true);

        match builder.build() {
            Ok(_) => {}
            Err(e) => log::error!("Failed to create Quick Access window: {e}"),
        }
    }
}
