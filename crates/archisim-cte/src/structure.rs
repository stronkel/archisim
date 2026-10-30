//! The reinforcement against the Código Estructural (E11, decision D26):
//! what can be checked without calculating the structure — the cover,
//! the diameters, the spacing of the bars and the minimum geometric
//! amounts of steel. Whether the bars are enough for the loads is for the
//! structural program.
//!
//! The values used (to be confirmed by the designer for each project):
//!
//! - cover: 25 mm inside (exposure XC1), 50 mm in footings (on a blinding
//!   layer; 70 mm when the concrete is poured against the ground);
//! - columns: bars of 12 mm or more, four at least (six in a round
//!   column), 35 cm at most between them; steel between 4 ‰ and 4 % of
//!   the section; ties of a quarter of the largest bar and 6 mm at least,
//!   no further apart than fifteen times the smallest bar, the smaller
//!   side of the section or 30 cm;
//! - beams (B500S): 2,8 ‰ at the bottom and 30 % of that at the top;
//!   stirrups of 6 mm or more, no further apart than 30 cm nor three
//!   quarters of the useful depth; 20 mm or a bar between bars;
//! - slabs (B500S): 1,8 ‰ in each direction, top and bottom together;
//!   bars no further apart than 25 cm nor twice the thickness;
//! - footings: bars of 12 mm or more, 30 cm at most apart, 0,9 ‰ in each
//!   direction.

use archisim_core::{Counted, Model, Profile, Reinforcement, Spaced};

use crate::{Item, Report, Section, Status, Subject};

/// The name of an element with reinforcement: «Pilar 12 · P 30×30».
pub fn element_name(model: &Model, id: u64) -> String {
    use archisim_core::{BeamId, ColumnId, FootingId, SlabId};
    if let Some(c) = model.column(ColumnId(id)) {
        let t = model
            .member_type(c.member_type)
            .map_or("", |t| t.name.as_str());
        return format!("Pilar {id} · {t}");
    }
    if let Some(b) = model.beam(BeamId(id)) {
        let t = model
            .member_type(b.member_type)
            .map_or("", |t| t.name.as_str());
        return format!("Viga {id} · {t}");
    }
    if let Some(f) = model.footing(FootingId(id)) {
        return format!("{} {id}", f.kind_name());
    }
    if let Some(s) = model.slab(SlabId(id)) {
        let t = model.slab_type(s.slab_type).map_or("", |t| t.name.as_str());
        return format!("Forjado {id} · {t}");
    }
    format!("Elemento {id}")
}

fn mm(v: f64) -> String {
    format!("{} mm", (v * 1000.0).round() as i64)
}

fn cm(v: f64) -> String {
    format!("{:.1} cm", v * 100.0).replace('.', ",")
}

fn permil(v: f64) -> String {
    format!("{:.2} ‰", v * 1000.0).replace('.', ",")
}

struct Checker<'a> {
    report: &'a mut Report,
    subject: Subject,
    level: Option<archisim_core::LevelId>,
}

impl Checker<'_> {
    fn item(
        &mut self,
        ok: bool,
        rule: &'static str,
        text: String,
        value: String,
        required: String,
    ) {
        self.report.items.push(Item {
            section: Section::Structure,
            status: if ok { Status::Passes } else { Status::Fails },
            subject: self.subject,
            level: self.level,
            rule,
            text,
            value,
            required,
            route: Vec::new(),
        });
    }

    fn at_least(
        &mut self,
        rule: &'static str,
        what: &str,
        v: f64,
        min: f64,
        fmt: fn(f64) -> String,
    ) {
        self.item(
            v >= min - 1e-9,
            rule,
            format!("{what}: {}", fmt(v)),
            fmt(v),
            format!("≥ {}", fmt(min)),
        );
    }

    fn at_most(
        &mut self,
        rule: &'static str,
        what: &str,
        v: f64,
        max: f64,
        fmt: fn(f64) -> String,
    ) {
        self.item(
            v <= max + 1e-9,
            rule,
            format!("{what}: {}", fmt(v)),
            fmt(v),
            format!("≤ {}", fmt(max)),
        );
    }
}

/// Every element with reinforcement, checked.
pub fn check(model: &Model, report: &mut Report) {
    let items: Vec<(u64, Reinforcement)> = model.reinforcements().map(|(id, r)| (id, *r)).collect();
    for (id, r) in items {
        let level = level_of(model, id);
        let mut c = Checker {
            report,
            subject: Subject::Reinforced(id),
            level,
        };
        match r {
            Reinforcement::Column {
                cover, bars, ties, ..
            } => column(model, id, &mut c, cover, bars, ties),
            Reinforcement::Beam {
                cover,
                bottom,
                top,
                stirrups,
            } => beam(model, id, &mut c, cover, bottom, top, stirrups),
            Reinforcement::Footing { cover, bottom, top } => {
                footing(model, id, &mut c, cover, bottom, top)
            }
            Reinforcement::Slab { cover, bottom, top } => {
                slab(model, id, &mut c, cover, bottom, top)
            }
        }
    }
}

