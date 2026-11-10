//! Boards and sheets laid piece by piece (M14, `docs/MODULAR.md`): each
//! layer of a wall, floor, roof or false ceiling made of boards (plaster,
//! OSB, cement, rigid insulation, cladding panels) is laid in its
//! commercial format from the start of the wall (or the corner of the
//! slab), the joints of one layer half a board off those of the next, and
//! each board cut round the doors and windows. In a modular box each wall
//! and slab ends at the box, so the façade's joints fall on the boxes'.
//! From the pieces, the boards to buy (the cuts packed into whole boards)
//! and, with the frames, joints and fixings, the shopping list of a box.
//! Nothing is kept: all is worked out from the model.

use archisim_core::{BoxId, LayerFunction, MaterialId, Model, Point2, SlabFunction};

/// A commercial board: what it is and its size (width × length), m.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoardFormat {
    /// «Placa de yeso», «Tablero OSB»…
    pub kind: &'static str,
    pub width: f64,
    pub length: f64,
}

/// The usual formats, by what the material's name or class says (standard
/// values, to be confirmed with the supplier). Wool, membranes, mortars,
/// flooring and structure are not boards.
pub const FORMATS: [(&[&str], BoardFormat); 7] = [
    (
        &["yeso", "plaster"],
        BoardFormat {
            kind: "Placa de yeso laminado",
            width: 1.20,
            length: 2.60,
        },
    ),
    (
        &["osb"],
        BoardFormat {
            kind: "Tablero OSB",
            width: 1.25,
            length: 2.50,
        },
    ),
    (
        &["cemento"],
        BoardFormat {
            kind: "Placa de cemento",
            width: 1.20,
            length: 2.50,
        },
    ),
    (
        &["tablero"],
        BoardFormat {
            kind: "Tablero",
            width: 1.25,
            length: 2.50,
        },
    ),
    (
        &["cerámic", "ceramic", "porcel", "fenól", "fenol"],
        BoardFormat {
            kind: "Placa de revestimiento",
            width: 1.20,
            length: 2.40,
        },
    ),
    (
        &[
            "placa con aislamiento",
            "panel aislante",
            "xps",
            "eps",
            "pir",
            "poliestireno",
        ],
        BoardFormat {
            kind: "Panel aislante",
            width: 0.60,
            length: 1.20,
        },
    ),
    (
        &["placa", "panel"],
        BoardFormat {
            kind: "Placa",
            width: 1.20,
            length: 2.50,
        },
    ),
];

/// Thicker than this, a layer is not a board.
pub const BOARD_MAX: f64 = 0.08;
/// Saw cut, m.
pub const KERF: f64 = 0.003;

/// The board format of a layer's material, if it is laid in boards.
pub fn board_format(model: &Model, material: MaterialId, thickness: f64) -> Option<BoardFormat> {
    if !(0.004..=BOARD_MAX).contains(&thickness) {
        return None;
    }
    let m = model.material(material)?;
    let name = m.name.to_lowercase();
    if [
        "lana", "lámina", "membrana", "mortero", "tarima", "baldosa", "hormig",
    ]
    .iter()
    .any(|k| name.contains(k))
    {
        return None;
    }
    let cat = m.category.to_lowercase();
    FORMATS
        .iter()
        .find(|(keys, _)| keys.iter().any(|k| name.contains(k) || cat == *k))
        .map(|(_, f)| *f)
}

/// What a board piece is laid on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Host {
    Wall(u64),
    Slab(u64),
    Ceiling(u64),
}

impl Host {
    pub fn id(self) -> u64 {
        match self {
            Host::Wall(i) | Host::Slab(i) | Host::Ceiling(i) => i,
        }
    }
}

/// One board, whole or cut, in its place.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardPiece {
    /// «M3·2·05»: the host's mark, the layer (from outside or from the
    /// top, 1…) and the piece.
    pub mark: String,
    pub host: Host,
    pub layer: usize,
    pub material: MaterialId,
    pub thickness: f64,
    pub format: BoardFormat,
    /// Its corner on the host's face: along the wall from its start and
    /// up from its base (walls), or in the slab's axes (slabs), m.
    pub at: (f64, f64),
    /// Its size as cut (the rectangle that holds it), m.
    pub size: (f64, f64),
    /// Its area once the doors and windows are cut out, m².
    pub area: f64,
    /// Cut round a door or window (or the slab's outline), not only to
    /// length.
    pub notched: bool,
}

