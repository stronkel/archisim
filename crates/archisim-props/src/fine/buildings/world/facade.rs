//! A front of a building of the world, from its skin: the axes of its
//! windows, the ground floor, the windows of each floor with what closes
//! and frames them, the balconies, the cornice and what each place hangs
//! on its walls.

use super::super::ensanche::{arch_fill, voussoirs_of};
use super::super::more::wooden_balcony;
use super::super::offices::car;
use super::super::*;
use super::skins::*;
use super::Look;
use crate::build::{Builder, P3};

pub const REVEAL: f64 = 0.12;

/// One front: its face, its floors and how it is.
#[derive(Clone, Copy, Debug)]
pub struct Front {
    pub f: Face,
    pub floors: usize,
    pub seed: u64,
    pub ground: Ground,
    pub balc: Balc,
    pub extras: u32,
    pub thick: f64,
    /// The ground floor over the street.
    pub lift: f64,
    /// A back or a side: windows (and a door), nothing more.
    pub plain: bool,
    /// A door on the ground floor of a plain front.
    pub door: bool,
    /// Windows only on every other axis (the sides of a block).
    pub sparse: bool,
    /// Between party walls (wing walls of a verandah, no quoins).
    pub party: bool,
    /// How deep the walk under an arcade or the pilotis is.
    pub recess: f64,
    /// Steps up to a raised door (a deck brings its own).
    pub steps: bool,
    /// Nothing under the ground floor (on stilts).
    pub open_below: bool,
    pub sash: Sash,
    pub shut: Shut,
    pub head: Head,
    pub top: Top,
    pub storey: (f64, f64),
}

impl Front {
    /// The floor of storey `k` (0 the ground floor).
    pub fn z(&self, k: usize) -> f64 {
        if k == 0 {
            self.lift
        } else {
            self.lift + self.storey.0 + self.storey.1 * (k - 1) as f64
        }
    }

    /// The top of its walls.
    pub fn top(&self) -> f64 {
        self.z(self.floors)
    }

    /// Standing on stilts.
    pub fn stilts(&self) -> bool {
        self.open_below
    }

    fn has(&self, flag: u32) -> bool {
        self.extras & flag != 0
    }

    /// A front as the skin has it.
    pub fn of(f: Face, sk: &Skin, floors: usize, seed: u64) -> Front {
        Front {
            f,
            floors,
            seed,
            ground: sk.ground,
            balc: sk.balc,
            extras: sk.extras,
            thick: 0.3,
            lift: if sk.ground == Ground::Stoop {
                sk.lift
            } else {
                0.0
            },
            plain: false,
            door: true,
            sparse: false,
            party: true,
            recess: match sk.ground {
                Ground::Arcade => 2.6,
                Ground::Pilotis => 5.5,
                _ => 0.0,
            },
            steps: true,
            open_below: false,
            sash: sk.sash,
            shut: sk.shut,
            head: sk.head,
            top: sk.top,
            storey: sk.storey,
        }
    }

    /// The back (or a side) of the same building.
    pub fn plain_of(&self, f: Face, door: bool, sparse: bool) -> Front {
        Front {
            f,
            plain: true,
            door,
            sparse,
            ground: Ground::Door,
            balc: Balc::None,
            extras: self.extras & (AC | LAUNDRY | FLOWERS | BELT | CORNER_BOARDS | ALGARVE),
            recess: 0.0,
            head: match self.head {
                Head::Pediment | Head::Hood | Head::Keystone => Head::Band,
                h => h,
            },
            top: match self.top {
                Top::None => Top::None,
                Top::Tiles => Top::Tiles,
                _ => Top::Band,
            },
            ..*self
        }
    }
}

/// What a front tells the rest of the building.
pub struct Made {
    pub top: f64,
    /// The axes of its windows along the face.
    pub axes: Vec<f64>,
}

/// The kind of a hole.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Window,
    /// A balcony door (French window).
    Door,
    /// The entrance.
    Entrance,
    Shop,
    Garage,
    /// A small window of a basement.
    Low,
}

#[derive(Clone, Copy)]
struct Opening {
    h: Hole,
    kind: Kind,
    storey: usize,
    axis: usize,
}

fn ok(f: &Front, k: u64, p: f64) -> bool {
    unit(f.seed, k) < p
}

