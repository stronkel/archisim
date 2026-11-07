//! «Arreglar lo que no cumple» in the code check: the changes that make
//! the failing items pass when there is one obvious, smallest answer,
//! worked out on a copy of the model and shown before they are made.
//!
//! - the structure (and the light frames): the fix of the calculation
//!   (`archisim_struct::fix`), and the reinforcement it proposes for the
//!   elements whose drawn reinforcement breaks a construction rule;
//! - water and drainage: the pipe nearest an appliance takes the diameter
//!   HS 4 or HS 5 asks for (the largest of the appliances it serves), each
//!   stretch of the water supply the one its flow asks for, and
//!   a drain with too little or too much slope takes the nearest allowed
//!   (2 % for a branch, 1 % for a hung collector, 2 % for a buried one);
//! - doors: the door takes a type of its family wide enough for the clear
//!   width asked (an existing one, or a copy of its own a few centimetres
//!   wider), and a door that must open towards the way out is turned
//!   round, keeping the hinges on the same jamb.
//!
//! What asks for a decision of the project (a corridor, a room's size, a
//! stair, a turning circle, an exit) is listed with what fails.

use std::collections::BTreeMap;

use archisim_core::{
    Hand, Model, ModelError, OpeningId, OpeningTypeId, RunId, ServiceSystem, Value, WallSide,
};

use crate::{check, services, Item, Report, Section, Status, Subject};

/// One change the fix makes.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// A change of the structure's fix.
    Structure(archisim_struct::fix::Change),
    /// A pipe's diameter.
    RunWidth { run: RunId, width: f64 },
    /// A downpipe's diameter.
    DeviceWidth {
        device: archisim_core::DeviceId,
        width: f64,
    },
    /// A drain's slope.
    RunSlope { run: RunId, slope: f64 },
    /// A door of a type of its family this wide (the opening, in metres).
    DoorWidth { opening: OpeningId, width: f64 },
    /// A door opening to the other side of its wall.
    Handing {
        opening: OpeningId,
        opens_to: WallSide,
        hand: Hand,
    },
}

/// A line of what changes, for the window.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub subject: Subject,
    pub name: String,
    pub what: &'static str,
    pub before: String,
    pub after: String,
}

/// The fix of the code check.
#[derive(Debug, Clone, PartialEq)]
pub struct Fix {
    pub changes: Vec<Change>,
    pub lines: Vec<Line>,
    pub fails_before: usize,
    pub fails_after: usize,
    /// What is still wrong, to be decided by the project.
    pub left: Vec<String>,
}

fn fails(r: &Report) -> usize {
    r.count(Status::Fails)
}

/// The first length in metres of a «≥ 0,80 m».
fn metres(s: &str) -> Option<f64> {
    let t: String = s
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == ',' || *c == '.')
        .collect();
    t.replace(',', ".").parse().ok()
}

/// Makes a change on a model.
pub fn apply(model: &mut Model, change: &Change) -> Result<(), ModelError> {
    match change {
        Change::Structure(c) => archisim_struct::fix::apply(model, c),
        Change::RunWidth { run, width } => model.edit_run(*run, |r| r.width = *width),
        Change::DeviceWidth { device, width } => model.edit_device(*device, |d| {
            d.width = *width;
            d.depth = *width;
        }),
        Change::RunSlope { run, slope } => model.edit_run(*run, |r| r.slope = *slope),
        Change::DoorWidth { opening, width } => {
            let o = model
                .opening(*opening)
                .ok_or(ModelError::UnknownOpening(*opening))?;
            let t = model
                .opening_type(o.opening_type)
                .ok_or(ModelError::UnknownOpeningType(o.opening_type))?
                .clone();
            let mut values = t.values.clone();
            values.insert("width".into(), Value::Number(*width));
            let found: Option<OpeningTypeId> = model
                .opening_types()
                .find(|x| x.family == t.family && x.values == values)
                .map(|x| x.id);
            let ty = match found {
                Some(id) => id,
                None => {
                    let name = widened_name(&t.name, t.width, *width);
                    let id = model.duplicate_opening_type(t.id, name)?;
                    if let Err(e) = model.edit_family_type(id, t.family, values) {
                        let _ = model.remove_opening_type(id);
                        return Err(e);
                    }
                    id
                }
            };
            model.set_opening_type(*opening, ty)
        }
        Change::Handing {
            opening,
            opens_to,
            hand,
        } => model.set_opening_handing(*opening, *opens_to, *hand),
    }
}

