use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use anyhow::Result;
use sha2::{Digest, Sha256};

const BUF: usize = 64 * 1024;

pub fn hash_file(path: &Path) -> Result<String> {
    let f = File::open(path)?;
    let mut reader = BufReader::with_capacity(BUF, f);
    let mut hasher = Sha256::new();
    let mut buf = [0u8; BUF];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}
