//! Buildings of the surroundings made with the care of the furniture
//! (taller de edificios, tanda 1): Spanish buildings first, the ones these
//! programs never bring. Each is a whole building as one model, with its
//! wall pierced where its windows are, the carpentry sitting back in the
//! reveal, the ironwork, the blinds half down, the awnings, the pots, the
//! clothes hung out and what is on the roof.
//!
//! Axes as the rest of the workshop: the street front towards `+y` (along
//! `y = depth`), the back along `y = 0`, centred on `x = 0`, the ground at
//! `z = 0`.
//!
//! Everything is drawn on a [`Face`]: a vertical plane of the building
//! with its own axes (`s` along it, from its left end seen from outside,
//! `z` up, `d` out of the wall), so the same window, balcony or grille can
//! go on any side.

pub mod block60;
pub mod canaria;
pub mod caserio;
pub mod castilla;
pub mod chalet80;
pub mod coast;
pub mod colegio;
pub mod colonizacion;
pub mod commerce;
pub mod corral;
pub mod cortijo;
pub mod ensanche;
pub mod galeria;
pub mod gasolinera;
pub mod ibicenca;
pub mod iglesia;
pub mod manor;
pub mod masia;
pub mod mercado;
pub mod more;
pub mod nave;
pub mod offices;
pub mod pareada;
pub mod salud;
pub mod terraced;
pub mod toros;
pub mod village;

use crate::build::{Builder, P3};

/// A number from 0 to 1 for `seed` and `k`, always the same: what makes
/// one blind lower than the next.
pub fn unit(seed: u64, k: u64) -> f64 {
    let mut x = seed
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(k.wrapping_mul(0xBF58_476D_1CE4_E5B9));
    x ^= x >> 31;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 29;
    (x % 10_000) as f64 / 10_000.0
}

/// A vertical face of a building: its left end `o` (seen from outside)
/// and its direction `u` in plan; out of the wall is `u` turned a quarter
/// to the left (`(-u.y, u.x)`).
#[derive(Clone, Copy, Debug)]
pub struct Face {
    pub o: [f64; 2],
    pub u: [f64; 2],
    pub len: f64,
}

impl Face {
    /// The four faces of a `w` × `d` building: front (the street), right,
    /// back and left, each seen from outside.
    pub fn sides(w: f64, d: f64) -> [Face; 4] {
        let x = w / 2.0;
        [
            Face {
                o: [-x, d],
                u: [1.0, 0.0],
                len: w,
            },
            Face {
                o: [x, d],
                u: [0.0, -1.0],
                len: d,
            },
            Face {
                o: [x, 0.0],
                u: [-1.0, 0.0],
                len: w,
            },
            Face {
                o: [-x, 0.0],
                u: [0.0, 1.0],
                len: d,
            },
        ]
    }

    pub fn out(&self) -> [f64; 2] {
        [-self.u[1], self.u[0]]
    }

    /// The point `s` along, `z` up and `d` out.
    pub fn at(&self, s: f64, z: f64, d: f64) -> P3 {
        let n = self.out();
        [
            self.o[0] + self.u[0] * s + n[0] * d,
            self.o[1] + self.u[1] * s + n[1] * d,
            z,
        ]
    }

    /// The same face moved `d` out (into the wall if negative).
    pub fn moved(&self, d: f64) -> Face {
        let n = self.out();
        Face {
            o: [self.o[0] + n[0] * d, self.o[1] + n[1] * d],
            ..*self
        }
    }

    /// A box from `s0` to `s1`, `z0` to `z1` and `d0` to `d1`.
    #[allow(clippy::too_many_arguments)]
    pub fn bx(
        &self,
        b: &mut Builder,
        role: &'static str,
        s: (f64, f64),
        z: (f64, f64),
        d: (f64, f64),
    ) {
        let (s0, s1) = (s.0.min(s.1), s.0.max(s.1));
        let (z0, z1) = (z.0.min(z.1), z.0.max(z.1));
        let (d0, d1) = (d.0.min(d.1), d.0.max(d.1));
        if s1 - s0 < 1e-4 || z1 - z0 < 1e-4 || d1 - d0 < 1e-4 {
            return;
        }
        let n = self.out();
        let c = self.at((s0 + s1) / 2.0, (z0 + z1) / 2.0, (d0 + d1) / 2.0);
        let hs = (s1 - s0) / 2.0;
        let hd = (d1 - d0) / 2.0;
        b.obox(
            role,
            c,
            [self.u[0] * hs, self.u[1] * hs, 0.0],
            [n[0] * hd, n[1] * hd, 0.0],
            [0.0, 0.0, (z1 - z0) / 2.0],
        );
    }

    /// A round bar from (`s0`, `z0`) to (`s1`, `z1`), `d` out, `r` thick,
    /// with `seg` sides.
    #[allow(clippy::too_many_arguments)]
    pub fn bar(
        &self,
        b: &mut Builder,
        role: &'static str,
        a: (f64, f64),
        e: (f64, f64),
        d: f64,
        r: f64,
        seg: usize,
    ) {
        b.cone(role, self.at(a.0, a.1, d), self.at(e.0, e.1, d), r, r, seg);
    }

    /// A solid whose section in the face (`s`, `z`, convex) runs from `d0`
    /// to `d1`: a gable, the piece of wall over an arch.
    pub fn prism(&self, b: &mut Builder, role: &'static str, pts: &[(f64, f64)], d: (f64, f64)) {
        let n = pts.len();
        if n < 3 {
            return;
        }
        let front: Vec<P3> = pts.iter().map(|p| self.at(p.0, p.1, d.1)).collect();
        let back: Vec<P3> = pts.iter().map(|p| self.at(p.0, p.1, d.0)).collect();
        let centre = {
            let mut c = [0.0; 3];
            for p in front.iter().chain(&back) {
                for k in 0..3 {
                    c[k] += p[k] / (2 * n) as f64;
                }
            }
            c
        };
        for i in 1..n - 1 {
            out_tri(b, role, [front[0], front[i], front[i + 1]], centre);
            out_tri(b, role, [back[0], back[i], back[i + 1]], centre);
        }
        for i in 0..n {
            let j = (i + 1) % n;
            out_tri(b, role, [back[i], back[j], front[j]], centre);
            out_tri(b, role, [back[i], front[j], front[i]], centre);
        }
    }

