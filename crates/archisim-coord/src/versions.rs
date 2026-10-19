//! What changed between two versions of a model, element by element,
//! matched by `GlobalId` (which is why the `GlobalId` must never change).

use crate::elements::{Element, ModelElements};

/// How one element changed.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Added,
    Removed,
    /// Same element: what is different (moved, reshaped, data).
    Modified {
        moved: Option<f64>,
        reshaped: bool,
        data: bool,
    },
}

#[derive(Debug, Clone)]
pub struct ElementChange {
    pub global_id: String,
    pub class: String,
    pub name: String,
    pub storey: Option<String>,
    pub change: Change,
}

impl ElementChange {
    /// «movido 0,25 m, forma, datos».
    pub fn describe(&self) -> String {
        match &self.change {
            Change::Added => "nuevo".into(),
            Change::Removed => "borrado".into(),
            Change::Modified {
                moved,
                reshaped,
                data,
            } => {
                let mut parts = Vec::new();
                if let Some(d) = moved {
                    parts.push(format!("movido {}", metres(*d)));
                }
                if *reshaped {
                    parts.push("cambia de forma".into());
                }
                if *data {
                    parts.push("cambian sus datos".into());
                }
                parts.join(", ")
            }
        }
    }
}

/// `0,25 m`, `3 mm`.
pub fn metres(d: f64) -> String {
    if d < 0.01 {
        format!("{:.0} mm", d * 1000.0)
    } else {
        format!("{:.2} m", d).replace('.', ",")
    }
}

#[derive(Debug, Clone, Default)]
pub struct VersionReport {
    pub changes: Vec<ElementChange>,
    pub unchanged: usize,
}

impl VersionReport {
    pub fn count(&self, f: impl Fn(&Change) -> bool) -> usize {
        self.changes.iter().filter(|c| f(&c.change)).count()
    }

    pub fn summary(&self) -> String {
        let added = self.count(|c| *c == Change::Added);
        let removed = self.count(|c| *c == Change::Removed);
        let modified = self.changes.len() - added - removed;
        let mut s = format!(
            "{added} nuevos, {removed} borrados, {modified} modificados y {} sin cambios.\n",
            self.unchanged
        );
        for c in &self.changes {
            s += &format!(
                "  {} {} «{}»{} {}: {}\n",
                match c.change {
                    Change::Added => "+",
                    Change::Removed => "−",
                    Change::Modified { .. } => "~",
                },
                archisim_ids::model::class_display(&c.class),
                c.name,
                c.storey
                    .as_deref()
                    .map(|s| format!(" ({s})"))
                    .unwrap_or_default(),
                c.global_id,
                c.describe()
            );
        }
        s
    }
}

/// Compares an older and a newer version of the same model.
pub fn compare(old: &ModelElements, new: &ModelElements) -> VersionReport {
    let old_by = old.by_guid();
    let new_by = new.by_guid();
    let mut r = VersionReport::default();
    let entry = |e: &Element, change: Change| ElementChange {
        global_id: e.global_id.clone(),
        class: e.class.clone(),
        name: e.name.clone(),
        storey: e.storey.clone(),
        change,
    };
    for e in &new.elements {
        match old_by.get(e.global_id.as_str()) {
            None => r.changes.push(entry(e, Change::Added)),
            Some(o) => {
                let (moved, reshaped) = shape_change(o, e);
                let data = o.data_hash != e.data_hash;
                if moved.is_none() && !reshaped && !data {
                    r.unchanged += 1;
                } else {
                    r.changes.push(entry(
                        e,
                        Change::Modified {
                            moved,
                            reshaped,
                            data,
                        },
                    ));
                }
            }
        }
    }
    for e in &old.elements {
        if !new_by.contains_key(e.global_id.as_str()) {
            r.changes.push(entry(e, Change::Removed));
        }
    }
    r
}

/// Tolerances: 1 mm and 1 cm³ (or a ten-thousandth of the volume).
fn shape_change(a: &Element, b: &Element) -> (Option<f64>, bool) {
    let (Some((alo, ahi)), Some((blo, bhi))) = (a.bounds, b.bounds) else {
        return (None, a.bounds.is_some() != b.bounds.is_some());
    };
    let size = |lo: [f64; 3], hi: [f64; 3]| [0, 1, 2].map(|i| hi[i] - lo[i]);
    let (sa, sb) = (size(alo, ahi), size(blo, bhi));
    let same_size = (0..3).all(|i| (sa[i] - sb[i]).abs() < 1e-3);
    let dv = (a.volume - b.volume).abs();
    let same_volume = dv < 1e-6_f64.max(a.volume.abs() * 1e-4);
    let reshaped = !(same_size && same_volume);
    let shift = (0..3)
        .map(|i| {
            let ca = (alo[i] + ahi[i]) / 2.0;
            let cb = (blo[i] + bhi[i]) / 2.0;
            (cb - ca).powi(2)
        })
        .sum::<f64>()
        .sqrt();
    let moved = (shift >= 1e-3 && !reshaped).then_some(shift);
    (moved, reshaped)
}
