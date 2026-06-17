use tauri::{AppHandle, State, Emitter};
use std::thread;
use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::state::AppState;
use super::notification::show_notification_window;


#[tauri::command]
pub fn start_timer(app: AppHandle, state: State<AppState>) -> String {
    let mut state_guard = state.0.lock().unwrap();
    
    if let Some(flag) = &state_guard.running_flag {
        flag.store(false, Ordering::Relaxed);
    }

    let running_flag = Arc::new(AtomicBool::new(true));
    state_guard.running_flag = Some(running_flag.clone());
    state_guard.is_running = true;

    let duration = state_guard.duration_secs;
    let app_clone = app.clone();
    
    // Клон флага для передачи в поток
    let thread_flag = running_flag.clone();

    thread::spawn(move || {
        let mut remaining = duration;
        
        // Сразу шлем начальное время
        let _ = app_clone.emit("timer-tick", remaining);

        while remaining > 0 {
            // Проверяем флаг
            if !thread_flag.load(Ordering::Relaxed) { return; }

            thread::sleep(Duration::from_secs(1));
            remaining -= 1;

            // Шлём обновленное время
            if let Err(_) = app_clone.emit("timer-tick", remaining) { break; }
        }

        // Цикл кончился (remaining == 0). Проверяем, не остановили ли нас принудительно.
        if thread_flag.load(Ordering::Relaxed) {
            println!("Timer finished!");
            let _ = app_clone.emit("timer-finished", ());
            
            // Показываем окно
            show_notification_window(&app_clone);
        }
    });


    "Timer started".to_string()
}

#[tauri::command]
pub fn stop_timer(state: State<AppState>) -> String {
    let mut state_guard = state.0.lock().unwrap();
    
    // Даем сигнал потоку остановиться
    if let Some(flag) = &state_guard.running_flag {
        flag.store(false, Ordering::Relaxed);
    }
    
    state_guard.is_running = false;
    state_guard.running_flag = None;

    "Timer stopped".to_string()
}

#[tauri::command]
pub fn set_duration(state: State<AppState>, minutes: u64) -> String {
    let mut state_guard = state.0.lock().unwrap();
    
    if let Some(flag) = &state_guard.running_flag {
        flag.store(false, Ordering::Relaxed);
    }
    state_guard.is_running = false;
    
    state_guard.duration_secs = minutes * 60;
    println!("Duration set to {} minutes", minutes);
    format!("Time set to {} min", minutes)
}