    /// A slab leaning out of the face: from the line `z0` at `d0` to the
    /// line `z1` at `d1`, `s0` to `s1`, `th` thick (an awning, a canopy).
    #[allow(clippy::too_many_arguments)]
    pub fn leaning(
        &self,
        b: &mut Builder,
        role: &'static str,
        s: (f64, f64),
        (z0, d0): (f64, f64),
        (z1, d1): (f64, f64),
        th: f64,
    ) {
        let pts = [(d0, z0), (d1, z1), (d1, z1 + th), (d0, z0 + th)];
        // Its section is across the face: draw it on the face turned.
        let side = Face {
            o: [self.o[0] + self.u[0] * s.0, self.o[1] + self.u[1] * s.0],
            u: self.out(),
            len: 1.0,
        };
        // On `side`, out is −u of this face: from s.0 back to s.1 is −d.
        side.prism(b, role, &pts, (-(s.1 - s.0), 0.0));
    }
}

/// A triangle facing away from `centre`.
pub(crate) fn out_tri(b: &mut Builder, role: &'static str, t: [P3; 3], centre: P3) {
    let e1 = [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]];
    let e2 = [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]];
    let nrm = crate::build::cross(e1, e2);
    let m = [
        (t[0][0] + t[1][0] + t[2][0]) / 3.0 - centre[0],
        (t[0][1] + t[1][1] + t[2][1]) / 3.0 - centre[1],
        (t[0][2] + t[1][2] + t[2][2]) / 3.0 - centre[2],
    ];
    let a = b.point(t[0]);
    let p = b.point(t[1]);
    let q = b.point(t[2]);
    if nrm[0] * m[0] + nrm[1] * m[1] + nrm[2] * m[2] >= 0.0 {
        b.tri(role, a, p, q);
    } else {
        b.tri(role, a, q, p);
    }
}

/// A hole in a wall: from `s0` to `s1` and `z0` to `z1`.
#[derive(Clone, Copy, Debug)]
pub struct Hole {
    pub s: (f64, f64),
    pub z: (f64, f64),
}

/// The wall of a face from `z0` to `z1`, `thick` into the building, with
/// its holes: bands between the holes' heads and sills, each cut where the
/// holes are.
pub fn wall(
    b: &mut Builder,
    role: &'static str,
    f: &Face,
    z: (f64, f64),
    thick: f64,
    holes: &[Hole],
) {
    let mut zs = vec![z.0, z.1];
    for h in holes {
        zs.push(h.z.0.clamp(z.0, z.1));
        zs.push(h.z.1.clamp(z.0, z.1));
    }
    zs.sort_by(f64::total_cmp);
    zs.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    for k in 0..zs.len() - 1 {
        let (za, zb) = (zs[k], zs[k + 1]);
        let mut cuts: Vec<(f64, f64)> = holes
            .iter()
            .filter(|h| h.z.0 <= za + 1e-6 && h.z.1 >= zb - 1e-6)
            .map(|h| h.s)
            .collect();
        cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut s = 0.0;
        for (c0, c1) in cuts {
            if c0 > s {
                f.bx(b, role, (s, c0), (za, zb), (-thick, 0.0));
            }
            s = s.max(c1);
        }
        if s < f.len {
            f.bx(b, role, (s, f.len), (za, zb), (-thick, 0.0));
        }
    }
}

/// The dark inside of a building seen through its glass: the core inside
/// its walls, from `z0` to `z1`.
pub fn core(b: &mut Builder, w: f64, d: f64, thick: f64, z: (f64, f64)) {
    let x = w / 2.0 - thick;
    b.rbox("interior", [-x, thick, z.0], [x, d - thick, z.1], 0.0);
}

/// How a window opens and what it has.
#[derive(Clone, Copy, Debug)]
pub struct Win {
    /// Leaves across (1 to 4).
    pub leaves: usize,
    /// How far back the frame sits from the face.
    pub reveal: f64,
    /// A fixed light over the leaves (a fanlight), its height.
    pub fanlight: f64,
    /// Glazing bars across each leaf (0 none).
    pub bars: usize,
    /// The roller blind down this much of its height (0 up, 1 shut).
    pub blind: f64,
    /// The roller blind's box on the outside (cajón visto).
    pub blind_box: bool,
    /// A net curtain behind the glass.
    pub curtain: bool,
    /// Sliding leaves (aluminium) instead of casements.
    pub sliding: bool,
}

impl Default for Win {
    fn default() -> Self {
        Self {
            leaves: 2,
            reveal: 0.1,
            fanlight: 0.0,
            bars: 0,
            blind: 0.0,
            blind_box: false,
            curtain: false,
            sliding: false,
        }
    }
}

