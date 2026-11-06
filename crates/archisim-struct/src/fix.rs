//! «Arreglar lo que no cumple»: the smallest changes that make the
//! failing checks pass, worked out on a copy of the model and shown before
//! they are made.
//!
//! Round after round, on the copy:
//!
//! 1. the elements whose checks fail and for which the calculation gives
//!    a reinforcement (beams, columns, footings, solid slabs) take it;
//! 2. what still fails takes the next size up: a concrete or timber
//!    column 5 cm more each side, a beam 5 cm more deep (up to 1,20 m,
//!    then 5 cm wider), a round column 5 cm more across, a steel member
//!    the next larger section of its kind among the project's types, a
//!    pad footing 10 cm more each side (and deep enough to stay rigid), a
//!    strip footing 10 cm wider;
//! 3. the calculation runs again on the copy.
//!
//! It stops when nothing fails or nothing more can be done; what is left
//! is said with the calculation's own advice (a wall, a slab's depth, an
//! off-centre footing: they ask for a decision of the project). Bigger
//! sections make the frame stiffer and change its forces: that is why it
//! goes round by round, from the smallest step.

use archisim_core::{
    BeamId, ColumnId, FootingId, FootingKind, MaterialId, MemberUsage, Model, ModelError, Profile,
    Reinforcement,
};

use crate::lightframe::{Check, Status, Subject};

/// One change the fix makes.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// A column or beam takes a type of this section (one of the project
    /// that has it, or a new one).
    Section {
        element: u64,
        usage: MemberUsage,
        name: String,
        material: MaterialId,
        profile: Profile,
    },
    /// A footing takes these sizes.
    Footing {
        id: FootingId,
        width: f64,
        length: Option<f64>,
        depth: f64,
    },
    /// An element takes this reinforcement.
    Reinforce {
        element: u64,
        reinforcement: Reinforcement,
    },
}

/// A line of what changes, for the window: the element and its value
/// before and after.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub element: u64,
    pub name: String,
    pub what: &'static str,
    pub before: String,
    pub after: String,
}

/// The fix of a model.
#[derive(Debug, Clone, PartialEq)]
pub struct Fix {
    /// The changes, in the order they are made.
    pub changes: Vec<Change>,
    /// What changes, element by element (the last value of each).
    pub lines: Vec<Line>,
    /// Checks that fail before and after.
    pub fails_before: usize,
    pub fails_after: usize,
    /// What it could not fix, with the calculation's advice.
    pub left: Vec<String>,
}

const MAX_ROUNDS: usize = 40;
const MAX_BEAM_DEPTH: f64 = 1.2;
const MAX_COLUMN: f64 = 1.0;
const MAX_FOOTING: f64 = 6.0;

fn checks_of(model: &Model) -> Vec<Check> {
    let a = crate::analysis::analyse(model);
    crate::all_checks(model, &a)
}

fn fails(checks: &[Check]) -> usize {
    checks.iter().filter(|c| c.status == Status::Fail).count()
}

/// The element a check is about, as the id the reinforcement uses.
fn element_of(model: &Model, s: Subject) -> Option<u64> {
    match s {
        Subject::Beam(b) => Some(b.0),
        Subject::Column(c) => Some(c.0),
        Subject::Footing(id) => Some(id),
        Subject::Floor(s) => model.slab(s).map(|s| s.id.0),
        _ => None,
    }
}

/// The name of a section in centimetres: «35×35», «Ø40».
fn size_name(p: &Profile) -> String {
    let cm = |v: f64| format!("{:.0}", v * 100.0);
    match p {
        Profile::Rectangle { width, depth } => format!("{}×{}", cm(*width), cm(*depth)),
        Profile::Circle { diameter } => format!("Ø{}", cm(*diameter)),
        _ => String::new(),
    }
}

/// A type's name with its size changed: «Pilar HA 30×30» → «Pilar HA
/// 35×35».
fn renamed(old: &str, p: &Profile) -> String {
    let size = size_name(p);
    if size.is_empty() {
        return old.to_string();
    }
    let words: Vec<&str> = old.split_whitespace().collect();
    let keep: Vec<&str> = words
        .iter()
        .copied()
        .take_while(|w| !w.chars().any(|c| c.is_ascii_digit()))
        .collect();
    let base = if keep.is_empty() {
        old.to_string()
    } else {
        keep.join(" ")
    };
    format!("{base} {size}")
}

