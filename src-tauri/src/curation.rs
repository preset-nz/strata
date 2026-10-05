//! Favourites and colour labels: per image, across the whole catalog
//! (epic 08 decisions, 2026-10-05). A row in `image_mark` exists only while
//! an image has a heart or a label. Every change is one undo step.

use std::collections::HashMap;

use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::Serialize;

use crate::history::Step;

pub const LABELS: [&str; 7] = ["red", "orange", "yellow", "green", "blue", "purple", "grey"];

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Mark {
    pub favourite: bool,
    pub label: Option<String>,
}

/// One image's mark after a change, for the webview to update its cards.
#[derive(Debug, Clone, Serialize)]
pub struct Marked {
    pub id: String,
    #[serde(flatten)]
    pub mark: Mark,
}

pub fn check_label(label: Option<&str>) -> Result<()> {
    match label {
        Some(l) if !LABELS.contains(&l) => bail!("unknown colour label {l:?}"),
        _ => Ok(()),
    }
}

pub fn marks(conn: &Connection, ids: &[String]) -> Result<HashMap<String, Mark>> {
    let mut stmt = conn.prepare("SELECT favourite, label FROM image_mark WHERE image_id = ?")?;
    let mut out = HashMap::new();
    for id in ids {
        let mark = stmt
            .query_row(params![id], |r| Ok(Mark { favourite: r.get(0)?, label: r.get(1)? }))
            .unwrap_or_default();
        out.insert(id.clone(), mark);
    }
    Ok(out)
}

fn write(conn: &Connection, id: &str, mark: &Mark) -> Result<()> {
    if *mark == Mark::default() {
        conn.execute("DELETE FROM image_mark WHERE image_id = ?", params![id])?;
    } else {
        conn.execute(
            "INSERT INTO image_mark (image_id, favourite, label, updated_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(image_id) DO UPDATE SET favourite = excluded.favourite,
                 label = excluded.label, updated_at = excluded.updated_at",
            params![id, mark.favourite, mark.label, Utc::now()],
        )?;
    }
    Ok(())
}

fn write_all(conn: &Connection, marks: &[(String, Mark)]) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    for (id, mark) in marks {
        write(&tx, id, mark)?;
    }
    tx.commit()?;
    Ok(())
}

/// Applies `change` to each image's mark and returns what to show now and the
/// undo step that takes it back. Images whose mark doesn't change are left
/// out of both; `None` when nothing changed at all.
pub fn change(
    conn: &Connection,
    ids: &[String],
    label: &str,
    change: impl Fn(&Mark) -> Mark,
) -> Result<Option<(Vec<Marked>, Step)>> {
    let before = marks(conn, ids)?;
    let mut was = Vec::new();
    let mut now = Vec::new();
    for id in ids {
        let old = before.get(id).cloned().unwrap_or_default();
        let new = change(&old);
        if new != old {
            was.push((id.clone(), old));
            now.push((id.clone(), new));
        }
    }
    if now.is_empty() {
        return Ok(None);
    }
    write_all(conn, &now)?;
    let shown = now.iter().map(|(id, mark)| Marked { id: id.clone(), mark: mark.clone() }).collect();
    let redo = now;
    let step = Step {
        label: step_label(label, redo.len()),
        undo: Box::new(move |conn| write_all(conn, &was)),
        redo: Box::new(move |conn| write_all(conn, &redo)),
    };
    Ok(Some((shown, step)))
}

/// "Favourite", or "Favourite 3 Images" when it covered several.
fn step_label(action: &str, n: usize) -> String {
    if n == 1 { action.to_string() } else { format!("{action} {n} Images") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    fn catalog() -> (Db, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("strata-marks-{}", uuid::Uuid::new_v4()));
        (Db::open(&dir.join("catalog.sqlite")).unwrap(), dir)
    }

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn favourite_label_undo_and_redo() {
        let (db, dir) = catalog();
        let conn = db.0.lock().unwrap();
        let both = ids(&["a", "b"]);

        let (shown, step) = change(&conn, &both, "Favourite", |m| Mark { favourite: true, ..m.clone() })
            .unwrap()
            .unwrap();
        assert_eq!(shown.len(), 2);
        assert_eq!(step.label, "Favourite 2 Images");

        let (_, label_step) = change(&conn, &ids(&["a"]), "Label Red", |m| Mark { label: Some("red".into()), ..m.clone() })
            .unwrap()
            .unwrap();
        assert_eq!(marks(&conn, &ids(&["a"])).unwrap()["a"], Mark { favourite: true, label: Some("red".into()) });

        (label_step.undo)(&conn).unwrap();
        assert_eq!(marks(&conn, &ids(&["a"])).unwrap()["a"], Mark { favourite: true, label: None });
        (step.undo)(&conn).unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM image_mark", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0, "a cleared mark leaves no row");
        (step.redo)(&conn).unwrap();
        assert!(marks(&conn, &both).unwrap().values().all(|m| m.favourite));

        assert!(change(&conn, &both, "Favourite", |m| Mark { favourite: true, ..m.clone() }).unwrap().is_none(), "no-op is no step");
        drop(conn);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn labels_are_checked() {
        assert!(check_label(Some("red")).is_ok());
        assert!(check_label(None).is_ok());
        assert!(check_label(Some("teal")).is_err());
    }
}
