//! The fire resistance of the concrete structure (DB-SI 6 and its annex
//! C, by tables).
//!
//! What is asked (table 3.1): by the building's use and its evacuation
//! height (the highest level above the ground), and R 120 in basements;
//! a single-family house, R 30. What each element has, by annex C:
//! - columns (table C.2): the smaller side and the equivalent axis
//!   distance of their bars, am;
//! - beams exposed on three faces (table C.3): their width and am, any of
//!   its options;
//! - solid slabs (table C.4): their depth and am, one or two ways;
//! - one-way floors (C.2.3.5): am of the slabs one way, the blocks and the
//!   ceiling counting as concrete up to R 120;
//! - waffle floors (table C.5): the width of the ribs, am and the depth.
//!
//! am is the cover of the bars to their axis (cover, tie and half the
//! bar); without the reinforcement drawn, the one proposed.

use archisim_core::{BuildingUse, FloorKind, Joist, Model, Profile, Reinforcement};

use crate::analysis::Analysis;
use crate::analytical::Owner;
use crate::lightframe::{Check, Status, Subject};
use crate::sections::Kind;
use crate::text::dec;

/// The resistance asked of the structure, minutes, above the ground and
/// in a basement (DB-SI 6, table 3.1).
pub fn required(model: &Model) -> (u32, u32) {
    if model.structural_site().single_house {
        return (30, 30);
    }
    // The evacuation height: the highest level above the ground with
    // something on it (the roof's level does not count).
    let mut tops: Vec<f64> = model
        .levels()
        .iter()
        .filter(|l| model.slabs_on_level(l.id).next().is_some())
        .map(|l| l.elevation)
        .collect();
    tops.sort_by(f64::total_cmp);
    let h = if tops.len() >= 2 {
        tops[tops.len() - 2].max(0.0)
    } else {
        0.0
    };
    let use_ = model.code_settings().building_use;
    let above = match use_ {
        BuildingUse::Parking => 90,
        BuildingUse::Commercial | BuildingUse::Assembly | BuildingUse::Hospital => {
            if h <= 15.0 {
                90
            } else if h <= 28.0 {
                120
            } else {
                180
            }
        }
        _ => {
            if h <= 15.0 {
                60
            } else if h <= 28.0 {
                90
            } else {
                120
            }
        }
    };
    (above, 120)
}

/// The rows of a table: (minutes, values…).
const COLUMNS: [(u32, f64, f64); 6] = [
    (30, 150.0, 15.0),
    (60, 200.0, 20.0),
    (90, 250.0, 30.0),
    (120, 250.0, 40.0),
    (180, 350.0, 45.0),
    (240, 400.0, 50.0),
];

const BEAMS: [(u32, [(f64, f64); 4]); 6] = [
    (
        30,
        [(80.0, 20.0), (120.0, 15.0), (200.0, 10.0), (f64::NAN, 0.0)],
    ),
    (
        60,
        [(100.0, 30.0), (150.0, 25.0), (200.0, 20.0), (f64::NAN, 0.0)],
    ),
    (
        90,
        [(150.0, 40.0), (200.0, 35.0), (250.0, 30.0), (400.0, 25.0)],
    ),
    (
        120,
        [(200.0, 50.0), (250.0, 45.0), (300.0, 40.0), (500.0, 35.0)],
    ),
    (
        180,
        [(300.0, 75.0), (350.0, 65.0), (400.0, 60.0), (600.0, 50.0)],
    ),
    (
        240,
        [(400.0, 75.0), (500.0, 70.0), (700.0, 60.0), (f64::NAN, 0.0)],
    ),
];

/// Solid slabs: hmin, am one way, am two ways (ly/lx ≤ 1,5).
const SLABS: [(u32, f64, f64, f64); 6] = [
    (30, 60.0, 10.0, 10.0),
    (60, 80.0, 20.0, 10.0),
    (90, 100.0, 25.0, 15.0),
    (120, 120.0, 35.0, 20.0),
    (180, 150.0, 50.0, 30.0),
    (240, 175.0, 60.0, 50.0),
];

/// Rib widths with their axis distances, in mm.
type RibOptions = [(f64, f64); 3];

