//! The facilities and the commerce of the world's places (tandas 10 and
//! 11): the town hall, the school, the library, the fire and police
//! stations, the sports hall, the gym, the stadium and the factory; the
//! kiosk, the shop, the street market, the shopping arcade and the mall;
//! the beach bar, the restaurant and the tavern; the square and the
//! landmark of each place. Those that are a building of the street wear
//! the place's skin through [`building`]; the halls, the stands, the stalls
//! and the landmarks are drawn here with the place's colours.

use super::super::more::{plate, turn_from};
use super::super::*;
use super::plan::{building, Built, Plan, Shape, Side};
use super::roofs;
use super::skins::*;
use super::types::adapted;
use super::types::Typology;
use super::{look, Look};
use crate::build::{Builder, P3};
use std::f64::consts::PI;

/// What a facility is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Programme {
    TownHall,
    School,
    Library,
    FireStation,
    Police,
    SportsHall,
    Gym,
    Stadium,
    Factory,
    Kiosk,
    Shop,
    StreetMarket,
    Arcade,
    Mall,
    BeachBar,
    Restaurant,
    Tavern,
    Square,
    Landmark,
}

/// The programmes in their order (their number is kept in the recipes).
pub const PROGRAMMES: [Programme; 19] = [
    Programme::TownHall,
    Programme::School,
    Programme::Library,
    Programme::FireStation,
    Programme::Police,
    Programme::SportsHall,
    Programme::Gym,
    Programme::Stadium,
    Programme::Factory,
    Programme::Kiosk,
    Programme::Shop,
    Programme::StreetMarket,
    Programme::Arcade,
    Programme::Mall,
    Programme::BeachBar,
    Programme::Restaurant,
    Programme::Tavern,
    Programme::Square,
    Programme::Landmark,
];

impl Programme {
    pub fn of(n: f64) -> Programme {
        PROGRAMMES[(n.round().max(0.0) as usize).min(PROGRAMMES.len() - 1)]
    }

    pub fn index(self) -> usize {
        PROGRAMMES.iter().position(|p| *p == self).unwrap_or(0)
    }

    /// Its name for Antonio.
    pub fn name(self) -> &'static str {
        match self {
            Programme::TownHall => "Ayuntamiento",
            Programme::School => "Colegio",
            Programme::Library => "Biblioteca",
            Programme::FireStation => "Parque de bomberos",
            Programme::Police => "Comisaría",
            Programme::SportsHall => "Polideportivo",
            Programme::Gym => "Gimnasio",
            Programme::Stadium => "Estadio",
            Programme::Factory => "Fábrica",
            Programme::Kiosk => "Quiosco",
            Programme::Shop => "Tienda",
            Programme::StreetMarket => "Mercadillo",
            Programme::Arcade => "Galería comercial",
            Programme::Mall => "Centro comercial",
            Programme::BeachBar => "Chiringuito",
            Programme::Restaurant => "Restaurante",
            Programme::Tavern => "Taberna",
            Programme::Square => "Plaza",
            Programme::Landmark => "Edificio emblemático",
        }
    }

    /// Its usual width and depth.
    pub fn size(self, sk: &Skin) -> (f64, f64) {
        match self {
            Programme::TownHall => (odd_width(sk, 18.0), 16.0),
            Programme::School => (36.0, 30.0),
            Programme::Library => (odd_width(sk, 22.0), 16.0),
            Programme::FireStation => (26.0, 22.0),
            Programme::Police => (odd_width(sk, 20.0), 14.0),
            Programme::SportsHall => (44.0, 30.0),
            Programme::Gym => (16.0, 16.0),
            Programme::Stadium => (140.0, 108.0),
            Programme::Factory => (40.0, 34.0),
            Programme::Kiosk => (3.2, 3.2),
            Programme::Shop => (super::types::usual_width(sk).max(6.0), 12.0),
            Programme::StreetMarket => (30.0, 14.0),
            Programme::Arcade => (16.0, 40.0),
            Programme::Mall => (90.0, 100.0),
            Programme::BeachBar => (14.0, 12.0),
            Programme::Restaurant => (14.0, 16.0),
            Programme::Tavern => (super::types::usual_width(sk).max(6.0), 12.0),
            Programme::Square => (32.0, 32.0),
            Programme::Landmark => (24.0, 30.0),
        }
    }

    /// Its usual floors.
    pub fn floors(self, sk: &Skin) -> usize {
        match self {
            Programme::TownHall => 3,
            Programme::School => 2,
            Programme::Library => 2,
            Programme::FireStation => 2,
            Programme::Police => 3,
            Programme::Gym => 2,
            Programme::Shop | Programme::Tavern => sk.floors.clamp(1, 4),
            Programme::Restaurant => 1,
            Programme::Arcade => 2,
            _ => 1,
        }
    }
}

/// A width that gives a front an odd number of axes, its door in the
/// middle.
pub fn odd_width(sk: &Skin, about: f64) -> f64 {
    let n = ((about - 1.0) / sk.bay).round().max(3.0) as usize;
    let n = if n.is_multiple_of(2) { n + 1 } else { n };
    n as f64 * sk.bay + 1.0
}

/// A facility of `skin` over a `w` × `d` plot, its front to the street
/// (y = `d`); `today` for the place as built now. Gives the look that
/// colours its walls, trims and roof.
#[allow(clippy::too_many_arguments)]
pub fn facility(
    b: &mut Builder,
    p: Programme,
    sk: &Skin,
    w: f64,
    d: f64,
    floors: usize,
    seed: u64,
    light: bool,
    today: bool,
) -> Look {
    let c = Ctx {
        sk,
        w,
        d,
        floors: floors.max(1),
        seed,
        light,
        today,
    };
    match p {
        Programme::TownHall => town_hall(b, &c),
        Programme::School => school(b, &c),
        Programme::Library => library(b, &c),
        Programme::FireStation => fire_station(b, &c),
        Programme::Police => police(b, &c),
        Programme::SportsHall => sports_hall(b, &c),
        Programme::Gym => gym(b, &c),
        Programme::Stadium => stadium(b, &c),
        Programme::Factory => factory(b, &c),
        Programme::Kiosk => kiosk(b, &c),
        Programme::Shop => shop(b, &c),
        Programme::StreetMarket => street_market(b, &c),
        Programme::Arcade => arcade(b, &c),
        Programme::Mall => mall(b, &c),
        Programme::BeachBar => beach_bar(b, &c),
        Programme::Restaurant => restaurant(b, &c),
        Programme::Tavern => tavern(b, &c),
        Programme::Square => square(b, &c),
        Programme::Landmark => super::landmarks::landmark(b, sk, w, d, seed, light),
    }
}

/// A facility of a recipe of the workshop (`programme`, `skin`, `era`,
/// `variant`, `lod`, `width`, `depth`, `floors`; 0 the usual).
pub fn recipe(b: &mut Builder, p: &crate::Params) {
    let sk = super::types::skin_for(p);
    let prog = Programme::of(p.get("programme", 0.0));
    let (uw, ud) = prog.size(&sk);
    let w = p.get("width", 0.0);
    let d = p.get("depth", 0.0);
    let w = if w <= 0.0 { uw } else { w.clamp(3.0, 200.0) };
    let d = if d <= 0.0 { ud } else { d.clamp(3.0, 200.0) };
    let floors = super::types::floors_of(p, prog.floors(&sk), 12);
    let today = p.get("era", 0.0) > 0.5;
    let l = facility(
        b,
        prog,
        &sk,
        w,
        d,
        floors,
        super::types::seed_of(p),
        super::types::light_of(p),
        today,
    );
    super::types::finish(b, &l);
}

/// The plot a recipe's facility stands on, and its floors (for its state).
pub fn recipe_plan(p: &crate::Params, sk: &Skin) -> (f64, f64, usize) {
    let prog = Programme::of(p.get("programme", 0.0));
    let (uw, ud) = prog.size(sk);
    let w = p.get("width", 0.0);
    let d = p.get("depth", 0.0);
    (
        if w <= 0.0 { uw } else { w.clamp(3.0, 200.0) },
        if d <= 0.0 { ud } else { d.clamp(3.0, 200.0) },
        super::types::floors_of(p, prog.floors(sk), 12),
    )
}

/// What every facility is made with.
pub(super) struct Ctx<'a> {
    pub sk: &'a Skin,
    pub w: f64,
    pub d: f64,
    pub floors: usize,
    pub seed: u64,
    pub light: bool,
    pub today: bool,
}

// ---------------------------------------------------------------- helpers

/// The place's skin for a public building: no balconies, nothing hung,
/// no shops below.
fn civic(base: &Skin, t: Typology) -> Skin {
    let mut s = adapted(base, t);
    s.extras &= !(AC
        | LAUNDRY
        | SIGNS
        | AWNINGS
        | FLOWERS
        | PLANTS
        | HOIST
        | TANK
        | SOLAR
        | WATER_TOWER
        | PENT);
    s.balc = Balc::None;
    s.ground = match s.ground {
        Ground::Arcade => Ground::Arcade,
        Ground::Stoop => Ground::Stoop,
        _ => Ground::Door,
    };
    s
}

/// The shape of a building of the street with the skin's own roof.
fn shape_of(sk: &Skin, floors: usize, seed: u64, light: bool, pavement: bool) -> Shape {
    Shape {
        floors,
        seed,
        light,
        roof: sk.roof,
        cover: sk.cover,
        pitch: sk.pitch,
        eaves: sk.eaves,
        gable: sk.gable,
        top: sk.top,
        extras: sk.extras,
        block: false,
        pavement,
        sparse_free: true,
        lift: None,
        steps: true,
        open_below: false,
    }
}

/// A building of the street of skin `sk` over `plan`.
fn dressed(
    b: &mut Builder,
    plan: &Plan,
    sk: &Skin,
    floors: usize,
    seed: u64,
    light: bool,
) -> (Built, Look) {
    let l = look(sk, seed, false);
    let o = shape_of(sk, floors, seed, light, true);
    let built = building(b, plan, sk, &l, &o);
    (built, l)
}

/// The street front of a building made with [`building`] over a rectangle
/// (its side 2).
fn street_front(built: &Built, w: f64, d: f64) -> (Face, f64) {
    let f = built.fronts[2].map(|f| f.f).unwrap_or(Face {
        o: [-w / 2.0, d],
        u: [1.0, 0.0],
        len: w,
    });
    (f, built.top)
}

/// A sign on a face: its board from `s.0` to `s.1`, `z.0` to `z.1`, and
/// its letters standing on it (`n` of them, as blocks).
#[allow(clippy::too_many_arguments)]
pub(super) fn sign(
    b: &mut Builder,
    f: &Face,
    s: (f64, f64),
    z: (f64, f64),
    board: &'static str,
    letters: &'static str,
    n: usize,
    light: bool,
) {
    f.bx(b, board, s, z, (-0.12, 0.06));
    if light || n == 0 {
        return;
    }
    let h = (z.1 - z.0) * 0.55;
    let zc = (z.0 + z.1) / 2.0;
    let span = (s.1 - s.0) * 0.86;
    let pitch = span / n as f64;
    let lw = pitch * 0.7;
    let s0 = (s.0 + s.1) / 2.0 - span / 2.0;
    for i in 0..n {
        let x = s0 + pitch * (i as f64 + 0.5);
        f.bx(
            b,
            letters,
            (x - lw / 2.0, x + lw / 2.0),
            (zc - h / 2.0, zc + h / 2.0),
            (0.05, 0.09),
        );
    }
}

