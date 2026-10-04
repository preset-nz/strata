//! The native menu bar. Rule 1 of `guidance/design/native-apps.md`: every
//! command lives here with its accelerator, and each item is forwarded to
//! the webview as one event that maps to one handler.
//!
//! Strata has few commands yet. The App menu carries Settings on Cmd+, and
//! the Edit menu carries the OS text-editing items so fields behave, plus
//! Find… on Cmd+F, which focuses the library search, and Save Search…. View
//! carries Palette Markers (Shift+Cmd+M) for Quickview. This
//! file is the hand-built stand-in until the shared native-menu package
//! exists; the event name is the one `@preset.nz/preferences` listens for.

use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Runtime};

const EVT_APP_SETTINGS: &str = "menu://app/settings";
const EVT_EDIT_FIND: &str = "menu://edit/find";
const EVT_EDIT_SAVE_SEARCH: &str = "menu://edit/save-search";
const EVT_VIEW_PALETTE_MARKERS: &str = "menu://view/palette-markers";

pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let name = "Strata".to_string();
    let version = app.package_info().version.to_string();

    let about = AboutMetadata {
        name: Some(name.clone()),
        version: Some(version),
        ..Default::default()
    };

    let settings = MenuItem::with_id(app, "app-settings", "Settings…", true, Some("CmdOrCtrl+,"))?;
    let find = MenuItem::with_id(app, "edit-find", "Find…", true, Some("CmdOrCtrl+F"))?;
    let save_search = MenuItem::with_id(app, "edit-save-search", "Save Search…", true, None::<&str>)?;

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
            &PredefinedMenuItem::separator(app)?,
            &find,
            &save_search,
        ],
    )?;

    let palette_markers = MenuItem::with_id(
        app,
        "view-palette-markers",
        "Palette Markers",
        true,
        Some("CmdOrCtrl+Shift+M"),
    )?;
    let view_menu = Submenu::with_items(app, "View", true, &[&palette_markers])?;

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

    let menu = Menu::with_items(app, &[&app_menu, &edit_menu, &view_menu, &window_menu])?;
    app.set_menu(menu)?;

    app.on_menu_event(|handle, event| {
        let evt = match event.id().0.as_str() {
            "app-settings" => EVT_APP_SETTINGS,
            "edit-find" => EVT_EDIT_FIND,
            "edit-save-search" => EVT_EDIT_SAVE_SEARCH,
            "view-palette-markers" => EVT_VIEW_PALETTE_MARKERS,
            _ => return,
        };
        let _ = handle.emit(evt, ());
    });
    Ok(())
}
