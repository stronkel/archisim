//! How a street meets the ground when its grade does not follow it (as a
//! city builder does): where it runs over the ground, an embankment with
//! its slopes down to it (3 across, 2 down) or, higher up, a viaduct on
//! piers with its deck and parapets; where it runs under the ground, a
//! retaining wall at each side holding it back. Worked out from the
//! street's profile and the ground at its edges, station by station; none
//! of it is stored.

use archisim_core::{MaterialId, Model, Point2, Road, RoadId};

use crate::solid::{prism, Solid, SolidOwner, P3};

/// Spacing of the stations along the street, metres.
pub const STEP: f64 = 4.0;
/// Over the ground by more than this, the street is an embankment.
pub const FILL_FROM: f64 = 0.25;
/// Over the ground by more than this, it is a viaduct.
pub const VIADUCT_FROM: f64 = 6.0;
/// Under the ground by more than this at an edge, a retaining wall.
pub const WALL_FROM: f64 = 0.3;
/// The slope of an embankment: across per metre down.
pub const FILL_SLOPE: f64 = 1.5;
/// Spacing of the piers of a viaduct, metres.
pub const PIER_SPAN: f64 = 24.0;
/// The depth of the deck of a viaduct under the street's pavement.
pub const DECK: f64 = 1.1;

/// What one station of a street sees.
#[derive(Debug, Clone, Copy)]
struct Station {
    left: Point2,
    right: Point2,
    out_left: Point2,
    out_right: Point2,
    /// The top of the street (its sidewalks) and the ground at each edge.
    top: f64,
    ground_left: Option<f64>,
    ground_right: Option<f64>,
    ground_mid: Option<f64>,
}

fn ground_at(model: &Model, r: &Road, p: Point2) -> Option<f64> {
    let base = model.level(r.level).map_or(0.0, |l| l.elevation);
    model
        .terrains_on_level(r.level)
        .find_map(|t| t.height_at(p))
        .map(|z| z + base)
}

fn stations(model: &Model, r: &Road) -> Vec<Station> {
    let base = model.level(r.level).map_or(0.0, |l| l.elevation);
    let len = r.length();
    let n = (len / STEP).ceil().max(1.0) as usize;
    let kh = if r.section.single_level {
        0.0
    } else {
        r.section.kerb_height
    };
    (0..=n)
        .map(|k| {
            let s = len * k as f64 / n as f64;
            let (p, dir) = r.axis_at(s);
            let nrm = dir.perp_left();
            let (l, rr) = r.extent_at(s);
            let left = p + nrm * l;
            let right = p - nrm * rr;
            Station {
                left,
                right,
                out_left: left + nrm,
                out_right: right - nrm,
                top: base + r.top_at(s) + kh,
                ground_left: ground_at(model, r, left),
                ground_right: ground_at(model, r, right),
                ground_mid: ground_at(model, r, p),
            }
        })
        .collect()
}

/// A material of the model of one of these categories, or `or`.
fn material_of(model: &Model, cats: &[&str], or: MaterialId) -> MaterialId {
    cats.iter()
        .find_map(|c| model.materials().find(|m| m.category == *c))
        .map_or(or, |m| m.id)
}

/// The quad `a b c d` as two triangles.
fn quad(out: &mut Vec<[P3; 3]>, a: P3, b: P3, c: P3, d: P3) {
    out.push([a, b, c]);
    out.push([a, c, d]);
}

/// The toe of an embankment from an edge at height `top`, going out along
/// `dir`: where its slope meets the ground (found in a few steps).
fn toe(model: &Model, r: &Road, edge: Point2, dir: archisim_core::Vec2, top: f64) -> (Point2, f64) {
    let mut z = ground_at(model, r, edge).unwrap_or(top - 1.0);
    let mut q = edge;
    for _ in 0..4 {
        let w = ((top - z) * FILL_SLOPE).max(0.0);
        q = edge + dir * w;
        z = ground_at(model, r, q).unwrap_or(z);
    }
    (q, z.min(top))
}

/// The hole a street makes in the ground: only where it runs in it (its
/// stretches over the ground stand on embankments or piers), down to under
/// its pavement where it is lowest there; its bottom over its level.
pub fn road_holes(model: &Model, r: &Road) -> Vec<archisim_core::Excavation> {
    let base = model.level(r.level).map_or(0.0, |l| l.elevation);
    let whole = r.outline();
    let st = stations(model, r);
    let kh = if r.section.single_level {
        0.0
    } else {
        r.section.kerb_height
    };
    let in_ground = |x: &Station| {
        let road = x.top - kh - r.section.depth - FILL_FROM;
        [x.ground_left, x.ground_mid, x.ground_right]
            .iter()
            .any(|g| g.is_none_or(|g| g > road))
    };
    let flags: Vec<bool> = st.iter().map(in_ground).collect();
    if flags.iter().all(|f| *f) || !r.sloped() {
        let low = st.iter().map(|x| x.top - kh).fold(f64::MAX, f64::min);
        return vec![archisim_core::Excavation {
            outline: whole,
            bottom: low - r.section.depth - base,
        }];
    }
    let mut quads: Vec<crate::junctions::Shape> = Vec::new();
    let mut low = f64::MAX;
    for (i, w) in st.windows(2).enumerate() {
        if !(flags[i] || flags[i + 1]) {
            continue;
        }
        let (a, b) = (&w[0], &w[1]);
        low = low.min(a.top - kh).min(b.top - kh);
        // A little past the edges, as the outline of the whole street.
        let poly = crate::ensure_ccw(vec![a.right, b.right, b.left, a.left]);
        quads.push((poly, Vec::new()));
    }
    if quads.is_empty() {
        return Vec::new();
    }
    crate::junctions::union(&quads, &[])
        .into_iter()
        .filter(|(o, _)| o.len() >= 3)
        .map(|(outline, _)| archisim_core::Excavation {
            outline,
            bottom: low - r.section.depth - base,
        })
        .collect()
}

