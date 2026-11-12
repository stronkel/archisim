//! The rules of the modular system as warnings (M15, `docs/MODULAR.md`):
//! what a box must be (its issues: size, transport, joints), where its
//! doors and windows may go (not too near the corner posts, no window in a
//! joint between boxes, not wider than the chassis spans), wet rooms that
//! straddle a joint, and the services: what crosses a joint (a connection
//! with a register) and the holes in the studs (M6). The values are the
//! usual ones, gathered here to be agreed with the manufacturer.

use archisim_core::{BoxId, Model, Point2, SideState};
use archisim_geom::box_services::{joint_crossings, stud_holes, STUD_HOLE_END, STUD_HOLE_SHARE};

use crate::{Item, Report, Section, Status, Subject};

/// A door or window's jamb at least this far from a corner of its box (the
/// corner post and its gusset), m.
pub const CORNER_CLEAR: f64 = 0.30;
/// The widest opening in a box's side without a beam over it, m.
pub const MAX_OPENING: f64 = 2.40;

fn m2(v: f64) -> String {
    format!("{v:.2}").replace('.', ",")
}

fn item(
    status: Subject,
    s: Status,
    level: Option<archisim_core::LevelId>,
    rule: &'static str,
    text: String,
    value: String,
    required: String,
) -> Item {
    Item {
        section: Section::Modular,
        status: s,
        subject: status,
        level,
        rule,
        text,
        value,
        required,
        route: Vec::new(),
    }
}

/// Which box and side a wall is on: the side its line runs along.
fn box_side_of(model: &Model, b: BoxId, a: Point2, z: Point2) -> Option<usize> {
    let bx = model.modular_box(b)?;
    let c = bx.outline();
    let mid = (a + z) * 0.5;
    (0..4).min_by(|i, j| {
        let d = |k: usize| {
            let (p, q) = (c[k], c[(k + 1) % 4]);
            let u = (q - p).normalized().unwrap_or(Point2::new(1.0, 0.0));
            let n = Point2::new(-u.y, u.x);
            ((mid - p).dot(n)).abs()
        };
        d(*i).total_cmp(&d(*j))
    })
}

/// Whether a segment runs into a polygon: an end inside, or across an edge.
fn segment_meets(a: Point2, b: Point2, poly: &[Point2]) -> bool {
    if archisim_core::point_in_polygon(a, poly) || archisim_core::point_in_polygon(b, poly) {
        return true;
    }
    let cross =
        |o: Point2, p: Point2, q: Point2| (p.x - o.x) * (q.y - o.y) - (p.y - o.y) * (q.x - o.x);
    (0..poly.len()).any(|i| {
        let (c, d) = (poly[i], poly[(i + 1) % poly.len()]);
        cross(a, b, c) * cross(a, b, d) < 0.0 && cross(c, d, a) * cross(c, d, b) < 0.0
    })
}

