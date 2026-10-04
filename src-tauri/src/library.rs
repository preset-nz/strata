//! Where the library lives: `~/Pictures/Strata Library.strata`, a package
//! holding the content-addressed store and the catalog database, the way
//! Photos keeps `Photos Library.photoslibrary`. Finder shows it as one item
//! because the bundle exports its type as conforming to `com.apple.package`.

use std::path::{Path, PathBuf};

use crate::store::StoreRoot;

pub const PACKAGE_NAME: &str = "Strata Library.strata";
const STORE_DIR: &str = "store";
const CATALOG_FILE: &str = "catalog.sqlite";

pub struct Library {
    root: PathBuf,
}

impl Library {
    /// The library in the platform's picture directory.
    pub fn in_pictures(pictures_dir: &Path) -> Self {
        Self::at(pictures_dir.join(PACKAGE_NAME))
    }

    pub fn at(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn store(&self) -> StoreRoot {
        StoreRoot::new(self.root.join(STORE_DIR))
    }

    pub fn catalog_path(&self) -> PathBuf {
        self.root.join(CATALOG_FILE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_and_catalog_travel_together() {
        let library = Library::in_pictures(Path::new("/Users/someone/Pictures"));
        let root = Path::new("/Users/someone/Pictures/Strata Library.strata");
        assert_eq!(library.store().root(), root.join("store"));
        assert_eq!(library.catalog_path(), root.join("catalog.sqlite"));
    }
}