/// Builds a front. The core inside is the caller's.
pub fn front(b: &mut Builder, fr: &Front, sk: &Skin, l: &Look) -> Made {
    let f = fr.f;
    let len = f.len;
    let (gh, fh) = fr.storey;
    let top = fr.top();
    let margin = (len * 0.08).clamp(0.35, 0.6);
    let n = ((len - 2.0 * margin) / sk.bay).round().max(1.0) as usize;
    let step = (len - 2.0 * margin) / n as f64;
    let axes: Vec<f64> = (0..n).map(|i| margin + step * (i as f64 + 0.5)).collect();
    let ww = sk.win.0.min(step * 0.66);
    let door_axis = if n == 1 {
        0
    } else if fr.ground == Ground::Shop || fr.ground == Ground::Garage {
        if fr.ground == Ground::Garage {
            n - 1
        } else {
            0
        }
    } else if n % 2 == 1 {
        n / 2
    } else if unit(fr.seed, 11) > 0.5 {
        0
    } else {
        n - 1
    };
    let recessed = !fr.plain && matches!(fr.ground, Ground::Arcade | Ground::Pilotis);
    let zg = fr.z(1);

    // The bay window's axis and its storeys.
    let bay_axis = if fr.balc == Balc::Bay && !fr.plain {
        Some(if n == 1 {
            0
        } else if door_axis == 0 {
            n - 1
        } else {
            0
        })
    } else {
        None
    };
    let bay_storeys: Vec<usize> = match bay_axis {
        Some(_) if fr.ground == Ground::Garage => (1..fr.floors).collect(),
        Some(_) => (0..fr.floors.min(if ok(fr, 12, 0.5) { 2 } else { 1 })).collect(),
        None => Vec::new(),
    };
    let in_bay = |k: usize, i: usize| bay_axis == Some(i) && bay_storeys.contains(&k);
    let bay_span = bay_axis.map(|i| {
        let half = (step * 0.47).max(1.0);
        ((axes[i] - half).max(0.15), (axes[i] + half).min(len - 0.15))
    });

    // The holes.
    let mut ops: Vec<Opening> = Vec::new();
    let sill = 0.9;
    // The ground floor.
    let gz = (fr.lift, fr.lift + gh);
    let door_w = if step > 2.0 { 1.2 } else { 1.0 };
    let door_hole = |c: f64| Hole {
        s: (c - door_w / 2.0, c + door_w / 2.0),
        z: (gz.0, gz.0 + (gh - 0.45).min(2.9)),
    };
    let window_hole = |c: f64, z0: f64, z1: f64| Hole {
        s: (c - ww / 2.0, c + ww / 2.0),
        z: (z0, z1),
    };
    if !recessed {
        let ground = if fr.plain { Ground::Door } else { fr.ground };
        match ground {
            Ground::Shop => {
                if n >= 2 {
                    ops.push(Opening {
                        h: door_hole(axes[0]),
                        kind: Kind::Entrance,
                        storey: 0,
                        axis: 0,
                    });
                    ops.push(Opening {
                        h: Hole {
                            s: (axes[0] + step * 0.5 + 0.15, len - margin * 0.6),
                            z: (gz.0, gz.1 - 0.5),
                        },
                        kind: Kind::Shop,
                        storey: 0,
                        axis: 1,
                    });
                } else {
                    ops.push(Opening {
                        h: Hole {
                            s: (margin * 0.6, len - margin * 0.6),
                            z: (gz.0, gz.1 - 0.5),
                        },
                        kind: Kind::Shop,
                        storey: 0,
                        axis: 0,
                    });
                }
            }
            Ground::Garage => {
                let gw = (step * 0.9).min(2.7);
                ops.push(Opening {
                    h: Hole {
                        s: (axes[0] - gw / 2.0, axes[0] + gw / 2.0),
                        z: (0.0, 2.3),
                    },
                    kind: Kind::Garage,
                    storey: 0,
                    axis: 0,
                });
                if n >= 2 {
                    ops.push(Opening {
                        h: door_hole(axes[n - 1]),
                        kind: Kind::Entrance,
                        storey: 0,
                        axis: n - 1,
                    });
                }
                for (i, &c) in axes.iter().enumerate().take(n - 1).skip(1) {
                    ops.push(Opening {
                        h: window_hole(c, gz.0 + sill, gz.0 + sill + 1.3),
                        kind: Kind::Window,
                        storey: 0,
                        axis: i,
                    });
                }
            }
            Ground::Lattice => {
                ops.push(Opening {
                    h: Hole {
                        s: (0.2, len - 0.2),
                        z: (gz.0, gz.0 + 2.3),
                    },
                    kind: Kind::Shop,
                    storey: 0,
                    axis: 0,
                });
            }
            _ => {
                for (i, &c) in axes.iter().enumerate() {
                    if fr.sparse && i % 2 == 1 {
                        continue;
                    }
                    if in_bay(0, i) {
                        continue;
                    }
                    if i == door_axis && (!fr.plain || fr.door) {
                        ops.push(Opening {
                            h: door_hole(c),
                            kind: Kind::Entrance,
                            storey: 0,
                            axis: i,
                        });
                    } else {
                        let z1 = (gz.0 + sill + sk.win.1).min(gz.1 - 0.45);
                        ops.push(Opening {
                            h: window_hole(c, gz.0 + sill, z1),
                            kind: Kind::Window,
                            storey: 0,
                            axis: i,
                        });
                    }
                    if fr.lift > 1.0 && i != door_axis {
                        ops.push(Opening {
                            h: window_hole(c, 0.3, fr.lift - 0.25),
                            kind: Kind::Low,
                            storey: 0,
                            axis: i,
                        });
                    }
                }
            }
        }
    }
    // The floors above.
    for k in 1..fr.floors {
        let zf = fr.z(k);
        let head_max = zf + fh - 0.35;
        for (i, &c) in axes.iter().enumerate() {
            if fr.sparse && i % 2 == 1 {
                continue;
            }
            if in_bay(k, i) {
                continue;
            }
            let door = !fr.plain
                && match fr.balc {
                    Balc::Iron => {
                        k == 1
                            || (n > 1 && (i + k) % 2 == 0)
                            || ok(fr, 30 + (k * 7 + i) as u64, 0.3)
                    }
                    Balc::Continuous | Balc::Gallery => true,
                    Balc::Covered | Balc::Lace => k == 1,
                    Balc::Glazed | Balc::Slab => i % 2 == 0,
                    _ => false,
                };
            let mut z0 = if door { zf + 0.02 } else { zf + sill };
            let mut z1 = if door {
                (zf + sill + sk.win.1).max(zf + 2.2).min(head_max)
            } else {
                (zf + sill + sk.win.1).min(head_max)
            };
            if fr.head == Head::Arch {
                z1 = (z1 + ww / 2.0).min(zf + fh - 0.25);
            }
            if z1 - z0 < 0.6 {
                z0 = z1 - 0.6;
            }
            ops.push(Opening {
                h: window_hole(c, z0, z1),
                kind: if door { Kind::Door } else { Kind::Window },
                storey: k,
                axis: i,
            });
        }
    }

    // The walls.
    let mut holes: Vec<Hole> = ops.iter().map(|o| o.h).collect();
    if let Some(s) = bay_span {
        for &k in &bay_storeys {
            let zf = fr.z(k);
            let z = if k == 0 {
                (zf, zf + gh - 0.2)
            } else {
                (zf + 0.02, zf + fh - 0.25)
            };
            holes.push(Hole { s, z });
        }
    }
    let base_split = sk.base.is_empty() || l.base == l.wall;
    let base_top = if base_split {
        fr.lift
    } else if matches!(fr.ground, Ground::Shop | Ground::Arcade | Ground::Pilotis) || fr.has(RUSTIC)
    {
        if fr.plain {
            fr.lift + 0.9
        } else {
            zg
        }
    } else {
        fr.lift + 0.9
    };
    let wall_from = if recessed {
        zg - 0.4
    } else if fr.open_below {
        fr.lift
    } else {
        0.0
    };
    if base_top > wall_from + 0.01 {
        wall(b, l.base, &f, (wall_from, base_top), fr.thick, &holes);
    }
    wall(
        b,
        l.wall,
        &f,
        (base_top.max(wall_from), top),
        fr.thick,
        &holes,
    );
    if recessed {
        arcade(b, fr, sk, l, &axes, step, margin);
    }

    // The ground floor's holes.
    for o in &ops {
        let h = o.h;
        match o.kind {
            Kind::Entrance => {
                let leaves = if h.s.1 - h.s.0 > 1.1 { 2 } else { 1 };
                let fan = if h.z.1 - h.z.0 > 2.6 { 0.45 } else { 0.0 };
                panel_door(b, &f, h, leaves, fan, l.door, "iron", REVEAL);
                if fr.lift > 0.05 {
                    if fr.steps {
                        stoop(b, &f, (h.s.0 - 0.15, h.s.1 + 0.15), fr.lift, l);
                    }
                } else {
                    f.bx(
                        b,
                        l.base,
                        (h.s.0 - 0.1, h.s.1 + 0.1),
                        (0.0, 0.12),
                        (-REVEAL, 0.28),
                    );
                }
                frame_head(b, fr, l, h, o.storey, true);
                if fr.has(PLANTS) && ok(fr, 50 + o.axis as u64, 0.6) {
                    standing_pot(b, f.at(h.s.1 + 0.35, 0.0, 0.3), 0.9, fr.seed + 5);
                }
                if fr.has(ALGARVE) {
                    surround(b, &f, h, l.accent, 0.2, 0.012);
                }
            }
            Kind::Shop => {
                if fr.ground == Ground::Lattice {
                    lattice(b, &f, h, axes[door_axis], l);
                } else {
                    let down = if ok(fr, 60, 0.3) { 0.6 } else { 0.0 };
                    shop_front(b, &f, h, down, true, l.frame, REVEAL);
                    if fr.has(AWNINGS) && ok(fr, 61, 0.7) {
                        awning(b, &f, (h.s.0 + 0.1, h.s.1 - 0.1), h.z.1 + 0.25, 1.3, 0.5);
                    }
                }
            }
            Kind::Garage => {
                garage_door(b, &f, h, l);
            }
            Kind::Low => {
                let win = Win {
                    leaves: 1,
                    reveal: REVEAL,
                    ..Default::default()
                };
                window(b, &f, h, &win, l.frame);
                if fr.has(GRILLES) || sk.key == "nueva_york" {
                    grille_plane(
                        b,
                        &f,
                        "iron",
                        (h.s.0 - 0.05, h.s.1 + 0.05),
                        (h.z.0 - 0.05, h.z.1 + 0.05),
                        0.01,
                        0.1,
                        2,
                    );
                }
            }
            Kind::Window | Kind::Door => {
                glaze(b, fr, sk, l, o, step, ww);
            }
        }
    }

    // Balconies, galleries and the rest.
    balconies(b, fr, l, &ops, &axes, step, ww);
    if let (Some(s), false) = (bay_span, bay_storeys.is_empty()) {
        bay_window(b, fr, sk, l, s, &bay_storeys);
    }
    belts_and_corners(b, fr, l, &ops);
    cornice(b, fr, l, top);
    hangings(b, fr, l, &ops, step, ww);
    Made { top, axes }
}

