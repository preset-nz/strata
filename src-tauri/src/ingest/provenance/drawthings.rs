//! Reads the generation record Draw Things embeds in its PNG exports.
//!
//! Checked 2026-10-04 and 2026-10-05 against `~/Pictures/DrawThings/`: the
//! PNG's eXIf chunk carries only the dimensions. The record is in the XMP
//! packet (an uncompressed `iTXt` chunk, keyword `XML:com.adobe.xmp`):
//! `xmp:CreatorTool` is `Draw Things`, and `exif:UserComment` holds JSON
//! with `c` (prompt), `uc` (negative), `model`, `seed`, the sampler settings
//! and a `v2` block with the full configuration.

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use serde_json::Value;

pub const PRODUCER: &str = "drawthings";
const CREATOR_TOOL: &str = "Draw Things";
const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const XMP_KEYWORD: &[u8] = b"XML:com.adobe.xmp";

#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    pub prompt: String,
    pub negative: Option<String>,
    pub model: Option<String>,
    pub seed: Option<i64>,
    /// The whole embedded JSON, kept as Draw Things wrote it.
    pub settings: Value,
}

/// `None` for anything that isn't a Draw Things PNG with a readable record.
pub fn read(path: &Path) -> Option<Record> {
    let xmp = png_xmp(path).ok().flatten()?;
    if !xmp_value(&xmp, "xmp:CreatorTool")?.contains(CREATOR_TOOL) {
        return None;
    }
    parse_record(&xmp_value(&xmp, "exif:UserComment")?)
}

pub fn parse_record(json: &str) -> Option<Record> {
    let settings: Value = serde_json::from_str(json).ok()?;
    let text = |key: &str| {
        settings
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    Some(Record {
        prompt: text("c")?,
        negative: text("uc"),
        model: text("model"),
        seed: settings.get("seed").and_then(Value::as_i64),
        settings,
    })
}

/// The XMP packet from a PNG, walking chunk headers and seeking past image
/// data, so a large PNG costs a few reads.
fn png_xmp(path: &Path) -> std::io::Result<Option<String>> {
    let mut file = BufReader::new(File::open(path)?);
    let mut signature = [0u8; 8];
    file.read_exact(&mut signature)?;
    if &signature != PNG_SIGNATURE {
        return Ok(None);
    }
    loop {
        let mut header = [0u8; 8];
        if file.read_exact(&mut header).is_err() {
            return Ok(None);
        }
        let len = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
        let kind = &header[4..8];
        if kind == b"IEND" {
            return Ok(None);
        }
        if kind != b"iTXt" {
            file.seek(SeekFrom::Current(len as i64 + 4))?;
            continue;
        }
        let mut data = vec![0u8; len];
        file.read_exact(&mut data)?;
        file.seek(SeekFrom::Current(4))?;
        if let Some(text) = itxt_text(&data, XMP_KEYWORD) {
            return Ok(Some(text));
        }
    }
}

/// iTXt: keyword \0 compression-flag method language \0 translated \0 text.
/// Compressed text isn't read; Draw Things writes it plain.
fn itxt_text(data: &[u8], keyword: &[u8]) -> Option<String> {
    let rest = data.strip_prefix(keyword)?.strip_prefix(b"\0")?;
    let (&compressed, rest) = rest.split_first()?;
    if compressed != 0 {
        return None;
    }
    let rest = &rest[1..];
    let rest = &rest[rest.iter().position(|&b| b == 0)? + 1..];
    let text = &rest[rest.iter().position(|&b| b == 0)? + 1..];
    String::from_utf8(text.to_vec()).ok()
}

/// The value of a simple XMP property, in element form (with or without an
/// `rdf:Alt` wrapper) or attribute form, unescaped.
fn xmp_value(xmp: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    if let Some(start) = xmp.find(&open) {
        let body = &xmp[start + open.len()..];
        let body = &body[..body.find(&format!("</{tag}>"))?];
        let inner = match body.find("<rdf:li") {
            Some(li) => {
                let after = &body[li..];
                let content = &after[after.find('>')? + 1..];
                &content[..content.find("</rdf:li>")?]
            }
            None => body,
        };
        return Some(unescape(inner.trim()));
    }
    let attr = format!("{tag}=\"");
    let start = xmp.find(&attr)? + attr.len();
    let end = xmp[start..].find('"')?;
    Some(unescape(&xmp[start..start + end]))
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let tail = &rest[amp..];
        let Some(semi) = tail.find(';') else {
            out.push_str(tail);
            return out;
        };
        let entity = &tail[1..semi];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &tail[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const XMP: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF><rdf:Description rdf:about="">
         <xmp:CreatorTool>Draw Things</xmp:CreatorTool>
         <exif:UserComment>
            <rdf:Alt>
               <rdf:li xml:lang="x-default">{"c":"Lichen colony, (mist:1.3) &amp; bark\nclose","uc":"blurry","model":"flux_1_schnell_q5p.ckpt","seed":1172910896,"steps":1}</rdf:li>
            </rdf:Alt>
         </exif:UserComment>
      </rdf:Description></rdf:RDF></x:xmpmeta>"#;

    fn chunk(kind: &[u8], data: &[u8]) -> Vec<u8> {
        let mut out = (data.len() as u32).to_be_bytes().to_vec();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        out.extend_from_slice(&[0, 0, 0, 0]); // CRC, unchecked
        out
    }

    fn png_with_xmp(xmp: &str) -> std::path::PathBuf {
        let mut bytes = PNG_SIGNATURE.to_vec();
        bytes.extend(chunk(b"IHDR", &[0; 13]));
        bytes.extend(chunk(b"IDAT", &[0; 64]));
        let mut itxt = XMP_KEYWORD.to_vec();
        itxt.extend_from_slice(b"\0\0\0\0\0");
        itxt.extend_from_slice(xmp.as_bytes());
        bytes.extend(chunk(b"iTXt", &itxt));
        bytes.extend(chunk(b"IEND", &[]));
        let path = std::env::temp_dir().join(format!("strata-dt-{}.png", uuid::Uuid::new_v4()));
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn reads_the_record_from_xmp() {
        let path = png_with_xmp(XMP);
        let record = read(&path).unwrap();
        std::fs::remove_file(path).ok();
        assert_eq!(record.prompt, "Lichen colony, (mist:1.3) & bark\nclose");
        assert_eq!(record.negative.as_deref(), Some("blurry"));
        assert_eq!(record.model.as_deref(), Some("flux_1_schnell_q5p.ckpt"));
        assert_eq!(record.seed, Some(1172910896));
        assert_eq!(record.settings["steps"], 1);
    }

    #[test]
    fn other_creators_and_files_are_ignored() {
        let path = png_with_xmp(&XMP.replace("Draw Things", "Photoshop"));
        assert!(read(&path).is_none());
        std::fs::remove_file(path).ok();
        assert!(read(Path::new("/nonexistent.png")).is_none());
        assert!(parse_record(r#"{"c":"  ","seed":1}"#).is_none());
    }

    #[test]
    fn attribute_form_and_entities() {
        assert_eq!(
            xmp_value(r#"<d xmp:CreatorTool="Draw Things &#8212; 1.2"/>"#, "xmp:CreatorTool").as_deref(),
            Some("Draw Things \u{2014} 1.2")
        );
        assert_eq!(unescape("a &unknown; b &amp"), "a &unknown; b &amp");
    }
}