/// The next section up of a member, if there is one.
fn bigger(model: &Model, usage: MemberUsage, ty: &archisim_core::MemberType) -> Option<Profile> {
    let step = 0.05;
    match ty.profile {
        Profile::Rectangle { width, depth } => match usage {
            MemberUsage::Column => {
                let (w, d) = (width + step, depth + step);
                (w.max(d) <= MAX_COLUMN + 1e-9).then_some(Profile::Rectangle { width: w, depth: d })
            }
            MemberUsage::Beam => {
                if depth + step <= MAX_BEAM_DEPTH + 1e-9 {
                    Some(Profile::Rectangle {
                        width,
                        depth: depth + step,
                    })
                } else if width + step <= MAX_COLUMN {
                    Some(Profile::Rectangle {
                        width: width + step,
                        depth,
                    })
                } else {
                    None
                }
            }
        },
        Profile::Circle { diameter } => {
            (diameter + step <= MAX_COLUMN + 1e-9).then_some(Profile::Circle {
                diameter: diameter + step,
            })
        }
        // Steel: the next deeper section of the same kind and material
        // among the project's types.
        p => {
            let kind = std::mem::discriminant(&p);
            let depth = p.size().1;
            model
                .member_types()
                .filter(|t| {
                    t.material == ty.material
                        && t.usage == usage
                        && std::mem::discriminant(&t.profile) == kind
                        && t.profile.size().1 > depth + 1e-6
                })
                .min_by(|a, b| a.profile.size().1.total_cmp(&b.profile.size().1))
                .map(|t| t.profile)
        }
    }
}

fn same_profile(a: &Profile, b: &Profile) -> bool {
    format!("{a:?}") == format!("{b:?}")
}

/// Makes a change on a model.
pub fn apply(model: &mut Model, change: &Change) -> Result<(), ModelError> {
    match change {
        Change::Section {
            element,
            usage,
            name,
            material,
            profile,
        } => {
            let found = model
                .member_types()
                .find(|t| {
                    t.usage == *usage
                        && t.material == *material
                        && same_profile(&t.profile, profile)
                })
                .map(|t| t.id);
            let ty = match found {
                Some(t) => t,
                None => model.add_member_type(name.clone(), *usage, *material, *profile)?,
            };
            match usage {
                MemberUsage::Column => {
                    model.edit_column(ColumnId(*element), |c| c.member_type = ty)
                }
                MemberUsage::Beam => model.edit_beam(BeamId(*element), |b| b.member_type = ty),
            }
        }
        Change::Footing {
            id,
            width,
            length,
            depth,
        } => model.edit_footing(*id, |f| {
            f.width = *width;
            f.depth = *depth;
            if let (FootingKind::Pad { length: l, .. }, Some(v)) = (&mut f.kind, length) {
                *l = *v;
            }
        }),
        Change::Reinforce {
            element,
            reinforcement,
        } => model.set_reinforcement(*element, Some(*reinforcement)),
    }
}

/// The next step of a failing element's section or size, if it has one.
fn grow(model: &Model, s: Subject) -> Option<Change> {
    match s {
        Subject::Column(id) => {
            let c = model.column(id)?;
            let t = model.member_type(c.member_type)?;
            let p = bigger(model, MemberUsage::Column, t)?;
            Some(Change::Section {
                element: id.0,
                usage: MemberUsage::Column,
                name: renamed(&t.name, &p),
                material: t.material,
                profile: p,
            })
        }
        Subject::Beam(id) => {
            let b = model.beam(id)?;
            let t = model.member_type(b.member_type)?;
            let p = bigger(model, MemberUsage::Beam, t)?;
            Some(Change::Section {
                element: id.0,
                usage: MemberUsage::Beam,
                name: renamed(&t.name, &p),
                material: t.material,
                profile: p,
            })
        }
        Subject::Footing(id) => {
            let f = model.footing(FootingId(id))?;
            // Off-centre footings and pile caps ask for a decision.
            if model.footing_offset(f.id).is_some() || model.footing_piles(f.id).is_some() {
                return None;
            }
            match f.kind {
                FootingKind::Pad { length, column, .. } => {
                    let (w, l) = (f.width + 0.1, length + 0.1);
                    if w.max(l) > MAX_FOOTING {
                        return None;
                    }
                    let post = column
                        .and_then(|c| model.column(c))
                        .and_then(|c| model.member_type(c.member_type))
                        .map_or(0.3, |t| {
                            let (a, b) = t.profile.size();
                            a.max(b)
                        });
                    // Rigid: overhang ≤ 2 depths, rounded up to 5 cm.
                    let rigid = ((w.max(l) - post) / 4.0 / 0.05).ceil() * 0.05;
                    Some(Change::Footing {
                        id: f.id,
                        width: w,
                        length: Some(l),
                        depth: f.depth.max(rigid).max(0.4),
                    })
                }
                FootingKind::Strip { .. } => {
                    let w = f.width + 0.1;
                    (w <= MAX_FOOTING).then_some(Change::Footing {
                        id: f.id,
                        width: w,
                        length: None,
                        depth: f.depth,
                    })
                }
                FootingKind::TieBeam { .. } => None,
            }
        }
        _ => None,
    }
}

