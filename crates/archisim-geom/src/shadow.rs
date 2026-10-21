//! Shadows cast on a level's ground by the sun (presentation plans and
//! axonometrics): every solid of the model above the ground thrown along
//! the sun's rays onto it, joined into areas, without the floors of the
//! level (the shadow of a roof on the rooms under it is not drawn).

use archisim_core::{LevelId, Model, Point2};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;

use crate::junctions::{difference, Shape};
use crate::solid::model_solids;

/// Shadows shorter than this are left out (a sun almost overhead would
/// throw every solid onto itself).
const MIN_LENGTH: f64 = 1e-3;

/// The shadow areas on the ground of `level` (its elevation) with the sun
/// towards `sun` (a unit vector, z up).
pub fn shadows(model: &Model, level: LevelId, sun: [f64; 3]) -> Vec<Shape> {
    let Some(z0) = model.level(level).map(|l| l.elevation) else {
        return Vec::new();
    };
    if sun[2] <= 1e-3 {
        return Vec::new();
    }
    let throw = |p: [f64; 3]| {
        let h = (p[2] - z0).max(0.0);
        [p[0] - sun[0] / sun[2] * h, p[1] - sun[1] / sun[2] * h]
    };
    let mut contours: Vec<Vec<[f64; 2]>> = Vec::new();
    for s in model_solids(model) {
        for t in &s.triangles {
            if t.iter().all(|p| p[2] <= z0 + MIN_LENGTH) {
                continue;
            }
            let q = t.map(throw);
            let area = (q[1][0] - q[0][0]) * (q[2][1] - q[0][1])
                - (q[2][0] - q[0][0]) * (q[1][1] - q[0][1]);
            if area.abs() < 1e-8 {
                continue;
            }
            contours.push(if area > 0.0 {
                q.to_vec()
            } else {
                vec![q[0], q[2], q[1]]
            });
        }
    }
    if contours.is_empty() {
        return Vec::new();
    }
    let joined: Vec<Shape> = contours
        .overlay(
            &Vec::<Vec<[f64; 2]>>::new(),
            OverlayRule::Union,
            FillRule::NonZero,
        )
        .into_iter()
        .filter_map(|shape| {
            let mut it = shape.into_iter();
            let to = |c: Vec<[f64; 2]>| c.into_iter().map(|p| Point2::new(p[0], p[1])).collect();
            let outer: Vec<Point2> = crate::ensure_ccw(to(it.next()?));
            (outer.len() >= 3).then(|| (outer, it.map(to).collect()))
        })
        .collect();
    // Not on the floors of the level: inside, the rooms are drawn.
    let floors: Vec<Shape> = model
        .slabs_on_level(level)
        .map(|s| (crate::ensure_ccw(s.outline.clone()), Vec::new()))
        .collect();
    difference(&joined, &floors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{
        CutPattern, Layer, LayerFunction, NewWall, ReferenceLine, WallFunction, WallTop,
    };

    #[test]
    fn a_wall_throws_its_shadow_away_from_the_sun() {
        let mut m = Model::new("Sombra");
        let l = m.add_level("PB", 0.0);
        let mat = m.add_material("Hormigón", "concrete", [180, 180, 180], CutPattern::Solid);
        let t = m
            .add_wall_type(
                "Muro",
                WallFunction::Exterior,
                vec![Layer {
                    material: mat,
                    thickness: 0.2,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        m.add_wall(NewWall {
            wall_type: t,
            level: l,
            start: Point2::new(0.0, 0.0),
            end: Point2::new(10.0, 0.0),
            reference: ReferenceLine::Center,
            height: 3.0,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })
        .unwrap();
        // The sun to the south, 45° high: a 3 m shadow to the north.
        let s = std::f64::consts::FRAC_1_SQRT_2;
        let sh = shadows(&m, l, [0.0, -s, s]);
        assert_eq!(sh.len(), 1);
        let ys: Vec<f64> = sh[0].0.iter().map(|p| p.y).collect();
        let top = ys.iter().cloned().fold(f64::MIN, f64::max);
        let bottom = ys.iter().cloned().fold(f64::MAX, f64::min);
        assert!((top - 3.1).abs() < 1e-6, "{top}");
        assert!((bottom + 0.1).abs() < 1e-6, "{bottom}");
        let area = crate::junctions::area(&sh);
        assert!((area - 32.0).abs() < 0.1, "{area}");
    }
}
