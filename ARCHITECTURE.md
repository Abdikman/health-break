# Архитектура Health Break

> Версия 0.8.2 alfa. Таури 2 + Rust + Vanilla JS, только Windows Desktop.

---

## Структура проекта (дерево папок с пояснениями)

```
health-break-tauri/
├── package.json               # npm-обёртка: только @tauri-apps/cli как devDependency
├── src/                       # Весь фронтенд (статические HTML/CSS/JS-файлы)
│   ├── index.html             # Главное окно (таймер + каталог + настройки)
│   ├── notification.html      # Всплывающее окно уведомления
│   ├── css/
│   │   ├── main.css           # Базовая разметка: сайдбар, вкладки, анимации
│   │   ├── timer.css          # Стили вкладки "Главная": дисплей, кнопка старт/стоп
│   │   └── catalog.css        # Стили каталога: карточки, модалки, форма добавления
│   ├── js/
│   │   ├── main.js            # Точка входа: вызывает setup-функции при DOMContentLoaded
│   │   ├── timer.js           # Логика кнопки старт/стоп и обработка событий timer-tick
│   │   ├── audio.js           # Инициализация select звука, воспроизведение, localStorage
│   │   ├── ui.js              # Каталог: сетка, модалка просмотра, модалка добавления
│   │   └── debug.js           # Кнопка "Тест уведомления" (только для разработки)
│   └── assets/
│       ├── sounds/
│       │   └── kuplinov.mp3   # Единственный встроенный звук уведомления
│       └── exercises/
│           ├── eye_1-4.png    # Картинки для встроенного упражнения "Глаза"
│           ├── neck_1-3.png   # Картинки для встроенного упражнения "Шея"
│           └── spine_1-3.png  # Картинки для встроенного упражнения "Спина"
│
├── src-tauri/                 # Весь бэкенд (Rust / Tauri)
│   ├── Cargo.toml             # Зависимости Rust
│   ├── tauri.conf.json        # Конфигурация Tauri: окна, трей, assetProtocol
│   ├── capabilities/
│   │   └── default.json       # Разрешения для окон main и notification
│   ├── build.rs               # Стандартный Tauri build-скрипт (tauri-build)
│   └── src/
│       ├── main.rs            # Точка входа бинарника (вызывает lib::run())
│       ├── lib.rs             # Регистрация команд, плагинов, трея, обработчиков окон
│       ├── state.rs           # Структуры состояния таймера (TimerState, AppState)
│       └── handler/
│           ├── mod.rs         # Объявляет четыре подмодуля
│           ├── commands.rs    # Tauri-команды таймера: start_timer, stop_timer, set_duration
│           ├── notification.rs# Показ/скрытие окна уведомления
│           ├── audio.rs       # Получение списка звуков, открытие папки sounds
│           └── exercises.rs   # CRUD упражнений, пикеры файлов, base64-аудио
```

Нет никакой БД: все данные пользователя хранятся в JSON-файле в папке AppData.
Нет сборщика (Webpack, Vite): `src/` раздаётся Tauri через встроенный asset-сервер как есть — HTML-файлы подключают скрипты через `<script type="module">`, что работает напрямую в WebView.

---

## Backend (Rust / src-tauri)

### Структуры данных

#### `TimerState` (`state.rs`)

```rust
pub struct TimerState {
    pub is_running: bool,           // флаг "таймер сейчас работает"
    pub duration_secs: u64,         // длительность перерыва в секундах (по умолчанию 30*60)
    pub running_flag: Option<Arc<AtomicBool>>,  // "рубильник" фонового потока
}
```

- `Arc<AtomicBool>` — атомарный булев флаг, обёрнутый в `Arc` (счётчик ссылок) для передачи в фоновый поток без клонирования данных. Поток проверяет `flag.load(Ordering::Relaxed)` каждую секунду; когда `start_timer` вызывается повторно или `stop_timer` — главный поток пишет `flag.store(false, ...)`, и фоновый поток прерывается.
- `Option<Arc<AtomicBool>>` — `None` значит "потока нет", `Some(flag)` — поток работает.

#### `AppState` (`state.rs`)

```rust
pub struct AppState(pub Mutex<TimerState>);
```

Обёртка в `Mutex` нужна потому, что Tauri-команды могут вызываться из любого потока. `Mutex` гарантирует, что только один вызывающий одновременно изменяет `TimerState`. Tauri регистрирует `AppState` через `.manage(...)`, после чего в команды он приходит как `State<AppState>`.

---

#### `ExerciseImage` (`handler/exercises.rs`)

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct ExerciseImage {
    pub path: String,         // абсолютный путь к файлу в exercise_media/images/
    pub description: String,  // текстовое описание, введённое пользователем
}
```

`Serialize` — Rust умеет преобразовать в JSON для отправки в JS.
`Deserialize` — Rust умеет прочитать из JSON (из файла exercises.json или от JS).
`Clone` — структуру можно копировать (нужно при сборке Exercise).

---

#### `Exercise` (`handler/exercises.rs`)

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct Exercise {
    pub id: String,                   // для builtin: "1","2","3"; для user: миллисекунды UNIX
    pub title: String,
    pub description: String,
    pub image_url: Option<String>,    // устаревшее поле; для новых упражнений всегда None
    #[serde(default)]
    pub images: Vec<ExerciseImage>,   // список фото с описаниями
    pub audio_path: Option<String>,   // путь к MP3/WAV в exercise_media/audio/
    pub video_path: Option<String>,   // путь к видео в exercise_media/video/
    pub tags: Vec<String>,            // ["глаза"], ["шея"] и т.п.
    pub source: String,               // "builtin" или "user"
}
```

