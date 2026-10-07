use std::path::Path;

use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};

// ---- public types -----------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct Iptc {
    pub title: Option<String>,
    pub caption: Option<String>,
    pub byline: Option<String>,
    pub copyright: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub date_created: Option<DateTime<Utc>>,
    pub keywords: Vec<String>,
}

/// Read IPTC metadata from a JPEG file.  Returns `Ok(None)` if the file has no
/// APP13 segment or no IPTC IIM resource.  Returns `Err` only on I/O errors;
/// malformed-but-readable IPTC data returns a partial `Iptc`.
pub fn read(path: &Path) -> anyhow::Result<Option<Iptc>> {
    let bytes = std::fs::read(path)?;
    Ok(parse_jpeg(&bytes))
}

// ---- JPEG segment walk ------------------------------------------------------

const SOI: [u8; 2] = [0xFF, 0xD8];
const APP13: u8 = 0xED;
const SOS: u8 = 0xDA;
const EOI: u8 = 0xD9;
const PS3_HEADER: &[u8] = b"Photoshop 3.0\0";

fn parse_jpeg(data: &[u8]) -> Option<Iptc> {
    if data.len() < 2 || data[..2] != SOI {
        return None;
    }

    let mut pos = 2usize;

    loop {
        // Each segment starts with 0xFF.  A stray byte that isn't 0xFF means
        // we've drifted into image data — stop.
        if pos >= data.len() || data[pos] != 0xFF {
            return None;
        }
        pos += 1;

        let marker = *data.get(pos)?;
        pos += 1;

        match marker {
            EOI => return None,
            SOS => return None,
            // Standalone markers with no length (RST0-RST7, SOI)
            0xD0..=0xD8 => continue,
            APP13 => {
                // Length includes the 2 length bytes themselves.
                let len = read_u16_be(data, pos)? as usize;
                pos += 2;
                if len < 2 || pos + len - 2 > data.len() {
                    return None;
                }
                let payload = &data[pos..pos + len - 2];
                pos += len - 2;

                if let Some(iptc) = parse_app13(payload) {
                    return Some(iptc);
                }
                // Keep walking if this APP13 wasn't the IPTC carrier.
            }
            _ => {
                let len = read_u16_be(data, pos)? as usize;
                pos += 2;
                if len < 2 {
                    return None;
                }
                pos += len - 2;
            }
        }
    }
}

// ---- APP13 / Photoshop IRB walk ---------------------------------------------

const IPTC_RESOURCE_ID: u16 = 0x0404;

fn parse_app13(payload: &[u8]) -> Option<Iptc> {
    if !payload.starts_with(PS3_HEADER) {
        return None;
    }
    let mut pos = PS3_HEADER.len();

    while pos < payload.len() {
        // Each IRB starts with "8BIM".
        if payload.len() < pos + 4 || &payload[pos..pos + 4] != b"8BIM" {
            break;
        }
        pos += 4;

        let resource_id = read_u16_be(payload, pos)?;
        pos += 2;

        // Pascal string: 1-byte length + `len` bytes, padded so total is even.
        let name_len = *payload.get(pos)? as usize;
        pos += 1;
        pos += name_len;
        // Round up to even total (length-byte + content).
        if !(1 + name_len).is_multiple_of(2) {
            pos += 1;
        }

        let data_size = read_u32_be(payload, pos)? as usize;
        pos += 4;

        if pos + data_size > payload.len() {
            break;
        }
        let resource_data = &payload[pos..pos + data_size];
        // IRB data is padded to even length.
        pos += data_size + (data_size % 2);

        if resource_id == IPTC_RESOURCE_ID {
            return Some(parse_iim(resource_data));
        }
    }

    None
}

// ---- IIM dataset walk -------------------------------------------------------

