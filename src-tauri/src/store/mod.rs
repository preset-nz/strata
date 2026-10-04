use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct StoreRoot(pub PathBuf);

impl StoreRoot {
    pub fn new(root: PathBuf) -> Self {
        Self(root)
    }

    pub fn ensure(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.0)
    }

    pub fn target_dir(&self, hash_hex: &str) -> PathBuf {
        self.0.join(&hash_hex[0..2]).join(&hash_hex[2..4])
    }

    pub fn target_path(&self, hash_hex: &str, ext: &str) -> PathBuf {
        self.target_dir(hash_hex).join(format!("{hash_hex}.{ext}"))
    }

    pub fn thumb_path(&self, hash_hex: &str, size: u32) -> PathBuf {
        self.target_dir(hash_hex)
            .join(format!("{hash_hex}_{size}.jpg"))
    }

    pub fn root(&self) -> &Path {
        &self.0
    }
}
