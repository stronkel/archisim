//! What the factory needs from a project of modular boxes (M16,
//! `docs/MODULAR.md`): each piece of the wall panels with its mark, its
//! panel and box, its length and what the roll former does to it (the
//! service holes of M6, the screw dimples where studs meet tracks, the
//! notches of the lips at the ends of the tracks), and the file for the
//! roll former in a plain, open format (CSV, one line a piece) that any
//! machine's software can map. Nothing is kept: all comes from the model.

use archisim_core::{BoxId, Model};

use crate::box_services::stud_holes;
use crate::framing::{wall_panels, Member, Role};

/// What the roll former does at a point of a piece.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operation {
    /// A service hole through the web, its diameter, m.
    Hole(f64),
    /// A dimple (pilot) for a screw through both flanges.
    Dimple,
    /// The lips cut so another piece slides in (at a track's end).
    LipCut,
}

impl Operation {
    pub fn code(self) -> String {
        match self {
            Operation::Hole(d) => format!("AGUJERO Ø{:.0}", d * 1000.0),
            Operation::Dimple => "TORNILLO".to_string(),
            Operation::LipCut => "CORTE DE LABIO".to_string(),
        }
    }
}

/// A piece for the factory.
#[derive(Debug, Clone, PartialEq)]
pub struct FactoryPiece {
    /// «MOD-01·M3-2·07»: box, panel, piece.
    pub mark: String,
    pub box_mark: String,
    pub panel: String,
    pub role: Role,
    pub name: String,
    pub section: String,
    /// m.
    pub length: f64,
    /// Along the piece from its start, m.
    pub operations: Vec<(f64, Operation)>,
}

/// Screws at each end of a stud, from the end, m.
pub const END_SCREW: f64 = 0.010;

fn vertical(m: &Member) -> bool {
    (m.end[2] - m.start[2]).abs() > 0.5 * m.length()
}

/// The pieces of the panels of a box's walls, in panel order.
pub fn box_pieces(model: &Model, id: BoxId) -> Vec<FactoryPiece> {
    let Some(b) = model.modular_box(id) else {
        return Vec::new();
    };
    let held = model.box_members(id);
    let holes = stud_holes(model);
    let mut out = Vec::new();
    for p in wall_panels(model)
        .into_iter()
        .filter(|p| held.walls.contains(&p.wall))
    {
        let base = model
            .wall(p.wall)
            .and_then(|w| model.level(w.level))
            .map_or(0.0, |l| l.elevation);
        let studs: Vec<f64> = p
            .members
            .iter()
            .filter(|m| vertical(m))
            .map(|m| m.along[0])
            .collect();
        for (k, m) in p.members.iter().enumerate() {
            let length = m.length();
            let mut ops: Vec<(f64, Operation)> = Vec::new();
            if vertical(m) {
                ops.push((END_SCREW, Operation::Dimple));
                ops.push((length - END_SCREW, Operation::Dimple));
                let z0 = m.start[2].min(m.end[2]) - base;
                for h in holes
                    .iter()
                    .filter(|h| h.wall == p.wall && (h.along - m.along[0]).abs() < 1e-6)
                {
                    ops.push((h.z - z0, Operation::Hole(h.diameter)));
                }
            } else {
                // Along a track or a noggin: a dimple where each stud
                // meets it, and the lips cut at its ends.
                let (a0, a1) = (m.along[0].min(m.along[1]), m.along[0].max(m.along[1]));
                if matches!(m.role, Role::BottomTrack | Role::TopTrack | Role::SillTrack) {
                    ops.push((0.0, Operation::LipCut));
                    ops.push((length, Operation::LipCut));
                }
                for s in &studs {
                    if *s >= a0 - 1e-6 && *s <= a1 + 1e-6 {
                        ops.push((*s - a0, Operation::Dimple));
                    }
                }
            }
            ops.sort_by(|a, b| a.0.total_cmp(&b.0));
            out.push(FactoryPiece {
                mark: format!("{}·{}·{:02}", b.mark, p.mark, k + 1),
                box_mark: b.mark.clone(),
                panel: p.mark.clone(),
                role: m.role,
                name: m.role.name(m.material).to_string(),
                section: m.section.name.clone(),
                length,
                operations: ops,
            });
        }
    }
    out
}

/// The pieces of every box, in the boxes' order by mark.
pub fn all_pieces(model: &Model) -> Vec<FactoryPiece> {
    let mut boxes: Vec<_> = model.boxes().collect();
    boxes.sort_by(|a, b| a.mark.cmp(&b.mark));
    boxes.iter().flat_map(|b| box_pieces(model, b.id)).collect()
}

/// The file for the roll former: one line a piece, `;` between fields,
/// lengths and positions in millimetres; the operations as
/// «TORNILLO@10|AGUJERO Ø35@1250». Open and plain on purpose: the
/// machine's own software maps it to its format.
pub fn machine_csv(model: &Model) -> String {
    let mut s = String::from("Pieza;Caja;Panel;Elemento;Perfil;Largo mm;Cantidad;Operaciones\n");
    for p in all_pieces(model) {
        let ops: Vec<String> = p
            .operations
            .iter()
            .map(|(at, o)| format!("{}@{:.0}", o.code(), at * 1000.0))
            .collect();
        s.push_str(&format!(
            "{};{};{};{};{};{:.0};1;{}\n",
            p.mark,
            p.box_mark,
            p.panel,
            p.name,
            p.section,
            p.length * 1000.0,
            ops.join("|")
        ));
    }
    s
}

/// The text a piece's label's code carries: what the reader shows on the
/// floor of the factory and on site.
pub fn label_code(model: &Model, p: &FactoryPiece) -> String {
    format!(
        "ARCHISIM;{};{};{};{};{:.0}",
        model.project_name,
        p.mark,
        p.section,
        p.name,
        p.length * 1000.0
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operations_sit_on_their_piece() {
        let p = FactoryPiece {
            mark: "MOD-01·M1-1·01".into(),
            box_mark: "MOD-01".into(),
            panel: "M1-1".into(),
            role: Role::Stud,
            name: "Montante".into(),
            section: "C 100×40×12×1,2".into(),
            length: 2.5,
            operations: vec![(0.01, Operation::Dimple), (1.0, Operation::Hole(0.035))],
        };
        assert!(p
            .operations
            .iter()
            .all(|(at, _)| *at >= 0.0 && *at <= p.length));
        assert_eq!(Operation::Hole(0.035).code(), "AGUJERO Ø35");
    }
}