// Record 2 dataset numbers we care about.
const DS_OBJECT_NAME: u8 = 5;
const DS_KEYWORDS: u8 = 25;
const DS_DATE_CREATED: u8 = 55;
const DS_TIME_CREATED: u8 = 60;
const DS_BYLINE: u8 = 80;
const DS_CITY: u8 = 90;
const DS_STATE: u8 = 95;
const DS_COUNTRY: u8 = 101;
const DS_COPYRIGHT: u8 = 116;
const DS_CAPTION: u8 = 120;

// Record 1 dataset for charset declaration.
const DS_CODED_CHARSET: u8 = 90;

// ESC % G — ISO 2022 escape sequence designating UTF-8.
const UTF8_ESCAPE: [u8; 3] = [0x1B, 0x25, 0x47];

fn parse_iim(data: &[u8]) -> Iptc {
    let mut iptc = Iptc::default();
    let mut pos = 0usize;

    // Raw date/time strings accumulated during the walk; combined after.
    let mut date_raw: Option<String> = None;
    let mut time_raw: Option<String> = None;

    // 1:90 is parsed but both paths (UTF-8 and "other") use from_utf8_lossy,
    // so we note it but don't branch on it.
    let mut _charset_declared = false;

    while pos < data.len() {
        // Tag marker must be 0x1C; if not, the IIM block is corrupt here.
        // Return whatever we've accumulated so far rather than panicking.
        if data[pos] != 0x1C {
            break;
        }
        pos += 1;

        let record = match data.get(pos) {
            Some(&b) => b,
            None => break,
        };
        pos += 1;

        let dataset = match data.get(pos) {
            Some(&b) => b,
            None => break,
        };
        pos += 1;

        // Length field.  High bit set on the first byte signals extended length
        // (the low 15 bits encode the length-of-length).  Extended records are
        // vanishingly rare in practice; bail and return what we have.
        let len_hi = match data.get(pos) {
            Some(&b) => b,
            None => break,
        };
        if len_hi & 0x80 != 0 {
            break;
        }
        let len = match read_u16_be(data, pos) {
            Some(v) => v as usize,
            None => break,
        };
        pos += 2;

        // Sanity cap: no legitimate IPTC field is 64 KiB.
        if len > 65535 || pos + len > data.len() {
            break;
        }

        let field = &data[pos..pos + len];
        pos += len;

        match (record, dataset) {
            (1, DS_CODED_CHARSET) => {
                _charset_declared = field == UTF8_ESCAPE;
            }
            (2, DS_OBJECT_NAME) => {
                iptc.title = Some(decode_str(field));
            }
            (2, DS_CAPTION) => {
                iptc.caption = Some(decode_str(field));
            }
            (2, DS_BYLINE) => {
                iptc.byline = Some(decode_str(field));
            }
            (2, DS_COPYRIGHT) => {
                iptc.copyright = Some(decode_str(field));
            }
            (2, DS_CITY) => {
                iptc.city = Some(decode_str(field));
            }
            (2, DS_STATE) => {
                iptc.state = Some(decode_str(field));
            }
            (2, DS_COUNTRY) => {
                iptc.country = Some(decode_str(field));
            }
            (2, DS_KEYWORDS) => {
                iptc.keywords.push(decode_str(field));
            }
            (2, DS_DATE_CREATED) => {
                if field.len() == 8 {
                    date_raw = Some(decode_str(field));
                }
            }
            (2, DS_TIME_CREATED) if field.len() == 11 => {
                time_raw = Some(decode_str(field));
            }
            _ => {}
        }
    }

    iptc.date_created = combine_datetime(date_raw.as_deref(), time_raw.as_deref());
    iptc
}

// ---- date/time helpers -------------------------------------------------------

