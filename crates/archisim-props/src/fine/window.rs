//! Windows made with care (paquete 9, carpintería, segunda parte): the
//! frame and the sashes with the profile of their collection (straight or
//! rounded aluminium, PVC, timber plain or moulded, slim steel), the glass
//! held by beads and divided by glazing bars, the handle of each sash
//! (levers, a surface espagnolette, a wrought bolt) and its hinges; and
//! what goes outside — the roller blind down its guides, a pair of
//! shutters (louvred, boarded or panelled) open against the façade or
//! closed, and a grille (bars, bars with flats, a grid, wrought scrolls),
//! flat on the façade or a box standing out of it.
//!
//! The window family keeps in its recipe what its IFC and its plan say
//! (the subframe, the transom, the blind's box, the sill); with `detailed`
//! on, it leaves its frame, mullions and sashes out and tells this
//! generator where they go (`w_gc0`, `w_gc1`, `w_z0`, `w_z1`, `w_zl`,
//! `w_fw`, `w_left`, `w_right`, `w_outside`, `w_hand_left`, `w_width`,
//! `w_top`), and the program adds the wall's faces (`face_pos`, `face_neg`,
//! `face_ext`).
//!
//! Axes as the family's: `x` along the wall from the centre of the
//! opening, `y` across it from the middle of the frame towards the side
//! the sashes open to, `z` up from the sill.
//!
//! The sashes' bars and the grilles are data — [`Bars`] and [`Grille`] —
//! as the doors' leaves are, for the workshop of `docs/CARPINTERIA.md`.

use super::door::{self, Field, LeafDesign, Profile, Row};
use super::shape::{bar, sweep, Cap};
use crate::build::Builder;
use crate::Params;

const FRAME: &str = "frame_material";
const GLASS: &str = "panel_material";
const METAL: &str = "hardware_material";
const GRILLE: &str = "grille_material";
const SHUTTERS: &str = "shutters_material";

/// The collections, in the order of the family's «Modelo 3D».
pub const COLLECTIONS: [&str; 8] = door::COLLECTIONS;

/// The profiles the family offers, by name.
pub const PROFILES: &[&str] = &[
    "Como la colección",
    "Aluminio recto",
    "Aluminio redondeado",
    "PVC",
    "Madera",
    "Madera con moldura",
    "Acero fino",
];

/// How a profile is made: how wide its sash shows, how deep it is, how
/// rounded its edges, how far the sash stands proud of the frame on the
/// inside, whether the glass has a moulded bead and the hinges show.
#[derive(Debug, Clone, Copy)]
struct Section {
    sash: f64,
    depth: f64,
    round: f64,
    proud: f64,
    moulded: bool,
    hinges: bool,
    /// Seen from far away: no rebate, beads, bars, handle or hinges.
    plain: bool,
}

fn section(n: usize) -> Section {
    let s = |sash, depth, round, proud, moulded, hinges| Section {
        sash,
        depth,
        round,
        proud,
        moulded,
        hinges,
        plain: false,
    };
    match n {
        3 => s(0.06, 0.065, 0.006, 0.006, false, false),
        4 => s(0.07, 0.07, 0.008, 0.008, false, false),
        5 => s(0.065, 0.068, 0.002, 0.012, false, true),
        6 => s(0.07, 0.068, 0.002, 0.012, true, true),
        7 => s(0.032, 0.045, 0.0005, 0.0, false, true),
        _ => s(0.055, 0.06, 0.001, 0.0, false, false),
    }
}

/// How the glass of a sash is divided.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bars {
    None,
    /// A grid of panes about `w` wide and `h` high.
    Grid {
        w: f64,
        h: f64,
    },
    /// Bars across, about `h` apart.
    Across(f64),
    /// Bars up, about `w` apart.
    Up(f64),
    /// A grid of panes about `w` by `h` in the top third, over a bar.
    Top {
        w: f64,
        h: f64,
    },
}

/// The glazing bars the family offers, by name.
pub const BARS: &[&str] = &[
    "Como la colección",
    "Sin cuarterones",
    "Cuadrícula",
    "Cuadrícula fina",
    "Horizontales",
    "Verticales",
    "Solo arriba",
];

/// The bars by their number in [`BARS`].
pub fn bars(n: usize) -> Bars {
    match n {
        3 => Bars::Grid { w: 0.3, h: 0.34 },
        4 => Bars::Grid { w: 0.18, h: 0.2 },
        5 => Bars::Across(0.26),
        6 => Bars::Up(0.2),
        7 => Bars::Top { w: 0.2, h: 0.18 },
        _ => Bars::None,
    }
}

/// The handles the family offers, by name.
pub const HANDLES: &[&str] = &[
    "Como la colección",
    "Manilla moderna",
    "Manilla de latón",
    "Cremona vista",
    "Falleba de forja",
    "Sin manilla",
];

/// What closes the opening from outside.
pub const SHUTTERS_KINDS: &[&str] = &[
    "Sin contraventanas",
    "Mallorquinas",
    "De tablas",
    "De cuarterones",
];

/// A grille: how its bars run and what it carries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Grille {
    None,
    /// Upright bars about `step` apart.
    Bars {
        step: f64,
    },
    /// Upright bars through flats across, `flats` of them.
    Flats {
        step: f64,
        flats: usize,
    },
    /// Bars up and across, a grid of `step`.
    Grid {
        step: f64,
    },
    /// Upright bars with a band of scrolls at the top and the bottom.
    Scrolls {
        step: f64,
    },
}

/// The grilles the family offers, by name.
pub const GRILLES: &[&str] = &[
    "Sin reja",
    "Barrotes",
    "Barrotes y pletinas",
    "Cuadrícula",
    "Forja con volutas",
];

/// The grille by its number in [`GRILLES`].
pub fn grille(n: usize) -> Grille {
    match n {
        2 => Grille::Bars { step: 0.12 },
        3 => Grille::Flats {
            step: 0.12,
            flats: 2,
        },
        4 => Grille::Grid { step: 0.15 },
        5 => Grille::Scrolls { step: 0.13 },
        _ => Grille::None,
    }
}

/// How a collection makes a window (numbers in the lists above).
#[derive(Debug, Clone, Copy)]
struct Look {
    profile: usize,
    bars: usize,
    handle: usize,
}

const LOOKS: [Look; 8] = [
    // Nórdico cálido: timber, plain glass, modern lever.
    Look {
        profile: 5,
        bars: 2,
        handle: 2,
    },
    // Mediterráneo: moulded timber, a grid, a surface espagnolette.
    Look {
        profile: 6,
        bars: 3,
        handle: 4,
    },
    // Años 50: slim steel, bars across, brass lever.
    Look {
        profile: 7,
        bars: 5,
        handle: 3,
    },
    // Japandi: timber, bars up, modern lever.
    Look {
        profile: 5,
        bars: 6,
        handle: 2,
    },
    // Contemporáneo: straight aluminium, plain glass, modern lever.
    Look {
        profile: 2,
        bars: 2,
        handle: 2,
    },
    // Rústica: timber, a grid, a wrought bolt.
    Look {
        profile: 5,
        bars: 3,
        handle: 5,
    },
    // Industrial: slim steel, a fine grid, modern lever.
    Look {
        profile: 7,
        bars: 4,
        handle: 2,
    },
    // Clásica: moulded timber, panes in the top third, brass lever.
    Look {
        profile: 6,
        bars: 7,
        handle: 3,
    },
];

