//! Doors made with care (paquete 9, carpintería): the leaves with their
//! stiles, rails and fields (raised or sunk panels, glass with or without
//! glazing bars, louvres, planks, grooves, applied mouldings), the stop
//! the leaf closes against, the casings round the opening on the faces
//! of the wall and the lining across its reveal, and the hardware —
//! levers on roses or on plates, knobs, pull bars, hinges, a spyhole.
//!
//! The door family keeps the frame, the transom and the sidelights in its
//! recipe (they are what its IFC and its plan say); with `detailed` on, it
//! leaves the leaves out and tells this generator where they go (its
//! outputs `leaf_x0`, `leaf_x1`, `leaf_split`, `leaf_top`, `leaf_t`,
//! `lining_w`, `lining_h`), and the program adds where the wall's faces
//! are (`face_pos`, `face_neg`) and which of them take a casing
//! (`casing_pos`, `casing_neg`).
//!
//! Axes as the family's: `x` along the wall from the centre of the
//! opening, `y` across it from the middle of the frame towards the side
//! the leaves open to, `z` up from the floor.
//!
//! Everything a door is made of is data — [`LeafDesign`] (a frame and its
//! fields, row by row) and [`Profile`] (the section of a casing or a
//! moulding) — so that a workshop where one draws one's own leaf, its
//! mouldings or a window's grille can make them the same way (see
//! `docs/CARPINTERIA.md`).
//!
//! Roles are the family's material parameters: `frame_material` (lining,
//! stop, casings), `panel_material` (the leaves), `glass_material` and
//! `hardware_material`.

use super::shape::{bar, sweep, turned, Cap};
use crate::build::{cross, Builder, P3};
use crate::Params;

const FRAME: &str = "frame_material";
const LEAF: &str = "panel_material";
const GLASS: &str = "glass_material";
const METAL: &str = "hardware_material";

/// The collections, in the order of the family's «Modelo 3D».
pub const COLLECTIONS: [&str; 8] = [
    "Nórdico cálido",
    "Mediterráneo",
    "Años 50",
    "Japandi",
    "Contemporáneo",
    "Rústica",
    "Industrial",
    "Clásica",
];

/// What fills a field of a leaf.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Field {
    /// A flat panel sunk between the stiles and rails (shaker).
    Sunk,
    /// A raised and bevelled panel, with a moulding round it.
    Raised,
    /// Glass, held by beads.
    Glass,
    /// Glass divided by glazing bars: columns and rows.
    Bars(u8, u8),
    /// Slats tilted between the stiles.
    Louvres,
}

/// A row of fields across a leaf, `weight` its share of the height.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub weight: f64,
    pub fields: Vec<Field>,
}

/// A vision panel through a leaf (hospitals, kitchens, workshops, fire
/// doors): its glass and its centre `z` above the leaf's bottom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Vision {
    /// A square or oblong pane, `w` by `h`.
    Square { w: f64, h: f64, z: f64 },
    /// A round porthole of diameter `d`.
    Round { d: f64, z: f64 },
}

/// How a leaf is made.
#[derive(Debug, Clone, PartialEq)]
pub enum LeafDesign {
    /// A flush board: plain, with `grooves` across (horizontal) or up
    /// (vertical), or with applied moulded frames (`moulded` rows of
    /// rectangles of one or two columns).
    Flush {
        grooves: usize,
        vertical: bool,
        moulded: Option<Vec<Row>>,
        vision: Option<Vision>,
    },
    /// A steel leaf: two sheets folded round a core, its edge showing as a
    /// rim, plain or with `embossed` panels pressed into it (two, one over
    /// the other), with or without a vision panel. `armoured`: a security
    /// door, thicker, with its sheets on a frame of bolts.
    Sheet {
        embossed: bool,
        vision: Option<Vision>,
        armoured: bool,
    },
    /// Upright boards, tongued and grooved, on ledges behind.
    Planks { boards: usize },
    /// A frame of stiles and rails round its fields, row by row from the
    /// top; `stile` and `rail` their widths, `bottom` the bottom rail's.
    Framed {
        stile: f64,
        rail: f64,
        bottom: f64,
        rows: Vec<Row>,
    },
}

/// The leaves the family offers, by name (the first, «as the
/// collection»).
pub const DESIGNS: &[&str] = &[
    "Como la colección",
    "Lisa",
    "Ranurada",
    "Ranurada vertical",
    "Dos cuarterones",
    "Cuatro cuarterones",
    "Seis cuarterones",
    "Plafonada",
    "Moldura aplicada",
    "Vidriera",
    "Media vidriera",
    "Vidriera con junquillos",
    "Lamas",
    "Tablas",
    "Chapa lisa",
    "Chapa embutida",
    "Chapa con ventanilla cuadrada",
    "Chapa con ojo de buey",
    "Chapa con mirilla alargada",
    "Lisa con ventanilla cuadrada",
    "Lisa con ojo de buey",
    "Acorazada",
    "Retícula a medida",
];

/// The sides a door's handle is on (`handle_sides`), by name.
pub const HANDLE_SIDES: &[&str] = &[
    "A las dos caras",
    "Solo del lado al que abre",
    "Solo del otro lado",
];

/// The faces of a door's handle from `handle_sides` (+1: the side it
/// opens to).
fn handle_faces(p: &Params) -> &'static [f64] {
    match p.get("handle_sides", 1.0).round() as i32 {
        2 => &[1.0],
        3 => &[-1.0],
        _ => &[1.0, -1.0],
    }
}

/// The design of a leaf by its number in [`DESIGNS`], with its values
/// (a grid to measure: `grid_cols` by `grid_rows` panes of glass).
pub fn design_of(n: usize, p: &Params) -> LeafDesign {
    if n == 23 {
        let cols = p.get("grid_cols", 2.0).round().clamp(1.0, 8.0) as u8;
        let rows = p.get("grid_rows", 4.0).round().clamp(1.0, 12.0) as u8;
        return LeafDesign::Framed {
            stile: 0.11,
            rail: 0.11,
            bottom: 0.2,
            rows: vec![row(1.0, &[Field::Bars(cols, rows)])],
        };
    }
    design(n)
}

fn row(weight: f64, fields: &[Field]) -> Row {
    Row {
        weight,
        fields: fields.to_vec(),
    }
}

/// The design of a leaf by its number in [`DESIGNS`] (from 1).
pub fn design(n: usize) -> LeafDesign {
    use Field::*;
    let framed = |rows: Vec<Row>| LeafDesign::Framed {
        stile: 0.11,
        rail: 0.11,
        bottom: 0.2,
        rows,
    };
    match n {
        3 => LeafDesign::Flush {
            grooves: 6,
            vertical: false,
            moulded: None,
            vision: None,
        },
        4 => LeafDesign::Flush {
            grooves: 5,
            vertical: true,
            moulded: None,
            vision: None,
        },
        5 => framed(vec![row(1.0, &[Raised]), row(1.0, &[Raised])]),
        6 => framed(vec![
            row(1.0, &[Raised, Raised]),
            row(1.0, &[Raised, Raised]),
        ]),
        7 => framed(vec![
            row(0.45, &[Raised, Raised]),
            row(1.0, &[Raised, Raised]),
            row(1.0, &[Raised, Raised]),
        ]),
        8 => framed(vec![row(1.0, &[Sunk])]),
        9 => LeafDesign::Flush {
            grooves: 0,
            vertical: false,
            moulded: Some(vec![row(1.0, &[Sunk]), row(1.0, &[Sunk])]),
            vision: None,
        },
        10 => framed(vec![row(1.0, &[Glass])]),
        11 => framed(vec![row(1.3, &[Glass]), row(1.0, &[Raised])]),
        12 => framed(vec![row(1.0, &[Bars(2, 5)])]),
        13 => framed(vec![row(1.0, &[Louvres])]),
        14 => LeafDesign::Planks { boards: 5 },
        15..=19 => LeafDesign::Sheet {
            embossed: n == 16,
            vision: match n {
                17 => Some(Vision::Square {
                    w: 0.3,
                    h: 0.4,
                    z: 1.45,
                }),
                18 => Some(Vision::Round { d: 0.36, z: 1.5 }),
                19 => Some(Vision::Square {
                    w: 0.15,
                    h: 0.75,
                    z: 1.35,
                }),
                _ => None,
            },
            armoured: false,
        },
        20 | 21 => LeafDesign::Flush {
            grooves: 0,
            vertical: false,
            moulded: None,
            vision: Some(if n == 20 {
                Vision::Square {
                    w: 0.3,
                    h: 0.4,
                    z: 1.45,
                }
            } else {
                Vision::Round { d: 0.36, z: 1.5 }
            }),
        },
        22 => LeafDesign::Sheet {
            embossed: true,
            vision: None,
            armoured: true,
        },
        _ => LeafDesign::Flush {
            grooves: 0,
            vertical: false,
            moulded: None,
            vision: None,
        },
    }
}

/// The section of a casing or a moulding: points `(a, h)`, `a` across
/// its width from its inner edge (0) outwards, `h` off the face it lies
/// on, from the inner edge round to the outer one (both at `h` = 0
/// closing it on the face).
#[derive(Debug, Clone, PartialEq)]
pub struct Profile(pub Vec<[f64; 2]>);

/// The casings the family offers, by name.
pub const CASINGS: &[&str] = &[
    "Como la colección",
    "Sin tapajuntas",
    "Plano",
    "Redondeado",
    "Moldurado",
    "Con junquillo",
];

/// The section of a casing by its number in [`CASINGS`]; `None`: none.
pub fn casing(n: usize) -> Option<Profile> {
    let w = 0.07;
    Some(Profile(match n {
        2 => return None,
        4 => {
            // Rounded over on its outer edge.
            let mut p = vec![[0.0, 0.0], [0.0, 0.014]];
            for k in 0..=6 {
                let a = std::f64::consts::FRAC_PI_2 * k as f64 / 6.0;
                p.push([w - 0.014 + 0.014 * a.sin(), 0.014 * a.cos()]);
            }
            p.push([w, 0.0]);
            p
        }
        5 => vec![
            // An ogee rising from the outer edge to a flat at the inner
            // one, with a fillet.
            [0.0, 0.0],
            [0.0, 0.022],
            [0.012, 0.022],
            [0.016, 0.019],
            [0.026, 0.017],
            [0.036, 0.013],
            [0.046, 0.011],
            [0.054, 0.009],
            [0.060, 0.006],
            [0.066, 0.004],
            [0.072, 0.003],
            [0.08, 0.003],
            [0.08, 0.0],
        ],
        6 => vec![
            // Flat with a bead along its inner edge.
            [0.0, 0.0],
            [0.0, 0.016],
            [0.004, 0.02],
            [0.009, 0.02],
            [0.013, 0.016],
            [0.015, 0.012],
            [w, 0.012],
            [w, 0.0],
        ],
        _ => vec![[0.0, 0.0], [0.0, 0.012], [w, 0.012], [w, 0.0]],
    }))
}

