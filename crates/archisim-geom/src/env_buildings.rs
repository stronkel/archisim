//! The buildings of the surroundings in 3D (U6): a dressed mass made into
//! its body, its windows floor by floor along each free façade, its
//! grilles, balconies or terraces, its shop front, plinth and cornice, and
//! its roof — flat with a parapet, or tiled over the straight skeleton of
//! its contour, with gables on the party walls. Nothing of this is stored:
//! it is made from the mass and its dress each time.

use std::collections::BTreeMap;

use archisim_core::{
    offset_contour, signed_area, skeleton, BuildingStyle, Closure, Dress, EnvRoof, Mass, MassShape,
    MaterialId, Model, Point2, Vec2,
};

use crate::solid::{draped_prism, prism, triangulate, Solid, SolidOwner, P3};

/// The parapet of a flat roof: height and thickness.
const PARAPET: (f64, f64) = (0.9, 0.25);
/// How far the window frames and glass stand out of the wall face.
const FRAME_OUT: f64 = 0.03;
const GLASS_OUT: f64 = 0.045;
/// The frame round a window.
const FRAME: f64 = 0.07;
/// Shortest façade that gets windows, and the wall left at its ends.
const MIN_FACADE: f64 = 1.8;
const END_WALL: f64 = 0.35;
/// The spacing of the bars of grilles and railings.
const BARS: f64 = 0.13;
/// The tiles' thickness over the roof planes.
const TILES: f64 = 0.06;

/// One side of the contour, seen from outside.
#[derive(Clone, Copy)]
struct Side {
    a: Point2,
    u: Vec2,
    out: Vec2,
    len: f64,
}

impl Side {
    fn at(&self, s: f64, t: f64) -> Point2 {
        self.a + self.u * s + self.out * t
    }

    /// A box on the façade: `s` along it, `t` out of it, `z` up.
    fn block(&self, s: (f64, f64), t: (f64, f64), z: (f64, f64)) -> Vec<[P3; 3]> {
        if s.1 - s.0 <= 1e-6 || t.1 - t.0 <= 1e-6 {
            return Vec::new();
        }
        let q = [
            self.at(s.0, t.0),
            self.at(s.1, t.0),
            self.at(s.1, t.1),
            self.at(s.0, t.1),
        ];
        prism(&q, z.0, z.1)
    }
}

/// What a dressed building gives, by material.
#[derive(Default)]
struct Out {
    by: BTreeMap<MaterialId, Vec<[P3; 3]>>,
}

impl Out {
    fn add(&mut self, m: MaterialId, tris: Vec<[P3; 3]>) {
        if !tris.is_empty() {
            self.by.entry(m).or_default().extend(tris);
        }
    }
}

