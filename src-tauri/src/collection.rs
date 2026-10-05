//! Collections: tight, curated sets of images with a name (epic 08 decisions,
//! 2026-10-05). An image can be in many. Every change is one undo step that
//! carries exactly what it changed, so undo never touches anything else.

use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::history::Step;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    /// Live (not trashed) members.
    pub count: i64,
}

pub fn list(conn: &Connection) -> Result<Vec<Collection>> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.name,
                (SELECT COUNT(*) FROM collection_member m JOIN images i ON i.id = m.image_id
                  WHERE m.collection_id = c.id AND i.deleted_at IS NULL)
           FROM collection c ORDER BY c.name COLLATE NOCASE, c.created_at",
    )?;
    let rows = stmt
        .query_map([], |r| Ok(Collection { id: r.get(0)?, name: r.get(1)?, count: r.get(2)? }))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn clean(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        bail!("a collection needs a name");
    }
    Ok(name.to_string())
}

fn name_of(conn: &Connection, id: &str) -> Result<String> {
    conn.query_row("SELECT name FROM collection WHERE id = ?", params![id], |r| r.get(0))
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("no collection {id}"))
}

/// A row as it was, for putting back.
#[derive(Clone)]
struct Saved {
    id: String,
    name: String,
    created_at: String,
    members: Vec<(String, String)>,
}

fn save(conn: &Connection, id: &str) -> Result<Saved> {
    let (name, created_at) = conn.query_row(
        "SELECT name, created_at FROM collection WHERE id = ?",
        params![id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let members = conn
        .prepare("SELECT image_id, added_at FROM collection_member WHERE collection_id = ? ORDER BY added_at")?
        .query_map(params![id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(Saved { id: id.to_string(), name, created_at, members })
}

fn restore(conn: &Connection, s: &Saved) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO collection (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
        params![s.id, s.name, s.created_at],
    )?;
    for (image, added) in &s.members {
        tx.execute(
            "INSERT OR IGNORE INTO collection_member (collection_id, image_id, added_at) VALUES (?, ?, ?)",
            params![s.id, image, added],
        )?;
    }
    tx.commit()?;
    Ok(())
}

fn drop_collection(conn: &Connection, id: &str) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM collection_member WHERE collection_id = ?", params![id])?;
    tx.execute("DELETE FROM collection WHERE id = ?", params![id])?;
    tx.commit()?;
    Ok(())
}

/// Creates a collection holding `images` (possibly none). Returns its id and the step.
pub fn create(conn: &Connection, name: &str, images: &[String]) -> Result<(String, Step)> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let saved = Saved {
        id: id.clone(),
        name: clean(name)?,
        created_at: now.clone(),
        members: images.iter().map(|i| (i.clone(), now.clone())).collect(),
    };
    restore(conn, &saved)?;
    let undo_id = id.clone();
    let step = Step {
        label: format!("New Collection \u{201c}{}\u{201d}", saved.name),
        undo: Box::new(move |conn| drop_collection(conn, &undo_id)),
        redo: Box::new(move |conn| restore(conn, &saved)),
    };
    Ok((id, step))
}

pub fn rename(conn: &Connection, id: &str, name: &str) -> Result<Option<Step>> {
    let new = clean(name)?;
    let old = name_of(conn, id)?;
    if new == old {
        return Ok(None);
    }
    let set = |id: String, name: String| -> Box<dyn Fn(&Connection) -> Result<()> + Send + Sync> {
        Box::new(move |conn| {
            conn.execute(
                "UPDATE collection SET name = ?, updated_at = ? WHERE id = ?",
                params![name, Utc::now().to_rfc3339(), id],
            )?;
            Ok(())
        })
    };
    let redo = set(id.to_string(), new.clone());
    redo(conn)?;
    Ok(Some(Step {
        label: "Rename Collection".into(),
        undo: set(id.to_string(), old),
        redo,
    }))
}

pub fn delete(conn: &Connection, id: &str) -> Result<Step> {
    let saved = save(conn, id)?;
    drop_collection(conn, id)?;
    let gone = saved.id.clone();
    Ok(Step {
        label: format!("Delete Collection \u{201c}{}\u{201d}", saved.name),
        undo: Box::new(move |conn| restore(conn, &saved)),
        redo: Box::new(move |conn| drop_collection(conn, &gone)),
    })
}

fn put_members(conn: &Connection, id: &str, images: &[String]) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let tx = conn.unchecked_transaction()?;
    for image in images {
        tx.execute(
            "INSERT OR IGNORE INTO collection_member (collection_id, image_id, added_at) VALUES (?, ?, ?)",
            params![id, image, now],
        )?;
    }
    tx.commit()?;
    Ok(())
}

