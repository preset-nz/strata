//! Strata's undo history: the curation steps behind Edit > Undo and Redo
//! (hearts, labels, collections, projects). app-kit owns the menu items and
//! their titles; this is the `History` it runs. Session-scoped: it starts
//! empty at launch.
//!
//! A step carries both directions as closures over the catalog, so each
//! curation command records exactly how to take itself back.

use std::sync::Mutex;

use preset_app_kit::History;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::AppState;

/// Emitted after an undo or redo changed the catalog, so the library reloads.
pub const CHANGED_EVENT: &str = "history://changed";
/// Steps kept; the oldest go first.
const LIMIT: usize = 200;

type Apply = Box<dyn Fn(&rusqlite::Connection) -> anyhow::Result<()> + Send + Sync>;

pub struct Step {
    /// What the menu says: "Undo Favourite".
    pub label: String,
    pub undo: Apply,
    pub redo: Apply,
}

#[derive(Default)]
pub struct Curation {
    undo: Mutex<Vec<Step>>,
    redo: Mutex<Vec<Step>>,
}

impl Curation {
    /// Record a step that has just been done. Clears what Redo had.
    pub fn push(&self, step: Step) {
        let mut undo = self.undo.lock().unwrap();
        undo.push(step);
        if undo.len() > LIMIT {
            undo.remove(0);
        }
        self.redo.lock().unwrap().clear();
    }

    fn step<R: Runtime>(&self, app: &AppHandle<R>, back: bool) -> Result<(), String> {
        let (from, to) = if back {
            (&self.undo, &self.redo)
        } else {
            (&self.redo, &self.undo)
        };
        let Some(step) = from.lock().unwrap().pop() else {
            return Ok(());
        };
        let result = {
            let state = app.state::<AppState>();
            let conn = state.db.0.lock().unwrap();
            if back {
                (step.undo)(&conn)
            } else {
                (step.redo)(&conn)
            }
        };
        match result {
            Ok(()) => {
                to.lock().unwrap().push(step);
                let _ = app.emit(CHANGED_EVENT, ());
                Ok(())
            }
            // A step that can't apply (its rows are gone) is dropped rather than retried.
            Err(e) => Err(format!("{} failed: {e}", step.label)),
        }
    }
}

impl History for Curation {
    fn undo_label(&self) -> Option<String> {
        self.undo.lock().unwrap().last().map(|s| s.label.clone())
    }
    fn redo_label(&self) -> Option<String> {
        self.redo.lock().unwrap().last().map(|s| s.label.clone())
    }
    fn undo_labels(&self) -> Vec<String> {
        self.undo
            .lock()
            .unwrap()
            .iter()
            .map(|s| s.label.clone())
            .collect()
    }
    fn redo_labels(&self) -> Vec<String> {
        self.redo
            .lock()
            .unwrap()
            .iter()
            .rev()
            .map(|s| s.label.clone())
            .collect()
    }
    fn undo<R: Runtime>(&self, app: &AppHandle<R>) -> Result<(), String> {
        self.step(app, true)
    }
    fn redo<R: Runtime>(&self, app: &AppHandle<R>) -> Result<(), String> {
        self.step(app, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(label: &str) -> Step {
        Step {
            label: label.into(),
            undo: Box::new(|_| Ok(())),
            redo: Box::new(|_| Ok(())),
        }
    }

    #[test]
    fn labels_follow_the_stacks() {
        let h = Curation::default();
        assert_eq!(h.undo_label(), None);
        h.push(step("Favourite"));
        h.push(step("Label Red"));
        assert_eq!(h.undo_label().as_deref(), Some("Label Red"));
        assert_eq!(h.undo_labels(), ["Favourite", "Label Red"]);
        // Moving a step by hand, as `step` does, without a Tauri app.
        let s = h.undo.lock().unwrap().pop().unwrap();
        h.redo.lock().unwrap().push(s);
        assert_eq!(h.redo_label().as_deref(), Some("Label Red"));
        h.push(step("Unfavourite"));
        assert_eq!(h.redo_label(), None, "a new step clears Redo");
    }

    #[test]
    fn the_oldest_steps_fall_off() {
        let h = Curation::default();
        for i in 0..LIMIT + 5 {
            h.push(step(&i.to_string()));
        }
        assert_eq!(h.undo_labels().len(), LIMIT);
        assert_eq!(h.undo_labels()[0], "5");
    }
}
