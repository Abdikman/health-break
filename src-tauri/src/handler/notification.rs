use tauri::{AppHandle, Manager, PhysicalPosition, Emitter};

// Эту функцию мы зовем из Rust (из потока таймера)
pub fn show_notification_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("notification") {
        if let Ok(monitor) = window.current_monitor() {
            if let Some(m) = monitor {
                let screen_size = m.size();
                // Если не удалось получить размер окна, берем дефолт 320x140
                let window_size = window.outer_size().unwrap_or(tauri::PhysicalSize::new(320, 110));
                
                // Считаем координаты (справа снизу)
                let x = screen_size.width as i32 - window_size.width as i32;
                let y = screen_size.height as i32 - window_size.height as i32 - 60; 

                let _ = window.set_position(PhysicalPosition::new(x, y));
                let _ = window.show();
                let _ = window.set_focus();

                let _ = window.emit("notification-shown", ());
            }
        }
    }
}

// Эту команду мы зовем из JS (кнопка OK в notification.html)
#[tauri::command]
pub fn close_notification(app: AppHandle) {
    if let Some(window) = app.get_webview_window("notification") {
        let _ = window.hide();

        let _ = window.navigate(tauri::Url::parse("tauri://localhost/notification.html").unwrap());
    }
}

// Если вдруг захотим тестить показ окна из JS по кнопке
#[tauri::command]
pub fn show_notification(app: AppHandle) {
    show_notification_window(&app);
}

#[tauri::command]
pub fn notification_clicked(app: AppHandle) {
    // 1. Скрываем уведомление
    if let Some(win) = app.get_webview_window("notification") {
        let _ = win.hide();
    }

    // 2. Показываем главное окно
    if let Some(main_win) = app.get_webview_window("main") {
        let _ = main_win.show();
        let _ = main_win.set_focus();
        
        // 3. Шлём событие фронтенду главного окна: "перейди в каталог"
        // (если хочешь сразу открывать каталог)
        let _ = main_win.emit("navigate-to-catalog", ());
    }
}
