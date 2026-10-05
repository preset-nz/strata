//! Projects: the on-disk work projects the family shares (guidance
//! `design/work-projects.md`), seen from Strata. A project is a folder with a
//! `project.preset` marker; its key, name and description live in that file,
//! which is the source (epic 08 decisions, 2026-10-05). Strata keeps only its
//! own curation per key: favourite, archived, images added by hand, and the
//! project's saved searches. Images imported from under the folder belong to
//! the project without being added.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

use crate::history::Step;

pub const MARKER: &str = "project.preset";

/// What `project.preset` says. Other apps may add keys; they're kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Marker {
    pub key: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
}

/// A project as Strata shows it: the marker, where it is, and Strata's curation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Project {
    pub key: String,
    pub name: String,
    pub description: String,
    pub folder: String,
    pub favourite: bool,
    pub archived: bool,
    /// Live images: added by hand, or imported from under the folder.
    pub count: i64,
}

pub fn read_marker(dir: &Path) -> Option<Marker> {
    let text = std::fs::read_to_string(dir.join(MARKER)).ok()?;
    let mut marker: Marker = toml::from_str(&text).ok()?;
    if marker.key.trim().is_empty() {
        return None;
    }
    if marker.name.trim().is_empty() {
        marker.name = dir.file_name()?.to_string_lossy().into_owned();
    }
    Some(marker)
}

/// Every project directly under the roots, by marker. Sorted by name.
pub fn discover(roots: &[PathBuf]) -> Vec<(Marker, PathBuf)> {
    let mut found: Vec<(Marker, PathBuf)> = roots
        .iter()
        .filter_map(|root| std::fs::read_dir(root).ok())
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| read_marker(&e.path()).map(|m| (m, e.path())))
        .collect();
    found.sort_by(|a, b| a.0.name.to_lowercase().cmp(&b.0.name.to_lowercase()));
    found
}

pub fn folder_of(roots: &[PathBuf], key: &str) -> Option<PathBuf> {
    discover(roots).into_iter().find(|(m, _)| m.key == key).map(|(_, p)| p)
}

/// A project key from a name: lowercase ASCII, accents folded, `-` between words.
pub fn key_for(name: &str) -> String {
    let folded: String = name.nfd().filter(|c| !is_combining_mark(*c)).collect();
    folded
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("-")
}

/// File › New Project…: `<root>/<name>/project.preset`, and nothing else. Apps
/// add their own folders when they join (work-projects decision 2). Refuses a
/// folder that exists or a key another project has.
pub fn create_on_disk(root: &Path, name: &str, description: &str) -> Result<(Marker, PathBuf)> {
    let name = name.trim();
    let key = key_for(name);
    if key.is_empty() {
        bail!("a project needs a name with letters or digits");
    }
    if name.contains('/') || name.starts_with('.') {
        bail!("a project name can't contain / or start with a dot");
    }
    if discover(&[root.to_path_buf()]).iter().any(|(m, _)| m.key == key) {
        bail!("a project with the key {key:?} already exists");
    }
    let folder = root.join(name);
    if folder.exists() {
        bail!("{} already exists", folder.display());
    }
    std::fs::create_dir_all(&folder).with_context(|| format!("creating {}", folder.display()))?;
    let marker = Marker { key, name: name.to_string(), description: description.trim().to_string() };
    std::fs::write(folder.join(MARKER), toml::to_string(&marker)?)?;
    Ok((marker, folder))
}

/// The catalog side of every discovered project.
pub fn list(conn: &Connection, roots: &[PathBuf]) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare("SELECT favourite, archived_at IS NOT NULL FROM project_curation WHERE key = ?")?;
    let mut out = Vec::new();
    for (marker, folder) in discover(roots) {
        let (favourite, archived) = stmt
            .query_row(params![marker.key], |r| Ok((r.get::<_, bool>(0)?, r.get::<_, bool>(1)?)))
            .optional()?
            .unwrap_or((false, false));
        let (clause, bound) = membership(&marker.key, Some(&folder));
        let count: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM images i WHERE i.deleted_at IS NULL AND {clause}"),
            rusqlite::params_from_iter(bound.iter()),
            |r| r.get(0),
        )?;
        out.push(Project {
            key: marker.key,
            name: marker.name,
            description: marker.description,
            folder: folder.to_string_lossy().into_owned(),
            favourite,
            archived,
            count,
        });
    }
    Ok(out)
}

