//! Versioned names inside a project, driven by a schema rather than code.
//!
//! A path below the project folder is `<level>/.../<filename>`, the filename
//! printed from a template such as
//! `{element}[_{component}].v{version}[.{frame}].{ext}`. A token reference
//! mirrors it: `strata://night-drive/base/shard/patches/drums_kick@3`.
//! Everything here is deterministic from the schema and the folder; nothing
//! reads the catalog. Model: guidance `design/file-naming-and-versions.md`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const SCHEME: &str = "strata";
/// The part names the resolver sets itself.
pub const VERSION: &str = "version";
pub const EXT: &str = "ext";

/// Level and part values by name. Integer parts are held without padding.
pub type Tokens = BTreeMap<String, String>;

#[derive(Debug, thiserror::Error)]
pub enum NamingError {
    #[error("naming schema: {0}")]
    Schema(String),
    #[error("missing token `{0}`")]
    MissingToken(String),
    #[error("`{0}` is not a strata:// reference this schema understands")]
    InvalidReference(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionSelector {
    Latest,
    Number(u32),
    Published,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub project: String,
    /// Level values and the stem parts (no version, frame or extension),
    /// lowercase as written.
    pub tokens: Tokens,
    pub version: VersionSelector,
}

/// One versioned file found on disk.
#[derive(Debug, Clone)]
pub struct Found {
    pub path: PathBuf,
    pub tokens: Tokens,
    pub version: u32,
}

#[derive(Debug, Deserialize)]
struct SchemaFile {
    level: Vec<Level>,
    file: FileSpec,
    part: Vec<Part>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Level {
    pub name: String,
    pub default: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FileSpec {
    template: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Part {
    name: String,
    #[serde(default)]
    optional: bool,
    #[serde(default)]
    kind: PartKind,
    pad: Option<usize>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum PartKind {
    #[default]
    Text,
    Int,
    Ext,
}

#[derive(Debug, Clone)]
enum Segment {
    Literal(String),
    Part(String),
    Optional(Vec<Segment>),
}

#[derive(Debug, Clone)]
pub struct Schema {
    levels: Vec<Level>,
    parts: BTreeMap<String, Part>,
    file: Vec<Segment>,
    /// The filename up to the version: what a reference names.
    stem: Vec<Segment>,
}

impl Schema {
    /// The family's default schema, embedded.
    pub fn default_family() -> Self {
        Self::from_toml(include_str!("schema.toml")).expect("embedded naming schema is valid")
    }

    pub fn from_toml(source: &str) -> Result<Self, NamingError> {
        let file: SchemaFile =
            toml::from_str(source).map_err(|e| NamingError::Schema(e.to_string()))?;
        let parts: BTreeMap<String, Part> =
            file.part.into_iter().map(|p| (p.name.clone(), p)).collect();
        let segments = parse_template(&file.file.template)?;
        let optional = optional_parts(&segments);
        for name in segment_parts(&segments) {
            let part = parts
                .get(&name)
                .ok_or_else(|| NamingError::Schema(format!("template names unknown part `{name}`")))?;
            if part.optional != optional.contains(&name) {
                return Err(NamingError::Schema(format!(
                    "part `{name}` is optional in one of the template and the part list only"
                )));
            }
        }
        let at = segments
            .iter()
            .position(|s| matches!(s, Segment::Part(n) if n == VERSION))
            .ok_or_else(|| NamingError::Schema("template has no top-level {version}".into()))?;
        // Drop the literal that introduces the version (`.v`).
        let stem_end = match at.checked_sub(1).map(|i| &segments[i]) {
            Some(Segment::Literal(_)) => at - 1,
            _ => at,
        };
        Ok(Self {
            levels: file.level,
            stem: segments[..stem_end].to_vec(),
            file: segments,
            parts,
        })
    }

    pub fn levels(&self) -> &[Level] {
        &self.levels
    }

    /// Tokens in, path below the project folder out.
    pub fn format(&self, tokens: &Tokens) -> Result<PathBuf, NamingError> {
        let mut path = PathBuf::new();
        for level in &self.levels {
            let value = tokens
                .get(&level.name)
                .or(level.default.as_ref())
                .ok_or_else(|| NamingError::MissingToken(level.name.clone()))?;
            path.push(value);
        }
        path.push(self.format_filename(tokens)?);
        Ok(path)
    }

    pub fn format_filename(&self, tokens: &Tokens) -> Result<String, NamingError> {
        let mut out = String::new();
        self.print(&self.file, tokens, &mut out)?
            .then_some(())
            .ok_or_else(|| NamingError::MissingToken(first_missing(&self.file, tokens)))?;
        Ok(out)
    }

    /// Path below the project folder in, tokens out. `None` if the path
    /// doesn't follow the schema.
    pub fn parse(&self, relative: &Path) -> Option<Tokens> {
        let components: Vec<String> = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        let (filename, dirs) = components.split_last()?;
        if dirs.len() != self.levels.len() {
            return None;
        }
        let mut tokens = self.parse_filename(filename)?;
        for (level, value) in self.levels.iter().zip(dirs) {
            tokens.insert(level.name.clone(), value.clone());
        }
        Some(tokens)
    }

    pub fn parse_filename(&self, filename: &str) -> Option<Tokens> {
        let mut tokens = Tokens::new();
        self.matches(&self.file, filename, &mut tokens).then_some(tokens)
    }

    /// Tokens to a reference, lowercase, version omitted when absent.
    pub fn reference(&self, project: &str, tokens: &Tokens) -> Result<String, NamingError> {
        let mut out = format!("{SCHEME}://{}", project.to_lowercase());
        for level in &self.levels {
            let value = tokens
                .get(&level.name)
                .or(level.default.as_ref())
                .ok_or_else(|| NamingError::MissingToken(level.name.clone()))?;
            out.push('/');
            out.push_str(&value.to_lowercase());
        }
        let mut stem = String::new();
        if !self.print(&self.stem, tokens, &mut stem)? {
            return Err(NamingError::MissingToken(first_missing(&self.stem, tokens)));
        }
        out.push('/');
        out.push_str(&stem.to_lowercase());
        if let Some(v) = tokens.get(VERSION) {
            out.push('@');
            out.push_str(v);
        }
        Ok(out)
    }

    pub fn parse_reference(&self, reference: &str) -> Result<Reference, NamingError> {
        let invalid = || NamingError::InvalidReference(reference.to_string());
        let rest = reference
            .strip_prefix(SCHEME)
            .and_then(|r| r.strip_prefix("://"))
            .ok_or_else(invalid)?;
        let (path, version) = match rest.rsplit_once('@') {
            Some((path, "published")) => (path, VersionSelector::Published),
            Some((path, v)) => (path, VersionSelector::Number(v.parse().map_err(|_| invalid())?)),
            None => (rest, VersionSelector::Latest),
        };
        let segments: Vec<&str> = path.split('/').collect();
        if segments.len() != self.levels.len() + 2 || segments.iter().any(|s| s.is_empty()) {
            return Err(invalid());
        }
        let mut tokens = Tokens::new();
        if !self.matches(&self.stem, segments[segments.len() - 1], &mut tokens) {
            return Err(invalid());
        }
        for (level, value) in self.levels.iter().zip(&segments[1..]) {
            tokens.insert(level.name.clone(), value.to_string());
        }
        Ok(Reference {
            project: segments[0].to_string(),
            tokens,
            version,
        })
    }

    /// The versioned files in `dir` whose stem matches `stem` (compared
    /// case-insensitively; an optional part absent on one side must be
    /// absent on the other).
    pub fn versions(&self, dir: &Path, stem: &Tokens) -> std::io::Result<Vec<Found>> {
        let wanted = self.stem_tokens(stem);
        let mut found = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(tokens) = self.parse_filename(&name) else { continue };
            if !same_ignoring_case(&self.stem_tokens(&tokens), &wanted) {
                continue;
            }
            let Some(version) = tokens.get(VERSION).and_then(|v| v.parse().ok()) else { continue };
            found.push(Found { path: entry.path(), tokens, version });
        }
        found.sort_by(|a, b| a.version.cmp(&b.version).then_with(|| a.path.cmp(&b.path)));
        Ok(found)
    }

    /// The highest version on disk, if any.
    pub fn latest(&self, dir: &Path, stem: &Tokens) -> std::io::Result<Option<u32>> {
        Ok(self.versions(dir, stem)?.last().map(|f| f.version))
    }

    /// The version a save would write: one past the highest, or 1.
    pub fn next_version(&self, dir: &Path, stem: &Tokens) -> std::io::Result<u32> {
        Ok(self.latest(dir, stem)?.map_or(1, |v| v + 1))
    }

    fn stem_tokens(&self, tokens: &Tokens) -> Tokens {
        segment_parts(&self.stem)
            .into_iter()
            .filter_map(|name| tokens.get(&name).map(|v| (name, v.clone())))
            .collect()
    }

    /// Prints `segments`; `Ok(false)` if a required part has no value.
    fn print(&self, segments: &[Segment], tokens: &Tokens, out: &mut String) -> Result<bool, NamingError> {
        for segment in segments {
            match segment {
                Segment::Literal(text) => out.push_str(text),
                Segment::Part(name) => match tokens.get(name) {
                    Some(value) => out.push_str(&self.pad(name, value)),
                    None => return Ok(false),
                },
                Segment::Optional(inner) => {
                    let mut group = String::new();
                    if self.print(inner, tokens, &mut group)? {
                        out.push_str(&group);
                    }
                }
            }
        }
        Ok(true)
    }

    fn pad(&self, name: &str, value: &str) -> String {
        match self.parts.get(name).and_then(|p| p.pad) {
            Some(width) => format!("{value:0>width$}"),
            None => value.to_string(),
        }
    }

    /// Backtracking match of `input` against `segments`, filling `tokens`.
    fn matches(&self, segments: &[Segment], input: &str, tokens: &mut Tokens) -> bool {
        let Some((first, rest)) = segments.split_first() else {
            return input.is_empty();
        };
        match first {
            Segment::Literal(text) => input
                .strip_prefix(text.as_str())
                .is_some_and(|tail| self.matches(rest, tail, tokens)),
            Segment::Optional(inner) => {
                let mut joined = inner.clone();
                joined.extend_from_slice(rest);
                let mut with = tokens.clone();
                if self.matches(&joined, input, &mut with) {
                    *tokens = with;
                    return true;
                }
                self.matches(rest, input, tokens)
            }
            Segment::Part(name) => {
                let kind = self.parts[name].kind;
                // `/`, `_` and `.` separate, so a part never contains them.
                let max = input.find(['/', '_', '.']).unwrap_or(input.len());
                for end in (1..=max).rev() {
                    let value = &input[..end];
                    if kind == PartKind::Int && !value.bytes().all(|b| b.is_ascii_digit()) {
                        continue;
                    }
                    let mut attempt = tokens.clone();
                    let stored = match kind {
                        PartKind::Int => value.trim_start_matches('0').to_string(),
                        _ => value.to_string(),
                    };
                    let stored = if stored.is_empty() { "0".to_string() } else { stored };
                    attempt.insert(name.clone(), stored);
                    if self.matches(rest, &input[end..], &mut attempt) {
                        *tokens = attempt;
                        return true;
                    }
                }
                false
            }
        }
    }
}

fn same_ignoring_case(a: &Tokens, b: &Tokens) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|((ka, va), (kb, vb))| ka == kb && va.eq_ignore_ascii_case(vb))
}

fn segment_parts(segments: &[Segment]) -> Vec<String> {
    let mut names = Vec::new();
    for segment in segments {
        match segment {
            Segment::Literal(_) => {}
            Segment::Part(name) => names.push(name.clone()),
            Segment::Optional(inner) => names.extend(segment_parts(inner)),
        }
    }
    names
}

/// Parts inside a `[ ... ]` group.
fn optional_parts(segments: &[Segment]) -> Vec<String> {
    let mut names = Vec::new();
    for segment in segments {
        if let Segment::Optional(inner) = segment {
            names.extend(segment_parts(inner));
        }
    }
    names
}

fn first_missing(segments: &[Segment], tokens: &Tokens) -> String {
    segments
        .iter()
        .find_map(|s| match s {
            Segment::Part(name) if !tokens.contains_key(name) => Some(name.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

fn parse_template(template: &str) -> Result<Vec<Segment>, NamingError> {
    let mut chars = template.chars().peekable();
    let segments = parse_segments(&mut chars, false)?;
    Ok(segments)
}

fn parse_segments(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    in_group: bool,
) -> Result<Vec<Segment>, NamingError> {
    let mut segments = Vec::new();
    let mut literal = String::new();
    let flush = |literal: &mut String, segments: &mut Vec<Segment>| {
        if !literal.is_empty() {
            segments.push(Segment::Literal(std::mem::take(literal)));
        }
    };
    while let Some(c) = chars.next() {
        match c {
            '{' => {
                flush(&mut literal, &mut segments);
                let name: String = chars.by_ref().take_while(|&c| c != '}').collect();
                if name.is_empty() {
                    return Err(NamingError::Schema("empty {} in template".into()));
                }
                segments.push(Segment::Part(name));
            }
            '[' => {
                flush(&mut literal, &mut segments);
                segments.push(Segment::Optional(parse_segments(chars, true)?));
            }
            ']' if in_group => {
                flush(&mut literal, &mut segments);
                return Ok(segments);
            }
            ']' => return Err(NamingError::Schema("unbalanced ] in template".into())),
            _ => literal.push(c),
        }
    }
    if in_group {
        return Err(NamingError::Schema("unclosed [ in template".into()));
    }
    flush(&mut literal, &mut segments);
    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(pairs: &[(&str, &str)]) -> Tokens {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn formats_and_parses_round_trip() {
        let schema = Schema::default_family();
        let t = tokens(&[
            ("context", "base"),
            ("app", "Shard"),
            ("kind", "Patches"),
            ("element", "drums"),
            ("component", "kick"),
            ("version", "3"),
            ("ext", "shard"),
        ]);
        let path = schema.format(&t).unwrap();
        assert_eq!(path, Path::new("base/Shard/Patches/drums_kick.v003.shard"));
        assert_eq!(schema.parse(&path).unwrap(), t);
    }

    #[test]
    fn optional_parts_drop_out() {
        let schema = Schema::default_family();
        let parsed = schema.parse_filename("intro.v002.wav").unwrap();
        assert_eq!(parsed, tokens(&[("element", "intro"), ("version", "2"), ("ext", "wav")]));

        let framed = schema.parse_filename("cover_bg.v002.0042.png").unwrap();
        assert_eq!(framed["frame"], "42");
        assert_eq!(schema.format_filename(&framed).unwrap(), "cover_bg.v002.0042.png");
    }

    #[test]
    fn rejects_names_off_schema() {
        let schema = Schema::default_family();
        assert!(schema.parse_filename("notes.txt").is_none());
        assert!(schema.parse_filename("drums.vabc.shard").is_none());
    }

    #[test]
    fn references_round_trip() {
        let schema = Schema::default_family();
        let r = schema
            .parse_reference("strata://night-drive/base/shard/patches/drums_kick@3")
            .unwrap();
        assert_eq!(r.project, "night-drive");
        assert_eq!(r.version, VersionSelector::Number(3));
        assert_eq!(r.tokens["component"], "kick");

        let mut t = r.tokens.clone();
        t.insert("version".into(), "3".into());
        assert_eq!(
            schema.reference("Night-Drive", &t).unwrap(),
            "strata://night-drive/base/shard/patches/drums_kick@3"
        );

        let latest = schema.parse_reference("strata://night-drive/base/shard/patches/intro").unwrap();
        assert_eq!(latest.version, VersionSelector::Latest);
        assert!(!latest.tokens.contains_key("component"));

        let published = schema
            .parse_reference("strata://night-drive/base/shard/patches/intro@published")
            .unwrap();
        assert_eq!(published.version, VersionSelector::Published);
    }

    #[test]
    fn rejects_malformed_references() {
        let schema = Schema::default_family();
        for bad in [
            "night-drive/base/shard/patches/intro",
            "strata://night-drive/shard/patches/intro",
            "strata://night-drive/base/shard/patches/intro@three",
            "strata://night-drive/base/shard/patches/",
        ] {
            assert!(schema.parse_reference(bad).is_err(), "{bad}");
        }
    }
}
