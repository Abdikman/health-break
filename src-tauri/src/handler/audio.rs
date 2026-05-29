use tauri::{AppHandle, Manager};
use std::fs;

#[derive(serde::Serialize)]
pub struct SoundFile {
    name: String,
    path: String, // Полный путь или спец. URL
    is_custom: bool,
}

#[tauri::command]
pub fn get_sounds(app: AppHandle) -> Vec<SoundFile> {
    let mut sounds = Vec::new();

    // 1. Добавляем встроенные (мы знаем, что они есть в assets)
    // Пути для них будут относительными для JS (assets/sounds/...)
    sounds.push(SoundFile {
        name: "Куплинов".to_string(),
        path: "assets/sounds/kuplinov.mp3".to_string(),
        is_custom: false,
    });

    // 2. Сканируем папку AppData/sounds
    if let Ok(app_data_dir) = app.path().app_data_dir() {
        let sounds_dir = app_data_dir.join("sounds");
        
        // Создаем папку, если нет
        if !sounds_dir.exists() {
            let _ = fs::create_dir_all(&sounds_dir);
        }

        if let Ok(entries) = fs::read_dir(&sounds_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        let ext = ext.to_string_lossy().to_lowercase();
                        if ext == "mp3" || ext == "wav" {
                            if let Some(name) = path.file_name() {
                                let name_str = name.to_string_lossy().to_string();
                                // Для пользовательских файлов нам нужен asset-protocol путь
                                // В JS мы будем использовать convertFileSrc, поэтому тут вернем просто путь
                                sounds.push(SoundFile {
                                    name: name_str.clone(),
                                    path: path.to_string_lossy().to_string(),
                                    is_custom: true,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    sounds
}

// Открыть папку (чтобы юзер мог закинуть файл)
#[tauri::command]
pub fn open_sounds_folder(app: AppHandle) {
    if let Ok(app_data_dir) = app.path().app_data_dir() {
        let sounds_dir = app_data_dir.join("sounds");
        if !sounds_dir.exists() {
            let _ = fs::create_dir_all(&sounds_dir);
        }
        
        #[cfg(target_os = "windows")]
        {
            use std::process::Command;
            let _ = Command::new("explorer").arg(sounds_dir).spawn();
        }
    }
}
