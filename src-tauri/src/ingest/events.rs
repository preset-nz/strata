use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub const EVENT_NAME: &str = "ingest://file";
pub const BATCH_DONE_EVENT: &str = "ingest://batch-done";

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileState {
    Queued,
    Hashing,
    Copying,
    Verifying,
    Indexing,
    Thumbnailing,
    #[allow(dead_code)]
    Trashing,
    Done,
    SkippedDuplicate,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileEvent {
    pub batch_id: String,
    pub source_path: String,
    pub original_filename: String,
    pub state: FileState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchDoneEvent {
    pub batch_id: String,
    pub imported: usize,
    pub skipped: usize,
    pub failed: usize,
}

pub fn emit_file(app: &AppHandle, ev: &FileEvent) {
    let _ = app.emit(EVENT_NAME, ev);
}

pub fn emit_batch_done(app: &AppHandle, ev: &BatchDoneEvent) {
    let _ = app.emit(BATCH_DONE_EVENT, ev);
}