fn look(p: &Params) -> Look {
    let s = (p.style() as usize).clamp(1, 8);
    let mut l = LOOKS[s - 1];
    let pick = |key: &str| {
        let n = p.get(key, 1.0).round() as usize;
        (n >= 2).then_some(n)
    };
    if let Some(n) = pick("window_profile") {
        l.profile = n;
    }
    if let Some(n) = pick("glazing_bars") {
        l.bars = n;
    }
    if let Some(n) = pick("window_handle") {
        l.handle = n;
    }
    l
}

/// How the sashes open, as the family names them («Hoja 1…4»).
pub const OPERATIONS: &[&str] = &[
    "fija",
    "practicable",
    "oscilobatiente",
    "abatible",
    "proyectante",
    "basculante",
    "pivotante",
];

/// How each sash opens (the family's «Hoja 1…4», as numbers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Fixed,
    Side,
    TiltTurn,
    Bottom,
    Top,
    Horizontal,
    Vertical,
}

fn op(n: f64) -> Op {
    match n.round() as i32 {
        1 => Op::Fixed,
        3 => Op::TiltTurn,
        4 => Op::Bottom,
        5 => Op::Top,
        6 => Op::Horizontal,
        7 => Op::Vertical,
        _ => Op::Side,
    }
}

/// A window with its frame, sashes, glass and hardware, and what goes
/// outside it.
pub fn window(b: &mut Builder, p: &Params) {
    if outside_only(b, p) {
        return;
    }
    let l = look(p);
    // Far away (`lod`): frame, sashes and glass, square, with nothing on
    // them; what goes outside plain.
    let mut sec = section(l.profile);
    if p.flag("lod", false) {
        sec.round = 0.0;
        sec.moulded = false;
        sec.hinges = false;
        sec.plain = true;
    }
    let fw = p.get("w_fw", 0.06);
    let fd = p.get("frame_depth", 0.07);
    let gc0 = p.get("w_gc0", -0.5);
    let gc1 = p.get("w_gc1", 0.5);
    let w = p.get("w_w", gc1 - gc0 + 2.0 * fw);
    let z0 = p.get("w_z0", 0.0);
    let z1 = p.get("w_z1", 1.2);
    let zl = p.get("w_zl", z1);
    let left = p.flag("w_left", true);
    let right = p.flag("w_right", true);
    let outside = if p.get("w_outside", -1.0) > 0.0 {
        1.0
    } else {
        -1.0
    };
    let inside = -outside;
    let leaves = p.get("leaves", 1.0).round().clamp(1.0, 4.0) as usize;
    let hand_left = p.flag("w_hand_left", true);
    let r = sec.round;

    // The frame: jambs, sill and head, and the mullions between sashes.
    let (ya, yb) = (-fd / 2.0, fd / 2.0);
    if left {
        b.rbox(FRAME, [-w / 2.0, ya, z0], [gc0, yb, z1], r);
    }
    if right {
        b.rbox(FRAME, [gc1, ya, z0], [w / 2.0, yb, z1], r);
    }
    b.rbox(FRAME, [gc0 - 0.001, ya, z0], [gc1 + 0.001, yb, z0 + fw], r);
    b.rbox(FRAME, [gc0 - 0.001, ya, z1 - fw], [gc1 + 0.001, yb, z1], r);
    let clear = (gc1 - gc0 - (leaves as f64 - 1.0) * fw) / leaves as f64;
    for i in 1..leaves {
        let a = gc0 + i as f64 * (clear + fw);
        b.rbox(
            FRAME,
            [a - fw, ya, z0 + fw - 0.001],
            [a, yb, zl - fw + 0.001],
            r,
        );
    }
    // The rebate on the outside edge of the frame the sashes close
    // against, all round each clear opening.
    let lip = 0.012;
    let (ra, rb) = sorted(outside * fd / 2.0, outside * (fd / 2.0 - 0.02));
    let ops = [
        p.get("leaf_1", 2.0),
        p.get("leaf_2", 2.0),
        p.get("leaf_3", 2.0),
        p.get("leaf_4", 2.0),
    ];
    let bars = if sec.plain { Bars::None } else { bars(l.bars) };
    let handle = if sec.plain { 6 } else { l.handle };
    let (za, zc) = (z0 + fw, zl - fw);
    for (i, &o) in ops.iter().enumerate().take(leaves) {
        let a = gc0 + i as f64 * (clear + fw);
        let c = a + clear;
        let kind = op(o);
        // Which side its hinges are on: the left half of the window on
        // the left, the right half on the right, the middle one by hand.
        let hinge_left = if 2 * i + 1 == leaves {
            hand_left
        } else {
            2 * i + 1 < leaves
        };
        if !sec.plain {
            b.rbox(
                FRAME,
                [a - 0.0005, ra, za - 0.0005],
                [a + lip, rb, zc + 0.0005],
                0.0,
            );
            b.rbox(
                FRAME,
                [c - lip, ra, za - 0.0005],
                [c + 0.0005, rb, zc + 0.0005],
                0.0,
            );
            b.rbox(
                FRAME,
                [a + lip, ra, za - 0.0005],
                [c - lip, rb, za + lip],
                0.0,
            );
            b.rbox(
                FRAME,
                [a + lip, ra, zc - lip],
                [c - lip, rb, zc + 0.0005],
                0.0,
            );
        }
        if kind == Op::Fixed {
            // Glass straight in the frame, held by beads.
            pane(b, &sec, bars, a, c, za, zc, 0.0, 0.024, inside);
            continue;
        }
        sash(
            b, &sec, bars, handle, kind, hinge_left, a, c, za, zc, fd, inside,
        );
    }

    shutters_blind(b, p, fd, outside, z0);
    outside_shutters(b, p, outside);
    grille_outside(b, p, outside);

    if p.flag("mirror", false) {
        mirror_x(b);
    }
}

fn sorted(a: f64, b: f64) -> (f64, f64) {
    (a.min(b), a.max(b))
}