/// A flagpole standing at `c` from `z`, `h` high, its flag (`role`).
pub(super) fn flagpole(
    b: &mut Builder,
    c: [f64; 2],
    z: f64,
    h: f64,
    role: &'static str,
    dir: [f64; 2],
) {
    b.cyl("metal", [c[0], c[1], z], [c[0], c[1], z + h], 0.04);
    b.ellipsoid("metal", [c[0], c[1], z + h + 0.04], [0.06, 0.06, 0.06]);
    let (fw, fh) = (1.5, 1.0);
    let e = [c[0] + dir[0] * fw, c[1] + dir[1] * fw];
    let m = [(c[0] + e[0]) / 2.0, (c[1] + e[1]) / 2.0];
    b.obox(
        role,
        [m[0], m[1], z + h - 0.1 - fh / 2.0],
        [dir[0] * fw / 2.0, dir[1] * fw / 2.0, 0.0],
        [-dir[1] * 0.01, dir[0] * 0.01, 0.0],
        [0.0, 0.0, fh / 2.0],
    );
}

/// A tree: its trunk and a crown of leaves.
pub(super) fn tree(b: &mut Builder, c: [f64; 2], z: f64, h: f64, seed: u64) {
    let r = h * 0.32 * (0.85 + 0.3 * unit(seed, 3));
    b.cone(
        "trunk",
        [c[0], c[1], z],
        [c[0], c[1], z + h - r],
        0.14,
        0.09,
        7,
    );
    b.ellipsoid("leaves", [c[0], c[1], z + h - r * 0.9], [r, r, r * 0.9]);
}

/// A lamp post at `c`, `h` high.
pub(super) fn lamp_post(b: &mut Builder, c: [f64; 2], z: f64, h: f64) {
    b.cone("iron", [c[0], c[1], z], [c[0], c[1], z + h], 0.07, 0.045, 8);
    b.cone(
        "iron",
        [c[0], c[1], z + h],
        [c[0], c[1], z + h + 0.25],
        0.18,
        0.05,
        8,
    );
    b.ellipsoid("lamp", [c[0], c[1], z + h - 0.02], [0.13, 0.13, 0.12]);
}

/// A bench along `u` at `c`.
pub(super) fn bench(b: &mut Builder, c: [f64; 2], u: [f64; 2], wood: &'static str) {
    let n = [-u[1], u[0]];
    for s in [-0.7, 0.7] {
        let p = [c[0] + u[0] * s, c[1] + u[1] * s];
        b.obox(
            "iron",
            [p[0], p[1], 0.22],
            [u[0] * 0.03, u[1] * 0.03, 0.0],
            [n[0] * 0.22, n[1] * 0.22, 0.0],
            [0.0, 0.0, 0.22],
        );
    }
    b.obox(
        wood,
        [c[0], c[1], 0.46],
        [u[0] * 0.9, u[1] * 0.9, 0.0],
        [n[0] * 0.22, n[1] * 0.22, 0.0],
        [0.0, 0.0, 0.03],
    );
    let back = [c[0] - n[0] * 0.2, c[1] - n[1] * 0.2];
    b.obox(
        wood,
        [back[0], back[1], 0.75],
        [u[0] * 0.9, u[1] * 0.9, 0.0],
        [n[0] * 0.025, n[1] * 0.025, 0.0],
        [0.0, 0.0, 0.3],
    );
}

/// A fence round the sides `from` to `to` of a polygon, `h` high, posts
/// every `step` and a mesh between them; gaps where `gate` says (a point
/// on the fence and a width).
pub(super) fn fence(
    b: &mut Builder,
    a: [f64; 2],
    e: [f64; 2],
    h: f64,
    role: &'static str,
    gate: Option<(f64, f64)>,
    light: bool,
) {
    let len = ((e[0] - a[0]).powi(2) + (e[1] - a[1]).powi(2)).sqrt();
    if len < 0.1 {
        return;
    }
    let f = Face {
        o: a,
        u: [(e[0] - a[0]) / len, (e[1] - a[1]) / len],
        len,
    };
    let mut spans = vec![(0.0, len)];
    if let Some((at, wd)) = gate {
        spans = vec![
            (0.0, (at - wd / 2.0).max(0.0)),
            ((at + wd / 2.0).min(len), len),
        ];
    }
    for (s0, s1) in spans {
        if s1 - s0 < 0.1 {
            continue;
        }
        let n = ((s1 - s0) / 2.5).ceil().max(1.0) as usize;
        for i in 0..=n {
            let x = s0 + (s1 - s0) * i as f64 / n as f64;
            f.bx(b, role, (x - 0.03, x + 0.03), (0.0, h), (-0.03, 0.03));
        }
        f.bx(b, role, (s0, s1), (h - 0.05, h), (-0.025, 0.025));
        if light {
            f.bx(b, "metal", (s0, s1), (0.05, h - 0.05), (-0.006, 0.006));
        } else {
            f.bx(b, role, (s0, s1), (0.05, 0.09), (-0.02, 0.02));
            let m = ((s1 - s0) / 0.12).round().max(1.0) as usize;
            for i in 1..m {
                let x = s0 + (s1 - s0) * i as f64 / m as f64;
                f.bx(
                    b,
                    role,
                    (x - 0.008, x + 0.008),
                    (0.09, h - 0.05),
                    (-0.008, 0.008),
                );
            }
        }
    }
}

/// A box of walls `w` × `d` (x from `x0`, y from `y0`) up to `h`, with the
/// holes of each face (front, right, back, left), its core and nothing on
/// top.
#[allow(clippy::too_many_arguments)]
pub(super) fn walls_box(
    b: &mut Builder,
    x: (f64, f64),
    y: (f64, f64),
    z: (f64, f64),
    role: &'static str,
    holes: [&[Hole]; 4],
    core: bool,
) -> [Face; 4] {
    let fs = roofs::faces(x, y);
    for (k, f) in fs.iter().enumerate() {
        wall(b, role, f, z, 0.3, holes[k]);
    }
    if core {
        b.rbox(
            "interior",
            [x.0 + 0.3, y.0 + 0.3, z.0],
            [x.1 - 0.3, y.1 - 0.3, z.1],
            0.0,
        );
    }
    fs
}

/// Windows in a row of holes.
fn windows(
    b: &mut Builder,
    f: &Face,
    holes: &[Hole],
    frame: &'static str,
    light: bool,
    sliding: bool,
) {
    for h in holes {
        if light {
            f.bx(b, "glass", h.s, h.z, (-0.14, -0.1));
        } else {
            window(
                b,
                f,
                *h,
                &Win {
                    leaves: if h.s.1 - h.s.0 > 1.6 { 3 } else { 2 },
                    sliding,
                    ..Win::default()
                },
                frame,
            );
        }
    }
}

/// Holes of windows along a face: `n` of them, `ww` wide, from `z0` to
/// `z1`, leaving `skip` (a span) free.
fn row_of(len: f64, n: usize, ww: f64, z: (f64, f64), skip: Option<(f64, f64)>) -> Vec<Hole> {
    let step = len / n as f64;
    (0..n)
        .map(|i| step * (i as f64 + 0.5))
        .filter(|c| skip.is_none_or(|(a, e)| *c + ww / 2.0 < a || *c - ww / 2.0 > e))
        .map(|c| Hole {
            s: (c - ww / 2.0, c + ww / 2.0),
            z,
        })
        .collect()
}

/// A flat roof over a rectangle at `z` with its parapet.
fn flat_top(
    b: &mut Builder,
    x: (f64, f64),
    y: (f64, f64),
    z: f64,
    wall: &'static str,
    roof: &'static str,
) {
    b.rbox(
        roof,
        [x.0 + 0.3, y.0 + 0.3, z - 0.3],
        [x.1 - 0.3, y.1 - 0.3, z + 0.05],
        0.0,
    );
    for f in roofs::faces(x, y) {
        f.bx(b, wall, (0.0, f.len), (z, z + 0.8), (-0.25, 0.0));
        f.bx(
            b,
            "trim",
            (-0.03, f.len + 0.03),
            (z + 0.8, z + 0.86),
            (-0.28, 0.03),
        );
    }
}

/// Steps up to a door: `n` of them, `rise` each, out of the face.
fn steps(b: &mut Builder, f: &Face, s: (f64, f64), n: usize, role: &'static str) {
    for k in 0..n {
        let out = 0.32 * (n - k) as f64;
        f.bx(
            b,
            role,
            (s.0 - 0.1 * (n - k) as f64, s.1 + 0.1 * (n - k) as f64),
            (0.0, 0.16 * (k + 1) as f64),
            (0.0, out),
        );
    }
}

/// A canopy (marquesina) over a door, on two posts or hung.
fn canopy(
    b: &mut Builder,
    f: &Face,
    s: (f64, f64),
    z: f64,
    out: f64,
    role: &'static str,
    posts: bool,
) {
    f.bx(b, role, s, (z, z + 0.18), (0.0, out));
    if posts {
        for x in [s.0 + 0.15, s.1 - 0.15] {
            f.bar(b, "metal", (x, 0.0), (x, z), out - 0.15, 0.05, 8);
        }
    } else {
        for x in [s.0 + 0.2, s.1 - 0.2] {
            let a = f.at(x, z + 0.18, out - 0.1);
            let e = f.at(x, z + 1.2, 0.0);
            b.cone("metal", a, e, 0.015, 0.015, 6);
        }
    }
}

/// A car, its length along `dir` (0 along y, π/2 along x).
pub(super) fn car_at(b: &mut Builder, c: [f64; 2], dir: f64, pick: f64) {
    let start = b.mark();
    super::super::offices::car(b, c, pick);
    if dir.abs() > 1e-6 {
        turn_from(b, start, c, dir);
    }
}

/// A parasol at `c`: its pole and canvas.
pub(super) fn parasol(b: &mut Builder, c: [f64; 2], h: f64, r: f64, role: &'static str) {
    b.cyl("metal", [c[0], c[1], 0.0], [c[0], c[1], h + 0.15], 0.025);
    b.cone(
        role,
        [c[0], c[1], h - 0.25],
        [c[0], c[1], h + 0.15],
        r,
        0.03,
        8,
    );
}

/// A café table with its chairs round it.
pub(super) fn table_set(
    b: &mut Builder,
    c: [f64; 2],
    chairs: usize,
    top: &'static str,
    seat: &'static str,
) {
    b.cyl("metal", [c[0], c[1], 0.0], [c[0], c[1], 0.72], 0.03);
    b.cone(top, [c[0], c[1], 0.7], [c[0], c[1], 0.74], 0.38, 0.38, 10);
    for k in 0..chairs {
        let a = 2.0 * PI * k as f64 / chairs as f64 + 0.4;
        let p = [c[0] + 0.62 * a.cos(), c[1] + 0.62 * a.sin()];
        b.rbox(
            seat,
            [p[0] - 0.2, p[1] - 0.2, 0.0],
            [p[0] + 0.2, p[1] + 0.2, 0.45],
            0.0,
        );
        let q = [p[0] + 0.2 * a.cos(), p[1] + 0.2 * a.sin()];
        b.obox(
            seat,
            [q[0], q[1], 0.65],
            [-a.sin() * 0.2, a.cos() * 0.2, 0.0],
            [a.cos() * 0.02, a.sin() * 0.02, 0.0],
            [0.0, 0.0, 0.2],
        );
    }
}

/// A planter with its hedge.
fn planter(b: &mut Builder, lo: [f64; 2], hi: [f64; 2], role: &'static str) {
    b.rbox(role, [lo[0], lo[1], 0.0], [hi[0], hi[1], 0.45], 0.0);
    b.rbox(
        "hedge",
        [lo[0] + 0.05, lo[1] + 0.05, 0.45],
        [hi[0] - 0.05, hi[1] - 0.05, 0.95],
        0.08,
    );
}

