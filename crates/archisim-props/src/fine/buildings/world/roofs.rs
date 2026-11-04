//! The roofs of the world's buildings: flat behind a parapet, two slopes
//! to the street or across it, four slopes, mansards with their dormers
//! and the deep dark roofs of Asia; covered in curved tiles, in courses
//! (slates, plain tiles, shingles) or in sheet metal with its seams; and
//! what stands on them.

use super::super::more::{hipped_roof, tile_roof_along_y, turn_from};
use super::super::*;
use super::skins::*;
use super::Look;
use crate::build::{cross, norm, Builder, P3};
use std::f64::consts::PI;

/// The roof over a rectangle of walls.
#[derive(Clone, Debug)]
pub struct RoofPlan {
    pub x: (f64, f64),
    pub y: (f64, f64),
    /// The top of the walls.
    pub z: f64,
    pub form: Roof,
    pub cover: Cover,
    pub pitch: f64,
    pub eaves: f64,
    /// Between party walls: no eaves at the sides, the ends closed.
    pub party: bool,
    /// The axes of the dormers, along x, on the front (and the back).
    pub dormers: Vec<f64>,
    pub gable: Gable,
    pub seed: u64,
    pub extras: u32,
    pub top: Top,
    /// A block: the box of the stair on a flat roof.
    pub block: bool,
    /// Seen from afar: plain slopes, nothing small.
    pub light: bool,
}

/// Builds the roof.
pub fn roof(b: &mut Builder, r: &RoofPlan, l: &Look) {
    let plain;
    let r = if r.light {
        plain = RoofPlan {
            cover: match r.cover {
                Cover::Courses | Cover::Curved | Cover::Bands => Cover::Bands,
                _ => Cover::Plain,
            },
            ..r.clone()
        };
        &plain
    } else {
        r
    };
    match r.form {
        Roof::Flat => flat(b, r, l),
        Roof::Gable => gable(b, r, l),
        Roof::GableToStreet => to_street(b, r, l),
        Roof::Hip => hip(b, r, l),
        Roof::Mansard => mansard(b, r, l),
        Roof::Asian => asian(b, r),
        Roof::Pent => pent(b, r, l),
    }
    if r.extras & CHIMNEYS != 0 {
        chimneys(b, r, l);
    }
}

/// The four faces of the rectangle, seen from outside: front (y1),
/// right, back, left.
pub fn faces(x: (f64, f64), y: (f64, f64)) -> [Face; 4] {
    let (w, d) = (x.1 - x.0, y.1 - y.0);
    [
        Face {
            o: [x.0, y.1],
            u: [1.0, 0.0],
            len: w,
        },
        Face {
            o: [x.1, y.1],
            u: [0.0, -1.0],
            len: d,
        },
        Face {
            o: [x.1, y.0],
            u: [-1.0, 0.0],
            len: w,
        },
        Face {
            o: [x.0, y.0],
            u: [0.0, 1.0],
            len: d,
        },
    ]
}

fn flat(b: &mut Builder, r: &RoofPlan, l: &Look) {
    let (x, y, z) = (r.x, r.y, r.z);
    b.rbox(
        l.roof,
        [x.0 + 0.15, y.0 + 0.15, z - 0.12],
        [x.1 - 0.15, y.1 - 0.15, z + 0.04],
        0.0,
    );
    let high = if r.extras & ALGARVE != 0 { 0.7 } else { 1.0 };
    for (i, f) in faces(x, y).iter().enumerate() {
        if i == 0 && r.top == Top::Balustrade && !r.light {
            balustrade(b, f, z, l);
            continue;
        }
        f.bx(b, l.wall, (0.0, f.len), (z, z + high), (-0.2, 0.0));
        f.bx(
            b,
            l.trim,
            (-0.03, f.len + 0.03),
            (z + high, z + high + 0.06),
            (-0.23, 0.03),
        );
        if i == 0 && r.extras & ALGARVE != 0 {
            f.bx(
                b,
                l.accent,
                (0.0, f.len),
                (z + 0.1, z + 0.35),
                (-0.01, 0.012),
            );
        }
    }
    let (w, d) = (x.1 - x.0, y.1 - y.0);
    let c = [(x.0 + x.1) / 2.0, (y.0 + y.1) / 2.0];
    let zr = z + 0.04;
    if r.block {
        // The box of the stair, with its door.
        let sx = [c[0] - 1.6, c[0] + 1.6];
        let sy = [y.0 + d * 0.35, y.0 + d * 0.35 + 3.0];
        b.rbox(l.wall, [sx[0], sy[0], zr], [sx[1], sy[1], zr + 2.6], 0.0);
        b.rbox(
            l.trim,
            [sx[0] - 0.05, sy[0] - 0.05, zr + 2.6],
            [sx[1] + 0.05, sy[1] + 0.05, zr + 2.7],
            0.0,
        );
        b.rbox(
            l.door,
            [sx[0] + 0.4, sy[1] - 0.02, zr],
            [sx[0] + 1.3, sy[1] + 0.03, zr + 2.1],
            0.0,
        );
    }
    if r.extras & TANK != 0 {
        water_tank(b, [x.0 + w * 0.22, y.0 + d * 0.3], zr);
        aerial(b, [x.1 - w * 0.25, y.0 + d * 0.6], zr, r.seed);
        if r.block && w > 12.0 {
            water_tank(b, [x.1 - w * 0.15, y.0 + d * 0.3], zr);
        }
    }
    if r.extras & LAUNDRY != 0 && d > 6.0 {
        roof_clothes(
            b,
            [x.0 + w * 0.15, y.1 - d * 0.25],
            [x.0 + w * 0.6, y.1 - d * 0.25],
            zr,
            r.seed,
        );
    }
    if r.extras & SOLAR != 0 {
        let n = ((w - 2.0) / 2.6).floor().max(1.0) as usize;
        for k in 0..n {
            solar_heater(b, [x.0 + 1.6 + 2.6 * k as f64, y.1 - 1.8], zr);
        }
    }
    if r.extras & WATER_TOWER != 0 {
        water_tower(b, [x.0 + w * 0.7, y.0 + d * 0.35], zr);
    }
    if r.extras & ALGARVE != 0 {
        algarve_chimney(b, [x.0 + 0.9, y.0 + 1.0], zr, l);
    }
}