`Option<T>` — необязательное значение: либо `Some(значение)`, либо `None` (отсутствует). При сериализации в JSON `None` превращается в `null`.

`#[serde(default)]` означает: если поля нет в JSON-файле (старые записи), использовать значение по умолчанию — для `Vec` это пустой вектор. Без этого атрибута десериализация упала бы с ошибкой.

---

#### `UserExerciseOnDisk` (`handler/exercises.rs`)

Промежуточная структура только для _чтения_ из `exercises.json`. Понимает оба формата:

```rust
#[derive(Deserialize)]
struct UserExerciseOnDisk {
    // ... общие поля ...
    #[serde(default)]
    pub image_paths: Vec<String>,    // СТАРЫЙ формат: просто пути без описаний
    #[serde(default)]
    pub images: Vec<ExerciseImage>,  // НОВЫЙ формат: пути + описания
}
```

Конвертация в `Exercise` делается через `From`:

```rust
impl From<UserExerciseOnDisk> for Exercise {
    fn from(raw: UserExerciseOnDisk) -> Self {
        let images = if !raw.images.is_empty() {
            raw.images   // новый формат — берём как есть
        } else {
            // старый формат — оборачиваем в ExerciseImage с пустым description
            raw.image_paths.into_iter()
                .map(|path| ExerciseImage { path, description: String::new() })
                .collect()
        };
        // ...
    }
}
```

Это называется **миграцией на лету**: никакого скрипта обновления не нужно — при каждом чтении файла старые записи автоматически конвертируются в новый формат в памяти.

---

#### `NewExercisePayload` (`handler/exercises.rs`)

```rust
#[derive(Deserialize)]
pub struct NewExercisePayload {
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub images: Vec<ExerciseImage>,   // пути уже указывают на копии в exercise_media
    pub audio_path: Option<String>,
    pub video_path: Option<String>,
}
```

Принимается как параметр `add_user_exercise`. JS передаёт объект, Tauri автоматически десериализует его в эту структуру.

---

#### `PickImagesResult` (`handler/exercises.rs`)

```rust
#[derive(Serialize)]
pub struct PickImagesResult {
    pub paths: Vec<String>,   // успешно скопированные пути
    pub errors: Vec<String>,  // имена файлов, которые не удалось скопировать
}
```

Возвращается из `pick_exercise_images`. JS проверяет оба поля.

---

#### `SoundFile` (`handler/audio.rs`)

```rust
#[derive(Serialize)]
pub struct SoundFile {
    name: String,
    path: String,      // для builtin: относительный "assets/sounds/...", для user: абсолютный
    is_custom: bool,
}
```

JS по флагу `is_custom` решает, нужно ли применять `convertFileSrc`.

---

### Tauri-команды

Все функции ниже помечены `#[tauri::command]` и зарегистрированы в `lib.rs` через `tauri::generate_handler![]`.

---

#### `start_timer` (`handler/commands.rs`)

**Принимает:** `AppHandle` (доступ к приложению), `State<AppState>` (состояние).
**Возвращает:** `String` (`"Timer started"`).

Пошаговая логика:
1. Берём эксклюзивный доступ к `TimerState` через `state.0.lock().unwrap()`. `unwrap()` здесь означает: "паникуй если мьютекс отравлен" (это произойдёт только при панике в другом потоке — маловероятно в нашем случае).
2. Если уже есть старый фоновый поток — убиваем его мягко: `flag.store(false, Ordering::Relaxed)`.
3. Создаём новый атомарный флаг `Arc::new(AtomicBool::new(true))`, сохраняем в состоянии.
4. Клонируем `AppHandle` и флаг для передачи в поток (`Clone` здесь дешёвый — увеличивает счётчик ссылок).
5. Запускаем `thread::spawn(...)` — отдельный поток ОС. В нём:
   - Каждую секунду проверяем флаг; если `false` — выходим.
   - Отправляем `app.emit("timer-tick", remaining)` — Tauri шлёт событие во все окна.
   - После обнуления счётчика: эмитируем `"timer-finished"` и вызываем `show_notification_window`.

`.ok()` — игнорирует ошибку (преобразует `Result<T,E>` в `Option<T>`). Здесь используется при отправке событий — если окно закрыто, ошибку молча проглатываем.

---

#### `stop_timer` (`handler/commands.rs`)

**Принимает:** `State<AppState>`.
**Возвращает:** `String`.

Записывает `false` в флаг → фоновый поток увидит и выйдет сам. Очищает `running_flag = None`.

---

#### `set_duration` (`handler/commands.rs`)

**Принимает:** `State<AppState>`, `minutes: u64`.
**Возвращает:** `String`.

Останавливает текущий таймер (через флаг), записывает `duration_secs = minutes * 60`. Следующий `start_timer` возьмёт новое значение.

---

#### `show_notification_window` / `show_notification` (`handler/notification.rs`)

`show_notification_window` — внутренняя Rust-функция (без `#[tauri::command]`), вызывается из потока таймера.
`show_notification` — Tauri-команда для JS (тест-кнопка).

Логика:
1. Находим окно с label `"notification"`.
2. Получаем размер экрана через `window.current_monitor()`. `if let Some(m) = monitor { ... }` — раскрывает `Option<Monitor>`: если монитор не найден, ничего не делаем.
3. Считаем позицию: правый нижний угол минус размер окна минус 60px от низа (над панелью задач).
4. `window.set_position(...)`, `window.show()`, `window.set_focus()`.
5. Эмитируем `"notification-shown"` — JS в `notification.html` запускает автотаймер закрытия.

---

#### `close_notification` (`handler/notification.rs`)

**Вызывается из JS** `notification.html` после таймаута или клика.