/// A barrel standing at `c` (a table of a tavern).
fn barrel(b: &mut Builder, c: [f64; 2], h: f64) {
    b.cone(
        "b_cedar",
        [c[0], c[1], 0.0],
        [c[0], c[1], h / 2.0],
        0.26,
        0.31,
        12,
    );
    b.cone(
        "b_cedar",
        [c[0], c[1], h / 2.0],
        [c[0], c[1], h],
        0.31,
        0.26,
        12,
    );
    for z in [0.08, h - 0.08] {
        b.cone(
            "iron",
            [c[0], c[1], z - 0.025],
            [c[0], c[1], z + 0.025],
            0.285,
            0.285,
            12,
        );
    }
}

// --------------------------------------------------------------- civic

fn town_hall(b: &mut Builder, c: &Ctx) -> Look {
    let mut sk = civic(c.sk, Typology::Townhouse);
    sk.storey = (sk.storey.0.max(4.0), sk.storey.1.max(3.6));
    sk.top = match sk.top {
        Top::None | Top::Band => Top::Heavy,
        t => t,
    };
    let arcade = sk.ground == Ground::Arcade;
    let sides = if arcade {
        [Side::Back, Side::Party, Side::Street, Side::Party]
    } else {
        [Side::Back, Side::Free, Side::Street, Side::Free]
    };
    let plan = Plan::rect(c.w, c.d, sides);
    let (built, l) = dressed(b, &plan, &sk, c.floors, c.seed, c.light);
    let (f, top) = street_front(&built, c.w, c.d);
    let fr = built.fronts[2];
    let z1 = fr.map_or(4.0, |fr| fr.z(1));
    let mid = f.len / 2.0;
    // The balcony of honour over the door, its flags.
    if c.floors >= 2 {
        let hw = (sk.bay * 1.5).min(f.len / 2.0 - 0.6);
        balcony(
            b,
            &f,
            (mid - hw, mid + hw),
            z1,
            1.1,
            "trim",
            "iron",
            !c.light,
        );
        let flags = ["cloth_a", "cloth_b", "cloth_c"];
        for (k, role) in flags.iter().enumerate() {
            let x = mid + (k as f64 - 1.0) * hw * 0.6;
            let a = f.at(x, z1 + 0.9, 0.95);
            let e = f.at(x, z1 + 3.2, 2.2);
            b.cone("metal", a, e, 0.025, 0.02, 6);
            let o = f.out();
            // The flag hangs from the pole's top, along the pole's line.
            let p = f.at(x, z1 + 2.55, 1.85);
            b.obox(
                role,
                [p[0], p[1], p[2] - 0.45],
                [f.u[0] * 0.01, f.u[1] * 0.01, 0.0],
                [o[0] * 0.45, o[1] * 0.45, 0.0],
                [0.0, 0.0, 0.5],
            );
        }
    }
    // The sign over the door.
    sign(
        b,
        &f,
        (mid - 2.4, mid + 2.4),
        (z1 - 0.75, z1 - 0.3),
        "trim",
        "brass",
        12,
        c.light,
    );
    // The attic with its clock, and the bell gable or the turret.
    let tw = (f.len * 0.3).clamp(4.0, 7.0);
    let at = top + if sk.roof == Roof::Flat { 0.85 } else { 0.0 };
    let hispanic =
        matches!(c.sk.region, "España" | "Península" | "América") && c.sk.key != "nueva_york";
    let pts = [
        (mid - tw / 2.0, at - 0.6),
        (mid + tw / 2.0, at - 0.6),
        (mid + tw / 2.0, at + 1.6),
        (mid, at + 2.6),
        (mid - tw / 2.0, at + 1.6),
    ];
    f.prism(b, "trim", &pts, (-0.6, 0.05));
    let clock = f.at(mid, at + 1.3, 0.05);
    let o = f.out();
    b.cone(
        "f_white",
        clock,
        [clock[0] + o[0] * 0.06, clock[1] + o[1] * 0.06, clock[2]],
        0.62,
        0.62,
        16,
    );
    if !c.light {
        let face = f.at(mid, at + 1.3, 0.12);
        b.obox(
            "iron",
            [face[0], face[1], face[2] + 0.18],
            [o[0] * 0.01, o[1] * 0.01, 0.0],
            [f.u[0] * 0.03, f.u[1] * 0.03, 0.0],
            [0.0, 0.0, 0.2],
        );
        let hand = f.at(mid + 0.14, at + 1.3, 0.12);
        b.obox(
            "iron",
            hand,
            [f.u[0] * 0.15, f.u[1] * 0.15, 0.0],
            [o[0] * 0.01, o[1] * 0.01, 0.0],
            [0.0, 0.0, 0.025],
        );
    }
    if hispanic {
        // The bell gable (espadaña): a wall with its bell, a cross on top.
        let gz = at + 2.0;
        let pts = [
            (mid - 1.1, gz),
            (mid + 1.1, gz),
            (mid + 1.1, gz + 2.0),
            (mid, gz + 2.7),
            (mid - 1.1, gz + 2.0),
        ];
        f.prism(b, "trim", &pts, (-0.5, -0.1));
        let bell = f.at(mid, gz + 1.15, -0.3);
        b.cone(
            "brass",
            [bell[0], bell[1], bell[2] + 0.35],
            bell,
            0.12,
            0.3,
            10,
        );
        f.bx(
            b,
            "iron",
            (mid - 0.03, mid + 0.03),
            (gz + 2.65, gz + 3.3),
            (-0.33, -0.27),
        );
        f.bx(
            b,
            "iron",
            (mid - 0.2, mid + 0.2),
            (gz + 3.05, gz + 3.11),
            (-0.33, -0.27),
        );
    } else {
        // A small turret with its cupola behind the attic.
        let p = f.at(mid, 0.0, -2.2);
        let zt = at + 1.4;
        b.rbox(
            l.trim,
            [p[0] - 1.0, p[1] - 1.0, top - 0.3],
            [p[0] + 1.0, p[1] + 1.0, zt + 1.8],
            0.0,
        );
        b.cone(
            "roof",
            [p[0], p[1], zt + 1.8],
            [p[0], p[1], zt + 3.6],
            1.25,
            0.08,
            8,
        );
        b.cyl(
            "metal",
            [p[0], p[1], zt + 3.5],
            [p[0], p[1], zt + 4.4],
            0.03,
        );
    }
    if !arcade {
        let door = (mid - 1.0, mid + 1.0);
        steps(b, &f, door, 3, "trim");
    }
    // The square in front: two lamp posts.
    if !c.light {
        for s in [mid - tw, mid + tw] {
            let p = f.at(s, 0.0, 3.0);
            lamp_post(b, [p[0], p[1]], 0.0, 3.6);
        }
    }
    l
}

fn school(b: &mut Builder, c: &Ctx) -> Look {
    let mut sk = civic(c.sk, Typology::Block);
    sk.win = ((sk.win.0 * 1.5).min(2.4), sk.win.1.max(1.5));
    sk.bay = sk.bay.max(3.2);
    sk.ground = Ground::Door;
    sk.storey = (3.5, 3.4);
    let bd = 12.0f64.min(c.d * 0.5);
    let plan = Plan::rect(c.w, bd, [Side::Back, Side::Free, Side::Street, Side::Free]);
    let (built, l) = dressed(b, &plan, &sk, c.floors, c.seed, c.light);
    let (f, _) = street_front(&built, c.w, bd);
    let z1 = built.fronts[2].map_or(3.5, |fr| fr.z(1));
    sign(
        b,
        &f,
        (f.len / 2.0 - 2.6, f.len / 2.0 + 2.6),
        (z1 - 0.8, z1 - 0.3),
        "f_white",
        "f_blue",
        7,
        c.light,
    );
    // The yard in front: the court, its hoops, the fence and the gate.
    let (x0, x1) = (-c.w / 2.0, c.w / 2.0);
    b.rbox("paving", [x0, bd + 2.2, 0.0], [x1, c.d, 0.02], 0.0);
    let cw = (c.w * 0.55).min(28.0);
    let cd = (c.d - bd - 5.0).clamp(6.0, 15.0);
    let cy = bd + 3.0 + cd / 2.0;
    b.rbox(
        "court",
        [-cw / 2.0, cy - cd / 2.0, 0.0],
        [cw / 2.0, cy + cd / 2.0, 0.03],
        0.0,
    );
    if !c.light {
        // The court's lines.
        for y in [cy - cd / 2.0 + 0.1, cy + cd / 2.0 - 0.15] {
            b.rbox(
                "f_white",
                [-cw / 2.0 + 0.1, y, 0.03],
                [cw / 2.0 - 0.1, y + 0.05, 0.034],
                0.0,
            );
        }
        for x in [-cw / 2.0 + 0.1, -0.025, cw / 2.0 - 0.15] {
            b.rbox(
                "f_white",
                [x, cy - cd / 2.0 + 0.1, 0.03],
                [x + 0.05, cy + cd / 2.0 - 0.1, 0.034],
                0.0,
            );
        }
    }
    for (x, s) in [(-cw / 2.0 - 0.6, 1.0), (cw / 2.0 + 0.6, -1.0)] {
        b.cyl("metal", [x, cy, 0.0], [x, cy, 3.3], 0.06);
        b.obox(
            "metal",
            [x + s * 0.5, cy, 3.25],
            [0.5, 0.0, 0.0],
            [0.0, 0.04, 0.0],
            [0.0, 0.0, 0.04],
        );
        b.rbox(
            "f_white",
            [x + s * 1.0 - 0.02, cy - 0.9, 2.85],
            [x + s * 1.0 + 0.02, cy + 0.9, 3.9],
            0.0,
        );
        if !c.light {
            let hx = x + s * 1.25;
            b.cone("f_red", [hx, cy, 3.05], [hx, cy, 3.07], 0.24, 0.24, 12);
            b.obox(
                "f_red",
                [x + s * 1.06, cy, 3.06],
                [0.05, 0.0, 0.0],
                [0.0, 0.05, 0.0],
                [0.0, 0.0, 0.01],
            );
        }
    }
    let gate_at = c.w / 2.0;
    fence(
        b,
        [x0, c.d],
        [x1, c.d],
        2.0,
        "iron",
        Some((gate_at, 3.0)),
        c.light,
    );
    fence(b, [x0, bd], [x0, c.d], 2.0, "iron", None, c.light);
    fence(b, [x1, c.d], [x1, bd], 2.0, "iron", None, c.light);
    for k in 0..3 {
        let x = 2.6 + 1.2 * k as f64;
        flagpole(
            b,
            [x, c.d - 1.5],
            0.0,
            7.0,
            ["cloth_a", "cloth_b", "cloth_c"][k],
            [1.0, 0.0],
        );
    }
    if !c.light {
        for x in [x0 + 2.5, x1 - 2.5] {
            tree(b, [x, c.d - 2.5], 0.0, 6.0, c.seed + x.abs() as u64);
        }
    }
    l
}