/// A balustrade on a parapet: plinth, balusters and the rail.
pub(super) fn balustrade(b: &mut Builder, f: &Face, z: f64, l: &Look) {
    f.bx(b, l.trim, (0.0, f.len), (z, z + 0.18), (-0.25, 0.02));
    f.bx(
        b,
        l.trim,
        (-0.02, f.len + 0.02),
        (z + 0.85, z + 0.97),
        (-0.27, 0.04),
    );
    let mut s = 0.25;
    let mut posts = 0;
    while s < f.len - 0.15 {
        let c = f.at(s, 0.0, -0.11);
        if posts % 8 == 0 {
            b.rbox(
                l.trim,
                [c[0] - 0.12, c[1] - 0.12, z + 0.18],
                [c[0] + 0.12, c[1] + 0.12, z + 0.85],
                0.0,
            );
        } else {
            b.cone(
                l.trim,
                [c[0], c[1], z + 0.18],
                [c[0], c[1], z + 0.48],
                0.05,
                0.085,
                6,
            );
            b.cone(
                l.trim,
                [c[0], c[1], z + 0.48],
                [c[0], c[1], z + 0.85],
                0.085,
                0.04,
                6,
            );
        }
        s += 0.19;
        posts += 1;
    }
    for x in [0.0, f.len] {
        let c = f.at(x.clamp(0.12, f.len - 0.12), 0.0, -0.11);
        b.rbox(
            l.trim,
            [c[0] - 0.12, c[1] - 0.12, z + 0.18],
            [c[0] + 0.12, c[1] + 0.12, z + 0.85],
            0.0,
        );
    }
}

/// Two slopes with the ridge along x.
fn gable(b: &mut Builder, r: &RoofPlan, l: &Look) {
    let (x, y, z) = (r.x, r.y, r.z);
    let verge = if r.party { 0.0 } else { r.eaves * 0.6 };
    let ym = (y.0 + y.1) / 2.0;
    let zr = z + (y.1 - ym) * r.pitch;
    if r.cover == Cover::Curved {
        tile_roof(b, x, y, z, r.pitch, (r.eaves, verge), (true, true));
    } else {
        let e = r.eaves;
        let ze = z - e * r.pitch;
        let (a, c) = (x.0 - verge, x.1 + verge);
        covered(
            b,
            l.roof,
            &[[a, y.1 + e, ze], [c, y.1 + e, ze], [c, ym, zr], [a, ym, zr]],
            r.cover,
        );
        covered(
            b,
            l.roof,
            &[[c, y.0 - e, ze], [a, y.0 - e, ze], [a, ym, zr], [c, ym, zr]],
            r.cover,
        );
        b.cone(
            l.roof,
            [a, ym, zr + 0.07],
            [c, ym, zr + 0.07],
            0.08,
            0.08,
            6,
        );
        gable_ends(b, x, y, z, zr, l);
    }
    dormers(b, r, l, |yy| z + (y.1 - yy) * r.pitch, false);
}

/// The triangles of wall at the ends of a roof with its ridge along x.
fn gable_ends(b: &mut Builder, x: (f64, f64), y: (f64, f64), z: f64, zr: f64, l: &Look) {
    let len = y.1 - y.0;
    for f in [
        Face {
            o: [x.0, y.0],
            u: [0.0, 1.0],
            len,
        },
        Face {
            o: [x.1, y.1],
            u: [0.0, -1.0],
            len,
        },
    ] {
        f.prism(
            b,
            l.wall,
            &[(0.0, z), (len, z), (len / 2.0, zr + 0.03)],
            (-0.3, 0.0),
        );
    }
}

