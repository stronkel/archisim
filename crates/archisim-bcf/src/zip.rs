//! The little of ZIP that BCF needs: reading archives written by other
//! programs (stored or deflated entries, with or without data
//! descriptors) and writing deflated ones. No encryption, no ZIP64 (a BCF
//! file is far below 4 GB).

use std::io::{Read, Write};

use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::{Compression, Crc};

#[derive(Debug, Clone, PartialEq)]
pub struct ZipError(pub String);

impl std::fmt::Display for ZipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ZIP: {}", self.0)
    }
}

impl std::error::Error for ZipError {}

fn u16_at(b: &[u8], i: usize) -> Result<u16, ZipError> {
    b.get(i..i + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or_else(|| ZipError("fichero cortado".into()))
}

fn u32_at(b: &[u8], i: usize) -> Result<u32, ZipError> {
    b.get(i..i + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| ZipError("fichero cortado".into()))
}

/// Every file of an archive, by name (folders are skipped), in archive
/// order. Names use `/` as written in the archive.
pub fn read(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, ZipError> {
    // The end of central directory record, searched from the end (it may
    // be followed by a comment).
    let min = bytes.len().saturating_sub(22 + 65535);
    let eocd = (min..=bytes.len().saturating_sub(22))
        .rev()
        .find(|&i| bytes[i..].starts_with(&[0x50, 0x4b, 0x05, 0x06]))
        .ok_or_else(|| ZipError("no es un fichero ZIP".into()))?;
    let count = u16_at(bytes, eocd + 10)? as usize;
    let mut at = u32_at(bytes, eocd + 16)? as usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if u32_at(bytes, at)? != 0x0201_4b50 {
            return Err(ZipError("directorio central dañado".into()));
        }
        let flags = u16_at(bytes, at + 8)?;
        let method = u16_at(bytes, at + 10)?;
        let csize = u32_at(bytes, at + 20)? as usize;
        let usize_ = u32_at(bytes, at + 24)? as usize;
        let name_len = u16_at(bytes, at + 28)? as usize;
        let extra_len = u16_at(bytes, at + 30)? as usize;
        let comment_len = u16_at(bytes, at + 32)? as usize;
        let local = u32_at(bytes, at + 42)? as usize;
        let name_bytes = bytes
            .get(at + 46..at + 46 + name_len)
            .ok_or_else(|| ZipError("nombre cortado".into()))?;
        // Bit 11: UTF-8 names; otherwise CP437, which for BCF names
        // (GUIDs and ASCII) is the same.
        let name = String::from_utf8_lossy(name_bytes).replace('\\', "/");
        at += 46 + name_len + extra_len + comment_len;
        if flags & 1 != 0 {
            return Err(ZipError(format!("«{name}» está cifrado")));
        }
        if name.ends_with('/') {
            continue;
        }
        if u32_at(bytes, local)? != 0x0403_4b50 {
            return Err(ZipError(format!("cabecera local de «{name}» dañada")));
        }
        let lname = u16_at(bytes, local + 26)? as usize;
        let lextra = u16_at(bytes, local + 28)? as usize;
        let start = local + 30 + lname + lextra;
        let data = bytes
            .get(start..start + csize)
            .ok_or_else(|| ZipError(format!("«{name}» cortado")))?;
        let content = match method {
            0 => data.to_vec(),
            8 => {
                let mut v = Vec::with_capacity(usize_);
                DeflateDecoder::new(data)
                    .read_to_end(&mut v)
                    .map_err(|e| ZipError(format!("«{name}»: {e}")))?;
                v
            }
            m => {
                return Err(ZipError(format!(
                    "«{name}» usa una compresión no admitida ({m})"
                )))
            }
        };
        out.push((name, content));
    }
    Ok(out)
}

/// An archive with the given files, deflated (and stored when that is
/// smaller, as for PNG snapshots). The date is fixed so that the same
/// content gives the same bytes.
pub fn write(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    // 2026-01-01 00:00, MS-DOS format.
    let (time, date) = (0u16, ((2026 - 1980) << 9 | 1 << 5 | 1) as u16);
    for (name, content) in files {
        let mut crc = Crc::new();
        crc.update(content);
        let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
        enc.write_all(content).expect("in memory");
        let deflated = enc.finish().expect("in memory");
        let (method, data): (u16, &[u8]) = if deflated.len() < content.len() {
            (8, &deflated)
        } else {
            (0, content)
        };
        let offset = out.len() as u32;
        let name_b = name.as_bytes();
        let flags: u16 = 1 << 11; // UTF-8 names
        let header = |sig: u32, central: bool| -> Vec<u8> {
            let mut h = Vec::new();
            h.extend_from_slice(&sig.to_le_bytes());
            if central {
                h.extend_from_slice(&20u16.to_le_bytes()); // made by
            }
            h.extend_from_slice(&20u16.to_le_bytes()); // needed
            h.extend_from_slice(&flags.to_le_bytes());
            h.extend_from_slice(&method.to_le_bytes());
            h.extend_from_slice(&time.to_le_bytes());
            h.extend_from_slice(&date.to_le_bytes());
            h.extend_from_slice(&crc.sum().to_le_bytes());
            h.extend_from_slice(&(data.len() as u32).to_le_bytes());
            h.extend_from_slice(&(content.len() as u32).to_le_bytes());
            h.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
            h.extend_from_slice(&0u16.to_le_bytes()); // extra
            if central {
                h.extend_from_slice(&0u16.to_le_bytes()); // comment
                h.extend_from_slice(&0u16.to_le_bytes()); // disk
                h.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
                h.extend_from_slice(&0u32.to_le_bytes()); // external attrs
                h.extend_from_slice(&offset.to_le_bytes());
            }
            h.extend_from_slice(name_b);
            h
        };
        out.extend(header(0x0403_4b50, false));
        out.extend_from_slice(data);
        central.extend(header(0x0201_4b50, true));
    }
    let cd_offset = out.len() as u32;
    let cd_size = central.len() as u32;
    out.extend(central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let files = vec![
            ("bcf.version".to_string(), b"<Version/>".to_vec()),
            (
                "a/markup.bcf".to_string(),
                "ñandú ".repeat(200).into_bytes(),
            ),
            ("a/snapshot.png".to_string(), vec![0x89, b'P', b'N', b'G']),
        ];
        let bytes = write(&files);
        assert_eq!(read(&bytes).unwrap(), files);
        assert!(read(b"no zip").is_err());
    }
}