Прячет окно (`window.hide()`) и делает `window.navigate(...)` обратно на `notification.html` — это сброс состояния JS: иначе при следующем показе JS-переменные сохраняли бы старое состояние.

---

#### `notification_clicked` (`handler/notification.rs`)

**Вызывается из JS** при клике на карточку уведомления.

1. Прячет уведомление.
2. Показывает и фокусирует главное окно.
3. Эмитирует `"navigate-to-catalog"` — JS в главном окне переключает вкладку.

---

#### `get_exercises` (`handler/exercises.rs`)

**Возвращает:** `Vec<Exercise>`.

Конкатенирует `builtin_exercises()` (три встроенных, захардкожены в коде) с `load_user_exercises()` (из exercises.json). Builtin-упражнения всегда идут первыми.

---

#### `add_user_exercise` (`handler/exercises.rs`)

**Принимает:** `NewExercisePayload`.
**Возвращает:** `Result<Exercise, String>`.

`Result<T, E>` — либо успех `Ok(Exercise)`, либо ошибка `Err(String)`. При ошибке JS получает исключение в `catch`.

1. Загружает текущий список пользователей.
2. Генерирует ID = миллисекунды с UNIX epoch (практически уникально в нашем контексте). `.unwrap_or_else(|_| ...)` — если системные часы раньше 1970 (невозможно на практике), берём запасной вариант.
3. Пути к медиафайлам в `payload` уже указывают на `exercise_media/` — копирование было сделано на предыдущих шагах через `pick_exercise_*`.
4. Добавляет, сохраняет, возвращает новое упражнение.

---

#### `delete_user_exercise` (`handler/exercises.rs`)

**Принимает:** `id: String`.
**Возвращает:** `Result<(), String>`.

1. Находит упражнение по id.
2. Удаляет медиафайлы через `delete_if_in_media_dir` — функция проверяет `path.starts_with(media_dir)`, чтобы не удалить исходные файлы пользователя из упражнений старого формата (до введения копирования).
3. Удаляет запись из вектора через `.retain(|e| e.id != id)`.
4. Если ничего не удалено (длина не изменилась) — возвращает ошибку.

---

#### `pick_exercise_audio` / `pick_exercise_video` (`handler/exercises.rs`)

**Возвращают:** `Option<String>` — путь к копии в `exercise_media/audio/` или `None` при отмене.

1. Открывают системный диалог файла через `tauri_plugin_dialog`.
2. `FilePath::Path(p) => p.display().to_string()` — преобразует `PathBuf` в строку.
3. Вызывают `copy_to_media_dir(&app, &src, "audio")`.
4. `.map_err(|e| eprintln!(...)).ok()` — если копирование не удалось, печатают ошибку в stderr и возвращают `None` (вместо паники или передачи ошибки в JS).

---

#### `pick_exercise_images` (`handler/exercises.rs`)

**Возвращает:** `PickImagesResult { paths, errors }`.

Аналогично, но с множественным выбором (`blocking_pick_files()`). Каждый файл копируется независимо — ошибки копирования не прерывают обработку остальных файлов.

---

#### `get_exercise_audio_data` (`handler/exercises.rs`)

**Принимает:** `path: String`.
**Возвращает:** `Result<String, String>` — Base64-строка или ошибка.

Читает бинарный файл (`fs::read`) и кодирует в Base64. JS строит `data:audio/mpeg;base64,...` URL. Это обходит ограничение `assetProtocol.scope` — в конфиге доступна только папка `sounds/`, но не `exercise_media/`. Альтернативой было бы расширить scope, но разработчики выбрали base64-подход.

---

#### `get_sounds` (`handler/audio.rs`)

**Возвращает:** `Vec<SoundFile>`.

1. Добавляет встроенный `kuplinov.mp3` с `is_custom: false`.
2. Создаёт папку `$APPDATA/sounds/` если не существует.
3. Перечисляет файлы `.mp3` и `.wav` в ней, добавляет с `is_custom: true`.

`.flatten()` на итераторе `Result<DirEntry>` — пропускает ошибки чтения записей (нет прав и т.п.).

---

#### `open_sounds_folder` (`handler/audio.rs`)

Открывает папку `$APPDATA/sounds/` в Проводнике Windows через `Command::new("explorer")`. `#[cfg(target_os = "windows")]` — компилируется только на Windows.

---

### Хранение данных

#### Структура папок AppData

```
%APPDATA%\com.healthbreak.app\   (Windows: C:\Users\{user}\AppData\Roaming\...)
├── exercises.json               # Список упражнений пользователя
├── sounds/                      # Пользовательские звуки уведомлений
│   └── mой_звук.mp3
└── exercise_media/              # Медиафайлы упражнений (копии)
    ├── audio/
    │   └── 1718000000000_track.mp3   # {timestamp_ms}_{оригинальное_имя}
    ├── video/
    │   └── 1718000001234_demo.mp4
    └── images/
        ├── 1718000002000_photo1.jpg
        └── 1718000002100_photo2.png
```

Путь получается через `app.path().app_data_dir()` — Tauri возвращает правильный путь для текущей ОС.

#### Формат exercises.json

```json
[
  {
    "id": "1718000000000",
    "title": "Разминка плеч",
    "description": "Круговые движения плечами",
    "image_url": null,
    "images": [
      {
        "path": "C:\\Users\\...\\exercise_media\\images\\1718000002000_photo1.jpg",
        "description": "Исходное положение — руки вдоль тела"
      }
    ],
    "audio_path": "C:\\Users\\...\\exercise_media\\audio\\1718000000000_track.mp3",
    "video_path": null,
    "tags": ["шея", "спина"],
    "source": "user"
  }
]
```

