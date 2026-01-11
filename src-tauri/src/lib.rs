// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::{
        Manager,
        WindowEvent,
        menu::{Menu, MenuItemBuilder},
        tray::{TrayIconEvent, MouseButton}, // TrayIconBuilder НЕ нужен
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
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