/// The solids of a dressed mass: its body, façades and roof, one solid per
/// material. Empty if the mass is not a dressed contour.
pub fn dressed_solids(model: &Model, m: &Mass) -> Vec<Solid> {
    let (Some(d), MassShape::Prism(poly)) = (m.dress.as_ref(), &m.shape) else {
        return Vec::new();
    };
    if poly.len() < 3 || m.height <= 0.0 {
        return Vec::new();
    }
    let mut poly = poly.clone();
    if signed_area(&poly) < 0.0 {
        poly.reverse();
    }
    let z0 = model.level(m.level).map_or(0.0, |l| l.elevation) + m.base_offset;
    let floors = m.floors().max(1);
    let fh = m.floor_height.unwrap_or(m.height / floors as f64);
    let top = z0 + m.height;
    let party = model.party_sides(m.id);
    let n = poly.len();
    let sides: Vec<Side> = (0..n)
        .filter_map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            let u = (b - a).normalized()?;
            Some(Side {
                a,
                u,
                out: -u.perp_left(),
                len: a.distance(b),
            })
        })
        .collect();
    if sides.len() != n {
        return Vec::new();
    }
    let party = |i: usize| party.get(i).copied().unwrap_or(false);
    let mat = d.materials;
    let mut out = Out::default();

    // The body, or the wings round the courtyard.
    let court = (d.patio > 0.0)
        .then(|| offset_contour(&poly, -d.patio))
        .flatten()
        .filter(|q| q.len() == n && signed_area(q) > 4.0);
    match &court {
        Some(inner) => {
            for i in 0..n {
                let j = (i + 1) % n;
                let quad = [poly[i], poly[j], inner[j], inner[i]];
                out.add(mat.wall, prism(&quad, z0, top));
            }
            // The courtyard's floor, paved.
            out.add(mat.trim, prism(inner, z0, z0 + 0.02));
        }
        None => out.add(mat.wall, prism(&poly, z0, top)),
    }

    // The façades; the shops and the big door on those to the street.
    let longest_free = (0..n)
        .filter(|i| !party(*i))
        .max_by(|a, b| sides[*a].len.total_cmp(&sides[*b].len));
    let street = model.street_sides(m.id);
    let on_street = |i: usize| street.get(i).copied().unwrap_or(false);
    let door_side = (0..n)
        .filter(|i| on_street(*i))
        .max_by(|a, b| sides[*a].len.total_cmp(&sides[*b].len));
    for (i, side) in sides.iter().enumerate() {
        if party(i) {
            continue;
        }
        let prev_party = party((i + n - 1) % n);
        let next_party = party((i + 1) % n);
        facade(
            &mut out,
            d,
            side,
            (z0, top, fh, floors),
            (prev_party, next_party),
            (on_street(i), door_side == Some(i)),
        );
    }
    // Courtyard façades: windows looking in.
    if let Some(inner) = &court {
        for i in 0..n {
            let (a, b) = (inner[(i + 1) % n], inner[i]);
            let Some(u) = (b - a).normalized() else {
                continue;
            };
            let side = Side {
                a,
                u,
                out: -u.perp_left(),
                len: a.distance(b),
            };
            let plain = Dress {
                closure: Closure::None,
                shop: false,
                plinth: 0.0,
                cornice: false,
                ..*d
            };
            facade(
                &mut out,
                &plain,
                &side,
                (z0, top, fh, floors),
                (true, true),
                (false, false),
            );
        }
    }

    // The roof.
    let tiled = match d.roof {
        EnvRoof::Flat => false,
        EnvRoof::Gable | EnvRoof::Hip if court.is_some() => false,
        EnvRoof::Gable | EnvRoof::Hip => {
            tiled_roof(&mut out, d, &poly, &sides, &|i| party(i), top, longest_free)
        }
    };
    if !tiled {
        flat_roof(&mut out, d, &poly, &sides, court.as_deref(), top);
    }

    out.by
        .into_iter()
        .map(|(material, triangles)| Solid {
            owner: SolidOwner::Mass(m.id),
            material,
            triangles,
        })
        .collect()
}

