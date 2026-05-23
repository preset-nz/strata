use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

use crate::ingest::hash::hash_file;
use crate::store::StoreRoot;

pub struct StoredCopy {
    pub target_path: PathBuf,
}

pub fn copy_and_verify(
    store: &StoreRoot,
    source: &Path,
    hash_hex: &str,
    ext: &str,
) -> Result<StoredCopy> {
    let target_dir = store.target_dir(hash_hex);
    fs::create_dir_all(&target_dir)?;
    let target = store.target_path(hash_hex, ext);

    if target.exists() {
        return Ok(StoredCopy { target_path: target });
    }

    let partial = target.with_extension(format!("{ext}.partial"));
    // Cross-volume safe: fs::copy falls back to a byte copy when rename can't.
    if let Err(e) = fs::copy(source, &partial) {
        let _ = fs::remove_file(&partial);
        return Err(anyhow!("copy to .partial failed: {e}"));
    }

    if let Err(e) = atomic_rename(&partial, &target) {
        let _ = fs::remove_file(&partial);
        return Err(anyhow!("rename to final path failed: {e}"));
    }

    let observed = hash_file(&target)?;
    if observed != hash_hex {
        let _ = fs::remove_file(&target);
        return Err(anyhow!(
            "read-back verify mismatch: expected {hash_hex}, got {observed}"
        ));
    }

    Ok(StoredCopy { target_path: target })
}

fn atomic_rename(from: &Path, to: &Path) -> io::Result<()> {
    // On every supported platform fs::rename is atomic within a filesystem
    // and produces an error across filesystems. Both endpoints live under the
    // store root so we're always within one filesystem here.
    fs::rename(from, to)
}