/// A type's name with its new width: «Puerta 80×205» → «Puerta
/// 90×205»; otherwise the width after it.
fn widened_name(name: &str, old: f64, new: f64) -> String {
    let (o, n) = (
        format!("{:.0}×", old * 100.0),
        format!("{:.0}×", new * 100.0),
    );
    if name.contains(&o) {
        name.replacen(&o, &n, 1)
    } else {
        format!("{} {:.0}", base_name(name), new * 100.0)
    }
}

/// «Puerta 82» → «Puerta»: a type's name without a trailing size.
fn base_name(name: &str) -> &str {
    match name.rsplit_once(' ') {
        Some((head, tail))
            if !head.is_empty() && tail.chars().all(|c| c.is_ascii_digit() || c == ',') =>
        {
            head
        }
        _ => name,
    }
}

fn other(side: WallSide) -> WallSide {
    match side {
        WallSide::Interior => WallSide::Exterior,
        WallSide::Exterior => WallSide::Interior,
    }
}

/// The hand that keeps the hinges on the same jamb when the leaf moves to
/// the other side (the same rule as the plan).
fn same_jamb(side: WallSide, hand: Hand) -> Hand {
    let at_start = (side == WallSide::Interior) == (hand == Hand::Left);
    if (other(side) == WallSide::Interior) == at_start {
        Hand::Left
    } else {
        Hand::Right
    }
}

/// The clear width of a door of a type (what the evacuation counts).
fn clear_of(model: &Model, id: OpeningTypeId) -> Option<(f64, bool)> {
    let t = model.opening_type(id)?;
    let c = model
        .opening_clear_width(id)
        .unwrap_or(t.width - 2.0 * t.frame_width);
    let double = matches!(
        t.kind,
        archisim_core::OpeningKind::Door(archisim_core::DoorOperation::DoubleSwing)
    );
    Some((c, double))
}

/// The opening width that gives at least a clear width (or a leaf), in
/// steps of 5 cm.
fn door_width_for(model: &Model, opening: OpeningId, clear: f64, leaf: bool) -> Option<f64> {
    let o = model.opening(opening)?;
    let t = model.opening_type(o.opening_type)?;
    let (now, double) = clear_of(model, t.id)?;
    let need = if leaf && double { 2.0 * clear } else { clear };
    if now + 1e-6 >= need {
        return None;
    }
    let w = t.width + need - now;
    Some((w / 0.05 - 1e-6).ceil() * 0.05)
}

fn struct_failing(r: &Report) -> bool {
    r.items.iter().any(|i| {
        i.status == Status::Fails && matches!(i.section, Section::Structure | Section::Framing)
    })
}

