//! The buildings of the surroundings in 3D (U6): a dressed mass made into
//! its body, its windows floor by floor along each free façade, its
//! grilles, balconies or terraces, its shop front, plinth and cornice, and
//! its roof — flat with a parapet, or tiled over the straight skeleton of
//! its contour, with gables on the party walls. Nothing of this is stored:
//! it is made from the mass and its dress each time.

use std::collections::BTreeMap;

use archisim_core::{
    offset_contour, signed_area, BuildingStyle, Closure, Dress, EnvRoof, Mass, MassShape,
    MaterialId, Model, Point2, Vec2,
};

use crate::solid::{draped_prism, prism, Solid, SolidOwner, P3};

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
/// The highest a tiled roof climbs over its eaves; past it, flat.
const MAX_RIDGE: f64 = 4.5;
/// Deeper than this from the street, a house between party walls has its
/// slope only over the first bay (`FRONT_BAY`), a flat roof behind.
const DEEP_HOUSE: f64 = 12.0;
const FRONT_BAY: f64 = 5.0;

/// The corners of the box round a contour.
fn bounds_2d(poly: &[Point2]) -> (Point2, Point2) {
    let mut lo = poly[0];
    let mut hi = poly[0];
    for p in poly {
        lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    (lo, hi)
}

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

/// Up to this many dressed buildings in the project, each is made with
/// all its detail; past it (a whole neighbourhood brought in), the light
/// version: windows as their glass, no grilles, railings as two rails.
pub const FULL_DRESS_LIMIT: usize = 150;

/// The touches a building gets beyond its dress, by its style and its own
/// seed (so that the neighbours do not all look alike): nothing of this is
/// stored, the same building always gets the same.
#[derive(Default, Clone, Copy)]
struct Touches {
    shutters: bool,
    /// A band round each window.
    surrounds: bool,
    /// A band at each floor.
    courses: bool,
    /// Stones at the free corners.
    quoins: bool,
    /// Chimneys on the roof.
    chimneys: usize,
    /// A front door (if the ground floor is not a shop).
    door: bool,
    awnings: bool,
    /// Stair housing, machines, tanks and an aerial on a flat roof.
    roof_gear: bool,
    /// Windows in the slope of a tiled roof.
    roof_windows: usize,
    /// A little tiled canopy over the front door.
    canopy: bool,
}

impl Touches {
    fn of(d: &Dress, seed: u64) -> Touches {
        let roll = |k: u64, p: f64| {
            let mut h = seed
                .wrapping_add(k.wrapping_mul(0x9E37_79B9))
                .wrapping_mul(0xBF58_476D_1CE4_E5B9);
            h ^= h >> 31;
            (h % 1000) as f64 / 1000.0 < p
        };
        use BuildingStyle as S;
        let homely = matches!(
            d.style,
            S::Andalusian
                | S::Terraced
                | S::SemiDetached
                | S::Detached
                | S::HistoricCentre
                | S::Courtyard
        );
        Touches {
            shutters: match d.style {
                S::HistoricCentre | S::Ensanche => roll(1, 0.6),
                S::Andalusian | S::Detached | S::Terraced | S::SemiDetached => roll(1, 0.45),
                _ => false,
            },
            surrounds: match d.style {
                S::HistoricCentre | S::Ensanche => true,
                S::Andalusian => roll(2, 0.5),
                S::Detached | S::Courtyard => roll(2, 0.3),
                _ => false,
            },
            courses: matches!(
                d.style,
                S::Ensanche | S::HistoricCentre | S::ShopAndFlats | S::Facility
            ) || (d.style == S::Block1960s && roll(3, 0.5)),
            quoins: match d.style {
                S::HistoricCentre | S::Ensanche => roll(4, 0.7),
                S::Detached | S::Facility => roll(4, 0.3),
                _ => false,
            },
            chimneys: if homely || matches!(d.style, S::Ensanche) {
                1 + roll(5, 0.4) as usize
            } else {
                0
            },
            door: !matches!(d.style, S::Warehouse | S::Offices),
            awnings: roll(6, 0.7),
            roof_gear: !matches!(d.style, S::Warehouse),
            roof_windows: if matches!(d.style, S::Terraced | S::SemiDetached | S::Detached) {
                roll(7, 0.6) as usize + roll(8, 0.3) as usize
            } else if d.style == S::HistoricCentre {
                roll(7, 0.3) as usize
            } else {
                0
            },
            canopy: matches!(d.style, S::Terraced | S::SemiDetached | S::Detached) && roll(9, 0.7),
        }
    }
}

/// How far the eaves of a tiled roof stand out of the walls.
const EAVES: f64 = 0.45;
/// The thickness of a tiled roof (chunky, as a model, not a drawing).
const ROOF_SLAB: f64 = 0.16;

/// What a dressed building gives, by material.
#[derive(Default)]
struct Out {
    by: BTreeMap<MaterialId, Vec<[P3; 3]>>,
    /// The light version.
    light: bool,
    touches: Touches,
}

impl Out {
    fn add(&mut self, m: MaterialId, tris: Vec<[P3; 3]>) {
        if !tris.is_empty() {
            self.by.entry(m).or_default().extend(tris);
        }
    }
}

/// The solids of a dressed mass: its body, façades and roof, one solid per
/// material. Empty if the mass is not a dressed contour. In its light
/// version when the project has many of them (`light_dress`).
pub fn dressed_solids(model: &Model, m: &Mass) -> Vec<Solid> {
    dressed_solids_detail(model, m, light_dress(model))
}

/// The same, with all its detail or in its light version (for far away).
pub fn dressed_solids_detail(model: &Model, m: &Mass, light: bool) -> Vec<Solid> {
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
    if let Some(w) = d.world {
        return world_solids(model, m, d, &poly, w, z0, floors, light);
    }
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
    let mut out = Out {
        light,
        touches: Touches::of(d, m.id.0),
        ..Out::default()
    };

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
    // The front door on the street (or on the longest free side).
    let front = door_side.or(longest_free);
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
            (on_street(i), front == Some(i)),
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
        EnvRoof::Gable | EnvRoof::Hip
            if lean_to(&poly, &sides, &|i| party(i), door_side.or(longest_free)).is_some() =>
        {
            // A deep house between party walls: one slope over the bay on
            // the street, a flat roof behind.
            let (a, inward) =
                lean_to(&poly, &sides, &|i| party(i), door_side.or(longest_free)).unwrap();
            let split = |front: bool| -> Vec<Vec<Point2>> {
                let k = FRONT_BAY;
                let (lo, hi) = bounds_2d(&poly);
                let big = (hi - lo).length() + 10.0;
                let along = inward.perp_left();
                let base = a + inward * k;
                let far = if front { -big } else { big };
                let half = vec![
                    base + along * -big,
                    base + along * big,
                    base + along * big + inward * far,
                    base + along * -big + inward * far,
                ];
                crate::junctions::intersection(
                    &[(crate::ensure_ccw(poly.clone()), Vec::new())],
                    &[(crate::ensure_ccw(half), Vec::new())],
                )
                .into_iter()
                .map(|(o, _)| o)
                .filter(|o| o.len() >= 3 && signed_area(o).abs() > 0.5)
                .collect()
            };
            let mut any = false;
            for piece in split(true) {
                let (ps, pp) = piece_sides(&piece, &sides, &|i| party(i));
                if ps.len() == piece.len() {
                    any |= pitched_roof_with(
                        &mut out,
                        d,
                        &piece,
                        &ps,
                        &|i| pp[i],
                        (z0, top),
                        inward.perp_left(),
                        m.id.0,
                        Some((a, inward)),
                    );
                }
            }
            for piece in split(false) {
                let (ps, _) = piece_sides(&piece, &sides, &|i| party(i));
                if ps.len() == piece.len() {
                    flat_roof(&mut out, d, &piece, &ps, None, top, (floors, m.id.0));
                }
            }
            any
        }
        EnvRoof::Gable | EnvRoof::Hip => {
            // The ridge along the street front (or the longest free
            // side, or the longest), turned if asked.
            let along = door_side
                .or(longest_free)
                .or_else(|| (0..n).max_by(|a, b| sides[*a].len.total_cmp(&sides[*b].len)))
                .map_or(Vec2::new(1.0, 0.0), |i| sides[i].u);
            let axis = if d.ridge_turned {
                along.perp_left()
            } else {
                along
            };
            pitched_roof(
                &mut out,
                d,
                &poly,
                &sides,
                &|i| party(i),
                (z0, top),
                axis,
                m.id.0,
            )
        }
    };
    if !tiled {
        flat_roof(
            &mut out,
            d,
            &poly,
            &sides,
            court.as_deref(),
            top,
            (floors, m.id.0),
        );
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

/// Whether the project's dressed buildings are made in their light
/// version (more than `FULL_DRESS_LIMIT` of them).
pub fn light_dress(model: &Model) -> bool {
    model
        .masses()
        .filter(|m| m.dress.is_some())
        .nth(FULL_DRESS_LIMIT)
        .is_some()
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
    let t = out.touches;
    if !out.light && t.courses && d.closure != Closure::CurtainWall {
        for f in 1..floors {
            let z = z0 + f as f64 * fh;
            out.add(
                mat.trim,
                side.block(ends(0.06), (0.0, 0.06), (z - 0.06, z + 0.08)),
            );
        }
    }
    if !out.light && t.quoins {
        let lo = z0 + d.plinth;
        for (free, at_end) in [(!prev_party, false), (!next_party, true)] {
            if !free || len < 2.0 {
                continue;
            }
            let mut z = lo;
            let mut k = 0;
            while z + 0.3 < top - if d.cornice { 0.45 } else { 0.0 } {
                let w = if k % 2 == 0 { 0.55 } else { 0.35 };
                let s = if at_end { (len - w, len) } else { (0.0, w) };
                out.add(mat.trim, side.block(s, (0.0, 0.03), (z, z + 0.3)));
                z += 0.34;
                k += 1;
            }
        }
    }
    if len < MIN_FACADE {
        return;
    }
    if d.closure == Closure::CurtainWall {
        curtain(out, d, side, (z0, top, fh, floors));
        return;
    }
    let bays = window_axes(len, d.bay, d.window_width);
    // The front door: in the bay nearest the middle of the ground floor.
    let door_bay = (main && t.door && !(d.shop && street))
        .then(|| {
            (0..bays.len()).min_by(|a, b| {
                (bays[*a] - len / 2.0)
                    .abs()
                    .total_cmp(&(bays[*b] - len / 2.0).abs())
            })
        })
        .flatten();
    // Room for shutters beside the windows.
    let shutters = !out.light && t.shutters && d.bay - d.window_width >= d.window_width + 0.3;
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
        for (k, c) in bays.iter().enumerate() {
            let s = (c - d.window_width / 2.0, c + d.window_width / 2.0);
            if f == 0 && door_bay == Some(k) {
                front_door(out, d, side, *c, floor, fh);
                continue;
            }
            window(out, d, side, s, floor + sill, h, sill > 0.0);
            if shutters && d.closure != Closure::Grilles {
                shutter_pair(out, d, side, s, floor + sill, h);
            }
            if d.closure == Closure::Grilles {
                grille(out, d, side, s, (floor, floor + sill), h);
            }
            if d.closure == Closure::Balconies && f > 0 {
                balcony_on(out, d, side, s, floor);
            }
        }
    }
}

/// A front door in the wall: its frame, its leaf with panels, a fanlight
/// when the floor is tall, the step and a little lintel over it.
fn front_door(out: &mut Out, d: &Dress, side: &Side, c: f64, floor: f64, fh: f64) {
    let mat = d.materials;
    let w = (d.bay * 0.85).clamp(0.85, 1.1);
    let h = (fh - 0.4).min(2.2);
    let s = (c - w / 2.0, c + w / 2.0);
    out.add(
        mat.joinery,
        side.block(
            (s.0 - FRAME, s.1 + FRAME),
            (0.0, FRAME_OUT),
            (floor, floor + h + FRAME),
        ),
    );
    // The leaf a little back, with two raised panels.
    out.add(
        mat.joinery,
        side.block(
            s,
            (FRAME_OUT, FRAME_OUT + 0.015),
            (floor + 0.02, floor + h - 0.02),
        ),
    );
    if !out.light {
        for (z0, z1) in [(0.15, h * 0.45), (h * 0.52, h - 0.15)] {
            out.add(
                mat.joinery,
                side.block(
                    (s.0 + 0.12, s.1 - 0.12),
                    (FRAME_OUT + 0.015, FRAME_OUT + 0.035),
                    (floor + z0, floor + z1),
                ),
            );
        }
    }
    // A fanlight over it when there is room.
    if fh - h > 0.9 {
        out.add(
            mat.joinery,
            side.block(
                (s.0 - FRAME, s.1 + FRAME),
                (0.0, FRAME_OUT),
                (floor + h + FRAME, floor + h + 0.55),
            ),
        );
        out.add(
            mat.glass,
            side.block(
                s,
                (FRAME_OUT, GLASS_OUT),
                (floor + h + FRAME + 0.05, floor + h + 0.48),
            ),
        );
    }
    // The step, and the lintel standing out over the door.
    out.add(
        mat.trim,
        side.block((s.0 - 0.25, s.1 + 0.25), (0.0, 0.4), (floor, floor + 0.14)),
    );
    let top = floor + h + if fh - h > 0.9 { 0.55 } else { FRAME };
    if out.touches.canopy && !out.light {
        // A tiled canopy on two brackets.
        let (a, b) = (s.0 - 0.45, s.1 + 0.45);
        let (z, deep) = (top + 0.12, 0.8);
        let tris = side.block((a, b), (0.0, deep), (z, z + 0.08));
        out.add(mat.trim, tris);
        // Its tiles, falling from the wall.
        let quad = [
            side.at(a - 0.03, 0.0),
            side.at(b + 0.03, 0.0),
            side.at(b + 0.03, deep + 0.05),
            side.at(a - 0.03, deep + 0.05),
        ];
        out.add(
            mat.roof,
            draped_prism(&quad, |p| {
                let t = (p - side.a).dot(side.out);
                (z + 0.06, z + 0.11 + 0.25 * (1.0 - t / deep).max(0.0))
            }),
        );
        for x in [a + 0.08, b - 0.12] {
            out.add(
                mat.joinery,
                side.block((x, x + 0.04), (0.0, deep * 0.6), (z - 0.3, z)),
            );
        }
    } else {
        out.add(
            mat.trim,
            side.block(
                (s.0 - 0.3, s.1 + 0.3),
                (0.0, 0.28),
                (top + 0.04, top + 0.16),
            ),
        );
    }
}

/// Two shutters folded back against the wall either side of a window.
fn shutter_pair(out: &mut Out, d: &Dress, side: &Side, s: (f64, f64), z: f64, h: f64) {
    let mat = d.materials;
    let w = (s.1 - s.0) / 2.0;
    for (a, b) in [
        (s.0 - FRAME - w, s.0 - FRAME - 0.01),
        (s.1 + FRAME + 0.01, s.1 + FRAME + w),
    ] {
        out.add(mat.joinery, side.block((a, b), (0.0, 0.04), (z, z + h)));
        // Its rails, as a louvred shutter's.
        for k in 1..4 {
            let zk = z + h * k as f64 / 4.0;
            out.add(
                mat.joinery,
                side.block((a + 0.03, b - 0.03), (0.04, 0.055), (zk - 0.02, zk + 0.02)),
            );
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
    if out.light {
        out.add(
            mat.glass,
            side.block((s.0, s.1), (0.0, GLASS_OUT), (z, z + h)),
        );
        return;
    }
    if out.touches.surrounds {
        // A band of render or stone round it, and a lintel over it.
        let b = 0.14;
        out.add(
            mat.trim,
            side.block(
                (s.0 - FRAME - b, s.1 + FRAME + b),
                (0.0, 0.02),
                (if sill { z - FRAME } else { z }, z + h + FRAME + b),
            ),
        );
        out.add(
            mat.trim,
            side.block(
                (s.0 - FRAME - b - 0.04, s.1 + FRAME + b + 0.04),
                (0.0, 0.07),
                (z + h + FRAME + b, z + h + FRAME + b + 0.1),
            ),
        );
    }
    out.add(
        mat.joinery,
        side.block(
            (s.0 - FRAME, s.1 + FRAME),
            (0.0, FRAME_OUT + 0.01),
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
    if out.light {
        return;
    }
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
    if out.light {
        return;
    }
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
        if !out.light && out.touches.awnings {
            awning(
                out,
                mat.joinery,
                side,
                (s0 - 0.1, s1 + 0.1),
                floor + h + 0.1,
            );
        }
    }
    // The fascia, for the sign.
    out.add(
        mat.trim,
        side.block((0.0, len), (0.0, 0.08), (floor + h + 0.12, floor + h + 0.6)),
    );
}

/// A sloping awning over a shop window, `z` its lowest edge at the wall
/// side's height: out 0,9 m, down 0,45 m, with its valance.
fn awning(out: &mut Out, m: MaterialId, side: &Side, s: (f64, f64), z: f64) {
    let (reach, drop, thick) = (0.9, 0.45, 0.04);
    let p = |s: f64, t: f64, z: f64| -> P3 {
        let q = side.at(s, t);
        [q.x, q.y, z]
    };
    let hi = z + drop;
    let lo = z;
    let top = [
        p(s.0, 0.0, hi),
        p(s.1, 0.0, hi),
        p(s.1, reach, lo),
        p(s.0, reach, lo),
    ];
    let bot = top.map(|q| [q[0], q[1], q[2] - thick]);
    let mut tris = Vec::new();
    let mut quad = |a: P3, b: P3, c: P3, d: P3| {
        tris.push([a, b, c]);
        tris.push([a, c, d]);
    };
    quad(top[0], top[3], top[2], top[1]);
    quad(bot[0], bot[1], bot[2], bot[3]);
    for i in 0..4 {
        let j = (i + 1) % 4;
        quad(bot[i], bot[j], top[j], top[i]);
    }
    out.add(m, tris);
    out.add(m, side.block(s, (reach - 0.02, reach), (lo - 0.25, lo)));
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
    (floors, seed): (usize, u64),
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
            roof_gear(out, d, poly, top + 0.03, (floors, seed));
        }
    }
}

/// A little of a hash of the seed and `k`, from 0 to 1.
fn unit(seed: u64, k: u64) -> f64 {
    let mut h = seed.wrapping_add(k.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    h = (h ^ (h >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    ((h ^ (h >> 31)) % 10_000) as f64 / 10_000.0
}

/// Whether a point is inside a contour.
fn inside(p: Point2, poly: &[Point2]) -> bool {
    let mut c = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            c = !c;
        }
    }
    c
}

/// A point well inside a contour, about `along`, `across` of its box (0 to
/// 1), with `room` of clear space round it; `None` if there is none.
fn spot(poly: &[Point2], along: f64, across: f64, room: f64) -> Option<Point2> {
    let (lo, hi) = bounds_2d(poly);
    let p = Point2::new(lo.x + (hi.x - lo.x) * along, lo.y + (hi.y - lo.y) * across);
    let ok = |q: Point2| {
        inside(q, poly)
            && [(room, 0.0), (-room, 0.0), (0.0, room), (0.0, -room)]
                .iter()
                .all(|(dx, dy)| inside(Point2::new(q.x + dx, q.y + dy), poly))
    };
    ok(p).then_some(p)
}

/// A box standing on the roof at `p`: `w` × `d` and `h` high from `z`.
fn roof_box(p: Point2, (w, d): (f64, f64), (z, h): (f64, f64)) -> Vec<[P3; 3]> {
    let q = [
        Point2::new(p.x - w / 2.0, p.y - d / 2.0),
        Point2::new(p.x + w / 2.0, p.y - d / 2.0),
        Point2::new(p.x + w / 2.0, p.y + d / 2.0),
        Point2::new(p.x - w / 2.0, p.y + d / 2.0),
    ];
    prism(&q, z, z + h)
}

/// A chimney from `z` (where it leaves the roof) up to `to`, with its cap.
fn chimney(out: &mut Out, d: &Dress, p: Point2, z: f64, to: f64) {
    let mat = d.materials;
    out.add(mat.wall, roof_box(p, (0.55, 0.7), (z, to - z)));
    out.add(mat.trim, roof_box(p, (0.75, 0.9), (to, 0.08)));
    if !out.light {
        out.add(mat.trim, roof_box(p, (0.3, 0.3), (to + 0.08, 0.18)));
        out.add(mat.trim, roof_box(p, (0.45, 0.45), (to + 0.26, 0.05)));
    }
}

/// What stands on a flat roof: the stair housing on blocks, machines,
/// a tank, an aerial, and the chimneys of the houses.
fn roof_gear(out: &mut Out, d: &Dress, poly: &[Point2], z: f64, (floors, seed): (usize, u64)) {
    let t = out.touches;
    if !t.roof_gear {
        return;
    }
    let mat = d.materials;
    let area = signed_area(poly).abs();
    // The stair housing, on blocks of flats.
    if floors >= 3 && area > 80.0 {
        if let Some(p) = spot(
            poly,
            0.35 + 0.3 * unit(seed, 1),
            0.35 + 0.3 * unit(seed, 2),
            2.0,
        ) {
            out.add(mat.wall, roof_box(p, (2.4, 3.2), (z, 2.6)));
            out.add(mat.trim, roof_box(p, (2.7, 3.5), (z + 2.6, 0.12)));
            let door = Point2::new(p.x, p.y - 1.6);
            out.add(mat.joinery, roof_box(door, (0.9, 0.06), (z, 2.05)));
        }
    }
    if out.light {
        return;
    }
    // Air conditioning machines.
    if !matches!(
        d.style,
        BuildingStyle::Andalusian | BuildingStyle::Courtyard
    ) || unit(seed, 3) < 0.5
    {
        let n = 1 + (unit(seed, 4) * 3.0) as usize;
        for k in 0..n {
            if let Some(p) = spot(
                poly,
                0.2 + 0.6 * unit(seed, 10 + k as u64),
                0.2 + 0.6 * unit(seed, 20 + k as u64),
                0.8,
            ) {
                out.add(mat.metal, roof_box(p, (0.9, 0.45), (z, 0.65)));
                out.add(mat.trim, roof_box(p, (0.95, 0.5), (z + 0.65, 0.04)));
            }
        }
    }
    // A water tank on the houses' terraces.
    if matches!(
        d.style,
        BuildingStyle::Andalusian
            | BuildingStyle::HistoricCentre
            | BuildingStyle::Courtyard
            | BuildingStyle::ShopAndFlats
    ) && unit(seed, 5) < 0.6
    {
        if let Some(p) = spot(
            poly,
            0.25 + 0.5 * unit(seed, 6),
            0.25 + 0.5 * unit(seed, 7),
            0.9,
        ) {
            out.add(mat.trim, roof_box(p, (0.8, 0.8), (z, 0.3)));
            out.add(mat.wall, roof_box(p, (1.0, 1.0), (z + 0.3, 1.0)));
        }
    }
    // An aerial.
    if unit(seed, 8) < 0.7 {
        if let Some(p) = spot(poly, 0.3 + 0.4 * unit(seed, 9), 0.6, 0.5) {
            out.add(mat.metal, roof_box(p, (0.05, 0.05), (z, 2.2)));
            for (k, w) in [(1.6, 1.0), (1.9, 0.8), (2.15, 0.6)] {
                out.add(mat.metal, roof_box(p, (w, 0.03), (z + k, 0.03)));
            }
        }
    }
    // The chimneys of the houses.
    for k in 0..t.chimneys {
        if let Some(p) = spot(
            poly,
            0.15 + 0.7 * unit(seed, 30 + k as u64),
            0.2 + 0.6 * unit(seed, 40 + k as u64),
            0.5,
        ) {
            chimney(out, d, p, z, z + PARAPET.0 + 0.6);
        }
    }
}

/// A tiled roof as the lowest of a few planes over the whole contour: two
/// falling from the ridge (a gabled roof; with two more at its ends, a
/// hipped one, when the building stands free) and a level one at
/// `MAX_RIDGE` over the eaves that flattens what would climb higher. Each
/// plane covers the part of the contour where it is the lowest, so the
/// roof always closes, whatever the contour; the walls go up to it along
/// every side (the gables). The ridge runs along `axis`. False if nothing
/// could be made (then the roof is flat).
#[allow(clippy::too_many_arguments)]
fn pitched_roof(
    out: &mut Out,
    d: &Dress,
    poly: &[Point2],
    sides: &[Side],
    party: &dyn Fn(usize) -> bool,
    (z0, top): (f64, f64),
    axis: Vec2,
    seed: u64,
) -> bool {
    pitched_roof_with(out, d, poly, sides, party, (z0, top), axis, seed, None)
}

/// Where a house between party walls is deeper than `DEEP_HOUSE` from its
/// street front: a point of the front and the way in.
fn lean_to(
    poly: &[Point2],
    sides: &[Side],
    party: &dyn Fn(usize) -> bool,
    door_side: Option<usize>,
) -> Option<(Point2, Vec2)> {
    let i = door_side?;
    if !(0..sides.len()).any(party) {
        return None;
    }
    let s = sides[i];
    let inward = -s.out;
    let depth = poly
        .iter()
        .map(|p| (*p - s.a).dot(inward))
        .fold(0.0, f64::max);
    (depth > DEEP_HOUSE).then_some((s.a, inward))
}

/// The sides of a piece cut from the contour, and which of them lie on a
/// party wall of the whole.
fn piece_sides(
    piece: &[Point2],
    whole: &[Side],
    party: &dyn Fn(usize) -> bool,
) -> (Vec<Side>, Vec<bool>) {
    let n = piece.len();
    let mut out = Vec::new();
    let mut on_party = Vec::new();
    for i in 0..n {
        let (a, b) = (piece[i], piece[(i + 1) % n]);
        let Some(u) = (b - a).normalized() else {
            continue;
        };
        let mid = a + (b - a) * 0.5;
        let p = whole.iter().enumerate().any(|(k, s)| {
            party(k) && {
                let q = mid - s.a;
                let t = q.dot(s.u);
                t > -0.05 && t < s.len + 0.05 && q.dot(s.out).abs() < 0.05
            }
        });
        out.push(Side {
            a,
            u,
            out: -u.perp_left(),
            len: a.distance(b),
        });
        on_party.push(p);
    }
    (out, on_party)
}

/// [`pitched_roof`], or with `lean` (a point of the street front and the
/// way in) one slope rising from the street.
#[allow(clippy::too_many_arguments)]
fn pitched_roof_with(
    out: &mut Out,
    d: &Dress,
    poly: &[Point2],
    sides: &[Side],
    party: &dyn Fn(usize) -> bool,
    (z0, top): (f64, f64),
    axis: Vec2,
    seed: u64,
    lean: Option<(Point2, Vec2)>,
) -> bool {
    let across = axis.perp_left();
    let c0 = poly[0];
    let (mut u0, mut u1, mut n0, mut n1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for p in poly {
        let q = *p - c0;
        u0 = u0.min(q.dot(axis));
        u1 = u1.max(q.dot(axis));
        n0 = n0.min(q.dot(across));
        n1 = n1.max(q.dot(across));
    }
    let pitch = d.pitch.max(0.05);
    let (hw, cn) = ((n1 - n0) / 2.0, (n0 + n1) / 2.0);
    let (hl, cu) = ((u1 - u0) / 2.0, (u0 + u1) / 2.0);
    // Planes as (slope, height at c0) over the eaves.
    let mut planes: Vec<(Vec2, f64)> = match lean {
        Some((a, inward)) => vec![(inward * pitch, pitch * (c0 - a).dot(inward))],
        None => vec![
            (across * -pitch, pitch * (hw + cn)),
            (across * pitch, pitch * (hw - cn)),
        ],
    };
    let free = !(0..sides.len()).any(party);
    let hip = d.roof == EnvRoof::Hip && free && lean.is_none();
    if hip {
        planes.push((axis * -pitch, pitch * (hl + cu)));
        planes.push((axis * pitch, pitch * (hl - cu)));
    }
    planes.push((Vec2::new(0.0, 0.0), MAX_RIDGE));
    // Over the eaves (below zero out of the walls, where the eaves stand
    // out and drop).
    let lowest = |p: Point2| -> f64 {
        let q = p - c0;
        planes
            .iter()
            .map(|(g, k)| g.dot(q) + k)
            .fold(f64::INFINITY, f64::min)
    };
    let rise = |p: Point2| lowest(p).max(0.0);
    let mat = d.materials;
    // The roof stands out of the free walls by the eaves.
    let eaves: Vec<f64> = (0..sides.len())
        .map(|i| if party(i) { 0.0 } else { EAVES })
        .collect();
    let roof_outline = grown(poly, sides, &eaves);
    let slab = if out.light { TILES } else { ROOF_SLAB };
    // The faces: where each plane is the lowest.
    let (lo, hi) = bounds_2d(&roof_outline);
    let pad = 1.0;
    let boxed = vec![
        Point2::new(lo.x - pad, lo.y - pad) - c0,
        Point2::new(hi.x + pad, lo.y - pad) - c0,
        Point2::new(hi.x + pad, hi.y + pad) - c0,
        Point2::new(lo.x - pad, hi.y + pad) - c0,
    ];
    let footprint = vec![(crate::ensure_ccw(roof_outline.clone()), Vec::new())];
    let mut any = false;
    // The tiled faces, for the roof windows.
    let mut faces: Vec<(Vec2, Vec<Point2>)> = Vec::new();
    for (k, (gk, kk)) in planes.iter().enumerate() {
        let mut region = boxed.clone();
        for (j, (gj, kj)) in planes.iter().enumerate() {
            if j != k {
                region = clip_half(&region, *gk - *gj, kk - kj);
            }
        }
        if region.len() < 3 || signed_area(&region).abs() < 1e-6 {
            continue;
        }
        let region: Vec<Point2> = region.into_iter().map(|q| q + c0).collect();
        for (outer, _) in crate::junctions::intersection(&footprint, &[(region, Vec::new())]) {
            if outer.len() < 3 || signed_area(&outer).abs() < 1e-4 {
                continue;
            }
            any = true;
            out.add(
                mat.roof,
                draped_prism(&outer, |q| {
                    let z = top + lowest(q);
                    (z, z + slab)
                }),
            );
            if gk.length() > 1e-9 {
                tile_courses(out, mat.roof, &outer, *gk, &|q| top + lowest(q) + slab);
                if !out.light {
                    faces.push((*gk, outer));
                }
            }
        }
    }
    if !any {
        return false;
    }
    // The walls up to the roof along every side, broken where the roof
    // changes plane.
    for side in sides {
        let (a, b) = (side.a, side.at(side.len, 0.0));
        let mut ts = vec![0.0, 1.0];
        for (j, (gj, kj)) in planes.iter().enumerate() {
            for (gk, kk) in &planes[j + 1..] {
                let g = *gj - *gk;
                let fa = g.dot(a - c0) + kj - kk;
                let fb = g.dot(b - c0) + kj - kk;
                if (fa - fb).abs() > 1e-12 {
                    let t = fa / (fa - fb);
                    if t > 1e-6 && t < 1.0 - 1e-6 {
                        ts.push(t);
                    }
                }
            }
        }
        ts.sort_by(f64::total_cmp);
        let mut tris = Vec::new();
        for w in ts.windows(2) {
            let (p, q) = (a + (b - a) * w[0], a + (b - a) * w[1]);
            let (zp, zq) = (rise(p), rise(q));
            if zp < 1e-3 && zq < 1e-3 {
                continue;
            }
            let (p0, q0) = ([p.x, p.y, top], [q.x, q.y, top]);
            let (p1, q1) = ([p.x, p.y, top + zp], [q.x, q.y, top + zq]);
            for t in [[p0, q0, q1], [p0, q1, p1]] {
                tris.push(t);
                tris.push([t[0], t[2], t[1]]);
            }
        }
        out.add(mat.wall, tris);
    }
    // The fascia along the eaves, at their edge.
    let n = roof_outline.len();
    for i in 0..n {
        let (a, b) = (roof_outline[i], roof_outline[(i + 1) % n]);
        let Some(u) = (b - a).normalized() else {
            continue;
        };
        let (za, zb) = (lowest(a), lowest(b));
        if (za - zb).abs() > 1e-3 || za > -1e-3 {
            continue;
        }
        let edge = Side {
            a,
            u,
            out: -u.perp_left(),
            len: a.distance(b),
        };
        out.add(
            mat.trim,
            edge.block(
                (0.0, edge.len),
                (-0.04, 0.0),
                (top + za - 0.16, top + za + slab),
            ),
        );
        if !out.light && !party(i) {
            gutter(out, mat.metal, &edge, &sides[i], (z0, top + za - 0.16));
        }
    }
    // The ridge cap, where the two falls meet below the level top.
    if pitch * hw < MAX_RIDGE {
        let (s0, s1) = if hip {
            (u0 + hw, u1 - hw)
        } else {
            (u0 - EAVES, u1 + EAVES)
        };
        if s1 - s0 > 0.3 {
            let a = c0 + axis * s0 + across * cn;
            let b = c0 + axis * s1 + across * cn;
            let z = top + pitch * hw + slab;
            for (p, q) in inside_parts(a, b, &roof_outline) {
                let Some(u) = (q - p).normalized() else {
                    continue;
                };
                let ridge = Side {
                    a: p,
                    u,
                    out: -u.perp_left(),
                    len: p.distance(q),
                };
                out.add(
                    mat.roof,
                    ridge.block((0.0, ridge.len), (-0.13, 0.13), (z - 0.05, z + 0.09)),
                );
            }
        }
    }
    // On a house between party walls, the chimneys go on them, by the
    // ridge: each house its own, so that the two neighbours' stand in a
    // pair.
    let party_sides: Vec<usize> = (0..sides.len()).filter(|i| party(*i)).collect();
    if !party_sides.is_empty() && out.touches.chimneys > 0 {
        let ridge = top + pitch * hw.min(MAX_RIDGE / pitch);
        for &i in party_sides.iter().take(2) {
            let side = &sides[i];
            // Where the roof is highest along the wall.
            let best = (1..20)
                .map(|k| side.len * k as f64 / 20.0)
                .max_by(|a, b| rise(side.at(*a, 0.0)).total_cmp(&rise(side.at(*b, 0.0))));
            let Some(t) = best else { continue };
            let t = t.clamp(0.5, (side.len - 0.5).max(0.5));
            let p = side.at(t, -0.32);
            if rise(p) < 0.5 {
                continue;
            }
            chimney(out, d, p, top + rise(p) - 0.1, ridge + 0.7);
        }
    } else {
        // Chimneys through the roof, near the ridge.
        for k in 0..out.touches.chimneys {
            let along = 0.2 + 0.6 * unit(seed, 50 + k as u64);
            let p = c0
                + axis * (u0 + (u1 - u0) * along)
                + across * (cn + 0.7 * if k % 2 == 0 { 1.0 } else { -1.0 });
            if !inside(p, poly) {
                continue;
            }
            let ridge = top + pitch * hw.min(MAX_RIDGE / pitch);
            chimney(out, d, p, top + rise(p) - 0.1, ridge + 0.8);
        }
    }
    // Windows in the slope, on the biggest faces.
    if !out.light && out.touches.roof_windows > 0 {
        faces.sort_by(|a, b| signed_area(&b.1).abs().total_cmp(&signed_area(&a.1).abs()));
        for (k, (g, face)) in faces.iter().take(out.touches.roof_windows).enumerate() {
            roof_window(
                out,
                d,
                face,
                *g,
                &|q| top + lowest(q) + slab,
                unit(seed, 60 + k as u64),
            );
        }
    }
    true
}

/// Courses of tiles over a tiled face that rises along `g`: plates
/// thicker at their lower edge, so each course steps down over the next.
fn tile_courses(out: &mut Out, m: MaterialId, face: &[Point2], g: Vec2, z: &dyn Fn(Point2) -> f64) {
    let Some(up) = g.normalized() else { return };
    let k = (1.0 + g.dot(g)).sqrt();
    // From afar (the light version) one step every two courses, made of a
    // single plate: still a line across the roof at every step.
    let course = if out.light { 0.68 } else { 0.34 } / k;
    let ds: Vec<f64> = face
        .iter()
        .map(|p| (*p - Point2::new(0.0, 0.0)).dot(up))
        .collect();
    let (d0, d1) = ds
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &x| {
            (a.min(x), b.max(x))
        });
    let across = up.perp_left();
    let cs: Vec<f64> = face
        .iter()
        .map(|p| (*p - Point2::new(0.0, 0.0)).dot(across))
        .collect();
    let (c0, c1) = cs
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &x| {
            (a.min(x), b.max(x))
        });
    let shape = vec![(crate::ensure_ccw(face.to_vec()), Vec::new())];
    let at = |c: f64, d: f64| Point2::new(0.0, 0.0) + across * c + up * d;
    let n = ((d1 - d0) / course).ceil().clamp(1.0, 200.0) as usize;
    // Each course: its nose at the lower edge, rising steeply, and the
    // tile falling gently back to the roof up to the next course, so that
    // a line shows at every course from any side.
    let nose = (0.025 / k).min(course / 3.0);
    let (low, high) = (0.008, 0.04);
    for i in 0..n {
        let lo = d0 + course * i as f64;
        let hi = (lo + course).min(d1);
        let steps: &[(f64, f64, f64, f64)] = if out.light {
            &[(lo, hi, 0.05, low)]
        } else {
            &[(lo, lo + nose, low, high), (lo + nose, hi, high, low)]
        };
        for &(a, b, za, zb) in steps {
            if b - a < 1e-6 {
                continue;
            }
            let strip = crate::ensure_ccw(vec![
                at(c0 - 1.0, a),
                at(c1 + 1.0, a),
                at(c1 + 1.0, b),
                at(c0 - 1.0, b),
            ]);
            let span = b - a;
            for (piece, _) in crate::junctions::intersection(&shape, &[(strip, Vec::new())]) {
                out.add(
                    m,
                    draped_prism(&piece, |q| {
                        let f = (((q - Point2::new(0.0, 0.0)).dot(up) - a) / span).clamp(0.0, 1.0);
                        let base = z(q);
                        (base - 0.01, base + za + (zb - za) * f)
                    }),
                );
            }
        }
    }
}

/// A window lying in a tiled face that rises along `g`: its frame and its
/// glass, somewhere along the face (`at`, 0 to 1), if it fits.
fn roof_window(
    out: &mut Out,
    d: &Dress,
    face: &[Point2],
    g: Vec2,
    z: &dyn Fn(Point2) -> f64,
    at: f64,
) {
    let Some(up) = g.normalized() else { return };
    let k = (1.0 + g.dot(g)).sqrt();
    let across = up.perp_left();
    let (w, h) = (0.78, 1.18 / k);
    let n = face.len() as f64;
    let c = face.iter().fold(Point2::new(0.0, 0.0), |a, p| {
        a + (*p - Point2::new(0.0, 0.0))
    }) * (1.0 / n);
    let ds: Vec<f64> = face.iter().map(|p| (*p - c).dot(across)).collect();
    let (lo, hi) = ds
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &x| {
            (a.min(x), b.max(x))
        });
    let mid = c + across * (lo + (hi - lo) * (0.25 + 0.5 * at));
    let rect = |m: f64| {
        let (a, b) = (w / 2.0 - m, h / 2.0 - m / k);
        [
            mid - across * a - up * b,
            mid + across * a - up * b,
            mid + across * a + up * b,
            mid - across * a + up * b,
        ]
    };
    let frame = rect(0.0);
    if !frame.iter().all(|p| inside(*p, face)) || !frame.iter().all(|p| inside(*p + up * 0.3, face))
    {
        return;
    }
    let mat = d.materials;
    out.add(
        mat.joinery,
        draped_prism(&frame, |q| (z(q) - 0.02, z(q) + 0.09)),
    );
    out.add(
        mat.glass,
        draped_prism(&rect(0.07), |q| (z(q) + 0.05, z(q) + 0.1)),
    );
}

/// A gutter under the eaves along `edge` (its outer edge) and a downpipe
/// down the wall at its start, to the ground at `z0`.
fn gutter(out: &mut Out, m: MaterialId, edge: &Side, wall: &Side, (z0, z): (f64, f64)) {
    out.add(m, edge.block((0.0, edge.len), (-0.02, 0.12), (z - 0.12, z)));
    // The downpipe at the wall's corner, with its bend to the gutter.
    let s = 0.2;
    out.add(
        m,
        wall.block((s, s + 0.08), (0.02, 0.1), (z0 + 0.05, z - 0.25)),
    );
    let reach = (edge.at(s, 0.05) - wall.at(s, 0.0)).dot(wall.out);
    if reach > 0.1 {
        out.add(
            m,
            wall.block((s, s + 0.08), (0.02, reach + 0.06), (z - 0.3, z - 0.18)),
        );
    }
}

/// A contour with each side moved out by its `by` (the eaves standing out
/// of the free walls).
fn grown(poly: &[Point2], sides: &[Side], by: &[f64]) -> Vec<Point2> {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let prev = &sides[(i + n - 1) % n];
            let next = &sides[i];
            let (bp, bn) = (by[(i + n - 1) % n], by[i]);
            // Where the two moved sides meet.
            let p0 = prev.a + prev.out * bp;
            let q0 = next.a + next.out * bn;
            let cross = prev.u.cross(next.u);
            if cross.abs() < 1e-9 {
                return q0;
            }
            let t = (q0 - p0).cross(next.u) / cross;
            let p = p0 + prev.u * t;
            // A very sharp corner would shoot far out: keep it near.
            if p.distance(poly[i]) > 3.0 * bp.max(bn).max(0.01) {
                q0
            } else {
                p
            }
        })
        .collect()
}