/// A window or a balcony door: carpentry, panes, what closes it, its
/// head and its sill.
fn glaze(b: &mut Builder, fr: &Front, sk: &Skin, l: &Look, o: &Opening, step: f64, ww: f64) {
    let f = &fr.f;
    let h = o.h;
    let key = 100 + (o.storey * 17 + o.axis) as u64;
    let roller = fr.shut == Shut::Roller;
    let blind = if roller {
        [0.0, 0.25, 0.5, 0.8][(unit(fr.seed, key) * 4.0) as usize % 4]
    } else {
        0.0
    };
    let curtain = unit(fr.seed, key + 1) > 0.45;
    let leaves = if o.kind == Kind::Door { 2 } else { sk.leaves };
    match fr.sash {
        Sash::Casement => {
            let win = Win {
                leaves,
                reveal: REVEAL,
                blind,
                blind_box: roller,
                curtain,
                ..Default::default()
            };
            window(b, f, h, &win, l.frame);
            muntins(b, f, h, leaves * sk.panes.0, sk.panes.1, false, l.frame);
        }
        Sash::Sash => {
            let win = Win {
                leaves: 1,
                reveal: REVEAL,
                bars: 1,
                blind,
                blind_box: roller,
                curtain,
                ..Default::default()
            };
            window(b, f, h, &win, l.frame);
            muntins(b, f, h, sk.panes.0, sk.panes.1, true, l.frame);
        }
        Sash::Sliding => {
            let win = Win {
                leaves: 2,
                reveal: REVEAL,
                sliding: true,
                blind,
                blind_box: roller,
                curtain,
                ..Default::default()
            };
            window(b, f, h, &win, l.frame);
            if sk.panes.0 > 1 || sk.panes.1 > 1 {
                muntins(b, f, h, 2 * sk.panes.0, sk.panes.1, false, l.frame);
            }
        }
    }
    // What closes it.
    match fr.shut {
        Shut::Louvred | Shut::Board => {
            let lw = (h.s.1 - h.s.0) / 2.0 + 0.03;
            let room = (step - ww) / 2.0;
            let open = room > lw + 0.08 && unit(fr.seed, key + 2) > 0.2;
            shutters(
                b,
                f,
                h,
                l.shutter,
                fr.shut == Shut::Louvred,
                open,
                unit(fr.seed, key + 3),
            );
        }
        _ => {}
    }
    if o.storey > 0 || o.kind == Kind::Window {
        frame_head(b, fr, l, h, o.storey, false);
    }
    if o.kind == Kind::Window && fr.head != Head::Plain {
        sill(b, f, h, l.trim, 0.06, REVEAL);
    }
    if fr.has(ALGARVE) {
        surround(b, f, h, l.accent, 0.2, 0.012);
    }
    // Grilles on the ground floor.
    if o.storey == 0 && o.kind == Kind::Window && fr.has(GRILLES) {
        let role = if sk.balc == Balc::Covered {
            l.door
        } else {
            "iron"
        };
        projecting_grille(b, f, h, role, 0.16);
    }
}

/// Glazing bars over the glass: `cols` across the whole window, `rows`
/// up (of each half of a sash window).
fn muntins(
    b: &mut Builder,
    f: &Face,
    h: Hole,
    cols: usize,
    rows: usize,
    sash: bool,
    role: &'static str,
) {
    let (s0, s1) = (h.s.0 + 0.1, h.s.1 - 0.1);
    let (z0, z1) = (h.z.0 + 0.12, h.z.1 - 0.1);
    let d = (-REVEAL - 0.032, -REVEAL - 0.014);
    let t = 0.011;
    for c in 1..cols.max(1) {
        let x = s0 + (s1 - s0) * c as f64 / cols as f64;
        f.bx(b, role, (x - t, x + t), (z0, z1), d);
    }
    let halves: Vec<(f64, f64)> = if sash {
        let m = (z0 + z1) / 2.0;
        vec![(z0, m - 0.03), (m + 0.03, z1)]
    } else {
        vec![(z0, z1)]
    };
    for (a, e) in halves {
        for r in 1..rows.max(1) {
            let z = a + (e - a) * r as f64 / rows as f64;
            f.bx(b, role, (s0, s1), (z - t, z + t), d);
        }
    }
}

/// A pair of shutters: open against the wall, or shut over the glass
/// (one of them only, now and then).
fn shutters(
    b: &mut Builder,
    f: &Face,
    h: Hole,
    role: &'static str,
    louvred: bool,
    open: bool,
    pick: f64,
) {
    let lw = (h.s.1 - h.s.0) / 2.0 + 0.02;
    let (z0, z1) = (h.z.0 - 0.02, h.z.1 + 0.02);
    let leaves: Vec<((f64, f64), (f64, f64))> = if open {
        vec![
            ((h.s.0 - lw - 0.04, h.s.0 - 0.04), (0.0, 0.035)),
            ((h.s.1 + 0.04, h.s.1 + lw + 0.04), (0.0, 0.035)),
        ]
    } else {
        let c = (h.s.0 + h.s.1) / 2.0;
        let shut = (-0.05, -0.015);
        if pick < 0.5 {
            vec![((h.s.0, c), shut), ((c, h.s.1), shut)]
        } else {
            vec![((h.s.0, c), shut)]
        }
    };
    for (s, d) in leaves {
        let (a, e) = s;
        let (z0, z1) = if open { (z0, z1) } else { (h.z.0, h.z.1) };
        // The frame of the leaf.
        f.bx(b, role, (a, a + 0.05), (z0, z1), d);
        f.bx(b, role, (e - 0.05, e), (z0, z1), d);
        f.bx(b, role, (a + 0.05, e - 0.05), (z1 - 0.06, z1), d);
        f.bx(b, role, (a + 0.05, e - 0.05), (z0, z0 + 0.08), d);
        if louvred {
            f.bx(
                b,
                role,
                (a + 0.05, e - 0.05),
                (z0 + 0.08, z1 - 0.06),
                (d.0 + 0.01, d.0 + 0.02),
            );
            let mut z = z0 + 0.12;
            while z < z1 - 0.1 {
                f.bx(
                    b,
                    role,
                    (a + 0.05, e - 0.05),
                    (z, z + 0.03),
                    (d.1 - 0.005, d.1 + 0.008),
                );
                z += 0.1;
            }
        } else {
            f.bx(
                b,
                role,
                (a + 0.05, e - 0.05),
                (z0 + 0.08, z1 - 0.06),
                (d.0 + 0.005, d.1 - 0.005),
            );
            for zz in [z0 + 0.3, z1 - 0.3] {
                f.bx(
                    b,
                    role,
                    (a + 0.05, e - 0.05),
                    (zz, zz + 0.1),
                    (d.1 - 0.005, d.1 + 0.012),
                );
            }
        }
        if open {
            let x = if a < h.s.0 { e + 0.02 } else { a - 0.02 };
            for zz in [z0 + 0.2, z1 - 0.25] {
                f.bx(
                    b,
                    "iron",
                    (x - 0.025, x + 0.025),
                    (zz, zz + 0.05),
                    (-0.01, 0.04),
                );
            }
        }
    }
}

/// What frames the head of a hole.
fn frame_head(b: &mut Builder, fr: &Front, l: &Look, h: Hole, storey: usize, entrance: bool) {
    let f = &fr.f;
    let (s0, s1) = h.s;
    let z1 = h.z.1;
    let c = (s0 + s1) / 2.0;
    match fr.head {
        Head::Plain => {}
        Head::Band => surround(b, f, h, l.trim, 0.13, 0.015),
        Head::Hood | Head::Pediment => {
            surround(b, f, h, l.trim, 0.13, 0.015);
            f.bx(
                b,
                l.trim,
                (s0 - 0.2, s1 + 0.2),
                (z1 + 0.13, z1 + 0.25),
                (-0.01, 0.14),
            );
            if fr.head == Head::Pediment && (storey == 1 || entrance) {
                f.prism(
                    b,
                    l.trim,
                    &[
                        (s0 - 0.2, z1 + 0.25),
                        (s1 + 0.2, z1 + 0.25),
                        (c, z1 + 0.25 + 0.3),
                    ],
                    (-0.01, 0.12),
                );
            }
        }
        Head::Arch => {
            let r = (s1 - s0) / 2.0;
            arch_fill(b, f, l.wall, (s0, s1), z1 - r, (-fr.thick, 0.0));
            voussoirs_of(b, f, (s0, s1), z1 - r, l.trim, 0.14);
        }
        Head::Keystone => {
            surround(b, f, h, l.trim, 0.13, 0.015);
            f.bx(
                b,
                l.trim,
                (c - 0.1, c + 0.1),
                (z1 - 0.04, z1 + 0.26),
                (-0.01, 0.05),
            );
        }
    }
}