pub fn check(model: &Model, report: &mut Report) {
    if model.boxes().next().is_none() {
        return;
    }
    // The boxes' own issues.
    for b in model.boxes() {
        let issues = model.box_issues(b.id);
        if issues.is_empty() {
            report.items.push(item(
                Subject::Building,
                Status::Passes,
                Some(b.level),
                "Sistema modular: caja",
                format!("Caja {}: medidas, transporte y juntas.", b.mark),
                format!("{} × {} m", m2(b.length), m2(b.width)),
                "sin avisos".into(),
            ));
        }
        for i in issues {
            report.items.push(item(
                Subject::Building,
                if i.is_error() {
                    Status::Fails
                } else {
                    Status::Warning
                },
                Some(b.level),
                "Sistema modular: caja",
                format!("Caja {}: {}", b.mark, i.text(model)),
                String::new(),
                String::new(),
            ));
        }
        // Its doors and windows.
        let held = model.box_members(b.id);
        let corners = b.outline();
        for o in held.openings.iter().filter_map(|o| model.opening(*o)) {
            let (Some(w), Some(t)) = (model.wall(o.wall), model.opening_type(o.opening_type))
            else {
                continue;
            };
            let Some(u) = (w.end - w.start).normalized() else {
                continue;
            };
            let centre = w.start + u * o.along;
            let half = t.width / 2.0;
            let jambs = [centre - u * half, centre + u * half];
            let near = corners
                .iter()
                .flat_map(|c| jambs.iter().map(move |j| j.distance(*c)))
                .fold(f64::INFINITY, f64::min);
            let name = if t.mark.is_empty() {
                t.name.clone()
            } else {
                t.mark.clone()
            };
            if near < CORNER_CLEAR {
                report.items.push(item(
                    Subject::Opening(o.id),
                    Status::Warning,
                    Some(b.level),
                    "Sistema modular: huecos",
                    format!(
                        "{name} de la caja {}: su jamba está junto al pilar de esquina del chasis.",
                        b.mark
                    ),
                    format!("{} m", m2(near)),
                    format!("≥ {} m", m2(CORNER_CLEAR)),
                ));
            }
            if t.width > MAX_OPENING + 1e-6 {
                report.items.push(item(
                    Subject::Opening(o.id),
                    Status::Warning,
                    Some(b.level),
                    "Sistema modular: huecos",
                    format!(
                        "{name} de la caja {}: más ancho de lo que salva el marco del chasis sin una viga.",
                        b.mark
                    ),
                    format!("{} m", m2(t.width)),
                    format!("≤ {} m", m2(MAX_OPENING)),
                ));
            }
            // A window in a joint between boxes.
            if !t.is_door() {
                if let Some(side) = box_side_of(model, b.id, w.start, w.end) {
                    let against = model.box_contacts(b.id).iter().any(|k| k.side == side);
                    if against && b.sides[side] != SideState::Free {
                        report.items.push(item(
                            Subject::Opening(o.id),
                            Status::Fails,
                            Some(b.level),
                            "Sistema modular: huecos",
                            format!(
                                "{name} de la caja {}: una ventana en el lado que da a otra caja.",
                                b.mark
                            ),
                            String::new(),
                            "en un lado libre".into(),
                        ));
                    }
                }
            }
        }
    }
    // Wet rooms over a joint.
    for r in model.rooms() {
        let use_ = r.usage.to_lowercase();
        let name = r.name.to_lowercase();
        let wet = ["baño", "aseo", "cocina", "lavadero"]
            .iter()
            .any(|k| use_.contains(k) || name.contains(k));
        if !wet {
            continue;
        }
        let crossed = model
            .box_joints_on(r.level)
            .iter()
            .any(|j| segment_meets(j.start, j.end, &r.outline));
        if crossed {
            report.items.push(item(
                Subject::Room(r.id),
                Status::Warning,
                Some(r.level),
                "Sistema modular: zonas húmedas",
                format!(
                    "{} {}: el local húmedo pasa por una junta entre cajas (agua y saneamiento con unión en obra).",
                    r.number, r.name
                ),
                String::new(),
                "en una sola caja".into(),
            ));
        }
    }
    // What crosses a joint (M6).
    for c in joint_crossings(model) {
        let (a, b) = (
            model
                .modular_box(c.a)
                .map_or("?".into(), |x| x.mark.clone()),
            model
                .modular_box(c.b)
                .map_or("?".into(), |x| x.mark.clone()),
        );
        let subject = c.run.map_or(Subject::Building, Subject::Run);
        report.items.push(item(
            subject,
            Status::Info,
            None,
            "Sistema modular: instalaciones",
            format!(
                "{} {} entre {a} y {b}: se corta en fábrica y se une en obra ({}), con registro.",
                c.system.name(),
                c.size,
                c.kind.label().to_lowercase()
            ),
            format!("a {} m", m2(c.z)),
            String::new(),
        ));
    }
    // The holes in the studs (M6).
    for h in stud_holes(model) {
        let level = model.wall(h.wall).map(|w| w.level);
        let status = if h.fails() {
            Status::Fails
        } else {
            Status::Passes
        };
        let why = if h.too_big {
            format!(
                "el paso de Ø {:.0} mm es grande para el alma del {}",
                h.diameter * 1000.0,
                h.stud
            )
        } else if h.too_near_end {
            format!(
                "el paso está a menos de {:.0} cm del canal",
                STUD_HOLE_END * 100.0
            )
        } else {
            format!("paso de Ø {:.0} mm por el {}", h.diameter * 1000.0, h.stud)
        };
        report.items.push(item(
            Subject::Run(h.run),
            status,
            level,
            "Sistema modular: pasos por montantes",
            format!("Muro {} a {} m: {why}.", h.wall.0, m2(h.along)),
            format!("Ø {:.0} mm", h.diameter * 1000.0),
            format!("≤ {:.0} mm", STUD_HOLE_SHARE * h.web * 1000.0),
        ));
    }
    // The faces composed by hand: pieces that will not come out of a board
    // or are too small, joints off a jamb or over another layer's.
    for f in model.board_faces() {
        let level = model.wall(f.wall).map(|w| w.level);
        for w in archisim_geom::boards::face_warnings(model, f.wall, f.layer, &f.joints, &f.marks) {
            use archisim_geom::boards::FaceWarningKind as K;
            let status = match w.kind {
                K::TooBig => Status::Fails,
                _ => Status::Warning,
            };
            report.items.push(item(
                Subject::Wall(f.wall),
                status,
                level,
                "Sistema modular: despiece a mano",
                format!("Muro {}, capa {}: {}.", f.wall.0, f.layer + 1, w.text),
                String::new(),
                String::new(),
            ));
        }
    }
    // And the slabs and ceilings composed by hand in plan.
    for f in model.board_plans() {
        let (subject, level, what) = match f.host {
            archisim_core::PlanHost::Slab(id) => (
                Some(Subject::Floor(id)),
                model.slab(id).map(|s| s.level),
                format!("Forjado {}", id.0),
            ),
            archisim_core::PlanHost::Ceiling(id) => (
                None,
                model.ceiling(id).map(|c| c.level),
                format!("Falso techo {}", id.0),
            ),
        };
        for w in archisim_geom::boards::plan_warnings(model, f.host, f.layer, &f.joints, &f.marks) {
            use archisim_geom::boards::FaceWarningKind as K;
            let status = match w.kind {
                K::TooBig => Status::Fails,
                _ => Status::Warning,
            };
            report.items.push(item(
                subject
                    .or(level.map(Subject::Level))
                    .unwrap_or(Subject::Building),
                status,
                level,
                "Sistema modular: despiece a mano",
                format!("{what}, capa {}: {}.", f.layer + 1, w.text),
                String::new(),
                String::new(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{NewBox, NewRoom, BOX_JOINT, BOX_WIDTH};

    #[test]
    fn a_bathroom_over_a_joint_is_noted() {
        let mut m = Model::new("T");
        let lv = m.add_level("PB", 0.0);
        m.add_box(NewBox::usual(lv, Point2::new(0.0, 0.0), 0.0))
            .unwrap();
        m.add_box(NewBox::usual(
            lv,
            Point2::new(0.0, BOX_WIDTH + BOX_JOINT),
            0.0,
        ))
        .unwrap();
        let y = BOX_WIDTH + BOX_JOINT / 2.0;
        m.add_room(NewRoom {
            level: lv,
            number: "1".into(),
            name: "Baño".into(),
            outline: vec![
                Point2::new(1.0, y - 1.0),
                Point2::new(3.0, y - 1.0),
                Point2::new(3.0, y + 1.0),
                Point2::new(1.0, y + 1.0),
            ],
            holes: Vec::new(),
            label: Point2::new(2.0, y),
            bounded: false,
        })
        .unwrap();
        let mut report = Report::default();
        check(&m, &mut report);
        assert!(report
            .items
            .iter()
            .any(|i| i.section == Section::Modular && i.rule == "Sistema modular: zonas húmedas"));
    }
}