impl BoardPiece {
    pub fn whole(&self) -> bool {
        !self.notched
            && (self.size.0 - self.format.width).abs() < 1e-3
            && (self.size.1 - self.format.length).abs() < 1e-3
    }
}

/// A rectangle `[u0, v0, u1, v1]`.
type Rect = [f64; 4];

fn overlap(a: Rect, b: Rect) -> Option<Rect> {
    let r = [
        a[0].max(b[0]),
        a[1].max(b[1]),
        a[2].min(b[2]),
        a[3].min(b[3]),
    ];
    (r[2] - r[0] > 1e-6 && r[3] - r[1] > 1e-6).then_some(r)
}

fn area(r: Rect) -> f64 {
    (r[2] - r[0]) * (r[3] - r[1])
}

/// Boards of `f` over the rectangle `face`, in rows of `f.length` (or
/// columns: `upright` lays the length along v), starting at its corner
/// shifted by `shift` (a part of a board), less the `holes`.
fn lay(
    face: Rect,
    f: BoardFormat,
    upright: bool,
    shift: f64,
    holes: &[Rect],
) -> Vec<(Rect, f64, bool)> {
    let (bw, bl) = if upright {
        (f.width, f.length)
    } else {
        (f.length, f.width)
    };
    let mut out = Vec::new();
    let mut v = face[1];
    let mut row = 0usize;
    while v < face[3] - 1e-6 {
        // Rows stagger by half a board.
        let off = ((shift + 0.5 * (row % 2) as f64) % 1.0) * bw;
        let mut u = face[0] - off;
        while u < face[2] - 1e-6 {
            let sheet = [u, v, u + bw, v + bl];
            if let Some(r) = overlap(sheet, face) {
                let cut: f64 = holes.iter().filter_map(|h| overlap(*h, r)).map(area).sum();
                let left = area(r) - cut;
                if left > 0.005 {
                    out.push((r, left, cut > 1e-6));
                }
            }
            u += bw;
        }
        v += bl;
        row += 1;
    }
    out
}

/// The boards of a wall, layer by layer (from the outside in).
pub fn wall_boards(model: &Model, wall: archisim_core::WallId) -> Vec<BoardPiece> {
    let Some(w) = model.wall(wall) else {
        return Vec::new();
    };
    let Some(t) = model.wall_type(w.wall_type) else {
        return Vec::new();
    };
    let holes: Vec<Rect> = crate::openings_in_wall(model, wall)
        .iter()
        .map(|o| [o.t0, o.sill, o.t1, o.head])
        .collect();
    let face = [0.0, 0.0, w.length(), w.height];
    let mark = format!("M{}", wall.0);
    let mut out = Vec::new();
    let mut nth = 0usize;
    for (i, l) in t.layers.iter().enumerate() {
        let Some(f) = board_format(model, l.material, l.thickness) else {
            continue;
        };
        // Joints half a board off the layer before.
        let shift = 0.5 * (nth % 2) as f64;
        nth += 1;
        for (k, (r, a, notched)) in lay(face, f, true, shift, &holes).into_iter().enumerate() {
            out.push(BoardPiece {
                mark: format!("{mark}·{}·{:02}", i + 1, k + 1),
                host: Host::Wall(wall.0),
                layer: i,
                material: l.material,
                thickness: l.thickness,
                format: f,
                at: (r[0], r[1]),
                size: (r[2] - r[0], r[3] - r[1]),
                area: a,
                notched,
            });
        }
    }
    out
}

/// The axes boards are laid in over a plan outline: along its longest
/// side, from the corner that starts it.
fn plan_axes(outline: &[Point2]) -> (Point2, Point2, Point2) {
    let n = outline.len();
    let mut best = (0.0, 0usize);
    for i in 0..n {
        let d = (outline[(i + 1) % n] - outline[i]).length();
        if d > best.0 {
            best = (d, i);
        }
    }
    let a = outline[best.1];
    let u = (outline[(best.1 + 1) % n] - a)
        .normalized()
        .unwrap_or(Point2::new(1.0, 0.0));
    let v = Point2::new(-u.y, u.x);
    (a, u, v)
}

