//! Placing doors and windows: one click on a wall.
//!
//! The opening goes where the cursor is along the wall (its centre, rounded
//! to the centimetre and kept inside the wall). The side of the wall the
//! cursor is on is the side the leaves open towards, as when ArchiCAD asks
//! for the opening side. The hand (UNE-EN 12519), sill and frame setback
//! come from the tool settings.
//!
//! How it is placed along the clear stretch of the wall under the cursor
//! (between the walls that meet it and the other doors and windows, on
//! the face it opens to) is the tool's method, [`OpeningPlace`]: where the
//! cursor is, in the middle, at a distance from the nearest end, or
//! several spread out with equal piers.

use crate::geo::Geo;
use archisim_core::{Hand, LevelId, NewOpening, OpeningTypeId, Point2, WallId, WallSide};
use archisim_geom::OpeningGeometry;

use crate::pick::{contains, distance_to_outline};
use crate::snap::FREE_STEP;
use crate::units::explain;

/// Settings of the door or window tool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpeningSettings {
    pub opening_type: OpeningTypeId,
    /// Height of the bottom of the opening above the level.
    pub sill: f64,
    pub hand: Hand,
    /// Distance from the wall's exterior face to the frame; `None` centres
    /// the frame in the wall.
    pub setback: Option<f64>,
    /// How it is placed along the wall.
    pub place: OpeningPlace,
}

/// How a door or window is placed along the clear stretch of the wall
/// under the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum OpeningPlace {
    /// Its centre where the cursor is.
    #[default]
    Free,
    /// In the middle of the stretch.
    Centre,
    /// Its jamb at this distance from the end of the stretch nearest the
    /// cursor.
    FromEnd(f64),
    /// This many of them, with the same pier between them and at the ends.
    Spread(usize),
}

impl OpeningPlace {
    /// The name on the tool's bar.
    pub fn name(self) -> &'static str {
        match self {
            OpeningPlace::Free => "Libre",
            OpeningPlace::Centre => "Por el centro",
            OpeningPlace::FromEnd(_) => "Desde un extremo",
            OpeningPlace::Spread(_) => "Repartidas",
        }
    }
}

/// Where a door or window would go, and whether it fits.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub spec: NewOpening,
    /// Clear wall left before and after the opening, along the drawn line.
    pub before: f64,
    pub after: f64,
    /// Why it cannot be placed there, in Spanish.
    pub problem: Option<String>,
    /// Where it would be drawn (also when it does not fit, to show why).
    pub geometry: OpeningGeometry,
    /// The others placed with it, when they are spread out.
    pub more: Vec<(NewOpening, OpeningGeometry)>,
}

impl Placement {
    /// Every door or window a click would place.
    pub fn specs(&self) -> Vec<NewOpening> {
        std::iter::once(self.spec.clone())
            .chain(self.more.iter().map(|(s, _)| s.clone()))
            .collect()
    }

    /// Where each of them would be drawn.
    pub fn geometries(&self) -> impl Iterator<Item = &OpeningGeometry> {
        std::iter::once(&self.geometry).chain(self.more.iter().map(|(_, g)| g))
    }
}

/// The clear stretch around `along` on the face at `face` of a straight
/// wall: between the walls that meet or cross it and the other doors and
/// windows. `None` in a curved wall or inside another opening.
pub fn clear_span<G: Geo + ?Sized>(
    geo: &G,
    wall: WallId,
    face: f64,
    along: f64,
) -> Option<(f64, f64)> {
    let model = geo.model();
    let w = model.wall(wall)?;
    if w.is_curved() {
        return None;
    }
    let u = (w.end - w.start).normalized()?;
    let n = u.perp_left();
    let origin = w.start + n * face;
    let mut stops = crate::temp_dims::face_stops(geo, wall, origin, u, n);
    for o in archisim_geom::openings_in_wall(model, wall) {
        if along > o.t0 + 1e-9 && along < o.t1 - 1e-9 {
            return None;
        }
        stops.push(o.t0);
        stops.push(o.t1);
    }
    let len = w.length();
    let a = stops
        .iter()
        .copied()
        .filter(|t| *t <= along + 1e-9 && *t > -1.0)
        .max_by(f64::total_cmp)
        .unwrap_or(0.0);
    let b = stops
        .iter()
        .copied()
        .filter(|t| *t >= along - 1e-9 && *t < len + 1.0)
        .min_by(f64::total_cmp)
        .unwrap_or(len);
    (b - a > 1e-6).then_some((a, b))
}