/// The carpentry of a hole (`frame` its role): frame, leaves, glass, the
/// blind and the net curtain; the inside of the hole is the wall's.
pub fn window(b: &mut Builder, f: &Face, h: Hole, w: &Win, frame: &'static str) {
    let (s0, s1) = h.s;
    let (z0, z1) = h.z;
    let r = w.reveal;
    let fw = 0.05;
    let depth = (-r - 0.065, -r);
    // The frame round the hole.
    f.bx(b, frame, (s0, s0 + fw), (z0, z1), depth);
    f.bx(b, frame, (s1 - fw, s1), (z0, z1), depth);
    f.bx(b, frame, (s0 + fw, s1 - fw), (z1 - fw, z1), depth);
    f.bx(b, frame, (s0 + fw, s1 - fw), (z0, z0 + fw * 1.4), depth);
    let top = if w.fanlight > 0.05 {
        let t = z1 - fw - w.fanlight;
        f.bx(b, frame, (s0 + fw, s1 - fw), (t - fw, t), depth);
        f.bx(
            b,
            "glass",
            (s0 + fw, s1 - fw),
            (t, z1 - fw),
            (-r - 0.045, -r - 0.035),
        );
        t - fw
    } else {
        z1 - fw
    };
    let bottom = z0 + fw * 1.4;
    let n = w.leaves.clamp(1, 4);
    let lw = (s1 - s0 - 2.0 * fw) / n as f64;
    for k in 0..n {
        let a = s0 + fw + lw * k as f64;
        let e = a + lw;
        // Each leaf: its own frame a little forward of the main one
        // (sliding leaves one behind the other).
        let lf = 0.045;
        let back = if w.sliding && k % 2 == 1 { 0.03 } else { 0.0 };
        let ld = (-r - 0.05 - back, -r + 0.008 - back);
        let (a, e) = if w.sliding {
            let a = if k > 0 { a - lf * 0.5 } else { a };
            let e = if k + 1 < n { e + lf * 0.5 } else { e };
            (a, e)
        } else {
            (a, e)
        };
        f.bx(b, frame, (a, a + lf), (bottom, top), ld);
        f.bx(b, frame, (e - lf, e), (bottom, top), ld);
        f.bx(b, frame, (a + lf, e - lf), (top - lf, top), ld);
        f.bx(b, frame, (a + lf, e - lf), (bottom, bottom + lf * 1.3), ld);
        f.bx(
            b,
            "glass",
            (a + lf, e - lf),
            (bottom + lf * 1.3, top - lf),
            (ld.0 + 0.012, ld.0 + 0.022),
        );
        for i in 1..=w.bars {
            let zb = bottom + (top - bottom) * i as f64 / (w.bars + 1) as f64;
            f.bx(
                b,
                frame,
                (a + lf, e - lf),
                (zb - 0.012, zb + 0.012),
                (ld.0 + 0.005, ld.1 - 0.012),
            );
        }
    }
    // The net curtain, gathered, behind the glass.
    if w.curtain {
        let cw = (s1 - s0) * 0.42;
        for (a, e) in [(s0 + fw, s0 + fw + cw), (s1 - fw - cw, s1 - fw)] {
            f.bx(b, "curtain", (a, e), (bottom, top), (-r - 0.11, -r - 0.075));
            // Touching the frame: hung from its head.
            f.bx(
                b,
                "curtain",
                (a, e),
                (top - 0.02, z1 - fw),
                (-r - 0.075, -r - 0.065),
            );
        }
    }
    // The roller blind: guides in the reveal and the slats down to where
    // they are.
    if w.blind > 0.01 || w.blind_box {
        let g = 0.022;
        let gd = (-r, -r + 0.035);
        f.bx(b, "blind", (s0, s0 + g), (z0, z1), gd);
        f.bx(b, "blind", (s1 - g, s1), (z0, z1), gd);
        let down = w.blind.clamp(0.0, 1.0);
        if down > 0.01 {
            let low = z1 - (z1 - z0) * down;
            let bd = (-r + 0.008, -r + 0.024);
            f.bx(b, "blind", (s0 + g * 0.5, s1 - g * 0.5), (low, z1), bd);
            let mut zz = z1 - 0.055;
            while zz > low + 0.03 {
                f.bx(
                    b,
                    "blind",
                    (s0 + g, s1 - g),
                    (zz - 0.006, zz),
                    (bd.1, bd.1 + 0.006),
                );
                zz -= 0.055;
            }
            f.bx(
                b,
                "blind",
                (s0 + g * 0.5, s1 - g * 0.5),
                (low, low + 0.04),
                (bd.0, bd.1 + 0.01),
            );
        }
        if w.blind_box {
            f.bx(
                b,
                "blind",
                (s0 - 0.02, s1 + 0.02),
                (z1, z1 + 0.2),
                (-0.01, 0.06),
            );
        }
    }
}

/// A sill under a hole: `role`, `out` out of the face.
pub fn sill(b: &mut Builder, f: &Face, h: Hole, role: &'static str, out: f64, reveal: f64) {
    f.bx(
        b,
        role,
        (h.s.0 - 0.04, h.s.1 + 0.04),
        (h.z.0 - 0.04, h.z.0),
        (-reveal, out),
    );
}

/// A band round a hole (recercado), `wide` and `out` of the face.
pub fn surround(b: &mut Builder, f: &Face, h: Hole, role: &'static str, wide: f64, out: f64) {
    let (s0, s1) = h.s;
    let (z0, z1) = h.z;
    f.bx(b, role, (s0 - wide, s0), (z0, z1 + wide), (-0.005, out));
    f.bx(b, role, (s1, s1 + wide), (z0, z1 + wide), (-0.005, out));
    f.bx(b, role, (s0, s1), (z1, z1 + wide), (-0.005, out));
}

/// A grille: flat bars on the face's plane `d` out, from `s0` to `s1`
/// and `z0` to `z1`; `step` between the uprights and `rails` across.
#[allow(clippy::too_many_arguments)]
pub fn grille_plane(
    b: &mut Builder,
    f: &Face,
    role: &'static str,
    s: (f64, f64),
    z: (f64, f64),
    d: f64,
    step: f64,
    rails: usize,
) {
    let t = 0.012;
    let n = ((s.1 - s.0) / step).round().max(1.0) as usize;
    for i in 0..=n {
        let x = s.0 + (s.1 - s.0) * i as f64 / n as f64;
        f.bx(b, role, (x - t, x + t), z, (d - t, d + t));
    }
    for k in 0..rails.max(2) {
        let zz = z.0 + (z.1 - z.0) * k as f64 / (rails.max(2) - 1) as f64;
        f.bx(b, role, s, (zz - 0.018, zz + 0.018), (d - 0.016, d + 0.016));
    }
}