/// The steps up to a raised ground floor, with their cheeks and rails.
fn stoop(b: &mut Builder, f: &Face, s: (f64, f64), lift: f64, l: &Look) {
    let n = (lift / 0.18).ceil().max(1.0) as usize;
    let rise = lift / n as f64;
    let run = 0.3;
    let out = run * n as f64;
    for j in 0..n {
        f.bx(
            b,
            l.base,
            s,
            (0.0, rise * (j + 1) as f64),
            (-0.02, out - run * j as f64),
        );
    }
    if lift > 0.5 {
        for (edge, sgn) in [(s.0, -1.0), (s.1, 1.0)] {
            let a = edge;
            let e = edge + sgn * 0.25;
            let (a, e) = (a.min(e), a.max(e));
            let side = Face {
                o: [f.at(e, 0.0, 0.0)[0], f.at(e, 0.0, 0.0)[1]],
                u: f.out(),
                len: out,
            };
            side.prism(
                b,
                l.base,
                &[(-0.02, 0.0), (out, 0.0), (out, 0.5), (-0.02, lift + 0.5)],
                (0.0, e - a),
            );
            // The rail over the cheek.
            let x = (a + e) / 2.0;
            let p0 = f.at(x, lift + 1.4, 0.05);
            let p1 = f.at(x, 1.4, out - 0.05);
            b.cone("iron", p0, p1, 0.022, 0.022, 6);
            let m = (out / 0.16).round().max(2.0) as usize;
            for k in 0..=m {
                let t = k as f64 / m as f64;
                let d = 0.05 + (out - 0.1) * t;
                let zc = lift + 0.5 - (lift) * t * (out / (out + 0.02));
                let zr = lift + 1.4 - lift * t;
                b.cone(
                    "iron",
                    f.at(x, zc - 0.02, d),
                    f.at(x, zr, d),
                    0.009,
                    0.009,
                    4,
                );
            }
        }
    }
}

/// A sectional garage door.
fn garage_door(b: &mut Builder, f: &Face, h: Hole, l: &Look) {
    f.bx(b, l.door, h.s, h.z, (-REVEAL - 0.05, -REVEAL));
    let mut z = h.z.0 + 0.5;
    while z < h.z.1 - 0.1 {
        f.bx(
            b,
            l.trim,
            h.s,
            (z - 0.015, z + 0.015),
            (-REVEAL, -REVEAL + 0.012),
        );
        z += 0.5;
    }
    f.bx(
        b,
        "paving",
        (h.s.0 - 0.2, h.s.1 + 0.2),
        (0.0, 0.03),
        (-REVEAL, 1.2),
    );
}

/// The wooden lattice of a machiya's ground floor, its sliding door and
/// the curtain over it.
fn lattice(b: &mut Builder, f: &Face, h: Hole, door_c: f64, l: &Look) {
    let door = (door_c - 0.6, door_c + 0.6);
    let (z0, z1) = h.z;
    // Paper behind, rails, slats.
    f.bx(b, "curtain", h.s, (z0, z1), (-0.2, -0.18));
    f.bx(b, l.door, h.s, (z1 - 0.12, z1), (-0.1, 0.02));
    f.bx(b, l.door, h.s, (z0, z0 + 0.1), (-0.1, 0.02));
    let mut x = h.s.0 + 0.04;
    while x < h.s.1 - 0.03 {
        if x < door.0 - 0.03 || x > door.1 + 0.03 {
            f.bx(
                b,
                l.door,
                (x - 0.014, x + 0.014),
                (z0 + 0.1, z1 - 0.12),
                (-0.07, -0.03),
            );
        }
        x += 0.055;
    }
    // The door: frame, glass and the curtain (noren) over it.
    f.bx(b, l.door, (door.0, door.0 + 0.06), (z0, z1), (-0.12, -0.05));
    f.bx(b, l.door, (door.1 - 0.06, door.1), (z0, z1), (-0.12, -0.05));
    f.bx(b, "glass", door, (z0 + 0.1, z1 - 0.12), (-0.11, -0.1));
    f.bx(
        b,
        "cloth_b",
        (door.0 - 0.1, door.1 + 0.1),
        (z1 - 0.75, z1 - 0.12),
        (0.02, 0.035),
    );
    f.bx(
        b,
        l.door,
        (door.0 - 0.12, door.1 + 0.12),
        (z1 - 0.14, z1 - 0.1),
        (-0.02, 0.06),
    );
}

/// The walk under an arcade or the floors on pilotis: piers or columns on
/// the line of the front, the arches, the ceiling, the paving and the
/// wall at the back with its shops (or the cars, under pilotis).
fn arcade(b: &mut Builder, fr: &Front, sk: &Skin, l: &Look, axes: &[f64], step: f64, margin: f64) {
    let f = fr.f;
    let len = f.len;
    let zg = fr.z(1);
    let rec = fr.recess;
    let pilotis = fr.ground == Ground::Pilotis;
    // The beam over the walk.
    f.bx(b, l.base, (0.0, len), (zg - 0.4, zg), (-fr.thick, 0.0));
    // Piers: at the ends and between the axes.
    let mut piers = vec![(0.0, margin)];
    for i in 1..axes.len() {
        let x = margin + step * i as f64;
        piers.push((x - 0.22, x + 0.22));
    }
    piers.push((len - margin, len));
    let col = if pilotis { l.trim } else { l.base };
    for &(a, e) in &piers {
        f.bx(b, col, (a, e), (0.0, zg - 0.4), (-0.45, 0.0));
    }
    // Arches between the piers.
    if !pilotis && matches!(sk.head, Head::Arch | Head::Band | Head::Hood) && sk.key != "china" {
        for w in piers.windows(2) {
            let (a, e) = (w[0].1, w[1].0);
            let r = (e - a) / 2.0;
            let spring = zg - 0.4 - r - 0.02;
            if spring > 2.1 {
                arch_fill(b, &f, l.base, (a, e), spring, (-0.45, 0.0));
            }
        }
    }
    // Ceiling and floor of the walk.
    f.bx(b, l.wall, (0.0, len), (zg - 0.3, zg), (-rec, -0.45));
    f.bx(b, "paving", (0.0, len), (0.0, 0.03), (-rec, 0.0));
    // The wall at the back of the walk.
    let fi = f.moved(-rec);
    if pilotis {
        let entry = (len * 0.65, (len * 0.65 + 2.4).min(len - 0.3));
        let door = Hole {
            s: (entry.0 + 0.5, entry.1 - 0.5),
            z: (0.0, 2.4),
        };
        wall(b, l.trim, &fi, (0.0, zg - 0.3), fr.thick, &[door]);
        let win = Win {
            leaves: 1,
            reveal: REVEAL,
            ..Default::default()
        };
        window(b, &fi, door, &win, l.frame);
        // The cars.
        let n = ((len * 0.6) / 2.6).floor().max(1.0) as usize;
        for k in 0..n {
            let s = 0.5 + 1.3 + 2.6 * k as f64;
            let p = f.at(s, 0.0, -rec / 2.0 - 0.1);
            car(b, [p[0], p[1]], unit(fr.seed, 70 + k as u64));
        }
        return;
    }
    let mut holes = Vec::new();
    for (i, &c) in axes.iter().enumerate() {
        let w = (step - 0.7).max(0.9);
        holes.push((
            Hole {
                s: (c - w / 2.0, c + w / 2.0),
                z: (0.0, zg - 0.9),
            },
            i,
        ));
    }
    let hs: Vec<Hole> = holes.iter().map(|h| h.0).collect();
    wall(b, l.base, &fi, (0.0, zg - 0.3), fr.thick, &hs);
    for (h, i) in holes {
        if i == axes.len() / 2 && axes.len() > 2 {
            panel_door(b, &fi, h, 2, 0.4, l.door, "iron", REVEAL);
        } else {
            let down = if unit(fr.seed, 80 + i as u64) < 0.25 {
                0.7
            } else {
                0.0
            };
            shop_front(b, &fi, h, down, true, l.frame, REVEAL);
        }
    }
    if fr.has(SIGNS) {
        // Signs hung from the beam, across the walk.
        for (i, &c) in axes.iter().enumerate() {
            if unit(fr.seed, 90 + i as u64) < 0.6 {
                let role = if i % 2 == 0 { "sign" } else { "sign_b" };
                f.bx(
                    b,
                    role,
                    (c - 0.7, c + 0.7),
                    (zg - 1.0, zg - 0.4),
                    (-0.25, -0.2),
                );
            }
        }
    }
}