fn library(b: &mut Builder, c: &Ctx) -> Look {
    let mut sk = civic(c.sk, Typology::Townhouse);
    sk.ground = Ground::Shop;
    sk.storey = (4.2, 3.4);
    sk.win = (sk.win.0.max(1.2), sk.win.1.max(1.9));
    let plan = Plan::rect(c.w, c.d, [Side::Back, Side::Free, Side::Street, Side::Free]);
    let (built, l) = dressed(b, &plan, &sk, c.floors, c.seed, c.light);
    let (f, _) = street_front(&built, c.w, c.d);
    let z1 = built.fronts[2].map_or(4.2, |fr| fr.z(1));
    let axes = &built.axes[2];
    let door = axes.first().copied().unwrap_or(1.5);
    canopy(b, &f, (door - 1.6, door + 1.6), 3.05, 2.2, "metal", false);
    sign(
        b,
        &f,
        (f.len / 2.0 - 3.2, f.len / 2.0 + 3.2),
        (z1 - 0.5, z1 - 0.1),
        "f_black",
        "f_white",
        10,
        c.light,
    );
    // The racks of bicycles and a bench by the door.
    if !c.light {
        for k in 0..5 {
            let x = door + 3.0 + 0.7 * k as f64;
            let a = f.at(x, 0.0, 1.2);
            let e = f.at(x, 0.0, 2.0);
            b.cone("metal", [a[0], a[1], 0.0], [a[0], a[1], 0.8], 0.02, 0.02, 6);
            b.cone("metal", [e[0], e[1], 0.0], [e[0], e[1], 0.8], 0.02, 0.02, 6);
            b.cone("metal", [a[0], a[1], 0.8], [e[0], e[1], 0.8], 0.02, 0.02, 6);
        }
        let p = f.at(f.len - 2.0, 0.0, 1.0);
        bench(b, [p[0], p[1]], f.u, "b_cedar");
    }
    l
}

fn fire_station(b: &mut Builder, c: &Ctx) -> Look {
    let l0 = look(c.sk, c.seed, false);
    let (x0, x1) = (-c.w / 2.0, c.w / 2.0);
    let bays = ((c.w - 8.0) / 5.0).floor().clamp(1.0, 5.0) as usize;
    let hall_w = bays as f64 * 5.0 + 1.0;
    let hx = (x0, x0 + hall_w);
    let hd = (c.d - 6.0).max(12.0);
    let (y0, y1) = (c.d - hd, c.d);
    let hh = 6.5;
    // The hall of the engines and its doors.
    let doors: Vec<Hole> = (0..bays)
        .map(|i| {
            let s = 0.5 + 5.0 * i as f64;
            Hole {
                s: (s + 0.6, s + 4.4),
                z: (0.0, 4.6),
            }
        })
        .collect();
    let wall_role = if c.today { "c_panel" } else { "wall" };
    let back_holes = row_of(hall_w, bays, 2.0, (2.5, 4.0), None);
    let fs = walls_box(
        b,
        hx,
        (y0, y1),
        (0.0, hh),
        wall_role,
        [&doors, &[], &back_holes, &[]],
        true,
    );
    windows(b, &fs[2], &back_holes, l0.frame, c.light, true);
    for (k, h) in doors.iter().enumerate() {
        let f = &fs[0];
        f.bx(b, "f_red", h.s, h.z, (-0.2, -0.15));
        if !c.light {
            for p in 1..6 {
                let z = h.z.0 + (h.z.1 - h.z.0) * p as f64 / 6.0;
                f.bx(b, "metal", h.s, (z - 0.02, z + 0.02), (-0.15, -0.13));
            }
            // A row of windows across the door.
            f.bx(
                b,
                "glass",
                (h.s.0 + 0.3, h.s.1 - 0.3),
                (3.0, 3.5),
                (-0.155, -0.13),
            );
        }
        f.bx(
            b,
            "trim",
            (h.s.0 - 0.15, h.s.1 + 0.15),
            (h.z.1, h.z.1 + 0.2),
            (0.0, 0.05),
        );
        let _ = k;
    }
    flat_top(b, hx, (y0, y1), hh, wall_role, "r_gravel");
    sign(
        b,
        &fs[0],
        (0.6, hall_w - 0.6),
        (5.0, 5.9),
        "f_red",
        "f_white",
        8,
        c.light,
    );
    // The offices beside, of the place's skin, two floors.
    let ow = c.w - hall_w;
    if ow >= 4.0 {
        let mut sk = civic(c.sk, Typology::Townhouse);
        sk.ground = Ground::Door;
        let plan = Plan {
            poly: vec![[hx.1, y0], [x1, y0], [x1, y1], [hx.1, y1]],
            sides: vec![Side::Back, Side::Free, Side::Street, Side::Party],
        };
        let mut o = shape_of(&sk, c.floors.max(2), c.seed, c.light, false);
        o.roof = Roof::Flat;
        building(
            b,
            &plan,
            &sk,
            &Look {
                roof: "r_gravel",
                ..l0
            },
            &o,
        );
    }
    // The drill tower behind, with its windows.
    let tz = 16.0;
    let t = (x1 - 5.0, y0 - 0.5);
    let tx = (t.0, t.0 + 4.0);
    let ty = (t.1 - 4.0, t.1);
    let mut tower_holes: [Vec<Hole>; 4] = Default::default();
    for (k, h) in tower_holes.iter_mut().enumerate() {
        if k == 0 {
            continue;
        }
        for fl in 0..4 {
            let z = 2.0 + 3.4 * fl as f64;
            h.push(Hole {
                s: (1.3, 2.7),
                z: (z, z + 1.5),
            });
        }
    }
    let tf = walls_box(
        b,
        tx,
        ty,
        (0.0, tz),
        "trim",
        [
            &tower_holes[0],
            &tower_holes[1],
            &tower_holes[2],
            &tower_holes[3],
        ],
        true,
    );
    for (k, f) in tf.iter().enumerate() {
        if k > 0 {
            for h in &tower_holes[k] {
                f.bx(b, "interior", h.s, h.z, (-0.25, -0.2));
            }
        }
    }
    flat_top(b, tx, ty, tz, "trim", "r_gravel");
    // The apron in front.
    b.rbox("c_concrete", [hx.0, y1, 0.0], [hx.1, y1 + 6.0, 0.03], 0.0);
    if !c.light {
        let p = [hx.0 + 2.0, y1 + 3.0];
        flagpole(b, [hx.1 + 1.0, y1 + 1.0], 0.0, 7.5, "cloth_a", [1.0, 0.0]);
        let _ = p;
    }
    Look {
        wall: if c.today { "c_panel" } else { l0.wall },
        ..l0
    }
}

fn police(b: &mut Builder, c: &Ctx) -> Look {
    let mut sk = civic(c.sk, Typology::Block);
    sk.ground = Ground::Door;
    let plan = Plan::rect(c.w, c.d, [Side::Back, Side::Free, Side::Street, Side::Free]);
    let (built, l) = dressed(b, &plan, &sk, c.floors, c.seed, c.light);
    let (f, _) = street_front(&built, c.w, c.d);
    let z1 = built.fronts[2].map_or(3.5, |fr| fr.z(1));
    let mid = f.len / 2.0;
    sign(
        b,
        &f,
        (mid - 2.2, mid + 2.2),
        (z1 - 0.75, z1 - 0.3),
        "f_blue",
        "f_white",
        8,
        c.light,
    );
    canopy(
        b,
        &f,
        (mid - 1.5, mid + 1.5),
        z1 - 0.95,
        1.5,
        "metal",
        false,
    );
    // The flag on the front, bollards along the kerb and the cars.
    let p = f.at(mid + 3.5, 0.0, 0.0);
    let o = f.out();
    let a = [p[0], p[1], z1 + 0.6];
    let e = [p[0] + o[0] * 1.2, p[1] + o[1] * 1.2, z1 + 1.6];
    b.cone("metal", a, e, 0.03, 0.025, 6);
    let fp = [e[0] - o[0] * 0.3, e[1] - o[1] * 0.3];
    b.obox(
        "cloth_a",
        [fp[0], fp[1], e[2] - 0.55],
        [f.u[0] * 0.01, f.u[1] * 0.01, 0.0],
        [o[0] * 0.3, o[1] * 0.3, 0.0],
        [0.0, 0.0, 0.5],
    );
    if !c.light {
        let n = (f.len / 1.6) as usize;
        for k in 0..=n {
            let q = f.at(0.4 + 1.6 * k as f64, 0.0, 2.6);
            if (0.4 + 1.6 * k as f64 - mid).abs() < 1.5 {
                continue;
            }
            b.cone(
                "f_white",
                [q[0], q[1], 0.0],
                [q[0], q[1], 0.8],
                0.09,
                0.08,
                8,
            );
        }
        for (k, s) in [1.6, 4.4].iter().enumerate() {
            let q = f.at(*s, 0.0, 4.6);
            car_at(b, [q[0], q[1]], PI / 2.0, 0.34 * k as f64);
        }
    }
    l
}

// --------------------------------------------------------------- sport

fn sports_hall(b: &mut Builder, c: &Ctx) -> Look {
    let l = look(c.sk, c.seed, false);
    let ed = 6.0;
    let (x0, x1) = (-c.w / 2.0, c.w / 2.0);
    let (y0, y1) = (0.0, c.d - ed);
    let h = 9.0;
    let base = if c.today { "c_concrete" } else { "wall" };
    // The plinth of the walls, the cladding and the band of light.
    for f in roofs::faces((x0, x1), (y0, y1)) {
        f.bx(b, base, (0.0, f.len), (0.0, 3.0), (-0.3, 0.0));
        f.bx(b, "r_galv", (0.0, f.len), (3.0, 6.0), (-0.3, -0.05));
        f.bx(b, "glass", (0.0, f.len), (6.0, 7.6), (-0.25, -0.1));
        f.bx(b, "r_galv", (0.0, f.len), (7.6, h), (-0.3, -0.05));
        if !c.light {
            let n = (f.len / 5.0).round().max(1.0) as usize;
            for i in 0..=n {
                let s = f.len * i as f64 / n as f64;
                f.bx(b, "metal", (s - 0.1, s + 0.1), (3.0, h), (-0.05, 0.05));
            }
        }
    }
    b.rbox(
        "interior",
        [x0 + 0.3, y0 + 0.3, 0.0],
        [x1 - 0.3, y1 - 0.3, h],
        0.0,
    );
    // The roof: a vault across the depth, in strips.
    let span = y1 - y0;
    let rise = span * 0.12;
    let n = if c.light { 8 } else { 18 };
    let yc = (y0 + y1) / 2.0;
    let r = (span * span / 4.0 + rise * rise) / (2.0 * rise);
    let zc = h + rise - r;
    let a0 = ((span / 2.0 + 0.4) / r).asin();
    for k in 0..n {
        let ta = -a0 + 2.0 * a0 * k as f64 / n as f64;
        let tb = -a0 + 2.0 * a0 * (k + 1) as f64 / n as f64;
        let pa = (yc + r * ta.sin(), zc + r * ta.cos());
        let pb = (yc + r * tb.sin(), zc + r * tb.cos());
        let pts: [P3; 4] = [
            [x0 - 0.4, pa.0, pa.1],
            [x1 + 0.4, pa.0, pa.1],
            [x1 + 0.4, pb.0, pb.1],
            [x0 - 0.4, pb.0, pb.1],
        ];
        plate(b, "r_galv", &pts, 0.12);
    }
    // The gable ends under the vault.
    for (x, sgn) in [(x0, 1.0), (x1, -1.0)] {
        let f = Face {
            o: if sgn > 0.0 { [x, y0] } else { [x, y1] },
            u: [0.0, sgn],
            len: span,
        };
        let m = 8;
        let mut pts = vec![(0.0, h - 0.05)];
        for k in 0..=m {
            let t = -a0.min((span / 2.0 / r).asin())
                + 2.0 * (span / 2.0 / r).asin() * k as f64 / m as f64;
            let s = span / 2.0 + r * t.sin() * sgn;
            pts.push((s, zc + r * t.cos() - 0.02));
        }
        pts.push((span, h - 0.05));
        pts.sort_by(|a, e| a.0.total_cmp(&e.0));
        let mut poly = vec![(0.0, h - 0.05)];
        poly.extend(pts.iter().copied().filter(|p| p.1 > h - 0.04));
        poly.push((span, h - 0.05));
        f.prism(b, "r_galv", &poly, (-0.3, -0.05));
    }
    // The entrance in front, glazed, with its canopy and sign.
    let ex = (-7.0f64.min(c.w / 4.0), 7.0f64.min(c.w / 4.0));
    let ef = walls_box(
        b,
        ex,
        (y1 - 0.3, y1 + ed - 2.0),
        (0.0, 4.2),
        base,
        [
            &[Hole {
                s: (0.5, ex.1 - ex.0 - 0.5),
                z: (0.0, 3.4),
            }],
            &[],
            &[],
            &[],
        ],
        false,
    );
    ef[0].bx(
        b,
        "glass",
        (0.5, ex.1 - ex.0 - 0.5),
        (0.0, 3.4),
        (-0.2, -0.15),
    );
    if !c.light {
        let n = ((ex.1 - ex.0 - 1.0) / 1.5).round() as usize;
        for i in 0..=n {
            let s = 0.5 + (ex.1 - ex.0 - 1.0) * i as f64 / n as f64;
            ef[0].bx(b, "metal", (s - 0.04, s + 0.04), (0.0, 3.4), (-0.2, -0.12));
        }
    }
    b.rbox(
        "interior",
        [ex.0 + 0.3, y1, 0.0],
        [ex.1 - 0.3, y1 + ed - 2.3, 4.0],
        0.0,
    );
    flat_top(b, ex, (y1 - 0.3, y1 + ed - 2.0), 4.2, base, "r_gravel");
    canopy(
        b,
        &ef[0],
        (-1.0, ex.1 - ex.0 + 1.0),
        3.5,
        2.0,
        "metal",
        true,
    );
    let fr = roofs::faces((x0, x1), (y0, y1))[0];
    sign(
        b,
        &fr,
        (c.w / 2.0 - 6.0, c.w / 2.0 + 6.0),
        (6.1, 7.4),
        "f_white",
        l.accent,
        13,
        c.light,
    );
    b.rbox("paving", [x0, y1, 0.0], [x1, c.d, 0.02], 0.0);
    l
}