fn level_of(model: &Model, id: u64) -> Option<archisim_core::LevelId> {
    use archisim_core::{BeamId, ColumnId, FootingId, SlabId};
    model
        .column(ColumnId(id))
        .map(|c| c.level)
        .or_else(|| model.beam(BeamId(id)).map(|b| b.level))
        .or_else(|| model.footing(FootingId(id)).map(|f| f.level))
        .or_else(|| model.slab(SlabId(id)).map(|s| s.level))
}

const COVER_INSIDE: f64 = 0.025;
const COVER_FOOTING: f64 = 0.05;

fn column(model: &Model, id: u64, c: &mut Checker, cover: f64, bars: Counted, ties: Spaced) {
    let Some(col) = model.column(archisim_core::ColumnId(id)) else {
        return;
    };
    let Some(t) = model.member_type(col.member_type) else {
        return;
    };
    c.at_least(
        "Recubrimiento (XC1)",
        "Recubrimiento nominal",
        cover,
        COVER_INSIDE,
        mm,
    );
    c.at_least(
        "Pilares: diámetro mínimo",
        "Diámetro de las barras longitudinales",
        bars.diameter,
        0.012,
        mm,
    );
    let (area, perimeter, least, round) = match t.profile {
        Profile::Rectangle { width, depth } => (
            width * depth,
            2.0 * (width + depth - 4.0 * (cover + ties.diameter)),
            width.min(depth),
            false,
        ),
        Profile::Circle { diameter } => (
            std::f64::consts::PI * diameter * diameter / 4.0,
            std::f64::consts::PI * (diameter - 2.0 * (cover + ties.diameter)),
            diameter,
            true,
        ),
        _ => return,
    };
    let min_bars = if round { 6 } else { 4 };
    c.item(
        bars.count >= min_bars,
        "Pilares: número mínimo de barras",
        format!("{} barras longitudinales", bars.count),
        bars.count.to_string(),
        format!("≥ {min_bars}"),
    );
    let gap = perimeter / bars.count.max(1) as f64;
    c.at_most(
        "Pilares: separación de barras",
        "Separación entre barras longitudinales",
        gap,
        0.35,
        cm,
    );
    let rho = bars.area() / area;
    c.at_least(
        "Cuantía geométrica mínima",
        "Cuantía de acero longitudinal",
        rho,
        0.004,
        permil,
    );
    c.item(
        rho <= 0.04,
        "Cuantía máxima",
        format!("Cuantía de acero longitudinal: {}", permil(rho)),
        permil(rho),
        "≤ 40,00 ‰".into(),
    );
    c.at_least(
        "Cercos: diámetro",
        "Diámetro de los cercos",
        ties.diameter,
        (bars.diameter / 4.0).max(0.006),
        mm,
    );
    let max_s = (15.0 * bars.diameter).min(least).min(0.30);
    c.at_most(
        "Cercos: separación",
        "Separación de los cercos",
        ties.spacing,
        max_s,
        cm,
    );
}

#[allow(clippy::too_many_arguments)]
fn beam(
    model: &Model,
    id: u64,
    c: &mut Checker,
    cover: f64,
    bottom: Counted,
    top: Counted,
    stirrups: Spaced,
) {
    let Some(b) = model.beam(archisim_core::BeamId(id)) else {
        return;
    };
    let Some(t) = model.member_type(b.member_type) else {
        return;
    };
    let (w, d) = t.profile.size();
    let area = w * d;
    c.at_least(
        "Recubrimiento (XC1)",
        "Recubrimiento nominal",
        cover,
        COVER_INSIDE,
        mm,
    );
    let need = 0.0028 * area;
    c.at_least(
        "Cuantía geométrica mínima (B500S)",
        "Cuantía de la armadura inferior",
        bottom.area() / area,
        0.0028,
        permil,
    );
    c.at_least(
        "Cuantía geométrica mínima, cara opuesta",
        "Cuantía de la armadura superior",
        top.area() / area,
        0.3 * need / area,
        permil,
    );
    for (row, name) in [(bottom, "inferiores"), (top, "superiores")] {
        if row.count >= 2 {
            let inner = w - 2.0 * (cover + stirrups.diameter) - row.diameter;
            let clear = inner / (row.count - 1) as f64 - row.diameter;
            c.at_least(
                "Separación libre entre barras",
                &format!("Separación libre entre barras {name}"),
                clear,
                0.02f64.max(row.diameter),
                mm,
            );
        }
    }
    c.at_least(
        "Estribos: diámetro",
        "Diámetro de los estribos",
        stirrups.diameter,
        0.006,
        mm,
    );
    let useful = d - cover - stirrups.diameter - bottom.diameter / 2.0;
    c.at_most(
        "Estribos: separación",
        "Separación de los estribos",
        stirrups.spacing,
        (0.75 * useful).min(0.30),
        cm,
    );
}