/// The handles the family offers, by name.
pub const HANDLES: &[&str] = &[
    "Como la colección",
    "Palanca con roseta",
    "Palanca con placa",
    "Palanca recta",
    "Pomo",
    "Tirador de barra",
    "Sin manilla",
    "Barra antipánico",
    "Placa de empuje y tirador",
    "Escudo de seguridad",
    "Tirador largo",
];

/// The hinges the family offers, by name.
pub const HINGES: &[&str] = &["Como la colección", "Ocultas", "De pala", "De pernio"];

/// How a collection makes a door: the leaf, the casing, the handle and
/// the hinges (by their numbers in the lists above).
#[derive(Debug, Clone, Copy)]
struct Look {
    design: usize,
    casing: usize,
    handle: usize,
    hinges: usize,
}

const LOOKS: [Look; 8] = [
    // Nórdico cálido: flush, flat casing, lever on a rose, concealed.
    Look {
        design: 2,
        casing: 3,
        handle: 2,
        hinges: 2,
    },
    // Mediterráneo: four panels, rounded casing, knob, butt hinges.
    Look {
        design: 6,
        casing: 4,
        handle: 5,
        hinges: 3,
    },
    // Años 50: grooved across, flat casing, lever on a plate.
    Look {
        design: 3,
        casing: 3,
        handle: 3,
        hinges: 3,
    },
    // Japandi: grooved up, no casing, straight lever.
    Look {
        design: 4,
        casing: 2,
        handle: 4,
        hinges: 2,
    },
    // Contemporáneo: flush, no casing, straight lever, concealed.
    Look {
        design: 2,
        casing: 2,
        handle: 4,
        hinges: 2,
    },
    // Rústica: planks, rounded casing, knob, pin hinges.
    Look {
        design: 14,
        casing: 4,
        handle: 5,
        hinges: 4,
    },
    // Industrial: glazing bars, flat casing, pull bar, butt hinges.
    Look {
        design: 12,
        casing: 3,
        handle: 6,
        hinges: 3,
    },
    // Clásica: six panels, moulded casing, lever on a plate, pin hinges.
    Look {
        design: 7,
        casing: 5,
        handle: 3,
        hinges: 4,
    },
];

/// The look of the door: its collection's (the model chosen, `style`),
/// with what was chosen over it.
fn look(p: &Params) -> Look {
    let s = (p.style() as usize).clamp(1, 8);
    let mut l = LOOKS[s - 1];
    let pick = |key: &str| {
        let n = p.get(key, 1.0).round() as usize;
        (n >= 2).then_some(n)
    };
    if let Some(n) = pick("leaf_design") {
        l.design = n;
    }
    if let Some(n) = pick("casing") {
        l.casing = n;
    }
    if let Some(n) = pick("handle_style") {
        l.handle = n;
    }
    if let Some(n) = pick("hinges") {
        l.hinges = n;
    }
    l
}

/// A door with its leaves, casings and hardware.
pub fn door(b: &mut Builder, p: &Params) {
    let l = look(p);
    let x0 = p.get("leaf_x0", -0.35);
    let x1 = p.get("leaf_x1", 0.35);
    let top = p.get("leaf_top", 2.0);
    let t = p.get("leaf_t", 0.04).clamp(0.02, 0.07);
    let w = p.get("lining_w", x1 - x0 + 0.1);
    let h = p.get("lining_h", top + 0.05);
    let fw = p.get("frame_width", 0.05);
    let fd = p.get("frame_depth", 0.07);
    // A pivot door: one leaf turning on pins this far from its hinge-side
    // edge, with no hinges and, unless one is chosen, a long pull.
    let pivot = p.get("pivot", 0.0).max(0.0);
    let leaves = if pivot > 0.0 {
        1
    } else {
        p.get("leaves", 1.0).round() as i32
    };
    let hinge_left = p.flag("hinge_left", true);
    let mut l = l;
    if pivot > 0.0 && p.get("handle_style", 1.0).round() < 2.0 {
        l.handle = 11;
    }
    // The leaves: one, or two meeting at the split; the hinges at the
    // outer edges, the handle where they meet (on the active one).
    let floor = 0.008;
    let gap = 0.003;
    let mut leaves_at: Vec<(f64, f64, bool, bool)> = Vec::new();
    if leaves == 2 {
        let split = p.get("leaf_split", (x0 + x1) / 2.0);
        let active_left = hinge_left;
        leaves_at.push((x0 + gap, split - gap / 2.0, true, active_left));
        leaves_at.push((split + gap / 2.0, x1 - gap, false, !active_left));
    } else {
        leaves_at.push((x0 + gap, x1 - gap, hinge_left, true));
    }
    let design = design_of(l.design, p);
    // Far away (`lod`): the leaves plain, no hardware, the casings flat.
    // The handle: its height over the floor and the faces it is on.
    let hz = p.get("handle_height", 1.02).clamp(0.3, 2.0);
    let faces = handle_faces(p);
    let far = p.flag("lod", false);
    if far {
        for &(a, c, _, _) in &leaves_at {
            leaf_plain(b, &design, a, c, floor, top - gap, t);
        }
    }
    for &(a, c, hinge_at_a, active) in leaves_at.iter().filter(|_| !far) {
        leaf(b, &design, a, c, floor, top - gap, t);
        if pivot > 0.0 {
            let x = if hinge_at_a { a + pivot } else { c - pivot };
            pivot_pins(b, x, floor, top - gap, t);
        } else {
            hinges(
                b,
                l.hinges,
                if hinge_at_a { a } else { c },
                hinge_at_a,
                floor,
                top,
                t,
            );
        }
        let edge = if hinge_at_a { c } else { a };
        // Towards the hinges from the free edge.
        let inward = if hinge_at_a { -1.0 } else { 1.0 };
        if l.handle == 8 {
            // A panic bar on the side it is pushed from, on every leaf of a
            // pair (each opens to escape); a lever on the other side of
            // the active one.
            panic_bar(b, a, c, inward, t);
            if active {
                handle_on(b, 2, edge + inward * 0.065, hz, t, inward, &[1.0]);
            }
        } else if active {
            let inset = if l.handle == 11 { 0.1 } else { 0.065 };
            handle_on(b, l.handle, edge + inward * inset, hz, t, inward, faces);
            if p.flag("peephole", false) {
                spyhole(b, (a + c) / 2.0, 1.55, t);
            }
        } else if l.handle != 7 {
            // The passive leaf of a pair: a flush bolt on its edge.
            b.rbox(
                METAL,
                [edge - 0.004 * inward.signum(), -0.006, 1.6],
                [edge + 0.0005 * inward.signum(), 0.006, 1.8],
                0.001,
            );
        }
    }
    for &(a, c, hinge_at_a, _) in leaves_at.iter().filter(|_| !far) {
        let hinge_x = if hinge_at_a { a } else { c };
        let inward = if hinge_at_a { -1.0 } else { 1.0 };
        if p.flag("kick_plate", false) {
            // A steel plate along the bottom of each face.
            for side in [1.0, -1.0] {
                let (ya, yb) = sorted2(side * (t / 2.0 - 0.0005), side * (t / 2.0 + 0.0015));
                b.rbox(
                    METAL,
                    [a + 0.025, ya, floor + 0.01],
                    [c - 0.025, yb, floor + 0.26],
                    0.0,
                );
            }
        }
        if p.flag("closer", false) {
            closer(b, hinge_x, inward, top, t, p.get("face_pos", fd / 2.0));
        }
    }
    // The stop the leaves close against, on the side away from where they
    // open, round the jambs and the head.
    let si = 0.012;
    let (ya, yb) = (-t / 2.0 - 0.015, -t / 2.0 - 0.0005);
    if fd > t + 0.02 {
        b.rbox(FRAME, [x0 - 0.0005, ya, 0.0], [x0 + si, yb, top], 0.002);
        b.rbox(FRAME, [x1 - si, ya, 0.0], [x1 + 0.0005, yb, top], 0.002);
        b.rbox(
            FRAME,
            [x0 + si, ya, top - si],
            [x1 - si, yb, top + 0.0005],
            0.002,
        );
    }
    lining_and_casings(b, p, &l, w, h, fw, fd, far);
    // Seen as the plan draws it: the hand mirrored when asked.
    if p.flag("mirror", false) {
        mirror_x(b);
    }
}

/// The lining across the reveal to each face that takes a casing
/// (`casing_pos`, `casing_neg`), and the collection's casing on it.
#[allow(clippy::too_many_arguments)]
fn lining_and_casings(
    b: &mut Builder,
    p: &Params,
    l: &Look,
    w: f64,
    h: f64,
    fw: f64,
    fd: f64,
    far: bool,
) {
    let profile = if far && l.casing != 2 {
        casing(3)
    } else {
        casing(l.casing)
    };
    for (face, takes, dir) in [
        (p.get("face_pos", fd / 2.0), p.flag("casing_pos", true), 1.0),
        (
            p.get("face_neg", -fd / 2.0),
            p.flag("casing_neg", true),
            -1.0,
        ),
    ] {
        if !takes {
            continue;
        }
        let edge = dir * fd / 2.0;
        let (lo, hi) = if dir > 0.0 {
            (edge, face)
        } else {
            (face, edge)
        };
        if hi - lo > 0.003 {
            // Jambs and head of the lining (forro), as thick as the frame.
            b.rbox(FRAME, [-w / 2.0, lo, 0.0], [-w / 2.0 + fw, hi, h], 0.0);
            b.rbox(FRAME, [w / 2.0 - fw, lo, 0.0], [w / 2.0, hi, h], 0.0);
            b.rbox(
                FRAME,
                [-w / 2.0 + fw, lo, h - fw],
                [w / 2.0 - fw, hi, h],
                0.0,
            );
        }
        if let Some(pr) = &profile {
            // Its inner edge a little back from the lining's face.
            let xi = w / 2.0 - fw + 0.008;
            let zi = h - fw + 0.008;
            let path = [[-xi, 0.0], [-xi, zi], [xi, zi], [xi, 0.0]];
            moulding(b, FRAME, &path, false, face, dir, pr);
        }
    }
}