Файл содержит **только** пользовательские упражнения. Builtin-упражнения генерируются в коде каждый раз заново.

#### Именование файлов при копировании

`copy_to_media_dir` использует схему `{timestamp_millis}_{оригинальное_имя}`:
- Предотвращает конфликты имён (два файла с одним именем из разных папок).
- Сохраняет читаемое имя для отладки.

#### Миграция старого формата

До версии 0.8.0 поле называлось `image_paths: Vec<String>`. Если в exercises.json встречается старая запись (без поля `images`), `#[serde(default)]` подставит пустой `Vec`, и `From<UserExerciseOnDisk>` сконвертирует `image_paths` в `images` с пустыми описаниями. Это прозрачно для пользователя.

---

### Инициализация приложения (`lib.rs`)

```
tauri::Builder::default()
  .plugin(opener)          — открытие URL/файлов через ОС (не используется активно)
  .plugin(dialog)          — системные диалоги выбора файлов
  .manage(AppState(...))   — регистрирует глобальное состояние
  .invoke_handler(...)     — регистрирует все Tauri-команды
  .setup(...)              — создаёт меню трея
  .on_tray_icon_event(...) — левый клик по иконке трея → показывает главное окно
  .on_menu_event(...)      — "Выход" → app.exit(0)
  .on_window_event(...)    — CloseRequested (красная кнопка X) → скрывает окно вместо закрытия
  .run(...)
```

**Важное поведение**: нажатие X не закрывает приложение — оно сворачивается в трей. Закрыть можно только через меню трея "Выход". Это реализовано через `api.prevent_close()` + `win.hide()`.

Трей создаётся из `tauri.conf.json` (секция `trayIcon`), а меню добавляется в `setup()` — два шага специально разделены, потому что в Tauri 2 конфигурация трея и меню идут отдельными API.

---

## Frontend (src/)

### index.html — структура документа

Документ состоит из двух частей:

**Навигация** (`.sidebar`): три кнопки-вкладки (`data-tab="home"`, `catalog`, `settings`) и кнопка тест-уведомления.

**Основное содержимое** (`.content`): три `<section class="tab-content">`:
- `#home` — таймер: большой дисплей `30:00` и круглая кнопка включения.
- `#catalog` — сетка упражнений `#exercise-grid` и шапка с кнопкой "+ Добавить".
- `#settings` — поле числа минут, выпадающий список звуков, кнопка открыть папку.

**Модалка просмотра** (`#exercise-modal`): заголовок, теги, зона медиа (`#modal-media` — галерея рендерится сюда через JS), видео, аудио, общее описание, кнопка удаления.

**Модалка добавления** (`#add-exercise-modal`): форма с полями "Название", "Описание", чекбоксами тегов, зоной превью изображений (`#image-preview-strip`), пикерами аудио и видео, кнопками "Отмена" и "Сохранить".

Все три CSS-файла подключены в `<head>`, единственный скрипт `js/main.js` — с атрибутом `type="module"` и `defer`.

---

### ui.js — все функции

#### `setupTabs()`

**Параметры:** нет. **Side-effects:** вешает click-обработчики на `.nav-btn`, подписывается на событие `"navigate-to-catalog"`.

Логика переключения: при клике снимает класс `active` у всех кнопок и вкладок, добавляет `active` кнопке-источнику и соответствующей секции (`document.getElementById(btn.dataset.tab)`).

При получении события `"navigate-to-catalog"` от Rust — программно вызывает `.click()` на кнопку каталога.

---

#### `setupCatalog()`

**Параметры:** нет. **Side-effects:** настраивает все взаимодействия с каталогом, вешает десятки обработчиков событий.

Внутри функции объявлены:

- Ссылки на DOM-элементы (получаются один раз при инициализации).
- Переменные состояния:
  - `exercises: Exercise[]` — текущий список всех упражнений.
  - `currentExercise` — упражнение, открытое в модалке просмотра.
  - `selectedImages: { path, description }[]` — изображения в форме добавления.
  - `selectedAudioPath`, `selectedVideoPath` — пути медиа в форме.
  - `galleryImages: { src, description }[]` — нормализованные картинки в галерее.
  - `currentImageIndex` — индекс активного фото в галерее.
  - `galleryMainImg`, `galleryThumbStrip`, `galleryPhotoDesc` — ссылки на DOM-элементы галереи (создаются динамически, поэтому нужны ссылки).

**Вложенные функции:**

---

##### `escapeHtml(str)`
Экранирует `&`, `<`, `>`, `"` перед вставкой пользовательских строк в `innerHTML`. Защита от XSS (пользователь может написать `<script>` в названии упражнения).

---

##### `getImages(ex)`
Нормализует картинки упражнения в `{ src, description }[]`.
- Если `ex.source === 'builtin'`: пути типа `"assets/exercises/eye_1.png"` — браузер разрешает их напрямую, `convertFileSrc` применять нельзя (испортит URL).
- Если `ex.source === 'user'`: пути абсолютные (`C:\Users\...`) — нужен `convertFileSrc`, чтобы превратить их в `asset://localhost/...` URL, который WebView умеет загружать.

---

##### `navigateGallery(delta)`
Меняет текущее фото в галерее.
- `delta = 1` → следующее, `delta = -1` → предыдущее, `delta = 0` → перейти к `currentImageIndex` (при клике по миниатюре).
- Вычисление по кругу: `(currentImageIndex + delta + length) % length` — при `delta = -1` и `index = 0` даёт последний элемент.
- Обновляет `galleryMainImg.src`, `galleryPhotoDesc.textContent` и класс `active` на миниатюрах.

---