const WAFFLES: [(u32, RibOptions, f64); 6] = [
    (30, [(80.0, 20.0), (120.0, 15.0), (200.0, 10.0)], 60.0),
    (60, [(100.0, 30.0), (150.0, 25.0), (200.0, 20.0)], 80.0),
    (90, [(120.0, 40.0), (200.0, 30.0), (250.0, 25.0)], 100.0),
    (120, [(160.0, 50.0), (250.0, 40.0), (300.0, 35.0)], 120.0),
    (180, [(200.0, 70.0), (300.0, 60.0), (400.0, 55.0)], 150.0),
    (240, [(250.0, 90.0), (350.0, 75.0), (500.0, 70.0)], 175.0),
];

/// The row for `r` minutes or the next one up.
fn row<T: Copy>(rows: &[(u32, T)], r: u32) -> Option<T> {
    rows.iter().find(|x| x.0 >= r).map(|x| x.1)
}

/// The highest resistance a column `b` wide with `am` reaches.
pub fn column_resistance(b: f64, am: f64) -> u32 {
    COLUMNS
        .iter()
        .rev()
        .find(|(_, bmin, amin)| b * 1000.0 >= *bmin - 1e-6 && am * 1000.0 >= *amin - 1e-6)
        .map_or(0, |x| x.0)
}

/// The highest resistance a beam `b` wide with `am` reaches.
pub fn beam_resistance(b: f64, am: f64) -> u32 {
    BEAMS
        .iter()
        .rev()
        .find(|(_, opts)| {
            opts.iter().any(|(bmin, amin)| {
                bmin.is_finite() && b * 1000.0 >= *bmin - 1e-6 && am * 1000.0 >= *amin - 1e-6
            })
        })
        .map_or(0, |x| x.0)
}

/// The highest resistance of a solid slab `h` deep with `am`.
pub fn slab_resistance(h: f64, am: f64, two_way: bool) -> u32 {
    SLABS
        .iter()
        .rev()
        .find(|(_, hmin, one, two)| {
            h * 1000.0 >= *hmin - 1e-6 && am * 1000.0 >= if two_way { *two } else { *one } - 1e-6
        })
        .map_or(0, |x| x.0)
}

/// The highest resistance of a waffle floor with ribs `b` wide, `am` and
/// `h` deep.
pub fn waffle_resistance(b: f64, am: f64, h: f64) -> u32 {
    WAFFLES
        .iter()
        .rev()
        .find(|(_, opts, hmin)| {
            h * 1000.0 >= *hmin - 1e-6
                && opts
                    .iter()
                    .any(|(bmin, amin)| b * 1000.0 >= *bmin - 1e-6 && am * 1000.0 >= *amin - 1e-6)
        })
        .map_or(0, |x| x.0)
}

fn verdict(
    subject: Subject,
    what: String,
    rule: &'static str,
    has: u32,
    need: u32,
    advice: String,
) -> Check {
    let ok = has >= need;
    Check {
        subject,
        what: format!(
            "{what}: R {} (pide R {need})",
            if has == 0 {
                "menos de 30".to_string()
            } else {
                has.to_string()
            }
        ),
        rule,
        ratio: if ok {
            0.0
        } else {
            f64::from(need) / f64::from(has.max(15))
        },
        status: if ok { Status::Pass } else { Status::Fail },
        advice: (!ok).then_some(advice),
    }
}

