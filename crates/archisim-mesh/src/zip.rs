//! A `.zip` as a manufacturer's download comes: folders with the model in
//! several formats, its textures, often a `.zip` inside another. Read
//! without unpacking it: the models it holds ([`Zip::models`]), and its
//! files for the model that refers to them ([`InZip`]).

use std::path::Path;

use crate::{Error, Resources, EXTENSIONS};

/// A file of the archive: its path inside and where its data is.
struct Entry {
    name: String,
    /// Which archive holds it: 0 the outer one, more: one inside.
    archive: usize,
    method: u16,
    offset: usize,
    size: usize,
}

/// An archive and the archives inside it.
pub struct Zip {
    archives: Vec<Vec<u8>>,
    entries: Vec<Entry>,
}

fn u16_at(b: &[u8], i: usize) -> Option<usize> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?) as usize)
}

fn u32_at(b: &[u8], i: usize) -> Option<usize> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?) as usize)
}

fn invalid(s: &str) -> Error {
    Error::Invalid(format!("zip: {s}"))
}

/// Names written in the old DOS code page (when the archive does not say
/// UTF-8): the accented letters of Spanish, French and German.
fn cp437(b: &[u8]) -> String {
    const HIGH: &str = "ÇüéâäàåçêëèïîìÄÅÉæÆôöòûùÿÖÜ¢£¥₧ƒáíóúñÑªº¿⌐¬½¼¡«»░▒▓│┤╡╢╖╕╣║╗╝╜╛┐└┴┬├─┼╞╟╚╔╩╦╠═╬╧╨╤╥╙╘╒╓╫╪┘┌█▄▌▐▀αßΓπΣσµτΦΘΩδ∞φε∩≡±≥≤⌠⌡÷≈°∙·√ⁿ²■ ";
    let high: Vec<char> = HIGH.chars().collect();
    b.iter()
        .map(|&c| {
            if c < 128 {
                c as char
            } else {
                high[(c - 128) as usize]
            }
        })
        .collect()
}

/// The entries of one archive (its central directory).
fn directory(b: &[u8], archive: usize) -> Result<Vec<Entry>, Error> {
    let start = b.len().saturating_sub(65_557);
    let end = (start..b.len().saturating_sub(21))
        .rev()
        .find(|&i| b[i..i + 4] == [0x50, 0x4b, 0x05, 0x06])
        .ok_or_else(|| invalid("no es un .zip"))?;
    let count = u16_at(b, end + 10).unwrap_or(0);
    let mut at = u32_at(b, end + 16).ok_or_else(|| invalid("directorio"))?;
    if at == 0xFFFF_FFFF {
        return Err(invalid("demasiado grande (ZIP64): descomprímalo antes"));
    }
    let mut out = Vec::new();
    for _ in 0..count {
        if b.get(at..at + 4) != Some(&[0x50, 0x4b, 0x01, 0x02]) {
            break;
        }
        let read = || -> Option<Entry> {
            let flags = u16_at(b, at + 8)?;
            let method = u16_at(b, at + 10)? as u16;
            let size = u32_at(b, at + 20)?;
            let name_len = u16_at(b, at + 28)?;
            let local = u32_at(b, at + 42)?;
            let raw = b.get(at + 46..at + 46 + name_len)?;
            let name = if flags & 0x800 != 0 {
                String::from_utf8_lossy(raw).into_owned()
            } else {
                match std::str::from_utf8(raw) {
                    Ok(s) => s.to_string(),
                    Err(_) => cp437(raw),
                }
            };
            // The data follows the local header, whose extra field may
            // differ from the directory's.
            if b.get(local..local + 4) != Some(&[0x50, 0x4b, 0x03, 0x04]) {
                return None;
            }
            let offset = local + 30 + u16_at(b, local + 26)? + u16_at(b, local + 28)?;
            Some(Entry {
                name: name.replace('\\', "/"),
                archive,
                method,
                offset,
                size,
            })
        };
        let len = 46
            + u16_at(b, at + 28).unwrap_or(0)
            + u16_at(b, at + 30).unwrap_or(0)
            + u16_at(b, at + 32).unwrap_or(0);
        if let Some(e) = read() {
            if !e.name.ends_with('/') {
                out.push(e);
            }
        }
        at += len;
    }
    Ok(out)
}

/// Lower case, `/` separators, without `./`.
fn norm(s: &str) -> String {
    s.replace('\\', "/").trim_start_matches("./").to_lowercase()
}

fn file_name(s: &str) -> &str {
    s.rsplit('/').next().unwrap_or(s)
}

