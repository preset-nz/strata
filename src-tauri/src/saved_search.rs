//! Saved searches: a named query the person can re-run. The query is the
//! frontend's own JSON (a versioned envelope); the catalog stores it and
//! never interprets it, so the query shape can grow without a migration.

use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SavedSearch {
    pub id: String,
    pub name: String,
    pub query: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

pub fn list(conn: &Connection) -> Result<Vec<SavedSearch>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, query, created_at, updated_at FROM saved_search ORDER BY name COLLATE NOCASE, created_at",
    )?;
    let rows = stmt
        .query_map([], from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Creates the search, or replaces the name and query of the one with this
/// id. Restoring a deleted search passes its old id and creation time.
pub fn put(
    conn: &Connection,
    id: Option<&str>,
    name: &str,
    query: &serde_json::Value,
    created_at: Option<&str>,
) -> Result<SavedSearch> {
    let name = name.trim();
    if name.is_empty() {
        bail!("a saved search needs a name");
    }
    if !query.is_object() {
        bail!("a saved search's query must be an object");
    }
    let id = id.map(str::to_string).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO saved_search (id, name, query, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, query = excluded.query, updated_at = excluded.updated_at",
        params![id, name, query.to_string(), created_at.unwrap_or(&now), now],
    )?;
    Ok(conn.query_row(
        "SELECT id, name, query, created_at, updated_at FROM saved_search WHERE id = ?",
        params![id],
        from_row,
    )?)
}

/// Removes the search and hands it back, so the caller can offer Undo.
pub fn delete(conn: &Connection, id: &str) -> Result<Option<SavedSearch>> {
    let row = conn
        .query_row(
            "SELECT id, name, query, created_at, updated_at FROM saved_search WHERE id = ?",
            params![id],
            from_row,
        )
        .optional()?;
    conn.execute("DELETE FROM saved_search WHERE id = ?", params![id])?;
    Ok(row)
}

fn from_row(r: &rusqlite::Row) -> rusqlite::Result<SavedSearch> {
    Ok(SavedSearch {
        id: r.get(0)?,
        name: r.get(1)?,
        query: serde_json::from_str(&r.get::<_, String>(2)?).unwrap_or(serde_json::Value::Null),
        created_at: r.get(3)?,
        updated_at: r.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use serde_json::json;

    fn catalog() -> (Db, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("strata-saved-{}", uuid::Uuid::new_v4()));
        (Db::open(&dir.join("catalog.sqlite")).unwrap(), dir)
    }

    #[test]
    fn save_list_update_delete_and_restore() {
        let (db, dir) = catalog();
        let conn = db.0.lock().unwrap();
        let q = json!({"v": 1, "text": "bark", "orientations": ["landscape"]});

        let saved = put(&conn, None, "  Olive bark ", &q, None).unwrap();
        assert_eq!(saved.name, "Olive bark");
        assert_eq!(saved.query, q);
        put(&conn, None, "alpha", &json!({"v": 1}), None).unwrap();
        let names: Vec<String> = list(&conn).unwrap().into_iter().map(|s| s.name).collect();
        assert_eq!(names, ["alpha", "Olive bark"]);

        let renamed = put(&conn, Some(&saved.id), "Bark", &q, None).unwrap();
        assert_eq!((renamed.id.as_str(), renamed.created_at.as_str()), (saved.id.as_str(), saved.created_at.as_str()));

        let gone = delete(&conn, &saved.id).unwrap().unwrap();
        assert_eq!(list(&conn).unwrap().len(), 1);
        let back = put(&conn, Some(&gone.id), &gone.name, &gone.query, Some(&gone.created_at)).unwrap();
        assert_eq!(back.created_at, saved.created_at);
        assert_eq!(list(&conn).unwrap().len(), 2);

        assert!(put(&conn, None, " ", &q, None).is_err());
        assert!(put(&conn, None, "x", &json!([1]), None).is_err());
        assert!(delete(&conn, "missing").unwrap().is_none());
        drop(conn);
        std::fs::remove_dir_all(dir).ok();
    }
}
