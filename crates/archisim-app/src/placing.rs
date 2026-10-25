//! Placing as in Paralives, the part without the interface (revisión de
//! Antonio del 23/10/2026): what surface a small thing rests on, where a
//! piece of furniture goes against a wall, and pieces that become one when
//! placed side by side. The interface is in `app/placing`.

use archisim_core::{Model, ObjectId, ObjectTypeId, Point2, Vec2};

/// How near a wall a piece of furniture is pulled against it.
pub const WALL_PULL: f64 = 0.35;

/// How near two pieces are to be joined.
const JOIN_GAP: f64 = 0.06;

/// The top of the highest piece of furniture of a level whose plan holds
/// `p`, over the level (none: the floor). `skip` is left out.
pub fn plan_surface_at(
    m: &Model,
    level: archisim_core::LevelId,
    p: Point2,
    skip: Option<ObjectId>,
) -> Option<f64> {
    m.objects()
        .filter(|o| o.level == level && Some(o.id) != skip)
        .filter_map(|o| {
            let (lo, hi) = m.object_type_box_3d(o.object_type)?;
            let (c, s) = (o.rotation.cos(), o.rotation.sin());
            let d = p - o.position;
            // Into the object's axes.
            let (x, y) = (d.x * c + d.y * s, -d.x * s + d.y * c);
            if !(x >= lo[0] && x <= hi[0] && y >= lo[1] && y <= hi[1]) {
                return None;
            }
            // The highest face looking up right there (a table's top, not
            // the backs of its chairs); without a model, its box's top.
            let top = top_at(m, o.object_type, x, y).unwrap_or(hi[2]);
            Some(o.base_offset + top)
        })
        .filter(|z| *z < 2.4)
        .reduce(f64::max)
}