/// Rounds to the millimetre, so the dimensions read clean.
fn mm(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

/// The wall under `cursor` (inside it or within `tolerance` metres).
pub fn wall_at<G: Geo + ?Sized>(
    geo: &G,
    level: LevelId,
    cursor: Point2,
    tolerance: f64,
) -> Option<WallId> {
    let model = geo.model();
    model
        .walls_on_level(level)
        .filter_map(|w| {
            let g = geo.wall_geometry(w.id)?;
            let d = if contains(&g.outline, cursor) {
                0.0
            } else {
                distance_to_outline(&g.outline, cursor)
            };
            (d <= tolerance).then_some((d, w.id))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, id)| id)
}

/// Where a door or window of the given settings would go for this cursor.
pub fn placement<G: Geo + ?Sized>(
    geo: &G,
    level: LevelId,
    cursor: Point2,
    tolerance: f64,
    s: &OpeningSettings,
) -> Option<Placement> {
    let model = geo.model();
    let wall_id = wall_at(geo, level, cursor, tolerance)?;
    let wall = model.wall(wall_id)?;
    let otype = model.opening_type(s.opening_type)?;
    let width = otype.width;
    let thickness = model.wall_type(wall.wall_type)?.thickness();
    let len = wall.length();
    // Along the wall (the arc, in a curved wall) and across it.
    let (raw, across) = match wall.arc() {
        Some(arc) => {
            let (f, off) = arc.project(cursor);
            (f * len, off)
        }
        None => {
            let u = (wall.end - wall.start).normalized()?;
            (
                (cursor - wall.start).dot(u),
                (cursor - wall.start).dot(u.perp_left()),
            )
        }
    };
    let (ext, int) = wall.face_offsets(thickness);
    let opens_to = if across > (ext + int) / 2.0 {
        WallSide::Exterior
    } else {
        WallSide::Interior
    };
    let free = || {
        let mut along = (raw / FREE_STEP).round() * FREE_STEP;
        if len >= width {
            along = along.clamp(width / 2.0, len - width / 2.0);
        }
        along
    };
    // The clear stretch on the face it opens to.
    let face = match opens_to {
        WallSide::Exterior => ext,
        WallSide::Interior => int,
    };
    let span = match s.place {
        OpeningPlace::Free => None,
        _ => clear_span(geo, wall_id, face, raw),
    };
    let alongs: Vec<f64> = match (s.place, span) {
        (OpeningPlace::Centre, Some((a, b))) => vec![mm((a + b) / 2.0)],
        (OpeningPlace::FromEnd(d), Some((a, b))) => {
            if raw - a <= b - raw {
                vec![mm(a + d + width / 2.0)]
            } else {
                vec![mm(b - d - width / 2.0)]
            }
        }
        (OpeningPlace::Spread(k), Some((a, b))) if k >= 1 => {
            let gap = (b - a - k as f64 * width) / (k as f64 + 1.0);
            (0..k)
                .map(|i| mm(a + gap * (i as f64 + 1.0) + width * (i as f64 + 0.5)))
                .collect()
        }
        _ => vec![free()],
    };
    let setback = s.setback.unwrap_or((thickness - otype.frame_depth) / 2.0);
    let mut made: Vec<(NewOpening, OpeningGeometry)> = Vec::new();
    for along in alongs {
        // Its frame: the wall, or the tangent at its centre in a curved one.
        let (c, u) = wall.opening_frame(along, width)?;
        let origin = c - u * along;
        let n = u.perp_left();
        let spec = NewOpening {
            opening_type: s.opening_type,
            wall: wall_id,
            along,
            sill: s.sill,
            opens_to,
            hand: s.hand,
            setback: s.setback,
        };
        let geometry = OpeningGeometry {
            opening: archisim_core::OpeningId(0),
            wall: wall_id,
            kind: otype.kind,
            origin,
            u,
            n,
            t0: along - width / 2.0,
            t1: along + width / 2.0,
            exterior_offset: ext,
            interior_offset: int,
            frame_exterior: ext - setback,
            frame_interior: ext - setback - otype.frame_depth,
            frame_width: otype.frame_width,
            sill: s.sill,
            head: s.sill + otype.height,
            opens_to,
            hand: s.hand,
            corner: None,
            shape: otype.shape,
        };
        made.push((spec, geometry));
    }
    // Spread out: they must fit in the stretch, with some pier left.
    if let (OpeningPlace::Spread(k), Some((a, b))) = (s.place, span) {
        if k as f64 * width > b - a - 0.02 * (k as f64 + 1.0) {
            let mut v: Vec<(NewOpening, OpeningGeometry)> = made;
            let (spec, geometry) = v.remove(0);
            return Some(Placement {
                before: geometry.t0,
                after: len - geometry.t1,
                spec,
                problem: Some(format!(
                    "no caben {k} de {} en el tramo libre de {}",
                    crate::units::fmt_len(width),
                    crate::units::fmt_len(b - a)
                )),
                geometry,
                more: v,
            });
        }
    }
    // All of them must fit, one after the other.
    let mut trial = model.clone();
    let problem = made
        .iter()
        .find_map(|(spec, _)| trial.add_opening(spec.clone()).err())
        .map(|e| explain(&e));
    let mut made = made.into_iter();
    let (spec, geometry) = made.next()?;
    let more: Vec<_> = made.collect();
    let last = more.last().map_or(&geometry, |(_, g)| g);
    Some(Placement {
        before: geometry.t0,
        after: len - last.t1,
        spec,
        problem,
        geometry,
        more,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pick::tests::room;

    #[test]
    fn places_on_the_wall_under_the_cursor() {
        // 6 × 4 m room drawn clockwise; the south wall runs from (6,0) to
        // (0,0) with a 80 cm door centred at 3 m.
        let (m, lvl, lib) = room();
        let s = OpeningSettings {
            opening_type: lib.window_one_leaf,
            sill: 0.9,
            hand: Hand::Left,
            setback: None,
            place: OpeningPlace::Free,
        };
        let north = m.walls().nth(1).unwrap().id;
        // Inside the north wall (y 3.735–4), slightly inside the room.
        let pl = placement(&m, lvl, Point2::new(2.004, 3.8), 0.1, &s).unwrap();
        assert_eq!(pl.spec.wall, north);
        assert!((pl.spec.along - 2.0).abs() < 1e-9);
        assert_eq!(pl.spec.opens_to, WallSide::Interior);
        assert!((pl.before - 1.5).abs() < 1e-9 && (pl.after - 3.5).abs() < 1e-9);
        assert_eq!(pl.problem, None);
        // The preview matches what the model will draw once placed.
        let mut placed = m.clone();
        let id = placed.add_opening(pl.spec.clone()).unwrap();
        let real = archisim_geom::opening_geometry(&placed, id).unwrap();
        assert_eq!(
            OpeningGeometry {
                opening: id,
                ..pl.geometry.clone()
            },
            real
        );
        // Just outside the façade: opens outwards.
        let pl = placement(&m, lvl, Point2::new(2.0, 4.05), 0.1, &s).unwrap();
        assert_eq!(pl.spec.opens_to, WallSide::Exterior);
        // Near a corner it is kept inside the wall.
        let pl = placement(&m, lvl, Point2::new(0.2, 3.9), 0.1, &s).unwrap();
        assert!((pl.spec.along - 0.5).abs() < 1e-9, "{}", pl.spec.along);
        // Away from every wall: nothing.
        assert!(placement(&m, lvl, Point2::new(3.0, 2.0), 0.1, &s).is_none());
        // On top of the existing door: refused, with the reason.
        let pl = placement(&m, lvl, Point2::new(3.0, 0.1), 0.1, &s).unwrap();
        assert!(pl.problem.unwrap().contains("solaparía"));
    }

    #[test]
    fn centred_from_an_end_and_spread() {
        // The north wall's inner face runs clear from x 0.265 to 5.735
        // (between the inner faces of the side walls).
        let (m, lvl, lib) = room();
        let mut s = OpeningSettings {
            opening_type: lib.window_one_leaf,
            sill: 0.9,
            hand: Hand::Left,
            setback: None,
            place: OpeningPlace::Centre,
        };
        let at = Point2::new(1.0, 3.8);
        let pl = placement(&m, lvl, at, 0.1, &s).unwrap();
        assert!((pl.spec.along - 3.0).abs() < 1e-9, "{}", pl.spec.along);
        s.place = OpeningPlace::FromEnd(0.3);
        let pl = placement(&m, lvl, at, 0.1, &s).unwrap();
        assert!((pl.spec.along - 1.065).abs() < 1e-9, "{}", pl.spec.along);
        let pl = placement(&m, lvl, Point2::new(5.0, 3.8), 0.1, &s).unwrap();
        assert!((pl.spec.along - 4.935).abs() < 1e-9, "{}", pl.spec.along);
        s.place = OpeningPlace::Spread(2);
        let pl = placement(&m, lvl, at, 0.1, &s).unwrap();
        let alongs: Vec<f64> = pl.specs().iter().map(|o| o.along).collect();
        assert_eq!(alongs.len(), 2);
        // Equal piers: 3 × 1.157 m.
        assert!((alongs[0] + alongs[1] - 6.0).abs() < 2e-3, "{alongs:?}");
        assert!((alongs[0] - 1.922).abs() < 1e-3, "{alongs:?}");
        assert_eq!(pl.problem, None);
        // The south wall (drawn from x 6 to 0): either side of its door, a
        // stretch each; inside, it ends at the side wall's inner face, and
        // outside at the corner.
        s.place = OpeningPlace::Centre;
        let pl = placement(&m, lvl, Point2::new(1.0, 0.2), 0.1, &s).unwrap();
        let (a, b) = (3.4, 6.0 - 0.265);
        assert!(
            (pl.spec.along - (a + b) / 2.0).abs() < 2e-3,
            "{}",
            pl.spec.along
        );
        let pl = placement(&m, lvl, Point2::new(1.0, 0.05), 0.1, &s).unwrap();
        assert!((pl.spec.along - 4.7).abs() < 2e-3, "{}", pl.spec.along);
        // Too many to fit: refused.
        s.place = OpeningPlace::Spread(6);
        let pl = placement(&m, lvl, at, 0.1, &s).unwrap();
        assert!(pl.problem.unwrap().contains("no caben 6"));
    }
}