fn gym(b: &mut Builder, c: &Ctx) -> Look {
    let mut sk = adapted(c.sk, Typology::Townhouse);
    sk.ground = Ground::Shop;
    sk.balc = Balc::None;
    sk.sash = Sash::Sliding;
    sk.win = (sk.win.0.max(1.6), sk.win.1.max(1.6));
    sk.extras &= !(LAUNDRY | HOIST | FLOWERS);
    let plan = Plan::rect(
        c.w,
        c.d,
        [Side::Back, Side::Party, Side::Street, Side::Party],
    );
    let (built, l) = dressed(b, &plan, &sk, c.floors, c.seed, c.light);
    let (f, _) = street_front(&built, c.w, c.d);
    let z1 = built.fronts[2].map_or(3.5, |fr| fr.z(1));
    // A great sign along the first floor and the light box.
    sign(
        b,
        &f,
        (0.4, f.len - 0.4),
        (z1 + 0.15, z1 + 0.85),
        "f_black",
        "f_yellow",
        7,
        c.light,
    );
    let p = f.at(f.len - 0.5, z1 + 0.5, 0.0);
    let o = f.out();
    b.obox(
        "lamp",
        [p[0] + o[0] * 0.4, p[1] + o[1] * 0.4, p[2]],
        [o[0] * 0.4, o[1] * 0.4, 0.0],
        [f.u[0] * 0.06, f.u[1] * 0.06, 0.0],
        [0.0, 0.0, 0.45],
    );
    l
}

fn stadium(b: &mut Builder, c: &Ctx) -> Look {
    let l = look(c.sk, c.seed, false);
    let (pw, pd) = (105.0f64.min(c.w - 30.0), 68.0f64.min(c.d - 34.0));
    let cy = c.d / 2.0;
    // The pitch and its lines.
    b.rbox(
        "lawn",
        [-pw / 2.0 - 4.0, cy - pd / 2.0 - 4.0, 0.0],
        [pw / 2.0 + 4.0, cy + pd / 2.0 + 4.0, 0.04],
        0.0,
    );
    if !c.light {
        let lw = 0.12;
        for y in [cy - pd / 2.0, cy + pd / 2.0 - lw] {
            b.rbox(
                "f_white",
                [-pw / 2.0, y, 0.04],
                [pw / 2.0, y + lw, 0.045],
                0.0,
            );
        }
        for x in [-pw / 2.0, -lw / 2.0, pw / 2.0 - lw] {
            b.rbox(
                "f_white",
                [x, cy - pd / 2.0, 0.04],
                [x + lw, cy + pd / 2.0, 0.045],
                0.0,
            );
        }
        for s in [-1.0, 1.0] {
            let x = s * pw / 2.0;
            let xi = x - s * 16.5;
            let (a, e) = (x.min(xi), x.max(xi));
            b.rbox(
                "f_white",
                [a, cy - 20.16, 0.04],
                [e, cy - 20.04, 0.045],
                0.0,
            );
            b.rbox(
                "f_white",
                [a, cy + 20.04, 0.04],
                [e, cy + 20.16, 0.045],
                0.0,
            );
            b.rbox(
                "f_white",
                [xi - lw / 2.0, cy - 20.16, 0.04],
                [xi + lw / 2.0, cy + 20.16, 0.045],
                0.0,
            );
            // The goal.
            let gx = x + s * 1.0;
            for y in [cy - 3.66, cy + 3.66] {
                b.cyl("f_white", [x, y, 0.0], [x, y, 2.44], 0.06);
                b.cyl("f_white", [gx, y, 0.0], [gx, y, 2.0], 0.03);
                b.cyl("f_white", [x, y, 2.44], [gx, y, 2.0], 0.03);
            }
            b.cyl("f_white", [x, cy - 3.66, 2.44], [x, cy + 3.66, 2.44], 0.06);
            b.cyl(
                "f_white",
                [gx, cy - 3.66, 0.02],
                [gx, cy + 3.66, 0.02],
                0.03,
            );
            b.cyl("f_white", [gx, cy - 3.66, 2.0], [gx, cy + 3.66, 2.0], 0.03);
        }
        let n = 16;
        for k in 0..n {
            let a = 2.0 * PI * k as f64 / n as f64;
            let (a2, r) = (2.0 * PI * (k + 1) as f64 / n as f64, 9.15);
            b.rbox(
                "f_white",
                [r * a.cos() - 0.06, cy + r * a.sin() - 0.06, 0.04],
                [r * a.cos() + 0.06, cy + r * a.sin() + 0.06, 0.045],
                0.0,
            );
            let _ = a2;
        }
    }
    // The stands: stepped rows on the four sides, the seats the club's.
    let seat_roles = [l.accent, "f_white"];
    let rows = if c.light { 6 } else { 16 };
    let stands: [(Face, f64); 4] = {
        let (x0, x1) = (-pw / 2.0 - 6.0, pw / 2.0 + 6.0);
        let (y0, y1) = (cy - pd / 2.0 - 6.0, cy + pd / 2.0 + 6.0);
        // Faces looking at the pitch (their out towards it).
        [
            (
                Face {
                    o: [x1, y0],
                    u: [-1.0, 0.0],
                    len: x1 - x0,
                },
                1.0,
            ),
            (
                Face {
                    o: [x0, y1],
                    u: [1.0, 0.0],
                    len: x1 - x0,
                },
                1.0,
            ),
            (
                Face {
                    o: [x0, y0],
                    u: [0.0, 1.0],
                    len: y1 - y0,
                },
                0.8,
            ),
            (
                Face {
                    o: [x1, y1],
                    u: [0.0, -1.0],
                    len: y1 - y0,
                },
                0.8,
            ),
        ]
    };
    let depth = 14.0;
    for (k, (f, scale)) in stands.iter().enumerate() {
        let rd = depth / rows as f64;
        let rz = 0.42 * scale;
        let main = k == 1;
        let rows_here = if main { rows + rows / 3 } else { rows };
        for r in 0..rows_here {
            let d0 = -(r as f64) * rd - rd;
            let z = rz * (r as f64 + 1.0);
            f.bx(b, "c_concrete", (0.0, f.len), (0.0, z), (d0, d0 + rd));
            let role = seat_roles[if (r / 4) % 2 == 0 { 0 } else { 1 }];
            f.bx(
                b,
                role,
                (0.4, f.len - 0.4),
                (z, z + 0.42),
                (d0 + 0.15, d0 + 0.45),
            );
        }
        let back = -(rows_here as f64) * rd;
        let top = rz * rows_here as f64;
        f.bx(
            b,
            "c_concrete",
            (0.0, f.len),
            (top, top + 1.1),
            (back, back + 0.3),
        );
        if main {
            // The roof over the main stand on its columns.
            let zr = top + 6.0;
            let n = (f.len / 12.0).round() as usize;
            for i in 0..=n {
                let s = f.len * i as f64 / n as f64;
                f.bx(
                    b,
                    "metal",
                    (s - 0.3, s + 0.3),
                    (0.0, zr),
                    (back - 0.6, back),
                );
            }
            f.bx(b, "metal", (0.0, f.len), (zr, zr + 0.6), (back - 0.6, -1.0));
            f.bx(
                b,
                "r_galv",
                (0.0, f.len),
                (zr + 0.6, zr + 0.75),
                (back - 0.6, -0.5),
            );
            sign(
                b,
                &f.moved(-1.1),
                (f.len / 2.0 - 12.0, f.len / 2.0 + 12.0),
                (zr - 1.2, zr),
                "f_white",
                l.accent,
                8,
                c.light,
            );
        }
        // The floodlight at the corner.
        let p = f.at(0.0, 0.0, back - 2.0);
        let hp = top + 18.0;
        b.cone("metal", [p[0], p[1], 0.0], [p[0], p[1], hp], 0.45, 0.28, 8);
        let o = f.out();
        let q = [p[0] + o[0] * 0.3, p[1] + o[1] * 0.3, hp + 1.2];
        b.obox(
            "metal",
            q,
            [f.u[0] * 2.0, f.u[1] * 2.0, 0.0],
            [o[0] * 0.3, o[1] * 0.3, 0.0],
            [0.0, 0.0, 1.4],
        );
        b.obox(
            "lamp",
            [q[0] + o[0] * 0.31, q[1] + o[1] * 0.31, q[2]],
            [f.u[0] * 1.8, f.u[1] * 1.8, 0.0],
            [o[0] * 0.02, o[1] * 0.02, 0.0],
            [0.0, 0.0, 1.2],
        );
    }
    l
}