/// The pins a pivot door turns on: a plate in the floor and a bracket in
/// the head, both on the axis at `x`.
fn pivot_pins(b: &mut Builder, x: f64, z0: f64, z1: f64, t: f64) {
    b.rbox(
        METAL,
        [x - 0.03, -t / 2.0, 0.0],
        [x + 0.03, t / 2.0, z0 + 0.03],
        0.002,
    );
    b.cyl(METAL, [x, 0.0, z1 - 0.03], [x, 0.0, z1 + 0.004], 0.009);
    b.rbox(
        METAL,
        [x - 0.03, -t / 2.0, z1 - 0.04],
        [x + 0.03, t / 2.0, z1 - 0.02],
        0.002,
    );
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

/// A leaf as seen from far away: a board, or for a framed one its frame
/// with each field flat (glass where it has glass).
pub fn leaf_plain(b: &mut Builder, d: &LeafDesign, a: f64, c: f64, z0: f64, z1: f64, t: f64) {
    let h2 = t / 2.0;
    let vision = match d {
        LeafDesign::Flush { vision, .. } | LeafDesign::Sheet { vision, .. } => *vision,
        _ => None,
    };
    if let Some(v) = vision {
        let h = hole_of(v, a, c, z0, z1);
        board(b, LEAF, a, c, z0, z1, -h2, h2, Some(h));
        b.rbox(GLASS, [h.xa, -0.004, h.za], [h.xc, 0.004, h.zc], 0.0);
        return;
    }
    let LeafDesign::Framed {
        stile,
        rail,
        bottom,
        rows,
    } = d
    else {
        b.rbox(LEAF, [a, -h2, z0], [c, h2, z1], 0.0);
        return;
    };
    let (stile, rail, bottom) = (
        stile.min((c - a) / 3.0),
        rail.min((z1 - z0) / 8.0),
        bottom.min((z1 - z0) / 6.0),
    );
    b.rbox(LEAF, [a, -h2, z0], [a + stile, h2, z1], 0.0);
    b.rbox(LEAF, [c - stile, -h2, z0], [c, h2, z1], 0.0);
    b.rbox(
        LEAF,
        [a + stile, -h2, z0],
        [c - stile, h2, z0 + bottom],
        0.0,
    );
    b.rbox(LEAF, [a + stile, -h2, z1 - rail], [c - stile, h2, z1], 0.0);
    let total: f64 = rows.iter().map(|r| r.weight).sum::<f64>().max(1e-9);
    let (za, zc) = (z0 + bottom, z1 - rail);
    let mut zt = zc;
    for (k, row) in rows.iter().enumerate() {
        let zb = zt - (zc - za - rail * (rows.len() as f64 - 1.0)) * row.weight / total;
        if k + 1 < rows.len() {
            b.rbox(LEAF, [a + stile, -h2, zb - rail], [c - stile, h2, zb], 0.0);
        }
        let n = row.fields.len().max(1);
        let w = (c - a - 2.0 * stile - stile * (n as f64 - 1.0)) / n as f64;
        for (i, f) in row.fields.iter().enumerate() {
            let x0 = a + stile + i as f64 * (w + stile);
            if i > 0 {
                b.rbox(LEAF, [x0 - stile, -h2, zb], [x0, h2, zt], 0.0);
            }
            let (role, k) = match f {
                Field::Glass | Field::Bars(..) => (GLASS, 0.004),
                _ => (LEAF, h2 * 0.35),
            };
            b.rbox(role, [x0, -k, zb], [x0 + w, k, zt], 0.0);
        }
        zt = zb - rail;
    }
}

/// A leaf from `a` to `c`, `z0` to `z1`, `t` thick, centred on `y` = 0.
pub fn leaf(b: &mut Builder, d: &LeafDesign, a: f64, c: f64, z0: f64, z1: f64, t: f64) {
    let h2 = t / 2.0;
    match d {
        LeafDesign::Flush {
            grooves,
            vertical,
            moulded,
            vision,
        } => {
            if let Some(v) = vision {
                // A core and its skins round the vision panel's hole.
                let d = 0.003;
                let hole = hole_of(*v, a, c, z0, z1);
                let at = Some(hole);
                board(b, LEAF, a, c, z0, z1, -h2 + d, h2 - d, at);
                board(b, LEAF, a, c, z0, z1, h2 - d - 0.0005, h2, at);
                board(b, LEAF, a, c, z0, z1, -h2, -h2 + d + 0.0005, at);
                glaze(b, hole, h2);
            } else if *grooves == 0 {
                // A core and a skin on each face (the face as one thin
                // board: a single deep box this size takes the shadow of
                // its own back in the 3D view).
                let d = 0.003;
                b.rbox(LEAF, [a, -h2 + d, z0], [c, h2 - d, z1], 0.0);
                b.rbox(LEAF, [a, h2 - d - 0.0005, z0], [c, h2, z1], 0.0);
                b.rbox(LEAF, [a, -h2, z0], [c, -h2 + d + 0.0005, z1], 0.0);
            } else {
                // Grooves on both faces: the core 3 mm in from each face,
                // and the face in strips with 4 mm between them, so the
                // grooves are real (their shadow and their lines).
                let d = 0.003;
                let g = 0.002;
                b.rbox(LEAF, [a, -h2 + d, z0], [c, h2 - d, z1], 0.0);
                let n = *grooves + 1;
                for side in [1.0, -1.0] {
                    let (ya, yb) = if side > 0.0 {
                        (h2 - d - 0.0005, h2)
                    } else {
                        (-h2, -h2 + d + 0.0005)
                    };
                    for k in 0..n {
                        let (f0, f1) = (k as f64 / n as f64, (k + 1) as f64 / n as f64);
                        let ga = if k == 0 { 0.0 } else { g };
                        let gb = if k + 1 == n { 0.0 } else { g };
                        if *vertical {
                            let (xa, xc) = (a + (c - a) * f0 + ga, a + (c - a) * f1 - gb);
                            b.rbox(LEAF, [xa, ya, z0], [xc, yb, z1], 0.0);
                        } else {
                            let (za, zc) = (z0 + (z1 - z0) * f0 + ga, z0 + (z1 - z0) * f1 - gb);
                            b.rbox(LEAF, [a, ya, za], [c, yb, zc], 0.0);
                        }
                    }
                }
            }
            if let Some(rows) = moulded {
                // Applied moulded frames on both faces.
                let m = 0.1;
                let gap = 0.08;
                let total: f64 = rows.iter().map(|r| r.weight).sum::<f64>().max(1e-6);
                let usable = (z1 - z0) - 2.0 * m - gap * (rows.len() as f64 - 1.0);
                let mut zt = z1 - m;
                for r in rows {
                    let rh = usable * r.weight / total;
                    let n = r.fields.len().max(1);
                    let cw = ((c - a) - 2.0 * m - gap * (n as f64 - 1.0)) / n as f64;
                    for i in 0..n {
                        let xa = a + m + i as f64 * (cw + gap);
                        let path = [[xa, zt - rh], [xa, zt], [xa + cw, zt], [xa + cw, zt - rh]];
                        let bead = Profile(vec![
                            [0.0, 0.0],
                            [0.0, 0.004],
                            [0.004, 0.009],
                            [0.01, 0.009],
                            [0.016, 0.004],
                            [0.018, 0.0],
                        ]);
                        for side in [1.0, -1.0] {
                            moulding(b, LEAF, &path, true, side * h2, side, &bead);
                        }
                    }
                    zt -= rh + gap;
                }
            }
        }
        LeafDesign::Sheet {
            embossed,
            vision,
            armoured,
        } => {
            let hole = vision.map(|v| hole_of(v, a, c, z0, z1));
            // Two sheets on a core.
            let d = 0.0015;
            board(b, LEAF, a, c, z0, z1, -h2 + d, h2 - d, hole);
            board(b, LEAF, a, c, z0, z1, h2 - d - 0.0005, h2, hole);
            board(b, LEAF, a, c, z0, z1, -h2, -h2 + d + 0.0005, hole);
            // The folded edge, a rim round each face.
            let rim = if *armoured { 0.05 } else { 0.022 };
            let k = 0.0015;
            for side in [1.0, -1.0] {
                let (ya, yb) = sorted2(side * (h2 - 0.0005), side * (h2 + k));
                b.rbox(LEAF, [a, ya, z0], [a + rim, yb, z1], 0.0);
                b.rbox(LEAF, [c - rim, ya, z0], [c, yb, z1], 0.0);
                b.rbox(LEAF, [a + rim, ya, z0], [c - rim, yb, z0 + rim], 0.0);
                b.rbox(LEAF, [a + rim, ya, z1 - rim], [c - rim, yb, z1], 0.0);
            }
            if *embossed {
                // Two panels pressed out of each face, one over the other
                // (below the vision panel if there is one).
                let m = 0.11_f64.min((c - a) * 0.2);
                let top = hole.map_or(z1 - m, |h| h.za - m * 0.8);
                let gap = 0.09;
                let low = z0 + m;
                if top - low > 2.0 * gap + 0.3 {
                    let mid = low + (top - low) * 0.42;
                    for (za, zc) in [(low, mid - gap / 2.0), (mid + gap / 2.0, top)] {
                        for side in [1.0, -1.0] {
                            let (ya, yb) = sorted2(side * (h2 - 0.0005), side * (h2 + 0.004));
                            b.rbox(LEAF, [a + m, ya, za], [c - m, yb, zc], 0.006);
                        }
                    }
                }
            }
            if let Some(h) = hole {
                glaze(b, h, h2 + k);
            }
        }
        LeafDesign::Planks { boards } => {
            let n = (*boards).max(2);
            let bw = (c - a) / n as f64;
            for i in 0..n {
                let xa = a + bw * i as f64;
                b.rbox(
                    LEAF,
                    [xa + 0.0008, -h2, z0],
                    [xa + bw - 0.0008, h2, z1],
                    0.003,
                );
            }
            // The ledges on the side away from the opening, and a brace.
            let yl = -h2 - 0.022;
            for z in [z0 + 0.2, (z0 + z1) / 2.0, z1 - 0.2] {
                b.rbox(
                    LEAF,
                    [a + 0.04, yl, z - 0.06],
                    [c - 0.04, -h2 + 0.0005, z + 0.06],
                    0.004,
                );
            }
            let (pa, pc) = ([a + 0.08, (z0 + 0.26)], [c - 0.08, (z0 + z1) / 2.0 - 0.06]);
            brace(b, pa, pc, yl, -h2 + 0.0005);
            let (pa, pc) = ([a + 0.08, (z0 + z1) / 2.0 + 0.06], [c - 0.08, z1 - 0.26]);
            brace(b, pa, pc, yl, -h2 + 0.0005);
        }
        LeafDesign::Framed {
            stile,
            rail,
            bottom,
            rows,
        } => {
            let s = stile.min((c - a) * 0.25);
            let total: f64 = rows.iter().map(|r| r.weight).sum::<f64>().max(1e-6);
            let usable = (z1 - z0) - rail - bottom - rail * (rows.len() as f64 - 1.0);
            // Stiles full height.
            b.rbox(LEAF, [a, -h2, z0], [a + s, h2, z1], 0.002);
            b.rbox(LEAF, [c - s, -h2, z0], [c, h2, z1], 0.002);
            // Top and bottom rails.
            b.rbox(LEAF, [a + s, -h2, z1 - rail], [c - s, h2, z1], 0.002);
            b.rbox(LEAF, [a + s, -h2, z0], [c - s, h2, z0 + bottom], 0.002);
            let mut zt = z1 - rail;
            for (k, r) in rows.iter().enumerate() {
                let rh = usable * r.weight / total;
                let zb = zt - rh;
                let n = r.fields.len().max(1);
                let cw = ((c - a) - 2.0 * s - s * (n as f64 - 1.0)) / n as f64;
                for (i, f) in r.fields.iter().enumerate() {
                    let xa = a + s + i as f64 * (cw + s);
                    field(b, *f, xa, xa + cw, zb, zt, t);
                    if i + 1 < n {
                        // The muntin between two fields.
                        b.rbox(LEAF, [xa + cw, -h2, zb], [xa + cw + s, h2, zt], 0.002);
                    }
                }
                if k + 1 < rows.len() {
                    // The rail under this row.
                    b.rbox(LEAF, [a + s, -h2, zb - rail], [c - s, h2, zb], 0.002);
                }
                zt = zb - rail;
            }
        }
    }
}

fn sorted2(a: f64, b: f64) -> (f64, f64) {
    (a.min(b), a.max(b))
}

/// Where a vision panel's hole is in a leaf from `a` to `c`, `z0` to `z1`:
/// its box (`round`: the box round its circle).
#[derive(Debug, Clone, Copy)]
struct Hole {
    xa: f64,
    xc: f64,
    za: f64,
    zc: f64,
    round: bool,
}

fn hole_of(v: Vision, a: f64, c: f64, z0: f64, z1: f64) -> Hole {
    let x = (a + c) / 2.0;
    let room = (c - a) * 0.7;
    let (w, h, z, round) = match v {
        Vision::Square { w, h, z } => (w.min(room), h, z, false),
        Vision::Round { d, z } => (d.min(room), d.min(room), z, true),
    };
    let z = (z0 + z).clamp(z0 + h / 2.0 + 0.15, z1 - h / 2.0 - 0.15);
    Hole {
        xa: x - w / 2.0,
        xc: x + w / 2.0,
        za: z - h / 2.0,
        zc: z + h / 2.0,
        round,
    }
}

/// A board from `a` to `c`, `z0` to `z1`, `y0` to `y1`, with `hole`
/// through it (a round one as its square box and the four corners round
/// the circle).
#[allow(clippy::too_many_arguments)]
fn board(
    b: &mut Builder,
    role: &'static str,
    a: f64,
    c: f64,
    z0: f64,
    z1: f64,
    y0: f64,
    y1: f64,
    hole: Option<Hole>,
) {
    let Some(h) = hole else {
        b.rbox(role, [a, y0, z0], [c, y1, z1], 0.0);
        return;
    };
    let o = 0.0005;
    b.rbox(role, [a, y0, z0], [h.xa + o, y1, z1], 0.0);
    b.rbox(role, [h.xc - o, y0, z0], [c, y1, z1], 0.0);
    b.rbox(role, [h.xa, y0, z0], [h.xc, y1, h.za + o], 0.0);
    b.rbox(role, [h.xa, y0, h.zc - o], [h.xc, y1, z1], 0.0);
    if !h.round {
        return;
    }
    // The corners between the box and the circle, each a fan from its
    // corner (it sees the whole quarter of the circle facing it).
    let (cx, cz) = ((h.xa + h.xc) / 2.0, (h.za + h.zc) / 2.0);
    let r = (h.xc - h.xa) / 2.0;
    let n = 10;
    for (k, corner) in [[h.xc, h.zc], [h.xa, h.zc], [h.xa, h.za], [h.xc, h.za]]
        .iter()
        .enumerate()
    {
        let a0 = k as f64 * std::f64::consts::FRAC_PI_2;
        let ring: Vec<[f64; 2]> = (0..=n)
            .map(|i| {
                let t = a0 + std::f64::consts::FRAC_PI_2 * i as f64 / n as f64;
                [cx + r * t.cos(), cz + r * t.sin()]
            })
            .collect();
        let first = b.parts.get(role).map_or(0, |t| t.len());
        let front: Vec<u32> = ring.iter().map(|q| b.point([q[0], y1, q[1]])).collect();
        let back: Vec<u32> = ring.iter().map(|q| b.point([q[0], y0, q[1]])).collect();
        let cf = b.point([corner[0], y1, corner[1]]);
        let cb = b.point([corner[0], y0, corner[1]]);
        for i in 0..n {
            b.tri(role, cf, front[i], front[i + 1]);
            b.tri(role, cb, back[i + 1], back[i]);
            // The face of the hole, round.
            b.quad(role, front[i], back[i], back[i + 1], front[i + 1]);
        }
        // Its two straight sides, against the boxes.
        b.quad(role, cf, cb, back[0], front[0]);
        b.quad(role, cf, front[n], back[n], cb);
        orient(b, role, first);
    }
}

/// The glass of a vision panel and what holds it: beads round a square
/// one, a ring on each face round a porthole (`face`: half the leaf).
fn glaze(b: &mut Builder, h: Hole, face: f64) {
    let o = 0.001;
    if h.round {
        let (cx, cz) = ((h.xa + h.xc) / 2.0, (h.za + h.zc) / 2.0);
        let r = (h.xc - h.xa) / 2.0;
        b.cyl(GLASS, [cx, -0.004, cz], [cx, 0.004, cz], r + o);
        for side in [1.0, -1.0] {
            let y = side * (face + 0.002);
            let path: Vec<P3> = (0..=40)
                .map(|i| {
                    let t = std::f64::consts::TAU * i as f64 / 40.0;
                    [cx + (r + 0.008) * t.cos(), y, cz + (r + 0.008) * t.sin()]
                })
                .collect();
            sweep(b, METAL, &path, |_| 0.011, 8, Cap::Flat, Cap::Flat);
        }
    } else {
        b.rbox(
            GLASS,
            [h.xa - o, -0.004, h.za - o],
            [h.xc + o, 0.004, h.zc + o],
            0.0,
        );
        beads(b, h.xa, h.xc, h.za, h.zc, face);
    }
}

/// A diagonal brace on a ledged door, from `a` to `c` (`x`, `z`), between
/// `y0` and `y1`.
fn brace(b: &mut Builder, a: [f64; 2], c: [f64; 2], y0: f64, y1: f64) {
    let d = [c[0] - a[0], 0.0, c[1] - a[1]];
    let l = (d[0] * d[0] + d[2] * d[2]).sqrt().max(1e-6);
    let along = [d[0] / l, 0.0, d[2] / l];
    let across = [-along[2], 0.0, along[0]];
    let mid = [(a[0] + c[0]) / 2.0, (y0 + y1) / 2.0, (a[1] + c[1]) / 2.0];
    b.obox(
        LEAF,
        mid,
        [along[0] * l / 2.0, 0.0, along[2] * l / 2.0],
        [0.0, (y1 - y0) / 2.0, 0.0],
        [across[0] * 0.05, 0.0, across[2] * 0.05],
    );
}

/// What fills a field from `xa` to `xc`, `za` to `zc`, of a leaf `t`
/// thick.
fn field(b: &mut Builder, f: Field, xa: f64, xc: f64, za: f64, zc: f64, t: f64) {
    let h2 = t / 2.0;
    let o = 0.0005;
    match f {
        Field::Sunk => {
            b.rbox(
                LEAF,
                [xa - o, -h2 * 0.35, za - o],
                [xc + o, h2 * 0.35, zc + o],
                0.0,
            );
            sticking(b, xa, xc, za, zc, h2);
        }
        Field::Raised => {
            b.rbox(
                LEAF,
                [xa - o, -h2 * 0.35, za - o],
                [xc + o, h2 * 0.35, zc + o],
                0.0,
            );
            // The raised middle: a bevel down to the field's edge.
            let k = 0.035_f64.min((xc - xa) * 0.25).min((zc - za) * 0.25);
            for side in [1.0, -1.0] {
                let y0 = side * h2 * 0.35;
                let y1 = side * h2 * 0.8;
                let (ya, yb) = if side > 0.0 {
                    (y0 - o, y1)
                } else {
                    (y1, y0 + o)
                };
                b.rbox(LEAF, [xa + k, ya, za + k], [xc - k, yb, zc - k], 0.003);
                // The bevel as a frame of slopes from the middle down to
                // the field.
                bevel(
                    b,
                    xa + 0.004,
                    xc - 0.004,
                    za + 0.004,
                    zc - 0.004,
                    k - 0.004,
                    y0,
                    y1,
                );
            }
            sticking(b, xa, xc, za, zc, h2);
        }
        Field::Glass => {
            b.rbox(
                GLASS,
                [xa - o, -0.004, za - o],
                [xc + o, 0.004, zc + o],
                0.0,
            );
            beads(b, xa, xc, za, zc, h2);
        }
        Field::Bars(cols, rows) => {
            b.rbox(
                GLASS,
                [xa - o, -0.004, za - o],
                [xc + o, 0.004, zc + o],
                0.0,
            );
            beads(b, xa, xc, za, zc, h2);
            let m = 0.012;
            for i in 1..cols.max(1) {
                let x = xa + (xc - xa) * i as f64 / cols as f64;
                b.rbox(
                    LEAF,
                    [x - m / 2.0, -h2 * 0.7, za],
                    [x + m / 2.0, h2 * 0.7, zc],
                    0.002,
                );
            }
            for j in 1..rows.max(1) {
                let z = za + (zc - za) * j as f64 / rows as f64;
                b.rbox(
                    LEAF,
                    [xa, -h2 * 0.7, z - m / 2.0],
                    [xc, h2 * 0.7, z + m / 2.0],
                    0.002,
                );
            }
        }
        Field::Louvres => {
            let step = 0.045;
            let n = ((zc - za) / step).floor().max(1.0) as usize;
            let step = (zc - za) / n as f64;
            for k in 0..n {
                let z = za + step * (k as f64 + 0.5);
                // Each slat tilted 40° across the leaf.
                let (s, c) = (40f64.to_radians()).sin_cos();
                b.obox(
                    LEAF,
                    [(xa + xc) / 2.0, 0.0, z],
                    [(xc - xa) / 2.0 + o, 0.0, 0.0],
                    [0.0, h2 * 0.9 * c, h2 * 0.9 * s],
                    [0.0, -0.004 * s, 0.004 * c],
                );
            }
        }
    }
}

/// The moulding stuck round a field on both faces of a framed leaf.
fn sticking(b: &mut Builder, xa: f64, xc: f64, za: f64, zc: f64, h2: f64) {
    let bead = Profile(vec![[0.0, 0.0], [0.0, 0.006], [0.006, 0.006], [0.012, 0.0]]);
    let path = [[xa, za], [xa, zc], [xc, zc], [xc, za]];
    for side in [1.0, -1.0] {
        // On the field's face, against the frame.
        moulding(b, LEAF, &path, true, side * h2 * 0.35, side, &bead);
    }
}

/// The beads that hold the glass of a field, on both faces.
fn beads(b: &mut Builder, xa: f64, xc: f64, za: f64, zc: f64, h2: f64) {
    let bead = Profile(vec![
        [0.0, 0.0],
        [0.0, h2 - 0.004],
        [0.006, h2 - 0.004],
        [0.012, 0.0],
    ]);
    let path = [[xa, za], [xa, zc], [xc, zc], [xc, za]];
    for side in [1.0, -1.0] {
        moulding(b, LEAF, &path, true, side * 0.004, side, &bead);
    }
}

/// The bevel of a raised panel: four slopes from the inset `k` at height
/// `y1` down to the field's edge at `y0`.
#[allow(clippy::too_many_arguments)]
fn bevel(b: &mut Builder, xa: f64, xc: f64, za: f64, zc: f64, k: f64, y0: f64, y1: f64) {
    let ym = y0 + (y1 - y0) * 0.55;
    let outer = [[xa, za], [xa, zc], [xc, zc], [xc, za]];
    let inner = [
        [xa + k, za + k],
        [xa + k, zc - k],
        [xc - k, zc - k],
        [xc - k, za + k],
    ];
    let side = (y1 - y0).signum();
    let p: Vec<u32> = outer.iter().map(|q| b.point([q[0], y0, q[1]])).collect();
    let q: Vec<u32> = inner.iter().map(|q| b.point([q[0], ym, q[1]])).collect();
    for i in 0..4 {
        let j = (i + 1) % 4;
        // The outer ring runs up the left, across the top, down the right:
        // clockwise seen from +y; turned so the slopes look out.
        if side > 0.0 {
            b.quad(LEAF, p[i], q[i], q[j], p[j]);
        } else {
            b.quad(LEAF, p[i], p[j], q[j], q[i]);
        }
    }
}

/// A moulding of section `profile` along `path` (`x`, `z`) lying on the
/// face at `y` = `face`, standing out towards `dir` (+1 or -1). Its
/// width runs to the left of the path's direction (outwards round an
/// opening drawn up the left jamb, across the head and down the right;
/// inwards round a field drawn the same way, `closed`).
pub fn moulding(
    b: &mut Builder,
    role: &'static str,
    path: &[[f64; 2]],
    closed: bool,
    face: f64,
    dir: f64,
    profile: &Profile,
) {
    let n = path.len();
    if n < 2 || profile.0.len() < 3 {
        return;
    }
    // A field is drawn clockwise seen from +y, as an opening is: its
    // width must go in, so to the right.
    let sign = if closed { -1.0 } else { 1.0 };
    let normal = |i: usize, j: usize| -> [f64; 2] {
        let (a, c) = (path[i], path[j]);
        let (dx, dz) = (c[0] - a[0], c[1] - a[1]);
        let l = (dx * dx + dz * dz).sqrt().max(1e-9);
        [sign * -dz / l, sign * dx / l]
    };
    let mitre = |i: usize| -> [f64; 2] {
        let prev = if i > 0 {
            Some(normal(i - 1, i))
        } else if closed {
            Some(normal(n - 1, 0))
        } else {
            None
        };
        let next = if i + 1 < n {
            Some(normal(i, i + 1))
        } else if closed {
            Some(normal(n - 1, 0))
        } else {
            None
        };
        match (prev, next) {
            (Some(a), Some(c)) => {
                let s = 1.0 + a[0] * c[0] + a[1] * c[1];
                if s < 1e-6 {
                    a
                } else {
                    [(a[0] + c[0]) / s, (a[1] + c[1]) / s]
                }
            }
            (Some(a), None) | (None, Some(a)) => a,
            (None, None) => [0.0, 0.0],
        }
    };
    let rings: Vec<Vec<u32>> = (0..n)
        .map(|i| {
            let m = mitre(i);
            profile
                .0
                .iter()
                .map(|&[a, h]| {
                    b.point([path[i][0] + m[0] * a, face + dir * h, path[i][1] + m[1] * a])
                })
                .collect()
        })
        .collect();
    let k = profile.0.len();
    let first_tri = b.parts.get(role).map_or(0, |t| t.len());
    let segs = if closed { n } else { n - 1 };
    for s in 0..segs {
        let (r0, r1) = (&rings[s], &rings[(s + 1) % n]);
        for j in 0..k {
            let j1 = (j + 1) % k;
            b.quad(role, r0[j], r0[j1], r1[j1], r1[j]);
        }
    }
    if !closed {
        // Its ends cut square at the floor: the section as a fan.
        for (ring, flip) in [(&rings[0], false), (&rings[n - 1], true)] {
            for j in 1..k - 1 {
                if flip {
                    b.tri(role, ring[0], ring[j], ring[j + 1]);
                } else {
                    b.tri(role, ring[0], ring[j + 1], ring[j]);
                }
            }
        }
    }
    orient(b, role, first_tri);
}

/// Turns the triangles of `role` from `first` out (by the sign of the
/// volume they hold), whichever way they were made.
fn orient(b: &mut Builder, role: &'static str, first: usize) {
    let Some(tris) = b.parts.get_mut(role) else {
        return;
    };
    let mine = &mut tris[first..];
    if mine.is_empty() {
        return;
    }
    // Round the middle of the piece, not the origin (a thin piece far
    // from it would still give the right sign, but this is steadier).
    let pts: Vec<[f64; 3]> = mine
        .iter()
        .flatten()
        .map(|&i| {
            let q = b.positions[i as usize];
            [q[0] as f64, q[1] as f64, q[2] as f64]
        })
        .collect();
    let c = pts
        .iter()
        .fold([0.0; 3], |a, q| [a[0] + q[0], a[1] + q[1], a[2] + q[2]]);
    let c = c.map(|v| v / pts.len() as f64);
    let vol: f64 = mine
        .iter()
        .map(|t| {
            let p = t.map(|i| {
                let q = b.positions[i as usize];
                [q[0] as f64 - c[0], q[1] as f64 - c[1], q[2] as f64 - c[2]]
            });
            let x = cross(p[1], p[2]);
            p[0][0] * x[0] + p[0][1] * x[1] + p[0][2] * x[2]
        })
        .sum();
    if vol < 0.0 {
        for t in mine.iter_mut() {
            t.swap(1, 2);
        }
    }
}

/// The hinges of a leaf on its edge at `x` (`at_a`: the leaf runs from it
/// towards +x), from `z0` to `top`, seen on the side it opens to.
fn hinges(b: &mut Builder, kind: usize, x: f64, at_a: bool, z0: f64, top: f64, t: f64) {
    if kind <= 2 {
        // Concealed.
        return;
    }
    let tall = top - z0;
    let zs: Vec<f64> = if tall > 2.2 {
        vec![z0 + 0.2, z0 + tall / 2.0, top - 0.22]
    } else {
        vec![z0 + 0.2, top - 0.22]
    };
    let y = t / 2.0;
    let out = if at_a { -1.0 } else { 1.0 };
    for z in zs {
        if kind == 4 {
            // A pin hinge: the knuckle with a ball at each end.
            let c = [x + out * 0.002, y + 0.004, z];
            b.cyl(
                METAL,
                [c[0], c[1], c[2] - 0.05],
                [c[0], c[1], c[2] + 0.05],
                0.007,
            );
            for zb in [c[2] - 0.05, c[2] + 0.05] {
                b.ellipsoid(
                    METAL,
                    [c[0], c[1], zb + (zb - c[2]).signum() * 0.006],
                    [0.008, 0.008, 0.008],
                );
            }
        } else {
            // A butt hinge: the knuckle and a leaf on the frame.
            b.cyl(
                METAL,
                [x, y + 0.003, z - 0.045],
                [x, y + 0.003, z + 0.045],
                0.0055,
            );
        }
    }
}

/// The handle of the active leaf at `x`, `z` on both faces (`inward`:
/// the hinges' side), on a leaf `t` thick.
fn handle(b: &mut Builder, kind: usize, x: f64, z: f64, t: f64, inward: f64) {
    handle_on(b, kind, x, z, t, inward, &[1.0, -1.0]);
}

/// [`handle`] on the faces `sides` only (+1: the side it opens to).
fn handle_on(b: &mut Builder, kind: usize, x: f64, z: f64, t: f64, inward: f64, sides: &[f64]) {
    for &side in sides {
        let y = side * t / 2.0;
        let at = |dx: f64, dy: f64, dz: f64| -> P3 { [x + dx, y + side * dy, z + dz] };
        match kind {
            7 | 8 => {}
            9 => {
                if side < 0.0 {
                    // A push plate on the side it is pushed from.
                    let (ya, yb) = sorted2(y, y + side * 0.002);
                    b.rbox(
                        METAL,
                        [x + inward * 0.06 - 0.05, ya, z + 0.05],
                        [x + inward * 0.06 + 0.05, yb, z + 0.4],
                        0.004,
                    );
                } else {
                    // A pull handle, upright, on the other.
                    let (zb, zt) = (z - 0.15, z + 0.15);
                    for zz in [zb, zt] {
                        b.cyl(METAL, at(0.0, -0.001, zz - z), at(0.0, 0.06, zz - z), 0.01);
                    }
                    b.cyl(
                        METAL,
                        at(0.0, 0.06, zb - z - 0.01),
                        at(0.0, 0.06, zt - z + 0.01),
                        0.013,
                    );
                }
            }
            10 => {
                // A security escutcheon: a long plate with its lever and,
                // below, the raised guard round the lock's cylinder.
                let lo = at(-0.035, -0.0005, -0.24);
                let hi = at(0.035, 0.012, 0.06);
                let (ya, yb) = sorted2(lo[1], hi[1]);
                b.rbox(METAL, [lo[0], ya, lo[2]], [hi[0], yb, hi[2]], 0.01);
                b.cone(
                    METAL,
                    at(0.0, 0.011, -0.16),
                    at(0.0, 0.03, -0.16),
                    0.028,
                    0.022,
                    24,
                );
                let path = [
                    at(0.0, 0.011, 0.0),
                    at(0.0, 0.05, 0.0),
                    at(inward * 0.012, 0.062, 0.0),
                    at(inward * 0.06, 0.064, 0.0),
                    at(inward * 0.125, 0.056, -0.004),
                ];
                sweep(
                    b,
                    METAL,
                    &path,
                    |s| 0.01 - 0.002 * s,
                    12,
                    Cap::Flat,
                    Cap::Round(0.8),
                );
            }
            5 => {
                // A knob on a rose.
                rose(b, at(0.0, 0.0, 0.0), side, 0.027);
                let start = b.mark();
                turned(
                    b,
                    METAL,
                    [0.0; 3],
                    &[
                        (0.0, 0.0),
                        (0.009, 0.0),
                        (0.009, 0.03),
                        (0.026, 0.045),
                        (0.028, 0.058),
                        (0.02, 0.068),
                        (0.0, 0.07),
                    ],
                    18,
                    0.0,
                );
                let base = at(0.0, 0.008, 0.0);
                b.transform_from(start, |q| {
                    [base[0] + q[0], base[1] + side * q[2], base[2] + q[1]]
                });
                // The escutcheon below.
                rose(b, at(0.0, 0.0, -0.08), side, 0.016);
            }
            11 => {
                // A long pull bar, as on a pivot door: from knee to above
                // the head.
                let (zb, zt) = (z - 0.55, z + 0.65);
                for zz in [zb + 0.08, zt - 0.08] {
                    b.cyl(METAL, at(0.0, -0.001, zz - z), at(0.0, 0.06, zz - z), 0.01);
                }
                b.cyl(METAL, at(0.0, 0.06, zb - z), at(0.0, 0.06, zt - z), 0.016);
            }
            6 => {
                // A pull bar on two posts, upright.
                let (zb, zt) = (z - 0.2, z + 0.2);
                for zz in [zb + 0.03, zt - 0.03] {
                    b.cyl(METAL, at(0.0, -0.001, zz - z), at(0.0, 0.05, zz - z), 0.009);
                }
                b.cyl(METAL, at(0.0, 0.05, zb - z), at(0.0, 0.05, zt - z), 0.0125);
            }
            _ => {
                // A lever: on a rose, on a long plate, or straight.
                if kind == 3 {
                    let lo = at(-0.024, -0.0005, -0.16);
                    let hi = at(0.024, 0.006, 0.07);
                    let (ya, yb) = if side > 0.0 {
                        (lo[1], hi[1])
                    } else {
                        (hi[1], lo[1])
                    };
                    b.rbox(METAL, [lo[0], ya, lo[2]], [hi[0], yb, hi[2]], 0.004);
                    b.cyl(METAL, at(0.0, 0.005, -0.09), at(0.0, 0.008, -0.09), 0.008);
                } else {
                    rose(b, at(0.0, 0.0, 0.0), side, 0.026);
                    rose(b, at(0.0, 0.0, -0.08), side, 0.016);
                }
                let off = if kind == 3 { 0.006 } else { 0.008 };
                if kind == 4 {
                    // Straight and square: out, then across.
                    bar(
                        b,
                        METAL,
                        at(0.0, off, 0.0),
                        at(0.0, off + 0.06, 0.0),
                        0.018,
                        [0.0, 0.0, 1.0],
                    );
                    bar(
                        b,
                        METAL,
                        at(0.0, off + 0.051, 0.0),
                        at(inward * 0.13, off + 0.051, 0.0),
                        0.018,
                        [0.0, 0.0, 1.0],
                    );
                } else {
                    let path = [
                        at(0.0, off, 0.0),
                        at(0.0, off + 0.045, 0.0),
                        at(inward * 0.012, off + 0.058, 0.0),
                        at(inward * 0.06, off + 0.06, 0.0),
                        at(inward * 0.125, off + 0.052, -0.004),
                    ];
                    sweep(
                        b,
                        METAL,
                        &path,
                        |s| 0.0095 - 0.002 * s,
                        12,
                        Cap::Flat,
                        Cap::Round(0.8),
                    );
                }
            }
        }
    }
}

/// A panic bar across a leaf from `a` to `c` on the side it is pushed
/// from (-y): the bar on two brackets, the box of its latch at the free
/// edge (`inward`: towards the hinges).
fn panic_bar(b: &mut Builder, a: f64, c: f64, inward: f64, t: f64) {
    let y = -t / 2.0;
    let z = 0.98;
    let (x_free, x_hinge) = if inward < 0.0 { (c, a) } else { (a, c) };
    let xa = x_free + inward * 0.06;
    let xc = x_hinge - inward * 0.08;
    // The latch box at the free edge and the bracket at the hinge side.
    for (x0, x1) in [
        sorted2(xa, xa + inward * 0.17),
        sorted2(xc, xc - inward * 0.06),
    ] {
        b.rbox(
            METAL,
            [x0, y - 0.075, z - 0.04],
            [x1, y + 0.0005, z + 0.04],
            0.008,
        );
    }
    // The bar between them.
    b.cyl(
        METAL,
        [xa + inward * 0.16, y - 0.055, z],
        [xc - inward * 0.05, y - 0.055, z],
        0.017,
    );
}

/// A door closer on the face the leaf opens to, near its hinges at
/// `hinge_x` (`inward`: from the free edge towards them): its body on the
/// leaf by the top and its arm to a shoe on the head of the frame, on the
/// wall's face at `face`.
fn closer(b: &mut Builder, hinge_x: f64, inward: f64, top: f64, t: f64, face: f64) {
    let y = t / 2.0;
    let out = -inward;
    let (x0, x1) = sorted2(hinge_x + out * 0.05, hinge_x + out * 0.33);
    b.rbox(
        METAL,
        [x0, y - 0.0005, top - 0.13],
        [x1, y + 0.055, top - 0.07],
        0.008,
    );
    // The pivot near the hinge end, the arm folded out and up to the shoe.
    let pivot = [hinge_x + out * 0.09, y + 0.06, top - 0.1];
    let shoe_y = face.max(y + 0.07);
    let shoe = [hinge_x + out * 0.28, shoe_y - 0.012, top + 0.035];
    let elbow = [
        hinge_x + out * 0.2,
        (pivot[1] + shoe[1]) / 2.0 + 0.04,
        top - 0.06,
    ];
    b.cyl(METAL, [pivot[0], y + 0.054, pivot[2]], pivot, 0.008);
    bar(b, METAL, pivot, elbow, 0.014, [0.0, 0.0, 1.0]);
    bar(b, METAL, elbow, shoe, 0.012, [0.0, 0.0, 1.0]);
    let (sx0, sx1) = sorted2(shoe[0] - 0.035, shoe[0] + 0.035);
    b.rbox(
        METAL,
        [sx0, shoe_y - 0.02, top + 0.012],
        [sx1, shoe_y + 0.0005, top + 0.055],
        0.004,
    );
}

/// A round rose on the face at `c`, standing towards `side`.
fn rose(b: &mut Builder, c: P3, side: f64, r: f64) {
    b.cone(
        METAL,
        [c[0], c[1] - side * 0.0005, c[2]],
        [c[0], c[1] + side * 0.007, c[2]],
        r,
        r * 0.92,
        24,
    );
}

/// A spyhole through the leaf at `x`, `z`.
fn spyhole(b: &mut Builder, x: f64, z: f64, t: f64) {
    b.cyl(
        METAL,
        [x, -t / 2.0 - 0.004, z],
        [x, t / 2.0 + 0.004, z],
        0.007,
    );
}

/// The guides a sliding door hangs from, by name (`slide_guide`).
pub const SLIDE_GUIDES: &[&str] = &[
    "Como la colección",
    "Carril visto",
    "Carril con galería",
    "Carril oculto",
];

/// The pulls of a sliding door, by name (`slide_pull`).
pub const SLIDE_PULLS: &[&str] = &[
    "Como la colección",
    "Uñero embutido",
    "Tirador de barra",
    "Asa de granero",
    "Sin tirador",
];

/// Each collection's guide and pull (numbers in the lists above).
const SLIDE_LOOKS: [(usize, usize); 8] = [
    (3, 2), // Nórdico cálido: a galería, a recessed pull.
    (3, 4), // Mediterráneo: a galería, a barn handle.
    (3, 3), // Años 50: a galería, a pull bar.
    (4, 2), // Japandi: hidden, a recessed pull.
    (4, 2), // Contemporáneo: hidden, a recessed pull.
    (2, 4), // Rústica: a barn rail, a barn handle.
    (2, 3), // Industrial: a barn rail, a pull bar.
    (3, 2), // Clásica: a galería, a recessed pull.
];

/// A sliding door (paquete 9, correderas): over the face of the wall,
/// hanging from a guide (`pocket` off), or into the wall (`pocket` on);
/// open `slide_open` per cent towards `slide_dir` (±1), with the leaf,
/// lining and casings of the hinged door's collection and a sliding
/// door's pull. The family's outputs: `leaf_x0`, `leaf_x1`, `leaf_top`,
/// `leaf_t`, `lining_w`, `lining_h`, `pocket`, `slide_dir`.
pub fn sliding_door(b: &mut Builder, p: &Params) {
    let l = look(p);
    let style = (p.style() as usize).clamp(1, 8);
    let (mut guide, mut pull) = SLIDE_LOOKS[style - 1];
    let pick = |key: &str| {
        let n = p.get(key, 1.0).round() as usize;
        (n >= 2).then_some(n)
    };
    if let Some(n) = pick("slide_guide") {
        guide = n;
    }
    if let Some(n) = pick("slide_pull") {
        pull = n;
    }
    let far = p.flag("lod", false);
    let x0 = p.get("leaf_x0", -0.4);
    let x1 = p.get("leaf_x1", 0.4);
    let top = p.get("leaf_top", 2.0);
    let t = p.get("leaf_t", 0.04).clamp(0.02, 0.06);
    let w = p.get("lining_w", x1 - x0 + 0.1);
    let h = p.get("lining_h", top + 0.05);
    let fw = p.get("frame_width", 0.05);
    let fd = p.get("frame_depth", 0.07);
    let pocket = p.flag("pocket", false);
    let s = if p.get("slide_dir", 1.0) < 0.0 {
        -1.0
    } else {
        1.0
    };
    let open = (p.get("slide_open", 0.0) / 100.0).clamp(0.0, 1.0);
    let face = p.get("face_pos", fd / 2.0);
    let back = p.get("face_neg", -fd / 2.0);
    let design = design_of(l.design, p);
    let lw = x1 - x0;
    // The lining across the whole reveal, and the casings: on both faces
    // of a pocket door; on the face away from the leaf of one that slides
    // over the wall (the leaf covers the other).
    let profile = if far && l.casing != 2 {
        casing(3)
    } else {
        casing(l.casing)
    };
    {
        let (lo, hi) = (back, face);
        b.rbox(FRAME, [-w / 2.0, lo, 0.0], [-w / 2.0 + fw, hi, h], 0.0);
        b.rbox(FRAME, [w / 2.0 - fw, lo, 0.0], [w / 2.0, hi, h], 0.0);
        b.rbox(
            FRAME,
            [-w / 2.0 + fw, lo, h - fw],
            [w / 2.0 - fw, hi, h],
            0.0,
        );
    }
    if let Some(pr) = &profile {
        let xi = w / 2.0 - fw + 0.008;
        let zi = h - fw + 0.008;
        let path = [[-xi, 0.0], [-xi, zi], [xi, zi], [xi, 0.0]];
        moulding(b, FRAME, &path, false, back, -1.0, pr);
        if pocket {
            moulding(b, FRAME, &path, false, face, 1.0, pr);
        }
    }
    let floor = 0.008;
    if pocket {
        // In the plane of the lining, slid into the wall.
        let shift = s * open * lw;
        let (a, c) = (x0 + 0.002 + shift, x1 - 0.002 + shift);
        if far {
            leaf_plain(b, &design, a, c, floor, top - 0.003, t);
        } else {
            leaf(b, &design, a, c, floor, top - 0.003, t);
        }
        if !far {
            // Flush pulls on both faces by the leading edge, and the pull
            // in its edge.
            // The edge left out of the wall.
            let edge = if s > 0.0 { a } else { c };
            let x = edge + s * 0.05;
            for side in [1.0, -1.0] {
                let (ya, yb) = sorted2(side * (t / 2.0 - 0.0005), side * (t / 2.0 + 0.002));
                b.rbox(METAL, [x - 0.02, ya, 0.95], [x + 0.02, yb, 1.15], 0.006);
            }
            let (ea, ec) = sorted2(edge - s * 0.002, edge + s * 0.004);
            b.rbox(METAL, [ea, -0.008, 0.98], [ec, 0.008, 1.12], 0.002);
        }
        if p.flag("mirror", false) {
            mirror_x(b);
        }
        return;
    }
    // Over the face of the wall: the leaf a little past the opening on
    // both sides, off the wall, up past the head.
    let gap = 0.012;
    let over = 0.05;
    let shift = s * open * (lw + over);
    let (a, c) = (x0 - over + shift, x1 + over + shift);
    let leaf_top = h + 0.03;
    let yl = face + gap + t / 2.0;
    let start = b.mark();
    if far {
        leaf_plain(b, &design, a, c, floor, leaf_top, t);
    } else {
        leaf(b, &design, a, c, floor, leaf_top, t);
    }
    b.transform_from(start, |q| [q[0], q[1] + yl, q[2]]);
    // The header board over the opening the guide is fixed to, along all
    // the way the leaf runs.
    let (ra, rc) = sorted2(x0 - over - s * 0.1, x1 + over + s * (lw + over + 0.1));
    let zr = leaf_top + 0.05;
    b.rbox(
        FRAME,
        [ra, face - 0.0005, h - 0.03],
        [rc, face + 0.02, zr + 0.06],
        0.0,
    );
    let yr = face + 0.02;
    match guide {
        3 | 4 => {
            if guide == 3 {
                // The galería: a box in front of the rail, its ends closed.
                let yf = yl + t / 2.0 + 0.02;
                b.rbox(
                    FRAME,
                    [ra, yf, leaf_top - 0.01],
                    [rc, yf + 0.018, zr + 0.06],
                    0.002,
                );
                b.rbox(FRAME, [ra, yr, zr + 0.045], [rc, yf, zr + 0.06], 0.0);
                for x in [ra, rc - 0.018] {
                    b.rbox(
                        FRAME,
                        [x, yr, leaf_top - 0.01],
                        [x + 0.018, yf, zr + 0.06],
                        0.0,
                    );
                }
            } else {
                // Hidden: a slim profile on the board the leaf runs under.
                b.rbox(
                    METAL,
                    [ra, yr, leaf_top + 0.004],
                    [rc, yl + t / 2.0 + 0.004, zr],
                    0.002,
                );
            }
            // The leaf hangs from it (from the galería's top).
            let up = if guide == 3 { zr + 0.046 } else { zr };
            for k in [0.15, 0.85] {
                let x = a + (c - a) * k;
                b.rbox(
                    METAL,
                    [x - 0.02, yl - 0.008, leaf_top - 0.001],
                    [x + 0.02, yl + 0.008, up],
                    0.0,
                );
            }
        }
        _ => {
            // A barn door's rail: a flat bar on spacers, the leaf on two
            // straps with wheels.
            let yb = yr + 0.03;
            let n = (((rc - ra) / 0.6).ceil() as usize).max(2);
            for i in 0..=n {
                let x = ra + 0.04 + (rc - ra - 0.08) * i as f64 / n as f64;
                b.cyl(METAL, [x, yr - 0.001, zr], [x, yb, zr], 0.011);
            }
            b.rbox(
                METAL,
                [ra, yb - 0.0005, zr - 0.02],
                [rc, yb + 0.008, zr + 0.02],
                0.002,
            );
            if !far {
                for k in [0.18, 0.82] {
                    let x = a + (c - a) * k;
                    let wy = yb + 0.022;
                    b.cyl(
                        METAL,
                        [x, yb + 0.008, zr + 0.035],
                        [x, yb + 0.036, zr + 0.035],
                        0.042,
                    );
                    let (sa, sb) = sorted2(yl + t / 2.0 - 0.0005, wy + 0.006);
                    b.rbox(
                        METAL,
                        [x - 0.02, sa, leaf_top - 0.22],
                        [x + 0.02, sb, zr + 0.035],
                        0.003,
                    );
                }
            }
        }
    }
    if !far {
        // The pull by the edge it closes with, on both faces.
        let edge = if s > 0.0 { a } else { c };
        let x = edge + s * 0.08;
        for side in [1.0, -1.0] {
            let fy = yl + side * t / 2.0;
            match pull {
                5 => {}
                3 | 4 => {
                    let (zb, zt) = if pull == 3 {
                        (0.75, 1.35)
                    } else {
                        (0.92, 1.18)
                    };
                    let r = if pull == 3 { 0.011 } else { 0.009 };
                    let o = side * 0.055;
                    for z in [zb + 0.02, zt - 0.02] {
                        b.cyl(METAL, [x, fy - side * 0.0005, z], [x, fy + o, z], r * 0.8);
                    }
                    b.cyl(METAL, [x, fy + o, zb], [x, fy + o, zt], r);
                }
                _ => {
                    let (ya, yb) = sorted2(fy - side * 0.0005, fy + side * 0.002);
                    b.rbox(METAL, [x - 0.025, ya, 0.92], [x + 0.025, yb, 1.18], 0.008);
                }
            }
        }
    }
    if p.flag("mirror", false) {
        mirror_x(b);
    }
}

/// A folding door (de librillo): `leaves` leaves of the collection hung
/// from a track in the head, hinged to each other, gathering towards the
/// jamb at `hinge_left` (the hand); open `fold_open` per cent, they fold
/// in a zigzag towards the side they open to (+y). The family's outputs:
/// `leaf_x0`, `leaf_x1`, `leaf_top`, `leaf_t`, `lining_w`, `lining_h`.
pub fn folding_door(b: &mut Builder, p: &Params) {
    let mut l = look(p);
    // A folding leaf takes a knob or a pull, not a panic bar or a plate.
    if l.handle >= 8 {
        l.handle = 6;
    }
    let far = p.flag("lod", false);
    let x0 = p.get("leaf_x0", -0.8);
    let x1 = p.get("leaf_x1", 0.8);
    let top = p.get("leaf_top", 2.0);
    let t = p.get("leaf_t", 0.03).clamp(0.018, 0.045);
    let w = p.get("lining_w", x1 - x0 + 0.1);
    let h = p.get("lining_h", top + 0.05);
    let fw = p.get("frame_width", 0.05);
    let fd = p.get("frame_depth", 0.07);
    let n = p.get("leaves", 4.0).round().clamp(2.0, 8.0) as usize;
    let s = if p.flag("hinge_left", true) {
        1.0
    } else {
        -1.0
    };
    let hz = p.get("handle_height", 1.02).clamp(0.3, 2.0);
    let open = (p.get("fold_open", 0.0) / 100.0).clamp(0.0, 1.0);
    let angle = open * 1.35;
    let design = design_of(l.design, p);
    let lw = (x1 - x0) / n as f64;
    let start_x = if s > 0.0 { x0 } else { x1 };
    // The track along the head, the leaves hanging from it.
    let rail = 0.028;
    b.rbox(METAL, [x0, -0.016, top - rail], [x1, 0.016, top], 0.002);
    let (z0, z1) = (0.01, top - rail - 0.004);
    let mut at = [start_x, 0.0];
    for i in 0..n {
        let up = if i % 2 == 0 { 1.0 } else { -1.0 };
        let d = [s * angle.cos(), up * angle.sin()];
        let perp = [-d[1], d[0]];
        let mark = b.mark();
        let (a, c) = (0.0015, lw - 0.0015);
        if far {
            leaf_plain(b, &design, a, c, z0, z1, t);
        } else {
            leaf(b, &design, a, c, z0, z1, t);
            if i + 1 == n && l.handle != 7 {
                // The handle by the free edge of the last leaf (its x
                // grows away from the gathered stack).
                handle_on(b, l.handle, c - 0.06, hz, t, -1.0, handle_faces(p));
            }
        }
        // The leaf's hangers in the track: the joints on the line of the
        // head (every other one), and the first leaf's pivot.
        if i % 2 == 0 {
            b.cyl(
                METAL,
                [0.0, 0.0, z1 - 0.01],
                [0.0, 0.0, top - rail + 0.001],
                0.008,
            );
            b.cyl(METAL, [0.0, 0.0, 0.0], [0.0, 0.0, z0 + 0.02], 0.008);
        }
        let o = at;
        b.transform_from(mark, |q| {
            [
                o[0] + d[0] * q[0] + perp[0] * q[1],
                o[1] + d[1] * q[0] + perp[1] * q[1],
                q[2],
            ]
        });
        at = [o[0] + d[0] * lw, o[1] + d[1] * lw];
        if i + 1 < n && !far {
            // The hinges between this leaf and the next, round the joint.
            let tall = z1 - z0;
            for z in [z0 + 0.2, z0 + tall / 2.0, z1 - 0.2] {
                b.cyl(
                    METAL,
                    [at[0], at[1], z - 0.04],
                    [at[0], at[1], z + 0.04],
                    t / 2.0 + 0.003,
                );
            }
        }
    }
    let _ = (fw, fd);
    lining_and_casings(b, p, &l, w, h, fw, fd, far);
    if p.flag("mirror", false) {
        mirror_x(b);
    }
}

/// The looks of a garage door's leaf (`garage_look`).
pub const GARAGE_LOOKS: &[&str] = &[
    "Según el funcionamiento",
    "Acanalada",
    "Lisa",
    "Cuarterones",
    "Lamas de madera",
    "Con ventanas",
    "Nervada vertical",
];

/// A garage door (`garage_kind` 1 sectional, 2 up-and-over, 3 rolling)
/// between `leaf_x0` and `leaf_x1`, up to `leaf_top`, with its guides,
/// its look (`garage_look`), a wicket door (`wicket`, `wicket_width`,
/// `wicket_left`) and open `garage_open` per cent. It hangs on the side
/// it opens to (+y); the street is at −y.
pub fn garage_door(b: &mut Builder, p: &Params) {
    let far = p.flag("lod", false);
    let kind = p.get("garage_kind", 1.0).round() as i32;
    let c0 = p.get("leaf_x0", -1.2);
    let c1 = p.get("leaf_x1", 1.2);
    let top = p.get("leaf_top", 2.1);
    let h = p.get("lining_h", top + 0.1);
    let fd = p.get("frame_depth", 0.1);
    let face = p.get("face_pos", fd / 2.0).max(fd / 2.0);
    let open = (p.get("garage_open", 0.0) / 100.0).clamp(0.0, 1.0);
    let mut look = p.get("garage_look", 1.0).round() as i32;
    if look < 2 {
        look = match kind {
            2 => 7,
            3 => 2,
            _ => 2,
        };
    }
    let t = 0.042;
    let wicket = p.flag("wicket", false) && kind != 3;
    let ww = p.get("wicket_width", 0.8);
    let wx = if p.flag("wicket_left", true) {
        c0 + 0.3
    } else {
        c1 - 0.3 - ww
    };
    let wz = (top - 0.1).min(2.0);
    match kind {
        3 => {
            // Rolling: the curtain of slats down from the drum, in its
            // guides, with the bottom bar.
            let zb = open * (top - 0.15);
            let slat = 0.077;
            let (ya, yc) = (-0.009, 0.009);
            b.rbox(
                LEAF,
                [c0 + 0.012, ya, zb + 0.05],
                [c1 - 0.012, yc, top],
                0.0,
            );
            if !far {
                let mut z = zb + 0.05 + slat;
                while z < top - 0.01 {
                    for side in [1.0, -1.0] {
                        let (y0, y1) = sorted2(side * 0.009, side * 0.0115);
                        b.rbox(
                            LEAF,
                            [c0 + 0.014, y0, z - 0.004],
                            [c1 - 0.014, y1, z + 0.004],
                            0.001,
                        );
                    }
                    z += slat;
                }
            }
            b.rbox(
                METAL,
                [c0 + 0.012, -0.016, zb],
                [c1 - 0.012, 0.016, zb + 0.05],
                0.004,
            );
            for (xa, xc) in [(c0 - 0.03, c0 + 0.03), (c1 - 0.03, c1 + 0.03)] {
                b.rbox(METAL, [xa, -0.03, 0.0], [xc, -0.012, top], 0.002);
                b.rbox(METAL, [xa, 0.012, 0.0], [xc, 0.03, top], 0.002);
                b.rbox(METAL, [xa, -0.03, 0.0], [xa + 0.012, 0.03, top], 0.0);
            }
            // The drum's box over the opening, closed on the street side.
            b.rbox(
                FRAME,
                [c0 - 0.03, -fd / 2.0 + 0.005, top],
                [c1 + 0.03, fd / 2.0, h - 0.005],
                0.01,
            );
            if !far {
                // A lock in the bottom bar.
                b.cyl(
                    METAL,
                    [0.0, -0.016, zb + 0.025],
                    [0.0, -0.022, zb + 0.025],
                    0.012,
                );
            }
        }
        2 => {
            // Up-and-over: one leaf that tips up on arms, with its
            // counterweights in boxes by the jambs, on the inside.
            let phi = open * std::f64::consts::FRAC_PI_2;
            let pivot_z = top * 0.62;
            let lift = open * (top - pivot_z - 0.05);
            let mark = b.mark();
            let wk = wicket.then_some(Wicket {
                x: wx,
                w: ww,
                bottom: 0.1,
                top: wz,
                handle: 1.0,
            });
            // (No row of windows through the wicket door.)
            let look = if wicket && look == 6 { 2 } else { look };
            garage_panel(
                b,
                look,
                c0 + 0.008,
                c1 - 0.008,
                0.01,
                top - 0.008,
                t,
                far,
                wk,
            );
            if !far && !wicket {
                // The lock and handle in the middle, on the street side
                // (lower down when the windows are there).
                let z = if look == 6 { 0.5 } else { 1.0 };
                handle_on(b, 4, (c0 + c1) / 2.0, z, t, 1.0, &[-1.0]);
            }
            b.transform_from(mark, |q| {
                let (y, z) = (q[1], q[2] - pivot_z);
                let (sn, cs) = phi.sin_cos();
                [q[0], y * cs + z * sn, pivot_z + lift + z * cs - y * sn]
            });
            for (xa, xc) in [(c0 - 0.16, c0 - 0.02), (c1 + 0.02, c1 + 0.16)] {
                b.rbox(FRAME, [xa, face, 0.0], [xc, face + 0.09, top], 0.004);
            }
            // The arms from the boxes to the leaf's pivots, and the head
            // channel the top of the leaf runs in.
            for x in [c0 - 0.01, c1 + 0.01] {
                b.cyl(
                    METAL,
                    [x, face + 0.045, pivot_z],
                    [x, 0.0, pivot_z + lift],
                    0.01,
                );
            }
            for x in [c0 - 0.02, c1 - 0.03] {
                b.rbox(
                    METAL,
                    [x, t / 2.0, top - 0.05],
                    [x + 0.05, face + 0.6, top - 0.02],
                    0.002,
                );
            }
        }
        _ => {
            // Sectional: the panels up their guides and, once past the
            // head, flat along the tracks under it.
            let n = p.get("panels", 4.0).round().clamp(2.0, 10.0) as usize;
            let gap = 0.008;
            let ph = (top - gap * (n as f64 - 1.0)) / n as f64;
            let travel = open * (top - 0.15);
            let bend = top - 0.08;
            for i in 0..n {
                let zb = i as f64 * (ph + gap) + travel;
                let mark = b.mark();
                // The windows in the panel second from the top.
                let lk = match look {
                    6 if i + 2 == n => 6,
                    6 => 2,
                    other => other,
                };
                let pb = zb - travel;
                let wk = wicket.then_some(Wicket {
                    x: wx,
                    w: ww,
                    bottom: 0.1 - pb,
                    top: wz - pb,
                    handle: 1.0 - pb,
                });
                garage_panel(b, lk, c0 + 0.01, c1 - 0.01, 0.0, ph, t, far, wk);
                let mid = zb + ph / 2.0;
                if mid <= bend {
                    b.transform_from(mark, |q| [q[0], q[1], q[2] + zb]);
                } else {
                    // Turned flat, along the tracks into the room.
                    let along = mid - bend;
                    b.transform_from(mark, |q| {
                        [
                            q[0],
                            t / 2.0 + along + (q[2] - ph / 2.0),
                            bend + 0.02 - q[1],
                        ]
                    });
                }
            }
            if !far && !wicket {
                // A grip at the bottom on the street side.
                b.rbox(
                    METAL,
                    [-0.08, -t / 2.0 - 0.025, travel + 0.12],
                    [0.08, -t / 2.0, travel + 0.15],
                    0.006,
                );
            }
            // The guides up the jambs and the tracks under the head, and
            // the spring shaft above the opening on the inside.
            let reach = face + top * 0.9;
            for (xa, xc) in [(c0 - 0.02, c0 + 0.03), (c1 - 0.03, c1 + 0.02)] {
                b.rbox(
                    METAL,
                    [xa, t / 2.0, 0.0],
                    [xc, t / 2.0 + 0.03, bend + 0.06],
                    0.002,
                );
                b.rbox(
                    METAL,
                    [xa, t / 2.0, bend + 0.03],
                    [xc, reach, bend + 0.06],
                    0.002,
                );
            }
            let ys = face + 0.07;
            b.cyl(
                METAL,
                [c0 - 0.02, ys, bend + 0.16],
                [c1 + 0.02, ys, bend + 0.16],
                0.016,
            );
            for x in [c0 - 0.02, c1 - 0.03] {
                b.rbox(
                    METAL,
                    [x, ys - 0.03, bend + 0.06],
                    [x + 0.05, ys + 0.03, bend + 0.2],
                    0.002,
                );
            }
        }
    }
    if p.flag("mirror", false) {
        mirror_x(b);
    }
}

/// A wicket door in a garage door's leaf, in the panel's own heights.
#[derive(Clone, Copy)]
struct Wicket {
    x: f64,
    w: f64,
    bottom: f64,
    top: f64,
    handle: f64,
}

/// One panel of a garage door (or its single leaf), from `a` to `c` and
/// `z0` to `z1`, `t` thick, in the look `look` (numbers of
/// [`GARAGE_LOOKS`]); the part of a `wicket` in it is outlined, with its
/// handle.
#[allow(clippy::too_many_arguments)]
fn garage_panel(
    b: &mut Builder,
    look: i32,
    a: f64,
    c: f64,
    z0: f64,
    z1: f64,
    t: f64,
    far: bool,
    wicket: Option<Wicket>,
) {
    let h2 = t / 2.0;
    if look == 6 && !far {
        // A row of windows across the middle of the panel.
        let n = (((c - a) / 0.62).floor() as usize).max(2);
        let wide = (c - a) / n as f64;
        let (wa, wc) = (z0 + (z1 - z0) * 0.28, z1 - (z1 - z0) * 0.28);
        b.rbox(LEAF, [a, -h2, z0], [c, h2, wa], 0.0);
        b.rbox(LEAF, [a, -h2, wc], [c, h2, z1], 0.0);
        for k in 0..n {
            let x = a + k as f64 * wide;
            let (xa, xc) = (x + wide * 0.2, x + wide * 0.8);
            b.rbox(LEAF, [x, -h2, wa], [xa, h2, wc], 0.0);
            b.rbox(LEAF, [xc, -h2, wa], [x + wide, h2, wc], 0.0);
            b.rbox(GLASS, [xa, -0.003, wa], [xc, 0.003, wc], 0.0);
            for side in [1.0, -1.0] {
                let (y0, y1) = sorted2(side * h2, side * (h2 + 0.004));
                for (p0, p1) in [
                    ([xa - 0.012, y0, wa - 0.012], [xc + 0.012, y1, wa]),
                    ([xa - 0.012, y0, wc], [xc + 0.012, y1, wc + 0.012]),
                    ([xa - 0.012, y0, wa], [xa, y1, wc]),
                    ([xc, y0, wa], [xc + 0.012, y1, wc]),
                ] {
                    b.rbox(METAL, p0, p1, 0.0);
                }
            }
        }
    } else {
        b.rbox(LEAF, [a, -h2, z0], [c, h2, z1], 0.002);
    }
    if !far {
        let ribs = |b: &mut Builder, za: f64, zc: f64| {
            for side in [1.0, -1.0] {
                let (y0, y1) = sorted2(side * h2, side * (h2 + 0.003));
                b.rbox(LEAF, [a + 0.005, y0, za], [c - 0.005, y1, zc], 0.001);
            }
        };
        match look {
            2 => {
                // Ribbed: two shallow ribs across each panel.
                for k in 1..3 {
                    let z = z0 + (z1 - z0) * k as f64 / 3.0;
                    ribs(b, z - 0.006, z + 0.006);
                }
            }
            3 => {
                // Cassettes: raised plates side by side.
                let n = (((c - a) / 0.55).round() as usize).max(2);
                let wide = (c - a) / n as f64;
                for k in 0..n {
                    let x = a + k as f64 * wide;
                    for side in [1.0, -1.0] {
                        let (y0, y1) = sorted2(side * h2, side * (h2 + 0.006));
                        b.rbox(
                            LEAF,
                            [x + 0.06, y0, z0 + 0.06],
                            [x + wide - 0.06, y1, z1 - 0.06],
                            0.004,
                        );
                    }
                }
            }
            5 => {
                // Timber boards: a board across every 15 cm.
                let mut z = z0 + 0.15;
                while z < z1 - 0.02 {
                    ribs(b, z - 0.004, z + 0.002);
                    z += 0.15;
                }
            }
            7 => {
                // Ribbed upright: ribs every 12 cm.
                let n = ((c - a) / 0.12).floor() as usize;
                for k in 1..n {
                    let x = a + k as f64 * (c - a) / n as f64;
                    for side in [1.0, -1.0] {
                        let (y0, y1) = sorted2(side * h2, side * (h2 + 0.008));
                        b.rbox(
                            LEAF,
                            [x - 0.012, y0, z0 + 0.01],
                            [x + 0.012, y1, z1 - 0.01],
                            0.002,
                        );
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(k) = wicket {
        if far {
            return;
        }
        // The wicket's outline on both faces, its hinges and its lever.
        let (zb, zt) = (k.bottom.max(z0), k.top.min(z1));
        if zt <= zb {
            return;
        }
        let (x, w) = (k.x, k.w);
        for side in [1.0, -1.0] {
            let (y0, y1) = sorted2(side * h2, side * (h2 + 0.007));
            b.rbox(METAL, [x - 0.02, y0, zb], [x, y1, zt], 0.0);
            b.rbox(METAL, [x + w, y0, zb], [x + w + 0.02, y1, zt], 0.0);
            if k.top < z1 {
                b.rbox(
                    METAL,
                    [x - 0.02, y0, zt],
                    [x + w + 0.02, y1, zt + 0.02],
                    0.0,
                );
            }
            if k.bottom > z0 {
                b.rbox(
                    METAL,
                    [x - 0.02, y0, zb - 0.02],
                    [x + w + 0.02, y1, zb],
                    0.0,
                );
            }
        }
        if k.handle > z0 + 0.1 && k.handle < z1 - 0.1 {
            handle(b, 2, x + w - 0.07, k.handle, t, -1.0);
        }
    }
}