/// A sash over the clear opening `a`…`c`, `za`…`zc`: its stiles and rails
/// overlapping the frame, its glass and bars, its handle and hinges.
#[allow(clippy::too_many_arguments)]
fn sash(
    b: &mut Builder,
    sec: &Section,
    bars: Bars,
    handle: usize,
    kind: Op,
    hinge_left: bool,
    a: f64,
    c: f64,
    za: f64,
    zc: f64,
    fd: f64,
    inside: f64,
) {
    let ov = 0.012;
    let (xa, xc, z_a, z_c) = (a - ov, c + ov, za - ov, zc + ov);
    let sw = sec.sash.min((xc - xa) / 3.0).min((z_c - z_a) / 3.0);
    // Its depth from the face it shows inside back towards the outside.
    let y_in = inside * (fd / 2.0 + sec.proud);
    let (ya, yb) = sorted(y_in, y_in - inside * sec.depth);
    let r = sec.round;
    b.rbox(FRAME, [xa, ya, z_a], [xa + sw, yb, z_c], r);
    b.rbox(FRAME, [xc - sw, ya, z_a], [xc, yb, z_c], r);
    b.rbox(
        FRAME,
        [xa + sw - 0.001, ya, z_a],
        [xc - sw + 0.001, yb, z_a + sw],
        r,
    );
    b.rbox(
        FRAME,
        [xa + sw - 0.001, ya, z_c - sw],
        [xc - sw + 0.001, yb, z_c],
        r,
    );
    let mid = (ya + yb) / 2.0;
    pane(
        b,
        sec,
        bars,
        xa + sw,
        xc - sw,
        z_a + sw,
        z_c - sw,
        mid,
        (yb - ya) * 0.7,
        inside,
    );
    // The handle, on the inside face of the sash where it locks.
    let face = y_in;
    let (hx, hz, up) = match kind {
        Op::Bottom | Op::Horizontal => ((xa + xc) / 2.0, z_c - sw / 2.0, false),
        Op::Top => ((xa + xc) / 2.0, z_a + sw / 2.0, false),
        _ => {
            let x = if hinge_left {
                xc - sw / 2.0
            } else {
                xa + sw / 2.0
            };
            (x, (z_a + z_c) / 2.0, true)
        }
    };
    window_handle(b, handle, hx, hz, face, inside, up, z_a, z_c);
    if sec.hinges {
        // Hinges on the inside where the sash turns.
        match kind {
            Op::Side | Op::TiltTurn => {
                let x = if hinge_left { xa } else { xc };
                for z in [z_a + 0.1, z_c - 0.1] {
                    b.cyl(
                        METAL,
                        [x, face + inside * 0.004, z - 0.035],
                        [x, face + inside * 0.004, z + 0.035],
                        0.0045,
                    );
                }
            }
            Op::Bottom => {
                for x in [xa + 0.1, xc - 0.1] {
                    b.cyl(
                        METAL,
                        [x - 0.035, face + inside * 0.004, z_a],
                        [x + 0.035, face + inside * 0.004, z_a],
                        0.0045,
                    );
                }
            }
            Op::Top => {
                for x in [xa + 0.1, xc - 0.1] {
                    b.cyl(
                        METAL,
                        [x - 0.035, face + inside * 0.004, z_c],
                        [x + 0.035, face + inside * 0.004, z_c],
                        0.0045,
                    );
                }
            }
            _ => {}
        }
    }
}

/// Glass from `xa` to `xc`, `za` to `zc`, centred on `y`, with its beads
/// on the inside face and its glazing bars.
#[allow(clippy::too_many_arguments)]
fn pane(
    b: &mut Builder,
    sec: &Section,
    bars: Bars,
    xa: f64,
    xc: f64,
    za: f64,
    zc: f64,
    y: f64,
    depth: f64,
    inside: f64,
) {
    if xc - xa < 0.03 || zc - za < 0.03 {
        return;
    }
    let o = 0.001;
    b.rbox(
        GLASS,
        [xa - o, y - 0.008, za - o],
        [xc + o, y + 0.008, zc + o],
        0.0,
    );
    if sec.plain {
        return;
    }
    // The beads that hold it, inside.
    let bead = if sec.moulded {
        Profile(vec![
            [0.0, 0.0],
            [0.0, 0.012],
            [0.004, 0.012],
            [0.008, 0.009],
            [0.012, 0.004],
            [0.014, 0.0],
        ])
    } else {
        Profile(vec![[0.0, 0.0], [0.0, 0.01], [0.012, 0.01], [0.012, 0.0]])
    };
    let path = [[xa, za], [xa, zc], [xc, zc], [xc, za]];
    door::moulding(b, FRAME, &path, true, y + inside * 0.008, inside, &bead);
    // The glazing bars, through the glass, as deep as the sash nearly.
    let m = if sec.sash < 0.04 { 0.014 } else { 0.022 };
    let d = (depth * 0.8).max(0.02);
    let (w, h) = (xc - xa, zc - za);
    let across = |b: &mut Builder, z: f64, x0: f64, x1: f64| {
        b.rbox(
            FRAME,
            [x0, y - d / 2.0, z - m / 2.0],
            [x1, y + d / 2.0, z + m / 2.0],
            0.0,
        );
    };
    let up = |b: &mut Builder, x: f64, z0: f64, z1: f64| {
        b.rbox(
            FRAME,
            [x - m / 2.0, y - d / 2.0, z0],
            [x + m / 2.0, y + d / 2.0, z1],
            0.0,
        );
    };
    let count = |len: f64, step: f64| ((len / step).round() as usize).max(1);
    match bars {
        Bars::None => {}
        Bars::Grid { w: sw, h: sh } => {
            let (nc, nr) = (count(w, sw), count(h, sh));
            for i in 1..nc {
                up(b, xa + w * i as f64 / nc as f64, za, zc);
            }
            for j in 1..nr {
                across(b, za + h * j as f64 / nr as f64, xa, xc);
            }
        }
        Bars::Across(step) => {
            let nr = count(h, step);
            for j in 1..nr {
                across(b, za + h * j as f64 / nr as f64, xa, xc);
            }
        }
        Bars::Up(step) => {
            let nc = count(w, step);
            for i in 1..nc {
                up(b, xa + w * i as f64 / nc as f64, za, zc);
            }
        }
        Bars::Top { w: sw, h: sh } => {
            let zb = zc - h / 3.0;
            across(b, zb, xa, xc);
            let (nc, nr) = (count(w, sw), count(h / 3.0, sh));
            for i in 1..nc {
                up(b, xa + w * i as f64 / nc as f64, zb, zc);
            }
            for j in 1..nr {
                across(b, zb + (zc - zb) * j as f64 / nr as f64, xa, xc);
            }
        }
    }
}