##### `handleGalleryKeys(e)`
Обработчик `keydown` для навигации стрелками по галерее.
- Игнорирует всё кроме `ArrowLeft`/`ArrowRight`.
- Не работает если галерея ≤ 1 фото.
- Пропускает событие если `document.activeElement === mVideo` — пользователь управляет видеоплеером.
- Вызывает `e.preventDefault()` чтобы не прокручивать страницу.
- Добавляется при открытии модалки, удаляется при закрытии (избегая накопления слушателей).

---

##### `closeDetailModal()`
1. Удаляет `handleGalleryKeys` с документа.
2. Убирает класс `active` с модалки.
3. Останавливает видео и аудио (`.pause()`, `.src = ''`), очищает `mMedia.innerHTML`.
4. Сбрасывает все переменные галереи в начальные значения.

---

##### `openDetailModal(data)`
Асинхронная. Заполняет модалку данными упражнения:
1. Заголовок, теги (через `innerHTML` со `span.tag`).
2. Видео: если есть `data.video_path` — конвертирует через `convertFileSrc`, показывает контейнер.
3. Галерея: `renderModalGallery(getImages(data))`.
4. Аудио: вызывает `invoke('get_exercise_audio_data', { path })`, собирает `data:audio/mpeg;base64,...`. Отдельный `try/catch` — ошибка загрузки аудио не ломает открытие модалки.
5. Кнопка удаления: видима только для `source === 'user'`.
6. Добавляет `handleGalleryKeys`.

---

##### `renderModalGallery(images)`
Полностью очищает `mMedia` и строит галерею:
- **0 фото** → пусто.
- **1 фото** → `<img class="modal-img">` + `<p class="gallery-photo-desc">`.
- **2+ фото** → конструкция:
  ```
  div.modal-gallery
    div.modal-gallery-main-wrap
      img.modal-gallery-main   ← большое фото
      button.gallery-arrow--prev
      button.gallery-arrow--next
    p.gallery-photo-desc       ← описание текущего фото
    div.modal-gallery-strip
      img.modal-gallery-thumb (x N)
  ```
  Первая миниатюра получает класс `active`. Клик по миниатюре устанавливает `currentImageIndex = i` и вызывает `navigateGallery(0)`.

---

##### `closeAddModal()`
Убирает класс `active` с модалки добавления, сбрасывает форму (`addForm.reset()`), очищает массивы `selectedImages`, пути медиа, превью.

---

##### `renderImagePreviews()`
Полностью перестраивает `#image-preview-strip` из массива `selectedImages`:
- Каждый элемент: превью-изображение (`<img>` с `convertFileSrc(item.path)`) + кнопка удаления (`×`) + `<textarea>` для описания.
- `removeBtn.click` → `selectedImages.splice(i, 1)` + рекурсивный `renderImagePreviews()`.
- `textarea.input` → `selectedImages[i].description = caption.value` — двусторонняя привязка к массиву.
- При перерисовке восстанавливает уже введённые описания из `item.description`.

---

##### `loadExercises()`
Асинхронная. `invoke('get_exercises')` → кладёт в `exercises` → `renderGrid()`.

---

##### `renderGrid()`
Строит innerHTML для `#exercise-grid` через `.map().join('')`:
- `getImages(ex)[0].src` — обложка карточки (первое фото).
- Бейджи: `user-badge` ("Моё"), `video-badge` (🎬), `audio-badge` (🎵), `img-count-badge` (количество фото).
- Пользовательские карточки дополнительно получают класс `user-exercise` → зелёная рамка.
- После рендера: вешает click-обработчики на каждую карточку через `querySelectorAll`.

---

#### `setupSettings()`

Вешает обработчик `change` на `#duration-input`: при изменении вызывает `invoke('set_duration', { minutes: val })`.

---

### Обработчики событий

| Элемент | Событие | Действие |
|---|---|---|
| `.nav-btn` (3 штуки) | `click` | Переключение вкладки |
| `"navigate-to-catalog"` (Tauri event) | — | Программный клик по кнопке каталога |
| `#toggle-timer` | `click` | `start_timer` или `stop_timer` |
| `"timer-tick"` (Tauri event) | — | Обновление дисплея таймера |
| `"timer-finished"` (Tauri event) | — | `playSound()` + `show_notification` |
| `[data-tab="catalog"]` | `click` | `loadExercises()` |
| `.exercise-card` (динамические) | `click` | `openDetailModal(data)` |
| `.close-modal` (в просмотре) | `click` | `closeDetailModal()` |
| `window` | `click` (делегация) | Закрыть модалку при клике на оверлей |
| `#delete-exercise-btn` | `click` | `delete_user_exercise` + перезагрузка |
| `document` | `keydown` | Навигация галереи стрелками |
| `#add-exercise-btn` | `click` | Открыть форму добавления |
| `#close-add-modal`, `#cancel-add-exercise` | `click` | `closeAddModal()` |
| `#pick-images-btn` | `click` | `pick_exercise_images` → `renderImagePreviews()` |
| `.remove-img-btn` (динамические) | `click` | Удалить из `selectedImages` + перерисовка |
| `.img-caption-input` (динамические) | `input` | Обновить `selectedImages[i].description` |
| `#pick-audio-btn` | `click` | `pick_exercise_audio` |
| `#pick-video-btn` | `click` | `pick_exercise_video` |
| `#add-exercise-form` | `submit` | `add_user_exercise` + перезагрузка |
| `#duration-input` | `change` | `set_duration` |
| `#sound-select` | `change` | Сохранить в localStorage + тест звука |
| `#open-sounds-btn` | `click` | `open_sounds_folder` |
| `#test-notify-btn` | `click` | `playSound()` + `show_notification` |
| `#card` (notification.html) | `click` | `notification_clicked` |
| `"notification-shown"` (notification.html) | — | Перезапустить таймер автозакрытия |

