//! Strata's preference declaration. The mechanics (file, validation,
//! commands, event) are `preset-preferences`; this file is only what Strata
//! has knobs for and what they default to.
//!
//! Ids are `section.key` and are the wire format the frontend uses too.

use std::path::Path;

use preset_preferences::{options, pref, section, Kind, Schema};

pub const RETENTION_DAYS: &str = "library.retention_days";
pub const DEFAULT_SORT: &str = "library.default_sort";
pub const CONCURRENCY: &str = "ingest.concurrency";
pub const EXTENSION_ALLOW_LIST: &str = "ingest.extension_allow_list";
pub const KEEP_SOURCE_FILES: &str = "ingest.keep_source_files";
pub const STORE_ROOT: &str = "catalog.store_root";
pub const DB_PATH: &str = "catalog.db_path";
pub const THEME: &str = "appearance.theme";

pub const FILE_VERSION: u32 = 1;

/// Built at startup because the catalog paths are resolved then.
pub fn schema(store_root: &Path, db_path: &Path) -> Schema {
    let cpus = num_cpus::get() as i64;
    Schema {
        version: FILE_VERSION,
        sections: vec![
            section("library", "Library"),
            section("ingest", "Ingest"),
            section("catalog", "Catalog"),
            section("appearance", "Appearance"),
        ],
        prefs: vec![
            pref(
                RETENTION_DAYS,
                "Keep trashed images for",
                Kind::Int { min: Some(1), max: Some(365) },
                30,
            )
            .help("Days before the Trash is emptied for good. Checked at launch."),
            pref(
                DEFAULT_SORT,
                "Default sort",
                Kind::Select {
                    options: options(&[
                        ("imported", "Imported"),
                        ("filename", "Filename"),
                        ("created", "Created"),
                        ("updated", "Updated"),
                        ("colour", "Colour"),
                        ("orientation", "Orientation"),
                    ]),
                },
                "imported",
            )
            .help("Sort the Library opens with. Changing the sort in the rail is remembered separately."),
            pref(
                CONCURRENCY,
                "Parallel imports",
                Kind::Int { min: Some(1), max: Some(cpus.max(1)) },
                (cpus / 2).max(1),
            )
            .help("Files processed at once. Takes effect on the next import."),
            pref(
                EXTENSION_ALLOW_LIST,
                "File types",
                Kind::List,
                &["jpg", "jpeg", "png"][..],
            )
            .help("Extensions an import picks up. Only JPEG and PNG can be decoded, so this narrows, it does not widen."),
            pref(
                KEEP_SOURCE_FILES,
                "Leave source files in place",
                Kind::Bool,
                true,
            )
            .help("Off: a source file moves to the system Trash once it is imported or found to be a duplicate."),
            pref(
                STORE_ROOT,
                "Store",
                Kind::ReadonlyPath,
                store_root.to_string_lossy().as_ref(),
            ),
            pref(
                DB_PATH,
                "Catalog database",
                Kind::ReadonlyPath,
                db_path.to_string_lossy().as_ref(),
            ),
            pref(
                THEME,
                "Theme",
                Kind::Select {
                    options: options(&[("system", "System"), ("light", "Light"), ("dark", "Dark")]),
                },
                "system",
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declaration_is_well_formed() {
        schema(Path::new("/tmp/store"), Path::new("/tmp/strata.duckdb")).assert_valid();
    }
}