/// The parts of a segment inside a contour.
fn inside_parts(a: Point2, b: Point2, poly: &[Point2]) -> Vec<(Point2, Point2)> {
    let mut ts = vec![0.0, 1.0];
    let n = poly.len();
    let d = b - a;
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let e = q - p;
        let den = d.cross(e);
        if den.abs() < 1e-12 {
            continue;
        }
        let t = (p - a).cross(e) / den;
        let u = (p - a).cross(d) / den;
        if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
            ts.push(t);
        }
    }
    ts.sort_by(f64::total_cmp);
    ts.windows(2)
        .filter(|w| w[1] - w[0] > 1e-6 && inside(a + d * ((w[0] + w[1]) / 2.0), poly))
        .map(|w| (a + d * w[0], a + d * w[1]))
        .collect()
}

/// The part of a convex polygon where `g·q + k <= 0`.
fn clip_half(poly: &[Point2], g: Vec2, k: f64) -> Vec<Point2> {
    let f = |q: Point2| g.dot(q) + k;
    let mut out = Vec::new();
    for i in 0..poly.len() {
        let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
        let (fp, fq) = (f(p), f(q));
        if fp <= 0.0 {
            out.push(p);
        }
        if (fp < 0.0 && fq > 0.0) || (fp > 0.0 && fq < 0.0) {
            out.push(p + (q - p) * (fp / (fp - fq)));
        }
    }
    out
}