fn subject_name(model: &Model, s: Subject) -> String {
    match s {
        Subject::Beam(b) => format!(
            "Viga {}{}",
            b.0,
            model
                .beam(b)
                .and_then(|x| model.member_type(x.member_type))
                .map_or(String::new(), |t| format!(" ({})", t.name))
        ),
        Subject::Column(c) => format!(
            "Pilar {}{}",
            c.0,
            model
                .column(c)
                .and_then(|x| model.member_type(x.member_type))
                .map_or(String::new(), |t| format!(" ({})", t.name))
        ),
        Subject::Footing(id) => format!("Zapata {id}"),
        Subject::Floor(s) => format!("Forjado {}", s.0),
        Subject::Wall(w) => format!("Muro {}", w.0),
        Subject::Roof(r) => format!("Cubierta {}", r.0),
        Subject::Stair(s) => format!("Escalera {}", s.0),
        Subject::Building => "Edificio".into(),
    }
}

/// Works out the fix of a model (it is not changed).
pub fn fix(model: &Model) -> Fix {
    let original = crate::phases::as_built(model).into_owned();
    let mut m = original.clone();
    let mut changes: Vec<Change> = Vec::new();
    let first = checks_of(&m);
    let fails_before = fails(&first);
    let mut checks = first;
    let mut reinforced = std::collections::BTreeSet::new();
    for _ in 0..MAX_ROUNDS {
        let failing: Vec<Subject> = {
            let mut v: Vec<Subject> = checks
                .iter()
                .filter(|c| c.status == Status::Fail)
                .map(|c| c.subject)
                .collect();
            v.sort_by_key(|s| format!("{s:?}"));
            v.dedup();
            v
        };
        if failing.is_empty() {
            break;
        }
        let mut changed = false;
        // 1. The reinforcement the calculation asks for, the first time an
        //    element fails; 2. a size up if it still fails, with the
        //    reinforcement of the new size.
        let an = crate::analysis::analyse(&m);
        let proposals = crate::proposals::proposals(&m, &an);
        let mut grown = Vec::new();
        for s in &failing {
            let id = element_of(&m, *s);
            let fresh = id.is_some_and(|id| !reinforced.contains(&id));
            if fresh {
                let id = id.unwrap();
                reinforced.insert(id);
                if let Some(p) = proposals
                    .iter()
                    .find(|p| p.element == id && p.now != Some(p.reinforcement))
                {
                    let c = Change::Reinforce {
                        element: id,
                        reinforcement: p.reinforcement,
                    };
                    if apply(&mut m, &c).is_ok() {
                        changes.push(c);
                        changed = true;
                        continue;
                    }
                }
            }
            if let Some(c) = grow(&m, *s) {
                if apply(&mut m, &c).is_ok() {
                    changes.push(c);
                    changed = true;
                    grown.extend(id);
                }
            }
        }
        if !grown.is_empty() {
            let an = crate::analysis::analyse(&m);
            for p in crate::proposals::proposals(&m, &an) {
                if grown.contains(&p.element) && p.now != Some(p.reinforcement) {
                    let c = Change::Reinforce {
                        element: p.element,
                        reinforcement: p.reinforcement,
                    };
                    if apply(&mut m, &c).is_ok() {
                        changes.push(c);
                    }
                }
            }
        }
        if !changed {
            break;
        }
        checks = checks_of(&m);
    }
    let after = checks_of(&m);
    let fails_after = fails(&after);
    let mut left: Vec<String> = after
        .iter()
        .filter(|c| c.status == Status::Fail)
        .map(|c| {
            format!(
                "{} · {}{}",
                subject_name(&m, c.subject),
                c.rule,
                c.advice
                    .as_ref()
                    .map_or(String::new(), |a| format!(": {a}"))
            )
        })
        .collect();
    left.dedup();
    let lines = lines_of(&original, &m, &changes);
    Fix {
        changes,
        lines,
        fails_before,
        fails_after,
        left,
    }
}

