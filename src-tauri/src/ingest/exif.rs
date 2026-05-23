use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use chrono::{DateTime, NaiveDate, Utc};
use exif::{In, Reader, Tag, Value};

pub fn read_created_at(path: &Path) -> Option<DateTime<Utc>> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let exif = Reader::new().read_from_container(&mut reader).ok()?;

    let field = exif.get_field(Tag::DateTimeOriginal, In::PRIMARY)?;
    let bytes = match &field.value {
        Value::Ascii(vec) => vec.first()?,
        _ => return None,
    };

    let dt = exif::DateTime::from_ascii(bytes).ok()?;
    let date = NaiveDate::from_ymd_opt(dt.year as i32, dt.month as u32, dt.day as u32)?
        .and_hms_opt(dt.hour as u32, dt.minute as u32, dt.second as u32)?;
    Some(DateTime::from_naive_utc_and_offset(date, Utc))
}

pub fn fs_mtime(path: &Path) -> Option<DateTime<Utc>> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?;
    Some(mtime.into())
}