/// The SQL that says an image (`i`) belongs to the project: added by hand,
/// or imported from a path under its folder. The folder test is a range, so
/// `/Projects/Bark` doesn't catch `/Projects/Bark Two`.
pub fn membership(key: &str, folder: Option<&Path>) -> (String, Vec<rusqlite::types::Value>) {
    let Some(folder) = folder else {
        // The folder is gone (moved, or on an unmounted disk): hand-added images only.
        return (
            "i.id IN (SELECT image_id FROM project_member WHERE key = ?)".to_string(),
            vec![rusqlite::types::Value::Text(key.to_string())],
        );
    };
    let prefix = format!("{}/", folder.to_string_lossy().trim_end_matches('/'));
    let upper = format!("{}0", &prefix[..prefix.len() - 1]); // '/' + 1 == '0'
    (
        "(i.id IN (SELECT image_id FROM project_member WHERE key = ?) \
          OR (i.original_path >= ? AND i.original_path < ?))"
            .to_string(),
        vec![
            rusqlite::types::Value::Text(key.to_string()),
            rusqlite::types::Value::Text(prefix),
            rusqlite::types::Value::Text(upper),
        ],
    )
}

#[derive(Clone, Copy)]
struct Flags {
    favourite: bool,
    archived: bool,
}

fn flags(conn: &Connection, key: &str) -> Result<Flags> {
    Ok(conn
        .query_row(
            "SELECT favourite, archived_at IS NOT NULL FROM project_curation WHERE key = ?",
            params![key],
            |r| Ok(Flags { favourite: r.get(0)?, archived: r.get(1)? }),
        )
        .optional()?
        .unwrap_or(Flags { favourite: false, archived: false }))
}

fn set_flags(conn: &Connection, key: &str, f: Flags) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO project_curation (key, favourite, archived_at, updated_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(key) DO UPDATE SET favourite = excluded.favourite,
             archived_at = excluded.archived_at, updated_at = excluded.updated_at",
        params![key, f.favourite, f.archived.then_some(now.clone()), now],
    )?;
    Ok(())
}

fn flag_step(conn: &Connection, key: &str, label: String, change: impl Fn(Flags) -> Flags) -> Result<Option<Step>> {
    let before = flags(conn, key)?;
    let after = change(before);
    if (before.favourite, before.archived) == (after.favourite, after.archived) {
        return Ok(None);
    }
    set_flags(conn, key, after)?;
    let (k1, k2) = (key.to_string(), key.to_string());
    Ok(Some(Step {
        label,
        undo: Box::new(move |conn| set_flags(conn, &k1, before)),
        redo: Box::new(move |conn| set_flags(conn, &k2, after)),
    }))
}

pub fn set_favourite(conn: &Connection, key: &str, name: &str, on: bool) -> Result<Option<Step>> {
    let verb = if on { "Favourite" } else { "Unfavourite" };
    flag_step(conn, key, format!("{verb} Project \u{201c}{name}\u{201d}"), |f| Flags { favourite: on, ..f })
}

pub fn set_archived(conn: &Connection, key: &str, name: &str, on: bool) -> Result<Option<Step>> {
    let verb = if on { "Archive" } else { "Unarchive" };
    flag_step(conn, key, format!("{verb} Project \u{201c}{name}\u{201d}"), |f| Flags { archived: on, ..f })
}

fn put(conn: &Connection, key: &str, images: &[String]) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let tx = conn.unchecked_transaction()?;
    for image in images {
        tx.execute(
            "INSERT OR IGNORE INTO project_member (key, image_id, added_at) VALUES (?, ?, ?)",
            params![key, image, now],
        )?;
    }
    tx.commit()?;
    Ok(())
}

fn take(conn: &Connection, key: &str, images: &[String]) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    for image in images {
        tx.execute("DELETE FROM project_member WHERE key = ? AND image_id = ?", params![key, image])?;
    }
    tx.commit()?;
    Ok(())
}

fn added_among(conn: &Connection, key: &str, images: &[String]) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT 1 FROM project_member WHERE key = ? AND image_id = ?")?;
    let mut found = Vec::new();
    for image in images {
        if stmt.exists(params![key, image])? {
            found.push(image.clone());
        }
    }
    Ok(found)
}

fn images_word(n: usize) -> String {
    if n == 1 { "Image".into() } else { format!("{n} Images") }
}

/// Adds images by hand. Images already added are skipped; `None` if all were.
pub fn add(conn: &Connection, key: &str, name: &str, images: &[String]) -> Result<Option<Step>> {
    let already = added_among(conn, key, images)?;
    let new: Vec<String> = images.iter().filter(|i| !already.contains(i)).cloned().collect();
    if new.is_empty() {
        return Ok(None);
    }
    put(conn, key, &new)?;
    let (k1, k2, n1, n2) = (key.to_string(), key.to_string(), new.clone(), new.clone());
    Ok(Some(Step {
        label: format!("Add {} to Project \u{201c}{name}\u{201d}", images_word(new.len())),
        undo: Box::new(move |conn| take(conn, &k1, &n1)),
        redo: Box::new(move |conn| put(conn, &k2, &n2)),
    }))
}

