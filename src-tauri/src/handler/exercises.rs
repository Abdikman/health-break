use tauri::{AppHandle, Manager};
use std::fs;
use serde::{Serialize, Deserialize};

/// Одна картинка упражнения с привязанным описанием.
#[derive(Serialize, Deserialize, Clone)]
pub struct ExerciseImage {
    pub path: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Exercise {
    pub id: String,
    pub title: String,
    pub description: String,
    /// Статическая картинка для builtin-упражнений (относительный asset URL).
    /// Пользовательские упражнения оставляют это поле None и используют images.
    pub image_url: Option<String>,
    /// Пользовательские картинки с описаниями. #[serde(default)] обеспечивает
    /// совместимость: старые записи без этого поля получат пустой Vec.
    #[serde(default)]
    pub images: Vec<ExerciseImage>,
    pub audio_path: Option<String>,
    pub video_path: Option<String>,
    pub tags: Vec<String>,
    pub source: String,
}

fn builtin_exercises() -> Vec<Exercise> {
    vec![
        Exercise {
            id: "1".to_string(),
            title: "Глаза: Вдаль".to_string(),
            description: "Посмотрите в окно на самый дальний объект и задержите взгляд на 20 секунд.".to_string(),
            image_url: Some("assets/exercises/eye_1.png".to_string()),
            images: vec![],
            audio_path: None,
            video_path: None,
            tags: vec!["глаза".to_string()],
            source: "builtin".to_string(),
        },
        Exercise {
            id: "2".to_string(),
            title: "Шея: Разминка".to_string(),
            description: "Медленные круговые движения головой. 5 раз в одну сторону, 5 в другую.".to_string(),
            image_url: Some("assets/exercises/neck_1.png".to_string()),
            images: vec![],
            audio_path: None,
            video_path: None,
            tags: vec!["шея".to_string()],
            source: "builtin".to_string(),
        },
        Exercise {
            id: "3".to_string(),
            title: "Спина: Потягивания".to_string(),
            description: "Встаньте, поднимите руки вверх и тянитесь к потолку.".to_string(),
            image_url: Some("assets/exercises/spine_1.png".to_string()),
            images: vec![],
            audio_path: None,
            video_path: None,
            tags: vec!["спина".to_string()],
            source: "builtin".to_string(),
        },
        Exercise {
            id: "4".to_string(),
            title: "Кисти рук".to_string(),
            description: "Вращайте кистями рук по часовой стрелке.".to_string(),
            image_url: Some("assets/exercises/wrists_1.png".to_string()),
            images: vec![],
            audio_path: None,
            video_path: None,
            tags: vec!["кисти".to_string()],
            source: "builtin".to_string(),
        },
    ]
}

fn exercises_file_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("exercises.json"))
}

/// Копирует файл src_path в $APPDATA/exercise_media/{subdir}/ с уникальным именем
/// {timestamp_millis}_{оригинальное_имя}. Возвращает полный путь к копии.
///
/// Используется для всех типов медиа (audio/images/video), чтобы упражнение
/// не зависело от исходного расположения файла на диске пользователя.
fn copy_to_media_dir(app: &AppHandle, src_path: &str, subdir: &str) -> Result<String, String> {
    let appdata = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let dest_dir = appdata.join("exercise_media").join(subdir);
    fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;

    let src = std::path::Path::new(src_path);
    let filename = src
        .file_name()
        .ok_or_else(|| format!("Не удалось определить имя файла: {}", src_path))?
        .to_string_lossy();

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    let unique_name = format!("{}_{}", ts, filename);
    let dest_path = dest_dir.join(&unique_name);

    fs::copy(src_path, &dest_path)
        .map_err(|e| format!("Ошибка копирования '{}': {}", src_path, e))?;

    Ok(dest_path.display().to_string())
}

/// Удаляет файл, только если он находится внутри нашей папки exercise_media.
/// Это защищает исходные файлы пользователя в упражнениях старого формата
/// (созданных до введения копирования), где путь ведёт не в appdata.
fn delete_if_in_media_dir(path: &str, media_dir: &std::path::Path) {
    let p = std::path::Path::new(path);
    if p.starts_with(media_dir) {
        let _ = fs::remove_file(p);
    }
}

/// Промежуточная структура для чтения exercises.json.
/// Понимает как старый формат (image_paths: Vec<String>), так и новый
/// (images: Vec<ExerciseImage>), что позволяет мигрировать без потери данных.
#[derive(Deserialize)]
struct UserExerciseOnDisk {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub image_url: Option<String>,
    /// Старый формат (до добавления описаний к фото)
    #[serde(default)]
    pub image_paths: Vec<String>,
    /// Новый формат
    #[serde(default)]
    pub images: Vec<ExerciseImage>,
    #[serde(default)]
    pub audio_path: Option<String>,
    #[serde(default)]
    pub video_path: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source: String,
}

impl From<UserExerciseOnDisk> for Exercise {
    fn from(raw: UserExerciseOnDisk) -> Self {
        // Новый формат имеет приоритет; если его нет — мигрируем из старого.
        // Описания картинок при миграции оставляем пустыми.
        let images = if !raw.images.is_empty() {
            raw.images
        } else {
            raw.image_paths.into_iter()
                .map(|path| ExerciseImage { path, description: String::new() })
                .collect()
        };
        Exercise {
            id: raw.id,
            title: raw.title,
            description: raw.description,
            image_url: raw.image_url,
            images,
            audio_path: raw.audio_path,
            video_path: raw.video_path,
            tags: raw.tags,
            source: if raw.source.is_empty() { "user".to_string() } else { raw.source },
        }
    }
}

