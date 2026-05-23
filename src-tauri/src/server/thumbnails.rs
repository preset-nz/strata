use std::path::PathBuf;

use tauri::http::{Request, Response, StatusCode};
use tauri::{Manager, UriSchemeContext};

use crate::AppState;

pub const SCHEME: &str = "thumb";

// URI form: thumb://<hash>/<size>.jpg  (size in {256,512,1024})
pub fn handler<R: tauri::Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let state = match ctx.app_handle().try_state::<AppState>() {
        Some(s) => s,
        None => return not_found(),
    };

    let uri = request.uri().to_string();
    let (hash, size) = match parse_uri(&uri) {
        Some(parts) => parts,
        None => return not_found(),
    };

    let path: PathBuf = state.store.thumb_path(&hash, size);
    match std::fs::read(&path) {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "image/jpeg")
            .header("Cache-Control", "max-age=86400")
            .body(bytes)
            .unwrap_or_else(|_| not_found()),
        Err(_) => not_found(),
    }
}

fn parse_uri(uri: &str) -> Option<(String, u32)> {
    // tauri may surface this as `thumb://<host>/<path>` or `thumb://localhost/<host>/<path>`.
    // Normalise by stripping the scheme and any leading host.
    let without_scheme = uri.strip_prefix("thumb://")?;
    let trimmed = without_scheme
        .split_once('?')
        .map(|(p, _)| p)
        .unwrap_or(without_scheme);
    let mut parts = trimmed.split('/').filter(|s| !s.is_empty());
    let first = parts.next()?;
    let second = parts.next();
    let (hash, file_part) = match second {
        // host-style: thumb://hash/size.jpg
        Some(file) if file.ends_with(".jpg") => (first.to_string(), file),
        // localhost-prefixed: thumb://localhost/hash/size.jpg
        Some(file) => {
            let third = parts.next()?;
            if !third.ends_with(".jpg") {
                return None;
            }
            (file.to_string(), third)
        }
        None => return None,
    };
    let size_str = file_part.strip_suffix(".jpg")?;
    let size: u32 = size_str.parse().ok()?;
    if !matches!(size, 256 | 512 | 1024) {
        return None;
    }
    Some((hash, size))
}

fn not_found() -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Vec::new())
        .expect("builder")
}