/// A polygon clipped to a rectangle (Sutherland–Hodgman: the rectangle is
/// convex), in the same axes.
fn clip(poly: &[(f64, f64)], r: Rect) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = poly.to_vec();
    for edge in 0..4 {
        let inside = |p: (f64, f64)| match edge {
            0 => p.0 >= r[0],
            1 => p.0 <= r[2],
            2 => p.1 >= r[1],
            _ => p.1 <= r[3],
        };
        let cross = |a: (f64, f64), b: (f64, f64)| {
            let t = match edge {
                0 => (r[0] - a.0) / (b.0 - a.0),
                1 => (r[2] - a.0) / (b.0 - a.0),
                2 => (r[1] - a.1) / (b.1 - a.1),
                _ => (r[3] - a.1) / (b.1 - a.1),
            };
            (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
        };
        let input = std::mem::take(&mut out);
        if input.is_empty() {
            break;
        }
        for i in 0..input.len() {
            let (a, b) = (input[i], input[(i + 1) % input.len()]);
            match (inside(a), inside(b)) {
                (true, true) => out.push(b),
                (true, false) => out.push(cross(a, b)),
                (false, true) => {
                    out.push(cross(a, b));
                    out.push(b);
                }
                _ => {}
            }
        }
    }
    out
}

fn poly_area(p: &[(f64, f64)]) -> f64 {
    let n = p.len();
    (0..n)
        .map(|i| p[i].0 * p[(i + 1) % n].1 - p[(i + 1) % n].0 * p[i].1)
        .sum::<f64>()
        .abs()
        / 2.0
}

/// The boards over a plan outline (with its holes), for a slab or a false
/// ceiling with these layers.
fn plan_boards(
    model: &Model,
    host: Host,
    mark: &str,
    outline: &[Point2],
    holes: &[Vec<Point2>],
    layers: &[archisim_core::Layer],
) -> Vec<BoardPiece> {
    if outline.len() < 3 {
        return Vec::new();
    }
    let (o, u, v) = plan_axes(outline);
    let local = |p: &Point2| {
        let d = *p - o;
        (d.x * u.x + d.y * u.y, d.x * v.x + d.y * v.y)
    };
    let poly: Vec<(f64, f64)> = outline.iter().map(local).collect();
    let holes: Vec<Vec<(f64, f64)>> = holes
        .iter()
        .map(|h| h.iter().map(local).collect())
        .collect();
    let (mut lo, mut hi) = ((f64::MAX, f64::MAX), (f64::MIN, f64::MIN));
    for p in &poly {
        lo = (lo.0.min(p.0), lo.1.min(p.1));
        hi = (hi.0.max(p.0), hi.1.max(p.1));
    }
    let face = [lo.0, lo.1, hi.0, hi.1];
    let whole = poly_area(&poly);
    let mut out = Vec::new();
    let mut nth = 0usize;
    for (i, l) in layers.iter().enumerate() {
        let Some(f) = board_format(model, l.material, l.thickness) else {
            continue;
        };
        let shift = 0.5 * (nth % 2) as f64;
        nth += 1;
        for (k, (r, _, _)) in lay(face, f, false, shift, &[]).into_iter().enumerate() {
            let inside = clip(&poly, r);
            let mut a = poly_area(&inside);
            for h in &holes {
                a -= poly_area(&clip(h, r));
            }
            if a < 0.005 {
                continue;
            }
            // Cut to a shape, not only to length: what is left of its
            // rectangle is not all of it.
            let notched = (a - area(r)).abs() > 1e-4 && whole > 0.0;
            out.push(BoardPiece {
                mark: format!("{mark}·{}·{:02}", i + 1, k + 1),
                host,
                layer: i,
                material: l.material,
                thickness: l.thickness,
                format: f,
                at: (r[0], r[1]),
                size: (r[2] - r[0], r[3] - r[1]),
                area: a,
                notched,
            });
        }
    }
    out
}

/// The boards of a slab (floor or roof), from the top down.
pub fn slab_boards(model: &Model, slab: archisim_core::SlabId) -> Vec<BoardPiece> {
    let Some(s) = model.slab(slab) else {
        return Vec::new();
    };
    let Some(t) = model.slab_type(s.slab_type) else {
        return Vec::new();
    };
    if t.function == SlabFunction::Ceiling {
        return Vec::new();
    }
    let mark = format!(
        "{}{}",
        if t.function == SlabFunction::Roof {
            "C"
        } else {
            "S"
        },
        slab.0
    );
    plan_boards(
        model,
        Host::Slab(slab.0),
        &mark,
        &s.outline,
        &s.holes,
        &t.layers,
    )
}