/// A free façade: plinth, cornice and, floor by floor, its windows with
/// what closes them.
fn facade(
    out: &mut Out,
    d: &Dress,
    side: &Side,
    (z0, top, fh, floors): (f64, f64, f64, usize),
    (prev_party, next_party): (bool, bool),
    (street, main): (bool, bool),
) {
    let mat = d.materials;
    let len = side.len;
    // Past the corner where the next façade is free too, so that the
    // bands turn it.
    let ends = |reach: f64| {
        (
            if prev_party { 0.0 } else { -reach },
            if next_party { len } else { len + reach },
        )
    };
    if d.plinth > 0.0 {
        out.add(
            mat.trim,
            side.block(ends(0.03), (0.0, 0.03), (z0, z0 + d.plinth.min(top - z0))),
        );
    }
    if d.cornice {
        let depth = if d.style == BuildingStyle::HistoricCentre {
            0.45
        } else {
            0.25
        };
        out.add(
            mat.trim,
            side.block(ends(depth), (0.0, depth), (top - 0.3, top)),
        );
        out.add(
            mat.trim,
            side.block(
                ends(depth * 0.5),
                (0.0, depth * 0.5),
                (top - 0.45, top - 0.3),
            ),
        );
    }
    if len < MIN_FACADE {
        return;
    }
    if d.closure == Closure::CurtainWall {
        curtain(out, d, side, (z0, top, fh, floors));
        return;
    }
    let bays = window_axes(len, d.bay, d.window_width);
    for f in 0..floors {
        let floor = z0 + f as f64 * fh;
        if f == 0 && d.shop && street {
            if d.style == BuildingStyle::Warehouse {
                if main {
                    // The big door, in the middle.
                    let w = (len - 2.0 * END_WALL).min(5.0);
                    let h = (fh - 0.6).min(5.0);
                    let c = len / 2.0;
                    out.add(
                        mat.joinery,
                        side.block((c - w / 2.0, c + w / 2.0), (0.0, 0.05), (floor, floor + h)),
                    );
                    for k in 1..((h / 0.5) as usize) {
                        let z = floor + k as f64 * 0.5;
                        out.add(
                            mat.metal,
                            side.block((c - w / 2.0, c + w / 2.0), (0.05, 0.07), (z - 0.02, z)),
                        );
                    }
                }
            } else {
                shop_front(out, d, side, floor, fh);
                continue;
            }
        }
        let balcony = f > 0 && matches!(d.closure, Closure::Balconies | Closure::Terraces);
        let (sill, h) = if balcony {
            (0.0, d.window_height.min(fh - 0.3))
        } else {
            let h = d.window_height.min(fh - 0.3);
            (d.sill.min(fh - h - 0.1).max(0.0), h)
        };
        if d.closure == Closure::Terraces && f > 0 {
            terrace(out, d, side, floor, ends(0.0));
        }
        for c in &bays {
            let s = (c - d.window_width / 2.0, c + d.window_width / 2.0);
            window(out, d, side, s, floor + sill, h, sill > 0.0);
            if d.closure == Closure::Grilles {
                grille(out, d, side, s, (floor, floor + sill), h);
            }
            if d.closure == Closure::Balconies && f > 0 {
                balcony_on(out, d, side, s, floor);
            }
        }
    }
}

/// The axes of the windows along a façade: as many bays as fit, centred.
fn window_axes(len: f64, bay: f64, width: f64) -> Vec<f64> {
    let room = len - 2.0 * END_WALL;
    if room < width {
        return Vec::new();
    }
    let k = (((room - width) / bay).floor() as usize + 1).max(1);
    let first = len / 2.0 - (k as f64 - 1.0) * bay / 2.0;
    (0..k).map(|j| first + j as f64 * bay).collect()
}

/// A window: its frame and its glass, with a sill if it has one.
fn window(out: &mut Out, d: &Dress, side: &Side, s: (f64, f64), z: f64, h: f64, sill: bool) {
    let mat = d.materials;
    out.add(
        mat.joinery,
        side.block(
            (s.0 - FRAME, s.1 + FRAME),
            (0.0, FRAME_OUT),
            (if sill { z - FRAME } else { z }, z + h + FRAME),
        ),
    );
    out.add(
        mat.glass,
        side.block((s.0, s.1), (FRAME_OUT, GLASS_OUT), (z, z + h)),
    );
    // A transom across the tall ones, a mullion down the wide ones.
    if h > 1.8 {
        let zt = z + h - 0.45;
        out.add(
            mat.joinery,
            side.block(
                (s.0, s.1),
                (FRAME_OUT, GLASS_OUT + 0.01),
                (zt - 0.03, zt + 0.03),
            ),
        );
    }
    if s.1 - s.0 > 0.9 {
        let c = (s.0 + s.1) / 2.0;
        out.add(
            mat.joinery,
            side.block(
                (c - 0.03, c + 0.03),
                (FRAME_OUT, GLASS_OUT + 0.01),
                (z, z + h),
            ),
        );
    }
    if sill {
        out.add(
            mat.trim,
            side.block(
                (s.0 - 0.06, s.1 + 0.06),
                (0.0, 0.09),
                (z - FRAME - 0.05, z - FRAME),
            ),
        );
    }
}

