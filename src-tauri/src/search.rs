//! Keyword search: an FTS5 index with one row per image over the words that
//! describe it, ranked by BM25. Lexical, not semantic: it finds the image
//! whose prompt, keyword, caption or filename says the word.
//! Lean recorded in guidance `projects/strata/open-questions.md`, "BM25".
//!
//! The row is rebuilt from the source tables whenever they change
//! (`reindex`), so the index never holds anything the catalog doesn't.

use anyhow::Result;
use rusqlite::{params, Connection};

/// Column weights for `bm25()`, in column order after `image_id`: the prompt
/// counts most, then keywords, caption, and the filename last, since a
/// Draw Things filename is a truncated copy of the prompt.
pub const BM25_WEIGHTS: &str = "0.0, 10.0, 5.0, 3.0, 1.0";

/// Rebuilds one image's row from the catalog.
pub fn reindex(conn: &Connection, image_id: &str) -> Result<()> {
    conn.execute("DELETE FROM image_search WHERE image_id = ?", params![image_id])?;
    let source: Option<(String, Option<String>, Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT i.original_filename,
                    (SELECT pr.text || ' ' || COALESCE(pr.negative, '')
                       FROM generation_output go
                       JOIN generation g ON g.generation_id = go.generation_id
                       JOIN prompt pr ON pr.prompt_id = g.prompt_id
                      WHERE go.image_id = i.id LIMIT 1),
                    (SELECT group_concat(keyword, ' ') FROM image_keyword WHERE image_id = i.id),
                    m.iptc_caption
               FROM images i LEFT JOIN image_metadata m ON m.image_id = i.id
              WHERE i.id = ?",
            params![image_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .ok();
    let Some((filename, prompt, keywords, caption)) = source else {
        return Ok(());
    };
    conn.execute(
        "INSERT INTO image_search (image_id, prompt, keywords, caption, filename) VALUES (?, ?, ?, ?, ?)",
        params![
            image_id,
            prompt.as_deref().map(strip_weights).unwrap_or_default(),
            keywords.unwrap_or_default(),
            caption.unwrap_or_default(),
            filename_words(&filename),
        ],
    )?;
    Ok(())
}

/// Indexes every image that has no row yet: the first launch after the
/// index exists, and anything a crash left out.
pub fn backfill(conn: &Connection) -> Result<usize> {
    let ids: Vec<String> = conn
        .prepare("SELECT id FROM images WHERE id NOT IN (SELECT image_id FROM image_search)")?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    for id in &ids {
        reindex(conn, id)?;
    }
    Ok(ids.len())
}

/// Turns what was typed into an FTS5 query: every word must appear, and the
/// last may be a prefix, so results follow the typing. `None` when nothing
/// searchable was typed.
pub fn match_query(typed: &str) -> Option<String> {
    let words: Vec<String> = typed
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .collect();
    let (last, rest) = words.split_last()?;
    let mut terms: Vec<String> = rest.iter().map(|w| format!("\"{w}\"")).collect();
    terms.push(format!("\"{last}\"*"));
    Some(terms.join(" "))
}

/// Draw Things weights a phrase as `(mist:1.3)`; the number is noise to a
/// reader looking for "mist". Drops `:<number>` before a closing bracket.
pub fn strip_weights(prompt: &str) -> String {
    let mut out = String::with_capacity(prompt.len());
    let mut rest = prompt;
    while let Some(colon) = rest.find(':') {
        let (head, tail) = rest.split_at(colon);
        let number_len = tail[1..]
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(tail.len() - 1);
        let after = &tail[1 + number_len..];
        out.push_str(head);
        if number_len > 0 && after.starts_with(')') {
            rest = after;
        } else {
            out.push(':');
            rest = &tail[1..];
        }
    }
    out.push_str(rest);
    out
}

/// A filename's words, extension included so a pasted filename matches.
/// Long digit runs (a Draw Things seed like `2736034274`) go: nobody types them.
fn filename_words(filename: &str) -> String {
    filename.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && !(w.len() > 6 && w.bytes().all(|b| b.is_ascii_digit())))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_are_stripped_and_other_colons_kept() {
        assert_eq!(strip_weights("(mist:1.3) over (bark:0.8), ratio 3:2"), "(mist) over (bark), ratio 3:2");
        assert_eq!(strip_weights("no weights here"), "no weights here");
        assert_eq!(strip_weights("trailing:"), "trailing:");
    }

    #[test]
    fn queries_need_every_word_and_prefix_the_last() {
        assert_eq!(match_query("Fungal  bar").as_deref(), Some("\"fungal\" \"bar\"*"));
        assert_eq!(match_query("\"(mist)\"").as_deref(), Some("\"mist\"*"));
        assert_eq!(match_query("  -- "), None);
    }

    #[test]
    fn filenames_lose_the_seed() {
        assert_eq!(
            filename_words("lichen_colony__wet_bark_2736034274(1).png"),
            "lichen colony wet bark 1 png"
        );
    }
}