/// The height of the highest face looking up of an object type's model
/// over the point `(x, y)` of its axes.
fn top_at(m: &Model, t: ObjectTypeId, x: f64, y: f64) -> Option<f64> {
    let (mesh, _) = m.type_prop(t)?;
    let pts = &mesh.positions;
    let mut best: Option<f64> = None;
    for (_, tris) in &mesh.parts {
        for tri in tris {
            let [a, b, c] = tri.map(|i| pts[i as usize].map(f64::from));
            let (e1, e2) = (
                [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
                [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
            );
            let nz = e1[0] * e2[1] - e1[1] * e2[0];
            let n = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                nz,
            ];
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if len < 1e-12 || nz / len < 0.7 {
                continue;
            }
            // Barycentric in plan.
            let (px, py) = (x - a[0], y - a[1]);
            let u = (px * e2[1] - py * e2[0]) / nz;
            let v = (e1[0] * py - e1[1] * px) / nz;
            if u < -1e-9 || v < -1e-9 || u + v > 1.0 + 1e-9 {
                continue;
            }
            let z = a[2] + u * e1[2] + v * e2[2];
            best = Some(best.map_or(z, |b: f64| b.max(z)));
        }
    }
    best
}

/// Where a piece of furniture dragged to `p` goes if a wall is near: its
/// back on the wall's face and its front away from it (position and
/// rotation); none if no wall of its level is within [`WALL_PULL`].
pub fn against_wall(m: &Model, level: archisim_core::LevelId, p: Point2) -> Option<(Point2, f64)> {
    let mut best: Option<(f64, Point2, f64)> = None;
    for w in m
        .walls()
        .filter(|w| w.level == level && w.bulge.abs() < 1e-9)
    {
        let half = m
            .wall_type(w.wall_type)
            .map_or(0.1, |t| t.thickness() / 2.0);
        // The middle line of the wall (its drawn line may be a face; the
        // exterior face is on the left of start → end).
        let ab = w.end - w.start;
        let len = ab.length();
        if len < 1e-6 {
            continue;
        }
        let u = ab * (1.0 / len);
        let left = Vec2::new(-u.y, u.x);
        let shift = match w.reference {
            archisim_core::ReferenceLine::Center => 0.0,
            archisim_core::ReferenceLine::ExteriorFace => -half,
            archisim_core::ReferenceLine::InteriorFace => half,
        };
        let a = w.start + left * shift;
        let t = (p - a).dot(u);
        if t < 0.0 || t > len {
            continue;
        }
        let foot = a + u * t;
        let off = p - foot;
        let dist = off.length();
        let gap = dist - half;
        if gap > WALL_PULL || gap < -half {
            continue;
        }
        // The side of the wall the point is on.
        let n = if dist > 1e-9 {
            off * (1.0 / dist)
        } else {
            Vec2::new(-u.y, u.x)
        };
        let at = foot + n * half;
        // Forward (+y of the object) along `n`: rotation θ with
        // (−sin θ, cos θ) = n.
        let rot = (-n.x).atan2(n.y);
        if best.is_none_or(|(g, ..)| gap < g) {
            best = Some((gap, at, rot));
        }
    }
    best.map(|(_, at, rot)| (at, rot))
}

/// How near the end of another piece of its kind a joinable piece is
/// pulled to touch it.
pub const JOIN_PULL: f64 = 0.4;

/// Where a piece of a family that joins (`joins = "width"`), of type `t`,
/// placed at `at` turned `rot`, goes to touch the end of another of its
/// family beside it, in line with it; none if there is none near.
pub fn snap_alongside(
    m: &Model,
    level: archisim_core::LevelId,
    t: ObjectTypeId,
    at: Point2,
    rot: f64,
    skip: Option<ObjectId>,
) -> Option<Point2> {
    let ty = m.object_type(t)?;
    let joins = m
        .family(ty.family)?
        .def()
        .outputs
        .iter()
        .any(|o| o.name == "joins");
    if !joins {
        return None;
    }
    let (lo, hi) = m.object_type_box_3d(t)?;
    let w = hi[0] - lo[0];
    let d = hi[1] - lo[1];
    let along = Vec2::new(rot.cos(), rot.sin());
    let across = Vec2::new(-rot.sin(), rot.cos());
    // Base kitchen modules also meet in a corner and back to back (an
    // island): under one worktop.
    let base_unit = m.family_prop(ty.family).as_deref() == Some("base_unit");
    let mut best: Option<(f64, Point2)> = None;
    let mut offer = |gap: f64, q: Point2| {
        if gap <= JOIN_PULL && best.is_none_or(|(g, _)| gap < g) {
            best = Some((gap, q));
        }
    };
    for o in m
        .objects()
        .filter(|o| o.level == level && Some(o.id) != skip)
    {
        let Some(ot) = m.object_type(o.object_type) else {
            continue;
        };
        if ot.family != ty.family {
            continue;
        }
        let Some((olo, ohi)) = m.object_type_box_3d(o.object_type) else {
            continue;
        };
        let ow = ohi[0] - olo[0];
        let od = ohi[1] - olo[1];
        let turn = (o.rotation - rot).rem_euclid(std::f64::consts::TAU);
        let near =
            |a: f64| (turn - a).abs() < 0.02 || (turn - a - std::f64::consts::TAU).abs() < 0.02;
        if base_unit && !near(0.0) {
            // In the other's axes (x along it, y from its back to its front).
            let (oc, os) = (o.rotation.cos(), o.rotation.sin());
            let to_world =
                |x: f64, y: f64| o.position + Vec2::new(x * oc - y * os, x * os + y * oc);
            let (mc, ms) = (rot.cos(), rot.sin());
            let from_me = |x: f64, y: f64| Vec2::new(x * mc - y * ms, x * ms + y * mc);
            let quarter = std::f64::consts::FRAC_PI_2;
            if near(-quarter) || near(quarter) {
                // A corner. This one's side against the other's front, its
                // back flush with the other's end; or the other way round.
                let sign = if near(-quarter) { 1.0 } else { -1.0 };
                let q = to_world(sign * ow / 2.0, od + w / 2.0);
                offer(q.distance(at), q);
                let q = o.position - from_me(-sign * w / 2.0, d + ow / 2.0);
                offer(q.distance(at), q);
            } else if near(std::f64::consts::PI) {
                // Back to back: an island. Their backs on one line, ends
                // or middles in line when near.
                let rel = at - o.position;
                let x = rel.dot(Vec2::new(oc, os));
                let best_x = [0.0, (ow - w) / 2.0, (w - ow) / 2.0]
                    .into_iter()
                    .filter(|c| (x - c).abs() < 0.1)
                    .min_by(|a, b| (x - a).abs().total_cmp(&(x - b).abs()))
                    .unwrap_or(x);
                let q = to_world(best_x, 0.0);
                offer(q.distance(at), q);
            }
            continue;
        }
        if !near(0.0) {
            continue;
        }
        let d = at - o.position;
        if d.dot(across).abs() > 0.15 {
            continue;
        }
        let side = d.dot(along);
        let gap = side.abs() - (w + ow) / 2.0;
        if gap.abs() > JOIN_PULL {
            continue;
        }
        let sign = if side >= 0.0 { 1.0 } else { -1.0 };
        let q = o.position + along * (sign * (w + ow) / 2.0);
        offer(gap.abs(), q);
    }
    best.map(|(_, q)| q)
}

/// If the object `id` was placed against another of its family that is
/// made to be joined (`joins = "width"` in the recipe's outputs), at the
/// same height and turned the same way, side by side and in line: they
/// become one, as wide as both, and `id` goes. Returns the one kept.
pub fn join_alongside(m: &mut Model, id: ObjectId) -> Option<ObjectId> {
    if let Some(k) = corner_sofa(m, id) {
        return Some(k);
    }
    let o = m.object(id)?.clone();
    let t = m.object_type(o.object_type)?.clone();
    if m.evaluate_object(id)
        .ok()?
        .outputs
        .get("joins")
        .and_then(|v| v.as_text())
        != Some("width")
    {
        return None;
    }
    let width_of = |m: &Model, ty: ObjectTypeId| {
        let (lo, hi) = m.object_type_box_3d(ty)?;
        Some((lo, hi))
    };
    let (lo, hi) = width_of(m, o.object_type)?;
    let (c, s) = (o.rotation.cos(), o.rotation.sin());
    let along = Vec2::new(c, s);
    let others: Vec<_> = m
        .objects()
        .filter(|x| x.id != id && x.level == o.level)
        .cloned()
        .collect();
    for other in others {
        let Some(ot) = m.object_type(other.object_type) else {
            continue;
        };
        if ot.family != t.family {
            continue;
        }
        let turn = (other.rotation - o.rotation).rem_euclid(std::f64::consts::TAU);
        if turn.min(std::f64::consts::TAU - turn) > 0.02
            || (other.base_offset - o.base_offset).abs() > 0.01
        {
            continue;
        }
        let Some((olo, ohi)) = width_of(m, other.object_type) else {
            continue;
        };
        // Same depth and height, in line (their backs on one line).
        if ((ohi[1] - olo[1]) - (hi[1] - lo[1])).abs() > 0.02
            || ((ohi[2] - olo[2]) - (hi[2] - lo[2])).abs() > 0.02
        {
            continue;
        }
        let d = o.position - other.position;
        let sideways = d.dot(along);
        let back = d.dot(Vec2::new(-s, c));
        if back.abs() > 0.03 {
            continue;
        }
        let (w, ow) = (hi[0] - lo[0], ohi[0] - olo[0]);
        let touching = (sideways.abs() - (w + ow) / 2.0).abs() <= JOIN_GAP;
        if !touching {
            continue;
        }
        // One piece over both: the other grows, centred on the pair.
        let total = w + ow;
        let left = (sideways - w / 2.0).min(-ow / 2.0);
        let mid = other.position + along * (left + total / 2.0);
        crate::drag::set_object_size(m, other.id, "width", (total * 1000.0).round() / 1000.0)
            .ok()?;
        m.edit_object(other.id, |x| x.position = mid).ok()?;
        m.remove_object(id).ok()?;
        return Some(other.id);
    }
    None
}

/// The values of an object type: its own over its family's defaults.
fn type_values(m: &Model, t: ObjectTypeId) -> Option<archisim_core::family::Values> {
    let ty = m.object_type(t)?;
    let mut v = m
        .family_defaults(ty.family, archisim_core::family::ParamScope::Type)
        .ok()?;
    v.extend(ty.values.clone());
    Some(v)
}

/// How far the arm of an L of two straight sofas may be from where it
/// would make a corner sofa (m).
const CORNER_GAP: f64 = 0.06;

/// If the sofa `id` was placed in an L with another straight sofa of the
/// same depth and height (the end of one against the front of the other,
/// turned a quarter): both become one corner sofa, as long as the first
/// and with its arm as long as the second. Returns the corner sofa.
pub fn corner_sofa(m: &mut Model, id: ObjectId) -> Option<ObjectId> {
    let o = m.object(id)?.clone();
    let sofa = |m: &Model, x: &archisim_core::Object| -> Option<(f64, f64, f64)> {
        let t = m.object_type(x.object_type)?;
        let f = m.family(t.family)?;
        if f.def().name != "Sofá" {
            return None;
        }
        let v = type_values(m, x.object_type)?;
        let num = |k: &str| match v.get(k) {
            Some(archisim_core::family::Value::Number(n)) => Some(*n),
            _ => None,
        };
        Some((num("width")?, num("depth")?, num("height")?))
    };
    sofa(m, &o)?;
    let others: Vec<_> = m
        .objects()
        .filter(|x| {
            x.id != id && x.level == o.level && (x.base_offset - o.base_offset).abs() < 0.01
        })
        .cloned()
        .collect();
    for other in others {
        if sofa(m, &other).is_none() {
            continue;
        }
        // Either of the two may be the long one.
        for (a, b) in [(&o, &other), (&other, &o)] {
            let (Some((wa, da, ha)), Some((wb, db, hb))) = (sofa(m, a), sofa(m, b)) else {
                continue;
            };
            if (da - db).abs() > 0.02 || (ha - hb).abs() > 0.02 {
                continue;
            }
            // `b` in the frame of `a`: x along it, y forward.
            let (c, s) = (a.rotation.cos(), a.rotation.sin());
            let d = b.position - a.position;
            let local = Point2::new(d.x * c + d.y * s, -d.x * s + d.y * c);
            let turn = (b.rotation - a.rotation).rem_euclid(std::f64::consts::TAU);
            let quarter = std::f64::consts::FRAC_PI_2;
            let side = if (turn - quarter).abs() < 0.02 {
                // Its back on a's right end, facing left: the arm on the right.
                "Derecha"
            } else if (turn - 3.0 * quarter).abs() < 0.02 {
                "Izquierda"
            } else {
                continue;
            };
            let x = if side == "Derecha" {
                wa / 2.0
            } else {
                -wa / 2.0
            };
            let y = da + wb / 2.0;
            if (local.x - x).abs() > CORNER_GAP || (local.y - y).abs() > CORNER_GAP {
                continue;
            }
            let (a, b) = (a.clone(), b.clone());
            let base = archisim_library::add_object_entry(m, "Sofá rinconera 260×170").ok()?;
            let corner = m.object_type(base)?.clone();
            let va = type_values(m, a.object_type)?;
            let vb = type_values(m, b.object_type)?;
            let mut values = corner.values.clone();
            use archisim_core::family::Value;
            let length2 = ((da + wb) * 1000.0).round() / 1000.0;
            values.insert("width".into(), Value::Number(wa));
            values.insert("length2".into(), Value::Number(length2));
            values.insert("depth".into(), Value::Number(da));
            values.insert("side".into(), Value::Text(side.into()));
            for k in [
                "seat", "height", "places", "mat", "model3d", "cushion", "frame",
            ] {
                if let Some(v) = va.get(k) {
                    values.insert(k.into(), v.clone());
                }
            }
            if let Some(Value::Number(n)) = vb.get("places") {
                values.insert("places2".into(), Value::Number((n - 1.0).max(1.0)));
            }
            let name = format!("Sofá rinconera {:.0}×{:.0}", wa * 100.0, length2 * 100.0);
            let existing = m
                .object_types()
                .find(|t| t.name == name && t.family == corner.family)
                .map(|t| t.id);
            let t = match existing {
                Some(t) => t,
                None => m.add_object_type(&name, corner.family, values).ok()?,
            };
            let new = m
                .add_object(archisim_core::NewObject {
                    level: a.level,
                    object_type: t,
                    position: a.position,
                    rotation: a.rotation,
                    base_offset: a.base_offset,
                })
                .ok()?;
            m.remove_object(a.id).ok()?;
            m.remove_object(b.id).ok()?;
            return Some(new);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pick::tests::room;
    use archisim_core::NewObject;

    fn place(m: &mut Model, name: &str, at: Point2, rotation: f64, base: f64) -> ObjectId {
        let level = m.levels()[0].id;
        let t = archisim_library::add_object_entry(m, name).unwrap();
        m.add_object(NewObject {
            level,
            object_type: t,
            position: at,
            rotation,
            base_offset: base,
        })
        .unwrap()
    }

    #[test]
    fn furniture_goes_against_the_wall_facing_the_room() {
        let (m, level, _) = room();
        let w = m.walls().next().unwrap();
        let t = m.wall_type(w.wall_type).unwrap().thickness();
        // Near the west wall (its inner face at x = t): its back on it,
        // looking east.
        let (at, rot) = against_wall(&m, level, Point2::new(t + 0.2, 2.0)).unwrap();
        assert!(
            (at.x - t).abs() < 1e-6 && (at.y - 2.0).abs() < 1e-6,
            "{at:?}"
        );
        let forward = Vec2::new(-rot.sin(), rot.cos());
        assert!((forward.x - 1.0).abs() < 1e-6, "{forward:?}");
        // In the middle of the room: no wall.
        assert!(against_wall(&m, level, Point2::new(3.0, 2.0)).is_none());
    }

    #[test]
    fn small_things_rest_on_tables_and_pieces_join() {
        let (mut m, level, _) = room();
        let table = place(
            &mut m,
            "Mesa de comedor 160×90",
            Point2::new(3.0, 1.5),
            0.0,
            0.0,
        );
        let top = plan_surface_at(&m, level, Point2::new(3.0, 1.9), None).unwrap();
        assert!((top - 0.75).abs() < 0.02, "{top}");
        assert!(plan_surface_at(&m, level, Point2::new(5.5, 3.5), None).is_none());
        assert!(plan_surface_at(&m, level, Point2::new(3.0, 1.9), Some(table)).is_none());
        // Two worktops side by side along the north wall: one of 4,80 m.
        let a = place(
            &mut m,
            "Muebles de cocina 240",
            Point2::new(2.0, 3.7),
            std::f64::consts::PI,
            0.0,
        );
        let b = place(
            &mut m,
            "Muebles de cocina 240",
            Point2::new(4.4, 3.7),
            std::f64::consts::PI,
            0.0,
        );
        assert_eq!(join_alongside(&mut m, b), Some(a));
        assert!(m.object(b).is_none());
        let (lo, hi) = m
            .object_type_box_3d(m.object(a).unwrap().object_type)
            .unwrap();
        assert!((hi[0] - lo[0] - 4.8).abs() < 0.02, "{lo:?} {hi:?}");
        let mid = m.object(a).unwrap().position;
        assert!(
            (mid.x - 3.2).abs() < 0.01 && (mid.y - 3.7).abs() < 1e-6,
            "{mid:?}"
        );
        // Apart, they do not.
        let c = place(
            &mut m,
            "Muebles de cocina 240",
            Point2::new(2.0, 1.0),
            0.0,
            0.0,
        );
        assert_eq!(join_alongside(&mut m, c), None);
        // Placed 25 cm short of its end, one is pulled to touch it.
        let t = m.object(c).unwrap().object_type;
        let q = snap_alongside(&m, level, t, Point2::new(4.65, 1.05), 0.0, None).unwrap();
        assert!(
            (q.x - 4.4).abs() < 1e-6 && (q.y - 1.0).abs() < 1e-6,
            "{q:?}"
        );
    }

    /// Base kitchen modules also meet in a corner and back to back.
    #[test]
    fn kitchen_modules_meet_in_a_corner_and_back_to_back() {
        let (mut m, level, _) = room();
        let quarter = std::f64::consts::FRAC_PI_2;
        let a = place(&mut m, "Bajo cajonera 60", Point2::new(2.3, 0.0), 0.0, 0.0);
        let t = m.object(a).unwrap().object_type;
        // Turned a quarter, brought near the end of its front: its side
        // against that front, its back flush with that end.
        let q = snap_alongside(&m, level, t, Point2::new(2.5, 1.0), quarter, None).unwrap();
        assert!(
            (q.x - 2.6).abs() < 1e-6 && (q.y - 0.9).abs() < 1e-6,
            "{q:?}"
        );
        // The other way round: the row's piece brought to the leg.
        let b = place(
            &mut m,
            "Bajo cajonera 60",
            Point2::new(2.6, 0.9),
            quarter,
            0.0,
        );
        let q = snap_alongside(&m, level, t, Point2::new(2.2, 0.1), 0.0, Some(a)).unwrap();
        assert!((q.x - 2.3).abs() < 1e-6 && q.y.abs() < 1e-6, "{q:?}");
        assert!(m.worktop_run_of(b).is_some());
        // Back to back, nearly in line: on one line, ends together.
        let c = place(&mut m, "Bajo cajonera 60", Point2::new(6.0, 2.0), 0.0, 0.0);
        let q = snap_alongside(
            &m,
            level,
            t,
            Point2::new(6.05, 1.9),
            std::f64::consts::PI,
            Some(a),
        )
        .unwrap();
        assert!(
            (q.x - 6.0).abs() < 1e-6 && (q.y - 2.0).abs() < 1e-6,
            "{q:?}"
        );
        let _ = c;
    }

    #[test]
    fn two_sofas_in_an_l_become_a_corner_sofa() {
        let (mut m, _, _) = room();
        // A three-seater along the north wall facing south, and a
        // two-seater down from its east end, facing west.
        let a = place(
            &mut m,
            "Sofá de 3 plazas",
            Point2::new(3.0, 3.7),
            std::f64::consts::PI,
            0.0,
        );
        let wa = type_values(&m, m.object(a).unwrap().object_type).unwrap();
        let num = |v: &archisim_core::family::Values, k: &str| match v.get(k) {
            Some(archisim_core::family::Value::Number(n)) => *n,
            _ => panic!("{k}"),
        };
        let (w, d) = (num(&wa, "width"), num(&wa, "depth"));
        let b = place(&mut m, "Sofá de 3 plazas", Point2::new(0.0, 0.0), 0.0, 0.0);
        let wb = num(
            &type_values(&m, m.object(b).unwrap().object_type).unwrap(),
            "width",
        );
        // In a's frame (turned half round), its left end is at x = 3 + w/2.
        m.edit_object(b, |x| {
            x.rotation = std::f64::consts::PI - std::f64::consts::FRAC_PI_2;
            x.position = Point2::new(3.0 + w / 2.0, 3.7 - d - wb / 2.0);
        })
        .unwrap();
        let kept = join_alongside(&mut m, b).expect("a corner sofa");
        assert!(m.object(a).is_none() && m.object(b).is_none());
        let v = type_values(&m, m.object(kept).unwrap().object_type).unwrap();
        assert_eq!(v.get("side").and_then(|v| v.as_text()), Some("Izquierda"));
        assert!((num(&v, "width") - w).abs() < 1e-9);
        assert!((num(&v, "length2") - (d + wb)).abs() < 1e-6);
        let o = m.object(kept).unwrap();
        assert!((o.position.x - 3.0).abs() < 1e-9 && (o.position.y - 3.7).abs() < 1e-9);
    }
}