/// The handle of a sash at `x`, `z` on its inside face `face` (`up`: on a
/// stile, the lever hanging down; otherwise on a rail, lying across).
#[allow(clippy::too_many_arguments)]
fn window_handle(
    b: &mut Builder,
    kind: usize,
    x: f64,
    z: f64,
    face: f64,
    inside: f64,
    up: bool,
    z_a: f64,
    z_c: f64,
) {
    let at = |dx: f64, dy: f64, dz: f64| [x + dx, face + inside * dy, z + dz];
    // Which way the lever hangs: down on a stile, along on a rail.
    let along = |k: f64| if up { [0.0, 0.0, -k] } else { [k, 0.0, 0.0] };
    let plus = |p: [f64; 3], d: [f64; 3]| [p[0] + d[0], p[1] + d[1], p[2] + d[2]];
    match kind {
        6 => {}
        4 | 5 if up => {
            // A rod up the sash's stile in guides, the lever in its middle.
            let r = if kind == 4 { 0.006 } else { 0.0075 };
            let y = 0.012;
            b.cyl(
                METAL,
                at(0.0, y, z_a + 0.04 - z),
                at(0.0, y, z_c - 0.04 - z),
                r,
            );
            for zz in [z_a + 0.1, z_c - 0.1] {
                b.rbox(
                    METAL,
                    [x - 0.012, face.min(face + inside * (y + r)), zz - 0.012],
                    [x + 0.012, face.max(face + inside * (y + r)), zz + 0.012],
                    0.002,
                );
            }
            b.rbox(
                METAL,
                [x - 0.018, face.min(face + inside * 0.006), z - 0.04],
                [x + 0.018, face.max(face + inside * 0.006), z + 0.04],
                0.004,
            );
            let p0 = at(0.0, y + 0.004, 0.0);
            if kind == 4 {
                // A brass-ish lever, flat, down the rod.
                bar(
                    b,
                    METAL,
                    p0,
                    plus(p0, [0.0, inside * 0.03, -0.11]),
                    0.014,
                    [1.0, 0.0, 0.0],
                );
            } else {
                // A wrought lever ending in a ball.
                let end = plus(p0, [0.0, inside * 0.035, -0.12]);
                sweep(
                    b,
                    METAL,
                    &[p0, plus(p0, [0.0, inside * 0.03, -0.04]), end],
                    |_| 0.006,
                    10,
                    Cap::Flat,
                    Cap::Flat,
                );
                b.ellipsoid(METAL, end, [0.012, 0.012, 0.012]);
            }
        }
        _ => {
            // A lever on its rose: modern square or brass curved.
            let brass = kind == 3;
            b.rbox(
                METAL,
                [x - 0.017, face.min(face + inside * 0.01), z - 0.035],
                [x + 0.017, face.max(face + inside * 0.01), z + 0.035],
                if brass { 0.012 } else { 0.003 },
            );
            let p0 = at(0.0, 0.01, if up { 0.012 } else { 0.0 });
            let p1 = at(0.0, 0.045, if up { 0.012 } else { 0.0 });
            b.cyl(METAL, p0, p1, 0.007);
            let end = plus(p1, along(0.11));
            if brass {
                let mid = plus(plus(p1, along(0.055)), [0.0, inside * 0.008, 0.0]);
                sweep(
                    b,
                    METAL,
                    &[p1, mid, end],
                    |s| 0.0075 - 0.002 * s,
                    10,
                    Cap::Flat,
                    Cap::Round(0.8),
                );
            } else {
                let side = if up { [1.0, 0.0, 0.0] } else { [0.0, 0.0, 1.0] };
                bar(b, METAL, p1, end, 0.014, side);
            }
        }
    }
}

/// The roller blind of a window with a box (the family's «Persiana»): its
/// guides down the outside of the reveal and the slats let down from the
/// box (`shutter_down`, a percentage), the bottom slat always.
fn shutters_blind(b: &mut Builder, p: &Params, fd: f64, outside: f64, z0: f64) {
    if !p.flag("shutter", false) {
        return;
    }
    let width = p.get("w_width", 1.2);
    let top = p.get("w_top", 1.0);
    let down = (p.get("shutter_down", 0.0) / 100.0).clamp(0.0, 1.0);
    let (ga, gb) = sorted(outside * (fd / 2.0 + 0.004), outside * (fd / 2.0 + 0.04));
    let g = 0.022;
    for (x0, x1) in [
        (-width / 2.0, -width / 2.0 + g),
        (width / 2.0 - g, width / 2.0),
    ] {
        b.rbox(FRAME, [x0, ga, z0], [x1, gb, top], 0.001);
    }
    let yc = (ga + gb) / 2.0;
    let bottom = top - down * (top - z0);
    let (xa, xc) = (-width / 2.0 + g * 0.5, width / 2.0 - g * 0.5);
    let pitch = 0.045;
    if p.flag("lod", false) {
        // The slats let down as one sheet.
        let zb = bottom.min(top - 0.001).max(z0 + 0.05) - 0.05;
        b.rbox(FRAME, [xa, yc - 0.006, zb], [xc, yc + 0.006, top], 0.0);
        return;
    }
    let n = ((top - bottom) / pitch).floor() as usize;
    for k in 0..n {
        let zt = top - k as f64 * pitch;
        b.rbox(
            FRAME,
            [xa, yc - 0.006, zt - pitch + 0.003],
            [xc, yc + 0.006, zt],
            0.004,
        );
    }
    // The bottom slat, stiffer.
    // It rests on the sill at most.
    let zb = bottom.min(top - 0.001).max(z0 + 0.05);
    b.rbox(
        FRAME,
        [xa, yc - 0.009, zb - 0.05],
        [xc, yc + 0.009, zb],
        0.004,
    );
}

/// A pair of shutters on the façade (`outside_shutters`): louvred,
/// boarded or panelled, each half the opening wide, open back against the
/// wall beside it (`shutters_open`) or closed over it.
fn outside_shutters(b: &mut Builder, p: &Params, outside: f64) {
    let kind = p.get("outside_shutters", 1.0).round() as usize;
    if kind < 2 {
        return;
    }
    let width = p.get("w_width", 1.2);
    let top = p.get("w_top", 1.2);
    let face = p.get("face_ext", outside * 0.15);
    let open = p.flag("shutters_open", true);
    let t = 0.03;
    let design = match kind {
        2 => LeafDesign::Framed {
            stile: 0.06,
            rail: 0.06,
            bottom: 0.09,
            rows: vec![
                Row {
                    weight: 1.0,
                    fields: vec![Field::Louvres],
                },
                Row {
                    weight: 1.0,
                    fields: vec![Field::Louvres],
                },
            ],
        },
        3 => LeafDesign::Planks {
            boards: ((width / 2.0) / 0.11).round().max(2.0) as usize,
        },
        _ => LeafDesign::Framed {
            stile: 0.07,
            rail: 0.07,
            bottom: 0.1,
            rows: vec![
                Row {
                    weight: 1.0,
                    fields: vec![Field::Raised],
                },
                Row {
                    weight: 1.0,
                    fields: vec![Field::Raised],
                },
            ],
        },
    };
    let half = width / 2.0;
    // Their plane, just off the wall.
    let y = face + outside * (t / 2.0 + 0.004);
    for side in [-1.0, 1.0] {
        let hinge = side * half;
        // Closed it runs from its hinge to the middle; open, from its hinge
        // out along the wall.
        let (a, c) = if open {
            sorted(hinge, hinge + side * (half - 0.003))
        } else {
            sorted(hinge, hinge - side * (half - 0.003))
        };
        if p.flag("lod", false) {
            let (ya, yb) = sorted(y - t / 2.0, y + t / 2.0);
            b.rbox(SHUTTERS, [a, ya, 0.0], [c, yb, top], 0.0);
            continue;
        }
        let mut sb = Builder::default();
        door::leaf(&mut sb, &design, a, c, 0.0, top, t);
        // The leaf's back (its ledges and brace) against the wall: its own
        // -y towards the wall, turned over when that is +y.
        sb.transform_from(0, |q| [q[0], y + outside * q[1], q[2]]);
        if outside < 0.0 {
            for tris in sb.parts.values_mut() {
                for t in tris.iter_mut() {
                    t.swap(1, 2);
                }
            }
        }
        merge(b, sb, |_| SHUTTERS);
        // The hinges, on the wall's face.
        for z in [0.18, top - 0.18] {
            b.cyl(
                METAL,
                [hinge, face + outside * 0.001, z - 0.04],
                [hinge, face + outside * 0.001, z + 0.04],
                0.007,
            );
            let reach = hinge + side * 0.11 * if open { 1.0 } else { -1.0 };
            let (lo, hi) = sorted3(
                [hinge - side * 0.002, face, z - 0.012],
                [reach, y + outside * (t / 2.0 + 0.003), z + 0.012],
            );
            b.rbox(METAL, lo, hi, 0.001);
        }
    }
}