/// Two slopes with the gable to the street, and that gable as the place
/// builds it.
fn to_street(b: &mut Builder, r: &RoofPlan, l: &Look) {
    let (x, y, z) = (r.x, r.y, r.z);
    let w = x.1 - x.0;
    let xm = (x.0 + x.1) / 2.0;
    let zr = z + w / 2.0 * r.pitch;
    let e = if r.party { 0.0 } else { r.eaves };
    let kind = match r.gable {
        Gable::Varied => [Gable::Stepped, Gable::Bell, Gable::Plain, Gable::Bell]
            [(unit(r.seed, 900) * 4.0) as usize % 4],
        g => g,
    };
    let fancy = kind != Gable::Plain;
    let front_y = if fancy { y.1 - 0.12 } else { y.1 + 0.25 };
    if r.cover == Cover::Curved {
        tile_roof_along_y(b, x, (y.0, y.1), z, r.pitch, (e, 0.2), (true, !fancy));
    } else {
        let ze = z - e * r.pitch;
        covered(
            b,
            l.roof,
            &[
                [x.0 - e, y.0 - 0.25, ze],
                [x.0 - e, front_y, ze],
                [xm, front_y, zr],
                [xm, y.0 - 0.25, zr],
            ],
            r.cover,
        );
        covered(
            b,
            l.roof,
            &[
                [x.1 + e, front_y, ze],
                [x.1 + e, y.0 - 0.25, ze],
                [xm, y.0 - 0.25, zr],
                [xm, front_y, zr],
            ],
            r.cover,
        );
        b.cone(
            l.roof,
            [xm, y.0 - 0.25, zr + 0.07],
            [xm, front_y, zr + 0.07],
            0.08,
            0.08,
            6,
        );
        // The back gable, plain.
        let back = Face {
            o: [x.1, y.0],
            u: [-1.0, 0.0],
            len: w,
        };
        back.prism(
            b,
            l.wall,
            &[(0.0, z), (w, z), (w / 2.0, zr + 0.03)],
            (-0.3, 0.0),
        );
        if !fancy {
            let front = Face {
                o: [x.0, y.1],
                u: [1.0, 0.0],
                len: w,
            };
            front.prism(
                b,
                l.wall,
                &[(0.0, z), (w, z), (w / 2.0, zr + 0.03)],
                (-0.3, 0.0),
            );
        }
    }
    let front = Face {
        o: [x.0, y.1],
        u: [1.0, 0.0],
        len: w,
    };
    street_gable(b, &front, z, zr, r.pitch, kind, l, r.extras & HOIST != 0);
}

/// The gable on the street: plain, stepped or bell-shaped, with the loft
/// door and the hoisting beam.
#[allow(clippy::too_many_arguments)]
fn street_gable(
    b: &mut Builder,
    f: &Face,
    z: f64,
    zr: f64,
    pitch: f64,
    kind: Gable,
    l: &Look,
    hoist: bool,
) {
    let w = f.len;
    let c = w / 2.0;
    let tri = |zz: f64| (w / 2.0 - (zz - z) / pitch).max(0.0);
    let d = (-0.3, 0.0);
    // The loft door.
    let door = if zr - z > 2.4 {
        Some(Hole {
            s: (c - 0.42, c + 0.42),
            z: (z + 0.45, z + 1.75),
        })
    } else {
        None
    };
    let slice = |b: &mut Builder, za: f64, zb: f64, half: f64| {
        let (a, e) = (c - half, c + half);
        match door {
            Some(h) if za < h.z.1 && zb > h.z.0 => {
                if h.s.0 > a {
                    f.bx(b, l.wall, (a, h.s.0), (za, zb), d);
                }
                if e > h.s.1 {
                    f.bx(b, l.wall, (h.s.1, e), (za, zb), d);
                }
            }
            _ => f.bx(b, l.wall, (a, e), (za, zb), d),
        }
    };
    let mut top = zr;
    match kind {
        Gable::Plain => {
            if let Some(h) = door {
                // Over the plain triangle (drawn with the roof): only the
                // door is to be opened in it, so it is drawn here in slices.
                let mut za = z;
                while za < zr {
                    let zb = (za + 0.15).min(zr + 0.03);
                    slice(b, za, zb, tri(za).max(0.05));
                    za = zb;
                }
                let _ = h;
            }
        }
        Gable::Stepped => {
            let hs = 0.55;
            let ds = hs / pitch * 0.95;
            let mut j = 0;
            loop {
                let za = z + hs * j as f64;
                let half = (w / 2.0 - ds * j as f64).max(0.42);
                let zb = za + hs;
                slice(b, za, zb, half);
                f.bx(
                    b,
                    l.trim,
                    (c - half - 0.03, c + half + 0.03),
                    (zb, zb + 0.06),
                    (-0.33, 0.04),
                );
                j += 1;
                if half <= 0.42 && zb > zr + 0.3 {
                    top = zb;
                    break;
                }
                if j > 30 {
                    top = zb;
                    break;
                }
            }
        }
        Gable::Bell | Gable::Varied => {
            let neck = (w * 0.2).max(0.6);
            let zt = zr + 0.55;
            let mut za = z;
            while za < zt - 1e-6 {
                let zb = (za + 0.12).min(zt);
                let t = ((zb - z) / (zt - z)).clamp(0.0, 1.0);
                let s = (1.0 - t / 0.6).max(0.0);
                let bell = neck + (w / 2.0 - neck) * s.powf(0.55);
                let half = bell.max(tri(za) + 0.12).max(neck).min(w / 2.0);
                slice(b, za, zb, half);
                za = zb;
            }
            f.prism(
                b,
                l.trim,
                &[(c - neck - 0.08, zt), (c + neck + 0.08, zt), (c, zt + 0.4)],
                (-0.33, 0.04),
            );
            f.bx(
                b,
                l.trim,
                (c - neck - 0.08, c + neck + 0.08),
                (zt - 0.06, zt),
                (-0.33, 0.06),
            );
            // Scrolls on the shoulders.
            let zs = z + (zt - z) * 0.6;
            for x in [c - neck - 0.15, c + neck + 0.15] {
                b.ellipsoid(l.trim, f.at(x, zs, 0.02), [0.14, 0.1, 0.14]);
            }
            top = zt + 0.4;
        }
    }
    let _ = top;
    if let Some(h) = door {
        let win = Win {
            leaves: 2,
            reveal: 0.1,
            ..Default::default()
        };
        window(b, f, h, &win, l.frame);
        surround(b, f, h, l.trim, 0.12, 0.015);
        if hoist {
            let zb = h.z.1 + 0.35;
            f.bx(
                b,
                l.door,
                (c - 0.07, c + 0.07),
                (zb, zb + 0.16),
                (-0.3, 0.95),
            );
            let hook = f.at(c, zb - 0.05, 0.85);
            b.cone(
                "metal",
                [hook[0], hook[1], zb],
                [hook[0], hook[1], zb - 0.35],
                0.012,
                0.012,
                4,
            );
        }
    }
}