/// A window grille, standing out of the wall.
fn grille(out: &mut Out, d: &Dress, side: &Side, s: (f64, f64), (floor, z): (f64, f64), h: f64) {
    let m = d.materials.metal;
    let (t0, t1) = (0.12, 0.14);
    let (lo, hi) = ((z - 0.1).max(floor), z + h + 0.1);
    let k = (((s.1 - s.0) / BARS).round() as usize).max(2);
    for j in 0..=k {
        let x = s.0 + (s.1 - s.0) * j as f64 / k as f64;
        out.add(m, side.block((x - 0.01, x + 0.01), (t0, t1), (lo, hi)));
    }
    for zb in [lo, z + h * 0.5, hi - 0.02] {
        out.add(
            m,
            side.block(
                (s.0 - 0.02, s.1 + 0.02),
                (t0 - 0.01, t1 + 0.01),
                (zb, zb + 0.025),
            ),
        );
    }
    // The arms that hold it to the wall.
    for x in [s.0, s.1] {
        for zb in [lo, hi - 0.02] {
            out.add(
                m,
                side.block((x - 0.012, x + 0.012), (0.0, t0), (zb, zb + 0.02)),
            );
        }
    }
}

/// A balcony in front of a balcony door: its slab and its railing.
fn balcony_on(out: &mut Out, d: &Dress, side: &Side, s: (f64, f64), floor: f64) {
    let depth = d.balcony_depth.max(0.2);
    let s = (s.0 - 0.3, s.1 + 0.3);
    out.add(
        d.materials.trim,
        side.block(s, (0.0, depth), (floor - 0.12, floor)),
    );
    railing(out, d.materials.metal, side, s, depth, floor, (true, true));
}

/// A railing round the front of a slab `depth` out, with its sides.
fn railing(
    out: &mut Out,
    m: MaterialId,
    side: &Side,
    s: (f64, f64),
    depth: f64,
    floor: f64,
    (left, right): (bool, bool),
) {
    let (h, b) = (1.0, 0.016);
    let t = (depth - 0.04, depth - 0.04 + 2.0 * b);
    out.add(
        m,
        side.block(s, (t.0 - 0.01, t.1 + 0.01), (floor + h - 0.04, floor + h)),
    );
    out.add(m, side.block(s, t, (floor + 0.08, floor + 0.11)));
    let k = (((s.1 - s.0) / BARS).round() as usize).max(2);
    for j in 0..=k {
        let x = s.0 + 0.02 + (s.1 - s.0 - 0.04) * j as f64 / k as f64;
        out.add(m, side.block((x - b, x + b), t, (floor, floor + h)));
    }
    for (on, x) in [(left, s.0 + 0.02), (right, s.1 - 0.02)] {
        if !on {
            continue;
        }
        out.add(
            m,
            side.block((x - b, x + b), (0.0, t.1), (floor + h - 0.04, floor + h)),
        );
        let k = ((depth / BARS).round() as usize).max(1);
        for j in 0..k {
            let y = depth * j as f64 / k as f64;
            out.add(
                m,
                side.block((x - b, x + b), (y, y + 2.0 * b), (floor, floor + h)),
            );
        }
    }
}