/// What changes, element by element: its first value and its last.
pub fn lines_of(before: &Model, after: &Model, changes: &[Change]) -> Vec<Line> {
    let mut out: Vec<((u64, u8), Line)> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for c in changes {
        let (key, line) = match c {
            Change::Section { element, usage, .. } => {
                let (b, a, name) = match usage {
                    MemberUsage::Column => {
                        let t = |m: &Model| {
                            m.column(ColumnId(*element))
                                .and_then(|x| m.member_type(x.member_type))
                                .map_or(String::new(), |t| t.name.clone())
                        };
                        (t(before), t(after), format!("Pilar {element}"))
                    }
                    MemberUsage::Beam => {
                        let t = |m: &Model| {
                            m.beam(BeamId(*element))
                                .and_then(|x| m.member_type(x.member_type))
                                .map_or(String::new(), |t| t.name.clone())
                        };
                        (t(before), t(after), format!("Viga {element}"))
                    }
                };
                (
                    (*element, 0u8),
                    Line {
                        element: *element,
                        name,
                        what: "Sección",
                        before: b,
                        after: a,
                    },
                )
            }
            Change::Footing { id, .. } => {
                let size = |m: &Model| {
                    m.footing(*id).map_or(String::new(), |f| match f.kind {
                        FootingKind::Pad { length, .. } => format!(
                            "{}×{}×{} m",
                            crate::text::dec(f.width, 2),
                            crate::text::dec(length, 2),
                            crate::text::dec(f.depth, 2)
                        ),
                        _ => format!(
                            "{} m de ancho, {} m de canto",
                            crate::text::dec(f.width, 2),
                            crate::text::dec(f.depth, 2)
                        ),
                    })
                };
                (
                    (id.0, 1),
                    Line {
                        element: id.0,
                        name: format!("Zapata {}", id.0),
                        what: "Tamaño",
                        before: size(before),
                        after: size(after),
                    },
                )
            }
            Change::Reinforce { element, .. } => {
                let r = |m: &Model| {
                    m.reinforcement(*element)
                        .map_or("sin armado".to_string(), |r| r.summary())
                };
                (
                    (*element, 2),
                    Line {
                        element: *element,
                        name: element_name(before, *element),
                        what: "Armado",
                        before: r(before),
                        after: r(after),
                    },
                )
            }
        };
        if seen.insert(key) {
            out.push((key, line));
        }
    }
    out.sort_by_key(|(k, _)| *k);
    out.into_iter()
        .map(|(_, l)| l)
        .filter(|l| l.before != l.after)
        .collect()
}

fn element_name(model: &Model, id: u64) -> String {
    if model.column(ColumnId(id)).is_some() {
        format!("Pilar {id}")
    } else if model.beam(BeamId(id)).is_some() {
        format!("Viga {id}")
    } else if model.footing(FootingId(id)).is_some() {
        format!("Zapata {id}")
    } else {
        format!("Forjado {id}")
    }
}

/// Makes the fix on the model: the same changes, in order.
pub fn apply_all(model: &mut Model, fix: &Fix) -> Result<(), ModelError> {
    for c in &fix.changes {
        apply(model, c)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_beam_on_thin_columns_gets_what_it_needs() {
        let mut m = Model::new("Pórtico");
        let (_, beam) = crate::members::tests::portal(&mut m);
        // Make it fail: a span far too long for its section.
        let far = m.columns().nth(1).unwrap().id;
        m.edit_column(far, |c| c.position.x = 9.0).unwrap();
        m.edit_beam(beam, |b| b.end.x = 9.0).unwrap();
        let before = checks_of(&m);
        let fix = fix(&m);
        assert_eq!(fix.fails_before, fails(&before));
        assert!(fix.fails_before > 0, "it must fail to begin with");
        assert_eq!(
            fix.fails_after, 0,
            "{} → {}: {:?}",
            fix.fails_before, fix.fails_after, fix.left
        );
        assert!(!fix.lines.is_empty());
        // The same changes on the model give what was worked out.
        let mut done = m.clone();
        apply_all(&mut done, &fix).unwrap();
        assert_eq!(fails(&checks_of(&done)), fix.fails_after);
    }

    #[test]
    fn sizes_and_names() {
        let p = Profile::Rectangle {
            width: 0.35,
            depth: 0.35,
        };
        assert_eq!(renamed("Pilar HA 30×30", &p), "Pilar HA 35×35");
        assert_eq!(renamed("Mi pilar", &p), "Mi pilar 35×35");
    }
}