fn combine_datetime(date: Option<&str>, time: Option<&str>) -> Option<DateTime<Utc>> {
    let date_str = date?;

    // "CCYYMMDD"
    let year = date_str[0..4].parse::<i32>().ok()?;
    let month = date_str[4..6].parse::<u32>().ok()?;
    let day = date_str[6..8].parse::<u32>().ok()?;
    let naive_date = NaiveDate::from_ymd_opt(year, month, day)?;

    if let Some(time_str) = time {
        // "HHMMSS+HHMM" or "HHMMSS-HHMM" — 11 bytes.
        if time_str.len() == 11 {
            if let Some(dt) = parse_time_field(naive_date, time_str) {
                return Some(dt);
            }
        }
    }

    // Date only — midnight UTC.
    Some(Utc.from_utc_datetime(&naive_date.and_time(NaiveTime::from_hms_opt(0, 0, 0)?)))
}

fn parse_time_field(date: NaiveDate, time_str: &str) -> Option<DateTime<Utc>> {
    let hh = time_str[0..2].parse::<u32>().ok()?;
    let mm = time_str[2..4].parse::<u32>().ok()?;
    let ss = time_str[4..6].parse::<u32>().ok()?;
    let sign = &time_str[6..7];
    let tz_hh = time_str[7..9].parse::<i64>().ok()?;
    let tz_mm = time_str[9..11].parse::<i64>().ok()?;

    let offset_secs: i64 = (tz_hh * 3600 + tz_mm * 60) * if sign == "-" { -1 } else { 1 };

    let naive_dt = date.and_time(NaiveTime::from_hms_opt(hh, mm, ss)?);
    // Shift local time to UTC.
    let utc_dt = naive_dt - chrono::Duration::seconds(offset_secs);
    Some(Utc.from_utc_datetime(&utc_dt))
}

// ---- byte-slice utilities ---------------------------------------------------

fn decode_str(bytes: &[u8]) -> String {
    // Modern IPTC files are de-facto UTF-8.  Both the declared-UTF-8 path and
    // the undeclared/legacy path use lossy decoding — we prefer "possibly
    // garbled" over "dropped field".
    String::from_utf8_lossy(bytes).into_owned()
}

fn read_u16_be(data: &[u8], pos: usize) -> Option<u16> {
    let hi = *data.get(pos)? as u16;
    let lo = *data.get(pos + 1)? as u16;
    Some((hi << 8) | lo)
}

fn read_u32_be(data: &[u8], pos: usize) -> Option<u32> {
    let b0 = *data.get(pos)? as u32;
    let b1 = *data.get(pos + 1)? as u32;
    let b2 = *data.get(pos + 2)? as u32;
    let b3 = *data.get(pos + 3)? as u32;
    Some((b0 << 24) | (b1 << 16) | (b2 << 8) | b3)
}