/// Works out the fix of a model's code check (it is not changed).
pub fn fix(model: &Model) -> Fix {
    let original = model.clone();
    let mut m = model.clone();
    let mut changes: Vec<Change> = Vec::new();
    let first = check(&m);
    let fails_before = fails(&first);
    let push = |m: &mut Model, c: Change, changes: &mut Vec<Change>| {
        if apply(m, &c).is_ok() {
            changes.push(c);
        }
    };
    // The structure.
    if struct_failing(&first) {
        let s = archisim_struct::fix::fix(&m);
        for c in s.changes {
            push(&mut m, Change::Structure(c), &mut changes);
        }
    }
    let report = check(&m);
    // The drawn reinforcement that breaks a rule: what the calculation
    // proposes.
    let broken: Vec<u64> = report
        .items
        .iter()
        .filter(|i| i.status == Status::Fails)
        .filter_map(|i| match i.subject {
            Subject::Reinforced(id) => Some(id),
            _ => None,
        })
        .collect();
    if !broken.is_empty() {
        let built = archisim_struct::phases::as_built(&m).into_owned();
        let an = archisim_struct::analysis::analyse(&built);
        for p in archisim_struct::proposals::proposals(&built, &an) {
            if broken.contains(&p.element) && p.now != Some(p.reinforcement) {
                push(
                    &mut m,
                    Change::Structure(archisim_struct::fix::Change::Reinforce {
                        element: p.element,
                        reinforcement: p.reinforcement,
                    }),
                    &mut changes,
                );
            }
        }
    }
    // Pipes and doors.
    let mut widths: BTreeMap<RunId, f64> = BTreeMap::new();
    for i in report.items.iter().filter(|i| i.status == Status::Fails) {
        match (i.subject, i.rule) {
            (Subject::Object(o), "HS 4, tabla 4.2" | "HS 5, tabla 4.1") => {
                let Some(need) = metres(&i.required).map(|mm| mm / 1000.0) else {
                    continue;
                };
                let Some(obj) = m.objects().find(|x| x.id == o) else {
                    continue;
                };
                let outline = m.object_outline(o);
                let systems: Vec<ServiceSystem> = if i.rule.starts_with("HS 5") {
                    vec![ServiceSystem::Drainage]
                } else if i.text.contains("caliente") {
                    vec![ServiceSystem::HotWater]
                } else {
                    vec![ServiceSystem::ColdWater]
                };
                if let Some((r, _)) = services::nearest(&m, obj.level, &systems, &outline) {
                    let w = widths.entry(r.id).or_insert(r.width);
                    *w = w.max(need);
                }
            }
            (Subject::Run(run), rule) if rule.starts_with("HS 5, 3.3.1") => {
                let Some(r) = m.runs().find(|r| r.id == run) else {
                    continue;
                };
                let slope = match rule {
                    "HS 5, 3.3.1.1" => r.slope.clamp(0.02, 0.05),
                    "HS 5, 3.3.1.5" => 0.02,
                    _ => 0.01,
                };
                push(&mut m, Change::RunSlope { run, slope }, &mut changes);
            }
            _ => {}
        }
    }
    // The diameters of the water calculation, where a stretch is short.
    if report.items.iter().any(|i| {
        i.status == Status::Fails && i.rule == "HS 4, 4.2" && matches!(i.subject, Subject::Run(_))
    }) {
        let sized = archisim_mep::water::water(&m).sized_runs();
        for i in report.items.iter().filter(|i| i.status == Status::Fails) {
            if let (Subject::Run(r), "HS 4, 4.2") = (i.subject, i.rule) {
                if let (Some(w), Some(now)) =
                    (sized.get(&r), m.runs().find(|x| x.id == r).map(|x| x.width))
                {
                    let e = widths.entry(r).or_insert(now);
                    *e = e.max(*w);
                }
            }
        }
    }
    // The diameters of the drainage calculation (HS 5).
    let drains_short = |i: &&crate::Item| {
        i.status == Status::Fails
            && (i.rule.starts_with("HS 5, tabla 4.") || i.rule.starts_with("HS 5, 4.3"))
            && i.rule != "HS 5, tabla 4.1"
            && i.rule != "HS 5, tabla 4.6"
            && i.rule != "HS 5, tabla 4.7"
            || i.status == Status::Fails
                && i.rule == "HS 5, tabla 4.1"
                && matches!(i.subject, Subject::Run(_))
    };
    if report.items.iter().any(|i| drains_short(&i)) {
        let (runs, pipes) = archisim_mep::drain::drainage(&m).sized_runs();
        for i in report.items.iter().filter(drains_short) {
            match i.subject {
                Subject::Run(r) => {
                    if let (Some(w), Some(now)) =
                        (runs.get(&r), m.runs().find(|x| x.id == r).map(|x| x.width))
                    {
                        let e = widths.entry(r).or_insert(now);
                        *e = e.max(*w);
                    }
                }
                Subject::Device(dv) => {
                    if let Some(w) = pipes.get(&dv) {
                        let now = m.devices().find(|x| x.id == dv).map_or(0.0, |x| x.width);
                        if *w > now + 1e-6 {
                            push(
                                &mut m,
                                Change::DeviceWidth {
                                    device: dv,
                                    width: *w,
                                },
                                &mut changes,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }
    for (run, width) in widths {
        push(&mut m, Change::RunWidth { run, width }, &mut changes);
    }
    // Doors: the widest clear width each one is asked for, round by round
    // (a door wide enough for its leaf may then be short once the leaf's
    // thickness is taken off).
    let mut report = report;
    for round in 0..3 {
        if round > 0 {
            report = check(&m);
        }
        let before = changes.len();
        let mut doors: BTreeMap<OpeningId, (f64, bool)> = BTreeMap::new();
        for i in report.items.iter().filter(|i| i.status == Status::Fails) {
            let Subject::Opening(o) = i.subject else {
                continue;
            };
            let want = match i.rule {
                "SI 3, tabla 4.1 (puertas y pasos)" => metres(&i.required).map(|c| (c, false)),
                "SUA, Anejo A (puertas)" if i.text.starts_with("Con la hoja abierta") => {
                    metres(&i.required).map(|c| (c + crate::evacuation::LEAF_THICKNESS, true))
                }
                "SUA, Anejo A (puertas)" => metres(&i.required).map(|c| (c, true)),
                "SI 3, 6.3" => {
                    if let Some(op) = m.opening(o) {
                        let (side, hand) = (op.opens_to, op.hand);
                        let c = Change::Handing {
                            opening: o,
                            opens_to: other(side),
                            hand: same_jamb(side, hand),
                        };
                        push(&mut m, c, &mut changes);
                    }
                    None
                }
                _ => None,
            };
            if let Some((c, leaf)) = want {
                let e = doors.entry(o).or_insert((c, leaf));
                if c > e.0 {
                    *e = (c, leaf);
                }
            }
        }
        for (o, (c, leaf)) in doors {
            if let Some(width) = door_width_for(&m, o, c, leaf) {
                push(
                    &mut m,
                    Change::DoorWidth { opening: o, width },
                    &mut changes,
                );
            }
        }
        if changes.len() == before {
            break;
        }
    }
    let after = check(&m);
    let fails_after = fails(&after);
    let left = after
        .items
        .iter()
        .filter(|i| i.status == Status::Fails)
        .map(|i| left_line(&m, i))
        .collect();
    Fix {
        lines: lines(&original, &m, &changes),
        changes,
        fails_before,
        fails_after,
        left,
    }
}

fn left_line(m: &Model, i: &Item) -> String {
    let who = match i.subject {
        Subject::Building => String::new(),
        s => format!("{} · ", crate::subject_name(m, s)),
    };
    let fig = if i.required.is_empty() {
        String::new()
    } else {
        format!(" ({} ; exigido {})", i.value, i.required)
    };
    format!("{who}{}{fig} [{}]", i.text, i.rule)
}

fn lines(before: &Model, after: &Model, changes: &[Change]) -> Vec<Line> {
    let mut out: Vec<Line> = Vec::new();
    let mut structure = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for c in changes {
        match c {
            Change::Structure(s) => structure.push(s.clone()),
            Change::RunWidth { run, .. } | Change::RunSlope { run, .. } => {
                let slope = matches!(c, Change::RunSlope { .. });
                if !seen.insert((run.0, slope as u8)) {
                    continue;
                }
                let v = |m: &Model| {
                    m.runs().find(|r| r.id == *run).map_or(String::new(), |r| {
                        if slope {
                            crate::pct(r.slope)
                        } else {
                            format!("Ø{:.0} mm", r.width * 1000.0)
                        }
                    })
                };
                out.push(Line {
                    subject: Subject::Run(*run),
                    name: crate::subject_name(before, Subject::Run(*run)),
                    what: if slope { "Pendiente" } else { "Diámetro" },
                    before: v(before),
                    after: v(after),
                });
            }
            Change::DeviceWidth { device, .. } => {
                if !seen.insert((device.0, 4)) {
                    continue;
                }
                let v = |m: &Model| {
                    m.devices()
                        .find(|d| d.id == *device)
                        .map_or(String::new(), |d| format!("Ø{:.0} mm", d.width * 1000.0))
                };
                out.push(Line {
                    subject: Subject::Device(*device),
                    name: crate::subject_name(before, Subject::Device(*device)),
                    what: "Diámetro",
                    before: v(before),
                    after: v(after),
                });
            }
            Change::DoorWidth { opening, .. } | Change::Handing { opening, .. } => {
                let handing = matches!(c, Change::Handing { .. });
                if !seen.insert((opening.0, 2 + handing as u8)) {
                    continue;
                }
                let v = |m: &Model| {
                    let Some(o) = m.opening(*opening) else {
                        return String::new();
                    };
                    if handing {
                        match o.opens_to {
                            WallSide::Interior => "abre hacia la cara interior".into(),
                            WallSide::Exterior => "abre hacia la cara exterior".into(),
                        }
                    } else {
                        m.opening_type(o.opening_type).map_or(String::new(), |t| {
                            let c = clear_of(m, t.id).map_or(0.0, |x| x.0);
                            format!("{} (paso libre {})", t.name, crate::m(c))
                        })
                    }
                };
                out.push(Line {
                    subject: Subject::Opening(*opening),
                    name: crate::subject_name(before, Subject::Opening(*opening)),
                    what: if handing { "Sentido" } else { "Tipo" },
                    before: v(before),
                    after: v(after),
                });
            }
        }
    }
    if !structure.is_empty() {
        let mut s: Vec<Line> = archisim_struct::fix::lines_of(before, after, &structure)
            .into_iter()
            .map(|l| Line {
                subject: Subject::Reinforced(l.element),
                name: l.name,
                what: l.what,
                before: l.before,
                after: l.after,
            })
            .collect();
        s.append(&mut out);
        out = s;
    }
    out.retain(|l| l.before != l.after);
    out
}

/// Makes the fix on the model.
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
    fn lengths_in_the_text() {
        assert_eq!(metres("≥ 0,80 m"), Some(0.8));
        assert_eq!(metres("≥ 40 mm"), Some(40.0));
        assert_eq!(metres(""), None);
        assert_eq!(base_name("Puerta de paso 72"), "Puerta de paso");
        assert_eq!(base_name("P1"), "P1");
        assert_eq!(
            widened_name("Puerta de paso 1 hoja 80×205", 0.8, 0.9),
            "Puerta de paso 1 hoja 90×205"
        );
        assert_eq!(widened_name("Puerta 82", 0.82, 0.9), "Puerta 90");
    }
}