fn factory(b: &mut Builder, c: &Ctx) -> Look {
    let l0 = look(c.sk, c.seed, false);
    let wall_role = if c.today { "c_panel" } else { "k_red" };
    let (x0, x1) = (-c.w / 2.0, c.w / 2.0);
    let od = 8.0;
    let (y0, y1) = (0.0, c.d - od);
    let h = 6.5;
    let teeth = ((y1 - y0) / 7.5).round().max(2.0) as usize;
    let tooth = (y1 - y0) / teeth as f64;
    // The walls with their tall windows and the loading door.
    let side_holes: Vec<Hole> = row_of(y1 - y0, teeth * 2, 1.6, (2.0, 5.4), None);
    let dock = Hole {
        s: (2.0, 6.5),
        z: (0.0, 4.8),
    };
    let back_holes = row_of(x1 - x0, ((x1 - x0) / 5.0) as usize, 2.2, (2.0, 5.4), None);
    let fs = walls_box(
        b,
        (x0, x1),
        (y0, y1),
        (0.0, h),
        wall_role,
        [&[], &[dock], &back_holes, &side_holes],
        true,
    );
    windows(b, &fs[3], &side_holes, "f_black", c.light, false);
    windows(b, &fs[2], &back_holes, "f_black", c.light, false);
    fs[1].bx(b, "r_galv", dock.s, dock.z, (-0.2, -0.15));
    if !c.light {
        for f in &fs {
            let n = (f.len / 5.0).round().max(1.0) as usize;
            for i in 0..=n {
                let s = f.len * i as f64 / n as f64;
                f.bx(
                    b,
                    wall_role,
                    (s - 0.25, s + 0.25),
                    (0.0, h + 0.3),
                    (0.0, 0.12),
                );
            }
        }
        // The dock's platform.
        fs[1].bx(
            b,
            "c_concrete",
            (dock.s.0 - 0.5, dock.s.1 + 0.5),
            (0.0, 1.1),
            (0.0, 2.5),
        );
    }
    // The saw-tooth roof: a slope of sheet and a glazed upright each.
    for t in 0..teeth {
        let ya = y0 + tooth * t as f64;
        let yb = ya + tooth;
        let up = 2.6;
        let pts: [P3; 4] = [[x0, ya, h], [x1, ya, h], [x1, yb, h + up], [x0, yb, h + up]];
        plate(b, "r_galv", &pts, 0.12);
        let f = Face {
            o: [x0, yb],
            u: [1.0, 0.0],
            len: x1 - x0,
        };
        f.bx(b, "glass", (0.0, f.len), (h, h + up), (-0.08, -0.02));
        if !c.light {
            let n = (f.len / 2.0) as usize;
            for i in 0..=n {
                let s = f.len * i as f64 / n as f64;
                f.bx(b, "f_black", (s - 0.04, s + 0.04), (h, h + up), (-0.1, 0.0));
            }
        }
        for (x, sgn) in [(x0, 1.0), (x1, -1.0)] {
            let fe = Face {
                o: if sgn > 0.0 { [x, ya] } else { [x, yb] },
                u: [0.0, sgn],
                len: tooth,
            };
            let pts = if sgn > 0.0 {
                [(0.0, h - 0.02), (tooth, h - 0.02), (tooth, h + up)]
            } else {
                [(0.0, h - 0.02), (tooth, h - 0.02), (0.0, h + up)]
            };
            fe.prism(b, wall_role, &pts, (-0.3, 0.0));
        }
    }
    // The office block in front, of the place's skin.
    let mut sk = civic(c.sk, Typology::Townhouse);
    sk.ground = Ground::Door;
    let ow = (c.w * 0.6).max(10.0).min(c.w);
    let plan = Plan {
        poly: vec![
            [-ow / 2.0, y1],
            [ow / 2.0, y1],
            [ow / 2.0, c.d],
            [-ow / 2.0, c.d],
        ],
        sides: vec![Side::Party, Side::Free, Side::Street, Side::Free],
    };
    let mut o = shape_of(&sk, 2, c.seed, c.light, false);
    if c.today {
        o.roof = Roof::Flat;
    }
    let built = building(b, &plan, &sk, &l0, &o);
    let (f, top) = street_front(&built, ow, c.d);
    sign(
        b,
        &f,
        (1.0, f.len - 1.0),
        (top - 0.9, top - 0.2),
        wall_role,
        "f_white",
        9,
        c.light,
    );
    // The chimney.
    let ch = [x1 - 3.0, y0 + 3.0];
    let chh = if c.today { 14.0 } else { 26.0 };
    b.rbox(
        wall_role,
        [ch[0] - 1.3, ch[1] - 1.3, 0.0],
        [ch[0] + 1.3, ch[1] + 1.3, 3.0],
        0.0,
    );
    b.cone(
        wall_role,
        [ch[0], ch[1], 3.0],
        [ch[0], ch[1], chh],
        1.05,
        0.65,
        12,
    );
    b.cone(
        "iron",
        [ch[0], ch[1], chh - 0.4],
        [ch[0], ch[1], chh + 0.15],
        0.75,
        0.75,
        12,
    );
    Look {
        wall: l0.wall,
        ..l0
    }
}

// ------------------------------------------------------------ commerce

fn kiosk(b: &mut Builder, c: &Ctx) -> Look {
    let l = look(c.sk, c.seed, false);
    let r = c.w.min(c.d) / 2.0;
    let sides = 8;
    let ring = |rr: f64| -> Vec<[f64; 2]> {
        (0..sides)
            .map(|k| {
                let a = 2.0 * PI * (k as f64 + 0.5) / sides as f64;
                [rr * a.cos(), c.d / 2.0 + rr * a.sin()]
            })
            .collect()
    };
    let body = ring(r * 0.92);
    let structure = l.frame;
    b.slab(structure, &ring(r), 0.0, 0.25);
    b.slab(structure, &body, 0.25, 1.05);
    // The glass and the newspapers, the posts between.
    b.slab("glass", &ring(r * 0.88), 1.05, 2.25);
    b.slab("interior", &ring(r * 0.8), 1.05, 2.25);
    for p in &body {
        b.rbox(
            structure,
            [p[0] - 0.05, p[1] - 0.05, 1.0],
            [p[0] + 0.05, p[1] + 0.05, 2.35],
            0.0,
        );
    }
    b.slab(structure, &body, 2.25, 2.55);
    if !c.light {
        // The magazines on the shutters' ledge, round the front half.
        for (k, p) in body.iter().enumerate().take(sides / 2 + 1).skip(1) {
            let role = ["cloth_a", "cloth_b", "cloth_c"][k % 3];
            b.rbox(
                role,
                [p[0] * 1.02 - 0.18, p[1] - 0.12, 0.9],
                [p[0] * 1.02 + 0.18, p[1] + 0.12, 1.25],
                0.0,
            );
        }
    }
    // The roof: eaves and a dome or a cone, as the place does.
    let roof_role = if c.sk.roof == Roof::Asian {
        "roof"
    } else {
        structure
    };
    b.cone(
        roof_role,
        [0.0, c.d / 2.0, 2.55],
        [0.0, c.d / 2.0, 2.7],
        r * 1.25,
        r * 1.25,
        sides,
    );
    b.cone(
        roof_role,
        [0.0, c.d / 2.0, 2.7],
        [0.0, c.d / 2.0, 3.6],
        r * 1.25,
        0.12,
        sides,
    );
    b.ellipsoid("brass", [0.0, c.d / 2.0, 3.7], [0.12, 0.12, 0.14]);
    // The sign over the counter.
    let f = Face {
        o: [-r * 0.6, c.d / 2.0 + r * 0.82],
        u: [1.0, 0.0],
        len: r * 1.2,
    };
    sign(
        b,
        &f,
        (0.0, f.len),
        (2.28, 2.52),
        "sign",
        "f_white",
        6,
        c.light,
    );
    l
}

fn shop(b: &mut Builder, c: &Ctx) -> Look {
    let mut sk = adapted(c.sk, Typology::Townhouse);
    sk.ground = Ground::Shop;
    sk.extras |= AWNINGS | SIGNS;
    let plan = Plan::rect(
        c.w,
        c.d,
        [Side::Back, Side::Party, Side::Street, Side::Party],
    );
    let (built, l) = dressed(b, &plan, &sk, c.floors, c.seed, c.light);
    let (f, _) = street_front(&built, c.w, c.d);
    // Crates of fruit and flowers out on the pavement.
    if !c.light {
        let n = ((f.len - 2.5) / 1.1).floor().max(0.0) as usize;
        for k in 0..n.min(5) {
            let s = 2.0 + 1.1 * k as f64;
            let p = f.at(s, 0.0, 0.5);
            b.rbox(
                "b_cedar",
                [p[0] - 0.45, p[1] - 0.3, 0.0],
                [p[0] + 0.45, p[1] + 0.3, 0.62],
                0.0,
            );
            let role = ["fruit", "flowers", "leaves", "fruit"][k % 4];
            b.ellipsoid(role, [p[0], p[1], 0.65], [0.4, 0.25, 0.12]);
        }
    }
    l
}

fn street_market(b: &mut Builder, c: &Ctx) -> Look {
    let l = look(c.sk, c.seed, false);
    b.rbox(
        "paving",
        [-c.w / 2.0, 0.0, 0.0],
        [c.w / 2.0, c.d, 0.02],
        0.0,
    );
    let (sw, sd) = (3.0, 2.0);
    let per = ((c.w - 2.0) / (sw + 0.6)).floor().max(1.0) as usize;
    let rows = [c.d * 0.25, c.d * 0.75];
    for (ri, &y) in rows.iter().enumerate() {
        let facing = if ri == 0 { 1.0 } else { -1.0 };
        for i in 0..per {
            let x = -c.w / 2.0 + 1.0 + (sw + 0.6) * (i as f64 + 0.5);
            let k = ri * per + i;
            let (cl, cr) = ([x - sw / 2.0, y - sd / 2.0], [x + sw / 2.0, y + sd / 2.0]);
            // The table on its trestles and its cloth.
            b.rbox(
                "metal",
                [cl[0] + 0.1, cl[1] + 0.2, 0.0],
                [cl[0] + 0.15, cr[1] - 0.2, 0.78],
                0.0,
            );
            b.rbox(
                "metal",
                [cr[0] - 0.15, cl[1] + 0.2, 0.0],
                [cr[0] - 0.1, cr[1] - 0.2, 0.78],
                0.0,
            );
            let cloth = ["b_cedar", "f_white", "b_cedar"][k % 3];
            b.rbox(
                cloth,
                [cl[0], cl[1] + 0.1, 0.78],
                [cr[0], cr[1] - 0.1, 0.82],
                0.0,
            );
            // What is sold: fruit, clothes or pots.
            if !c.light {
                match k % 3 {
                    0 => {
                        for j in 0..4 {
                            let px = cl[0] + 0.4 + 0.72 * j as f64;
                            b.rbox(
                                "b_cedar",
                                [px - 0.3, y - 0.3, 0.82],
                                [px + 0.3, y + 0.3, 0.98],
                                0.0,
                            );
                            let role = ["fruit", "leaves", "flowers", "fruit"][j];
                            b.ellipsoid(role, [px, y, 1.0], [0.27, 0.27, 0.08]);
                        }
                    }
                    1 => {
                        let back = y - facing * (sd / 2.0 - 0.15);
                        b.cyl(
                            "metal",
                            [cl[0] + 0.2, back, 0.82],
                            [cr[0] - 0.2, back, 0.82],
                            0.01,
                        );
                        for j in 0..6 {
                            let px = cl[0] + 0.4 + 0.42 * j as f64;
                            let role = ["cloth_a", "cloth_b", "cloth_c"][(j + k) % 3];
                            b.rbox(
                                role,
                                [px - 0.17, back - 0.02, 0.82],
                                [px + 0.17, back + 0.02, 1.62],
                                0.0,
                            );
                        }
                    }
                    _ => {
                        for j in 0..5 {
                            let px = cl[0] + 0.4 + 0.55 * j as f64;
                            b.cone("pot", [px, y, 0.82], [px, y, 1.05], 0.12, 0.18, 8);
                        }
                    }
                }
            }
            // The awning on four posts.
            for p in [cl, [cr[0], cl[1]], cr, [cl[0], cr[1]]] {
                b.cyl("metal", [p[0], p[1], 0.0], [p[0], p[1], 2.3], 0.025);
            }
            let role = if k.is_multiple_of(2) {
                "awning"
            } else {
                "awning_b"
            };
            let pts: [P3; 4] = [
                [cl[0] - 0.1, y - facing * (sd / 2.0 + 0.1), 2.35],
                [cr[0] + 0.1, y - facing * (sd / 2.0 + 0.1), 2.35],
                [cr[0] + 0.1, y + facing * (sd / 2.0 + 0.5), 2.0],
                [cl[0] - 0.1, y + facing * (sd / 2.0 + 0.5), 2.0],
            ];
            plate(b, role, &pts, 0.02);
            b.rbox(
                "metal",
                [cl[0] - 0.1, y - facing * (sd / 2.0) - 0.03, 2.25],
                [cr[0] + 0.1, y - facing * (sd / 2.0) + 0.03, 2.35],
                0.0,
            );
        }
    }
    l
}

