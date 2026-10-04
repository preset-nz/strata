use std::collections::{BTreeMap, HashMap};
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

/// One row of the import confirmation's folder tree. `path` is relative to
/// the scanned root, `/`-separated, `""` for the root itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FolderRow {
    pub path: String,
    pub name: String,
    pub depth: usize,
    /// Importable files directly in this folder, not in its subfolders.
    pub own: usize,
    /// Importable files here and below.
    pub total: usize,
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

impl PrescanResult {
    /// The folders that hold importable files, plus every folder between
    /// them and the root so the nesting reads right. Depth-first order,
    /// siblings by name. One pass over `files`; no second walk.
    pub fn folders(&self) -> Vec<FolderRow> {
        let root = Path::new(&self.root);
        let mut own: BTreeMap<Vec<String>, usize> = BTreeMap::new();
        for file in &self.files {
            own.entry(Vec::new()).or_insert(0);
            let parts = relative_parts(root, file.parent().unwrap_or(root));
            for depth in 1..=parts.len() {
                own.entry(parts[..depth].to_vec()).or_insert(0);
            }
            *own.entry(parts).or_insert(0) += 1;
        }
        if self.files.is_empty() {
            return Vec::new();
        }
        // BTreeMap order on Vec<String> is depth-first with siblings sorted.
        own.iter()
            .map(|(parts, &n)| FolderRow {
                path: parts.join("/"),
                name: parts.last().cloned().unwrap_or_else(|| {
                    root.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| self.root.clone())
                }),
                depth: parts.len(),
                own: n,
                total: own
                    .range(parts.clone()..)
                    .take_while(|(p, _)| p.starts_with(parts))
                    .map(|(_, n)| n)
                    .sum(),
            })
            .collect()
    }

    /// The files whose folder was kept. Folders are `FolderRow::path`s.
    pub fn select(&self, kept: &[String]) -> Vec<PathBuf> {
        let root = Path::new(&self.root);
        self.files
            .iter()
            .filter(|f| {
                let folder = relative_parts(root, f.parent().unwrap_or(root)).join("/");
                kept.iter().any(|k| *k == folder)
            })
            .cloned()
            .collect()
    }
}

fn relative_parts(root: &Path, dir: &Path) -> Vec<String> {
    dir.strip_prefix(root)
        .map(|rel| {
            rel.components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(files: &[&str]) -> PrescanResult {
        PrescanResult {
            root: "/in/DrawThings".into(),
            total: files.len(),
            by_extension: HashMap::new(),
            files: files.iter().map(|f| Path::new("/in/DrawThings").join(f)).collect(),
        }
    }

    fn row(path: &str, name: &str, depth: usize, own: usize, total: usize) -> FolderRow {
        FolderRow { path: path.into(), name: name.into(), depth, own, total }
    }

    #[test]
    fn folders_nest_with_own_and_total_counts() {
        let s = scan(&["a.png", "bark/b.png", "bark/c.png", "moss/deep/d.png", "moss-2/e.png"]);
        assert_eq!(
            s.folders(),
            vec![
                row("", "DrawThings", 0, 1, 5),
                row("bark", "bark", 1, 2, 2),
                row("moss", "moss", 1, 0, 1),
                row("moss/deep", "deep", 2, 1, 1),
                row("moss-2", "moss-2", 1, 1, 1),
            ]
        );
    }

    #[test]
    fn root_without_files_of_its_own_still_heads_the_tree() {
        let s = scan(&["only/a.png"]);
        assert_eq!(s.folders(), vec![row("", "DrawThings", 0, 0, 1), row("only", "only", 1, 1, 1)]);
        assert!(scan(&[]).folders().is_empty());
    }

    #[test]
    fn select_keeps_only_files_directly_in_kept_folders() {
        let s = scan(&["a.png", "bark/b.png", "bark/inner/c.png"]);
        let kept = s.select(&["bark/inner".into(), "".into()]);
        assert_eq!(
            kept,
            vec![Path::new("/in/DrawThings/a.png"), Path::new("/in/DrawThings/bark/inner/c.png")]
        );
    }
}