/// What the ground needs round a street: its embankments, viaducts and
/// retaining walls.
pub fn road_ground_solids(model: &Model, id: RoadId) -> Vec<Solid> {
    let Some(r) = model.road(id) else {
        return Vec::new();
    };
    if !r.sloped() || model.terrains_on_level(r.level).next().is_none() {
        return Vec::new();
    }
    let st = stations(model, r);
    if st.len() < 2 {
        return Vec::new();
    }
    let earth = model
        .terrains_on_level(r.level)
        .next()
        .map_or(r.sidewalk_material, |t| t.material);
    let concrete = material_of(model, &["concrete"], r.kerb_material);
    let metal = material_of(model, &["metal", "steel"], concrete);
    let fill_of = |x: &Station, g: Option<f64>| g.map_or(0.0, |g| x.top - g);
    let viaduct = |x: &Station| {
        fill_of(x, x.ground_mid).max(fill_of(x, x.ground_left).min(fill_of(x, x.ground_right)))
            > VIADUCT_FROM
    };
    let mut slopes: Vec<[P3; 3]> = Vec::new();
    let mut walls: Vec<[P3; 3]> = Vec::new();
    let mut deck: Vec<[P3; 3]> = Vec::new();
    let mut rails: Vec<[P3; 3]> = Vec::new();
    let depth = r.section.depth;
    for w in st.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        let high = viaduct(a) || viaduct(b);
        for left in [true, false] {
            let (ea, eb, ga, gb) = if left {
                (a.left, b.left, a.ground_left, b.ground_left)
            } else {
                (a.right, b.right, a.ground_right, b.ground_right)
            };
            let (oa, ob) = if left {
                (a.out_left, b.out_left)
            } else {
                (a.out_right, b.out_right)
            };
            let fa = fill_of(a, ga);
            let fb = fill_of(b, gb);
            if high {
                // A parapet along the deck's edge: a low wall and a rail.
                let (ia, ib) = (ea + (oa - ea) * -0.25, eb + (ob - eb) * -0.25);
                walls.extend(strip_wall(ia, ib, a.top, b.top, 0.25, 0.75));
                rails.extend(strip_wall(ia, ib, a.top + 0.75, b.top + 0.75, 0.06, 0.3));
            } else if fa > FILL_FROM || fb > FILL_FROM {
                // The embankment's slope: from the edge down and out.
                let da = (oa - ea).normalized().unwrap_or_default();
                let db = (ob - eb).normalized().unwrap_or_default();
                let (ta, za) = toe(model, r, ea, da, a.top - depth);
                let (tb, zb) = toe(model, r, eb, db, b.top - depth);
                let pa = [ea.x, ea.y, a.top - 0.02];
                let pb = [eb.x, eb.y, b.top - 0.02];
                let qa = [ta.x, ta.y, za - 0.15];
                let qb = [tb.x, tb.y, zb - 0.15];
                // Facing out and up.
                if left {
                    quad(&mut slopes, pa, qa, qb, pb);
                } else {
                    quad(&mut slopes, pa, pb, qb, qa);
                }
            } else if -fa > WALL_FROM || -fb > WALL_FROM {
                // Held back: a wall from under the street up past the
                // ground at the edge.
                let za = ga.unwrap_or(a.top) + 0.3;
                let zb = gb.unwrap_or(b.top) + 0.3;
                let (o0, o1) = (ea + (oa - ea) * 0.3, eb + (ob - eb) * 0.3);
                walls.extend(between(
                    ea,
                    eb,
                    o1,
                    o0,
                    a.top - depth,
                    b.top - depth,
                    za,
                    zb,
                ));
            }
        }
        if high {
            // The deck under the pavement, between the edges.
            let poly = [a.right, b.right, b.left, a.left];
            deck.extend(between(
                poly[0],
                poly[1],
                poly[2],
                poly[3],
                a.top - depth - DECK,
                b.top - depth - DECK,
                a.top - depth,
                b.top - depth,
            ));
        }
    }
    // The piers of the viaducts: one each span, under the middle.
    let len = r.length();
    let mut s = PIER_SPAN / 2.0;
    while s < len {
        let i = ((s / len) * (st.len() - 1) as f64).round() as usize;
        let x = &st[i.min(st.len() - 1)];
        if viaduct(x) {
            let (p, dir) = r.axis_at(s);
            let nrm = dir.perp_left();
            let half_w = (x.left.distance(x.right) * 0.3).max(1.0);
            let bottom = x.ground_mid.unwrap_or(x.top - 10.0) - 0.5;
            let top = x.top - depth - DECK;
            let poly = [
                p - nrm * half_w - dir * 0.6,
                p - nrm * half_w + dir * 0.6,
                p + nrm * half_w + dir * 0.6,
                p + nrm * half_w - dir * 0.6,
            ];
            deck.extend(prism(&poly, bottom, top));
        }
        s += PIER_SPAN;
    }
    let mut out = Vec::new();
    for (material, triangles) in [
        (earth, slopes),
        (concrete, walls),
        (concrete, deck),
        (metal, rails),
    ] {
        if !triangles.is_empty() {
            out.push(Solid {
                owner: SolidOwner::Road(id),
                material,
                triangles,
            });
        }
    }
    out
}

