//! Placing doors and windows: one click on a wall.
//!
//! The opening goes where the cursor is along the wall (its centre, rounded
//! to the centimetre and kept inside the wall). The side of the wall the
//! cursor is on is the side the leaves open towards, as when ArchiCAD asks
//! for the opening side. The hand (UNE-EN 12519), sill and frame setback
//! come from the tool settings.

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
    if wall.is_curved() {
        // Not yet: the application says so on the click.
        return None;
    }
    let otype = model.opening_type(s.opening_type)?;
    let width = otype.width;
    let thickness = model.wall_type(wall.wall_type)?.thickness();
    let u = (wall.end - wall.start).normalized()?;
    let n = u.perp_left();
    let len = wall.length();

    let raw = (cursor - wall.start).dot(u);
    let mut along = (raw / FREE_STEP).round() * FREE_STEP;
    if len >= width {
        along = along.clamp(width / 2.0, len - width / 2.0);
    }
    let (ext, int) = wall.face_offsets(thickness);
    let across = (cursor - wall.start).dot(n);
    let opens_to = if across > (ext + int) / 2.0 {
        WallSide::Exterior
    } else {
        WallSide::Interior
    };
    let spec = NewOpening {
        opening_type: s.opening_type,
        wall: wall_id,
        along,
        sill: s.sill,
        opens_to,
        hand: s.hand,
        setback: s.setback,
    };
    let problem = model
        .clone()
        .add_opening(spec.clone())
        .err()
        .map(|e| explain(&e));
    let setback = s.setback.unwrap_or((thickness - otype.frame_depth) / 2.0);
    let geometry = OpeningGeometry {
        opening: archisim_core::OpeningId(0),
        wall: wall_id,
        kind: otype.kind,
        origin: wall.start,
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
    };
    Some(Placement {
        before: along - width / 2.0,
        after: len - along - width / 2.0,
        spec,
        problem,
        geometry,
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
}
