use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use walkdir::WalkDir;

use crate::ingest::extension_for;

#[derive(Debug, Clone, Serialize)]
pub struct PrescanResult {
    pub root: String,
    pub total: usize,
    pub by_extension: HashMap<String, usize>,
    pub files: Vec<PathBuf>,
}

/// `allow` is the lowercase extension allow-list from preferences. It narrows
/// within what the decoders support; `extension_for` still has the last word.
pub fn prescan(root: &Path, allow: &[String]) -> PrescanResult {
    let mut by_extension: HashMap<String, usize> = HashMap::new();
    let mut files: Vec<PathBuf> = Vec::new();

    for entry in WalkDir::new(root).follow_links(false).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();

        if is_hidden_or_resource_fork(path) {
            continue;
        }

        let Some(canonical_ext) = extension_for(path) else {
            continue;
        };
        let own_ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();
        if !allow.iter().any(|a| a.eq_ignore_ascii_case(&own_ext)) {
            continue;
        }

        *by_extension.entry(canonical_ext.to_string()).or_insert(0) += 1;
        files.push(path.to_path_buf());
    }

    PrescanResult {
        root: root.to_string_lossy().to_string(),
        total: files.len(),
        by_extension,
        files,
    }
}

fn is_hidden_or_resource_fork(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return true;
    };
    if name.starts_with("._") {
        return true;
    }
    name.starts_with('.')
}