/// A terrace along a whole façade: its slab and its parapet.
fn terrace(out: &mut Out, d: &Dress, side: &Side, floor: f64, s: (f64, f64)) {
    let depth = d.balcony_depth.max(0.4);
    let mat = d.materials;
    out.add(mat.trim, side.block(s, (0.0, depth), (floor - 0.15, floor)));
    if d.style == BuildingStyle::Block1960s {
        // A brick parapet, and the rail over it.
        out.add(
            mat.wall,
            side.block(s, (depth - 0.12, depth), (floor, floor + 0.85)),
        );
        out.add(
            mat.metal,
            side.block(s, (depth - 0.1, depth - 0.02), (floor + 0.85, floor + 1.0)),
        );
    } else if d.style == BuildingStyle::CoastalHotel {
        // A glass balustrade with a steel handrail.
        out.add(
            mat.metal,
            side.block(s, (depth - 0.03, depth - 0.01), (floor, floor + 1.0)),
        );
        out.add(
            mat.joinery,
            side.block(s, (depth - 0.05, depth + 0.01), (floor + 1.0, floor + 1.05)),
        );
    } else {
        railing(out, mat.metal, side, s, depth, floor, (false, false));
    }
}

/// A shop front on the ground floor: wide windows and the fascia above.
fn shop_front(out: &mut Out, d: &Dress, side: &Side, floor: f64, fh: f64) {
    let mat = d.materials;
    let len = side.len;
    let h = (fh - 0.9).clamp(1.8, 3.2);
    let bay = d.bay.max(2.4);
    let k = (((len - 2.0 * END_WALL) / bay).floor() as usize).max(1);
    let w = (len - 2.0 * END_WALL) / k as f64;
    for j in 0..k {
        let s0 = END_WALL + j as f64 * w + 0.15;
        let s1 = END_WALL + (j + 1) as f64 * w - 0.15;
        if s1 - s0 < 0.8 {
            continue;
        }
        out.add(
            mat.joinery,
            side.block(
                (s0 - 0.06, s1 + 0.06),
                (0.0, FRAME_OUT),
                (floor + 0.08, floor + h + 0.06),
            ),
        );
        out.add(
            mat.glass,
            side.block((s0, s1), (FRAME_OUT, GLASS_OUT), (floor + 0.15, floor + h)),
        );
    }
    // The fascia, for the sign.
    out.add(
        mat.trim,
        side.block((0.0, len), (0.0, 0.08), (floor + h + 0.12, floor + h + 0.6)),
    );
}

/// A curtain wall over the whole façade: glass, spandrels at the floors
/// and mullions at each bay.
fn curtain(out: &mut Out, d: &Dress, side: &Side, (z0, top, fh, floors): (f64, f64, f64, usize)) {
    let mat = d.materials;
    let len = side.len;
    out.add(
        mat.glass,
        side.block((0.0, len), (0.0, 0.04), (z0, top - 0.6)),
    );
    for f in 1..=floors {
        let z = z0 + f as f64 * fh;
        let lo = (z - 0.75).max(z0);
        let hi = if f == floors { top } else { z + 0.15 };
        out.add(mat.wall, side.block((0.0, len), (0.0, 0.06), (lo, hi)));
    }
    let k = ((len / d.bay).round() as usize).max(1);
    for j in 0..=k {
        let x = (len * j as f64 / k as f64).clamp(0.03, len - 0.03);
        out.add(
            mat.joinery,
            side.block((x - 0.03, x + 0.03), (0.0, 0.14), (z0, top - 0.6)),
        );
    }
}

/// A flat roof: its finish, the parapet round it.
fn flat_roof(
    out: &mut Out,
    d: &Dress,
    poly: &[Point2],
    sides: &[Side],
    court: Option<&[Point2]>,
    top: f64,
) {
    let mat = d.materials;
    let (h, t) = PARAPET;
    for side in sides {
        out.add(
            mat.wall,
            side.block((0.0, side.len), (-t, 0.0), (top, top + h)),
        );
        out.add(
            mat.trim,
            side.block(
                (-0.03, side.len + 0.03),
                (-t - 0.03, 0.03),
                (top + h, top + h + 0.05),
            ),
        );
    }
    let n = poly.len();
    match court {
        Some(inner) => {
            for i in 0..n {
                let j = (i + 1) % n;
                let quad = [poly[i], poly[j], inner[j], inner[i]];
                out.add(mat.roof, prism(&quad, top, top + 0.03));
            }
        }
        None => {
            let finish = offset_contour(poly, -t).unwrap_or_else(|| poly.to_vec());
            out.add(mat.roof, prism(&finish, top, top + 0.03));
        }
    }
}