/// An Andalusian grille standing out of the face (reja voladiza): a cage
/// round a hole, with its sides back to the wall, a bar of balls on top
/// and a small crest.
pub fn projecting_grille(b: &mut Builder, f: &Face, h: Hole, role: &'static str, out: f64) {
    let (s0, s1) = (h.s.0 - 0.08, h.s.1 + 0.08);
    let (z0, z1) = (h.z.0 - 0.12, h.z.1 + 0.1);
    grille_plane(b, f, role, (s0, s1), (z0, z1), out, 0.12, 4);
    // The sides, back to the wall.
    let side_l = Face {
        o: f.at(s0, 0.0, out).map_xy(),
        u: [-f.out()[0], -f.out()[1]],
        len: out,
    };
    let side_r = Face {
        o: f.at(s1, 0.0, 0.0).map_xy(),
        u: f.out(),
        len: out,
    };
    for side in [side_l, side_r] {
        grille_plane(b, &side, role, (0.0, out), (z0, z1), 0.0, 0.11, 4);
    }
    // The top and the floor of the cage back to the wall.
    for zz in [z0, z1] {
        let n = ((s1 - s0) / 0.36).round().max(1.0) as usize;
        for i in 0..=n {
            let x = s0 + (s1 - s0) * i as f64 / n as f64;
            f.bx(
                b,
                role,
                (x - 0.012, x + 0.012),
                (zz - 0.015, zz + 0.015),
                (0.0, out),
            );
        }
    }
    // A crest of small scrolls and balls along the front.
    let n = ((s1 - s0) / 0.24).round().max(1.0) as usize;
    for i in 0..n {
        let x = s0 + (s1 - s0) * (i as f64 + 0.5) / n as f64;
        let c = f.at(x, z1 + 0.07, out);
        b.ellipsoid(role, c, [0.035, 0.035, 0.035]);
        f.bar(b, role, (x - 0.06, z1), (x, z1 + 0.05), out, 0.009, 5);
        f.bar(b, role, (x + 0.06, z1), (x, z1 + 0.05), out, 0.009, 5);
    }
    for x in [s0, s1] {
        b.ellipsoid(role, f.at(x, z1 + 0.06, out), [0.04, 0.04, 0.04]);
    }
}

trait Xy {
    fn map_xy(self) -> [f64; 2];
}

impl Xy for P3 {
    fn map_xy(self) -> [f64; 2] {
        [self[0], self[1]]
    }
}

/// An iron railing from `s0` to `s1` at `z` (its foot), `d` out of the
/// face: handrail, foot bar and balusters, with knobs on its posts.
#[allow(clippy::too_many_arguments)]
pub fn railing(
    b: &mut Builder,
    f: &Face,
    role: &'static str,
    s: (f64, f64),
    z: f64,
    d: f64,
    high: f64,
    step: f64,
) {
    let (s0, s1) = s;
    f.bx(
        b,
        role,
        (s0, s1),
        (z + high - 0.035, z + high),
        (d - 0.025, d + 0.025),
    );
    f.bx(
        b,
        role,
        (s0, s1),
        (z + 0.06, z + 0.09),
        (d - 0.012, d + 0.012),
    );
    let n = ((s1 - s0) / step).round().max(1.0) as usize;
    for i in 0..=n {
        let x = s0 + (s1 - s0) * i as f64 / n as f64;
        let t = if i == 0 || i == n { 0.02 } else { 0.008 };
        f.bx(
            b,
            role,
            (x - t, x + t),
            (z, z + high - 0.035),
            (d - t, d + t),
        );
    }
}

/// A balcony: its slab with a moulded edge and two brackets, `out` deep,
/// and its iron railing round the front and the sides.
#[allow(clippy::too_many_arguments)]
pub fn balcony(
    b: &mut Builder,
    f: &Face,
    s: (f64, f64),
    floor: f64,
    out: f64,
    slab: &'static str,
    iron: &'static str,
    brackets: bool,
) {
    let (s0, s1) = s;
    f.bx(b, slab, (s0, s1), (floor - 0.1, floor), (-0.02, out));
    f.bx(
        b,
        slab,
        (s0 + 0.04, s1 - 0.04),
        (floor - 0.16, floor - 0.1),
        (-0.02, out - 0.05),
    );
    if brackets {
        for x in [s0 + 0.12, s1 - 0.12] {
            let side = Face {
                o: f.at(x, 0.0, 0.0).map_xy(),
                u: f.out(),
                len: out,
            };
            let pts = [
                (-0.02, floor - 0.16),
                (out * 0.55, floor - 0.16),
                (out * 0.12, floor - 0.32),
                (-0.02, floor - 0.46),
            ];
            side.prism(b, slab, &pts, (-0.055, 0.055));
        }
    }
    let d = out - 0.05;
    railing(b, f, iron, (s0 + 0.04, s1 - 0.04), floor, d, 1.0, 0.11);
    for x in [s0 + 0.04, s1 - 0.04] {
        let side = Face {
            o: f.at(x, 0.0, 0.0).map_xy(),
            u: f.out(),
            len: d,
        };
        // On the side face `s` is out of the wall; its own `d` is along
        // this face: zero is on the line of the front's post.
        railing(b, &side, iron, (0.0, d), floor, 0.0, 1.0, 0.11);
    }
}

/// A flower pot hung on the face (the pots of the patios and the white
/// villages): its hook, the terracotta pot and the geranium.
pub fn hung_pot(b: &mut Builder, f: &Face, s: f64, z: f64, seed: u64) {
    f.bx(
        b,
        "iron",
        (s - 0.01, s + 0.01),
        (z + 0.15, z + 0.18),
        (-0.01, 0.12),
    );
    let c = f.at(s, z, 0.1);
    b.cone(
        "pot",
        [c[0], c[1], z - 0.02],
        [c[0], c[1], z + 0.17],
        0.075,
        0.1,
        10,
    );
    let leaf = [0.15 + 0.05 * unit(seed, 1), 0.15, 0.12];
    b.blob(
        "leaves",
        [c[0], c[1], z + 0.22],
        [leaf[0], 0.0, 0.0],
        [0.0, leaf[1], 0.0],
        [0.0, 0.0, leaf[2]],
    );
    for k in 0..4 {
        let a = std::f64::consts::TAU * (k as f64 + unit(seed, 2 + k)) / 4.0;
        b.ellipsoid(
            "flowers",
            [
                c[0] + 0.1 * a.cos(),
                c[1] + 0.1 * a.sin(),
                z + 0.27 + 0.04 * unit(seed, 9 + k),
            ],
            [0.04, 0.04, 0.035],
        );
    }
}