---

### Таблица: JS → Rust invoke

| JS-код (файл) | Rust-команда | Зачем |
|---|---|---|
| `timer.js: invoke('start_timer')` | `commands::start_timer` | Запустить обратный отсчёт |
| `timer.js: invoke('stop_timer')` | `commands::stop_timer` | Остановить отсчёт |
| `timer.js: invoke('show_notification', {...})` | `notification::show_notification` | Дублирует показ окна из JS (на случай если событие timer-finished доходит позже окна) |
| `ui.js: invoke('get_exercises')` | `exercises::get_exercises` | Загрузить каталог |
| `ui.js: invoke('add_user_exercise', { payload })` | `exercises::add_user_exercise` | Сохранить новое упражнение |
| `ui.js: invoke('delete_user_exercise', { id })` | `exercises::delete_user_exercise` | Удалить упражнение и его медиафайлы |
| `ui.js: invoke('pick_exercise_images')` | `exercises::pick_exercise_images` | Диалог выбора файлов + копирование |
| `ui.js: invoke('pick_exercise_audio')` | `exercises::pick_exercise_audio` | Диалог выбора аудио + копирование |
| `ui.js: invoke('pick_exercise_video')` | `exercises::pick_exercise_video` | Диалог выбора видео + копирование |
| `ui.js: invoke('get_exercise_audio_data', { path })` | `exercises::get_exercise_audio_data` | Получить аудио как base64 |
| `audio.js: invoke('get_sounds')` | `audio::get_sounds` | Список звуков для select |
| `audio.js: invoke('open_sounds_folder')` | `audio::open_sounds_folder` | Открыть папку в Проводнике |
| `debug.js: invoke('show_notification', {...})` | `notification::show_notification` | Тест-кнопка |
| `notification.html: invoke('close_notification')` | `notification::close_notification` | Скрыть уведомление (автотаймер) |
| `notification.html: invoke('notification_clicked')` | `notification::notification_clicked` | Клик по карточке → открыть каталог |
| `settings: invoke('set_duration', { minutes })` | `commands::set_duration` | Изменить длительность перерыва |

---

### Галерея фото с описаниями — детальная логика

#### State-переменные (в `setupCatalog`)

```
galleryImages:     { src: string, description: string }[]  — нормализованный массив фото
currentImageIndex: number                                   — индекс активного фото (0-based)
galleryMainImg:    HTMLImageElement | null                  — ссылка на большое фото в DOM
galleryThumbStrip: HTMLElement | null                       — ссылка на полоску миниатюр
galleryPhotoDesc:  HTMLParagraphElement | null              — ссылка на <p> описания
```

Эти переменные живут в замыкании `setupCatalog()` — они недоступны глобально, но доступны всем вложенным функциям.

#### Нормализация (`getImages`)

Прежде чем рендерить, пути приводятся к `src` (правильный URL для WebView):
- `builtin`: `"assets/exercises/eye_1.png"` → остаётся как есть (работает как relative URL).
- `user`: `"C:\Users\...\images\photo.jpg"` → `convertFileSrc(...)` → `"asset://localhost/C:/Users/.../photo.jpg"`.

`convertFileSrc` — функция из Tauri JS API, она регистрирует файл в протоколе `asset://`, который контролируется `assetProtocol.scope` в `tauri.conf.json`.

#### Рендер (`renderModalGallery`)

При каждом открытии модалки `mMedia.innerHTML = ''` — очищается полностью. Это важно: нельзя менять `.src` без очистки, иначе могут остаться старые обработчики на кнопках.

Для **2+ фото** строится DOM-дерево программно (не через innerHTML — чтобы повесить обработчики напрямую без делегации). Ссылки на ключевые элементы сохраняются в state-переменных.

#### Навигация (`navigateGallery`)

```js
currentImageIndex = (currentImageIndex + delta + galleryImages.length) % galleryImages.length
```

`+ galleryImages.length` нужен чтобы избежать отрицательного остатка при `delta = -1` и `index = 0`:
`(0 - 1 + 4) % 4 = 3` — переходим на последнюю.

После смены индекса:
1. `galleryMainImg.src = current.src` — меняется большое фото.
2. `galleryPhotoDesc.textContent = current.description` — меняется описание.
3. Все `.modal-gallery-thumb` получают/теряют класс `active` через `classList.toggle`.

#### Клик по миниатюре

```js
thumb.addEventListener('click', () => {
    currentImageIndex = i          // прямое присваивание индекса
    navigateGallery(0)             // delta=0: не двигаемся, просто синхронизируем UI
})
```

#### Клавиатура (`handleGalleryKeys`)

Добавляется к `document` при открытии и удаляется при закрытии:
```js
document.removeEventListener('keydown', handleGalleryKeys)
document.addEventListener('keydown', handleGalleryKeys)
```

`removeEventListener` перед `addEventListener` гарантирует ровно один слушатель (иначе при повторных открытиях слушатели накапливались бы — классический баг).

---

### CSS — смысловые группы стилей

#### `main.css` — каркас

- **`:root`** — CSS-переменные для возможной смены темы: `--bg-color`, `--sidebar-bg`, `--accent-color`.
- **`body`** — `display: flex` делает сайдбар + контент в горизонтальный ряд; `overflow: hidden` убирает скролл страницы.
- **`.sidebar`** — фиксированная ширина 200px, тёмный фон. `.nav-btn.active` получает акцентный фон.
- **`.tab-content`** — `display: none` по умолчанию; `.active` → `display: block` + анимация `fadein` (появление снизу вверх с opacity).