fn mesh_amount(m: &Spaced) -> f64 {
    archisim_core::bar_area(m.diameter) / m.spacing
}

fn footing(
    model: &Model,
    id: u64,
    c: &mut Checker,
    cover: f64,
    bottom: [Spaced; 2],
    top: Option<[Spaced; 2]>,
) {
    let Some(f) = model.footing(archisim_core::FootingId(id)) else {
        return;
    };
    c.at_least(
        "Recubrimiento (contra el terreno, con hormigón de limpieza)",
        "Recubrimiento nominal",
        cover,
        COVER_FOOTING,
        mm,
    );
    for (k, m) in bottom.iter().enumerate() {
        let dir = if k == 0 { "x" } else { "y" };
        c.at_least(
            "Zapatas: diámetro mínimo",
            &format!("Diámetro de las barras en {dir}"),
            m.diameter,
            0.012,
            mm,
        );
        c.at_most(
            "Zapatas: separación máxima",
            &format!("Separación de las barras en {dir}"),
            m.spacing,
            0.30,
            cm,
        );
        let amount = mesh_amount(m) + top.map_or(0.0, |t| mesh_amount(&t[k]));
        c.at_least(
            "Cuantía geométrica mínima",
            &format!("Cuantía en {dir}"),
            amount / f.depth,
            0.0009,
            permil,
        );
    }
}

fn slab(
    model: &Model,
    id: u64,
    c: &mut Checker,
    cover: f64,
    bottom: [Spaced; 2],
    top: Option<[Spaced; 2]>,
) {
    let Some(s) = model.slab(archisim_core::SlabId(id)) else {
        return;
    };
    let Some(t) = model.slab_type(s.slab_type) else {
        return;
    };
    // The structural layer (the floor finishes do not count).
    let h = t
        .layers
        .iter()
        .find(|l| l.function == archisim_core::LayerFunction::Structure)
        .map_or(t.thickness(), |l| l.thickness);
    c.at_least(
        "Recubrimiento (XC1)",
        "Recubrimiento nominal",
        cover,
        COVER_INSIDE,
        mm,
    );
    for k in 0..2 {
        let dir = if k == 0 { "x" } else { "y" };
        let amount = mesh_amount(&bottom[k]) + top.map_or(0.0, |t| mesh_amount(&t[k]));
        c.at_least(
            "Cuantía geométrica mínima (B500S)",
            &format!("Cuantía en {dir}, las dos caras"),
            amount / h,
            0.0018,
            permil,
        );
        for (m, face) in
            std::iter::once((bottom[k], "inferior")).chain(top.map(|t| (t[k], "superior")))
        {
            c.at_most(
                "Losas: separación máxima",
                &format!("Separación {face} en {dir}"),
                m.spacing,
                (2.0 * h).min(0.25),
                cm,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{NewColumn, Point2, RebarKind, WallTop};

    #[test]
    fn a_column_is_checked() {
        let mut m = archisim_core::Model::new("x");
        let level = m.add_level("PB", 0.0);
        let t = archisim_library::add_library(&mut m).unwrap().column;
        let col = m
            .add_column(NewColumn {
                member_type: t,
                level,
                position: Point2::new(0.0, 0.0),
                rotation: 0.0,
                base_offset: 0.0,
                top: WallTop::Unconnected,
                height: 3.0,
            })
            .unwrap();
        let mut r = Reinforcement::default_for(RebarKind::Column);
        m.set_reinforcement(col.0, Some(r)).unwrap();
        let mut report = Report::default();
        check(&m, &mut report);
        assert!(!report.items.is_empty());
        assert!(
            report.items.iter().all(|i| i.status == Status::Passes),
            "{:#?}",
            report.items
        );
        // Ties too far apart.
        if let Reinforcement::Column { ties, .. } = &mut r {
            ties.spacing = 0.40;
        }
        m.set_reinforcement(col.0, Some(r)).unwrap();
        let mut report = Report::default();
        check(&m, &mut report);
        assert!(report
            .items
            .iter()
            .any(|i| i.rule == "Cercos: separación" && i.status == Status::Fails));
    }
}
