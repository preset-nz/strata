//! The one seam that turns a string into a path at the moment a file is read
//! or written, modelled on Maya's `MPxFileResolver`. A string without a
//! scheme is a path and passes through; a registered scheme hands the string
//! to its resolver. Documents keep what the person gave, path or reference.
//!
//! `strata://` token references resolve from the naming schema and the
//! folders on disk alone. Only `@published` (and asset ids, if they arrive)
//! need Strata's catalog, which this resolver does not read.

use std::path::{Path, PathBuf};

use crate::naming::{self, Found, NamingError, Schema, Tokens, VersionSelector};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Access {
    /// Resolves to a file that exists.
    Read,
    /// Save a Version: allocates the next version and returns the path to
    /// write. A reference carries no extension, so the writer names it.
    Write { ext: String },
}

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("no resolver registered for `{0}://`")]
    UnknownScheme(String),
    #[error(transparent)]
    Naming(#[from] NamingError),
    #[error("no project with key `{0}`")]
    UnknownProject(String),
    #[error("`{0}` needs Strata's catalog")]
    NeedsCatalog(String),
    #[error("nothing on disk for `{0}`")]
    NotFound(String),
    #[error("`{0}` matches more than one file: {1:?}")]
    Ambiguous(String, Vec<PathBuf>),
    #[error("`{0}` names a version; saving writes the next one, so leave it off")]
    VersionPinned(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub trait Resolver: Send + Sync {
    fn scheme(&self) -> &str;
    fn resolve(&self, reference: &str, mode: &Access) -> Result<PathBuf, ResolveError>;
}

/// The registered resolvers. Apps call `resolve_path` and nothing else.
#[derive(Default)]
pub struct Resolvers(Vec<Box<dyn Resolver>>);

impl Resolvers {
    pub fn register(&mut self, resolver: Box<dyn Resolver>) {
        self.0.push(resolver);
    }

    pub fn resolve_path(&self, s: &str, mode: &Access) -> Result<PathBuf, ResolveError> {
        let Some((scheme, _)) = s.split_once("://") else {
            return Ok(PathBuf::from(s));
        };
        self.0
            .iter()
            .find(|r| r.scheme() == scheme)
            .ok_or_else(|| ResolveError::UnknownScheme(scheme.to_string()))?
            .resolve(s, mode)
    }
}

/// Finds a project's folder from the key in its `project.preset`.
pub trait ProjectLocator: Send + Sync {
    fn locate(&self, key: &str) -> Option<PathBuf>;
}

/// Looks for `<root>/*/project.preset` with a matching `key`, so a project
/// folder can be renamed freely. The default root is
/// `~/preset-nz/Projects`.
pub struct ProjectsFolder {
    pub roots: Vec<PathBuf>,
}

pub const PROJECT_MARKER: &str = crate::project::MARKER;

impl ProjectsFolder {
    pub fn in_home(home: &Path) -> Self {
        Self {
            roots: vec![home.join("preset-nz").join("Projects")],
        }
    }
}

impl ProjectLocator for ProjectsFolder {
    fn locate(&self, key: &str) -> Option<PathBuf> {
        self.roots.iter().find_map(|root| {
            std::fs::read_dir(root).ok()?.flatten().find_map(|entry| {
                let dir = entry.path();
                let marker = crate::project::read_marker(&dir)?;
                marker.key.eq_ignore_ascii_case(key).then_some(dir)
            })
        })
    }
}

pub struct StrataResolver {
    schema: Schema,
    projects: Box<dyn ProjectLocator>,
}

impl StrataResolver {
    pub fn new(schema: Schema, projects: Box<dyn ProjectLocator>) -> Self {
        Self { schema, projects }
    }

    /// Walks the levels from the project folder, matching folder names
    /// case-insensitively, since a reference is lowercase.
    fn folder(
        &self,
        root: &Path,
        tokens: &Tokens,
        reference: &str,
    ) -> Result<PathBuf, ResolveError> {
        let mut dir = root.to_path_buf();
        for level in self.schema.levels() {
            let wanted = &tokens[&level.name];
            dir = child_dir_ignoring_case(&dir, wanted)?
                .ok_or_else(|| ResolveError::NotFound(reference.to_string()))?;
        }
        Ok(dir)
    }
}

impl Resolver for StrataResolver {
    fn scheme(&self) -> &str {
        naming::SCHEME
    }

    fn resolve(&self, reference: &str, mode: &Access) -> Result<PathBuf, ResolveError> {
        let parsed = self.schema.parse_reference(reference)?;
        if parsed.version == VersionSelector::Published {
            return Err(ResolveError::NeedsCatalog(reference.to_string()));
        }
        let root = self
            .projects
            .locate(&parsed.project)
            .ok_or_else(|| ResolveError::UnknownProject(parsed.project.clone()))?;
        let dir = self.folder(&root, &parsed.tokens, reference)?;
        let found = self.schema.versions(&dir, &parsed.tokens)?;

        match mode {
            Access::Read => {
                let version = match parsed.version {
                    VersionSelector::Number(n) => n,
                    _ => found
                        .last()
                        .map(|f| f.version)
                        .ok_or_else(|| ResolveError::NotFound(reference.to_string()))?,
                };
                let mut hits: Vec<&Found> = found.iter().filter(|f| f.version == version).collect();
                match hits.len() {
                    0 => Err(ResolveError::NotFound(reference.to_string())),
                    1 => Ok(hits.remove(0).path.clone()),
                    _ => Err(ResolveError::Ambiguous(
                        reference.to_string(),
                        hits.into_iter().map(|f| f.path.clone()).collect(),
                    )),
                }
            }
            Access::Write { ext } => {
                if parsed.version != VersionSelector::Latest {
                    return Err(ResolveError::VersionPinned(reference.to_string()));
                }
                // Keep the spelling already on disk; a first version takes
                // the reference's.
                let mut tokens = match found.last() {
                    Some(latest) => latest.tokens.clone(),
                    None => parsed.tokens.clone(),
                };
                tokens.remove("frame");
                let next = found.last().map_or(1, |f| f.version + 1);
                tokens.insert(naming::VERSION.to_string(), next.to_string());
                tokens.insert(naming::EXT.to_string(), ext.clone());
                Ok(dir.join(self.schema.format_filename(&tokens)?))
            }
        }
    }
}

fn child_dir_ignoring_case(dir: &Path, name: &str) -> std::io::Result<Option<PathBuf>> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(name)
        {
            return Ok(Some(entry.path()));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Project {
        dir: PathBuf,
    }

    impl Project {
        fn new() -> Self {
            let base =
                std::env::temp_dir().join(format!("strata-resolver-{}", uuid::Uuid::new_v4()));
            let dir = base.join("Night Drive");
            std::fs::create_dir_all(dir.join("base/Shard/Patches")).unwrap();
            std::fs::write(dir.join(PROJECT_MARKER), "key = \"night-drive\"\n").unwrap();
            Self { dir }
        }

        fn touch(&self, relative: &str) {
            std::fs::write(self.dir.join(relative), b"").unwrap();
        }

        fn resolvers(&self) -> Resolvers {
            let mut resolvers = Resolvers::default();
            resolvers.register(Box::new(StrataResolver::new(
                Schema::default_family(),
                Box::new(ProjectsFolder {
                    roots: vec![self.dir.parent().unwrap().to_path_buf()],
                }),
            )));
            resolvers
        }
    }

    impl Drop for Project {
        fn drop(&mut self) {
            std::fs::remove_dir_all(self.dir.parent().unwrap()).ok();
        }
    }

    fn write(ext: &str) -> Access {
        Access::Write {
            ext: ext.to_string(),
        }
    }

    #[test]
    fn plain_paths_pass_through() {
        let resolvers = Resolvers::default();
        let path = "/Users/someone/Pictures/cover.png";
        assert_eq!(
            resolvers.resolve_path(path, &Access::Read).unwrap(),
            Path::new(path)
        );
    }

    #[test]
    fn unknown_schemes_are_refused() {
        let resolvers = Resolvers::default();
        assert!(matches!(
            resolvers.resolve_path("strata://night-drive/base/shard/patches/drums", &Access::Read),
            Err(ResolveError::UnknownScheme(s)) if s == "strata"
        ));
    }

    #[test]
    fn reads_a_named_version_and_the_latest() {
        let project = Project::new();
        project.touch("base/Shard/Patches/drums_kick.v001.shard");
        project.touch("base/Shard/Patches/drums_kick.v003.shard");
        project.touch("base/Shard/Patches/drums.v009.shard");
        let resolvers = project.resolvers();
        let patches = project.dir.join("base/Shard/Patches");

        let pinned = resolvers
            .resolve_path(
                "strata://night-drive/base/shard/patches/drums_kick@1",
                &Access::Read,
            )
            .unwrap();
        assert_eq!(pinned, patches.join("drums_kick.v001.shard"));

        let latest = resolvers
            .resolve_path(
                "strata://night-drive/base/shard/patches/drums_kick",
                &Access::Read,
            )
            .unwrap();
        assert_eq!(latest, patches.join("drums_kick.v003.shard"));
    }

    #[test]
    fn write_allocates_the_next_version() {
        let project = Project::new();
        project.touch("base/Shard/Patches/Drums_Kick.v002.shard");
        let resolvers = project.resolvers();
        let patches = project.dir.join("base/Shard/Patches");

        let next = resolvers
            .resolve_path(
                "strata://night-drive/base/shard/patches/drums_kick",
                &write("shard"),
            )
            .unwrap();
        assert_eq!(next, patches.join("Drums_Kick.v003.shard"));

        let first = resolvers
            .resolve_path(
                "strata://night-drive/base/shard/patches/snare",
                &write("shard"),
            )
            .unwrap();
        assert_eq!(first, patches.join("snare.v001.shard"));
    }

    #[test]
    fn refusals_say_why() {
        let project = Project::new();
        project.touch("base/Shard/Patches/drums.v001.shard");
        project.touch("base/Shard/Patches/drums.v001.wav");
        let resolvers = project.resolvers();
        let resolve = |s: &str, mode: &Access| resolvers.resolve_path(s, mode);

        assert!(matches!(
            resolve(
                "strata://night-drive/base/shard/patches/drums@published",
                &Access::Read
            ),
            Err(ResolveError::NeedsCatalog(_))
        ));
        assert!(matches!(
            resolve("strata://elsewhere/base/shard/patches/drums", &Access::Read),
            Err(ResolveError::UnknownProject(_))
        ));
        assert!(matches!(
            resolve(
                "strata://night-drive/base/shard/patches/hats",
                &Access::Read
            ),
            Err(ResolveError::NotFound(_))
        ));
        assert!(matches!(
            resolve(
                "strata://night-drive/base/oblique/exports/cover",
                &Access::Read
            ),
            Err(ResolveError::NotFound(_))
        ));
        assert!(matches!(
            resolve("strata://night-drive/base/shard/patches/drums", &Access::Read),
            Err(ResolveError::Ambiguous(_, paths)) if paths.len() == 2
        ));
        assert!(matches!(
            resolve(
                "strata://night-drive/base/shard/patches/drums@1",
                &write("shard")
            ),
            Err(ResolveError::VersionPinned(_))
        ));
    }
}