/// The boards of a false ceiling.
pub fn ceiling_boards(model: &Model, ceiling: archisim_core::CeilingId) -> Vec<BoardPiece> {
    let Some(c) = model.ceiling(ceiling) else {
        return Vec::new();
    };
    let Some(t) = model.slab_type(c.ceiling_type) else {
        return Vec::new();
    };
    let mark = format!("T{}", ceiling.0);
    plan_boards(
        model,
        Host::Ceiling(ceiling.0),
        &mark,
        &c.outline,
        &c.holes,
        &t.layers,
    )
}

/// The boards of a modular box: those of its walls, floors, roofs and
/// ceilings.
pub fn box_boards(model: &Model, id: BoxId) -> Vec<BoardPiece> {
    let held = model.box_members(id);
    let mut out = Vec::new();
    for w in &held.walls {
        out.extend(wall_boards(model, *w));
    }
    for s in &held.slabs {
        out.extend(slab_boards(model, *s));
    }
    for c in &held.ceilings {
        out.extend(ceiling_boards(model, *c));
    }
    out
}

/// The boards of the whole project.
pub fn all_boards(model: &Model) -> Vec<BoardPiece> {
    let mut out = Vec::new();
    for w in model.walls() {
        out.extend(wall_boards(model, w.id));
    }
    for s in model.slabs() {
        out.extend(slab_boards(model, s.id));
    }
    for c in model.ceilings() {
        out.extend(ceiling_boards(model, c.id));
    }
    out
}

/// Whole boards to buy for these pieces of one format: the whole ones,
/// and the cuts packed into boards (guillotine cuts, each piece turned if
/// it fits better).
pub fn boards_needed(pieces: &[&BoardPiece]) -> usize {
    let Some(f) = pieces.first().map(|p| p.format) else {
        return 0;
    };
    let mut whole = 0usize;
    let mut cuts: Vec<(f64, f64)> = Vec::new();
    for p in pieces {
        if p.whole() {
            whole += 1;
        } else {
            let (a, b) = (p.size.0.min(f.width), p.size.1.min(f.length));
            cuts.push((a.min(b), a.max(b)));
        }
    }
    cuts.sort_by(|a, b| (b.0 * b.1).total_cmp(&(a.0 * a.1)));
    // Free rectangles of the boards opened so far.
    let mut free: Vec<(f64, f64)> = Vec::new();
    let mut opened = 0usize;
    for (w, l) in cuts {
        // A cut may lose its saw cut: a half board is half a board.
        let e = KERF + 1e-6;
        let fits =
            |r: &(f64, f64)| (r.0 + e >= w && r.1 + e >= l) || (r.0 + e >= l && r.1 + e >= w);
        let best = free
            .iter()
            .enumerate()
            .filter(|(_, r)| fits(r))
            .min_by(|a, b| (a.1 .0 * a.1 .1).total_cmp(&(b.1 .0 * b.1 .1)))
            .map(|(i, _)| i);
        let r = match best {
            Some(i) => free.swap_remove(i),
            None => {
                opened += 1;
                (f.width, f.length)
            }
        };
        // Put it the way it fits, cut what is left in two.
        let (pw, pl) = if r.0 + e >= w && r.1 + e >= l {
            (w, l)
        } else {
            (l, w)
        };
        let right = (r.0 - pw - KERF, r.1);
        let top = (pw, r.1 - pl - KERF);
        for x in [right, top] {
            if x.0 > 0.05 && x.1 > 0.05 {
                free.push(x);
            }
        }
    }
    whole + opened
}

/// A line of the shopping list.
#[derive(Debug, Clone, PartialEq)]
pub struct Purchase {
    /// «Tableros», «Perfiles», «Junta», «Uniones», «Fijaciones»…
    pub group: &'static str,
    pub what: String,
    pub unit: &'static str,
    pub amount: f64,
}

/// Rolls of membrane, m × m.
pub const ROLL: (f64, f64) = (1.50, 50.0);
/// Overlap of membrane rolls.
pub const ROLL_OVERLAP: f64 = 0.10;
/// Screws of the boards, per m².
pub const BOARD_SCREWS: f64 = 20.0;
/// Packs of mineral wool, m² each.
pub const WOOL_PACK: f64 = 7.2;