fn extension(s: &str) -> String {
    Path::new(s)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// The formats of models a download may bring that are not read, with
/// what they are (to say so).
const OTHERS: [(&str, &str); 12] = [
    ("skp", "SketchUp"),
    ("dwg", "DWG"),
    ("dxf", "DXF"),
    ("max", "3ds Max"),
    ("c4d", "Cinema 4D"),
    ("rfa", "Revit"),
    ("gsm", "ArchiCAD"),
    ("3dm", "Rhino"),
    ("blend", "Blender"),
    ("ifc", "IFC"),
    ("usdz", "USDZ"),
    ("step", "STEP"),
];

/// How much a format is preferred when an archive has several: FBX keeps
/// hierarchy, materials and textures, glTF too; OBJ and 3DS lose detail;
/// STL has no materials.
fn preference(ext: &str) -> usize {
    ["fbx", "glb", "gltf", "obj", "dae", "3ds", "stl"]
        .iter()
        .position(|e| *e == ext)
        .unwrap_or(9)
}

impl Zip {
    pub fn open(bytes: Vec<u8>) -> Result<Zip, Error> {
        let mut zip = Zip {
            entries: directory(&bytes, 0)?,
            archives: vec![bytes],
        };
        // Archives inside (one level, as downloads have them).
        let inner: Vec<usize> = (0..zip.entries.len())
            .filter(|&i| zip.entries[i].archive == 0 && extension(&zip.entries[i].name) == "zip")
            .collect();
        for i in inner {
            let Some(data) = zip.data(i) else { continue };
            let k = zip.archives.len();
            let Ok(list) = directory(&data, k) else {
                continue;
            };
            let prefix = zip.entries[i].name.clone();
            zip.archives.push(data);
            zip.entries.extend(list.into_iter().map(|mut e| {
                e.name = format!("{prefix}/{}", e.name);
                e
            }));
        }
        Ok(zip)
    }

    fn data(&self, i: usize) -> Option<Vec<u8>> {
        let e = &self.entries[i];
        let raw = self.archives[e.archive].get(e.offset..e.offset + e.size)?;
        match e.method {
            0 => Some(raw.to_vec()),
            8 => miniz_oxide::inflate::decompress_to_vec(raw).ok(),
            _ => None,
        }
    }

    /// Every file's path inside.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|e| e.name.as_str())
    }

    /// The models it holds that can be read, best first (by format, then
    /// by path).
    pub fn models(&self) -> Vec<String> {
        let mut out: Vec<&str> = self
            .names()
            .filter(|n| EXTENSIONS.contains(&extension(n).as_str()))
            .filter(|n| !file_name(n).starts_with("._") && !n.contains("__MACOSX"))
            .collect();
        out.sort_by_key(|n| (preference(&extension(n)), n.to_lowercase()));
        out.into_iter().map(String::from).collect()
    }

    /// The other model formats it brings, which are not read («SketchUp»,
    /// «Revit»…).
    pub fn other_formats(&self) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = Vec::new();
        for n in self.names() {
            let ext = extension(n);
            if let Some((_, what)) = OTHERS.iter().find(|(e, _)| *e == ext) {
                if !out.contains(what) {
                    out.push(what);
                }
            }
        }
        out
    }

    /// The file at `name` (its path inside, any case).
    pub fn read(&self, name: &str) -> Option<Vec<u8>> {
        let n = norm(name);
        let i = self.entries.iter().position(|e| norm(&e.name) == n)?;
        self.data(i)
    }
}

/// The files next to a model inside an archive: by their path from the
/// model's folder, or by their name anywhere in the archive (catalogues
/// write the path of their author's computer).
pub struct InZip<'a> {
    pub zip: &'a Zip,
    /// The model's folder inside, `""` at the top.
    pub folder: String,
}

impl Resources for InZip<'_> {
    fn get(&self, name: &str) -> Option<Vec<u8>> {
        let name = name.replace('\\', "/");
        let name = name.trim_start_matches("file://");
        let mut path = self.folder.clone();
        for part in name.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    path = path
                        .rsplit_once('/')
                        .map_or(String::new(), |(a, _)| a.to_string());
                }
                p => {
                    if !path.is_empty() {
                        path.push('/');
                    }
                    path.push_str(p);
                }
            }
        }
        if let Some(d) = self.zip.read(&path) {
            return Some(d);
        }
        let want = file_name(name).to_lowercase();
        // The same name: in the model's folder (or below) first.
        let folder = norm(&self.folder);
        let mut found: Vec<usize> = (0..self.zip.entries.len())
            .filter(|&i| file_name(&self.zip.entries[i].name).to_lowercase() == want)
            .collect();
        found.sort_by_key(|&i| !norm(&self.zip.entries[i].name).starts_with(&folder));
        found.into_iter().find_map(|i| self.zip.data(i))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_old_dos_letters() {
        let all: Vec<u8> = (128..=255).collect();
        assert_eq!(super::cp437(&all).chars().count(), 128);
        assert_eq!(
            super::cp437(&[0x53, 0x69, 0x6c, 0x6c, 0xa2, 0x6e]),
            "Sillón"
        );
    }
}