/// A thin wall standing on the line `a b` (its bottom at `za`, `zb`),
/// `thick` across and `high` tall.
fn strip_wall(a: Point2, b: Point2, za: f64, zb: f64, thick: f64, high: f64) -> Vec<[P3; 3]> {
    let Some(d) = (b - a).normalized() else {
        return Vec::new();
    };
    let n = d.perp_left() * (thick / 2.0);
    between(a - n, b - n, b + n, a + n, za, zb, za + high, zb + high)
}

/// A closed solid over the quad `p0 p1 p2 p3` (p0–p3 at station a, p1–p2
/// at station b), from `za0`/`zb0` up to `za1`/`zb1`.
#[allow(clippy::too_many_arguments)]
fn between(
    p0: Point2,
    p1: Point2,
    p2: Point2,
    p3: Point2,
    za0: f64,
    zb0: f64,
    za1: f64,
    zb1: f64,
) -> Vec<[P3; 3]> {
    let poly = crate::ensure_ccw(vec![p0, p1, p2, p3]);
    // Which of the corners are at station b (p1 and p2).
    let at_b = |q: Point2| q.distance(p1) < 1e-9 || q.distance(p2) < 1e-9;
    crate::solid::draped_prism(&poly, |q| {
        if at_b(q) {
            (zb0, zb1.max(zb0 + 0.01))
        } else {
            (za0, za1.max(za0 + 0.01))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{CutPattern, NewTerrain, RoadSection, RoadStretch, SurveyPoint};

    #[test]
    fn a_street_over_and_under_the_ground() {
        let mut m = Model::new("T");
        let l = m.add_level("Planta baja", 0.0);
        let earth = m.add_material("Tierra", "earth", [120, 100, 80], CutPattern::Solid);
        let conc = m.add_material("Hormigón", "concrete", [180, 180, 175], CutPattern::Solid);
        // A valley 12 m deep in the middle, between two hills.
        let sp = |x: f64, y: f64, z: f64| SurveyPoint {
            at: Point2::new(x, y),
            z,
        };
        let mut pts = Vec::new();
        for x in [-10.0, 40.0, 60.0, 80.0, 130.0] {
            let z = if x == 60.0 {
                -12.0
            } else if x == 40.0 || x == 80.0 {
                -6.0
            } else {
                3.0
            };
            pts.push(sp(x, -40.0, z));
            pts.push(sp(x, 40.0, z));
        }
        m.add_terrain(NewTerrain {
            level: l,
            points: pts,
            material: earth,
            depth: 1.0,
            contour: 1.0,
            lines: Vec::new(),
        })
        .unwrap();
        let id = m
            .add_road_stretch(RoadStretch {
                level: l,
                points: vec![Point2::new(0.0, 0.0), Point2::new(120.0, 0.0)],
                continues: None,
                section: RoadSection::default(),
                top_offset: 0.0,
                pavement: conc,
                kerb_material: conc,
                sidewalk_material: conc,
                // Nearly level: over the valley, in the hills.
                follow_ground: Some(0.02),
            })
            .unwrap();
        let s = road_ground_solids(&m, id);
        let by = |mat: MaterialId| -> usize {
            s.iter()
                .filter(|x| x.material == mat)
                .map(|x| x.triangles.len())
                .sum()
        };
        // Concrete: deck, piers, parapets and walls; earth: slopes.
        assert!(by(conc) > 0, "no concrete");
        assert!(by(earth) > 0, "no embankment");
        // Piers reach down near the valley's floor.
        let low = s
            .iter()
            .flat_map(|x| x.triangles.iter().flatten())
            .map(|p| p[2])
            .fold(f64::MAX, f64::min);
        assert!(low < -8.0, "{low}");
        // The solid of the street carries them.
        let all = crate::solid::road_solids(&m, id);
        assert!(all.iter().any(|x| x.material == earth));
    }
}
