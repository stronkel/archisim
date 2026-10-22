//! Changes between two versions of a project, element by element.
//!
//! The project is compared as its `.archisim` JSON, so every element kind
//! the file knows is covered without listing them here: a top-level list
//! whose entries all have an `id` is compared by id (an element added or
//! changed is a [`Op::Put`] of the whole entry, one removed a
//! [`Op::Del`]); any other list as a bag of values (phases, disciplines,
//! parameter values…); anything else is replaced whole ([`Op::Set`]).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// Keys of the file that say what it is, not what is in the project.
const HEADER: [&str; 4] = ["format", "version", "units", "generator"];

/// One change to the project file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum Op {
    /// An element added or changed: its whole entry in `list`.
    Put { list: String, entry: Value },
    /// The element `id` of `list` removed.
    Del { list: String, id: u64 },
    /// A value added to a list without ids.
    Add { list: String, entry: Value },
    /// A value taken out of a list without ids.
    Rem { list: String, entry: Value },
    /// Something that is not a list, replaced (`null`: removed).
    Set { key: String, value: Value },
}

impl Op {
    /// The element this change is about, if it is about one.
    pub fn element(&self) -> Option<u64> {
        match self {
            Op::Put { entry, .. } => entry_id(entry),
            Op::Del { id, .. } => Some(*id),
            _ => None,
        }
    }

    /// The list or key the change is in.
    pub fn list(&self) -> &str {
        match self {
            Op::Put { list, .. }
            | Op::Del { list, .. }
            | Op::Add { list, .. }
            | Op::Rem { list, .. } => list,
            Op::Set { key, .. } => key,
        }
    }
}

fn entry_id(v: &Value) -> Option<u64> {
    v.get("id").and_then(Value::as_u64)
}

/// Whether every entry has an id and no two share it.
fn keyed(items: &[Value]) -> bool {
    let mut seen = BTreeSet::new();
    items
        .iter()
        .all(|v| entry_id(v).is_some_and(|id| seen.insert(id)))
}

const EMPTY: &[Value] = &[];

fn items(v: Option<&Value>) -> Option<&[Value]> {
    match v {
        None => Some(EMPTY),
        Some(Value::Array(a)) => Some(a),
        _ => None,
    }
}

/// The changes that turn `old` into `new` (two project files as JSON).
pub fn diff(old: &Value, new: &Value) -> Vec<Op> {
    let empty = Map::new();
    let a = old.as_object().unwrap_or(&empty);
    let b = new.as_object().unwrap_or(&empty);
    let mut keys: Vec<&String> = b.keys().collect();
    keys.extend(a.keys().filter(|k| !b.contains_key(*k)));
    let mut ops = Vec::new();
    for key in keys {
        if HEADER.contains(&key.as_str()) {
            continue;
        }
        let (va, vb) = (a.get(key), b.get(key));
        if va == vb {
            continue;
        }
        match (items(va), items(vb)) {
            (Some(la), Some(lb)) if keyed(la) && keyed(lb) => diff_keyed(key, la, lb, &mut ops),
            (Some(la), Some(lb)) => diff_bag(key, la, lb, &mut ops),
            _ => ops.push(Op::Set {
                key: key.clone(),
                value: vb.cloned().unwrap_or(Value::Null),
            }),
        }
    }
    ops
}

fn diff_keyed(list: &str, a: &[Value], b: &[Value], ops: &mut Vec<Op>) {
    let before: BTreeMap<u64, &Value> = a.iter().filter_map(|v| Some((entry_id(v)?, v))).collect();
    let mut kept = BTreeSet::new();
    for v in b {
        let Some(id) = entry_id(v) else { continue };
        kept.insert(id);
        if before.get(&id) != Some(&v) {
            ops.push(Op::Put {
                list: list.into(),
                entry: v.clone(),
            });
        }
    }
    for id in before.keys().filter(|id| !kept.contains(id)) {
        ops.push(Op::Del {
            list: list.into(),
            id: *id,
        });
    }
}

fn diff_bag(list: &str, a: &[Value], b: &[Value], ops: &mut Vec<Op>) {
    let mut left: Vec<&Value> = a.iter().collect();
    let mut added = Vec::new();
    for v in b {
        if let Some(i) = left.iter().position(|x| *x == v) {
            left.remove(i);
        } else {
            added.push(v);
        }
    }
    for v in left {
        ops.push(Op::Rem {
            list: list.into(),
            entry: v.clone(),
        });
    }
    for v in added {
        ops.push(Op::Add {
            list: list.into(),
            entry: v.clone(),
        });
    }
}

/// Applies changes to a project file. Removing what is not there does
/// nothing (someone else already removed it).
pub fn apply(doc: &mut Value, ops: &[Op]) {
    if !doc.is_object() {
        *doc = Value::Object(Map::new());
    }
    let Some(map) = doc.as_object_mut() else {
        return;
    };
    for op in ops {
        match op {
            Op::Set { key, value } => {
                if value.is_null() {
                    map.remove(key);
                } else {
                    map.insert(key.clone(), value.clone());
                }
            }
            Op::Put { list, entry } => {
                let arr = list_mut(map, list);
                let id = entry_id(entry);
                match arr.iter_mut().find(|v| id.is_some() && entry_id(v) == id) {
                    Some(slot) => *slot = entry.clone(),
                    None => arr.push(entry.clone()),
                }
            }
            Op::Del { list, id } => {
                list_mut(map, list).retain(|v| entry_id(v) != Some(*id));
            }
            Op::Add { list, entry } => list_mut(map, list).push(entry.clone()),
            Op::Rem { list, entry } => {
                let arr = list_mut(map, list);
                if let Some(i) = arr.iter().position(|v| v == entry) {
                    arr.remove(i);
                }
            }
        }
    }
}

fn list_mut<'a>(map: &'a mut Map<String, Value>, list: &str) -> &'a mut Vec<Value> {
    let slot = map
        .entry(list.to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !slot.is_array() {
        *slot = Value::Array(Vec::new());
    }
    match slot {
        Value::Array(a) => a,
        _ => unreachable!("just made an array"),
    }
}

/// The elements a list of changes adds, changes or removes.
pub fn touched(ops: &[Op]) -> BTreeSet<u64> {
    ops.iter().filter_map(Op::element).collect()
}

/// Every element id in a project file (entries of keyed lists).
pub fn ids(doc: &Value) -> BTreeSet<u64> {
    let mut out = BTreeSet::new();
    let Some(map) = doc.as_object() else {
        return out;
    };
    for v in map.values() {
        if let Value::Array(a) = v {
            if keyed(a) {
                out.extend(a.iter().filter_map(entry_id));
            }
        }
    }
    out
}

/// Every number under an `id` key, at any depth: ids handed out from the
/// model's counter that live inside other entries (layers, parameters…).
pub fn all_ids(doc: &Value) -> BTreeSet<u64> {
    fn walk(v: &Value, out: &mut BTreeSet<u64>) {
        match v {
            Value::Object(m) => {
                for (k, x) in m {
                    if k == "id" {
                        if let Some(n) = x.as_u64() {
                            out.insert(n);
                        }
                    }
                    walk(x, out);
                }
            }
            Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut out = BTreeSet::new();
    walk(doc, &mut out);
    out
}
