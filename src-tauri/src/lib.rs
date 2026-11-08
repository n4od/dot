use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, Window,
};

const PET_LABEL: &str = "pet-1";
const PET_SIZE: f64 = 180.0;
const CONFIG_FILE: &str = "config.json";

/*
 * Rust owns what the pet looks like, not the frontend: `create_pet` runs in
 * `setup()`, before any webview exists that could be asked. The file is plain
 * JSON in the app config directory, and its ids are the ones the frontend
 * catalogues use (`src/bot/skins.ts`, `src/bot/expressions.ts`).
 *
 * Those four defaults are duplicated from the catalogues on purpose. The pet
 * resolves an id it does not know to a default, so a hand-edited or stale file
 * degrades to a sensible shape instead of refusing to start.
 */
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
struct Config {
    shape: String,
    color: String,
    expression: String,
    /// `plat` (the flat fill) or `argile` (lit). Unknown values read as `plat`.
    style: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            shape: "cercle".into(),
            color: "encre".into(),
            expression: "neutre".into(),
            style: "plat".into(),
        }
    }
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join(CONFIG_FILE))
        .map_err(|e| e.to_string())
}

/// A missing or unreadable file is not an error: the pet still has to look like
/// something, so the fallback is the default pet.
fn load_config(app: &AppHandle) -> Config {
    let Ok(path) = config_path(app) else {
        return Config::default();
    };
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_config(app: &AppHandle, config: &Config) -> Result<(), String> {
    let path = config_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| e.to_string())
}

/// Ids are `[a-z]+` in practice. Anything else is dropped so that a hand-edited
/// file cannot smuggle a second query parameter into the window URL.
fn param(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

fn pet_query(config: &Config) -> String {
    format!(
        "?window=pet&shape={}&color={}&expression={}&style={}",
        param(&config.shape),
        param(&config.color),
        param(&config.expression),
        param(&config.style)
    )
}

/// Physical-pixel geometry handed to the pet window so it can turn the global
/// cursor position into a look direction, and test the cursor against the
/// painted silhouette.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PetGeometry {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    scale: f64,
    mon_w: f64,
    mon_h: f64,
}

#[tauri::command]
fn pet_geometry(window: Window) -> Result<PetGeometry, String> {
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let pos = window.outer_position().map_err(|e| e.to_string())?;
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let monitor = window.current_monitor().map_err(|e| e.to_string())?;

    let (mon_w, mon_h) = match monitor {
        Some(m) => (m.size().width as f64, m.size().height as f64),
        None => (1920.0, 1080.0),
    };

    Ok(PetGeometry {
        x: pos.x as f64,
        y: pos.y as f64,
        w: size.width as f64,
        h: size.height as f64,
        scale,
        mon_w,
        mon_h,
    })
}

/// A transparent window intercepts clicks over its whole rectangle. The webview
/// decides per frame whether the cursor is on the pet and flips this.
#[tauri::command]
fn set_click_through(window: Window, through: bool) -> Result<(), String> {
    window
        .set_ignore_cursor_events(through)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn start_drag(window: Window) -> Result<(), String> {
    window.start_dragging().map_err(|e| e.to_string())
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn get_config(app: AppHandle) -> Config {
    load_config(&app)
}

/// Persist and apply in one step. The pet view reads its appearance from the
/// query string, so re-opening it with the new parameters is all a change needs:
/// setting `location.search` reloads the document in place, which leaves the
/// window, its position and the engine's behaviour otherwise untouched.
#[tauri::command]
fn set_config(app: AppHandle, config: Config) -> Result<Config, String> {
    save_config(&app, &config)?;

    if let Some(window) = app.get_webview_window(PET_LABEL) {
        let query = serde_json::to_string(&pet_query(&config)).map_err(|e| e.to_string())?;
        let _ = window.eval(format!("location.search = {query}"));
    }

    Ok(config)
}

/// Bottom-right of the primary monitor, with a margin so it does not sit under
/// the taskbar.
fn pet_anchor(app: &AppHandle) -> (f64, f64) {
    let fallback = (900.0, 500.0);
    let Ok(Some(monitor)) = app.primary_monitor() else {
        return fallback;
    };
    let scale = monitor.scale_factor();
    let size = monitor.size().to_logical::<f64>(scale);
    let origin = monitor.position().to_logical::<f64>(scale);
    (
        origin.x + size.width - PET_SIZE - 40.0,
        origin.y + size.height - PET_SIZE - 80.0,
    )
}

fn create_pet(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window(PET_LABEL).is_some() {
        return Ok(());
    }

    let config = load_config(app);
    let (x, y) = pet_anchor(app);
    let window = WebviewWindowBuilder::new(
        app,
        PET_LABEL,
        WebviewUrl::App(format!("index.html{}", pet_query(&config)).into()),
    )
    .title("Dot")
    .inner_size(PET_SIZE, PET_SIZE)
    .position(x, y)
    .transparent(true)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .shadow(false)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .focused(false)
    .build()?;

    // Start transparent to input: the webview turns interaction back on as soon
    // as the cursor is over a painted pixel.
    let _ = window.set_ignore_cursor_events(true);
    Ok(())
}

fn open_settings(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    WebviewWindowBuilder::new(
        app,
        "settings",
        WebviewUrl::App("index.html?window=settings".into()),
    )
    .title("Dot - Settings")
    .inner_size(780.0, 600.0)
    .min_inner_size(660.0, 520.0)
    .center()
    .build()?;
    Ok(())
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let add = MenuItem::with_id(app, "add-pet", "Add a pet", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings...", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&add, &settings, &quit])?;

    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("Dot")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "quit" => app.exit(0),
            "settings" => {
                let _ = open_settings(app);
            }
            "add-pet" => {
                let _ = create_pet(app);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

/// The engine is clock-free, but the pet still needs to know where the cursor is
/// in screen space — something a web page can never see. A plain thread polls it
/// and forwards only actual movement.
fn start_cursor_poll(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last: Option<(f64, f64)> = None;
        loop {
            std::thread::sleep(Duration::from_millis(33));

            if app.get_webview_window(PET_LABEL).is_none() {
                continue;
            }

            let Ok(pos) = app.cursor_position() else {
                continue;
            };

            let moved = last
                .map(|(x, y)| (x - pos.x).abs() > 0.5 || (y - pos.y).abs() > 0.5)
                .unwrap_or(true);
            if !moved {
                continue;
            }

            last = Some((pos.x, pos.y));
            let _ = app.emit("pet://cursor", (pos.x, pos.y));
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            pet_geometry,
            set_click_through,
            start_drag,
            quit_app,
            get_config,
            set_config
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            build_tray(&handle)?;
            create_pet(&handle)?;
            start_cursor_poll(handle);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Dot");
}