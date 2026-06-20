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
    let before = user_exercises.len();
    user_exercises.retain(|e| e.id != id);
    if user_exercises.len() == before {
        return Err(format!("Exercise '{}' not found", id));
    }
    save_user_exercises(&app, &user_exercises)
}

#[tauri::command]
pub fn pick_exercise_audio(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::{DialogExt, FilePath};
    app.dialog()
        .file()
        .add_filter("Audio", &["mp3", "wav"])
        .blocking_pick_file()
        .map(|f| match f {
            FilePath::Path(p) => p.display().to_string(),
            FilePath::Url(u) => u.to_string(),
        })
}

// Читает аудиофайл и возвращает base64, чтобы фронт мог сделать data: URL.
// Не расширяем assetProtocol.scope — он ограничен $APPDATA/sounds/.
// Трейдоф: небольшой IPC-оверхед при открытии модалки.
#[tauri::command]
pub fn get_exercise_audio_data(path: String) -> Result<String, String> {
    use base64::{Engine as _, engine::general_purpose};
    let bytes = fs::read(&path).map_err(|e| format!("Cannot read audio file: {}", e))?;
    Ok(general_purpose::STANDARD.encode(&bytes))
}

// Видео передаём через convertFileSrc (asset protocol, scope: "**"),
// потому что base64 для видео неприемлем по размеру.
#[tauri::command]
pub fn pick_exercise_video(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::{DialogExt, FilePath};
    app.dialog()
        .file()
        .add_filter("Video", &["mp4", "webm", "mov"])
        .blocking_pick_file()
        .map(|f| match f {
            FilePath::Path(p) => p.display().to_string(),
            FilePath::Url(u) => u.to_string(),
        })
}

// Возвращает выбранные пути; пустой Vec если отменили.
// Накопление нескольких вызовов делается на стороне фронта.
#[tauri::command]
pub fn pick_exercise_images(app: AppHandle) -> Vec<String> {
    use tauri_plugin_dialog::{DialogExt, FilePath};
    app.dialog()
        .file()
        .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
        .blocking_pick_files()
        .unwrap_or_default()
        .into_iter()
        .map(|f| match f {
            FilePath::Path(p) => p.display().to_string(),
            FilePath::Url(u) => u.to_string(),
        })
        .collect()
}
