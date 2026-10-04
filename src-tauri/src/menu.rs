//! Strata's commands for the native menu. app-kit builds the menu bar from
//! these and the built-in items `menu.toml` switches on (guidance
//! `design/native-apps.md`, rule 1); each item reaches the webview as one
//! `command` event, bound by id in `App.tsx`.

use preset_app_kit::{AppKit, Command, MenuName};
use tauri::{AppHandle, Emitter, Runtime};

use crate::history::Curation;

pub const MENU_CONFIG: &str = include_str!("../menu.toml");

/// `@preset.nz/preferences` opens its window on this event.
const SETTINGS_EVENT: &str = "menu://app/settings";

fn commands() -> Vec<Command> {
    vec![
        Command::item("edit.find", "Find…")
            .accelerator("CmdOrCtrl+F")
            .menu(MenuName::Edit)
            .section(2),
        Command::item("edit.save_search", "Save Search…")
            .menu(MenuName::Edit)
            .section(2),
        Command::toggle("view.palette_markers", "Palette Markers")
            .accelerator("CmdOrCtrl+Shift+M")
            .menu(MenuName::View)
            .section(0),
    ]
}

pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    AppKit::<R>::new("Strata")
        .menu_config(MENU_CONFIG)
        .commands(commands())
        .on_command(|app, id| match id {
            "app.settings" => {
                let _ = app.emit(SETTINGS_EVENT, ());
                true
            }
            _ => false,
        })
        .install_history::<Curation>(app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_config_parses_and_has_no_document_slots() {
        let config = preset_app_kit::MenuConfig::parse(MENU_CONFIG).unwrap();
        assert!(config.file.document_slots_on().is_empty());
        assert!(!config.file.close);
        assert!(config.app.settings);
    }
}
