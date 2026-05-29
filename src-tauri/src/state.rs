use std::sync::{Mutex, Arc};
use std::sync::atomic::AtomicBool;

// Описываем данные таймера
pub struct TimerState {
    pub is_running: bool,
    pub duration_secs: u64,
    pub running_flag: Option<Arc<AtomicBool>>,
}

// Описываем глобальное состояние приложения
// Оборачиваем в Mutex, чтобы менять из разных потоков
pub struct AppState(pub Mutex<TimerState>);