fn load_user_exercises(app: &AppHandle) -> Vec<Exercise> {
    let path = match exercises_file_path(app) {
        Ok(p) => p,
        Err(_) => return vec![],
    };
    if !path.exists() {
        return vec![];
    }
    let raw: Vec<UserExerciseOnDisk> = fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    raw.into_iter().map(Exercise::from).collect()
}

fn save_user_exercises(app: &AppHandle, exercises: &[Exercise]) -> Result<(), String> {
    let path = exercises_file_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(exercises).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_exercises(app: AppHandle) -> Vec<Exercise> {
    let mut all = builtin_exercises();
    all.extend(load_user_exercises(&app));
    all
}

#[derive(Deserialize)]
pub struct NewExercisePayload {
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub images: Vec<ExerciseImage>,
    pub audio_path: Option<String>,
    pub video_path: Option<String>,
}

#[tauri::command]
pub fn add_user_exercise(app: AppHandle, payload: NewExercisePayload) -> Result<Exercise, String> {
    let mut user_exercises = load_user_exercises(&app);

    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_else(|_| format!("user_{}", user_exercises.len() + 1));

    // Пути в payload уже указывают на копии в exercise_media — copy_to_media_dir
    // вызывается в pick_exercise_*, здесь ничего дополнительно не копируем.
    let exercise = Exercise {
        id,
        title: payload.title,
        description: payload.description,
        tags: payload.tags,
        image_url: None,
        images: payload.images,
        audio_path: payload.audio_path,
        video_path: payload.video_path,
        source: "user".to_string(),
    };

    user_exercises.push(exercise.clone());
    save_user_exercises(&app, &user_exercises)?;
    Ok(exercise)
}

#[tauri::command]
pub fn delete_user_exercise(app: AppHandle, id: String) -> Result<(), String> {
    let mut user_exercises = load_user_exercises(&app);

    // Удаляем медиафайлы упражнения из exercise_media до изменения списка.
    // Защита: удаляем только файлы внутри нашей exercise_media, чтобы не тронуть
    // исходные файлы пользователя у упражнений старого формата.
    if let Ok(appdata) = app.path().app_data_dir() {
        let media_dir = appdata.join("exercise_media");
        if let Some(ex) = user_exercises.iter().find(|e| e.id == id) {
            if let Some(ref path) = ex.audio_path {
                delete_if_in_media_dir(path, &media_dir);
            }
            if let Some(ref path) = ex.video_path {
                delete_if_in_media_dir(path, &media_dir);
            }
            for img in &ex.images {
                delete_if_in_media_dir(&img.path, &media_dir);
            }
        }
    }

    let before = user_exercises.len();
    user_exercises.retain(|e| e.id != id);
    if user_exercises.len() == before {
        return Err(format!("Exercise '{}' not found", id));
    }
    save_user_exercises(&app, &user_exercises)
}

/// Открывает диалог выбора аудио, копирует файл в $APPDATA/exercise_media/audio/
/// и возвращает путь к копии. None — если пользователь отменил или копирование не удалось.
#[tauri::command]
pub fn pick_exercise_audio(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::{DialogExt, FilePath};
    let src = app.dialog()
        .file()
        .add_filter("Audio", &["mp3", "wav"])
        .blocking_pick_file()
        .map(|f| match f {
            FilePath::Path(p) => p.display().to_string(),
            FilePath::Url(u) => u.to_string(),
        })?;

    copy_to_media_dir(&app, &src, "audio")
        .map_err(|e| eprintln!("Ошибка копирования аудио: {}", e))
        .ok()
}

// Читает аудиофайл и возвращает base64, чтобы фронт мог сделать data: URL.
// Файл теперь всегда находится в exercise_media/audio внутри appdata.
#[tauri::command]
pub fn get_exercise_audio_data(path: String) -> Result<String, String> {
    use base64::{Engine as _, engine::general_purpose};
    let bytes = fs::read(&path).map_err(|e| format!("Cannot read audio file: {}", e))?;
    Ok(general_purpose::STANDARD.encode(&bytes))
}

/// Открывает диалог выбора видео, копирует файл в $APPDATA/exercise_media/video/
/// и возвращает путь к копии. None — если пользователь отменил или копирование не удалось.
#[tauri::command]
pub fn pick_exercise_video(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::{DialogExt, FilePath};
    let src = app.dialog()
        .file()
        .add_filter("Video", &["mp4", "webm", "mov"])
        .blocking_pick_file()
        .map(|f| match f {
            FilePath::Path(p) => p.display().to_string(),
            FilePath::Url(u) => u.to_string(),
        })?;

    copy_to_media_dir(&app, &src, "video")
        .map_err(|e| eprintln!("Ошибка копирования видео: {}", e))
        .ok()
}

/// Открывает диалог выбора изображений (множественный выбор), копирует каждый файл
/// в $APPDATA/exercise_media/images/ и возвращает список путей к копиям.
/// Файлы, которые не удалось скопировать, пропускаются; пустой Vec — если отменили.
#[tauri::command]
pub fn pick_exercise_images(app: AppHandle) -> Vec<String> {
    use tauri_plugin_dialog::{DialogExt, FilePath};
    let src_paths: Vec<String> = app.dialog()
        .file()
        .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
        .blocking_pick_files()
        .unwrap_or_default()
        .into_iter()
        .map(|f| match f {
            FilePath::Path(p) => p.display().to_string(),
            FilePath::Url(u) => u.to_string(),
        })
        .collect();

    src_paths.into_iter()
        .filter_map(|src| {
            copy_to_media_dir(&app, &src, "images")
                .map_err(|e| eprintln!("Ошибка копирования изображения: {}", e))
                .ok()
        })
        .collect()
}