/// Removes images added by hand. Images that belong because they sit under
/// the folder can't be removed here; they stay. `None` if nothing changed.
pub fn remove(conn: &Connection, key: &str, name: &str, images: &[String]) -> Result<Option<Step>> {
    let present = added_among(conn, key, images)?;
    if present.is_empty() {
        return Ok(None);
    }
    take(conn, key, &present)?;
    let (k1, k2, p1, p2) = (key.to_string(), key.to_string(), present.clone(), present.clone());
    Ok(Some(Step {
        label: format!("Remove {} from Project \u{201c}{name}\u{201d}", images_word(present.len())),
        undo: Box::new(move |conn| put(conn, &k1, &p1)),
        redo: Box::new(move |conn| take(conn, &k2, &p2)),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    fn scratch() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("strata-proj-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn keys_from_names() {
        assert_eq!(key_for("Night Drive"), "night-drive");
        assert_eq!(key_for("Café Ōtautahi 2"), "cafe-otautahi-2");
        assert_eq!(key_for("  — "), "");
    }

    #[test]
    fn create_discover_and_refuse_duplicates() {
        let root = scratch();
        let (marker, folder) = create_on_disk(&root, "Night Drive", " A song ").unwrap();
        assert_eq!(marker.key, "night-drive");
        assert!(folder.join(MARKER).exists());
        assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 1, "only the marker");
        assert!(create_on_disk(&root, "Night Drive", "").is_err());
        assert!(create_on_disk(&root, "night drive", "").is_err(), "same key");
        assert!(create_on_disk(&root, "a/b", "").is_err());

        // A project made by another app, with keys Strata doesn't know.
        let other = root.join("Elsewhere");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join(MARKER), "key = \"else\"\napps = [\"shard\"]\n").unwrap();
        std::fs::create_dir_all(root.join("Not a project")).unwrap();
        let found = discover(&[root.clone()]);
        let names: Vec<&str> = found.iter().map(|(m, _)| m.name.as_str()).collect();
        assert_eq!(names, ["Elsewhere", "Night Drive"], "name falls back to the folder");
        assert_eq!(folder_of(&[root.clone()], "night-drive"), Some(folder));
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn curation_membership_and_undo() {
        let root = scratch();
        let (_, folder) = create_on_disk(&root, "Bark", "").unwrap();
        let db = Db::open(&root.join("catalog.sqlite")).unwrap();
        let conn = db.0.lock().unwrap();
        let insert = |id: &str, path: &str| {
            conn.execute(
                "INSERT INTO images (id, content_hash, store_path, original_filename, original_path, byte_size, mime, imported_at, ingest_batch_id)
                 VALUES (?, ?, 's', 'f', ?, 1, 'image/png', ?, 'b')",
                params![id, id, path, Utc::now()],
            )
            .unwrap();
        };
        insert("under", &format!("{}/Shard/Exports/a.png", folder.display()));
        insert("sibling", &format!("{} Two/a.png", folder.display()));
        insert("loose", "/elsewhere/b.png");

        let roots = vec![root.clone()];
        let count = |conn: &Connection| list(conn, &roots).unwrap()[0].count;
        assert_eq!(count(&conn), 1, "under the folder only, not the sibling");

        let add_step = add(&conn, "bark", "Bark", &["loose".into(), "under".into()]).unwrap().unwrap();
        assert_eq!(add_step.label, "Add 2 Images to Project \u{201c}Bark\u{201d}");
        assert_eq!(count(&conn), 2);
        (add_step.undo)(&conn).unwrap();
        assert_eq!(count(&conn), 1);
        (add_step.redo)(&conn).unwrap();
        assert!(remove(&conn, "bark", "Bark", &["sibling".into()]).unwrap().is_none());

        let fav = set_favourite(&conn, "bark", "Bark", true).unwrap().unwrap();
        let arch = set_archived(&conn, "bark", "Bark", true).unwrap().unwrap();
        let p = &list(&conn, &roots).unwrap()[0];
        assert!(p.favourite && p.archived);
        (arch.undo)(&conn).unwrap();
        (fav.undo)(&conn).unwrap();
        let p = &list(&conn, &roots).unwrap()[0];
        assert!(!p.favourite && !p.archived);
        assert!(set_archived(&conn, "bark", "Bark", false).unwrap().is_none());
        drop(conn);
        std::fs::remove_dir_all(root).ok();
    }
}
