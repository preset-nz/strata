//! Strata's commands for the native menu. app-kit builds the menu bar from
//! these and the built-in items `menu.toml` switches on (guidance
//! `design/native-apps.md`, rule 1); each item reaches the webview as one
//! `command` event, bound by id in `App.tsx`. Domain menus (Image) sit
//! between View and Window.

use preset_app_kit::{AppKit, Command, MenuName};
use tauri::{AppHandle, Emitter, Runtime};

use crate::history::Curation;

pub const MENU_CONFIG: &str = include_str!("../menu.toml");

/// `@preset.nz/preferences` opens its window on this event.
const SETTINGS_EVENT: &str = "menu://app/settings";

fn commands() -> Vec<Command> {
    vec![
        // The one File command: Strata has no documents (menu.toml), but it
        // makes the work projects the family shares (work-projects.md).
        // No shortcut: none is conventional, and Option+Cmd+N is a global
        // hotkey in some browsers (Arc's Little Arc), which takes it first.
        Command::item("file.new_project", "New Project…")
            .menu(MenuName::File)
            .section(0),
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
            .section(0)
            .unchecked(),
    ]
    .into_iter()
    .chain(image_commands())
    .collect()
}

/// The Image menu: curation of the selected image, and collections. Bare keys, as Lightroom and
/// Photos do: F hearts, 1 to 7 label, 0 clears. The webview disables them while
/// a text field has focus, so typing an "f" never hearts anything.
fn image_commands() -> Vec<Command> {
    let mut commands = vec![Command::toggle("image.favourite", "Favourite")
        .accelerator("F")
        .domain("Image")
        .section(0)
        .unchecked()
        .disabled()];
    let labels = crate::curation::LABELS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let title = format!("{}{}", label[..1].to_uppercase(), &label[1..]);
            Command::item(&format!("image.label.{label}"), &title)
                .accelerator(&(i + 1).to_string())
                .domain("Image")
                .submenu("Colour Label")
                .section(1)
                .disabled()
        });
    commands.extend(labels);
    commands.push(
        Command::item("image.new_collection", "New Collection…")
            .accelerator("CmdOrCtrl+Shift+N")
            .domain("Image")
            .section(2),
    );
    commands.push(
        Command::item("image.remove_from_collection", "Remove from Collection")
            .domain("Image")
            .section(2)
            .disabled(),
    );
    commands.push(
        Command::item("image.label.none", "No Label")
            .accelerator("0")
            .domain("Image")
            .submenu("Colour Label")
            .section(1)
            .category("clear")
            .disabled(),
    );
    commands
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