/// Four slopes.
fn hip(b: &mut Builder, r: &RoofPlan, l: &Look) {
    if r.cover == Cover::Curved {
        hipped_roof(b, r.x, r.y, r.z, r.pitch, r.eaves);
    } else {
        let ze = r.z - r.eaves * r.pitch;
        let x = (r.x.0 - r.eaves, r.x.1 + r.eaves);
        let y = (r.y.0 - r.eaves, r.y.1 + r.eaves);
        for p in hip_planes(x, y, ze, r.pitch) {
            covered(b, l.roof, &p, r.cover);
        }
    }
    let (y, z, pitch) = (r.y, r.z, r.pitch);
    dormers(b, r, l, |yy| z + (y.1 - yy) * pitch, false);
}

/// The planes of a hipped roof over `x` × `y` from `z` at `pitch`.
fn hip_planes(x: (f64, f64), y: (f64, f64), z: f64, pitch: f64) -> Vec<Vec<P3>> {
    let along_x = x.1 - x.0 >= y.1 - y.0;
    let (ax, bx, ay, by) = (x.0, x.1, y.0, y.1);
    if along_x {
        let h = (by - ay) / 2.0;
        let ym = (ay + by) / 2.0;
        let zr = z + h * pitch;
        let (r0, r1) = (ax + h, bx - h);
        vec![
            vec![[ax, by, z], [bx, by, z], [r1, ym, zr], [r0, ym, zr]],
            vec![[bx, ay, z], [ax, ay, z], [r0, ym, zr], [r1, ym, zr]],
            vec![[ax, ay, z], [ax, by, z], [r0, ym, zr]],
            vec![[bx, by, z], [bx, ay, z], [r1, ym, zr]],
        ]
    } else {
        let h = (bx - ax) / 2.0;
        let xm = (ax + bx) / 2.0;
        let zr = z + h * pitch;
        let (r0, r1) = (ay + h, by - h);
        vec![
            vec![[ax, ay, z], [ax, by, z], [xm, r1, zr], [xm, r0, zr]],
            vec![[bx, by, z], [bx, ay, z], [xm, r0, zr], [xm, r1, zr]],
            vec![[bx, ay, z], [ax, ay, z], [xm, r0, zr]],
            vec![[ax, by, z], [bx, by, z], [xm, r1, zr]],
        ]
    }
}

/// A mansard: the steep slope with its dormers, the low one over it.
fn mansard(b: &mut Builder, r: &RoofPlan, l: &Look) {
    let (x, y, z) = (r.x, r.y, r.z);
    let (m, hm, e) = (0.8, 2.7, 0.12);
    let zb = z + hm;
    let low = 0.3;
    if r.party {
        covered(
            b,
            l.roof,
            &[
                [x.0, y.1 + e, z],
                [x.1, y.1 + e, z],
                [x.1, y.1 - m, zb],
                [x.0, y.1 - m, zb],
            ],
            r.cover,
        );
        covered(
            b,
            l.roof,
            &[
                [x.1, y.0 - e, z],
                [x.0, y.0 - e, z],
                [x.0, y.0 + m, zb],
                [x.1, y.0 + m, zb],
            ],
            r.cover,
        );
        let ym = (y.0 + y.1) / 2.0;
        let zr = zb + (y.1 - m - ym) * low;
        covered(
            b,
            l.roof,
            &[
                [x.0, y.1 - m, zb],
                [x.1, y.1 - m, zb],
                [x.1, ym, zr],
                [x.0, ym, zr],
            ],
            seam(r),
        );
        covered(
            b,
            l.roof,
            &[
                [x.1, y.0 + m, zb],
                [x.0, y.0 + m, zb],
                [x.0, ym, zr],
                [x.1, ym, zr],
            ],
            seam(r),
        );
        // The party walls up to the roof's profile.
        let len = y.1 - y.0;
        for f in [
            Face {
                o: [x.0, y.0],
                u: [0.0, 1.0],
                len,
            },
            Face {
                o: [x.1, y.1],
                u: [0.0, -1.0],
                len,
            },
        ] {
            f.prism(
                b,
                l.wall,
                &[
                    (0.0, z),
                    (len, z),
                    (len - m, zb + 0.02),
                    (len / 2.0, zr + 0.03),
                    (m, zb + 0.02),
                ],
                (-0.3, 0.0),
            );
        }
    } else {
        let (ax, bx, ay, by) = (x.0 - e, x.1 + e, y.0 - e, y.1 + e);
        let (ix, jx, iy, jy) = (x.0 + m, x.1 - m, y.0 + m, y.1 - m);
        for p in [
            [[ax, by, z], [bx, by, z], [jx, jy, zb], [ix, jy, zb]],
            [[bx, by, z], [bx, ay, z], [jx, iy, zb], [jx, jy, zb]],
            [[bx, ay, z], [ax, ay, z], [ix, iy, zb], [jx, iy, zb]],
            [[ax, ay, z], [ax, by, z], [ix, jy, zb], [ix, iy, zb]],
        ] {
            covered(b, l.roof, &p, r.cover);
        }
        for p in hip_planes((ix, jx), (iy, jy), zb, low) {
            covered(b, l.roof, &p, seam(r));
        }
    }
    // The cornice band at the break.
    let steep = hm / (m + e);
    let _ = steep;
    dormers(b, r, l, |yy| z + (y.1 + e - yy) * hm / (m + e), true);
}

