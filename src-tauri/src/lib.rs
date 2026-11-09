use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, WebviewUrl, WebviewWindowBuilder,
};

pub mod engine;
mod pet;
pub mod render;

use pet::{PetConfig, PetHandle};

const PET_SIZE: f64 = 180.0;
const CONFIG_FILE: &str = "config.json";

/*
 * Rust owns what the pet looks like, not the frontend: the pet window is created
 * in `setup()`, before any webview exists that could be asked. The file is plain
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
    /// `plat` (the flat fill), `argile` or `peluche`. Unknown values read flat.
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

/// Tauri exits when its last window closes, and `AppHandle::exit` reports the
/// same `ExitRequested` event. This flag is what tells an explicit Quit from the
/// settings window merely being closed — closing Settings must not stop the pet.
static QUITTING: AtomicBool = AtomicBool::new(false);

fn request_quit(app: &AppHandle) {
    QUITTING.store(true, Ordering::SeqCst);
    app.exit(0);
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    request_quit(&app);
}

#[tauri::command]
fn get_config(app: AppHandle) -> Config {
    load_config(&app)
}

/// Persist and apply in one step. The pet is a native window now, so a change is
/// handed to its thread and applied in place — no reload, no flash, no relayout.
#[tauri::command]
fn set_config(app: AppHandle, config: Config) -> Result<Config, String> {
    save_config(&app, &config)?;

    if let Some(pet) = app.try_state::<PetHandle>() {
        pet.set(PetConfig::from_raw(
            &config.shape,
            &config.expression,
            &config.color,
            &config.style,
        ));
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
            "quit" => request_quit(app),
            "settings" => {
                let _ = open_settings(app);
            }
            // One native pet for now; the multi-pet list is a later phase.
            "add-pet" => {}
            _ => {}
        })
        .build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![quit_app, get_config, set_config])
        .setup(|app| {
            let handle = app.handle().clone();
            build_tray(&handle)?;

            let config = load_config(&handle);
            let (x, y) = pet_anchor(&handle);
            let scale = handle
                .primary_monitor()
                .ok()
                .flatten()
                .map(|m| m.scale_factor())
                .unwrap_or(1.0);
            let size = (PET_SIZE * scale).round() as i32;
            let pet = pet::spawn(
                PetConfig::from_raw(
                    &config.shape,
                    &config.expression,
                    &config.color,
                    &config.style,
                ),
                (x * scale).round() as i32,
                (y * scale).round() as i32,
                size,
                scale,
            );
            app.manage(pet);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Dot");

    app.run(|_app, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            // Settings is the only Tauri window. Closing it must not take the
            // native pet down; only an explicit Quit sets QUITTING.
            if !QUITTING.load(Ordering::SeqCst) {
                api.prevent_exit();
            }
        }
    });
}

/*
 * The config file is the app's only persistence, and it is hand-editable, so the
 * rules worth pinning are the ones a future field could break: an absent file or
 * field falls back rather than failing, and an id we do not know is kept as-is
 * for the frontend to resolve.
 */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_file_gets_the_default_pet() {
        let config: Config = serde_json::from_str("{}").unwrap();
        assert_eq!(config.shape, "cercle");
        assert_eq!(config.color, "encre");
        assert_eq!(config.expression, "neutre");
        assert_eq!(config.style, "plat");
    }

    #[test]
    fn a_file_from_before_styles_existed_still_loads_flat() {
        let config: Config = serde_json::from_str(
            r#"{"shape":"triangle","color":"violet","expression":"somnolent"}"#,
        )
        .unwrap();
        assert_eq!(config.shape, "triangle");
        assert_eq!(config.style, "plat");
    }

    #[test]
    fn an_unknown_style_is_kept_for_the_frontend_to_resolve() {
        let config: Config = serde_json::from_str(r#"{"style":"glass"}"#).unwrap();
        assert_eq!(config.style, "glass");
    }
}
