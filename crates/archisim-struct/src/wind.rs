//! The wind on the building (DB-SE-AE 3.3 and annex D): the global force
//! of each storey for each direction, from the facades facing the wind
//! and the slenderness of the building, and the local pressure on each
//! exterior wall for its own members.

use archisim_core::{Model, Point2, WallFunction, WallId};

use crate::actions::{exposure, facade_coefficients};
use crate::cases::WindDir;
use crate::geo;

/// The wind force on a storey band in one direction.
#[derive(Debug, Clone, PartialEq)]
pub struct StoreyWind {
    pub dir: WindDir,
    /// The band, heights above project zero.
    pub from: f64,
    pub to: f64,
    /// Width of facade facing the wind, m.
    pub width: f64,
    /// Exposure coefficient at the middle of the band.
    pub exposure: f64,
    /// Pressure coefficients, windward and leeward.
    pub cp: (f64, f64),
    /// The force, kN, and where it acts in plan (the middle of the
    /// facade).
    pub force: f64,
    pub at: Point2,
}

/// The building as the wind sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildingWind {
    /// Lowest base and highest top of the exterior walls.
    pub ground: f64,
    pub height: f64,
    /// Extent in plan: min and max corners.
    pub lo: Point2,
    pub hi: Point2,
    pub storeys: Vec<StoreyWind>,
}

fn exterior(model: &Model) -> Vec<(WallId, Vec<Point2>, f64, f64)> {
    model
        .walls()
        .filter(|w| {
            model
                .wall_type(w.wall_type)
                .is_some_and(|t| t.function == WallFunction::Exterior)
        })
        .map(|w| {
            let base = model.level(w.level).map_or(0.0, |l| l.elevation) + w.base_offset;
            (w.id, geo::wall_path(w), base, base + w.height)
        })
        .collect()
}

/// The wind on the whole building, storey by storey.
pub fn building_wind(model: &Model) -> Option<BuildingWind> {
    let site = model.structural_site();
    let walls = exterior(model);
    if walls.is_empty() {
        return None;
    }
    let ground = walls.iter().map(|w| w.2).fold(f64::INFINITY, f64::min);
    let top = walls.iter().map(|w| w.3).fold(f64::NEG_INFINITY, f64::max);
    let pts: Vec<Point2> = walls.iter().flat_map(|w| w.1.iter().copied()).collect();
    let (lo, hi) = geo::bounds(&pts);
    let height = top - ground;
    // Bands: between the levels inside the building's height.
    let mut cuts: Vec<f64> = model
        .levels()
        .iter()
        .map(|l| l.elevation)
        .filter(|z| *z > ground + 0.1 && *z < top - 0.1)
        .collect();
    cuts.push(ground);
    cuts.push(top);
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() < 0.1);
    let mut storeys = Vec::new();
    for dir in WindDir::ALL {
        let dx = dir.vector()[0];
        let depth = if dx != 0.0 { hi.x - lo.x } else { hi.y - lo.y };
        let cp = facade_coefficients(if depth > 0.0 { height / depth } else { 1.0 });
        for band in cuts.windows(2) {
            let (z0, z1) = (band[0], band[1]);
            // The width of the facades in the band, across the wind.
            let in_band: Vec<Point2> = walls
                .iter()
                .filter(|w| w.2 < z1 - 0.05 && w.3 > z0 + 0.05)
                .flat_map(|w| w.1.iter().copied())
                .collect();
            if in_band.is_empty() {
                continue;
            }
            let (blo, bhi) = geo::bounds(&in_band);
            let width = if dx != 0.0 {
                bhi.y - blo.y
            } else {
                bhi.x - blo.x
            };
            let z_mid = (z0 + z1) / 2.0 - ground;
            let ce = exposure(site.roughness, z_mid.max(0.0));
            let q = site.wind_zone.basic_pressure() * ce;
            let force = q * (cp.0 - cp.1) * width * (z1 - z0);
            let at = Point2::new((blo.x + bhi.x) / 2.0, (blo.y + bhi.y) / 2.0);
            storeys.push(StoreyWind {
                dir,
                from: z0,
                to: z1,
                width,
                exposure: ce,
                cp,
                force,
                at,
            });
        }
    }
    Some(BuildingWind {
        ground,
        height,
        lo,
        hi,
        storeys,
    })
}

/// The pressure on an exterior wall for its own members (studs, mullions,
/// panels): pressure at its top, with the coefficient of small areas (1 m²)
/// of table D.3 (windward 1,0) and, within a tenth of the building's
/// width from a corner, the suction of zone A (−1,4), with ±0,2 of inner
/// pressure; kN/m². Interior walls take none.
pub fn wall_pressure(model: &Model, id: WallId, building: Option<&BuildingWind>) -> f64 {
    let Some(w) = model.wall(id) else {
        return 0.0;
    };
    if !model
        .wall_type(w.wall_type)
        .is_some_and(|t| t.function == WallFunction::Exterior)
    {
        return 0.0;
    }
    let site = model.structural_site();
    let base = model.level(w.level).map_or(0.0, |l| l.elevation) + w.base_offset;
    let ground = building.map_or(base, |b| b.ground);
    let q = site.wind_zone.basic_pressure() * exposure(site.roughness, base + w.height - ground);
    let corner = building.is_some_and(|b| {
        let e = (b.hi.x - b.lo.x).min(b.hi.y - b.lo.y).min(2.0 * b.height) / 10.0;
        let near = |p: Point2| {
            [
                Point2::new(b.lo.x, b.lo.y),
                Point2::new(b.hi.x, b.lo.y),
                Point2::new(b.hi.x, b.hi.y),
                Point2::new(b.lo.x, b.hi.y),
            ]
            .iter()
            .any(|c| c.distance(p) <= e + 0.5)
        };
        near(w.start) || near(w.end)
    });
    let cp = if corner { 1.4 } else { 1.0 };
    q * (cp + 0.2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::*;

    #[test]
    fn a_house_feels_the_wind() {
        let mut m = Model::new("Viento");
        let l0 = m.add_level("Planta baja", 0.0);
        m.add_level("Planta 1", 3.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        let pts = [(0.0, 0.0), (10.0, 0.0), (10.0, 8.0), (0.0, 8.0)];
        for i in 0..4 {
            let (a, b) = (pts[i], pts[(i + 1) % 4]);
            m.add_wall(NewWall {
                wall_type: lib.facade,
                level: l0,
                start: Point2::new(a.0, a.1),
                end: Point2::new(b.0, b.1),
                bulge: 0.0,
                reference: ReferenceLine::Center,
                height: 6.0,
                base_offset: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        }
        let b = building_wind(&m).unwrap();
        // Two bands (0–3, 3–6) in four directions.
        assert_eq!(b.storeys.len(), 8);
        let x = b
            .storeys
            .iter()
            .filter(|s| s.dir == WindDir::PosX)
            .map(|s| s.force)
            .sum::<f64>();
        let y = b
            .storeys
            .iter()
            .filter(|s| s.dir == WindDir::PosY)
            .map(|s| s.force)
            .sum::<f64>();
        // The wind along X meets the 8 m facades; along Y, the 10 m ones.
        assert!(x > 0.0 && y > x, "{x} {y}");
        // Roughness IV under 5 m: ce = 0,619 · (0,619 + 1,54) ≈ 1,34.
        let low = b.storeys.iter().find(|s| s.dir == WindDir::PosX).unwrap();
        assert!(low.exposure > 1.0 && low.exposure < 1.6, "{}", low.exposure);
        let w = m.walls().next().unwrap().id;
        assert!(wall_pressure(&m, w, Some(&b)) > 0.5);
    }
}