fn take_members(conn: &Connection, id: &str, images: &[String]) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    for image in images {
        tx.execute(
            "DELETE FROM collection_member WHERE collection_id = ? AND image_id = ?",
            params![id, image],
        )?;
    }
    tx.commit()?;
    Ok(())
}

fn members_among(conn: &Connection, id: &str, images: &[String]) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT 1 FROM collection_member WHERE collection_id = ? AND image_id = ?")?;
    let mut found = Vec::new();
    for image in images {
        if stmt.exists(params![id, image])? {
            found.push(image.clone());
        }
    }
    Ok(found)
}

/// Adds the images not already in it. `None` when all were.
pub fn add(conn: &Connection, id: &str, images: &[String]) -> Result<Option<Step>> {
    let name = name_of(conn, id)?;
    let already = members_among(conn, id, images)?;
    let new: Vec<String> = images.iter().filter(|i| !already.contains(i)).cloned().collect();
    if new.is_empty() {
        return Ok(None);
    }
    put_members(conn, id, &new)?;
    let (a, b) = (id.to_string(), id.to_string());
    let (n1, n2) = (new.clone(), new.clone());
    Ok(Some(Step {
        label: format!("Add {} to \u{201c}{name}\u{201d}", images_word(new.len())),
        undo: Box::new(move |conn| take_members(conn, &a, &n1)),
        redo: Box::new(move |conn| put_members(conn, &b, &n2)),
    }))
}

/// Removes the images that are in it. `None` when none were.
pub fn remove(conn: &Connection, id: &str, images: &[String]) -> Result<Option<Step>> {
    let name = name_of(conn, id)?;
    let present = members_among(conn, id, images)?;
    if present.is_empty() {
        return Ok(None);
    }
    take_members(conn, id, &present)?;
    let (a, b) = (id.to_string(), id.to_string());
    let (p1, p2) = (present.clone(), present.clone());
    Ok(Some(Step {
        label: format!("Remove {} from \u{201c}{name}\u{201d}", images_word(present.len())),
        undo: Box::new(move |conn| put_members(conn, &a, &p1)),
        redo: Box::new(move |conn| take_members(conn, &b, &p2)),
    }))
}

fn images_word(n: usize) -> String {
    if n == 1 { "Image".into() } else { format!("{n} Images") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    fn catalog() -> (Db, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("strata-coll-{}", uuid::Uuid::new_v4()));
        (Db::open(&dir.join("catalog.sqlite")).unwrap(), dir)
    }

    fn members(conn: &Connection, id: &str) -> Vec<String> {
        conn.prepare("SELECT image_id FROM collection_member WHERE collection_id = ? ORDER BY image_id")
            .unwrap()
            .query_map(params![id], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<String>>>()
            .unwrap()
    }

    fn v(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn create_add_remove_rename_delete_with_undo() {
        let (db, dir) = catalog();
        let conn = db.0.lock().unwrap();

        let (id, created) = create(&conn, " Bark ", &v(&["a"])).unwrap();
        assert_eq!(created.label, "New Collection \u{201c}Bark\u{201d}");
        assert_eq!(list(&conn).unwrap()[0].name, "Bark");

        let added = add(&conn, &id, &v(&["a", "b", "c"])).unwrap().unwrap();
        assert_eq!(added.label, "Add 2 Images to \u{201c}Bark\u{201d}");
        assert_eq!(members(&conn, &id), v(&["a", "b", "c"]));
        (added.undo)(&conn).unwrap();
        assert_eq!(members(&conn, &id), v(&["a"]), "undo takes back only what it added");
        (added.redo)(&conn).unwrap();
        assert!(add(&conn, &id, &v(&["a"])).unwrap().is_none());

        let removed = remove(&conn, &id, &v(&["b", "z"])).unwrap().unwrap();
        assert_eq!(removed.label, "Remove Image from \u{201c}Bark\u{201d}");
        (removed.undo)(&conn).unwrap();
        assert_eq!(members(&conn, &id), v(&["a", "b", "c"]));

        let renamed = rename(&conn, &id, "Wet bark").unwrap().unwrap();
        (renamed.undo)(&conn).unwrap();
        assert_eq!(list(&conn).unwrap()[0].name, "Bark");
        assert!(rename(&conn, &id, "Bark").unwrap().is_none());
        assert!(rename(&conn, &id, "  ").is_err());

        let deleted = delete(&conn, &id).unwrap();
        assert!(list(&conn).unwrap().is_empty());
        (deleted.undo)(&conn).unwrap();
        assert_eq!(members(&conn, &id), v(&["a", "b", "c"]), "members come back with it");
        (deleted.redo)(&conn).unwrap();
        assert!(list(&conn).unwrap().is_empty());

        (created.undo)(&conn).ok();
        drop(conn);
        std::fs::remove_dir_all(dir).ok();
    }
}
