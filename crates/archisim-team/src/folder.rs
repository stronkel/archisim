//! The shared folder and the files in it.
//!
//! ```text
//! Casa en la playa/            the folder Google Drive (or a NAS) shares
//!   equipo.asequipo            what the project is (open this one)
//!   base.archisim              the project when it was shared
//!   miembros/antonio.json      each person: name, colour, block of ids
//!   cambios/antonio.json       each person's published changes, in order
//!   reservas/antonio.json      what each person has reserved
//!   presencia/antonio.json     when each person was last seen, and where
//!   mensajes/antonio.json      requests and notes each person has sent
//!   trabajo/antonio.archisim   each person's working copy (with changes
//!   trabajo/antonio.json       not yet published) and what they received
//! ```
//!
//! Every file under a person's name is written by that person only, so the
//! sync service never has to merge two versions of the same file.

use crate::diff::Op;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Extension of the file that marks a shared project folder.
pub const EXTENSION: &str = "asequipo";
/// Name of that file.
pub const TEAM_FILE: &str = "equipo.asequipo";
pub const FORMAT: &str = "ArchiSim equipo";
pub const VERSION: u32 = 1;

pub(crate) const BASE: &str = "base.archisim";
pub(crate) const MEMBERS: &str = "miembros";
pub(crate) const CHANGES: &str = "cambios";
pub(crate) const RESERVATIONS: &str = "reservas";
pub(crate) const PRESENCE: &str = "presencia";
pub(crate) const MESSAGES: &str = "mensajes";
pub(crate) const WORK: &str = "trabajo";

/// `equipo.asequipo`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TeamFile {
    pub format: String,
    pub version: u32,
    /// Project name.
    pub name: String,
    /// When it was shared (seconds since 1970).
    pub created: u64,
    /// Who shared it.
    pub created_by: String,
}

/// A person working on the project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Member {
    /// File name of their files (from the name, without accents).
    pub slug: String,
    pub name: String,
    /// `#RRGGBB`: their reservations are drawn in it.
    pub color: String,
    /// Their block of ids: new elements take ids from `slot << 32` on.
    pub slot: u64,
    pub joined: u64,
}

/// Size of each person's block of ids.
pub const BLOCK_BITS: u32 = 32;
/// Slots go up to this, so every id stays below `archisim_core::MAX_ID`.
pub const MAX_SLOT: u64 = 1 << 20;

impl Member {
    /// The first id of the block.
    pub fn first_id(&self) -> u64 {
        self.slot << BLOCK_BITS
    }

    /// Whether `id` is one this person handed out.
    pub fn owns_id(&self, id: u64) -> bool {
        id >> BLOCK_BITS == self.slot
    }

    pub fn initials(&self) -> String {
        let mut words = self.name.split_whitespace();
        let first = words.next().and_then(|w| w.chars().next());
        let second = words.next().and_then(|w| w.chars().next());
        match (first, second) {
            (Some(a), Some(b)) => format!("{a}{b}").to_uppercase(),
            (Some(a), None) => a.to_uppercase().collect(),
            _ => "?".into(),
        }
    }

    pub fn rgb(&self) -> [u8; 3] {
        parse_color(&self.color).unwrap_or([90, 90, 90])
    }
}

pub(crate) fn parse_color(s: &str) -> Option<[u8; 3]> {
    let h = s.strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let n = u32::from_str_radix(h, 16).ok()?;
    Some([(n >> 16) as u8, (n >> 8) as u8, n as u8])
}

/// One publication: a set of changes with a note.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Change {
    /// 1, 2, 3… for each person.
    pub seq: u64,
    /// Logical clock: one more than anything its author had seen. Changes
    /// apply in order of clock (and author, if equal), the same on every
    /// computer whatever their clocks say.
    pub clock: u64,
    pub time: u64,
    pub message: String,
    /// `.archisim` format version of the entries.
    pub version: u32,
    pub ops: Vec<Op>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ChangeFile {
    pub author: String,
    pub changes: Vec<Change>,
}

/// An element reserved.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Claim {
    pub id: u64,
    pub clock: u64,
    pub time: u64,
}

/// A zone reserved: everything of a level inside a polygon (or the whole
/// level, with no polygon).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Zone {
    pub name: String,
    pub level: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub polygon: Vec<[f64; 2]>,
    pub clock: u64,
    pub time: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ReservationFile {
    pub author: String,
    #[serde(default)]
    pub elements: Vec<Claim>,
    #[serde(default)]
    pub zones: Vec<Zone>,
}

/// Where a person is.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Presence {
    pub time: u64,
    /// Which computer (to warn when someone is open on two at once).
    pub machine: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u64>,
    /// Shared only if they choose to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<[f64; 2]>,
    /// Whether the program is open (false once closed).
    #[serde(default)]
    pub open: bool,
}

/// A request or a note to someone.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    /// 1, 2, 3… for each sender.
    pub n: u64,
    /// Slug of who it is for.
    pub to: String,
    pub time: u64,
    pub text: String,
    /// Elements it is about (a request to release them).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MessageFile {
    pub author: String,
    #[serde(default)]
    pub messages: Vec<Message>,
}

/// What a person had received when they last saved their working copy.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WorkState {
    /// Last change received from each person (their `seq`).
    pub seen: BTreeMap<String, u64>,
    /// Last message read from each person (their `n`).
    #[serde(default)]
    pub read: BTreeMap<String, u64>,
    pub machine: String,
    pub time: u64,
}

pub(crate) fn dir_of(root: &Path, sub: &str) -> PathBuf {
    root.join(sub)
}

pub(crate) fn file_of(root: &Path, sub: &str, slug: &str) -> PathBuf {
    root.join(sub).join(format!("{slug}.json"))
}

/// Reads a JSON file; `None` if it is not there.
pub(crate) fn read_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map(Some)
            .map_err(|e| format!("{} no se entiende: {e}", shown(path))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("no se puede leer {}: {e}", shown(path))),
    }
}

/// Writes a file whole or not at all: to a temporary name, then renamed,
/// so the sync service never uploads half a file.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("no se puede crear {}: {e}", shown(dir)))?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)
        .map_err(|e| format!("no se puede escribir {}: {e}", shown(path)))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("no se puede escribir {}: {e}", shown(path)))
}

pub(crate) fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    write_atomic(path, text.as_bytes())
}

/// The slugs that have a file in a subfolder: `antonio.json` → `antonio`.
/// Copies the sync service makes on a conflict (`antonio (1).json`) are
/// not anyone's file and are left alone.
pub(crate) fn slugs_in(root: &Path, sub: &str) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir_of(root, sub))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let slug = name.strip_suffix(".json")?;
            is_slug(slug).then(|| slug.to_string())
        })
        .collect();
    out.sort();
    out
}

pub(crate) fn is_slug(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// A name as a file name: lower case, no accents, dashes for spaces.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().chars().flat_map(char::to_lowercase) {
        let c = match c {
            'á' | 'à' | 'ä' | 'â' | 'ã' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' | 'õ' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        };
        if c == '-' && (out.is_empty() || out.ends_with('-')) {
            continue;
        }
        out.push(c);
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "persona".into()
    } else {
        out
    }
}

pub(crate) fn shown(p: &Path) -> String {
    p.display().to_string()
}

/// Seconds since 1970.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Colours given to people in turn: easy to tell apart on white paper.
pub const COLORS: [&str; 10] = [
    "#D9480F", "#1971C2", "#2F9E44", "#AE3EC9", "#F08C00", "#0C8599", "#C2255C", "#5C940D",
    "#4263EB", "#868E96",
];