/// A pot standing on a floor (a balcony, a doorstep).
pub fn standing_pot(b: &mut Builder, c: P3, size: f64, seed: u64) {
    b.cone(
        "pot",
        c,
        [c[0], c[1], c[2] + 0.3 * size],
        0.13 * size,
        0.18 * size,
        10,
    );
    b.blob(
        "leaves",
        [c[0], c[1], c[2] + 0.42 * size],
        [0.22 * size, 0.0, 0.0],
        [0.0, 0.2 * size, 0.0],
        [0.0, 0.0, 0.24 * size],
    );
    if unit(seed, 3) > 0.4 {
        for k in 0..3 {
            let a = std::f64::consts::TAU * (k as f64 + unit(seed, 4 + k)) / 3.0;
            b.ellipsoid(
                "flowers",
                [
                    c[0] + 0.13 * size * a.cos(),
                    c[1] + 0.13 * size * a.sin(),
                    c[2] + 0.55 * size,
                ],
                [0.045 * size, 0.045 * size, 0.04 * size],
            );
        }
    }
}

/// An air conditioning unit on brackets on the face (`z` its foot).
pub fn ac_unit(b: &mut Builder, f: &Face, s: f64, z: f64) {
    let (w, h, dd) = (0.8, 0.55, 0.3);
    for x in [s - w / 2.0 + 0.08, s + w / 2.0 - 0.08] {
        f.bx(
            b,
            "metal",
            (x - 0.015, x + 0.015),
            (z - 0.04, z),
            (0.0, dd + 0.05),
        );
    }
    f.bx(
        b,
        "appliance",
        (s - w / 2.0, s + w / 2.0),
        (z, z + h),
        (0.04, 0.04 + dd),
    );
    let c = f.at(s + 0.08, z + h / 2.0, 0.04 + dd);
    let n = f.out();
    b.cone(
        "metal",
        [c[0] - n[0] * 0.01, c[1] - n[1] * 0.01, c[2]],
        [c[0] + n[0] * 0.006, c[1] + n[1] * 0.006, c[2]],
        0.2,
        0.2,
        16,
    );
}

/// A clothes line out of a window: two brackets, four lines and what is
/// hung on them.
pub fn clothes_line(b: &mut Builder, f: &Face, s: (f64, f64), z: f64, seed: u64) {
    let out = 0.55;
    for x in [s.0, s.1] {
        f.bx(
            b,
            "metal",
            (x - 0.012, x + 0.012),
            (z - 0.012, z + 0.012),
            (-0.01, out),
        );
        f.bar(b, "metal", (x, z - 0.3), (x, z), 0.02, 0.01, 5);
        f.bar(b, "metal", (x, z - 0.3), (x, z - 0.02), out * 0.6, 0.008, 5);
        f.bx(
            b,
            "metal",
            (x - 0.008, x + 0.008),
            (z - 0.31, z - 0.29),
            (-0.01, out * 0.6),
        );
    }
    let lines = [0.12, 0.25, 0.38, 0.51];
    for dd in lines {
        f.bar(b, "line", (s.0, z), (s.1, z), dd, 0.004, 4);
    }
    let roles = ["cloth_a", "cloth_b", "cloth_c"];
    let mut x = s.0 + 0.1;
    let mut k = 0;
    while x < s.1 - 0.25 {
        let wdt = 0.22 + 0.3 * unit(seed, 20 + k);
        let e = (x + wdt).min(s.1 - 0.05);
        let dd = lines[(unit(seed, 40 + k) * 4.0) as usize % 4];
        let hang = 0.3 + 0.35 * unit(seed, 60 + k);
        let role = roles[(unit(seed, 80 + k) * 3.0) as usize % 3];
        f.bx(b, role, (x, e), (z - hang, z), (dd - 0.006, dd + 0.006));
        x = e + 0.06 + 0.2 * unit(seed, 100 + k);
        k += 1;
    }
}

/// A striped awning over a hole (toldo de brazos): the box on the wall,
/// the canvas leaning out with its valance and the two arms.
pub fn awning(b: &mut Builder, f: &Face, s: (f64, f64), z: f64, out: f64, drop: f64) {
    let (s0, s1) = (s.0 - 0.08, s.1 + 0.08);
    f.bx(b, "metal", (s0, s1), (z, z + 0.12), (-0.005, 0.13));
    let n = ((s1 - s0) / 0.25).round().max(1.0) as usize;
    let wst = (s1 - s0) / n as f64;
    for i in 0..n {
        let role = if i % 2 == 0 { "awning" } else { "awning_b" };
        let a = s0 + wst * i as f64;
        f.leaning(
            b,
            role,
            (a, a + wst),
            (z + 0.04, 0.1),
            (z - drop, out),
            0.012,
        );
        f.bx(
            b,
            role,
            (a, a + wst),
            (z - drop - 0.22, z - drop + 0.01),
            (out - 0.012, out + 0.006),
        );
    }
    for x in [s0 + 0.06, s1 - 0.06] {
        let a = f.at(x, z - drop - 0.5, 0.0);
        let e = f.at(x, z - drop, out);
        b.cone("metal", a, e, 0.014, 0.014, 6);
    }
}

/// A rainwater pipe down the face at `s`, from `z1` to the ground, with
/// its brackets.
pub fn downpipe(b: &mut Builder, f: &Face, s: f64, z1: f64) {
    f.bar(b, "metal", (s, 0.05), (s, z1), 0.06, 0.045, 10);
    let mut z = 0.6;
    while z < z1 {
        f.bx(
            b,
            "metal",
            (s - 0.05, s + 0.05),
            (z, z + 0.03),
            (-0.005, 0.03),
        );
        z += 1.6;
    }
}