fn sorted3(a: [f64; 3], c: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    (
        [a[0].min(c[0]), a[1].min(c[1]), a[2].min(c[2])],
        [a[0].max(c[0]), a[1].max(c[1]), a[2].max(c[2])],
    )
}

/// A grille outside the window (`grille`): flat on the façade, or a box
/// standing `grille_depth` out of it (`grille_box`), round the opening.
fn grille_outside(b: &mut Builder, p: &Params, outside: f64) {
    let g = grille(p.get("grille", 1.0).round() as usize);
    let step = match g {
        Grille::None => return,
        Grille::Bars { step }
        | Grille::Flats { step, .. }
        | Grille::Grid { step }
        | Grille::Scrolls { step } => step,
    };
    let width = p.get("w_width", 1.2);
    let height = p.get("w_height", 1.2);
    let face = p.get("face_ext", outside * 0.15);
    let boxed = p.flag("grille_box", false);
    let depth = if boxed {
        p.get("grille_depth", 0.18).clamp(0.08, 0.6)
    } else {
        0.03
    };
    // Round the opening, a little past it.
    let m = 0.06;
    let (x0, x1) = (-width / 2.0 - m, width / 2.0 + m);
    let (z0, z1) = (-m, height + m);
    let front = face + outside * depth;
    let flat = 0.04;
    let ft = 0.008;
    let r = 0.008;
    // The frame of flats round the front.
    let ring = |b: &mut Builder, y: f64| {
        let (ya, yb) = sorted(y - ft / 2.0, y + ft / 2.0);
        b.rbox(GRILLE, [x0, ya, z0], [x0 + flat, yb, z1], 0.001);
        b.rbox(GRILLE, [x1 - flat, ya, z0], [x1, yb, z1], 0.001);
        b.rbox(GRILLE, [x0, ya, z0], [x1, yb, z0 + flat], 0.001);
        b.rbox(GRILLE, [x0, ya, z1 - flat], [x1, yb, z1], 0.001);
    };
    ring(b, front);
    let n = (((x1 - x0) / step).round() as usize).max(2);
    let xs: Vec<f64> = (1..n)
        .map(|i| x0 + (x1 - x0) * i as f64 / n as f64)
        .collect();
    if p.flag("lod", false) {
        // Square bars, the box's corners back to the wall, nothing more.
        let (ya, yb) = sorted(front - r, front + r);
        for &x in &xs {
            b.rbox(GRILLE, [x - r, ya, z0], [x + r, yb, z1], 0.0);
        }
        let back = face + outside * 0.004;
        for x in [x0 + flat / 2.0, x1 - flat / 2.0] {
            for z in [z0 + flat / 2.0, z1 - flat / 2.0] {
                let (lo, hi) = sorted3(
                    [x - flat / 2.0, back, z - ft / 2.0],
                    [x + flat / 2.0, front, z + ft / 2.0],
                );
                b.rbox(GRILLE, lo, hi, 0.0);
            }
        }
        return;
    }
    for &x in &xs {
        b.cyl(
            GRILLE,
            [x, front, z0 + flat / 2.0],
            [x, front, z1 - flat / 2.0],
            r,
        );
    }
    match g {
        Grille::Flats { flats, .. } => {
            for k in 1..=flats {
                let z = z0 + (z1 - z0) * k as f64 / (flats + 1) as f64;
                let (ya, yb) = sorted(front - ft / 2.0 - 0.004, front + ft / 2.0 + 0.004);
                b.rbox(
                    GRILLE,
                    [x0, ya, z - flat / 2.0],
                    [x1, yb, z + flat / 2.0],
                    0.001,
                );
            }
        }
        Grille::Grid { step } => {
            let nz = (((z1 - z0) / step).round() as usize).max(2);
            for j in 1..nz {
                let z = z0 + (z1 - z0) * j as f64 / nz as f64;
                b.cyl(
                    GRILLE,
                    [x0 + flat / 2.0, front, z],
                    [x1 - flat / 2.0, front, z],
                    r * 0.9,
                );
            }
        }
        Grille::Scrolls { .. } => {
            // A band of scrolls between the bars near the top and the
            // bottom, each a C turned towards the next.
            let band = 0.16;
            for (zb, flip) in [(z0 + flat, false), (z1 - flat - band, true)] {
                let (ya, yb) = sorted(front - ft / 2.0, front + ft / 2.0);
                let zf = if flip { zb } else { zb + band };
                b.rbox(GRILLE, [x0, ya, zf - ft], [x1, yb, zf + ft], 0.001);
                let mut edges = vec![x0 + flat];
                edges.extend(xs.iter().copied());
                edges.push(x1 - flat);
                for pair in edges.windows(2) {
                    let (xa, xc) = (pair[0], pair[1]);
                    let cx = (xa + xc) / 2.0;
                    let cz = zb + band / 2.0;
                    let rad = ((xc - xa) / 2.0 - r).min(band / 2.0 - 0.01).max(0.02);
                    for s in [-1.0, 1.0] {
                        scroll(b, [cx, front, cz], rad, s, flip);
                    }
                }
            }
            // A crest of scrolls on top, on a flat face only.
            if !boxed {
                let mut x = x0 + 0.09;
                while x < x1 - 0.08 {
                    let xm = x + 0.045;
                    b.cyl(
                        GRILLE,
                        [xm, front, z1 - flat / 2.0],
                        [xm, front, z1 + 0.09],
                        r * 0.8,
                    );
                    scroll(b, [x, front, z1 + 0.05], 0.045, 1.0, false);
                    scroll(b, [x + 0.09, front, z1 + 0.05], 0.045, -1.0, false);
                    x += 0.18;
                }
            }
        }
        _ => {}
    }
    if boxed {
        // The box: its sides, top and bottom back to the wall, each with
        // bars, and tied to it.
        let back = face + outside * 0.004;
        for x in [x0 + flat / 2.0, x1 - flat / 2.0] {
            for z in [z0 + flat / 2.0, z1 - flat / 2.0] {
                let (lo, hi) = sorted3(
                    [x - flat / 2.0, back, z - ft / 2.0],
                    [x + flat / 2.0, front, z + ft / 2.0],
                );
                b.rbox(GRILLE, lo, hi, 0.001);
            }
        }
        // Bars across the top and the bottom, from the wall to the front.
        for &x in &xs {
            for z in [z0 + flat / 2.0, z1 - flat / 2.0] {
                b.cyl(GRILLE, [x, back, z], [x, front, z], r * 0.9);
            }
        }
        // Upright bars on the sides.
        let ns = ((depth / step).round() as usize).max(1);
        for k in 1..=ns {
            let y = back + (front - back) * k as f64 / (ns + 1) as f64;
            for x in [x0 + flat / 2.0, x1 - flat / 2.0] {
                b.cyl(GRILLE, [x, y, z0 + flat / 2.0], [x, y, z1 - flat / 2.0], r);
            }
        }
    } else {
        // Ties into the wall at its corners and sides.
        for x in [x0 + flat / 2.0, x1 - flat / 2.0] {
            for z in [z0 + 0.12, (z0 + z1) / 2.0, z1 - 0.12] {
                let (ya, yb) = sorted(face - outside * 0.04, front);
                b.rbox(
                    GRILLE,
                    [x - 0.012, ya, z - 0.004],
                    [x + 0.012, yb, z + 0.004],
                    0.001,
                );
            }
        }
    }
}