/// The balconies of the front and the other things built out of it.
#[allow(clippy::too_many_arguments)]
fn balconies(
    b: &mut Builder,
    fr: &Front,
    l: &Look,
    ops: &[Opening],
    axes: &[f64],
    step: f64,
    ww: f64,
) {
    if fr.plain {
        return;
    }
    let f = fr.f;
    let len = f.len;
    let (_, fh) = fr.storey;
    match fr.balc {
        Balc::Iron => {
            for o in ops.iter().filter(|o| o.kind == Kind::Door) {
                let s = (o.h.s.0 - 0.28, o.h.s.1 + 0.28);
                balcony(b, &f, s, o.h.z.0, 0.5, l.trim, "iron", true);
                if fr.has(PLANTS) && unit(fr.seed, 200 + o.axis as u64 + o.storey as u64 * 7) > 0.4
                {
                    standing_pot(
                        b,
                        f.at(s.0 + 0.2, o.h.z.0, 0.24),
                        0.7,
                        fr.seed + o.axis as u64,
                    );
                }
            }
        }
        Balc::Continuous => {
            let main: Vec<usize> = if fr.floors >= 4 {
                vec![1, fr.floors - 1]
            } else {
                vec![1]
            };
            for k in 1..fr.floors {
                let zf = fr.z(k) + 0.02;
                if main.contains(&k) {
                    balcony(b, &f, (0.12, len - 0.12), zf, 0.6, l.trim, "iron", false);
                    let m = (len / 1.4).round().max(2.0) as usize;
                    for i in 0..=m {
                        let x = (0.3 + (len - 0.6) * i as f64 / m as f64).clamp(0.3, len - 0.3);
                        corbel(b, &f, x, zf - 0.16, 0.5, l.trim);
                    }
                } else {
                    for o in ops.iter().filter(|o| o.storey == k && o.kind == Kind::Door) {
                        railing(b, &f, "iron", o.h.s, o.h.z.0, 0.03, 0.95, 0.1);
                    }
                }
            }
        }
        Balc::Covered => {
            if fr.floors >= 2 {
                let zf = fr.z(1) + 0.02;
                let s = (0.3, len - 0.3);
                let roof_z0 = fr.z(2) - 0.22;
                let out = 0.85;
                let posts = roof_z0 - 0.3 * (out - 0.06) / 1.05 - 0.02;
                wooden_balcony(b, &f, s, zf, out, l.door, Some(posts));
                f.leaning(
                    b,
                    l.roof,
                    (0.15, len - 0.15),
                    (roof_z0, 0.0),
                    (roof_z0 - 0.3, 1.05),
                    0.06,
                );
                // Brackets under the balcony.
                let m = ((len - 0.6) / 1.2).round().max(1.0) as usize;
                for i in 0..=m {
                    let x = (0.4 + (len - 0.8) * i as f64 / m as f64).clamp(0.4, len - 0.4);
                    corbel(b, &f, x, zf - 0.26, out, l.door);
                }
            }
        }
        Balc::Gallery => {
            for k in 1..fr.floors {
                gallery(b, &f, (0.3, len - 0.3), fr.z(k), fh, l);
            }
        }
        Balc::FireEscape => {
            if axes.len() >= 2 && fr.floors >= 3 {
                let i0 = if axes.len() >= 3 { 1 } else { 0 };
                let i1 = (i0 + 1).min(axes.len() - 1);
                let s = (axes[i0] - ww / 2.0 - 0.35, axes[i1] + ww / 2.0 + 0.35);
                fire_escape(b, fr, s);
            }
        }
        Balc::Lace => {
            if fr.floors >= 2 {
                verandah(b, fr, l);
            }
        }
        Balc::Mashrabiya => {
            for o in ops.iter().filter(|o| o.storey >= 1 && o.storey <= 2) {
                if o.storey == 2 && fr.floors < 4 {
                    continue;
                }
                if (o.axis + o.storey) % 2 == 1 && axes.len() > 1 {
                    continue;
                }
                let c = (o.h.s.0 + o.h.s.1) / 2.0;
                let zf = fr.z(o.storey);
                mashrabiya(
                    b,
                    &f,
                    (c - (ww / 2.0 + 0.35), c + (ww / 2.0 + 0.35)),
                    (zf + 0.25, zf + fh - 0.3),
                    l,
                );
            }
        }
        Balc::Cage => {
            for o in ops
                .iter()
                .filter(|o| o.storey >= 1 && o.kind == Kind::Window)
            {
                cage(b, &f, o.h, fr.seed + o.axis as u64 * 13 + o.storey as u64);
            }
        }
        Balc::Glazed => {
            for o in ops.iter().filter(|o| o.kind == Kind::Door) {
                let s = (o.h.s.0 - 0.5, (o.h.s.1 + step - ww - 0.2).min(len - 0.2));
                flat_balcony(
                    b,
                    fr,
                    l,
                    s,
                    o.h.z.0 - 0.02,
                    unit(fr.seed, 300 + (o.axis * 31 + o.storey) as u64),
                );
            }
        }
        Balc::Slab => {
            for k in 1..fr.floors {
                slab_balcony(b, fr, l, fr.z(k), axes, step);
            }
        }
        _ => {}
    }
}

/// A console under a balcony.
fn corbel(b: &mut Builder, f: &Face, x: f64, z: f64, out: f64, role: &'static str) {
    let side = Face {
        o: [f.at(x, 0.0, 0.0)[0], f.at(x, 0.0, 0.0)[1]],
        u: f.out(),
        len: out,
    };
    side.prism(
        b,
        role,
        &[
            (-0.02, z + 0.1),
            (out * 0.7, z + 0.1),
            (out * 0.15, z - 0.12),
            (-0.02, z - 0.3),
        ],
        (-0.05, 0.05),
    );
}

/// A glazed gallery over a floor: its floor, the solid apron, white
/// frames with small panes up top, glass, and its roof.
fn gallery(b: &mut Builder, f: &Face, s: (f64, f64), zf: f64, fh: f64, l: &Look) {
    let out = 0.62;
    let (z0, z1) = (zf, zf + fh - 0.2);
    let fr = l.frame;
    f.bx(b, fr, s, (z0 - 0.12, z0 + 0.02), (-0.02, out));
    f.bx(b, fr, s, (z1, z1 + 0.14), (-0.02, out + 0.04));
    f.bx(b, fr, s, (z0, z0 + 0.9), (out - 0.06, out));
    f.bx(b, "glass", s, (z0 + 0.9, z1), (out - 0.04, out - 0.03));
    for z in [z0 + 0.9, z1 - 0.45] {
        f.bx(b, fr, s, (z - 0.03, z + 0.03), (out - 0.07, out));
    }
    let n = ((s.1 - s.0) / 0.48).round().max(1.0) as usize;
    for i in 0..=n {
        let x = s.0 + (s.1 - s.0) * i as f64 / n as f64;
        let t = if i == 0 || i == n { 0.04 } else { 0.022 };
        let x = x.clamp(s.0 + t, s.1 - t);
        f.bx(b, fr, (x - t, x + t), (z0 + 0.9, z1), (out - 0.07, out));
    }
    // The ends.
    for (x, inward) in [(s.0, 1.0), (s.1, -1.0)] {
        let side = Face {
            o: [f.at(x, 0.0, 0.0)[0], f.at(x, 0.0, 0.0)[1]],
            u: f.out(),
            len: out,
        };
        let d = if inward > 0.0 {
            (-0.04, 0.0)
        } else {
            (0.0, 0.04)
        };
        side.bx(b, fr, (0.0, out), (z0, z0 + 0.9), d);
        side.bx(
            b,
            "glass",
            (0.0, out),
            (z0 + 0.9, z1),
            (d.0 + 0.015, d.1 - 0.015),
        );
        side.bx(b, fr, (0.0, out), (z1 - 0.48, z1 - 0.42), d);
    }
}