fn push(
    out: &mut Vec<Purchase>,
    group: &'static str,
    what: String,
    unit: &'static str,
    amount: f64,
) {
    if amount <= 0.0 {
        return;
    }
    match out.iter_mut().find(|p| p.group == group && p.what == what) {
        Some(p) => p.amount += amount,
        None => out.push(Purchase {
            group,
            what,
            unit,
            amount,
        }),
    }
}

/// What a box takes, to buy: its boards by format, its frame's profiles in
/// commercial bars, the wool and membranes of its walls and slabs, the
/// joints and connections (M13), the chassis' steel and the fixings.
pub fn shopping_list(model: &Model, id: BoxId) -> Vec<Purchase> {
    let mut out = Vec::new();
    // Boards.
    let boards = box_boards(model, id);
    let mut kinds: Vec<(MaterialId, String, f64)> = Vec::new();
    for p in &boards {
        let key = (p.material, p.format.kind.to_string(), p.thickness);
        if !kinds
            .iter()
            .any(|k| k.0 == key.0 && (k.2 - key.2).abs() < 1e-9)
        {
            kinds.push(key);
        }
    }
    for (material, kind, thickness) in kinds {
        let these: Vec<&BoardPiece> = boards
            .iter()
            .filter(|p| p.material == material && (p.thickness - thickness).abs() < 1e-9)
            .collect();
        let f = these[0].format;
        let name = model
            .material(material)
            .map_or(kind.clone(), |m| m.name.clone());
        push(
            &mut out,
            "Tableros",
            format!(
                "{name} {} mm, {}×{} mm",
                (thickness * 1000.0).round(),
                (f.width * 1000.0).round(),
                (f.length * 1000.0).round()
            ),
            "ud",
            boards_needed(&these) as f64,
        );
    }
    // Profiles in bars, and the frame's screws.
    let frames = crate::framing::members_by_box(model);
    let mut studs = 0usize;
    if let Some(members) = frames.get(&id) {
        let list = crate::framing::cut_list(model, members);
        for t in &list.totals {
            push(
                &mut out,
                "Perfiles",
                format!(
                    "{} en barras de {} m",
                    t.section,
                    crate::framing::BAR_STEEL.to_string().replace('.', ",")
                ),
                "ud",
                t.bars as f64,
            );
        }
        studs = members
            .iter()
            .filter(|m| {
                use crate::framing::Role;
                matches!(
                    m.role,
                    Role::Stud | Role::KingStud | Role::JackStud | Role::Cripple
                )
            })
            .count();
    }
    // The boards' screws, about 20 a m².
    let board_area: f64 = boards.iter().map(|p| p.area).sum();
    push(
        &mut out,
        "Fijaciones",
        "Tornillo autotaladrante de placa".into(),
        "ud",
        board_area * BOARD_SCREWS,
    );
    // Wool and membranes of what the box holds, by area.
    let held = model.box_members(id);
    let mut layered: Vec<(Vec<archisim_core::Layer>, f64)> = Vec::new();
    for w in held.walls.iter().filter_map(|w| model.wall(*w)) {
        if let Some(t) = model.wall_type(w.wall_type) {
            let holes: f64 = crate::openings_in_wall(model, w.id)
                .iter()
                .map(|o| (o.t1 - o.t0).max(0.0) * (o.head - o.sill).max(0.0))
                .sum();
            layered.push((t.layers.clone(), (w.length() * w.height - holes).max(0.0)));
        }
    }
    for s in held.slabs.iter().filter_map(|s| model.slab(*s)) {
        if let Some(t) = model.slab_type(s.slab_type) {
            layered.push((
                t.layers.clone(),
                archisim_core::signed_area(&s.outline).abs(),
            ));
        }
    }
    for (layers, a) in layered {
        for l in layers {
            let Some(m) = model.material(l.material) else {
                continue;
            };
            let name = m.name.to_lowercase();
            if name.contains("lana") {
                push(
                    &mut out,
                    "Aislamiento",
                    format!(
                        "{} {} mm, paquetes de {} m²",
                        m.name,
                        (l.thickness * 1000.0).round(),
                        WOOL_PACK.to_string().replace('.', ",")
                    ),
                    "ud",
                    a / WOOL_PACK,
                );
            } else if l.function == LayerFunction::Membrane
                || name.contains("membrana")
                || name.contains("lámina")
            {
                push(
                    &mut out,
                    "Láminas",
                    format!(
                        "{}, rollos de {}×{} m",
                        m.name,
                        ROLL.0.to_string().replace('.', ","),
                        ROLL.1
                    ),
                    "ud",
                    a * (1.0 + ROLL_OVERLAP) / (ROLL.0 * ROLL.1),
                );
            }
        }
    }
    // The joints and connections, and the chassis.
    let t = crate::box_joints::joint_tally(model, id, studs);
    push(
        &mut out,
        "Junta",
        "Lana mineral de junta".into(),
        "m²",
        t.wool,
    );
    push(
        &mut out,
        "Junta",
        "Banda impermeable de 30 cm".into(),
        "m",
        t.roof_band,
    );
    push(
        &mut out,
        "Junta",
        "Membrana de junta de 20 cm con cinta".into(),
        "m",
        t.facade_band,
    );
    push(
        &mut out,
        "Junta",
        "Tapajuntas de aluminio".into(),
        "m",
        t.strips,
    );
    push(
        &mut out,
        "Uniones",
        "Placa de unión 200×100×10 mm".into(),
        "ud",
        t.plates as f64,
    );
    push(
        &mut out,
        "Uniones",
        "Tornillo M16 8.8 con tuerca y arandela".into(),
        "ud",
        t.plate_bolts as f64,
    );
    push(
        &mut out,
        "Uniones",
        "Orejeta de izado".into(),
        "ud",
        t.lugs as f64,
    );
    push(
        &mut out,
        "Uniones",
        "Placa de apoyo".into(),
        "ud",
        t.base_plates as f64,
    );
    push(
        &mut out,
        "Uniones",
        "Perno de anclaje M16 con resina".into(),
        "ud",
        t.anchors as f64,
    );
    push(
        &mut out,
        "Fijaciones",
        "Tornillo autotaladrante de montante".into(),
        "ud",
        t.screws as f64,
    );
    if let Some(b) = model.modular_box(id) {
        push(
            &mut out,
            "Chasis",
            "Tubo estructural de acero S275 para el chasis".into(),
            "kg",
            crate::boxes::chassis_weight(b),
        );
        push(
            &mut out,
            "Chasis",
            "Soldadura en ángulo".into(),
            "m",
            t.welds,
        );
    }
    // Whole units, rounded up.
    for p in &mut out {
        if p.unit == "ud" {
            p.amount = p.amount.ceil();
        }
    }
    out
}