/// A C scroll of radius `rad` in the grille's plane at `c` (`y` its
/// plane), its open side towards `s` (±x), curling down (`flip`) or up.
fn scroll(b: &mut Builder, c: [f64; 3], rad: f64, s: f64, flip: bool) {
    let turns = 1.4;
    let k = 26;
    let path: Vec<[f64; 3]> = (0..=k)
        .map(|i| {
            let t = i as f64 / k as f64;
            let a = t * turns * std::f64::consts::TAU;
            let rr = rad * (1.0 - 0.75 * t);
            let dz = if flip { -1.0 } else { 1.0 };
            [c[0] + s * rr * a.cos(), c[1], c[2] + dz * rr * a.sin()]
        })
        .collect();
    sweep(b, GRILLE, &path, |_| 0.0045, 6, Cap::Flat, Cap::Flat);
}

/// Adds another model's triangles, each part to the role `role` gives.
fn merge(b: &mut Builder, other: Builder, role: impl Fn(&str) -> &'static str) {
    let off = b.positions.len() as u32;
    b.positions.extend(other.positions);
    for (r, tris) in other.parts {
        b.parts
            .entry(role(r))
            .or_default()
            .extend(tris.into_iter().map(|t| t.map(|i| i + off)));
    }
}

/// Mirrors the whole model across `x = 0`, keeping its faces turned out.
fn mirror_x(b: &mut Builder) {
    for q in &mut b.positions {
        q[0] = -q[0];
    }
    for tris in b.parts.values_mut() {
        for t in tris.iter_mut() {
            t.swap(1, 2);
        }
    }
}

/// The profiles of a sliding window, by name (`slider_profile`).
pub const SLIDER_PROFILES: &[&str] = &[
    "Como la colección",
    "Aluminio estándar",
    "Aluminio con rotura de puente térmico",
    "PVC",
    "Madera",
    "Acero fino",
    "Minimalista de perfil oculto",
];

/// The handles of a sliding window, by name (`slider_handle`).
pub const SLIDER_HANDLES: &[&str] = &[
    "Como la colección",
    "Uñero",
    "Manilla embutida",
    "Maneta de cierre",
    "Tirador de barra",
    "Sin tirador",
];

/// How a sliding profile is made: how wide its sash shows, how wide the
/// frame shows (the minimalist one hides it in the wall), how rounded.
#[derive(Debug, Clone, Copy)]
struct SliderSection {
    sash: f64,
    frame: f64,
    round: f64,
}

fn slider_section(n: usize) -> SliderSection {
    let s = |sash, frame, round| SliderSection { sash, frame, round };
    match n {
        3 => s(0.07, 0.06, 0.004),
        4 => s(0.075, 0.065, 0.008),
        5 => s(0.08, 0.07, 0.002),
        6 => s(0.035, 0.035, 0.0005),
        7 => s(0.022, 0.012, 0.0005),
        _ => s(0.055, 0.05, 0.001),
    }
}

/// How each collection makes a sliding window: profile, handle and bars
/// (numbers in [`SLIDER_PROFILES`], [`SLIDER_HANDLES`] and [`BARS`]).
const SLIDER_LOOKS: [(usize, usize, usize); 8] = [
    (5, 2, 2), // Nórdico cálido: timber, a recessed pull.
    (4, 4, 3), // Mediterráneo: PVC, a lever, a grid.
    (6, 3, 5), // Años 50: slim steel, a flush handle, bars across.
    (5, 2, 6), // Japandi: timber, a recessed pull, bars up.
    (7, 2, 2), // Contemporáneo: minimalist, a recessed pull.
    (5, 4, 3), // Rústica: timber, a lever, a grid.
    (6, 5, 4), // Industrial: slim steel, a pull bar, a fine grid.
    (3, 4, 7), // Clásica: thermally broken aluminium, a lever, top panes.
];