/// The roof of Asia: four slopes of dark tiles with deep eaves, the ridge
/// raised at its ends and the corners turned up.
fn asian(b: &mut Builder, r: &RoofPlan) {
    let eaves = r.eaves.max(0.8);
    if r.light {
        let ze = r.z - eaves * r.pitch;
        let x = (r.x.0 - eaves, r.x.1 + eaves);
        let y = (r.y.0 - eaves, r.y.1 + eaves);
        for p in hip_planes(x, y, ze, r.pitch) {
            covered(b, "tiles", &p, Cover::Plain);
        }
        return;
    }
    hipped_roof(b, r.x, r.y, r.z, r.pitch, eaves);
    let (x, y) = (r.x, r.y);
    let ze = r.z - eaves * r.pitch;
    let along_x = x.1 - x.0 >= y.1 - y.0;
    let h = if along_x {
        (y.1 - y.0 + 2.0 * eaves) / 2.0
    } else {
        (x.1 - x.0 + 2.0 * eaves) / 2.0
    };
    let zr = ze + h * r.pitch + 0.12;
    let (cx, cy) = ((x.0 + x.1) / 2.0, (y.0 + y.1) / 2.0);
    let half = if along_x {
        ((x.1 - x.0) / 2.0 + eaves - h).max(0.2)
    } else {
        ((y.1 - y.0) / 2.0 + eaves - h).max(0.2)
    };
    let (p0, p1) = if along_x {
        ([cx - half, cy], [cx + half, cy])
    } else {
        ([cx, cy - half], [cx, cy + half])
    };
    b.rbox(
        "tiles",
        [p0[0].min(p1[0]) - 0.12, p0[1].min(p1[1]) - 0.12, zr - 0.05],
        [p0[0].max(p1[0]) + 0.12, p0[1].max(p1[1]) + 0.12, zr + 0.2],
        0.0,
    );
    for (p, sgn) in [(p0, -1.0), (p1, 1.0)] {
        let dir = if along_x { [sgn, 0.0] } else { [0.0, sgn] };
        b.cone(
            "tiles",
            [p[0], p[1], zr + 0.08],
            [p[0] + dir[0] * 0.45, p[1] + dir[1] * 0.45, zr + 0.6],
            0.13,
            0.05,
            6,
        );
    }
    for (cx2, cy2) in [
        (x.0 - eaves, y.0 - eaves),
        (x.1 + eaves, y.0 - eaves),
        (x.1 + eaves, y.1 + eaves),
        (x.0 - eaves, y.1 + eaves),
    ] {
        let dx = (cx2 - cx).signum();
        let dy = (cy2 - cy).signum();
        let a = [cx2 - dx * 0.5, cy2 - dy * 0.5, ze + 0.5 * r.pitch + 0.08];
        b.cone(
            "tiles",
            a,
            [cx2 + dx * 0.15, cy2 + dy * 0.15, ze + 0.38],
            0.09,
            0.04,
            6,
        );
    }
}

/// Dormers on the front (and the back) of a roof, on its axes; `zp` is
/// the height of the roof at a `y` of the front slope.
fn dormers(b: &mut Builder, r: &RoofPlan, l: &Look, zp: impl Fn(f64) -> f64, mansard: bool) {
    if r.dormers.is_empty() || (!mansard && r.extras & DORMERS == 0) {
        return;
    }
    let (x, y) = (r.x, r.y);
    let ym = (y.0 + y.1) / 2.0;
    let (cx, cy) = ((x.0 + x.1) / 2.0, ym);
    let backs = !r.party || mansard;
    for (k, &dx) in r.dormers.iter().enumerate() {
        if !mansard && r.dormers.len() > 2 && k % 2 == 1 {
            continue;
        }
        for back in [false, true] {
            if back && !backs {
                continue;
            }
            let start = b.mark();
            let xx = if back { 2.0 * cx - dx } else { dx };
            dormer(b, r, l, xx, &zp, mansard);
            if back {
                turn_from(b, start, [cx, cy], PI);
            }
        }
    }
}