/// A building of the surroundings made as one of the world's: its plan
/// with its party walls and its fronts to the street, the typology in the
/// skin of its place (`archisim_core::props::fine::buildings::world`), its
/// pieces in the project's materials by their names.
#[allow(clippy::too_many_arguments)]
fn world_solids(
    model: &Model,
    m: &Mass,
    d: &Dress,
    poly: &[Point2],
    w: archisim_core::WorldDress,
    z0: f64,
    floors: usize,
    light: bool,
) -> Vec<Solid> {
    use archisim_core::props::fine::buildings::world::{
        material_of,
        plan::{Plan, Side as WSide},
        state::State,
        types::{dress_cached_with, Era},
    };
    use archisim_core::WorldTypology;
    let n = poly.len();
    let party = model.party_sides(m.id);
    let street = model.street_sides(m.id);
    let party = |i: usize| party.get(i).copied().unwrap_or(false);
    let on_street = |i: usize| street.get(i).copied().unwrap_or(false);
    let len = |i: usize| poly[i].distance(poly[(i + 1) % n]);
    // A front to the street: the street's, the one placed by hand, or
    // the longest free side.
    let by_hand = w.front.map(|f| f as usize).filter(|f| *f < n && !party(*f));
    let streets = (0..n)
        .filter(|i| on_street(*i) && !party(*i))
        .max_by(|a, b| len(*a).total_cmp(&len(*b)));
    // The one by hand wins if it is on the street or there is no street.
    let main = by_hand
        .filter(|f| on_street(*f) || streets.is_none())
        .or(streets)
        .or_else(|| {
            (0..n)
                .filter(|i| !party(*i))
                .max_by(|a, b| len(*a).total_cmp(&len(*b)))
        });
    let sides: Vec<WSide> = (0..n)
        .map(|i| {
            if party(i) {
                WSide::Party
            } else if on_street(i) || Some(i) == main {
                WSide::Street
            } else {
                match w.typology {
                    WorldTypology::Street => WSide::Back,
                    WorldTypology::Block
                    | WorldTypology::House
                    | WorldTypology::Tower
                    | WorldTypology::Facility => WSide::Free,
                    WorldTypology::Courtyard => WSide::Back,
                }
            }
        })
        .collect();
    let o = poly[0];
    let plan = Plan {
        poly: poly.iter().map(|p| [p.x - o.x, p.y - o.y]).collect(),
        sides,
    };
    // A facility stands on the rectangle round the plan, its front on the
    // main side; the rest on the plan itself.
    let facility_frame = (w.typology == WorldTypology::Facility).then(|| {
        let i = main.unwrap_or(0);
        let (a, e) = (poly[i], poly[(i + 1) % n]);
        let c = poly.iter().fold(Point2::new(0.0, 0.0), |s, p| {
            Point2::new(s.x + p.x / n as f64, s.y + p.y / n as f64)
        });
        let len = a.distance(e).max(1e-6);
        let u = [(e.x - a.x) / len, (e.y - a.y) / len];
        let mut nrm = [u[1], -u[0]];
        if (c.x - a.x) * nrm[0] + (c.y - a.y) * nrm[1] > 0.0 {
            nrm = [-nrm[0], -nrm[1]];
        }
        let along = [nrm[1], -nrm[0]];
        let (mut s0, mut s1, mut t0) = (f64::MAX, f64::MIN, 0.0f64);
        for p in poly {
            let (dx, dy) = (p.x - a.x, p.y - a.y);
            let s = dx * along[0] + dy * along[1];
            s0 = s0.min(s);
            s1 = s1.max(s);
            t0 = t0.min(dx * nrm[0] + dy * nrm[1]);
        }
        (a, along, nrm, s0, s1, -t0)
    });
    let mesh = match facility_frame {
        Some((_, _, _, s0, s1, depth)) => {
            archisim_core::props::fine::buildings::world::civic::facility_cached(
                archisim_core::props::fine::buildings::world::civic::Programme::of(
                    w.programme as f64,
                ),
                w.place as usize,
                Era::of(w.era as f64),
                State::of(w.state as f64),
                (s1 - s0).max(3.0),
                depth.max(3.0),
                floors,
                w.variant as u64,
                light,
            )
        }
        None => dress_cached_with(
            &plan,
            w.typology.props(),
            w.place as usize,
            Era::of(w.era as f64),
            State::of(w.state as f64),
            floors,
            w.variant as u64,
            light,
        ),
    };
    let by_name: std::collections::HashMap<&str, MaterialId> =
        model.materials().map(|x| (x.name.as_str(), x.id)).collect();
    let place = |q: [f32; 3]| -> P3 {
        match facility_frame {
            Some((a, u, nrm, s0, s1, depth)) => {
                let (x, y) = ((s0 + s1) / 2.0 + q[0] as f64, q[1] as f64 - depth);
                [
                    a.x + u[0] * x + nrm[0] * y,
                    a.y + u[1] * x + nrm[1] * y,
                    z0 + q[2] as f64,
                ]
            }
            None => [o.x + q[0] as f64, o.y + q[1] as f64, z0 + q[2] as f64],
        }
    };
    let mut by: BTreeMap<MaterialId, Vec<[P3; 3]>> = BTreeMap::new();
    for (role, tris) in &mesh.parts {
        let material = material_of(role)
            .and_then(|name| by_name.get(name).copied())
            .unwrap_or(d.materials.wall);
        by.entry(material).or_default().extend(
            tris.iter()
                .map(|t| t.map(|i| place(mesh.positions[i as usize]))),
        );
    }
    by.into_iter()
        .map(|(material, triangles)| Solid {
            owner: SolidOwner::Mass(m.id),
            material,
            triangles,
        })
        .collect()
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
            for so in &s {
                for t in &so.triangles {
                    for q in t {
                        assert!(q[2] > -1e-9, "{} {:?}: {t:?}", style.name(), so.material);
                    }
                }
            }
            assert!(lo[2].abs() < 1e-9, "{}: {lo:?}", style.name());
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

    /// A whole neighbourhood: past `FULL_DRESS_LIMIT` buildings, each is
    /// made in its light version, less than half as heavy and as tall.
    #[test]
    fn many_buildings_are_lighter() {
        let mut m = Model::new("T");
        let l = m.add_level("Planta baja", 0.0);
        let add = |m: &mut Model, i: usize| {
            let x = (i % 20) as f64 * 30.0;
            let y = (i / 20) as f64 * 30.0;
            m.add_env_building(NewEnvBuilding {
                name: format!("B{i}"),
                level: l,
                outline: rect(x, y, x + 16.0, y + 12.0),
                style: BuildingStyle::Block1960s,
                floors: Some(5),
                floor_height: None,
                base_offset: 0.0,
            })
            .unwrap()
        };
        let first = add(&mut m, 0);
        let count = |m: &Model| -> (usize, f64) {
            let s = dressed_solids(m, m.mass(first).unwrap());
            (s.iter().map(|s| s.triangles.len()).sum(), bounds(&s).1[2])
        };
        let (full, top) = count(&m);
        for i in 1..=FULL_DRESS_LIMIT {
            add(&mut m, i);
        }
        assert!(light_dress(&m));
        let (light, top_light) = count(&m);
        assert!(light * 2 < full, "{light} of {full}");
        assert!((top - top_light).abs() < 1e-9);
    }

    /// A row of houses made as the world's (tanda 7): each one of the
    /// street's houses in the skin of its place, in the project's
    /// materials, blind on its party walls, lighter from afar.
    #[test]
    fn houses_made_as_the_worlds() {
        let mut m = Model::new("T");
        let l = m.add_level("Planta baja", 0.0);
        let mut ids = Vec::new();
        for i in 0..3 {
            let x = i as f64 * 7.0;
            ids.push(
                m.add_env_building(NewEnvBuilding {
                    name: format!("Casa {i}"),
                    level: l,
                    outline: rect(x, 0.0, x + 7.0, 12.0),
                    style: BuildingStyle::Andalusian,
                    floors: Some(3),
                    floor_height: None,
                    base_offset: 0.0,
                })
                .unwrap(),
            );
        }
        // The town house of Paris (6).
        assert_eq!(m.dress_as_world(&ids, Some(6)).unwrap(), 3);
        let mid = m.mass(ids[1]).unwrap();
        let w = mid.dress.unwrap().world.unwrap();
        assert_eq!(w.typology, archisim_core::WorldTypology::Street);
        let full = dressed_solids_detail(&m, mid, false);
        let light = dressed_solids_detail(&m, mid, true);
        let count = |s: &[Solid]| s.iter().map(|s| s.triangles.len()).sum::<usize>();
        assert!(count(&full) > 5_000, "{}", count(&full));
        assert!(
            count(&light) * 10 < count(&full) * 6,
            "{} of {}",
            count(&light),
            count(&full)
        );
        // Its materials are the world's, by name: the zinc of the mansard.
        let zinc = m.materials().find(|x| x.name == "Zinc").unwrap().id;
        assert!(full.iter().any(|s| s.material == zinc));
        // Blind on its party walls: no glass out of x 7..14.
        let glass = m.materials().find(|x| x.name == "Vidrio claro").unwrap().id;
        for t in full
            .iter()
            .filter(|s| s.material == glass)
            .flat_map(|s| &s.triangles)
            .flatten()
        {
            assert!(t[0] > 7.0 + 0.1 && t[0] < 14.0 - 0.1, "{t:?}");
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
        // Its gabled roof rises to the ridge in the middle of the depth,
        // with its cap over it.
        let roof = mid.dress.unwrap().materials.roof;
        let tiles: Vec<Solid> = s.iter().filter(|s| s.material == roof).cloned().collect();
        let (_, hi) = bounds(&tiles);
        let ridge = 6.0 + 6.0 * 0.4;
        assert!(
            (hi[2] - (ridge + ROOF_SLAB + 0.09)).abs() < 0.05,
            "{}",
            hi[2]
        );
    }

    /// A deep house closed on three sides would climb 40 m × its pitch
    /// from the street: its roof is left flat, never in the air.
    #[test]
    fn a_roof_never_climbs_into_the_air() {
        let mut m = Model::new("T");
        let l = m.add_level("Planta baja", 0.0);
        let add = |m: &mut Model, outline| {
            m.add_env_building(NewEnvBuilding {
                name: "Casa".into(),
                level: l,
                outline,
                style: BuildingStyle::Andalusian,
                floors: Some(2),
                floor_height: None,
                base_offset: 0.0,
            })
            .unwrap()
        };
        let deep = add(&mut m, rect(0.0, 0.0, 6.0, 40.0));
        add(&mut m, rect(-6.0, 0.0, 0.0, 40.0));
        add(&mut m, rect(6.0, 0.0, 12.0, 40.0));
        add(&mut m, rect(0.0, 40.0, 6.0, 46.0));
        let mass = m.mass(deep).unwrap();
        let s = dressed_solids(&m, mass);
        let (lo, hi) = bounds(&s);
        assert!(hi[2] < mass.height + 6.0, "{}", hi[2]);
        // One slope over the first bay from the street, flat behind.
        let roof_over = |y0: f64, y1: f64| {
            s.iter()
                .flat_map(|x| x.triangles.iter())
                .flat_map(|t| t.iter())
                .filter(|p| p[1] > y0 && p[1] < y1)
                .map(|p| p[2])
                .fold(f64::MIN, f64::max)
        };
        assert!(
            roof_over(2.0, 5.0) > mass.height + 1.0,
            "{}",
            roof_over(2.0, 5.0)
        );
        // (the aerial and the chimneys of the flat roof stand over it)
        assert!(
            roof_over(10.0, 38.0) < mass.height + 2.6,
            "{}",
            roof_over(10.0, 38.0)
        );
        assert!(lo[0] > -1.0 && hi[0] < 7.0 && lo[1] > -2.0 && hi[1] < 41.0);
    }

    /// The roof covers the whole contour whatever its shape (an L, a
    /// tangled outline of the cadastre), never climbs past `MAX_RIDGE`,
    /// and its ridge can be turned.
    #[test]
    fn tiled_roofs_close_over_any_contour() {
        let mut m = Model::new("T");
        let l = m.add_level("Planta baja", 0.0);
        let shapes = [
            // An L.
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(20.0, 0.0),
                Point2::new(20.0, 8.0),
                Point2::new(8.0, 8.0),
                Point2::new(8.0, 18.0),
                Point2::new(0.0, 18.0),
            ],
            // A ragged outline with a notch and a slanted side.
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(9.0, 0.5),
                Point2::new(9.2, 4.0),
                Point2::new(12.0, 4.1),
                Point2::new(13.0, 11.0),
                Point2::new(6.0, 12.5),
                Point2::new(5.5, 7.0),
                Point2::new(0.4, 7.2),
            ],
        ];
        for (k, outline) in shapes.into_iter().enumerate() {
            for (style, turned) in [
                (BuildingStyle::Detached, false),
                (BuildingStyle::Detached, true),
                (BuildingStyle::Terraced, false),
            ] {
                let x = k as f64 * 100.0;
                let outline: Vec<Point2> =
                    outline.iter().map(|p| Point2::new(p.x + x, p.y)).collect();
                let id = m
                    .add_env_building(NewEnvBuilding {
                        name: "Casa".into(),
                        level: l,
                        outline: outline.clone(),
                        style,
                        floors: Some(2),
                        floor_height: None,
                        base_offset: 0.0,
                    })
                    .unwrap();
                let mut dress = m.mass(id).unwrap().dress.unwrap();
                if dress.roof == EnvRoof::Flat {
                    dress.roof = EnvRoof::Gable;
                }
                dress.ridge_turned = turned;
                m.set_mass_dress(id, Some(dress)).unwrap();
                let mass = m.mass(id).unwrap().clone();
                let s = dressed_solids(&m, &mass);
                let roof = mass.dress.unwrap().materials.roof;
                let mut covered = 0.0;
                // The light version: its bands of courses lie over the
                // slab, once more over it.
                for t in dressed_solids_detail(&m, &mass, true)
                    .iter()
                    .filter(|s| s.material == roof)
                    .flat_map(|s| &s.triangles)
                {
                    let a = [t[1][0] - t[0][0], t[1][1] - t[0][1]];
                    let b = [t[2][0] - t[0][0], t[2][1] - t[0][1]];
                    let up = (a[0] * b[1] - a[1] * b[0]) / 2.0;
                    if up > 0.0 {
                        covered += up;
                    }
                }
                // All of it, and the eaves round it.
                let area = signed_area(&outline).abs();
                assert!(
                    covered > area * 0.99 && covered < area * 3.0,
                    "{k} {turned}: {covered} of {area}"
                );
                let tiles: Vec<Solid> = s.iter().filter(|s| s.material == roof).cloned().collect();
                let (_, hi) = bounds(&tiles);
                assert!(
                    hi[2] <= mass.height + MAX_RIDGE + ROOF_SLAB + 0.1,
                    "{}",
                    hi[2]
                );
                m.remove_mass(id).unwrap();
            }
        }
    }
}