/// A fire escape: a landing on each floor, rails, stairs between them
/// and the ladder over the street.
fn fire_escape(b: &mut Builder, fr: &Front, s: (f64, f64)) {
    let f = fr.f;
    let out = 1.05;
    for k in 1..fr.floors {
        let zf = fr.z(k);
        // Landing of bars.
        f.bx(b, "iron", s, (zf - 0.06, zf - 0.02), (0.0, out));
        let mut x = s.0;
        while x < s.1 {
            f.bx(b, "iron", (x, x + 0.015), (zf - 0.02, zf), (0.0, out));
            x += 0.08;
        }
        railing(b, &f, "iron", s, zf, out - 0.03, 0.95, 0.12);
        for (x, sgn) in [(s.0, 1.0), (s.1, -1.0)] {
            let side = Face {
                o: [f.at(x, 0.0, 0.0)[0], f.at(x, 0.0, 0.0)[1]],
                u: f.out(),
                len: out,
            };
            railing(b, &side, "iron", (0.0, out), zf, 0.015 * sgn, 0.95, 0.12);
        }
        // Brackets.
        for x in [s.0 + 0.1, s.1 - 0.1] {
            b.cone(
                "iron",
                f.at(x, zf - 0.6, 0.0),
                f.at(x, zf - 0.05, out - 0.05),
                0.015,
                0.015,
                4,
            );
        }
        // The stair down to the landing below (or the ladder).
        let (a, e) = (s.0 + 0.25, s.1 - 0.35);
        let dd = out * 0.5;
        let zl = if k == 1 { zf - 2.2 } else { fr.z(k - 1) };
        let (top, low) = if k % 2 == 0 { (a, e) } else { (e, a) };
        for side_d in [dd - 0.3, dd + 0.3] {
            b.cone(
                "iron",
                f.at(top, zf, side_d),
                f.at(low, zl, side_d),
                0.018,
                0.018,
                4,
            );
        }
        let m = ((zf - zl) / 0.22).round().max(2.0) as usize;
        for j in 1..m {
            let t = j as f64 / m as f64;
            let x = top + (low - top) * t;
            let z = zf + (zl - zf) * t;
            f.bx(
                b,
                "iron",
                (x - 0.1, x + 0.1),
                (z - 0.015, z),
                (dd - 0.3, dd + 0.3),
            );
        }
    }
}

/// A verandah of two floors along a front between party walls: the
/// floor of the upper one, cast iron columns, lace and the roof, and the
/// wing walls at the sides.
fn verandah(b: &mut Builder, fr: &Front, l: &Look) {
    let f = fr.f;
    let len = f.len;
    let out = 1.9;
    let z1 = fr.z(1);
    let roof_hi = fr.z(2).min(fr.top()) - 0.15;
    let roof_lo = roof_hi - 0.55;
    f.bx(
        b,
        l.door,
        (0.2, len - 0.2),
        (z1 - 0.16, z1 + 0.02),
        (-0.02, out + 0.05),
    );
    railing(
        b,
        &f,
        "iron",
        (0.25, len - 0.25),
        z1 + 0.02,
        out - 0.02,
        0.95,
        0.09,
    );
    grille_plane(
        b,
        &f,
        "iron",
        (0.25, len - 0.25),
        (z1 - 0.62, z1 - 0.22),
        out,
        0.09,
        3,
    );
    let zr = roof_hi - (roof_hi - roof_lo) * (out / (out + 0.15));
    grille_plane(
        b,
        &f,
        "iron",
        (0.25, len - 0.25),
        (zr - 0.45, zr - 0.08),
        out,
        0.09,
        3,
    );
    let n = ((len - 0.5) / 2.4).round().max(1.0) as usize;
    for i in 0..=n {
        let x = 0.25 + (len - 0.5) * i as f64 / n as f64;
        f.bar(b, "iron", (x, 0.0), (x, zr), out, 0.05, 8);
    }
    f.leaning(
        b,
        l.roof,
        (0.1, len - 0.1),
        (roof_hi, 0.0),
        (roof_lo, out + 0.15),
        0.05,
    );
    if fr.party {
        for (a, e) in [(0.0, 0.22), (len - 0.22, len)] {
            f.bx(b, l.wall, (a, e), (0.0, roof_hi), (0.0, out + 0.15));
            f.bx(
                b,
                l.trim,
                (a - 0.02, e + 0.02),
                (roof_hi, roof_hi + 0.08),
                (0.0, out + 0.2),
            );
        }
    }
}

/// A closed bay of wooden lattice on its brackets, with its little roof.
fn mashrabiya(b: &mut Builder, f: &Face, s: (f64, f64), z: (f64, f64), l: &Look) {
    let out = 0.55;
    let role = l.door;
    f.bx(b, role, s, (z.0 - 0.12, z.0), (-0.02, out));
    f.bx(
        b,
        role,
        (s.0 - 0.05, s.1 + 0.05),
        (z.1, z.1 + 0.1),
        (-0.02, out + 0.06),
    );
    f.bx(
        b,
        role,
        (s.0 + 0.02, s.1 - 0.02),
        (z.0, z.1),
        (out - 0.18, out - 0.15),
    );
    grille_plane(b, f, role, s, (z.0, z.1), out - 0.02, 0.06, 9);
    for (x, sgn) in [(s.0, 1.0), (s.1, -1.0)] {
        let side = Face {
            o: [f.at(x, 0.0, 0.0)[0], f.at(x, 0.0, 0.0)[1]],
            u: f.out(),
            len: out,
        };
        grille_plane(b, &side, role, (0.0, out), (z.0, z.1), 0.015 * sgn, 0.06, 9);
        corbel(b, f, x + sgn * 0.1, z.0 - 0.14, out, role);
    }
}

/// A cage of bars round a window, with its tray.
fn cage(b: &mut Builder, f: &Face, h: Hole, seed: u64) {
    let out = 0.38;
    let (s0, s1) = (h.s.0 - 0.06, h.s.1 + 0.06);
    let (z0, z1) = (h.z.0 - 0.06, h.z.1 + 0.08);
    f.bx(b, "metal", (s0, s1), (z0 - 0.04, z0), (0.0, out));
    f.bx(b, "metal", (s0, s1), (z1, z1 + 0.03), (0.0, out));
    grille_plane(b, f, "metal", (s0, s1), (z0, z1), out - 0.012, 0.11, 2);
    for (x, sgn) in [(s0, 1.0), (s1, -1.0)] {
        let side = Face {
            o: [f.at(x, 0.0, 0.0)[0], f.at(x, 0.0, 0.0)[1]],
            u: f.out(),
            len: out,
        };
        grille_plane(
            b,
            &side,
            "metal",
            (0.0, out - 0.012),
            (z0, z1),
            0.012 * sgn,
            0.11,
            2,
        );
    }
    if unit(seed, 1) > 0.5 {
        standing_pot(b, f.at(s0 + 0.25, z0, out * 0.5), 0.6, seed);
    }
}

/// A balcony of its own for a flat: slab, front, and now and then glazed
/// in.
fn flat_balcony(b: &mut Builder, fr: &Front, l: &Look, s: (f64, f64), z: f64, pick: f64) {
    let f = fr.f;
    let out = 1.2;
    let fh = fr.storey.1;
    f.bx(b, l.trim, s, (z - 0.16, z), (-0.02, out));
    let solid = pick < 0.6;
    if solid {
        f.bx(b, l.wall, s, (z, z + 1.0), (out - 0.1, out));
        for x in [s.0, s.1] {
            let side = Face {
                o: [f.at(x, 0.0, 0.0)[0], f.at(x, 0.0, 0.0)[1]],
                u: f.out(),
                len: out,
            };
            let d = if x == s.0 { (-0.1, 0.0) } else { (0.0, 0.1) };
            side.bx(b, l.wall, (0.0, out - 0.1), (z, z + 1.0), d);
        }
    } else {
        railing(
            b,
            &f,
            "iron",
            (s.0 + 0.04, s.1 - 0.04),
            z,
            out - 0.05,
            1.0,
            0.11,
        );
    }
    if pick < 0.35 {
        // Glazed in: frames and glass over the front.
        let zt = z + fh - 0.2;
        f.bx(b, "f_alu", s, (z + 1.0, z + 1.05), (out - 0.1, out));
        f.bx(b, "glass", s, (z + 1.05, zt), (out - 0.06, out - 0.05));
        f.bx(b, "f_alu", s, (zt, zt + 0.05), (out - 0.1, out));
        let n = ((s.1 - s.0) / 0.9).round().max(1.0) as usize;
        for i in 0..=n {
            let x = (s.0 + (s.1 - s.0) * i as f64 / n as f64).clamp(s.0 + 0.03, s.1 - 0.03);
            f.bx(
                b,
                "f_alu",
                (x - 0.03, x + 0.03),
                (z + 1.0, zt),
                (out - 0.1, out),
            );
        }
        f.bx(b, l.trim, s, (zt + 0.05, zt + 0.15), (-0.02, out));
    } else if pick > 0.75 && fr.has(LAUNDRY) {
        clothes_line(b, &f, (s.0 + 0.2, s.1 - 0.2), z + 1.9, fr.seed + 9);
    } else if pick > 0.6 {
        standing_pot(b, f.at(s.0 + 0.3, z, out * 0.5), 0.8, fr.seed);
    }
}