#### `timer.css` — главная вкладка

- **`#home.tab-content.active`** — `display: flex` (переопределяет block из main.css), чтобы работал `align-items: center` для вертикального центрирования.
- **`.timer-display`** — шрифт 80px, `font-variant-numeric: tabular-nums` предотвращает "прыжки" когда цифры меняются (у всех цифр одинаковая ширина).
- **`.power-btn`** — круглая кнопка 100×100px. `box-shadow` в 3 слоя создаёт эффект концентрических колец.
- **`.power-btn.active`** — красный цвет + анимация `pulse` (пульсирующее свечение через изменение box-shadow).

#### `catalog.css` — сетка карточек

- **`.exercise-grid`** — CSS Grid с `auto-fill, minmax(220px, 1fr)`: карточки автоматически переносятся, минимальная ширина 220px.
- **`.exercise-card`** — белый фон, скругления, тень, hover-эффект `translateY(-4px)` (карточка "всплывает").
- **`.user-exercise`** — зелёная рамка вместо серой `#eee`, чтобы визуально отличить пользовательские.
- **`.card-img-container`** — фиксированная высота 140px с `overflow: hidden`; `.card-img` покрывает через `object-fit: cover`.
- **Бейджи** (`.user-badge`, `.audio-badge`, `.video-badge`, `.img-count-badge`) — абсолютно позиционированы внутри `.card-img-container` с полупрозрачным фоном.

#### `catalog.css` — модалка просмотра

- **`.modal-overlay`** — `position: fixed` на весь экран, `backdrop-filter: blur(5px)`. По умолчанию `display: none`; при `.active` становится `display: flex` (центрирование контента).
- **`.modal-content`** — белый блок с `max-height: 90vh` и `overflow-y: auto` (модалка скроллится внутри, не страница). `transform: scale(0.9)` → `scale(1)` при `.active` — анимация "вырастания".
- **Галерея** (`.modal-gallery-main-wrap`, `.gallery-arrow`, `.modal-gallery-strip`, `.modal-gallery-thumb`) — стрелки позиционированы абсолютно по бокам главного фото; полоска миниатюр скроллится горизонтально (`overflow-x: auto`). Активная миниатюра — синяя рамка.
- **`.gallery-photo-desc`** — небольшой текст серого цвета между главным фото и миниатюрами.

#### `catalog.css` — форма добавления

- **`.modal-content--form`** — более узкая модалка (max-width 520px).
- **`.tag-label:has(input:checked)`** — CSS-селектор `:has()` меняет вид пилюли-тега при установке галочки (работает в современных Chrome-based WebView).
- **`.image-preview-item`** — строка: миниатюра 72×72 + textarea для описания фото; `flex-direction: column` в `.image-preview-strip` складывает их вертикально.
- **`.remove-img-btn`** — абсолютно позиционированная кнопка в правом верхнем углу миниатюры.
- **`.btn-primary`** / **`.btn-secondary`** — кнопки "Сохранить" и "Отмена".

---

## Пользовательские сценарии (шаг за шагом)

### Добавление упражнения с фото

1. Пользователь открывает вкладку "Каталог" → `loadExercises()` → `get_exercises` (builtin + user).
2. Клик на "+ Добавить упражнение" → `addModal.classList.add('active')`.
3. Вводит название и описание.
4. Клик на "Добавить изображения":
   - JS вызывает `pick_exercise_images`.
   - Rust открывает системный диалог (блокирующий, в `blocking_pick_files`).
   - Для каждого файла: копирует в `$APPDATA/exercise_media/images/` с именем `{ts}_{name}`.
   - Возвращает `{ paths: [...], errors: [...] }`.
   - JS добавляет каждый путь в `selectedImages = [..., { path, description: '' }]`.
   - `renderImagePreviews()` строит превью с textarea.
5. Пользователь вводит описания к фото в textarea → `selectedImages[i].description` обновляется на каждое `input`.
6. Опционально выбирает аудио/видео (аналогичный диалог → копирование → сохранение пути).
7. Клик "Сохранить" → `submit`:
   - Собирает `{ title, description, tags, images: selectedImages, audio_path, video_path }`.
   - `invoke('add_user_exercise', { payload: ... })`.
   - Rust: генерирует id, создаёт `Exercise`, добавляет в список, сохраняет JSON.
   - JS: `closeAddModal()` + `loadExercises()`.

### Просмотр упражнения (галерея)

1. Клик по карточке → `openDetailModal(data)`.
2. Заполняется заголовок, теги, общее описание.
3. `getImages(data)` нормализует пути (builtin → relative, user → `convertFileSrc`).
4. `renderModalGallery(images)`:
   - 1 фото → одно изображение + описание.
   - 2+ → галерея со стрелками, описанием, миниатюрами.
5. Если есть аудио: `invoke('get_exercise_audio_data')` → base64 → `<audio src="data:audio/mpeg;base64,...">`.
6. Если есть видео: `<video src="asset://...">`.
7. Навигация: стрелки (click), клавиатура (ArrowLeft/Right), миниатюры (click).
8. Закрытие: X или клик на оверлей → `closeDetailModal()` → пауза медиа, очистка DOM и state.

### Удаление упражнения

1. Открыта модалка просмотра пользовательского упражнения.
2. Кнопка "🗑 Удалить" видима (`.style.display = 'inline-flex'`).
3. Клик → `invoke('delete_user_exercise', { id: currentExercise.id })`.
4. Rust:
   - Загружает список.
   - Для каждого медиафайла упражнения вызывает `delete_if_in_media_dir` — удаляет только если файл в `exercise_media/`.
   - `.retain(|e| e.id != id)` — убирает запись.
   - Сохраняет JSON.