// ---- tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // Builds a minimal valid JPEG containing one APP13 segment with a
    // Photoshop IRB carrying IIM records.
    //
    // `records` is a slice of (record_number, dataset_number, data).
    #[allow(clippy::vec_init_then_push)]
    fn build_jpeg_with_iptc(records: &[(u8, u8, &[u8])]) -> Vec<u8> {
        // Build the IIM payload.
        let mut iim: Vec<u8> = Vec::new();
        for (rec, ds, data) in records {
            iim.push(0x1C);
            iim.push(*rec);
            iim.push(*ds);
            let len = data.len() as u16;
            iim.push((len >> 8) as u8);
            iim.push(len as u8);
            iim.extend_from_slice(data);
        }

        // Wrap in Photoshop IRB 0x0404.
        let mut irb: Vec<u8> = Vec::new();
        irb.extend_from_slice(b"8BIM");
        irb.push(0x04);
        irb.push(0x04); // resource id 0x0404
        irb.push(0x00); // Pascal name: zero-length
        irb.push(0x00); // padding to make (1 + 0) even → already 1, add 1 pad
        let data_size = iim.len() as u32;
        irb.push((data_size >> 24) as u8);
        irb.push((data_size >> 16) as u8);
        irb.push((data_size >> 8) as u8);
        irb.push(data_size as u8);
        irb.extend_from_slice(&iim);
        if !iim.len().is_multiple_of(2) {
            irb.push(0x00); // data pad
        }

        // Wrap in APP13 segment.
        let mut app13_payload: Vec<u8> = Vec::new();
        app13_payload.extend_from_slice(PS3_HEADER);
        app13_payload.extend_from_slice(&irb);

        let segment_len = (app13_payload.len() + 2) as u16; // +2 for length field itself
        let mut jpeg: Vec<u8> = Vec::new();
        jpeg.push(0xFF);
        jpeg.push(0xD8); // SOI
        jpeg.push(0xFF);
        jpeg.push(0xED); // APP13
        jpeg.push((segment_len >> 8) as u8);
        jpeg.push(segment_len as u8);
        jpeg.extend_from_slice(&app13_payload);
        jpeg.push(0xFF);
        jpeg.push(0xD9); // EOI
        jpeg
    }

    // Writes bytes to a temp file, returns the path.  Caller is responsible
    // for deleting — we use a deterministic name based on PID + nanos so
    // parallel test runs don't collide.
    fn write_temp(data: &[u8]) -> std::path::PathBuf {
        // A clock-based name collides when parallel tests start in the same
        // tick (macOS counts microseconds), and one test reads another's file.
        let name = format!("strata_iptc_{}.jpg", uuid::Uuid::new_v4());
        let path = std::env::temp_dir().join(name);
        let mut f = std::fs::File::create(&path).expect("temp file create");
        f.write_all(data).expect("temp file write");
        path
    }

    #[test]
    fn empty_file_returns_none() {
        // /dev/null is empty — not a JPEG.
        let result = read(Path::new("/dev/null"));
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn parses_basic_iptc_fields() {
        let records: &[(u8, u8, &[u8])] = &[
            (2, DS_OBJECT_NAME, b"Harbour at dusk"),
            (2, DS_CAPTION, b"Looking west from Queens Wharf."),
            (2, DS_COPYRIGHT, b"(c) 2026 Georg Duemlein"),
            (2, DS_DATE_CREATED, b"20260315"),
            (2, DS_TIME_CREATED, b"143025+1200"),
            (2, DS_KEYWORDS, b"harbour"),
            (2, DS_KEYWORDS, b"dusk"),
        ];

        let jpeg = build_jpeg_with_iptc(records);
        let path = write_temp(&jpeg);

        let result = read(&path);
        let _ = std::fs::remove_file(&path);

        let result = result.expect("read should succeed");
        let iptc = result.expect("iptc should be present");

        assert_eq!(iptc.title.as_deref(), Some("Harbour at dusk"));
        assert_eq!(
            iptc.caption.as_deref(),
            Some("Looking west from Queens Wharf.")
        );
        assert_eq!(iptc.copyright.as_deref(), Some("(c) 2026 Georg Duemlein"));
        assert_eq!(iptc.keywords, vec!["harbour", "dusk"]);

        // 2026-03-15 14:30:25 +12:00 → 2026-03-15 02:30:25 UTC
        let dt = iptc.date_created.expect("date_created should be set");
        assert_eq!(
            dt.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            "2026-03-15T02:30:25Z"
        );
    }

    #[test]
    fn date_only_uses_midnight_utc() {
        let records: &[(u8, u8, &[u8])] = &[(2, DS_DATE_CREATED, b"20241231")];

        let jpeg = build_jpeg_with_iptc(records);
        let path = write_temp(&jpeg);
        let result = read(&path);
        let _ = std::fs::remove_file(&path);

        let iptc = result.expect("ok").expect("some");
        let dt = iptc.date_created.expect("date");
        assert_eq!(
            dt.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            "2024-12-31T00:00:00Z"
        );
    }

    #[test]
    fn non_jpeg_bytes_return_none() {
        let data = b"%PDF-1.4 this is not a JPEG";
        let path = write_temp(data);
        let result = read(&path);
        let _ = std::fs::remove_file(&path);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn jpeg_without_app13_returns_none() {
        // Minimal JPEG: SOI + EOI, no APP13.
        let data = b"\xFF\xD8\xFF\xD9";
        let path = write_temp(data);
        let result = read(&path);
        let _ = std::fs::remove_file(&path);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