/// A balcony along a floor with solid fronts, partitions between flats
/// and what stands on it.
fn slab_balcony(b: &mut Builder, fr: &Front, l: &Look, zf: f64, axes: &[f64], step: f64) {
    let f = fr.f;
    let len = f.len;
    let out = 1.15;
    f.bx(b, l.trim, (0.0, len), (zf - 0.18, zf + 0.02), (-0.02, out));
    f.bx(
        b,
        l.trim,
        (0.0, len),
        (zf + 0.02, zf + 1.1),
        (out - 0.08, out),
    );
    f.bx(
        b,
        "curtain",
        (0.05, len - 0.05),
        (zf + 0.35, zf + 1.0),
        (out - 0.1, out + 0.005),
    );
    for i in 0..=axes.len() {
        let x = if i == 0 {
            0.04
        } else if i == axes.len() {
            len - 0.04
        } else {
            axes[i - 1] + step / 2.0
        };
        f.bx(
            b,
            l.trim,
            (x - 0.03, x + 0.03),
            (zf + 0.02, zf + 2.0),
            (0.0, out - 0.08),
        );
    }
    for (i, &c) in axes.iter().enumerate() {
        let k = (zf * 10.0) as u64 + i as u64;
        if unit(fr.seed, 400 + k) < 0.55 {
            let p = f.at(c + step * 0.3, zf + 0.02, out * 0.45);
            b.rbox(
                "appliance",
                [p[0] - 0.35, p[1] - 0.15, zf + 0.02],
                [p[0] + 0.35, p[1] + 0.15, zf + 0.58],
                0.02,
            );
        }
        if fr.has(LAUNDRY) && unit(fr.seed, 500 + k) < 0.4 {
            f.bar(
                b,
                "metal",
                (c - step * 0.4, zf + 1.9),
                (c + step * 0.2, zf + 1.9),
                out * 0.6,
                0.015,
                6,
            );
            for x in [c - step * 0.4, c + step * 0.2] {
                f.bx(
                    b,
                    "metal",
                    (x - 0.012, x + 0.012),
                    (zf + 1.89, zf + 1.92),
                    (-0.01, out * 0.6 + 0.01),
                );
            }
            let mut x = c - step * 0.35;
            let mut j = 0;
            while x < c + step * 0.1 {
                let role = ["cloth_a", "cloth_b", "cloth_c"]
                    [(unit(fr.seed, 600 + k * 5 + j) * 3.0) as usize % 3];
                f.bx(
                    b,
                    role,
                    (x, x + 0.3),
                    (zf + 1.3, zf + 1.89),
                    (out * 0.6 - 0.006, out * 0.6 + 0.006),
                );
                x += 0.4;
                j += 1;
            }
        }
    }
}

/// A bay window on its storeys: three sides of windows, its floor and its
/// roof.
fn bay_window(b: &mut Builder, fr: &Front, sk: &Skin, l: &Look, s: (f64, f64), storeys: &[usize]) {
    let f = fr.f;
    let depth = 0.65;
    let cut = 0.45;
    let p = |x: f64, d: f64| -> [f64; 2] {
        let q = f.at(x, 0.0, d);
        [q[0], q[1]]
    };
    let corners = [
        p(s.0, 0.0),
        p(s.0 + cut, depth),
        p(s.1 - cut, depth),
        p(s.1, 0.0),
    ];
    let sides: Vec<Face> = (0..3)
        .map(|i| {
            let (a, e) = (corners[i], corners[i + 1]);
            let len = ((e[0] - a[0]).powi(2) + (e[1] - a[1]).powi(2)).sqrt();
            Face {
                o: a,
                u: [(e[0] - a[0]) / len, (e[1] - a[1]) / len],
                len,
            }
        })
        .collect();
    let lo = *storeys.first().unwrap();
    let hi = *storeys.last().unwrap();
    let z0 = fr.z(lo);
    let z1 = if hi == 0 {
        fr.z(0) + fr.storey.0 - 0.2
    } else {
        fr.z(hi) + fr.storey.1 - 0.25
    };
    // Base and roof.
    let poly = ccw(corners.to_vec());
    b.slab(
        l.base,
        &poly,
        if lo == 0 { 0.0 } else { z0 - 0.3 },
        z0 + 0.02,
    );
    b.slab(l.trim, &poly, z1, z1 + 0.18);
    for &k in storeys {
        let zf = fr.z(k);
        let hh = if k == 0 { fr.storey.0 } else { fr.storey.1 };
        let ztop = if k == hi { z1 } else { zf + hh };
        for (i, sf) in sides.iter().enumerate() {
            let m = if i == 1 { 0.12 } else { 0.08 };
            let h = Hole {
                s: (m, sf.len - m),
                z: (zf + 0.75, (zf + 0.75 + sk.win.1).min(ztop - 0.3)),
            };
            wall(b, l.trim, sf, (zf.max(z0), ztop), 0.12, &[h]);
            let win = Win {
                leaves: 1,
                reveal: 0.06,
                bars: if fr.sash == Sash::Sash { 1 } else { 0 },
                curtain: unit(fr.seed, 700 + (k * 3 + i) as u64) > 0.5,
                ..Default::default()
            };
            window(b, sf, h, &win, l.frame);
        }
        if k != hi {
            // The band between storeys.
            b.slab(l.trim, &poly, zf + hh - 0.05, zf + hh + 0.05);
        }
    }
}

/// Bands between floors, quoins, corner boards and the Algarve's painted
/// bands; rustication on the piers of the ground floor.
fn belts_and_corners(b: &mut Builder, fr: &Front, l: &Look, ops: &[Opening]) {
    let f = fr.f;
    let len = f.len;
    let top = fr.top();
    if fr.has(BELT) {
        for k in 1..fr.floors {
            let z = fr.z(k);
            f.bx(b, l.trim, (0.0, len), (z - 0.16, z - 0.02), (-0.01, 0.06));
        }
    }
    if fr.has(QUOINS) && !fr.party {
        let mut z = 0.0;
        let mut long = true;
        while z < top - 0.3 {
            let w = if long { 0.5 } else { 0.3 };
            for (a, e) in [(0.0, w), (len - w, len)] {
                f.bx(b, l.trim, (a, e), (z + 0.02, z + 0.34), (-0.01, 0.025));
            }
            z += 0.36;
            long = !long;
        }
    }
    if fr.has(CORNER_BOARDS) {
        for (a, e) in [(0.0, 0.16), (len - 0.16, len)] {
            f.bx(b, "b_white", (a, e), (fr.lift.max(0.0), top), (-0.01, 0.03));
        }
    }
    if fr.has(ALGARVE) {
        for (a, e) in [(0.0, 0.25), (len - 0.25, len)] {
            f.bx(b, l.accent, (a, e), (0.0, top), (-0.01, 0.012));
        }
        f.bx(b, l.accent, (0.0, len), (0.0, 0.5), (-0.01, 0.012));
    }
    if fr.has(RUSTIC) && !fr.plain {
        // Channels on the piers of the ground floor.
        let zg = fr.z(1) - 0.05;
        let mut cuts: Vec<(f64, f64)> = ops
            .iter()
            .filter(|o| o.storey == 0)
            .map(|o| (o.h.s.0 - 0.15, o.h.s.1 + 0.15))
            .collect();
        cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut s = 0.0;
        let mut piers = Vec::new();
        for (a, e) in cuts {
            if a > s + 0.1 {
                piers.push((s, a));
            }
            s = s.max(e);
        }
        if len > s + 0.1 {
            piers.push((s, len));
        }
        for (a, e) in piers {
            let mut z = 0.05;
            while z < zg - 0.3 {
                f.bx(b, l.base, (a, e), (z, z + 0.34), (-0.01, 0.025));
                z += 0.4;
            }
        }
    }
}

