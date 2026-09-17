//! The native menu bar. Rule 1 of `guidance/design/native-apps.md`: every
//! command lives here with its accelerator, and each item is forwarded to
//! the webview as one event that maps to one handler.
//!
//! Strata has few commands yet. The App menu carries Settings on Cmd+, and
//! the Edit menu carries the OS text-editing items so fields behave. This
//! file is the hand-built stand-in until the shared native-menu package
//! exists; the event name is the one `@preset.nz/preferences` listens for.

use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Runtime};

const EVT_APP_SETTINGS: &str = "menu://app/settings";

pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let name = "Strata".to_string();
    let version = app.package_info().version.to_string();

    let about = AboutMetadata {
        name: Some(name.clone()),
        version: Some(version),
        ..Default::default()
    };

    let settings = MenuItem::with_id(app, "app-settings", "Settings…", true, Some("CmdOrCtrl+,"))?;

    let app_menu = Submenu::with_items(
        app,
        &name,
        true,
        &[
            &PredefinedMenuItem::about(app, None, Some(about))?,
            &PredefinedMenuItem::separator(app)?,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;

    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;

    let window_menu = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;

    let menu = Menu::with_items(app, &[&app_menu, &edit_menu, &window_menu])?;
    app.set_menu(menu)?;

    app.on_menu_event(|handle, event| {
        let evt = match event.id().0.as_str() {
            "app-settings" => EVT_APP_SETTINGS,
            _ => return,
        };
        let _ = handle.emit(evt, ());
    });
    Ok(())
}