/// A gable roof of curved tiles over a `w` × `d` plan from `y0` to `y1`,
/// the ridge along `x`: its two slopes from the eaves at `z` (the top of
/// the walls) at `pitch` (rise over run), `eaves` out over the walls at
/// front and back and `verge` at the ends; the courses of tiles, the ridge
/// and the closed gables (`gables`).
#[allow(clippy::too_many_arguments)]
pub fn tile_roof(
    b: &mut Builder,
    (x0, x1): (f64, f64),
    (y0, y1): (f64, f64),
    z: f64,
    pitch: f64,
    (eaves, verge): (f64, f64),
    gables: (bool, bool),
) {
    let ym = (y0 + y1) / 2.0;
    let rise = (y1 - ym) * pitch;
    let th = 0.08;
    let hyp = (1.0 + pitch * pitch).sqrt();
    for front in [true, false] {
        let sgn = if front { 1.0 } else { -1.0 };
        let ye = if front { y1 + eaves } else { y0 - eaves };
        let ze = z - eaves * pitch;
        let zr = z + rise;
        // The slab on the plane through the top of the walls.
        let nrm = [0.0, sgn * pitch / hyp, 1.0 / hyp];
        let c = [
            (x0 + x1) / 2.0,
            (ye + ym) / 2.0 + nrm[1] * th / 2.0,
            (ze + zr) / 2.0 + nrm[2] * th / 2.0,
        ];
        b.obox(
            "roof",
            c,
            [(x1 - x0) / 2.0 + verge, 0.0, 0.0],
            [0.0, (ym - ye) / 2.0, (zr - ze) / 2.0],
            [nrm[0] * th / 2.0, nrm[1] * th / 2.0, nrm[2] * th / 2.0],
        );
        // The courses of cover tiles, eaves to ridge, a little past the
        // eaves.
        let n = (((x1 - x0) + 2.0 * verge) / 0.21).round().max(1.0) as usize;
        let r = 0.065;
        let lift = th + r * 0.55;
        for i in 0..n {
            let x = x0 - verge + ((x1 - x0) + 2.0 * verge) * (i as f64 + 0.5) / n as f64;
            let past = 0.06;
            let a = [
                x,
                ye + sgn * past + nrm[1] * lift,
                ze - past * pitch + nrm[2] * lift,
            ];
            let e = [x, ym + nrm[1] * lift, zr + nrm[2] * lift];
            b.cone("tiles", a, e, r, r * 0.9, 6);
        }
    }
    // The ridge.
    b.cone(
        "tiles",
        [x0 - verge - 0.03, ym, z + rise + th + 0.04],
        [x1 + verge + 0.03, ym, z + rise + th + 0.04],
        0.11,
        0.11,
        8,
    );
    // The gables: triangles of wall up to the slopes.
    for (on, x, sgn) in [(gables.0, x0, 1.0), (gables.1, x1, -1.0)] {
        if !on {
            continue;
        }
        let f = Face {
            o: [x, y0],
            u: [0.0, 1.0],
            len: y1 - y0,
        };
        let f = if sgn > 0.0 {
            f
        } else {
            Face {
                o: [x, y1],
                u: [0.0, -1.0],
                len: y1 - y0,
            }
        };
        let pts = [
            (0.0, z),
            (y1 - y0, z),
            ((y1 - y0) / 2.0, z + rise + th * 0.5),
        ];
        // Its thickness into the building (out is −x for the left end).
        f.prism(b, "wall", &pts, (-0.3, 0.0));
    }
    b.fine.insert("tiles");
}

/// A flat roof's parapet (pretil) round a `w` × `d` plan at `z`, `high`,
/// with its coping.
pub fn parapet(
    b: &mut Builder,
    w: f64,
    d: f64,
    z: f64,
    high: f64,
    wall: &'static str,
    coping: &'static str,
) {
    for f in Face::sides(w, d) {
        f.bx(b, wall, (0.0, f.len), (z, z + high), (-0.18, 0.0));
        f.bx(
            b,
            coping,
            (-0.03, f.len + 0.03),
            (z + high, z + high + 0.05),
            (-0.21, 0.03),
        );
    }
}

/// A TV aerial on a flat roof: a mast with its arms and dishes.
pub fn aerial(b: &mut Builder, c: [f64; 2], z: f64, seed: u64) {
    let h = 2.2 + 1.2 * unit(seed, 1);
    b.cyl("metal", [c[0], c[1], z], [c[0], c[1], z + h], 0.022);
    for k in 0..3 {
        let zz = z + h - 0.25 - 0.45 * k as f64;
        let l = 0.5 + 0.3 * unit(seed, 2 + k);
        let a = unit(seed, 7) * std::f64::consts::PI;
        let (s, co) = a.sin_cos();
        b.cyl(
            "metal",
            [c[0] - co * l, c[1] - s * l, zz],
            [c[0] + co * l, c[1] + s * l, zz],
            0.01,
        );
        for j in 0..5 {
            let t = -l + 2.0 * l * (j as f64 + 0.5) / 5.0;
            b.cyl(
                "metal",
                [c[0] + co * t + s * 0.14, c[1] + s * t - co * 0.14, zz],
                [c[0] + co * t - s * 0.14, c[1] + s * t + co * 0.14, zz],
                0.006,
            );
        }
    }
    if unit(seed, 9) > 0.45 {
        let zz = z + 1.0;
        let dir = [0.4, 0.2];
        let cc = [c[0] + dir[0] * 0.6, c[1] + dir[1] * 0.6, zz + 0.2];
        b.cyl("metal", [c[0], c[1], zz], [cc[0], cc[1], zz], 0.015);
        b.lathe(
            "appliance",
            [cc[0], cc[1], zz - 0.15],
            &[
                (0.0, 0.0),
                (0.18, 0.04),
                (0.32, 0.14),
                (0.34, 0.16),
                (0.0, 0.06),
            ],
            1.0,
            1.0,
        );
    }
}