/// A sliding window or balcony door (paquete 9, correderas): the frame
/// with its tracks, the sashes on alternate tracks overlapping where they
/// meet, their glass, bars and handles; an insect screen on a track of its
/// own outside (`screen`), and the blind, shutters and grille of any
/// window. The family's outputs: `w_fw` (the frame), `w_w`, `w_z0`,
/// `w_z1`, `leaves`, `w_outside`, and those of what goes outside.
pub fn slider(b: &mut Builder, p: &Params) {
    if outside_only(b, p) {
        return;
    }
    let style = (p.style() as usize).clamp(1, 8);
    let (mut prof, mut hand, mut bar_kind) = SLIDER_LOOKS[style - 1];
    let pick = |key: &str| {
        let n = p.get(key, 1.0).round() as usize;
        (n >= 2).then_some(n)
    };
    if let Some(n) = pick("slider_profile") {
        prof = n;
    }
    if let Some(n) = pick("slider_handle") {
        hand = n;
    }
    if let Some(n) = pick("glazing_bars") {
        bar_kind = n;
    }
    let far = p.flag("lod", false);
    let sec = slider_section(prof);
    let r = if far { 0.0 } else { sec.round };
    let fd = p.get("frame_depth", 0.07);
    let w = p.get("w_w", 1.8);
    let z0 = p.get("w_z0", 0.0);
    let z1 = p.get("w_z1", 1.3);
    let fw = sec.frame.min(p.get("w_fw", 0.06)).max(0.01);
    let outside = if p.get("w_outside", -1.0) > 0.0 {
        1.0
    } else {
        -1.0
    };
    let inside = -outside;
    let leaves = p.get("leaves", 2.0).round().clamp(2.0, 4.0) as usize;
    let screen = p.flag("screen", false);
    // The frame round the opening, its depth split into tracks: two for
    // the sashes, and one more outside for the screen.
    let (ya, yb) = (-fd / 2.0, fd / 2.0);
    b.rbox(FRAME, [-w / 2.0, ya, z0], [-w / 2.0 + fw, yb, z1], r);
    b.rbox(FRAME, [w / 2.0 - fw, ya, z0], [w / 2.0, yb, z1], r);
    b.rbox(
        FRAME,
        [-w / 2.0 + fw - 0.001, ya, z0],
        [w / 2.0 - fw + 0.001, yb, z0 + fw],
        r,
    );
    b.rbox(
        FRAME,
        [-w / 2.0 + fw - 0.001, ya, z1 - fw],
        [w / 2.0 - fw + 0.001, yb, z1],
        r,
    );
    let tracks = if screen { 3.0 } else { 2.0 };
    let step = fd / tracks;
    // Track `k` (0 the outermost) and its middle.
    let track = |k: f64| outside * (fd / 2.0 - step * (k + 0.5));
    let rail_h = 0.012;
    for k in 0..tracks as usize {
        let y = track(k as f64);
        b.rbox(
            FRAME,
            [-w / 2.0 + fw, y - 0.003, z0 + fw - 0.001],
            [w / 2.0 - fw, y + 0.003, z0 + fw + rail_h],
            0.0,
        );
    }
    let (c0, c1) = (-w / 2.0 + fw, w / 2.0 - fw);
    let (za, zc) = (z0 + fw + rail_h, z1 - fw - 0.004);
    let lap = sec.sash.max(0.03);
    let lw = (c1 - c0 + (leaves as f64 - 1.0) * lap) / leaves as f64;
    let bars = if far { Bars::None } else { bars(bar_kind) };
    let half = (step / 2.0 - 0.003).max(0.008);
    let off = if screen { 1.0 } else { 0.0 };
    for i in 0..leaves {
        let a = c0 + i as f64 * (lw - lap);
        let c = a + lw;
        // Alternate tracks: the end ones outside, the middle ones inside.
        let k = if i == 0 || i + 1 == leaves { 0.0 } else { 1.0 };
        let k = if leaves == 2 { i as f64 } else { k };
        let y = track(k + off);
        let (sa, sb) = (y - half, y + half);
        let sw = sec.sash.min((c - a) / 3.0);
        b.rbox(FRAME, [a, sa, za], [a + sw, sb, zc], r);
        b.rbox(FRAME, [c - sw, sa, za], [c, sb, zc], r);
        b.rbox(
            FRAME,
            [a + sw - 0.001, sa, za],
            [c - sw + 0.001, sb, za + sw],
            r,
        );
        b.rbox(
            FRAME,
            [a + sw - 0.001, sa, zc - sw],
            [c - sw + 0.001, sb, zc],
            r,
        );
        let psec = Section {
            sash: sw,
            depth: 2.0 * half,
            round: r,
            proud: 0.0,
            moulded: false,
            hinges: false,
            plain: far,
        };
        pane(
            b,
            &psec,
            bars,
            a + sw,
            c - sw,
            za + sw,
            zc - sw,
            y,
            2.0 * half * 0.7,
            inside,
        );
        if far {
            continue;
        }
        // Its handle on the stile by the nearer jamb (the middle sashes of
        // four, on the stile by the middle), on its inside face.
        let near_left = if leaves == 4 && (i == 1 || i == 2) {
            i == 2
        } else {
            (a + c) / 2.0 < 0.0
        };
        let x = if near_left {
            a + sw / 2.0
        } else {
            c - sw / 2.0
        };
        let face = y + inside * half;
        slider_handle(b, hand, x, (za + zc) / 2.0, face, inside, sw);
    }
    if screen && !far {
        // The insect screen: a light frame on the outer track, half the
        // width, its mesh (as a fine glass).
        let y = track(0.0);
        let (sa, sb) = (y - half * 0.6, y + half * 0.6);
        let (a, c) = (c0, c0 + (c1 - c0) / 2.0 + 0.02);
        let sw = 0.025;
        b.rbox(FRAME, [a, sa, za], [a + sw, sb, zc], 0.0);
        b.rbox(FRAME, [c - sw, sa, za], [c, sb, zc], 0.0);
        b.rbox(FRAME, [a + sw, sa, za], [c - sw, sb, za + sw], 0.0);
        b.rbox(FRAME, [a + sw, sa, zc - sw], [c - sw, sb, zc], 0.0);
        b.rbox(
            GLASS,
            [a + sw - 0.0005, y - 0.0008, za + sw - 0.0005],
            [c - sw + 0.0005, y + 0.0008, zc - sw + 0.0005],
            0.0,
        );
    }
    let fd_out = fd;
    shutters_blind(b, p, fd_out, outside, z0);
    outside_shutters(b, p, outside);
    grille_outside(b, p, outside);
    if p.flag("mirror", false) {
        mirror_x(b);
    }
}

/// Only what goes outside (`only`: 1 the grille, 2 the shutters), as an
/// element of its own; `true` if that was asked.
fn outside_only(b: &mut Builder, p: &Params) -> bool {
    let only = p.get("only", 0.0).round() as i32;
    if only <= 0 {
        return false;
    }
    let outside = if p.get("w_outside", -1.0) > 0.0 {
        1.0
    } else {
        -1.0
    };
    if only == 1 {
        grille_outside(b, p, outside);
    } else {
        outside_shutters(b, p, outside);
    }
    if p.flag("mirror", false) {
        mirror_x(b);
    }
    true
}