/// A tiled roof over the straight skeleton of the contour: the free sides
/// fall (all of them for a hipped roof, the long ones for a gabled one),
/// the others stand as gables. False if it cannot be made (then the roof
/// is flat).
fn tiled_roof(
    out: &mut Out,
    d: &Dress,
    poly: &[Point2],
    sides: &[Side],
    party: &dyn Fn(usize) -> bool,
    top: f64,
    longest_free: Option<usize>,
) -> bool {
    let n = poly.len();
    // The ridge runs along the street: between party walls, square to
    // them; free-standing, along the longest side.
    let along = (0..n)
        .find(|i| party(*i))
        .map(|i| sides[i].u.perp_left())
        .or(longest_free.map(|i| sides[i].u));
    let speeds: Vec<f64> = (0..n)
        .map(|i| {
            if party(i) {
                return 0.0;
            }
            match (d.roof, along) {
                (EnvRoof::Gable, Some(u)) if sides[i].u.dot(u).abs() < 0.5 => 0.0,
                _ => 1.0,
            }
        })
        .collect();
    if speeds.iter().all(|s| *s <= 0.0) {
        return false;
    }
    let Some(faces) = skeleton::faces(poly, &speeds) else {
        return false;
    };
    let pitch = d.pitch.max(0.05);
    // The height over the eaves of a point of side i's face.
    let rise = |i: usize, q: Point2| ((q - sides[i].a).dot(-sides[i].out)).max(0.0) * pitch;
    let mat = d.materials;
    let mut on_line: Vec<(usize, Point2, f64)> = Vec::new();
    for (i, face) in faces.iter().enumerate() {
        if face.len() < 3 {
            continue;
        }
        out.add(
            mat.roof,
            draped_prism(face, |q| {
                let z = top + rise(i, q);
                (z, z + TILES)
            }),
        );
        for q in face {
            on_line.push((i, *q, rise(i, *q)));
        }
    }
    // The eaves' fascia on the falling sides.
    for (i, side) in sides.iter().enumerate() {
        if speeds[i] > 0.0 {
            out.add(
                mat.trim,
                side.block((0.0, side.len), (0.0, 0.05), (top - 0.12, top + 0.02)),
            );
        }
    }
    // The gables: the wall up to the roof over each side that does not
    // fall.
    for (i, side) in sides.iter().enumerate() {
        if speeds[i] > 0.0 {
            continue;
        }
        let mut pts: Vec<(f64, f64)> = on_line
            .iter()
            .filter(|(_, q, _)| (*q - side.a).dot(side.out).abs() < 1e-6)
            .map(|(_, q, h)| ((*q - side.a).dot(side.u), *h))
            .filter(|(s, _)| *s > 1e-6 && *s < side.len - 1e-6)
            .collect();
        pts.sort_by(|a, b| b.0.total_cmp(&a.0));
        pts.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-6);
        if pts.iter().all(|(_, h)| *h <= 1e-6) {
            continue;
        }
        // In the plane of the façade: (s, z) counter-clockwise seen from
        // outside.
        let mut outline = vec![Point2::new(0.0, 0.0), Point2::new(side.len, 0.0)];
        outline.extend(pts.iter().map(|(s, h)| Point2::new(*s, *h + TILES)));
        let world = |p: Point2| -> P3 {
            let q = side.at(p.x, 0.0);
            [q.x, q.y, top + p.y]
        };
        let mut tris = Vec::new();
        for [a, b, c] in triangulate(&outline) {
            let (a, b, c) = (world(outline[a]), world(outline[b]), world(outline[c]));
            tris.push([a, b, c]);
            tris.push([a, c, b]);
        }
        out.add(mat.wall, tris);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::NewEnvBuilding;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point2> {
        vec![
            Point2::new(x0, y0),
            Point2::new(x1, y0),
            Point2::new(x1, y1),
            Point2::new(x0, y1),
        ]
    }

    fn bounds(s: &[Solid]) -> ([f64; 3], [f64; 3]) {
        let mut lo = [f64::MAX; 3];
        let mut hi = [f64::MIN; 3];
        for t in s.iter().flat_map(|s| &s.triangles).flatten() {
            for k in 0..3 {
                lo[k] = lo[k].min(t[k]);
                hi[k] = hi[k].max(t[k]);
            }
        }
        (lo, hi)
    }

    #[test]
    fn every_style_dresses_a_mass() {
        let mut m = Model::new("T");
        let l = m.add_level("Planta baja", 0.0);
        for (i, style) in BuildingStyle::ALL.into_iter().enumerate() {
            let x = i as f64 * 40.0;
            let id = m
                .add_env_building(NewEnvBuilding {
                    name: style.name().into(),
                    level: l,
                    outline: rect(x, 0.0, x + 16.0, 12.0),
                    style,
                    floors: None,
                    floor_height: None,
                    base_offset: 0.0,
                })
                .unwrap();
            let mass = m.mass(id).unwrap();
            let s = dressed_solids(&m, mass);
            let tris: usize = s.iter().map(|s| s.triangles.len()).sum();
            assert!(tris > 200, "{}: {tris}", style.name());
            let (lo, hi) = bounds(&s);
            // On the ground, and up to the top (and the roof over it).
            assert!(lo[2].abs() < 1e-9, "{}", style.name());
            assert!(hi[2] >= mass.height, "{}", style.name());
            assert!(
                hi[2] < mass.height + 16.0 * 0.5 + 1.5,
                "{}: {}",
                style.name(),
                hi[2]
            );
            // Nothing far out of the contour (balconies, cornices).
            assert!(lo[0] > x - 2.0 && hi[0] < x + 18.0, "{}", style.name());
            // It goes through the plain mass solids too.
            assert_eq!(crate::solid::mass_solids(&m, id).len(), s.len());
        }
    }

    #[test]
    fn a_house_between_party_walls_is_blind_on_them() {
        let mut m = Model::new("T");
        let l = m.add_level("Planta baja", 0.0);
        let mut ids = Vec::new();
        for i in 0..3 {
            let x = i as f64 * 6.0;
            ids.push(
                m.add_env_building(NewEnvBuilding {
                    name: format!("Casa {i}"),
                    level: l,
                    outline: rect(x, 0.0, x + 6.0, 12.0),
                    style: BuildingStyle::Andalusian,
                    floors: None,
                    floor_height: None,
                    base_offset: 0.0,
                })
                .unwrap(),
            );
        }
        let mid = m.mass(ids[1]).unwrap();
        let glass = mid.dress.unwrap().materials.glass;
        let s = dressed_solids(&m, mid);
        // Windows only on the street and the back: none out of x 6..12.
        for t in s
            .iter()
            .filter(|s| s.material == glass)
            .flat_map(|s| &s.triangles)
            .flatten()
        {
            assert!(t[0] > 6.0 + 0.2 && t[0] < 12.0 - 0.2, "{t:?}");
        }
        // Its gabled roof rises to the ridge in the middle of the depth.
        let (_, hi) = bounds(&s);
        let ridge = 6.0 + 6.0 * 0.4;
        assert!((hi[2] - (ridge + TILES)).abs() < 0.05, "{}", hi[2]);
    }
}
