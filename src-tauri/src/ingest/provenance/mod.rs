//! Provenance: how a generated image was made. One `prompt` (deduplicated
//! by a hash of its text and negative), one `generation` per output made,
//! and `generation_output` linking a generation to the catalogued image.
//! Shape: guidance `design/provenance-and-evals.md`, "The shared record".
//! Draw Things is the first producer; each producer gets its own reader.

pub mod backfill;
pub mod drawthings;

use std::path::Path;

use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

use crate::db::Db;

pub const STAGE_VERSION: &str = "provenance-read@1";
const TITLE_WORDS: usize = 5;

/// Reads whatever record the file carries and stores it, then marks the
/// image checked so the backfill doesn't read it again.
pub fn run(db: &Db, image_id: &str, content_hash: &str, stored_path: &Path) -> Result<()> {
    store(db, image_id, content_hash, drawthings::read(stored_path).as_ref())
}

/// Stores a record (if any) and marks the image checked, in one transaction.
pub fn store(
    db: &Db,
    image_id: &str,
    content_hash: &str,
    record: Option<&drawthings::Record>,
) -> Result<()> {
    let mut conn = db.0.lock().unwrap();
    let tx = conn.transaction()?;
    if let Some(record) = record {
        write_drawthings(&tx, image_id, content_hash, record)?;
    }
    tx.execute(
        "INSERT OR REPLACE INTO provenance_checked (image_id, stage_version, checked_at) VALUES (?, ?, ?)",
        params![image_id, STAGE_VERSION, Utc::now()],
    )?;
    if record.is_some() {
        crate::search::reindex(&tx, image_id)?;
    }
    tx.commit()?;
    Ok(())
}

fn write_drawthings(
    conn: &Connection,
    image_id: &str,
    content_hash: &str,
    record: &drawthings::Record,
) -> Result<()> {
    let prompt_id = prompt_id(&record.prompt, record.negative.as_deref());
    conn.execute(
        "INSERT OR IGNORE INTO prompt (prompt_id, text, negative, first_seen) VALUES (?, ?, ?, ?)",
        params![prompt_id, record.prompt, record.negative, Utc::now()],
    )?;
    // One file, one generation: keyed by the bytes, so a re-read replaces
    // rather than duplicates.
    let generation_id = format!("{}:{content_hash}", drawthings::PRODUCER);
    conn.execute(
        "INSERT OR REPLACE INTO generation (generation_id, prompt_id, producer, model, seed, settings, at) VALUES (?, ?, ?, ?, ?, ?, NULL)",
        params![
            generation_id,
            prompt_id,
            drawthings::PRODUCER,
            record.model,
            record.seed,
            record.settings.to_string(),
        ],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO generation_output (generation_id, image_id, kind) VALUES (?, ?, 'image')",
        params![generation_id, image_id],
    )?;
    Ok(())
}

/// Two runs of the same text and negative share a prompt row; any edit to
/// either is a new prompt.
pub fn prompt_id(text: &str, negative: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hasher.update([0u8]);
    hasher.update(negative.unwrap_or("").as_bytes());
    hex::encode(hasher.finalize())
}

/// The display title for a generated image until it has a better name:
/// the prompt's head (up to the first `:` or `,`, at most five words),
/// slugged. Lowercase ASCII with accents folded, `-` between words. `None` if nothing is left.
pub fn title_from_prompt(prompt: &str) -> Option<String> {
    let head = prompt.split([':', ',', '\n']).next().unwrap_or("");
    // Fold accents to their base letter (é → e, Ō → O) before dropping
    // whatever is still outside ASCII.
    let head: String = head.nfd().filter(|c| !is_combining_mark(*c)).collect();
    let words: Vec<String> = head
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .take(TITLE_WORDS)
        .map(|w| w.to_ascii_lowercase())
        .collect();
    (!words.is_empty()).then(|| words.join("-"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_take_the_prompt_head() {
        let t = |s: &str| title_from_prompt(s);
        assert_eq!(
            t("Parchment consumed by disease, wound tissue pressed flat").as_deref(),
            Some("parchment-consumed-by-disease")
        );
        assert_eq!(
            t("lichen colony spreading over wet bark at dusk in fog").as_deref(),
            Some("lichen-colony-spreading-over-wet")
        );
        assert_eq!(t("(mist:1.3) over the moor").as_deref(), Some("mist"));
        assert_eq!(t("Café — Ōtautahi").as_deref(), Some("cafe-otautahi"));
        assert_eq!(t(", leading comma"), None);
    }

    #[test]
    fn prompt_ids_separate_text_from_negative() {
        assert_eq!(prompt_id("a", Some("b")), prompt_id("a", Some("b")));
        assert_ne!(prompt_id("ab", None), prompt_id("a", Some("b")));
        assert_eq!(prompt_id("a", None), prompt_id("a", Some("")));
    }
}
