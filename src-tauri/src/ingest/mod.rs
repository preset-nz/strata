pub mod events;
pub mod exif;
pub mod hash;
pub mod job;
pub mod scan;
pub mod stages;
pub mod store;
pub mod thumb;
pub mod trash;

pub fn extension_for(path: &std::path::Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" => Some("jpg"),
        "png" => Some("png"),
        _ => None,
    }
}

pub fn mime_for_ext(ext: &str) -> &'static str {
    match ext {
        "jpg" => "image/jpeg",
        "png" => "image/png",
        _ => "application/octet-stream",
    }
}