/// A water tank on its stand (roofs of the south).
pub fn water_tank(b: &mut Builder, c: [f64; 2], z: f64) {
    for (dx, dy) in [(-0.4, -0.4), (0.4, -0.4), (0.4, 0.4), (-0.4, 0.4)] {
        b.rbox(
            "metal",
            [c[0] + dx - 0.03, c[1] + dy - 0.03, z],
            [c[0] + dx + 0.03, c[1] + dy + 0.03, z + 0.5],
            0.0,
        );
    }
    b.rbox(
        "metal",
        [c[0] - 0.5, c[1] - 0.5, z + 0.5],
        [c[0] + 0.5, c[1] + 0.5, z + 0.56],
        0.0,
    );
    b.cone(
        "appliance",
        [c[0], c[1], z + 0.56],
        [c[0], c[1], z + 1.6],
        0.5,
        0.5,
        18,
    );
    b.cone(
        "appliance",
        [c[0], c[1], z + 1.6],
        [c[0], c[1], z + 1.72],
        0.5,
        0.3,
        18,
    );
}

/// A clothes line on a flat roof: two posts, lines and washing.
pub fn roof_clothes(b: &mut Builder, a: [f64; 2], e: [f64; 2], z: f64, seed: u64) {
    let len = ((e[0] - a[0]).powi(2) + (e[1] - a[1]).powi(2)).sqrt();
    if len < 1.0 {
        return;
    }
    let f = Face {
        o: a,
        u: [(e[0] - a[0]) / len, (e[1] - a[1]) / len],
        len,
    };
    for s in [0.0, len] {
        f.bar(b, "metal", (s, z), (s, z + 1.8), 0.0, 0.025, 8);
        f.bx(
            b,
            "metal",
            (s - 0.02, s + 0.02),
            (z + 1.75, z + 1.79),
            (-0.4, 0.4),
        );
    }
    let lines = [-0.3, -0.1, 0.1, 0.3];
    for dd in lines {
        f.bar(b, "line", (0.0, z + 1.77), (len, z + 1.77), dd, 0.004, 4);
    }
    let roles = ["cloth_a", "cloth_b", "cloth_c"];
    let mut x = 0.2;
    let mut k = 0;
    while x < len - 0.4 {
        let wdt = 0.3 + 0.4 * unit(seed, 20 + k);
        let e = (x + wdt).min(len - 0.1);
        let dd = lines[(unit(seed, 40 + k) * 4.0) as usize % 4];
        let hang = 0.4 + 0.4 * unit(seed, 60 + k);
        let role = roles[(unit(seed, 80 + k) * 3.0) as usize % 3];
        f.bx(
            b,
            role,
            (x, e),
            (z + 1.77 - hang, z + 1.77),
            (dd - 0.006, dd + 0.006),
        );
        x = e + 0.1 + 0.3 * unit(seed, 100 + k);
        k += 1;
    }
}

/// A chimney of whitewashed brick with its little roof of tiles.
pub fn chimney(b: &mut Builder, c: [f64; 2], z0: f64, z1: f64) {
    b.rbox(
        "wall",
        [c[0] - 0.3, c[1] - 0.22, z0],
        [c[0] + 0.3, c[1] + 0.22, z1],
        0.0,
    );
    b.rbox(
        "trim",
        [c[0] - 0.34, c[1] - 0.26, z1],
        [c[0] + 0.34, c[1] + 0.26, z1 + 0.06],
        0.0,
    );
    for (dx, dy) in [(-0.24, -0.16), (0.24, -0.16), (0.24, 0.16), (-0.24, 0.16)] {
        b.rbox(
            "wall",
            [c[0] + dx - 0.05, c[1] + dy - 0.05, z1 + 0.06],
            [c[0] + dx + 0.05, c[1] + dy + 0.05, z1 + 0.3],
            0.0,
        );
    }
    b.cone(
        "tiles",
        [c[0] - 0.36, c[1], z1 + 0.34],
        [c[0] + 0.36, c[1], z1 + 0.34],
        0.08,
        0.08,
        6,
    );
    b.rbox(
        "trim",
        [c[0] - 0.36, c[1] - 0.3, z1 + 0.29],
        [c[0] + 0.36, c[1] + 0.3, z1 + 0.34],
        0.0,
    );
}

/// A street lantern on a bracket on the face (farol de forja).
pub fn wall_lantern(b: &mut Builder, f: &Face, s: f64, z: f64) {
    f.bx(
        b,
        "iron",
        (s - 0.06, s + 0.06),
        (z - 0.1, z + 0.1),
        (-0.01, 0.02),
    );
    let a = f.at(s, z, 0.01);
    let e = f.at(s, z + 0.12, 0.45);
    b.cone("iron", a, e, 0.014, 0.014, 6);
    let c = f.at(s, z - 0.05, 0.45);
    b.cone(
        "iron",
        [c[0], c[1], z + 0.12],
        [c[0], c[1], z + 0.02],
        0.02,
        0.02,
        6,
    );
    b.cone(
        "iron",
        [c[0], c[1], z + 0.02],
        [c[0], c[1], z - 0.05],
        0.03,
        0.12,
        6,
    );
    b.cone(
        "lamp",
        [c[0], c[1], z - 0.05],
        [c[0], c[1], z - 0.32],
        0.1,
        0.075,
        6,
    );
    b.cone(
        "iron",
        [c[0], c[1], z - 0.32],
        [c[0], c[1], z - 0.36],
        0.08,
        0.04,
        6,
    );
}