fn arcade(b: &mut Builder, c: &Ctx) -> Look {
    let l = look(c.sk, c.seed, false);
    let pw = (c.w * 0.32).clamp(4.0, 7.0);
    let (x0, x1) = (-c.w / 2.0, c.w / 2.0);
    let (a, e) = (-pw / 2.0, pw / 2.0);
    let fl = c.floors.max(2);
    let (gh, fh) = (4.2, 3.6);
    let h = gh + fh * (fl - 1) as f64;
    let units = (c.d / 5.0).floor().max(2.0) as usize;
    let uw = c.d / units as f64;
    // The two rows of shops facing the passage.
    for (wx0, wx1, side) in [(x0, a, 0), (e, x1, 1)] {
        let fs = roofs::faces((wx0, wx1), (0.0, c.d));
        // Their inner face looks at the passage: left face of the right
        // row, right face of the left row.
        let inner = if side == 0 { 1 } else { 3 };
        let mut holes = Vec::new();
        for u in 0..units {
            holes.push(Hole {
                s: (uw * u as f64 + 0.3, uw * (u + 1) as f64 - 0.3),
                z: (0.0, gh - 0.6),
            });
            for k in 1..fl {
                let z = gh + fh * (k - 1) as f64;
                holes.push(Hole {
                    s: (uw * u as f64 + 0.8, uw * (u + 1) as f64 - 0.8),
                    z: (z + 0.9, z + fh - 0.5),
                });
            }
        }
        let outer_holes = row_of(c.d, units, 1.2, (gh + 0.9, gh + 2.4), None);
        let outer = if side == 0 { 3 } else { 1 };
        for (k, f) in fs.iter().enumerate() {
            let hs: &[Hole] = if k == inner {
                &holes
            } else if k == outer {
                &outer_holes
            } else {
                &[]
            };
            wall(b, "wall", f, (0.0, h), 0.3, hs);
        }
        b.rbox(
            "interior",
            [wx0 + 0.3, 0.3, 0.0],
            [wx1 - 0.3, c.d - 0.3, h],
            0.0,
        );
        for hh in &holes {
            if hh.z.0 < 0.01 {
                shop_front(b, &fs[inner], *hh, 0.0, true, "f_black", 0.15);
            } else {
                windows(
                    b,
                    &fs[inner],
                    std::slice::from_ref(hh),
                    l.frame,
                    c.light,
                    false,
                );
            }
        }
        windows(b, &fs[outer], &outer_holes, l.frame, c.light, false);
        flat_top(b, (wx0, wx1), (0.0, c.d), h, "wall", "r_gravel");
    }
    // The fronts on the street and at the back with their arches.
    for (y, out) in [(c.d, 1.0), (0.0, -1.0)] {
        let f = if out > 0.0 {
            Face {
                o: [a, y],
                u: [1.0, 0.0],
                len: pw,
            }
        } else {
            Face {
                o: [e, y],
                u: [-1.0, 0.0],
                len: pw,
            }
        };
        let ztop = h + 1.2;
        let arch = (h - 1.5).max(4.5);
        f.bx(b, "wall", (-0.3, pw + 0.3), (arch, ztop), (-0.3, 0.0));
        f.bx(
            b,
            "trim",
            (-0.4, pw + 0.4),
            (ztop, ztop + 0.2),
            (-0.35, 0.08),
        );
        if out > 0.0 {
            sign(
                b,
                &f,
                (0.3, pw - 0.3),
                (arch + 0.3, arch + 1.1),
                "trim",
                "brass",
                7,
                c.light,
            );
        }
    }
    // The glass vault over the passage on its iron arches.
    let zr = h + 0.1;
    let ribs = (c.d / 3.0).round().max(2.0) as usize;
    let rise = pw * 0.35;
    let seg = if c.light { 4 } else { 8 };
    for k in 0..=ribs {
        let y = c.d * k as f64 / ribs as f64;
        for j in 0..seg {
            let t0 = PI * j as f64 / seg as f64;
            let t1 = PI * (j + 1) as f64 / seg as f64;
            let p0 = [-(pw / 2.0) * t0.cos(), y, zr + rise * t0.sin()];
            let p1 = [-(pw / 2.0) * t1.cos(), y, zr + rise * t1.sin()];
            b.cone("iron", p0, p1, 0.05, 0.05, 6);
        }
    }
    for j in 0..seg {
        let t0 = PI * j as f64 / seg as f64;
        let t1 = PI * (j + 1) as f64 / seg as f64;
        let (x0g, z0g) = (-(pw / 2.0) * t0.cos(), zr + rise * t0.sin());
        let (x1g, z1g) = (-(pw / 2.0) * t1.cos(), zr + rise * t1.sin());
        let pts: [P3; 4] = [
            [x0g, 0.0, z0g],
            [x0g, c.d, z0g],
            [x1g, c.d, z1g],
            [x1g, 0.0, z1g],
        ];
        plate(b, "glass", &pts, 0.02);
    }
    // The passage's floor and its lamps.
    b.rbox("paving", [a, 0.0, 0.0], [e, c.d, 0.02], 0.0);
    if !c.light {
        for k in 1..ribs {
            let y = c.d * k as f64 / ribs as f64;
            b.cyl("iron", [0.0, y, zr + rise], [0.0, y, zr + rise - 1.2], 0.01);
            b.ellipsoid("lamp", [0.0, y, zr + rise - 1.35], [0.2, 0.2, 0.18]);
        }
    }
    l
}

fn mall(b: &mut Builder, c: &Ctx) -> Look {
    let l = look(c.sk, c.seed, false);
    let bd = (c.d * 0.6).max(30.0);
    let (x0, x1) = (-c.w / 2.0, c.w / 2.0);
    let h = 13.0;
    let wall_role = if c.today { "c_panel" } else { "wall" };
    let ex = (-9.0, 9.0);
    let front_holes = [Hole {
        s: (c.w / 2.0 + ex.0, c.w / 2.0 + ex.1),
        z: (0.0, 11.0),
    }];
    let fs = walls_box(
        b,
        (x0, x1),
        (0.0, bd),
        (0.0, h),
        wall_role,
        [&front_holes, &[], &[], &[]],
        true,
    );
    // The glazed atrium of the entrance, its frame and the doors.
    let f = &fs[0];
    let hs = front_holes[0].s;
    f.bx(b, "glass", hs, (0.0, 11.0), (-0.1, 0.4));
    if !c.light {
        let n = ((hs.1 - hs.0) / 1.8).round() as usize;
        for i in 0..=n {
            let s = hs.0 + (hs.1 - hs.0) * i as f64 / n as f64;
            f.bx(b, "metal", (s - 0.05, s + 0.05), (0.0, 11.0), (0.4, 0.5));
        }
        for z in [3.2, 6.5, 9.0] {
            f.bx(b, "metal", hs, (z - 0.05, z + 0.05), (0.4, 0.5));
        }
    }
    f.bx(
        b,
        "metal",
        (hs.0 - 1.0, hs.1 + 1.0),
        (11.0, 11.6),
        (-0.3, 3.0),
    );
    // The bands of colour and the signs of the shops.
    f.bx(b, l.accent, (0.0, f.len), (h - 1.4, h - 0.6), (0.0, 0.08));
    sign(
        b,
        f,
        (hs.0 - 0.5, hs.1 + 0.5),
        (11.8, 13.6),
        "f_white",
        l.accent,
        12,
        c.light,
    );
    for s in [6.0, f.len - 18.0] {
        sign(
            b,
            f,
            (s, s + 12.0),
            (7.0, 8.6),
            "sign",
            "f_white",
            8,
            c.light,
        );
    }
    flat_top(b, (x0, x1), (0.0, bd), h, wall_role, "r_gravel");
    if !c.light {
        // The machines on the roof.
        for k in 0..6 {
            let x = x0 + 8.0 + (c.w - 16.0) * k as f64 / 5.0;
            b.rbox(
                "metal",
                [x - 1.5, bd * 0.4, h + 0.05],
                [x + 1.5, bd * 0.4 + 2.0, h + 1.6],
                0.0,
            );
        }
    }
    // The car park in front: its bays, cars, islands of trees and lamps.
    b.rbox("r_black", [x0, bd, 0.0], [x1, c.d, 0.02], 0.0);
    let rows = ((c.d - bd - 4.0) / 16.0).floor().max(1.0) as usize;
    for r in 0..rows {
        let y = bd + 6.0 + 16.0 * r as f64;
        let n = ((c.w - 6.0) / 2.6) as usize;
        for i in 0..n {
            let x = x0 + 3.0 + 2.6 * (i as f64 + 0.5);
            if !c.light {
                for yy in [y - 2.6, y + 2.6] {
                    b.rbox(
                        "f_white",
                        [x - 1.3 - 0.04, yy - 2.4, 0.02],
                        [x - 1.3 + 0.04, yy + 2.4, 0.025],
                        0.0,
                    );
                }
            }
            if unit(c.seed, (r * 100 + i) as u64) < 0.55 {
                car_at(b, [x, y - 2.6], 0.0, unit(c.seed, (r * 100 + i) as u64 + 7));
            }
            if unit(c.seed, (r * 100 + i) as u64 + 3) < 0.45 {
                car_at(b, [x, y + 2.6], 0.0, unit(c.seed, (r * 100 + i) as u64 + 9));
            }
        }
        if !c.light {
            for i in 0..4 {
                let x = x0 + c.w * (i as f64 + 0.5) / 4.0;
                lamp_post(b, [x, y + 6.5], 0.0, 8.0);
                b.rbox(
                    "hedge",
                    [x - 3.0, y + 5.5, 0.0],
                    [x + 3.0, y + 7.5, 0.3],
                    0.0,
                );
                tree(b, [x + 2.0, y + 6.5], 0.3, 5.0, c.seed + i as u64);
            }
        }
    }
    Look {
        wall: if c.today { "c_panel" } else { l.wall },
        ..l
    }
}

// --------------------------------------------------------- hospitality