/// One dormer on the front slope at `x`.
fn dormer(
    b: &mut Builder,
    r: &RoofPlan,
    l: &Look,
    x: f64,
    zp: &impl Fn(f64) -> f64,
    mansard: bool,
) {
    let y1 = r.y.1;
    let ym = (r.y.0 + r.y.1) / 2.0;
    let half = 0.6;
    let (yf, hd) = if mansard {
        (y1 - 0.25, 1.9)
    } else {
        (y1 - 0.9, 1.5)
    };
    let zb = zp(yf) - 0.05;
    let depth = if mansard {
        0.9
    } else {
        ((hd + 0.2) / r.pitch).min(yf - ym - 0.2).max(0.6)
    };
    let yb = yf - depth;
    if yb < ym - 0.01 || zb + hd > zp(ym) + 0.6 && !mansard {
        return;
    }
    let front = Face {
        o: [x - half, yf],
        u: [1.0, 0.0],
        len: 2.0 * half,
    };
    let hole = Hole {
        s: (0.22, 2.0 * half - 0.22),
        z: (zb + 0.25, zb + hd - 0.18),
    };
    wall(b, l.trim, &front, (zb, zb + hd), 0.1, &[hole]);
    let win = Win {
        leaves: 2,
        reveal: 0.06,
        ..Default::default()
    };
    window(b, &front, hole, &win, l.frame);
    let cheek = if mansard { l.roof } else { l.trim };
    b.rbox(
        cheek,
        [x - half, yb, zb],
        [x - half + 0.1, yf - 0.1, zb + hd],
        0.0,
    );
    b.rbox(
        cheek,
        [x + half - 0.1, yb, zb],
        [x + half, yf - 0.1, zb + hd],
        0.0,
    );
    b.rbox(
        l.wall,
        [x - half + 0.1, yb, zb],
        [x + half - 0.1, yb + 0.1, zb + hd],
        0.0,
    );
    // Its roof: two small slopes and the pediment.
    let zt = zb + hd;
    let rise = 0.42;
    let (a, e) = (x - half - 0.12, x + half + 0.12);
    covered(
        b,
        l.roof,
        &[
            [a, yf + 0.12, zt],
            [x, yf + 0.12, zt + rise],
            [x, yb, zt + rise],
            [a, yb, zt],
        ],
        seam(r),
    );
    covered(
        b,
        l.roof,
        &[
            [x, yf + 0.12, zt + rise],
            [e, yf + 0.12, zt],
            [e, yb, zt],
            [x, yb, zt + rise],
        ],
        seam(r),
    );
    front.prism(
        b,
        l.trim,
        &[(-0.05, zt), (2.0 * half + 0.05, zt), (half, zt + rise)],
        (-0.1, 0.04),
    );
}

/// Chimney stacks with their pots: on the party walls of a street's
/// house, on the ridge of the others.
fn chimneys(b: &mut Builder, r: &RoofPlan, l: &Look) {
    let (x, y, z) = (r.x, r.y, r.z);
    let ym = (y.0 + y.1) / 2.0;
    if r.form == Roof::Pent {
        let zh = z + (y.1 - y.0) * r.pitch * 0.6;
        for c in [[x.0 + 0.32, y.0 + 0.6], [x.1 - 0.32, y.0 + 0.6]] {
            b.rbox(
                l.wall,
                [c[0] - 0.3, c[1] - 0.5, z],
                [c[0] + 0.3, c[1] + 0.5, zh + 0.9],
                0.0,
            );
        }
        return;
    }
    let ridge = match r.form {
        Roof::Flat => z + 1.0,
        Roof::Mansard => z + 2.7 + 0.3 * (y.1 - 0.8 - ym),
        Roof::GableToStreet => z + (x.1 - x.0) / 2.0 * r.pitch,
        _ => z + (y.1 - ym) * r.pitch,
    };
    let spots: Vec<[f64; 2]> = if r.party {
        vec![[x.0 + 0.32, ym], [x.1 - 0.32, ym + 0.8]]
    } else {
        vec![[x.0 + (x.1 - x.0) * 0.3, ym]]
    };
    let role = if l.wall.starts_with('k') || l.wall.starts_with('s') {
        l.wall
    } else {
        "k_red"
    };
    for (i, c) in spots.iter().enumerate() {
        let z0 = if r.form == Roof::Flat { z } else { ridge - 1.2 };
        let z1 = ridge + 0.9;
        let half = [0.32, 0.55];
        b.rbox(
            role,
            [c[0] - half[0], c[1] - half[1], z0],
            [c[0] + half[0], c[1] + half[1], z1],
            0.0,
        );
        b.rbox(
            l.trim,
            [c[0] - half[0] - 0.05, c[1] - half[1] - 0.05, z1],
            [c[0] + half[0] + 0.05, c[1] + half[1] + 0.05, z1 + 0.1],
            0.0,
        );
        let pots = if r.light {
            0
        } else {
            2 + (unit(r.seed, 950 + i as u64) * 2.5) as usize
        };
        for k in 0..pots {
            let py = c[1] - half[1] + 0.2 + (2.0 * half[1] - 0.4) * (k as f64 + 0.5) / pots as f64;
            b.cone(
                "pot",
                [c[0], py, z1 + 0.1],
                [c[0], py, z1 + 0.45],
                0.1,
                0.08,
                10,
            );
        }
    }
}