/// The shopping list as CSV (`;`, comma decimals): one block per box and
/// the total.
pub fn shopping_csv(model: &Model) -> String {
    let mut s = String::from("Caja;Grupo;Concepto;Unidad;Cantidad\n");
    let mut total: Vec<Purchase> = Vec::new();
    let mut boxes: Vec<_> = model.boxes().collect();
    boxes.sort_by(|a, b| a.mark.cmp(&b.mark));
    for b in boxes {
        for p in shopping_list(model, b.id) {
            s.push_str(&format!(
                "{};{};{};{};{}\n",
                b.mark,
                p.group,
                p.what,
                p.unit,
                amount(&p)
            ));
            push(&mut total, p.group, p.what.clone(), p.unit, p.amount);
        }
    }
    for p in &total {
        s.push_str(&format!(
            "TOTAL;{};{};{};{}\n",
            p.group,
            p.what,
            p.unit,
            amount(p)
        ));
    }
    s
}

fn amount(p: &Purchase) -> String {
    if p.unit == "ud" {
        format!("{:.0}", p.amount)
    } else {
        format!("{:.2}", p.amount).replace('.', ",")
    }
}

/// The boards piece by piece, as CSV: mark, where, material, size as cut
/// and whether it is whole.
pub fn boards_csv(model: &Model) -> String {
    let mut s = String::from("Marca;Caja;Material;Espesor mm;Formato mm;Corte mm;Área m²;Pieza\n");
    let mut owner: std::collections::HashMap<u64, String> = Default::default();
    for b in model.boxes() {
        for id in model.box_members(b.id).ids() {
            owner.insert(id, b.mark.clone());
        }
    }
    for p in all_boards(model) {
        let mm = |v: f64| format!("{:.0}", v * 1000.0);
        s.push_str(&format!(
            "{};{};{};{};{}×{};{}×{};{};{}\n",
            p.mark,
            owner.get(&p.host.id()).cloned().unwrap_or_default(),
            model.material(p.material).map_or("", |m| m.name.as_str()),
            mm(p.thickness),
            mm(p.format.width),
            mm(p.format.length),
            mm(p.size.0),
            mm(p.size.1),
            format!("{:.3}", p.area).replace('.', ","),
            if p.whole() {
                "entera"
            } else if p.notched {
                "recortada"
            } else {
                "cortada"
            }
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{
        CutPattern, DoorOperation, Hand, Layer, NewOpening, NewOpeningType, NewWall, OpeningKind,
        ReferenceLine, WallFunction, WallSide, WallTop,
    };

    fn plaster_wall(length: f64) -> (Model, archisim_core::WallId) {
        let mut m = Model::new("T");
        let level = m.add_level("PB", 0.0);
        let gyp = m.add_material(
            "Placa de yeso laminado",
            "plaster",
            [240, 240, 236],
            CutPattern::Solid,
        );
        let wool = m.add_material(
            "Lana mineral",
            "insulation",
            [235, 225, 150],
            CutPattern::Solid,
        );
        let layer = |material, thickness, function| Layer {
            material,
            thickness,
            function,
        };
        let t = m
            .add_wall_type(
                "Tabique",
                WallFunction::Interior,
                vec![
                    layer(gyp, 0.015, LayerFunction::Finish),
                    layer(wool, 0.070, LayerFunction::Structure),
                    layer(gyp, 0.015, LayerFunction::Finish),
                ],
            )
            .unwrap();
        let w = m
            .add_wall(NewWall {
                wall_type: t,
                level,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(length, 0.0),
                bulge: 0.0,
                reference: ReferenceLine::Center,
                height: 2.60,
                base_offset: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        (m, w)
    }

    #[test]
    fn a_partition_in_whole_and_cut_boards() {
        let (m, w) = plaster_wall(6.0);
        let p = wall_boards(&m, w);
        // Two faces of plaster: 5 boards of 1,20 on one, 6 on the other
        // (half a board off), none of wool.
        let first: Vec<_> = p.iter().filter(|x| x.layer == 0).collect();
        let last: Vec<_> = p.iter().filter(|x| x.layer == 2).collect();
        assert_eq!(first.len(), 5);
        assert!(first.iter().all(|x| x.whole()));
        assert_eq!(last.len(), 6);
        assert!((last[0].size.0 - 0.60).abs() < 1e-9);
        let total: f64 = p.iter().map(|x| x.area).sum();
        assert!((total - 2.0 * 6.0 * 2.60).abs() < 1e-6);
        // To buy: 5 whole and the two halves from one more board.
        let refs: Vec<&BoardPiece> = last.to_vec();
        assert_eq!(boards_needed(&refs), 5);
    }

    #[test]
    fn boards_are_cut_round_a_door() {
        let (mut m, w) = plaster_wall(3.6);
        let board = m.add_material("Madera", "wood", [190, 150, 105], CutPattern::Solid);
        let ot = m
            .add_opening_type(NewOpeningType {
                name: "Puerta".into(),
                kind: OpeningKind::Door(DoorOperation::SingleSwing),
                width: 0.9,
                height: 2.1,
                frame_width: 0.05,
                frame_depth: 0.07,
                frame_material: board,
                panel_material: board,
            })
            .unwrap();
        m.add_opening(NewOpening {
            opening_type: ot,
            wall: w,
            along: 1.8,
            sill: 0.0,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        })
        .unwrap();
        let p = wall_boards(&m, w);
        assert!(p.iter().any(|x| x.notched));
        let total: f64 = p.iter().filter(|x| x.layer == 0).map(|x| x.area).sum();
        assert!(total < 3.6 * 2.60 - 1.5, "{total}");
    }

    #[test]
    fn cuts_pack_into_boards() {
        let f = FORMATS[0].1;
        let piece = |w: f64, l: f64| BoardPiece {
            mark: String::new(),
            host: Host::Wall(1),
            layer: 0,
            material: MaterialId(1),
            thickness: 0.015,
            format: f,
            at: (0.0, 0.0),
            size: (w, l),
            area: w * l,
            notched: false,
        };
        let v = [
            piece(0.6, 2.6),
            piece(0.6, 2.6),
            piece(0.3, 1.0),
            piece(0.3, 1.0),
        ];
        let refs: Vec<&BoardPiece> = v.iter().collect();
        assert_eq!(boards_needed(&refs), 2);
    }
}