fn beach_bar(b: &mut Builder, c: &Ctx) -> Look {
    let l = look(c.sk, c.seed, false);
    b.rbox(
        "sand",
        [-c.w / 2.0 - 2.0, -2.0, 0.0],
        [c.w / 2.0 + 2.0, c.d + 2.0, 0.02],
        0.0,
    );
    // The deck on its posts.
    let deck = 0.45;
    b.rbox(
        "b_cedar",
        [-c.w / 2.0, 0.0, deck - 0.08],
        [c.w / 2.0, c.d, deck],
        0.0,
    );
    let n = (c.w / 2.5).ceil() as usize;
    for i in 0..=n {
        for y in [0.2, c.d / 2.0, c.d - 0.2] {
            let x = -c.w / 2.0 + 0.2 + (c.w - 0.4) * i as f64 / n as f64;
            b.rbox(
                "b_cedar",
                [x - 0.08, y - 0.08, 0.0],
                [x + 0.08, y + 0.08, deck - 0.08],
                0.0,
            );
        }
    }
    // The hut of the bar at the back: walls of boards, counter and roof
    // of reed (or of thatch).
    let (hx0, hx1) = (-c.w * 0.3, c.w * 0.3);
    let (hy0, hy1) = (0.4, 0.4 + (c.d * 0.35).max(3.0));
    let fs = walls_box(
        b,
        (hx0, hx1),
        (hy0, hy1),
        (deck, deck + 2.7),
        "b_cedar",
        [
            &[Hole {
                s: (0.6, hx1 - hx0 - 0.6),
                z: (deck + 1.1, deck + 2.3),
            }],
            &[],
            &[],
            &[],
        ],
        true,
    );
    let f = &fs[0];
    f.bx(
        b,
        "b_cedar",
        (0.4, f.len - 0.4),
        (deck, deck + 1.1),
        (0.0, 0.6),
    );
    f.bx(
        b,
        "b_cedar",
        (0.3, f.len - 0.3),
        (deck + 1.1, deck + 1.16),
        (-0.05, 0.75),
    );
    let roof_role = "r_thatch";
    let pts: [P3; 4] = [
        [hx0 - 0.8, hy0 - 0.6, deck + 3.4],
        [hx1 + 0.8, hy0 - 0.6, deck + 3.4],
        [hx1 + 0.8, hy1 + 2.4, deck + 2.6],
        [hx0 - 0.8, hy1 + 2.4, deck + 2.6],
    ];
    plate(b, roof_role, &pts, 0.25);
    for x in [hx0 - 0.5, hx1 + 0.5] {
        b.rbox(
            "b_cedar",
            [x - 0.07, hy1 + 2.0, deck],
            [x + 0.07, hy1 + 2.14, deck + 2.72],
            0.0,
        );
        b.rbox(
            "b_cedar",
            [x - 0.07, hy0 - 0.3, deck],
            [x + 0.07, hy0 - 0.16, deck + 3.45],
            0.0,
        );
    }
    // The sign on its posts at the front.
    let sf = Face {
        o: [-2.5, c.d + 0.6],
        u: [1.0, 0.0],
        len: 5.0,
    };
    for s in [0.3, 4.7] {
        sf.bx(
            b,
            "b_cedar",
            (s - 0.07, s + 0.07),
            (0.0, 3.2),
            (-0.07, 0.07),
        );
    }
    sign(
        b,
        &sf,
        (0.0, 5.0),
        (2.4, 3.2),
        "b_cedar",
        l.accent,
        11,
        c.light,
    );
    // Tables with their parasols on the deck.
    if !c.light {
        let tables = ((c.w - 2.0) / 3.0).floor().max(1.0) as usize;
        for i in 0..tables {
            let x = -c.w / 2.0 + 1.5 + 3.0 * i as f64 + 0.5;
            let y = hy1 + 2.8 + (i % 2) as f64 * 1.2;
            if y + 1.0 > c.d {
                continue;
            }
            let start = b.mark();
            table_set(b, [x, y], 3, "f_white", "b_cedar");
            parasol(b, [x, y], 2.3, 1.2, "r_thatch");
            b.settle(start, deck);
        }
    }
    l
}

fn restaurant(b: &mut Builder, c: &Ctx) -> Look {
    let mut sk = adapted(c.sk, Typology::Townhouse);
    sk.ground = Ground::Shop;
    sk.balc = Balc::None;
    sk.extras &= !(LAUNDRY | HOIST);
    sk.extras |= AWNINGS;
    let bd = (c.d - 5.0).max(8.0);
    let plan = Plan::rect(c.w, bd, [Side::Back, Side::Free, Side::Street, Side::Free]);
    let (built, l) = dressed(b, &plan, &sk, c.floors, c.seed, c.light);
    let (f, top) = street_front(&built, c.w, bd);
    let z1 = built.fronts[2].map_or(3.5, |fr| fr.z(1));
    sign(
        b,
        &f,
        (1.0, f.len - 1.0),
        (z1 - 0.55, z1 - 0.1).min_max(top),
        "trim",
        "brass",
        10,
        c.light,
    );
    // The terrace: planters round it, tables with their parasols.
    let (x0, x1) = (-c.w / 2.0, c.w / 2.0);
    b.rbox("paving", [x0, bd, 0.0], [x1, c.d, 0.02], 0.0);
    planter(b, [x0, bd + 2.4], [x0 + 0.5, c.d - 0.2], "pot");
    planter(b, [x1 - 0.5, bd + 2.4], [x1, c.d - 0.2], "pot");
    planter(b, [x0 + 0.5, c.d - 0.6], [-1.2, c.d - 0.1], "pot");
    planter(b, [1.2, c.d - 0.6], [x1 - 0.5, c.d - 0.1], "pot");
    if !c.light {
        let n = ((c.w - 2.0) / 3.0).floor().max(1.0) as usize;
        for i in 0..n {
            let x = x0 + 1.5 + (c.w - 3.0) * (i as f64 + 0.5) / n as f64;
            let y = bd + 2.5 + (c.d - bd - 3.6) / 2.0;
            table_set(b, [x, y], 4, "f_white", l.frame);
            if c.sk.extras & AWNINGS == 0 {
                parasol(b, [x, y], 2.4, 1.3, "awning");
            }
        }
    }
    l
}

trait MinMax {
    fn min_max(self, top: f64) -> Self;
}

impl MinMax for (f64, f64) {
    fn min_max(self, top: f64) -> Self {
        (self.0.min(top - 0.6), self.1.min(top - 0.15))
    }
}

fn tavern(b: &mut Builder, c: &Ctx) -> Look {
    let mut sk = adapted(c.sk, Typology::Townhouse);
    sk.ground = Ground::Door;
    sk.extras &= !(AWNINGS | SIGNS);
    let plan = Plan::rect(
        c.w,
        c.d,
        [Side::Back, Side::Party, Side::Street, Side::Party],
    );
    let (built, l) = dressed(b, &plan, &sk, c.floors, c.seed, c.light);
    let (f, _) = street_front(&built, c.w, c.d);
    let z1 = built.fronts[2].map_or(3.5, |fr| fr.z(1));
    let door = built.axes[2].first().copied().unwrap_or(1.2);
    // The hanging sign on its bracket and the lantern.
    let s = (door + 1.2).min(f.len - 0.4);
    f.bx(
        b,
        "iron",
        (s - 0.02, s + 0.02),
        (z1 - 0.65, z1 - 0.6),
        (0.0, 1.0),
    );
    f.bx(
        b,
        "iron",
        (s - 0.02, s + 0.02),
        (z1 - 0.95, z1 - 0.6),
        (0.0, 0.04),
    );
    let p = f.at(s, z1 - 0.95, 0.75);
    b.obox(
        "b_cedar",
        [p[0], p[1], p[2] - 0.05],
        [f.out()[0] * 0.32, f.out()[1] * 0.32, 0.0],
        [f.u[0] * 0.02, f.u[1] * 0.02, 0.0],
        [0.0, 0.0, 0.25],
    );
    b.cyl(
        "iron",
        [p[0], p[1], p[2] + 0.2],
        [p[0], p[1], z1 - 0.65],
        0.006,
    );
    if !c.light {
        wall_lantern(b, &f, door, z1 - 0.2);
        // Barrels as tables, stools round them.
        for k in 0..2 {
            let q = f.at(door + 2.0 + 1.6 * k as f64, 0.0, 0.9);
            if door + 2.0 + 1.6 * k as f64 > f.len - 0.6 {
                continue;
            }
            barrel(b, [q[0], q[1]], 1.0);
            for a in [0.0, PI] {
                let st = [q[0] + 0.6 * a.cos() * f.u[0], q[1] + 0.6 * a.cos() * f.u[1]];
                b.cyl("b_cedar", [st[0], st[1], 0.0], [st[0], st[1], 0.72], 0.17);
            }
        }
    }
    l
}

// ---------------------------------------------------------------- square

fn square(b: &mut Builder, c: &Ctx) -> Look {
    let l = look(c.sk, c.seed, false);
    let (x0, x1) = (-c.w / 2.0, c.w / 2.0);
    let cy = c.d / 2.0;
    b.rbox("paving", [x0, 0.0, 0.0], [x1, c.d, 0.03], 0.0);
    // The kerb round it.
    for f in roofs::faces((x0, x1), (0.0, c.d)) {
        f.bx(b, "c_concrete", (0.0, f.len), (0.0, 0.15), (-0.3, 0.0));
    }
    // The fountain in the middle.
    let r = (c.w.min(c.d) * 0.12).clamp(2.0, 4.5);
    b.cone(
        l.trim,
        [0.0, cy, 0.0],
        [0.0, cy, 0.55],
        r + 0.25,
        r + 0.2,
        20,
    );
    b.cone("water", [0.0, cy, 0.4], [0.0, cy, 0.5], r, r, 20);
    b.cone(l.trim, [0.0, cy, 0.5], [0.0, cy, 1.8], 0.35, 0.25, 12);
    b.cone(l.trim, [0.0, cy, 1.8], [0.0, cy, 2.1], 1.1, 0.9, 16);
    b.cone("water", [0.0, cy, 2.0], [0.0, cy, 2.12], 0.85, 0.85, 16);
    b.cone(l.trim, [0.0, cy, 2.1], [0.0, cy, 3.0], 0.18, 0.12, 10);
    b.ellipsoid(l.trim, [0.0, cy, 3.1], [0.2, 0.2, 0.2]);
    // Trees and benches round the edges, lamp posts at the corners.
    let ring = [(0.2, 0.2), (0.8, 0.2), (0.8, 0.8), (0.2, 0.8)];
    let n_side = ((c.w - 6.0) / 7.0).floor().max(1.0) as usize;
    for (i, edge) in [(x0 + 2.5, true), (x1 - 2.5, true)].iter().enumerate() {
        for k in 0..=n_side {
            let y = 3.0 + (c.d - 6.0) * k as f64 / n_side as f64;
            let x = edge.0;
            if c.light {
                b.ellipsoid("leaves", [x, y, 3.8], [1.8, 1.8, 1.6]);
                b.cyl("trunk", [x, y, 0.0], [x, y, 2.4], 0.14);
            } else {
                tree(b, [x, y], 0.0, 6.0, c.seed + (i * 10 + k) as u64);
                if k < n_side {
                    let yb = y + (c.d - 6.0) / n_side as f64 / 2.0;
                    let dir = if i == 0 { [0.0, 1.0] } else { [0.0, -1.0] };
                    bench(b, [x, yb], dir, "b_cedar");
                }
            }
        }
    }
    if !c.light {
        for (fx, fy) in ring {
            lamp_post(b, [x0 + c.w * fx, c.d * fy], 0.0, 3.6);
        }
    }
    // The bandstand (templete) in big squares.
    if c.w >= 28.0 && c.d >= 28.0 {
        let p = [x1 - 7.0, c.d - 7.0];
        let rr = 2.8;
        let sides = 8;
        let ring = |r: f64| -> Vec<[f64; 2]> {
            (0..sides)
                .map(|k| {
                    let a = 2.0 * PI * (k as f64 + 0.5) / sides as f64;
                    [p[0] + r * a.cos(), p[1] + r * a.sin()]
                })
                .collect()
        };
        b.slab(l.trim, &ring(rr), 0.0, 0.9);
        for q in ring(rr - 0.2) {
            b.cyl("iron", [q[0], q[1], 0.9], [q[0], q[1], 3.9], 0.07);
        }
        b.cone(
            "iron",
            [p[0], p[1], 3.9],
            [p[0], p[1], 4.1],
            rr + 0.3,
            rr + 0.3,
            sides,
        );
        b.cone(
            "roof",
            [p[0], p[1], 4.1],
            [p[0], p[1], 5.6],
            rr + 0.3,
            0.1,
            sides,
        );
        if !c.light {
            let f = Face {
                o: [p[0] - rr * 0.9, p[1] - rr * 0.92],
                u: [1.0, 0.0],
                len: rr * 1.8,
            };
            let _ = f;
        }
    }
    l
}