/// A roof plane covered: its slab and the courses of slates or tiles, or
/// the seams of the metal.
pub fn covered(b: &mut Builder, role: &'static str, pts: &[P3], cover: Cover) {
    let n = newell(pts);
    let th = 0.07;
    plate_n(b, role, pts, n, th);
    let up = norm([-n[0] * n[2], -n[1] * n[2], 1.0 - n[2] * n[2]]);
    let along = norm(cross(up, n));
    let lifted: Vec<P3> = pts
        .iter()
        .map(|p| [p[0] + n[0] * th, p[1] + n[1] * th, p[2] + n[2] * th])
        .collect();
    let dot = |p: &P3, v: P3| p[0] * v[0] + p[1] * v[1] + p[2] * v[2];
    match cover {
        Cover::Courses | Cover::Curved => {
            let lo = lifted.iter().map(|p| dot(p, up)).fold(f64::MAX, f64::min);
            let hi = lifted.iter().map(|p| dot(p, up)).fold(f64::MIN, f64::max);
            let mut t = lo + 0.02;
            let mut k = 0;
            while t < hi - 0.04 {
                let part = clip(&lifted, up, t, (t + 0.21).min(hi));
                if part.len() >= 3 {
                    // Sunk a little into the slab, every other course
                    // standing out more.
                    let sink = -0.005;
                    let part: Vec<P3> = part
                        .iter()
                        .map(|p| [p[0] + n[0] * sink, p[1] + n[1] * sink, p[2] + n[2] * sink])
                        .collect();
                    plate_n(b, role, &part, n, 0.022 + 0.008 * (k % 2) as f64);
                }
                t += 0.24;
                k += 1;
            }
        }
        Cover::Bands => {
            let lo = lifted.iter().map(|p| dot(p, up)).fold(f64::MAX, f64::min);
            let hi = lifted.iter().map(|p| dot(p, up)).fold(f64::MIN, f64::max);
            let mut t = lo + 0.02;
            while t < hi - 0.1 {
                let part = clip(&lifted, up, t, (t + 0.6).min(hi));
                if part.len() >= 3 {
                    let sink = -0.005;
                    let part: Vec<P3> = part
                        .iter()
                        .map(|p| [p[0] + n[0] * sink, p[1] + n[1] * sink, p[2] + n[2] * sink])
                        .collect();
                    plate_n(b, role, &part, n, 0.03);
                }
                t += 0.72;
            }
        }
        Cover::Plain => {}
        Cover::Seam => {
            let lo = lifted
                .iter()
                .map(|p| dot(p, along))
                .fold(f64::MAX, f64::min);
            let hi = lifted
                .iter()
                .map(|p| dot(p, along))
                .fold(f64::MIN, f64::max);
            let mut t = lo + 0.25;
            while t < hi - 0.1 {
                let part = clip(&lifted, along, t - 0.018, t + 0.018);
                if part.len() >= 3 {
                    plate_n(b, role, &part, n, 0.04);
                }
                t += 0.45;
            }
        }
    }
}

/// The normal of a plane polygon (Newell), pointing up.
fn newell(pts: &[P3]) -> P3 {
    let mut n = [0.0; 3];
    for i in 0..pts.len() {
        let (p, q) = (pts[i], pts[(i + 1) % pts.len()]);
        n[0] += (p[1] - q[1]) * (p[2] + q[2]);
        n[1] += (p[2] - q[2]) * (p[0] + q[0]);
        n[2] += (p[0] - q[0]) * (p[1] + q[1]);
    }
    let n = norm(n);
    if n[2] < 0.0 {
        [-n[0], -n[1], -n[2]]
    } else {
        n
    }
}

/// A convex plane polygon made a slab `th` thick along `n`.
fn plate_n(b: &mut Builder, role: &'static str, pts: &[P3], n: P3, th: f64) {
    let m = pts.len();
    if m < 3 {
        return;
    }
    let top: Vec<P3> = pts
        .iter()
        .map(|p| [p[0] + n[0] * th, p[1] + n[1] * th, p[2] + n[2] * th])
        .collect();
    let mut c = [0.0; 3];
    for p in pts.iter().chain(&top) {
        for k in 0..3 {
            c[k] += p[k] / (2 * m) as f64;
        }
    }
    for i in 1..m - 1 {
        out_tri(b, role, [pts[0], pts[i], pts[i + 1]], c);
        out_tri(b, role, [top[0], top[i], top[i + 1]], c);
    }
    for i in 0..m {
        let j = (i + 1) % m;
        out_tri(b, role, [pts[i], pts[j], top[j]], c);
        out_tri(b, role, [pts[i], top[j], top[i]], c);
    }
}

/// The part of a convex polygon where `p · v` is between `lo` and `hi`.
fn clip(poly: &[P3], v: P3, lo: f64, hi: f64) -> Vec<P3> {
    let dot = |p: &P3| p[0] * v[0] + p[1] * v[1] + p[2] * v[2];
    let cut = |poly: Vec<P3>, keep: &dyn Fn(f64) -> f64| -> Vec<P3> {
        let mut out = Vec::new();
        for i in 0..poly.len() {
            let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
            let (a, e) = (keep(dot(&p)), keep(dot(&q)));
            if a >= 0.0 {
                out.push(p);
            }
            if (a >= 0.0) != (e >= 0.0) {
                let t = a / (a - e);
                out.push([
                    p[0] + (q[0] - p[0]) * t,
                    p[1] + (q[1] - p[1]) * t,
                    p[2] + (q[2] - p[2]) * t,
                ]);
            }
        }
        // Drop points that are one.
        out.dedup_by(|a, b| (a[0] - b[0]).abs() + (a[1] - b[1]).abs() + (a[2] - b[2]).abs() < 1e-7);
        out
    };
    let a = cut(poly.to_vec(), &|d| d - lo);
    cut(a, &|d| hi - d)
}