/// A door of panelled wood in a hole: frame, one or two leaves with their
/// raised panels and knobs, and a fanlight with its grille.
#[allow(clippy::too_many_arguments)]
pub fn panel_door(
    b: &mut Builder,
    f: &Face,
    h: Hole,
    leaves: usize,
    fanlight: f64,
    wood: &'static str,
    iron: &'static str,
    reveal: f64,
) {
    let (s0, s1) = h.s;
    let (z0, z1) = h.z;
    let fw = 0.08;
    let fd = (-reveal - 0.1, -reveal);
    f.bx(b, wood, (s0, s0 + fw), (z0, z1), fd);
    f.bx(b, wood, (s1 - fw, s1), (z0, z1), fd);
    f.bx(b, wood, (s0, s1), (z1 - fw, z1), fd);
    let top = if fanlight > 0.05 {
        let t = z1 - fw - fanlight;
        f.bx(b, wood, (s0 + fw, s1 - fw), (t - 0.07, t), fd);
        f.bx(
            b,
            "glass",
            (s0 + fw, s1 - fw),
            (t, z1 - fw),
            (-reveal - 0.06, -reveal - 0.05),
        );
        grille_plane(
            b,
            f,
            iron,
            (s0 + fw, s1 - fw),
            (t, z1 - fw),
            -reveal - 0.03,
            0.09,
            2,
        );
        t - 0.07
    } else {
        z1 - fw
    };
    let n = leaves.clamp(1, 2);
    let lw = (s1 - s0 - 2.0 * fw) / n as f64;
    for k in 0..n {
        let a = s0 + fw + lw * k as f64;
        let e = a + lw;
        let ld = (-reveal - 0.09, -reveal - 0.04);
        f.bx(b, wood, (a, e), (z0, top), ld);
        // Raised panels: one or two columns, three rows.
        let cols = if lw > 0.6 { 2 } else { 1 };
        let gap = 0.11;
        let pw = (lw - gap * (cols + 1) as f64) / cols as f64;
        let room = top - z0 - gap * 4.0;
        let mut zz = z0 + gap;
        for frac in [0.3, 0.42, 0.28] {
            let ph = room * frac;
            for c in 0..cols {
                let x = a + gap + (pw + gap) * c as f64;
                f.bx(b, wood, (x, x + pw), (zz, zz + ph), (ld.1, ld.1 + 0.02));
                f.bx(
                    b,
                    wood,
                    (x + 0.05, x + pw - 0.05),
                    (zz + 0.05, zz + ph - 0.05),
                    (ld.1 + 0.02, ld.1 + 0.03),
                );
            }
            zz += ph + gap;
        }
        // The knob or the knocker.
        let kx = if n == 2 {
            if k == 0 {
                e - 0.1
            } else {
                a + 0.1
            }
        } else {
            e - 0.12
        };
        b.ellipsoid(
            "brass",
            f.at(kx, z0 + 1.05, ld.1 + 0.035),
            [0.03, 0.03, 0.03],
        );
    }
}

/// A shop front (bajo comercial): the glazed front in a frame, the door,
/// the sign band and the metal roller shutter down `down` of the hole.
#[allow(clippy::too_many_arguments)]
pub fn shop_front(
    b: &mut Builder,
    f: &Face,
    h: Hole,
    down: f64,
    sign: bool,
    frame: &'static str,
    reveal: f64,
) {
    let (s0, s1) = h.s;
    let (z0, z1) = h.z;
    let top = if sign { z1 - 0.55 } else { z1 };
    if sign {
        f.bx(b, "sign", (s0, s1), (top, z1), (-reveal, -reveal + 0.04));
    }
    let fd = (-reveal - 0.06, -reveal);
    f.bx(b, frame, (s0, s1), (z0, z0 + 0.1), fd);
    f.bx(b, frame, (s0, s1), (top - 0.06, top), fd);
    let n = ((s1 - s0) / 1.4).round().max(1.0) as usize;
    for i in 0..=n {
        let x = s0 + (s1 - s0) * i as f64 / n as f64;
        f.bx(b, frame, (x - 0.03, x + 0.03), (z0, top), fd);
    }
    f.bx(
        b,
        "glass",
        (s0, s1),
        (z0 + 0.1, top - 0.06),
        (-reveal - 0.04, -reveal - 0.03),
    );
    // The roller shutter: its box under the sign and the slats down.
    let down = down.clamp(0.0, 1.0);
    f.bx(
        b,
        "shutter",
        (s0 - 0.03, s1 + 0.03),
        (top - 0.06, top + 0.02),
        (-reveal + 0.01, -reveal + 0.14),
    );
    if down > 0.02 {
        let low = top - (top - z0) * down;
        f.bx(
            b,
            "shutter",
            (s0, s1),
            (low, top - 0.06),
            (-reveal + 0.02, -reveal + 0.04),
        );
        let mut zz = top - 0.14;
        while zz > low + 0.04 {
            f.bx(
                b,
                "shutter",
                (s0, s1),
                (zz - 0.012, zz),
                (-reveal + 0.04, -reveal + 0.05),
            );
            zz -= 0.1;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{make, Params};

    /// Every building is made, with a sensible number of triangles for the
    /// size of a building (a whole street of them has to be drawn).
    #[test]
    fn buildings_are_made() {
        for kind in [
            "fine_village_house",
            "fine_manor_house",
            "fine_block_1960s",
            "fine_ensanche",
            "fine_coastal_tower",
            "fine_terraced_row",
            "fine_corral",
            "fine_cortijo",
            "fine_colonization_row",
            "fine_chalet_80s",
            "fine_semi_detached",
            "fine_warehouse",
            "fine_market",
            "fine_church",
            "fine_bullring",
            "fine_school",
            "fine_health_centre",
            "fine_petrol_station",
            "fine_castilian_arcade",
            "fine_caserio",
            "fine_galician_galleries",
            "fine_masia",
            "fine_canarian_house",
            "fine_ibizan_house",
            "fine_office_tower",
            "fine_office_1970s",
            "fine_business_park",
            "fine_corner_cafe",
            "fine_shop_and_flats",
            "fine_urban_hotel",
            "fine_supermarket",
        ] {
            let m = make(kind, &Params::default()).unwrap();
            let tris: usize = m.parts.iter().map(|(_, t)| t.len()).sum();
            let roles: Vec<_> = m.parts.iter().map(|(r, _)| *r).collect();
            println!("{kind}: {tris} triangles, {roles:?}");
            assert!(tris > 2_000 && tris < 300_000, "{kind}: {tris}");
        }
    }
}