pub fn check(model: &Model, an: &Analysis) -> Vec<Check> {
    let (above, basement) = required(model);
    let proposals = crate::proposals::proposals(model, an);
    let reinforcement = |id: u64| -> Option<Reinforcement> {
        model.reinforcement(id).copied().or_else(|| {
            proposals
                .iter()
                .find(|p| p.element == id)
                .map(|p| p.reinforcement)
        })
    };
    let need_at = |level: archisim_core::LevelId| {
        if model.level(level).is_some_and(|l| l.elevation < -0.5) {
            basement
        } else {
            above
        }
    };
    let mut out = Vec::new();
    for (owner, kind) in &an.model.kinds {
        if *kind != Kind::Concrete {
            continue;
        }
        match owner {
            Owner::Column(id) => {
                let Some(c) = model.column(*id) else { continue };
                let Some(t) = model.member_type(c.member_type) else {
                    continue;
                };
                let b = match t.profile {
                    Profile::Rectangle { width, depth } => width.min(depth),
                    Profile::Circle { diameter } => diameter,
                    _ => continue,
                };
                let am = match reinforcement(id.0) {
                    Some(Reinforcement::Column {
                        cover, bars, ties, ..
                    }) => cover + ties.diameter + bars.diameter / 2.0,
                    _ => 0.03 + 0.006 + 0.008,
                };
                let need = need_at(c.level);
                out.push(verdict(
                    Subject::Column(*id),
                    format!(
                        "{}: lado menor {} mm, am {} mm",
                        t.name,
                        dec(b * 1000.0, 0),
                        dec(am * 1000.0, 0)
                    ),
                    "Resistencia al fuego de soportes (DB-SI, anejo C, tabla C.2)",
                    column_resistance(b, am),
                    need,
                    format!(
                        "lado de {} mm y am de {} mm (más recubrimiento), o un revestimiento",
                        dec(row(&COLUMNS.map(|x| (x.0, x.1)), need).unwrap_or(400.0), 0),
                        dec(row(&COLUMNS.map(|x| (x.0, x.2)), need).unwrap_or(50.0), 0)
                    ),
                ));
            }
            Owner::Beam(id) => {
                let Some(bm) = model.beam(*id) else { continue };
                let Some(t) = model.member_type(bm.member_type) else {
                    continue;
                };
                let Profile::Rectangle { width, .. } = t.profile else {
                    continue;
                };
                let am = match reinforcement(id.0) {
                    Some(Reinforcement::Beam {
                        cover,
                        bottom,
                        stirrups,
                        ..
                    }) => cover + stirrups.diameter + bottom.diameter / 2.0,
                    _ => 0.03 + 0.008 + 0.008,
                };
                let need = need_at(bm.level);
                out.push(verdict(
                    Subject::Beam(*id),
                    format!(
                        "{}: ancho {} mm, am {} mm",
                        t.name,
                        dec(width * 1000.0, 0),
                        dec(am * 1000.0, 0)
                    ),
                    "Resistencia al fuego de vigas (DB-SI, anejo C, tabla C.3)",
                    beam_resistance(width, am),
                    need,
                    "más ancho o más recubrimiento a la armadura inferior, o un revestimiento"
                        .into(),
                ));
            }
        }
    }
    // Floors.
    for s in model.slabs() {
        let Some(t) = model.slab_type(s.slab_type) else {
            continue;
        };
        if t.function != archisim_core::SlabFunction::Floor
            && t.function != archisim_core::SlabFunction::Roof
        {
            continue;
        }
        let concrete: f64 = t
            .layers
            .iter()
            .filter(|l| {
                l.function == archisim_core::LayerFunction::Structure
                    && Kind::of(model.material(l.material)) == Kind::Concrete
            })
            .map(|l| l.thickness)
            .sum();
        // A box's floor and roof are its steel chassis, checked with the box.
        if concrete < 0.05
            || model.type_framing(s.slab_type.0).is_some()
            || crate::gravity::in_box(model, s.level, &s.outline)
        {
            continue;
        }
        let need = need_at(s.level);
        let subject = Subject::Floor(s.id);
        match model.floor_system(s.slab_type) {
            Some(f) if f.kind == FloorKind::Waffle => {
                let am = 0.025 + 0.006;
                out.push(verdict(
                    subject,
                    format!(
                        "{} (reticular): nervios de {} mm, am {} mm, canto {} mm",
                        t.name,
                        dec(f.rib * 1000.0, 0),
                        dec(am * 1000.0, 0),
                        dec(f.depth * 1000.0, 0)
                    ),
                    "Resistencia al fuego de forjados bidireccionales (DB-SI, anejo C, tabla C.5)",
                    waffle_resistance(f.rib, am, f.depth),
                    need,
                    "nervios más anchos o más recubrimiento, o un falso techo resistente".into(),
                ));
            }
            Some(f) => {
                // One-way: C.2.3.5, as a solid slab one way; with
                // ceramic or concrete blocks and a ceiling below, enough
                // up to R 120.
                let blocks = matches!(
                    f.infill,
                    archisim_core::Infill::Concrete | archisim_core::Infill::Ceramic
                );
                let am = if f.joist == Joist::Precast {
                    0.030
                } else {
                    0.025 + 0.006
                };
                let has = if blocks && need <= 120 {
                    slab_resistance(f.depth, am, false).max(if am >= 0.035 { 120 } else { 0 })
                } else {
                    slab_resistance(f.depth, am, false)
                };
                let mut c = verdict(
                    subject,
                    format!(
                        "{} (unidireccional): am {} mm, canto {} mm",
                        t.name,
                        dec(am * 1000.0, 0),
                        dec(f.depth * 1000.0, 0)
                    ),
                    "Resistencia al fuego de forjados unidireccionales (DB-SI, anejo C, C.2.3.5)",
                    has,
                    need,
                    if f.joist == Joist::Precast {
                        "la resistencia de la vigueta la da su fabricante: pídele la de este canto; o un falso techo resistente".into()
                    } else {
                        "más recubrimiento a los nervios o un falso techo resistente".into()
                    },
                );
                if f.joist == Joist::Precast && c.status == Status::Fail {
                    c.status = Status::Warn;
                }
                out.push(c);
            }
            None => {
                let am = match model.reinforcement(s.id.0) {
                    Some(Reinforcement::Slab { cover, bottom, .. }) => {
                        cover + bottom[0].diameter.max(bottom[1].diameter) / 2.0
                    }
                    _ => 0.025 + 0.006,
                };
                out.push(verdict(
                    subject,
                    format!(
                        "{} (losa maciza): canto {} mm, am {} mm",
                        t.name,
                        dec(concrete * 1000.0, 0),
                        dec(am * 1000.0, 0)
                    ),
                    "Resistencia al fuego de losas macizas (DB-SI, anejo C, tabla C.4)",
                    slab_resistance(concrete, am, false),
                    need,
                    "más recubrimiento a la armadura inferior o más canto".into(),
                ));
            }
        }
    }
    // Steel members: protected by hand.
    for (owner, kind) in &an.model.kinds {
        if *kind != Kind::Steel {
            continue;
        }
        let subject = match owner {
            Owner::Column(id) => Subject::Column(*id),
            Owner::Beam(id) => Subject::Beam(*id),
        };
        out.push(Check {
            subject,
            what: format!(
                "Perfil de acero sin proteger: pide R {above} (o R {basement} en sótano)"
            ),
            rule: "Resistencia al fuego del acero (DB-SI, anejo D)",
            ratio: 0.0,
            status: Status::Warn,
            advice: Some("protegerlo (pintura intumescente, mortero proyectado o placas) con el espesor que da el fabricante para su masividad".into()),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tables() {
        // A 30 × 30 column with 30 mm of cover, Ø8 ties and Ø16 bars: am 46.
        assert_eq!(column_resistance(0.30, 0.046), 120);
        assert_eq!(column_resistance(0.25, 0.046), 120);
        assert_eq!(column_resistance(0.20, 0.025), 60);
        // A 300 mm beam with am 46: option 3 of R 120 (300/40).
        assert_eq!(beam_resistance(0.30, 0.046), 120);
        assert_eq!(beam_resistance(0.25, 0.046), 120);
        assert_eq!(beam_resistance(0.20, 0.030), 60);
        // A solid slab of 20 with am 30: REI 90 one way.
        assert_eq!(slab_resistance(0.20, 0.030, false), 90);
        assert_eq!(waffle_resistance(0.12, 0.031, 0.30), 60);
    }

    #[test]
    fn what_is_asked() {
        let mut m = Model::new("Fuego");
        m.add_level("Planta baja", 0.0);
        assert_eq!(required(&m), (60, 120));
        let mut site = *m.structural_site();
        site.single_house = true;
        m.set_structural_site(site).unwrap();
        assert_eq!(required(&m), (30, 30));
    }
}