5. JS: `closeDetailModal()` + `loadExercises()` — перерисовывает сетку.

### Работа таймера и уведомлений

1. Пользователь кликает кнопку включения:
   - `isRunning = false` → запуск: `invoke('start_timer')`, кнопка получает `.active` (красное свечение).
   - `isRunning = true` → остановка: `invoke('stop_timer')`, кнопка теряет `.active`.
2. Rust запускает фоновый поток. Каждую секунду: `app.emit("timer-tick", remaining)`.
3. JS получает `timer-tick` (если `!ignoreTicks`): обновляет дисплей `MM:SS`.
4. После обнуления Rust: `app.emit("timer-finished")` + вызов `show_notification_window`.
5. JS получает `timer-finished`: `playSound()` → читает URL из localStorage → `new Audio(url).play()`.
6. Rust позиционирует окно `notification` в правом нижнем углу экрана и показывает его.
7. `notification.html` получает `"notification-shown"` → запускает `setTimeout(5000, close_notification)`.
8. Либо таймаут: `invoke('close_notification')` → Rust прячет окно и перезагружает notification.html.
9. Либо клик по карточке: `invoke('notification_clicked')` → прячет уведомление, показывает главное окно, эмитирует `"navigate-to-catalog"` → JS переключает вкладку.

---

## Подводные камни и хрупкие места

### 1. Путаница `builtin` vs `user` при работе с путями к файлам

`getImages()` обязательно должна проверять `ex.source`:
- Для `builtin` пути `"assets/exercises/eye_1.png"` — это **относительные URL**, WebView разрешает их сам. Вызов `convertFileSrc` превратил бы их в `asset://localhost/assets/...`, что **не работает**, потому что asset protocol не видит папку `src/assets/` внутри бандла.
- Для `user` пути абсолютные (`C:\Users\...\exercise_media\images\...`) — без `convertFileSrc` WebView не сможет их загрузить.

Если добавить новый тип `source`, не забудь обновить `getImages()`.

### 2. assetProtocol.scope не покрывает exercise_media

В `tauri.conf.json` в `assetProtocol.scope` прописано только `["$APPDATA/sounds/*", ...]`. Папка `exercise_media/` там **отсутствует**. Поэтому для видео используется `convertFileSrc` (он работает через `asset://`), а для аудио используется обходной путь через base64 (`get_exercise_audio_data`).

Почему так: возможно, разработчики опасались расширять scope на весь AppData, или столкнулись с тем, что `convertFileSrc` для аудио в WebView ведёт себя нестабильно. Если понадобится изменить подход — нужно добавить `"$APPDATA/exercise_media/**"` в scope и убрать `get_exercise_audio_data`.

### 3. Накопление keydown-слушателей

В `openDetailModal` делается:
```js
document.removeEventListener('keydown', handleGalleryKeys)
document.addEventListener('keydown', handleGalleryKeys)
```

Важно именно это — сначала `remove`, потом `add`. Если убрать `remove`, каждое открытие модалки добавляет новый слушатель. После 10 открытий одно нажатие стрелки вызывало бы `navigateGallery` 10 раз.

Аналогично в `notification.html` функция `startTimer()` делает `clearTimeout` перед `setTimeout` — чтобы не запускать два таймера параллельно.

### 4. Перезагрузка notification.html после скрытия

```rust
let _ = window.navigate(tauri::Url::parse("tauri://localhost/notification.html").unwrap());
```

Это сброс JS-состояния окна уведомления. Без этого переменная `timerId` в `notification.html` сохраняла бы старое значение между появлениями. WebView не пересоздаётся при `window.hide()` — его JS продолжает существовать.

### 5. Миграция exercise_media: удаление только "наших" файлов

`delete_if_in_media_dir` — защита от удаления пользовательских файлов в старых упражнениях. Если упражнение создавалось до версии, где появилось копирование, его `audio_path`/`video_path` могли указывать прямо на исходные файлы пользователя (например, `C:\Users\user\Music\track.mp3`). При удалении такое упражнение не должно стирать этот файл.

### 6. Двойное воспроизведение звука при завершении таймера

В `timer.js` на событии `timer-finished`:
```js
playSound()
invoke('show_notification', { ... })
```

Звук воспроизводится в главном окне через JS. Одновременно Rust вызывает `show_notification_window` из фонового потока. В `debug.js` тест-кнопка делает то же самое. Логика не дублируется — это единственная точка воспроизведения.

### 7. `ignoreTicks` — флаг в JS для устаревших событий

Проблема: после нажатия "Стоп" фоновый поток ещё может прислать несколько тиков, пока не проверит флаг. JS устанавливает `ignoreTicks = true` синхронно и игнорирует все последующие тики. Это graceful stop без race condition на стороне JS.

### 8. Отсутствие сохранения длительности таймера

Значение `duration_secs` хранится только в `AppState` (в памяти). При перезапуске приложения оно сбрасывается в 30 минут. Значение в `<input type="number" id="duration-input">` (HTML) также сбрасывается в `value="30"`. LocalStorage для этого не используется (в отличие от пути звука). Если добавить сохранение — нужно синхронизировать начальное значение в input при загрузке.

### 9. Порядок builtin-упражнений захардкожен

`builtin_exercises()` — просто вектор с захардкожеными данными. Добавление нового builtin-упражнения требует редактирования Rust-кода и перекомпиляции. Картинки должны лежать в `src/assets/exercises/`.

### 10. Нет проверки дубликатов при добавлении упражнений

`add_user_exercise` не проверяет, есть ли уже упражнение с таким же названием. Пользователь может создать несколько упражнений "Разминка плеч" — они получат разные id (timestamp) и будут сосуществовать.