/// The handle of a sliding sash at `x`, `z` on its inside face `face`:
/// a recessed pull, a flush handle, a lever lock or a pull bar.
fn slider_handle(b: &mut Builder, kind: usize, x: f64, z: f64, face: f64, inside: f64, sw: f64) {
    let (ya, yb) = sorted(face - inside * 0.0005, face + inside * 0.004);
    match kind {
        6 => {}
        2 => {
            // A recessed pull: a plate with its dark hollow.
            let wx = (sw * 0.45).min(0.02);
            b.rbox(METAL, [x - wx, ya, z - 0.07], [x + wx, yb, z + 0.07], 0.006);
        }
        3 => {
            // A flush handle: a long plate, a short lever lying in it.
            let wx = (sw * 0.4).min(0.018);
            b.rbox(METAL, [x - wx, ya, z - 0.09], [x + wx, yb, z + 0.09], 0.004);
            let (la, lb) = sorted(face, face + inside * 0.012);
            b.rbox(
                METAL,
                [x - wx * 0.6, la, z - 0.05],
                [x + wx * 0.6, lb, z + 0.03],
                0.004,
            );
        }
        5 => {
            // A pull bar up the stile, on two posts.
            for zz in [z - 0.25, z + 0.25] {
                b.cyl(
                    METAL,
                    [x, face - inside * 0.001, zz],
                    [x, face + inside * 0.05, zz],
                    0.008,
                );
            }
            b.cyl(
                METAL,
                [x, face + inside * 0.05, z - 0.27],
                [x, face + inside * 0.05, z + 0.27],
                0.011,
            );
        }
        _ => {
            // A lever lock: its base and the lever turned down.
            let wx = (sw * 0.45).min(0.02);
            b.rbox(METAL, [x - wx, ya, z - 0.05], [x + wx, yb, z + 0.05], 0.004);
            let p0 = [x, face + inside * 0.004, z + 0.02];
            let p1 = [x, face + inside * 0.03, z + 0.02];
            b.cyl(METAL, p0, p1, 0.006);
            bar(
                b,
                METAL,
                p1,
                [x, face + inside * 0.03, z - 0.07],
                0.012,
                [1.0, 0.0, 0.0],
            );
        }
    }
}

/// A sash window (guillotina, paquete 9): the frame, the two sashes on
/// their two planes — the lower towards the side it opens to, raised
/// `sash_open` per cent of what it can — their glass and bars, the
/// crescent lock where they meet, the lifts on the lower sash and, on a
/// timber one, the horns of the upper sash. The family's outputs: `w_fw`,
/// `w_w`, `w_z1`, `w_zm` (where the sashes meet), `w_outside`, and the
/// collection's profile, bars and handle as the casement window's.
pub fn sash_window(b: &mut Builder, p: &Params) {
    if outside_only(b, p) {
        return;
    }
    let l = look(p);
    let far = p.flag("lod", false);
    let mut sec = section(l.profile);
    if far {
        sec.round = 0.0;
        sec.moulded = false;
        sec.plain = true;
    }
    let r = sec.round;
    let fw = p.get("w_fw", 0.06);
    let fd = p.get("frame_depth", 0.1);
    let w = p.get("w_w", 1.0);
    let z1 = p.get("w_z1", 1.5);
    let zm = p.get("w_zm", z1 / 2.0).clamp(0.3, z1 - 0.3);
    let outside = if p.get("w_outside", -1.0) > 0.0 {
        1.0
    } else {
        -1.0
    };
    let inside = -outside;
    let (ya, yb) = (-fd / 2.0, fd / 2.0);
    b.rbox(FRAME, [-w / 2.0, ya, 0.0], [-w / 2.0 + fw, yb, z1], r);
    b.rbox(FRAME, [w / 2.0 - fw, ya, 0.0], [w / 2.0, yb, z1], r);
    b.rbox(
        FRAME,
        [-w / 2.0 + fw - 0.001, ya, 0.0],
        [w / 2.0 - fw + 0.001, yb, fw],
        r,
    );
    b.rbox(
        FRAME,
        [-w / 2.0 + fw - 0.001, ya, z1 - fw],
        [w / 2.0 - fw + 0.001, yb, z1],
        r,
    );
    // The parting bead between the two planes, up the jambs.
    let (c0, c1) = (-w / 2.0 + fw, w / 2.0 - fw);
    for x in [c0, c1 - 0.012] {
        b.rbox(
            FRAME,
            [x, -0.006, fw - 0.001],
            [x + 0.012, 0.006, z1 - fw + 0.001],
            0.0,
        );
    }
    let q = fd / 4.0;
    let half = (q - 0.004).max(0.01);
    let sw = sec.sash.min((c1 - c0) / 4.0);
    let rail = sw * 0.8;
    let lift = (p.get("sash_open", 0.0) / 100.0).clamp(0.0, 1.0) * (zm - fw - rail);
    let bars = if far { Bars::None } else { bars(l.bars) };
    // (plane, bottom, top): the upper sash outside, the lower inside.
    let lower = (inside * q, fw + lift, zm + rail / 2.0 + lift);
    let upper = (outside * q, zm - rail / 2.0, z1 - fw);
    for (y, za, zc) in [upper, lower] {
        let (sa, sbb) = (y - half, y + half);
        b.rbox(FRAME, [c0, sa, za], [c0 + sw, sbb, zc], r);
        b.rbox(FRAME, [c1 - sw, sa, za], [c1, sbb, zc], r);
        b.rbox(
            FRAME,
            [c0 + sw - 0.001, sa, za],
            [c1 - sw + 0.001, sbb, za + rail],
            r,
        );
        b.rbox(
            FRAME,
            [c0 + sw - 0.001, sa, zc - rail],
            [c1 - sw + 0.001, sbb, zc],
            r,
        );
        let psec = Section {
            sash: sw,
            depth: 2.0 * half,
            round: r,
            proud: 0.0,
            moulded: sec.moulded,
            hinges: false,
            plain: far,
        };
        pane(
            b,
            &psec,
            bars,
            c0 + sw,
            c1 - sw,
            za + rail,
            zc - rail,
            y,
            2.0 * half * 0.7,
            inside,
        );
    }
    if !far {
        let (yl, zlb, zlt) = lower;
        let face = yl + inside * half;
        // The crescent lock on the lower sash's top rail, in the middle.
        let (fa, fb) = sorted(face - inside * 0.0005, face + inside * 0.006);
        b.rbox(
            METAL,
            [-0.03, fa, zlt - rail * 0.8],
            [0.03, fb, zlt - rail * 0.2],
            0.004,
        );
        if l.handle != 6 {
            // Two lifts on its bottom rail.
            for x in [c0 + (c1 - c0) * 0.25, c0 + (c1 - c0) * 0.75] {
                let (ga, gb) = sorted(face - inside * 0.0005, face + inside * 0.022);
                b.rbox(
                    METAL,
                    [x - 0.03, ga, zlb + rail * 0.3],
                    [x + 0.03, gb, zlb + rail * 0.7],
                    0.005,
                );
            }
        }
        if l.profile >= 5 && l.profile <= 6 {
            // The horns: the upper sash's stiles run on below its rail.
            let (yu, zub, _) = upper;
            for (x0, x1) in [(c0, c0 + sw), (c1 - sw, c1)] {
                b.rbox(
                    FRAME,
                    [x0, yu - half, zub - 0.05],
                    [x1, yu + half, zub + 0.001],
                    0.006,
                );
            }
        }
    }
    shutters_blind(b, p, fd, outside, 0.0);
    outside_shutters(b, p, outside);
    grille_outside(b, p, outside);
    if p.flag("mirror", false) {
        mirror_x(b);
    }
}