/// A solar water heater: a frame leaning south, its tubes and the tank.
pub(super) fn solar_heater(b: &mut Builder, c: [f64; 2], z: f64) {
    for dx in [-0.8, 0.8] {
        b.cone(
            "metal",
            [c[0] + dx, c[1] + 0.6, z],
            [c[0] + dx, c[1] - 0.5, z + 1.1],
            0.02,
            0.02,
            4,
        );
        b.cone(
            "metal",
            [c[0] + dx, c[1] - 0.5, z],
            [c[0] + dx, c[1] - 0.5, z + 1.1],
            0.02,
            0.02,
            4,
        );
    }
    for k in 0..12 {
        let x = c[0] - 0.8 + 1.6 * (k as f64 + 0.5) / 12.0;
        b.cone(
            "glass",
            [x, c[1] + 0.6, z + 0.05],
            [x, c[1] - 0.45, z + 1.02],
            0.045,
            0.045,
            6,
        );
    }
    b.cone(
        "appliance",
        [c[0] - 0.95, c[1] - 0.55, z + 1.18],
        [c[0] + 0.95, c[1] - 0.55, z + 1.18],
        0.2,
        0.2,
        12,
    );
}

/// The wooden water tank of a New York roof on its steel legs, with its
/// cone.
pub(super) fn water_tower(b: &mut Builder, c: [f64; 2], z: f64) {
    let h = 2.4;
    for (dx, dy) in [(-1.1, -1.1), (1.1, -1.1), (1.1, 1.1), (-1.1, 1.1)] {
        b.cyl(
            "iron",
            [c[0] + dx, c[1] + dy, z],
            [c[0] + dx, c[1] + dy, z + h],
            0.06,
        );
    }
    b.rbox(
        "iron",
        [c[0] - 1.3, c[1] - 1.3, z + h],
        [c[0] + 1.3, c[1] + 1.3, z + h + 0.12],
        0.0,
    );
    b.cone(
        "f_brown",
        [c[0], c[1], z + h + 0.12],
        [c[0], c[1], z + h + 3.0],
        1.25,
        1.2,
        20,
    );
    for k in 0..4 {
        let zz = z + h + 0.5 + 0.7 * k as f64;
        b.cone(
            "iron",
            [c[0], c[1], zz],
            [c[0], c[1], zz + 0.04],
            1.27,
            1.26,
            20,
        );
    }
    b.cone(
        "metal",
        [c[0], c[1], z + h + 3.0],
        [c[0], c[1], z + h + 3.9],
        1.35,
        0.05,
        20,
    );
}

/// The chimney of the Algarve: a white shaft and its lantern of lattice
/// with a little roof and a finial.
pub(super) fn algarve_chimney(b: &mut Builder, c: [f64; 2], z: f64, l: &Look) {
    b.rbox(
        l.wall,
        [c[0] - 0.35, c[1] - 0.35, z],
        [c[0] + 0.35, c[1] + 0.35, z + 1.4],
        0.0,
    );
    b.rbox(
        l.trim,
        [c[0] - 0.4, c[1] - 0.4, z + 1.4],
        [c[0] + 0.4, c[1] + 0.4, z + 1.46],
        0.0,
    );
    for side in faces((c[0] - 0.3, c[0] + 0.3), (c[1] - 0.3, c[1] + 0.3)) {
        grille_plane(
            b,
            &side,
            l.wall,
            (0.0, 0.6),
            (z + 1.46, z + 2.1),
            -0.02,
            0.1,
            3,
        );
    }
    b.rbox(
        l.wall,
        [c[0] - 0.3, c[1] - 0.3, z + 2.1],
        [c[0] + 0.3, c[1] + 0.3, z + 2.18],
        0.0,
    );
    b.cone(
        l.wall,
        [c[0], c[1], z + 2.18],
        [c[0], c[1], z + 2.5],
        0.3,
        0.05,
        4,
    );
    b.cone(
        l.trim,
        [c[0], c[1], z + 2.5],
        [c[0], c[1], z + 2.8],
        0.025,
        0.025,
        6,
    );
}

/// The cover of the small slopes: seams, or nothing from afar.
fn seam(r: &RoofPlan) -> Cover {
    if r.light {
        Cover::Plain
    } else {
        Cover::Seam
    }
}

/// One slope falling from the back (a party wall) to the front.
fn pent(b: &mut Builder, r: &RoofPlan, l: &Look) {
    let (x, y, z) = (r.x, r.y, r.z);
    let pitch = r.pitch * 0.6;
    let zh = z + (y.1 - y.0) * pitch;
    let e = r.eaves;
    let ze = z - e * pitch;
    let cover = if r.cover == Cover::Curved {
        Cover::Courses
    } else {
        r.cover
    };
    covered(
        b,
        l.roof,
        &[
            [x.0, y.1 + e, ze],
            [x.1, y.1 + e, ze],
            [x.1, y.0, zh],
            [x.0, y.0, zh],
        ],
        cover,
    );
    let len = y.1 - y.0;
    Face {
        o: [x.0, y.0],
        u: [0.0, 1.0],
        len,
    }
    .prism(
        b,
        l.wall,
        &[(0.0, z), (len, z), (0.0, zh + 0.03)],
        (-0.3, 0.0),
    );
    Face {
        o: [x.1, y.1],
        u: [0.0, -1.0],
        len,
    }
    .prism(
        b,
        l.wall,
        &[(0.0, z), (len, z), (len, zh + 0.03)],
        (-0.3, 0.0),
    );
    Face {
        o: [x.1, y.0],
        u: [-1.0, 0.0],
        len: x.1 - x.0,
    }
    .bx(b, l.wall, (0.0, x.1 - x.0), (z, zh + 0.03), (-0.3, 0.0));
}
