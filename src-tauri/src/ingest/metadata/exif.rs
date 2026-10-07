use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use chrono::{DateTime, NaiveDate, Utc};
use exif::{Exif, In, Rational, Tag, Value};

#[derive(Debug, Default, Clone)]
pub struct ExifData {
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens_make: Option<String>,
    pub lens_model: Option<String>,
    pub focal_length_mm: Option<f32>,
    pub focal_length_35mm: Option<f32>,

    pub iso: Option<i32>,
    pub f_number: Option<f32>,
    pub exposure_time_sec: Option<f32>,
    pub exposure_bias: Option<f32>,
    pub exposure_program: Option<String>,
    pub metering_mode: Option<String>,
    pub flash_fired: Option<bool>,

    pub pixel_width: Option<i32>,
    pub pixel_height: Option<i32>,
    pub orientation: Option<i32>,
    pub color_space: Option<String>,

    pub gps_latitude: Option<f32>,
    pub gps_longitude: Option<f32>,
    pub gps_altitude_m: Option<f32>,

    pub software: Option<String>,

    pub date_time_original: Option<DateTime<Utc>>,
}

#[allow(clippy::field_reassign_with_default)]
pub fn read(path: &Path) -> Option<ExifData> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;

    let mut data = ExifData::default();

    data.camera_make = ascii(&exif, Tag::Make);
    data.camera_model = ascii(&exif, Tag::Model);
    data.lens_make = ascii(&exif, Tag::LensMake);
    data.lens_model = ascii(&exif, Tag::LensModel);
    data.software = ascii(&exif, Tag::Software);

    data.focal_length_mm = rational_f32(&exif, Tag::FocalLength);
    data.focal_length_35mm = short_or_long_i32(&exif, Tag::FocalLengthIn35mmFilm).map(|v| v as f32);

    data.iso = short_or_long_i32(&exif, Tag::PhotographicSensitivity)
        .or_else(|| short_or_long_i32(&exif, Tag::ISOSpeed));

    data.f_number = rational_f32(&exif, Tag::FNumber);
    data.exposure_time_sec = rational_f32(&exif, Tag::ExposureTime);
    data.exposure_bias = signed_rational_f32(&exif, Tag::ExposureBiasValue);

    data.exposure_program =
        short_or_long_i32(&exif, Tag::ExposureProgram).map(exposure_program_name);
    data.metering_mode = short_or_long_i32(&exif, Tag::MeteringMode).map(metering_mode_name);
    data.flash_fired = short_or_long_i32(&exif, Tag::Flash).map(|v| v & 0x01 != 0);

    data.pixel_width = short_or_long_i32(&exif, Tag::PixelXDimension);
    data.pixel_height = short_or_long_i32(&exif, Tag::PixelYDimension);
    data.orientation = short_or_long_i32(&exif, Tag::Orientation);
    data.color_space = short_or_long_i32(&exif, Tag::ColorSpace).map(colour_space_name);

    data.gps_latitude = gps_coord(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef, "S");
    data.gps_longitude = gps_coord(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef, "W");
    data.gps_altitude_m = gps_altitude(&exif);

    data.date_time_original = date_time_original(&exif);

    Some(data)
}

fn ascii(exif: &Exif, tag: Tag) -> Option<String> {
    let field = exif.get_field(tag, In::PRIMARY)?;
    if let Value::Ascii(vec) = &field.value {
        let bytes = vec.first()?;
        let s = String::from_utf8_lossy(bytes).trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    } else {
        None
    }
}

fn rational_f32(exif: &Exif, tag: Tag) -> Option<f32> {
    let field = exif.get_field(tag, In::PRIMARY)?;
    let r = match &field.value {
        Value::Rational(vec) => *vec.first()?,
        _ => return None,
    };
    rational_to_f32(r)
}

fn signed_rational_f32(exif: &Exif, tag: Tag) -> Option<f32> {
    let field = exif.get_field(tag, In::PRIMARY)?;
    match &field.value {
        Value::SRational(vec) => {
            let r = vec.first()?;
            if r.denom == 0 {
                None
            } else {
                Some(r.num as f32 / r.denom as f32)
            }
        }
        _ => None,
    }
}

fn rational_to_f32(r: Rational) -> Option<f32> {
    if r.denom == 0 {
        None
    } else {
        Some(r.num as f32 / r.denom as f32)
    }
}

fn short_or_long_i32(exif: &Exif, tag: Tag) -> Option<i32> {
    let field = exif.get_field(tag, In::PRIMARY)?;
    match &field.value {
        Value::Short(vec) => vec.first().map(|v| *v as i32),
        Value::Long(vec) => vec.first().map(|v| *v as i32),
        Value::SShort(vec) => vec.first().map(|v| *v as i32),
        Value::SLong(vec) => vec.first().copied(),
        _ => None,
    }
}

fn gps_coord(exif: &Exif, value_tag: Tag, ref_tag: Tag, negative_ref: &str) -> Option<f32> {
    let field = exif.get_field(value_tag, In::PRIMARY)?;
    let dms = match &field.value {
        Value::Rational(vec) if vec.len() >= 3 => vec,
        _ => return None,
    };
    let deg = rational_to_f32(dms[0])?;
    let min = rational_to_f32(dms[1])?;
    let sec = rational_to_f32(dms[2])?;
    let decimal = deg + min / 60.0 + sec / 3600.0;

    let negate = ascii(exif, ref_tag)
        .map(|r| r.trim().eq_ignore_ascii_case(negative_ref))
        .unwrap_or(false);
    Some(if negate { -decimal } else { decimal })
}

fn gps_altitude(exif: &Exif) -> Option<f32> {
    let field = exif.get_field(Tag::GPSAltitude, In::PRIMARY)?;
    let value = match &field.value {
        Value::Rational(vec) => rational_to_f32(*vec.first()?)?,
        _ => return None,
    };
    let below_sea = exif
        .get_field(Tag::GPSAltitudeRef, In::PRIMARY)
        .and_then(|f| match &f.value {
            Value::Byte(vec) => vec.first().copied(),
            _ => None,
        })
        .map(|b| b == 1)
        .unwrap_or(false);
    Some(if below_sea { -value } else { value })
}

fn date_time_original(exif: &Exif) -> Option<DateTime<Utc>> {
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

fn exposure_program_name(v: i32) -> String {
    match v {
        0 => "Not defined",
        1 => "Manual",
        2 => "Normal program",
        3 => "Aperture priority",
        4 => "Shutter priority",
        5 => "Creative program",
        6 => "Action program",
        7 => "Portrait mode",
        8 => "Landscape mode",
        _ => "Unknown",
    }
    .to_string()
}

fn metering_mode_name(v: i32) -> String {
    match v {
        0 => "Unknown",
        1 => "Average",
        2 => "Center-weighted",
        3 => "Spot",
        4 => "Multi-spot",
        5 => "Pattern",
        6 => "Partial",
        255 => "Other",
        _ => "Unknown",
    }
    .to_string()
}

fn colour_space_name(v: i32) -> String {
    match v {
        1 => "sRGB",
        2 => "Adobe RGB",
        65535 => "Uncalibrated",
        _ => "Unknown",
    }
    .to_string()
}

pub fn fs_mtime(path: &Path) -> Option<DateTime<Utc>> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?;
    Some(mtime.into())
}