/// The cornice at the top of the front.
fn cornice(b: &mut Builder, fr: &Front, l: &Look, top: f64) {
    let f = fr.f;
    let len = f.len;
    match fr.top {
        Top::None => {}
        Top::Band => {
            f.bx(b, l.trim, (0.0, len), (top - 0.3, top), (-0.01, 0.12));
        }
        Top::Tiles => {
            f.bx(
                b,
                l.trim,
                (0.0, len),
                (top - 0.32, top - 0.2),
                (-0.01, 0.07),
            );
            f.bx(
                b,
                "tiles",
                (0.0, len),
                (top - 0.2, top - 0.1),
                (-0.01, 0.15),
            );
            f.bx(b, l.trim, (0.0, len), (top - 0.1, top), (-0.01, 0.2));
        }
        Top::Brackets => {
            f.bx(
                b,
                l.trim,
                (0.0, len),
                (top - 0.75, top - 0.45),
                (-0.01, 0.06),
            );
            f.bx(
                b,
                l.trim,
                (-0.05, len + 0.05),
                (top - 0.12, top + 0.08),
                (-0.01, 0.55),
            );
            f.bx(
                b,
                l.trim,
                (-0.03, len + 0.03),
                (top - 0.2, top - 0.12),
                (-0.01, 0.42),
            );
            let n = (len / 0.75).round().max(2.0) as usize;
            for i in 0..=n {
                let x = (len * i as f64 / n as f64).clamp(0.12, len - 0.12);
                let side = Face {
                    o: [f.at(x, 0.0, 0.0)[0], f.at(x, 0.0, 0.0)[1]],
                    u: f.out(),
                    len: 0.5,
                };
                side.prism(
                    b,
                    l.trim,
                    &[
                        (-0.01, top - 0.2),
                        (0.48, top - 0.2),
                        (0.12, top - 0.62),
                        (-0.01, top - 0.72),
                    ],
                    (-0.05, 0.05),
                );
            }
        }
        Top::Heavy => {
            f.bx(b, l.trim, (0.0, len), (top - 0.5, top - 0.3), (-0.01, 0.12));
            f.bx(
                b,
                l.trim,
                (-0.03, len + 0.03),
                (top - 0.3, top - 0.15),
                (-0.01, 0.3),
            );
            f.bx(
                b,
                l.trim,
                (-0.06, len + 0.06),
                (top - 0.15, top),
                (-0.01, 0.48),
            );
            let mut x = 0.1;
            while x < len - 0.1 {
                f.bx(
                    b,
                    l.trim,
                    (x, x + 0.07),
                    (top - 0.42, top - 0.3),
                    (0.1, 0.18),
                );
                x += 0.16;
            }
        }
        Top::Balustrade => {
            f.bx(b, l.trim, (0.0, len), (top - 0.3, top), (-0.01, 0.15));
        }
    }
}

/// What hangs on the walls: flower boxes, air conditioners, washing,
/// pots, blade signs, pent roofs.
fn hangings(b: &mut Builder, fr: &Front, l: &Look, ops: &[Opening], step: f64, ww: f64) {
    let f = fr.f;
    for (j, o) in ops.iter().enumerate() {
        let h = o.h;
        let key = 800 + j as u64 * 11;
        let windowish = o.kind == Kind::Window && o.storey > 0;
        if fr.has(FLOWERS) && windowish && unit(fr.seed, key) < 0.55 {
            f.bx(
                b,
                "pot",
                (h.s.0, h.s.1),
                (h.z.0 - 0.24, h.z.0 - 0.04),
                (0.0, 0.22),
            );
            let c = f.at((h.s.0 + h.s.1) / 2.0, h.z.0, 0.11);
            let half = (h.s.1 - h.s.0) / 2.0;
            let along = [f.u[0] * half, f.u[1] * half, 0.0];
            b.blob(
                "leaves",
                [c[0], c[1], h.z.0 - 0.02],
                along,
                [0.0, 0.0, 0.15],
                [f.out()[0] * 0.15, f.out()[1] * 0.15, 0.0],
            );
            let m = ((h.s.1 - h.s.0) / 0.16) as usize;
            for k in 0..m {
                let x = h.s.0 + 0.08 + 0.16 * k as f64;
                let q = f.at(x, h.z.0 + 0.08 + 0.04 * unit(fr.seed, key + k as u64), 0.13);
                b.cone(
                    "flowers",
                    [q[0], q[1], q[2] - 0.03],
                    [q[0], q[1], q[2] + 0.04],
                    0.05,
                    0.03,
                    5,
                );
            }
        }
        if fr.has(AC) && windowish && unit(fr.seed, key + 1) < 0.28 {
            let room = step - ww;
            if room > 1.1 {
                ac_unit(b, &f, h.s.1 + room / 2.0, h.z.0 + 0.2);
            } else {
                ac_unit(b, &f, (h.s.0 + h.s.1) / 2.0, h.z.0 - 0.75);
            }
        }
        if fr.has(LAUNDRY) && windowish && fr.balc != Balc::Cage && unit(fr.seed, key + 2) < 0.18 {
            clothes_line(
                b,
                &f,
                (h.s.0 - 0.1, h.s.1 + 0.1),
                h.z.0 - 0.15,
                fr.seed + j as u64,
            );
        }
        if fr.has(PLANTS) && o.kind == Kind::Entrance && unit(fr.seed, key + 3) < 0.6 {
            // A climber over the door.
            let c = f.at(h.s.1 + 0.3, h.z.1 + 0.3, 0.12);
            b.blob(
                "leaves",
                c,
                [f.u[0] * 0.6, f.u[1] * 0.6, 0.0],
                [0.0, 0.0, 0.5],
                [f.out()[0] * 0.12, f.out()[1] * 0.12, 0.0],
            );
            for k in 0..9 {
                let p = f.at(
                    h.s.1 + 0.3 + (unit(fr.seed, key + 20 + k) - 0.5) * 1.0,
                    h.z.1 + 0.3 + (unit(fr.seed, key + 40 + k) - 0.5) * 0.8,
                    0.2,
                );
                b.ellipsoid("flowers", p, [0.07, 0.04, 0.07]);
            }
        }
        if fr.has(SIGNS)
            && !fr.plain
            && o.storey <= 1
            && o.kind != Kind::Entrance
            && unit(fr.seed, key + 4) < 0.45
        {
            let x = h.s.1 + 0.18;
            if x < f.len - 0.2 {
                let side = Face {
                    o: [f.at(x, 0.0, 0.0)[0], f.at(x, 0.0, 0.0)[1]],
                    u: f.out(),
                    len: 0.9,
                };
                let role = if unit(fr.seed, key + 5) < 0.5 {
                    "sign"
                } else {
                    "sign_b"
                };
                let z0 = fr.z(o.storey) + 0.6;
                side.bx(b, role, (0.2, 0.8), (z0, z0 + 2.2), (-0.06, 0.06));
                side.bx(b, "metal", (0.0, 0.2), (z0 + 0.3, z0 + 0.34), (-0.02, 0.02));
                side.bx(b, "metal", (0.0, 0.2), (z0 + 1.8, z0 + 1.84), (-0.02, 0.02));
            }
        }
    }
    if fr.has(PENT) && !fr.plain {
        for k in 1..fr.floors {
            let z = fr.z(k) + 0.1;
            f.leaning(b, l.roof, (0.0, f.len), (z, 0.0), (z - 0.28, 0.75), 0.05);
            for x in [0.2, f.len - 0.2] {
                let p0 = f.at(x, z - 0.6, 0.0);
                let p1 = f.at(x, z - 0.03, 0.6);
                b.cone(l.door, p0, p1, 0.035, 0.035, 4);
            }
        }
    }
}

/// The point of a front's face, as a point of the plan.
pub fn on(f: &Face, s: f64, d: f64) -> [f64; 2] {
    let p: P3 = f.at(s, 0.0, d);
    [p[0], p[1]]
}

/// A polygon of the plan turned anticlockwise (as `Builder::slab` wants).
pub fn ccw(mut poly: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    let mut a = 0.0;
    for i in 0..poly.len() {
        let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
        a += p[0] * q[1] - q[0] * p[1];
    }
    if a < 0.0 {
        poly.reverse();
    }
    poly
}
