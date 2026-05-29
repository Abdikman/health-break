mod handler;
mod state;

// Импортируем типы, чтобы использовать их ниже
use state::{AppState, TimerState};
use std::sync::Mutex;
use handler::commands;
use handler::notification;
use handler::audio;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::{
        Manager,
        WindowEvent,
        menu::{Menu, MenuItemBuilder},
        tray::{TrayIconEvent, MouseButton},
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        
        // Инициализируем состояние
        .manage(AppState(Mutex::new(TimerState {
            is_running: false,
            duration_secs: 30 * 60,
            running_flag: None, // Изначально потока нет
        })))

        .invoke_handler(tauri::generate_handler![
            commands::start_timer,
            commands::stop_timer,
            commands::get_exercises,
            commands::set_duration,

            notification::show_notification,
            notification::notification_clicked,
            notification::close_notification,

            audio::get_sounds,
            audio::open_sounds_folder
        ])
        
        .setup(|app| {
            // создаём только МЕНЮ, сам трей создаст Tauri из конфига
            let quit = MenuItemBuilder::with_id("quit", "Выход").build(app)?;
            let menu = Menu::with_items(app, &[&quit])?;
            
            // находим трей, который создался из конфига, и ставим ему меню
            if let Some(tray) = app.tray_by_id("main") {
                let _ = tray.set_menu(Some(menu));
            }
            Ok(())
        })
        // глобальный обработчик событий трея (для того трея, что в конфиге)
        .on_tray_icon_event(|app, event| {
             if let TrayIconEvent::Click { button, .. } = event {
                 if let MouseButton::Left = button {
                     if let Some(win) = app.get_webview_window("main") {
                         let _ = win.show();
                         let _ = win.set_focus();
                     }
                 }
             }
        })
        // глобальный обработчик меню
        .on_menu_event(|app, event| {
            if event.id() == "quit" {
                app.exit(0);
            }
        })
        .on_window_event(|app, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}