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

use archisim_core::{BoardJoints, BoxId, LayerFunction, MaterialId, Model, Point2, SlabFunction};

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

/// How a layer's material is laid: its format, upright or not, its joints
/// and its accent; the studio's layout (M18) or the usual one.
#[derive(Debug, Clone, PartialEq)]
pub struct Laying {
    pub format: BoardFormat,
    pub upright: bool,
    pub joints: BoardJoints,
    pub accent: Option<archisim_core::BoardAccent>,
}

/// How a layer is laid in boards, if it is: the studio's layout for its
/// material, or the usual format by its name, upright at half a board.
pub fn laying(model: &Model, material: MaterialId, thickness: f64) -> Option<Laying> {
    if let Some(l) = model.board_layout(material) {
        return Some(Laying {
            format: BoardFormat {
                kind: "Placa (despiece del estudio)",
                width: l.width,
                length: l.length,
            },
            upright: l.upright,
            joints: l.joints.clone(),
            accent: l.accent.clone(),
        });
    }
    board_format(model, material, thickness).map(|format| Laying {
        format,
        upright: true,
        joints: BoardJoints::HalfBond,
        accent: None,
    })
}

/// The board format of a layer's material, if it is laid in boards.
pub fn board_format(model: &Model, material: MaterialId, thickness: f64) -> Option<BoardFormat> {
    if let Some(l) = model.board_layout(material) {
        return Some(BoardFormat {
            kind: "Placa (despiece del estudio)",
            width: l.width,
            length: l.length,
        });
    }
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
    /// Its outline on the face when it is not its rectangle (a face
    /// composed by hand, M20), in the same axes as `at`.
    pub outline: Vec<(f64, f64)>,
    /// Fitted on site once the boxes stand, not in the factory (M20).
    pub on_site: bool,
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

/// A rectangle less some others, as rectangles.
fn subtract(r: Rect, holes: &[Rect]) -> Vec<Rect> {
    let mut parts = vec![r];
    for h in holes {
        let mut next = Vec::new();
        for p in parts {
            let Some(o) = overlap(p, *h) else {
                next.push(p);
                continue;
            };
            // Left and right full height, then below and above the hole.
            for q in [
                [p[0], p[1], o[0], p[3]],
                [o[2], p[1], p[2], p[3]],
                [o[0], p[1], o[2], o[1]],
                [o[0], o[3], o[2], p[3]],
            ] {
                if q[2] - q[0] > 1e-4 && q[3] - q[1] > 1e-4 {
                    next.push(q);
                }
            }
        }
        parts = next;
    }
    parts
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

/// A piece laid by [`lay_with`]: its rectangle, area left, whether it is
/// cut round a hole, and whether it takes the accent.
struct Laid {
    rect: Rect,
    area: f64,
    notched: bool,
    accent: bool,
}

/// Boards over the rectangle `face` as `laying` says, `shift` a part of a
/// board (the layer's offset), less the `holes`.
/// The narrowest piece a column of the studio's layout is left with at a
/// corner or a jamb (m): a narrower remnant is shared with the board next
/// to it, two equal pieces.
pub const MIN_PIECE: f64 = 0.30;

/// The columns of a studio layout across a face, as (from, to, frame):
/// a joint at each jamb of a door or window (and at the frame beside it,
/// `side`, which takes the accent), whole boards from the openings out to
/// the corners with the smaller pieces at the corners, and the run over an
/// opening or between two in equal pieces. No piece narrower than
/// [`MIN_PIECE`] unless the gap itself is.
pub fn layout_columns(face: Rect, bw: f64, holes: &[Rect], side: f64) -> Vec<(f64, f64, bool)> {
    let (a, b) = (face[0], face[2]);
    if b - a < 1e-6 || bw < 1e-6 {
        return Vec::new();
    }
    // The fixed joints: the jambs (and the frames), inside the face.
    let mut marks: Vec<f64> = vec![a, b];
    let mut frames: Vec<(f64, f64)> = Vec::new();
    let mut spans: Vec<(f64, f64)> = Vec::new();
    for h in holes {
        let (h0, h1) = (h[0].max(a), h[2].min(b));
        if h1 - h0 < 0.01 {
            continue;
        }
        spans.push((h0, h1));
        marks.extend([h0, h1]);
        if side > 0.01 {
            let (f0, f1) = ((h0 - side).max(a), (h1 + side).min(b));
            frames.push((f0, h0));
            frames.push((h1, f1));
            marks.extend([f0, f1]);
        }
    }
    marks.sort_by(|x, y| x.total_cmp(y));
    marks.dedup_by(|x, y| (*x - *y).abs() < 0.02);
    if let Some(last) = marks.last_mut() {
        *last = b;
    }
    let within = |u0: f64, u1: f64, r: &(f64, f64)| u0 >= r.0 - 0.02 && u1 <= r.1 + 0.02;
    let mut out = Vec::new();
    for w in marks.windows(2) {
        let (u0, u1) = (w[0], w[1]);
        let len = u1 - u0;
        if len < 0.005 {
            continue;
        }
        let frame =
            frames.iter().any(|r| within(u0, u1, r)) && !spans.iter().any(|r| within(u0, u1, r));
        let at_start = (u0 - a).abs() < 1e-6;
        let at_end = (u1 - b).abs() < 1e-6;
        let mut cuts: Vec<f64> = Vec::new();
        if frame || len <= bw + 1e-6 {
            cuts = vec![u0, u1];
        } else if at_start != at_end {
            // Out to a corner: whole boards from the opening, the remnant
            // at the corner.
            let n = (len / bw).floor() as usize;
            let rest = len - n as f64 * bw;
            let mut widths = vec![bw; n];
            if rest > 0.005 {
                if rest < MIN_PIECE && n > 0 {
                    let shared = (bw + rest) / 2.0;
                    widths.pop();
                    widths.extend([shared, shared]);
                } else {
                    widths.push(rest);
                }
            }
            if at_start {
                widths.reverse();
            }
            cuts.push(u0);
            for wd in widths {
                let v = cuts.last().unwrap() + wd;
                cuts.push(v);
            }
            *cuts.last_mut().unwrap() = u1;
        } else if at_start && at_end {
            // A face without openings: whole boards in the middle, the
            // remnant shared by the two corners.
            let n = (len / bw).floor() as usize;
            let rest = len - n as f64 * bw;
            let mut widths = vec![bw; n];
            if rest > 0.005 {
                let half = rest / 2.0;
                if half < MIN_PIECE && n > 0 {
                    let shared = (bw + rest) / 2.0;
                    widths.pop();
                    widths.insert(0, shared);
                    widths.push(shared);
                } else {
                    widths.insert(0, half);
                    widths.push(half);
                }
            }
            cuts.push(u0);
            for wd in widths {
                let v = cuts.last().unwrap() + wd;
                cuts.push(v);
            }
            *cuts.last_mut().unwrap() = u1;
        } else {
            // Over an opening or between two: equal pieces.
            let n = (len / bw - 1e-6).ceil().max(1.0) as usize;
            cuts = (0..=n).map(|k| u0 + len * k as f64 / n as f64).collect();
        }
        for c in cuts.windows(2) {
            out.push((c[0], c[1], frame));
        }
    }
    out
}

fn lay_with(face: Rect, laying: &Laying, shift: f64, holes: &[Rect]) -> Vec<Laid> {
    let f = laying.format;
    let side = laying.accent.as_ref().map_or(0.0, |a| a.side);
    let columns_of = |bw: f64, bl: f64, zipper: bool, lines: Option<&[f64]>| {
        let mut out: Vec<Laid> = Vec::new();
        let height = face[3] - face[1];
        for (c, (u0, u1, frame)) in layout_columns(face, bw, holes, side)
            .into_iter()
            .enumerate()
        {
            // The joints of this column, bottom to top.
            let mut cuts: Vec<f64> = vec![face[1]];
            match lines {
                Some(lines) => {
                    let mut l: Vec<f64> = lines
                        .iter()
                        .copied()
                        .filter(|h| *h > face[1] + 0.01 && *h < face[3] - 0.01)
                        .collect();
                    l.sort_by(|a, b| a.total_cmp(b));
                    l.push(face[3]);
                    for top in l {
                        // A band taller than a board: whole boards up.
                        let mut v = *cuts.last().unwrap();
                        while top - v > bl + 1e-6 {
                            v += bl;
                            cuts.push(v);
                        }
                        cuts.push(top);
                    }
                }
                None => {
                    let rest = height % bl;
                    let mut v = face[1];
                    if zipper && c % 2 == 1 && rest > 0.01 && height > bl {
                        // The cut piece at the bottom.
                        v += rest;
                        cuts.push(v);
                    }
                    while face[3] - v > 1e-6 {
                        v = (v + bl).min(face[3]);
                        cuts.push(v);
                    }
                }
            }
            for k in 0..cuts.len().saturating_sub(1) {
                let r = [u0, cuts[k], u1, cuts[k + 1]];
                let cut: f64 = holes.iter().filter_map(|h| overlap(*h, r)).map(area).sum();
                let left = area(r) - cut;
                // The accent: in the band of the doors' heads, in a column
                // a door or window crosses or in the frame beside one.
                let accent = laying.accent.as_ref().is_some_and(|a| {
                    let low = lines
                        .and_then(|l| l.iter().copied().reduce(f64::min))
                        .unwrap_or(face[1]);
                    r[1] >= low - 1e-6
                        && r[3] <= a.up_to + 1e-6
                        && (frame
                            || holes
                                .iter()
                                .any(|h| h[0] < r[2] - 1e-6 && h[2] > r[0] + 1e-6))
                });
                if left > 0.005 {
                    out.push(Laid {
                        rect: r,
                        area: left,
                        notched: cut > 1e-6,
                        accent,
                    });
                }
            }
        }
        let _ = shift;
        out
    };
    let (bw, bl) = if laying.upright {
        (f.width, f.length)
    } else {
        (f.length, f.width)
    };
    match &laying.joints {
        BoardJoints::HalfBond => lay(face, f, laying.upright, shift, holes)
            .into_iter()
            .map(|(rect, area, notched)| Laid {
                rect,
                area,
                notched,
                accent: false,
            })
            .collect(),
        BoardJoints::Zipper => columns_of(bw, bl, true, None),
        BoardJoints::Lines(lines) => columns_of(bw, bl, false, Some(lines)),
    }
}

/// How near a joint line must reach the edge of a piece to split it, m: a
/// line drawn to another one is taken as reaching it.
pub const JOINT_REACH: f64 = 0.02;

/// A polygon on a face, in the face's axes.
type FacePoly = Vec<(f64, f64)>;

/// A convex polygon cut by a line (through `a`, along `d`) in two: the side
/// on its left and the side on its right.
fn split_convex(poly: &[(f64, f64)], a: (f64, f64), d: (f64, f64)) -> (FacePoly, FacePoly) {
    let side = |p: (f64, f64)| d.0 * (p.1 - a.1) - d.1 * (p.0 - a.0);
    let (mut left, mut right) = (Vec::new(), Vec::new());
    let n = poly.len();
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let (sp, sq) = (side(p), side(q));
        if sp >= 0.0 {
            left.push(p);
        }
        if sp <= 0.0 {
            right.push(p);
        }
        if (sp > 0.0 && sq < 0.0) || (sp < 0.0 && sq > 0.0) {
            let t = sp / (sp - sq);
            let x = (p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t);
            left.push(x);
            right.push(x);
        }
    }
    (left, right)
}

/// Where a line (through `a`, along the unit `d`) goes through a convex
/// polygon: the distances along it, from `a`, it comes in and goes out.
fn chord(poly: &[(f64, f64)], a: (f64, f64), d: (f64, f64)) -> Option<(f64, f64)> {
    let side = |p: (f64, f64)| d.0 * (p.1 - a.1) - d.1 * (p.0 - a.0);
    let along = |p: (f64, f64)| (p.0 - a.0) * d.0 + (p.1 - a.1) * d.1;
    let mut ts: Vec<f64> = Vec::new();
    let n = poly.len();
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let (sp, sq) = (side(p), side(q));
        if (sp > 1e-9 && sq < -1e-9) || (sp < -1e-9 && sq > 1e-9) {
            let t = sp / (sp - sq);
            ts.push(along((p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t)));
        } else if sp.abs() <= 1e-9 {
            ts.push(along(p));
        }
    }
    let lo = ts.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = ts.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (hi - lo > 0.01).then_some((lo, hi))
}

/// The pieces a face (a rectangle `[u0, v0, u1, v1]`) is cut in by joint
/// lines drawn on it: each line splits the pieces it crosses from side to
/// side (reaching their edges within [`JOINT_REACH`]); one that stops short
/// of a piece's edge splits nothing there. Convex polygons, in the face's
/// axes, from the bottom left.
pub fn face_pieces(face: [f64; 4], joints: &[(Point2, Point2)]) -> Vec<Vec<(f64, f64)>> {
    let mut pieces: Vec<Vec<(f64, f64)>> = vec![vec![
        (face[0], face[1]),
        (face[2], face[1]),
        (face[2], face[3]),
        (face[0], face[3]),
    ]];
    // Again and again: a line may only reach across once another has cut.
    for _ in 0..8 {
        let mut changed = false;
        for (a, b) in joints {
            let len = (*b - *a).length();
            if len < 0.01 {
                continue;
            }
            let d = ((b.x - a.x) / len, (b.y - a.y) / len);
            let a = (a.x, a.y);
            let mut next = Vec::with_capacity(pieces.len() + 1);
            for p in pieces.drain(..) {
                let cuts = chord(&p, a, d).is_some_and(|(lo, hi)| {
                    lo >= -JOINT_REACH && hi <= len + JOINT_REACH && hi - lo > 0.01
                });
                if cuts {
                    let (l, r) = split_convex(&p, a, d);
                    if poly_area(&l) > 1e-4 && poly_area(&r) > 1e-4 {
                        next.push(l);
                        next.push(r);
                        changed = true;
                        continue;
                    }
                }
                next.push(p);
            }
            pieces = next;
        }
        if !changed {
            break;
        }
    }
    let low = |p: &Vec<(f64, f64)>| {
        let x = p.iter().map(|q| q.0).fold(f64::INFINITY, f64::min);
        let y = p.iter().map(|q| q.1).fold(f64::INFINITY, f64::min);
        (y, x)
    };
    pieces.sort_by(|a, b| {
        let (ya, xa) = low(a);
        let (yb, xb) = low(b);
        xa.total_cmp(&xb).then(ya.total_cmp(&yb))
    });
    pieces
}

/// Whether a point is inside a convex polygon (either way round).
pub fn inside_convex(poly: &[(f64, f64)], p: (f64, f64)) -> bool {
    let n = poly.len();
    let (mut pos, mut neg) = (false, false);
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let c = (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
        if c > 1e-9 {
            pos = true;
        } else if c < -1e-9 {
            neg = true;
        }
    }
    n >= 3 && !(pos && neg)
}

/// A convex polygon drawn in by `d` on every side (half the open joint
/// round a piece).
pub fn inset_convex(poly: &[(f64, f64)], d: f64) -> Vec<(f64, f64)> {
    let n = poly.len();
    if n < 3 || d <= 0.0 {
        return poly.to_vec();
    }
    let ccw = {
        let a: f64 = (0..n)
            .map(|i| poly[i].0 * poly[(i + 1) % n].1 - poly[(i + 1) % n].0 * poly[i].1)
            .sum();
        a > 0.0
    };
    // Each side moved in, as a point and a direction.
    let lines: Vec<((f64, f64), (f64, f64))> = (0..n)
        .filter_map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            let l = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
            if l < 1e-9 {
                return None;
            }
            let u = ((b.0 - a.0) / l, (b.1 - a.1) / l);
            let inward = if ccw { (-u.1, u.0) } else { (u.1, -u.0) };
            Some(((a.0 + inward.0 * d, a.1 + inward.1 * d), u))
        })
        .collect();
    let m = lines.len();
    let mut out = Vec::with_capacity(m);
    for i in 0..m {
        let (p, u) = lines[(i + m - 1) % m];
        let (q, v) = lines[i];
        let den = u.0 * v.1 - u.1 * v.0;
        if den.abs() < 1e-12 {
            out.push(q);
            continue;
        }
        let t = ((q.0 - p.0) * v.1 - (q.1 - p.1) * v.0) / den;
        out.push((p.0 + u.0 * t, p.1 + u.1 * t));
    }
    if poly_area(&out) < 1e-6 {
        return Vec::new();
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
    // On the face, along the wall from its start and up from the level.
    let holes: Vec<Rect> = crate::openings_in_wall(model, wall)
        .iter()
        .map(|o| [o.t0, o.sill, o.t1, o.head])
        .collect();
    let face = [0.0, w.base_offset, w.length(), w.base_offset + w.height];
    let mark = format!("M{}", wall.0);
    let mut out = Vec::new();
    let mut nth = 0usize;
    for (i, l) in t.layers.iter().enumerate() {
        let Some(laying) = laying(model, l.material, l.thickness) else {
            continue;
        };
        if let Some(by_hand) = model.board_face(wall, i) {
            // Composed by hand: the pieces its lines cut the face in.
            for (k, poly) in face_pieces(face, &by_hand.joints).into_iter().enumerate() {
                let cut: f64 = holes.iter().map(|h| poly_area(&clip(&poly, *h))).sum();
                let left = poly_area(&poly) - cut;
                if left <= 0.005 {
                    continue;
                }
                let mark_of = by_hand
                    .marks
                    .iter()
                    .find(|m| inside_convex(&poly, (m.at.x, m.at.y)));
                let material = mark_of.and_then(|m| m.material).unwrap_or(l.material);
                let format = if material == l.material {
                    laying.format
                } else {
                    let own = model.board_layout(material);
                    BoardFormat {
                        kind: "Acento",
                        width: own.map_or(laying.format.width, |o| o.width),
                        length: own.map_or(laying.format.length, |o| o.length),
                    }
                };
                let x0 = poly.iter().map(|q| q.0).fold(f64::INFINITY, f64::min);
                let x1 = poly.iter().map(|q| q.0).fold(f64::NEG_INFINITY, f64::max);
                let y0 = poly.iter().map(|q| q.1).fold(f64::INFINITY, f64::min);
                let y1 = poly.iter().map(|q| q.1).fold(f64::NEG_INFINITY, f64::max);
                let (sw, sl) = (x1 - x0, y1 - y0);
                let size = (sw, sl);
                out.push(BoardPiece {
                    mark: format!("{mark}·{}·{:02}", i + 1, k + 1),
                    host: Host::Wall(wall.0),
                    layer: i,
                    material,
                    thickness: l.thickness,
                    format,
                    at: (x0, y0),
                    size,
                    area: left,
                    notched: cut > 1e-6 || (poly_area(&poly) - sw * sl).abs() > 1e-4,
                    outline: poly,
                    on_site: mark_of.is_some_and(|m| m.on_site),
                });
            }
            nth += 1;
            continue;
        }
        // Joints half a board off the layer before (not where the studio
        // fixed the joints).
        let shift = if matches!(laying.joints, BoardJoints::HalfBond) {
            0.5 * (nth % 2) as f64
        } else {
            0.0
        };
        nth += 1;
        for (k, p) in lay_with(face, &laying, shift, &holes)
            .into_iter()
            .enumerate()
        {
            let (material, format) = match (&laying.accent, p.accent) {
                (Some(a), true) => {
                    // Its own commercial size, if the accent material has
                    // a layout.
                    let own = model.board_layout(a.material);
                    (
                        a.material,
                        BoardFormat {
                            kind: "Acento",
                            width: own.map_or(laying.format.width, |o| o.width),
                            length: own.map_or(laying.format.length, |o| o.length),
                        },
                    )
                }
                _ => (l.material, laying.format),
            };
            out.push(BoardPiece {
                mark: format!("{mark}·{}·{:02}", i + 1, k + 1),
                host: Host::Wall(wall.0),
                layer: i,
                material,
                thickness: l.thickness,
                format,
                at: (p.rect[0], p.rect[1]),
                size: (p.rect[2] - p.rect[0], p.rect[3] - p.rect[1]),
                area: p.area,
                notched: p.notched,
                outline: Vec::new(),
                on_site: false,
            });
        }
    }
    out
}

/// How near a joint can come to a jamb, a sill or a head without one of
/// them, m: closer, a sliver of board is left there.
pub const JAMB_GAP: f64 = 0.05;

/// What a face composed by hand warns of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaceWarningKind {
    /// A piece that does not come out of one board of its format.
    TooBig,
    /// A piece narrower than [`MIN_PIECE`].
    TooSmall,
    /// A joint a few centimetres off a jamb, a sill or a head.
    NearJamb,
    /// A joint over a joint of the layer next to it.
    OverJoint,
}

/// A warning of a face composed by hand, where it is on the face (along
/// the wall and up from the level): a piece (its outline) or a stretch of
/// joint.
#[derive(Debug, Clone, PartialEq)]
pub struct FaceWarning {
    pub kind: FaceWarningKind,
    pub outline: Vec<(f64, f64)>,
    pub text: String,
}

/// The joints of a layer as they stand: drawn by hand or by its rule.
pub fn layer_joints(
    model: &Model,
    wall: archisim_core::WallId,
    layer: usize,
) -> Vec<(Point2, Point2)> {
    match model.board_face(wall, layer) {
        Some(f) => f.joints.clone(),
        None => rule_joints(model, wall, layer),
    }
}

/// What a face of a wall's layer composed by hand (with these joints and
/// marks) warns of: pieces too big for their board or too small, joints
/// next to a jamb, and joints over those of the layers on either side.
pub fn face_warnings(
    model: &Model,
    wall: archisim_core::WallId,
    layer: usize,
    joints: &[(Point2, Point2)],
    marks: &[archisim_core::PieceMark],
) -> Vec<FaceWarning> {
    let mut out = Vec::new();
    let (Some(w), Some(t)) = (
        model.wall(wall),
        model.wall(wall).and_then(|w| model.wall_type(w.wall_type)),
    ) else {
        return out;
    };
    let Some(l) = t.layers.get(layer) else {
        return out;
    };
    let Some(lay) = laying(model, l.material, l.thickness) else {
        return out;
    };
    let holes: Vec<Rect> = crate::openings_in_wall(model, wall)
        .iter()
        .map(|o| [o.t0, o.sill, o.t1, o.head])
        .collect();
    let face = [0.0, w.base_offset, w.length(), w.base_offset + w.height];
    let cm = |v: f64| format!("{:.0}", v * 100.0);
    for poly in face_pieces(face, joints) {
        let cut: f64 = holes.iter().map(|h| poly_area(&clip(&poly, *h))).sum();
        if poly_area(&poly) - cut <= 0.005 {
            continue;
        }
        let material = marks
            .iter()
            .find(|m| inside_convex(&poly, (m.at.x, m.at.y)))
            .and_then(|m| m.material)
            .unwrap_or(l.material);
        let (fw, fl) = if material == l.material {
            (lay.format.width, lay.format.length)
        } else {
            model
                .board_layout(material)
                .map_or((lay.format.width, lay.format.length), |o| {
                    (o.width, o.length)
                })
        };
        let x0 = poly.iter().map(|q| q.0).fold(f64::INFINITY, f64::min);
        let x1 = poly.iter().map(|q| q.0).fold(f64::NEG_INFINITY, f64::max);
        let y0 = poly.iter().map(|q| q.1).fold(f64::INFINITY, f64::min);
        let y1 = poly.iter().map(|q| q.1).fold(f64::NEG_INFINITY, f64::max);
        let (a, b) = (x1 - x0, y1 - y0);
        let fits = |a: f64, b: f64| a <= fw + 1e-3 && b <= fl + 1e-3;
        if !fits(a, b) && !fits(b, a) {
            out.push(FaceWarning {
                kind: FaceWarningKind::TooBig,
                outline: poly.clone(),
                text: format!(
                    "Pieza de {}×{} cm: no sale de una placa de {}×{}",
                    cm(a),
                    cm(b),
                    cm(fw),
                    cm(fl)
                ),
            });
        }
        if a.min(b) < MIN_PIECE - 1e-3 {
            out.push(FaceWarning {
                kind: FaceWarningKind::TooSmall,
                outline: poly.clone(),
                text: format!(
                    "Pieza de {}×{} cm: más estrecha de {} cm",
                    cm(a),
                    cm(b),
                    cm(MIN_PIECE)
                ),
            });
        }
    }
    // Joints that come near a jamb, a sill or a head.
    let upright = |a: &Point2, b: &Point2| (a.x - b.x).abs() < 1e-3;
    let level = |a: &Point2, b: &Point2| (a.y - b.y).abs() < 1e-3;
    let overlap = |a0: f64, a1: f64, b0: f64, b1: f64| {
        a0.min(a1).max(b0.min(b1)) < a0.max(a1).min(b0.max(b1))
    };
    for (a, b) in joints {
        for h in &holes {
            let near = |v: f64, edge: f64| {
                let d = (v - edge).abs();
                d > 0.002 && d < JAMB_GAP
            };
            let hit = if upright(a, b) && overlap(a.y, b.y, h[1] - JAMB_GAP, h[3] + JAMB_GAP) {
                [h[0], h[2]]
                    .into_iter()
                    .find(|e| near(a.x, *e))
                    .map(|e| (a.x - e).abs())
            } else if level(a, b) && overlap(a.x, b.x, h[0] - JAMB_GAP, h[2] + JAMB_GAP) {
                [h[1], h[3]]
                    .into_iter()
                    .find(|e| near(a.y, *e))
                    .map(|e| (a.y - e).abs())
            } else {
                None
            };
            if let Some(d) = hit {
                out.push(FaceWarning {
                    kind: FaceWarningKind::NearJamb,
                    outline: vec![(a.x, a.y), (b.x, b.y)],
                    text: format!(
                        "Junta a {:.0} mm de un hueco: queda una tira de placa",
                        d * 1000.0
                    ),
                });
                break;
            }
        }
    }
    // Joints over the joints of the board layers on either side.
    let mut near_layers = Vec::new();
    for j in [layer.checked_sub(1), Some(layer + 1)]
        .into_iter()
        .flatten()
    {
        if let Some(n) = t.layers.get(j) {
            if laying(model, n.material, n.thickness).is_some() {
                near_layers.push(j);
            }
        }
    }
    for j in near_layers {
        let theirs = layer_joints(model, wall, j);
        for (a, b) in joints {
            for (c, d) in &theirs {
                let same = if upright(a, b) && upright(c, d) && (a.x - c.x).abs() < 0.01 {
                    let lo = a.y.min(b.y).max(c.y.min(d.y));
                    let hi = a.y.max(b.y).min(c.y.max(d.y));
                    (hi - lo > 0.05).then(|| vec![(a.x, lo), (a.x, hi)])
                } else if level(a, b) && level(c, d) && (a.y - c.y).abs() < 0.01 {
                    let lo = a.x.min(b.x).max(c.x.min(d.x));
                    let hi = a.x.max(b.x).min(c.x.max(d.x));
                    (hi - lo > 0.05).then(|| vec![(lo, a.y), (hi, a.y)])
                } else {
                    None
                };
                if let Some(line) = same {
                    out.push(FaceWarning {
                        kind: FaceWarningKind::OverJoint,
                        outline: line,
                        text: format!("Junta sobre la de la capa {}: conviene desplazarla", j + 1),
                    });
                }
            }
        }
    }
    out
}

/// Whether each end of a wall (start, end) meets a joint between boxes:
/// it is in a box and comes to one of its sides that touches another box.
/// The pieces there are the ones fitted on site once the boxes stand.
pub fn ends_at_joints(model: &Model, wall: archisim_core::WallId) -> (bool, bool) {
    let Some(w) = model.wall(wall) else {
        return (false, false);
    };
    let Some(b) = model.box_of(wall.0).and_then(|b| model.modular_box(b)) else {
        return (false, false);
    };
    const REACH: f64 = 0.35;
    let contacts = model.box_contacts(b.id);
    let at_joint = |p: Point2| {
        contacts.iter().any(|c| {
            let (s0, s1) = b.side(c.side);
            let d = s1 - s0;
            let len = d.length();
            if len < 1e-9 {
                return false;
            }
            let u = d * (1.0 / len);
            let t = (p - s0).dot(u);
            let off = (p - (s0 + u * t)).length();
            off < REACH && t > c.from - REACH && t < c.to + REACH
        })
    };
    (at_joint(w.start), at_joint(w.end))
}

/// The pieces of a face (these joints) at the ends of a wall that meet a
/// joint between boxes: the points inside them, to mark them on site.
pub fn joint_end_pieces(
    model: &Model,
    wall: archisim_core::WallId,
    joints: &[(Point2, Point2)],
) -> Vec<Point2> {
    let Some(w) = model.wall(wall) else {
        return Vec::new();
    };
    let (at_start, at_end) = ends_at_joints(model, wall);
    let len = w.length();
    let face = [0.0, w.base_offset, len, w.base_offset + w.height];
    face_pieces(face, joints)
        .into_iter()
        .filter(|poly| {
            let x0 = poly.iter().map(|q| q.0).fold(f64::INFINITY, f64::min);
            let x1 = poly.iter().map(|q| q.0).fold(f64::NEG_INFINITY, f64::max);
            (at_start && x0 < 1e-3) || (at_end && x1 > len - 1e-3)
        })
        .map(|poly| {
            let n = poly.len() as f64;
            Point2::new(
                poly.iter().map(|q| q.0).sum::<f64>() / n,
                poly.iter().map(|q| q.1).sum::<f64>() / n,
            )
        })
        .collect()
}

/// The joints of a wall's layer as its material's rule lays them, as lines
/// on the face (along the wall and up from the level), the ones in line
/// joined: where a face composed by hand starts from.
pub fn rule_joints(
    model: &Model,
    wall: archisim_core::WallId,
    layer: usize,
) -> Vec<(Point2, Point2)> {
    let Some(w) = model.wall(wall) else {
        return Vec::new();
    };
    let face = [0.0, w.base_offset, w.length(), w.base_offset + w.height];
    let mut m = model.clone();
    let _ = m.set_board_face(wall, layer, None);
    let mut level: Vec<(f64, f64, f64)> = Vec::new();
    let mut upright: Vec<(f64, f64, f64)> = Vec::new();
    let on_edge = |v: f64, a: f64, b: f64| (v - a).abs() < 1e-4 || (v - b).abs() < 1e-4;
    for p in wall_boards(&m, wall)
        .into_iter()
        .filter(|p| p.layer == layer)
    {
        let (x0, y0) = p.at;
        let (x1, y1) = (x0 + p.size.0, y0 + p.size.1);
        for y in [y0, y1] {
            if !on_edge(y, face[1], face[3]) {
                level.push((y, x0, x1));
            }
        }
        for x in [x0, x1] {
            if !on_edge(x, face[0], face[2]) {
                upright.push((x, y0, y1));
            }
        }
    }
    // Lines in line, touching or overlapping, become one.
    let join = |mut v: Vec<(f64, f64, f64)>| {
        v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
        let mut out: Vec<(f64, f64, f64)> = Vec::new();
        for s in v {
            match out.last_mut() {
                Some(l) if (l.0 - s.0).abs() < 1e-4 && s.1 <= l.2 + 1e-4 => l.2 = l.2.max(s.2),
                _ => out.push(s),
            }
        }
        out
    };
    let mut out = Vec::new();
    for (y, a, b) in join(level) {
        out.push((Point2::new(a, y), Point2::new(b, y)));
    }
    for (x, a, b) in join(upright) {
        out.push((Point2::new(x, a), Point2::new(x, b)));
    }
    out
}

/// How far the accent pieces stand out of the wall's exterior face, m:
/// enough to read in 3D and in the elevations.
pub const ACCENT_PROUD: f64 = 0.012;
/// How far the cladding's own pieces stand out, m.
pub const CLADDING_PROUD: f64 = 0.004;
/// The open joint between cladding pieces, m.
pub const CLADDING_JOINT: f64 = 0.006;

/// The pieces of the walls' outer cladding laid by joint lines (M18) as
/// thin plates on their exterior face with their open joints, the accent
/// ones in the accent's material and further out: the façade's drawing in
/// 3D and in the elevations. Straight walls only.
pub fn cladding_solids(model: &Model) -> Vec<crate::solid::Solid> {
    let mut out = Vec::new();
    for w in model.walls() {
        if w.bulge.abs() > 1e-9 {
            continue;
        }
        let Some(t) = model.wall_type(w.wall_type) else {
            continue;
        };
        let layout = t
            .layers
            .first()
            .and_then(|l| model.board_layout(l.material));
        let by_lines = layout.is_some_and(|l| matches!(l.joints, BoardJoints::Lines(_)));
        let by_hand = model.board_face(w.id, 0).is_some();
        if !by_lines && !by_hand {
            continue;
        }
        let joint = model
            .board_face(w.id, 0)
            .and_then(|f| f.joint)
            .or(layout.map(|l| l.joint))
            .unwrap_or(CLADDING_JOINT);
        let Some(u) = (w.end - w.start).normalized() else {
            continue;
        };
        let n = u.perp_left();
        let ext = w.face_offsets(t.thickness()).0;
        let lv = model.level(w.level).map_or(0.0, |l| l.elevation);
        let holes: Vec<Rect> = crate::openings_in_wall(model, w.id)
            .iter()
            .map(|o| [o.t0, o.sill, o.t1, o.head])
            .collect();
        for p in wall_boards(model, w.id)
            .into_iter()
            .filter(|p| p.layer == 0)
        {
            let proud = if p.format.kind == "Acento" {
                ACCENT_PROUD
            } else {
                CLADDING_PROUD
            };
            let j = joint / 2.0;
            if !p.outline.is_empty() {
                // A piece composed by hand: its outline drawn in by half
                // the joint, less the doors and windows.
                let poly = inset_convex(&p.outline, j);
                if poly.len() < 3 {
                    continue;
                }
                let pt = |q: (f64, f64)| Point2::new(q.0, q.1);
                let shape = (poly.iter().map(|q| pt(*q)).collect::<Vec<_>>(), Vec::new());
                let cutters: Vec<crate::junctions::Shape> = holes
                    .iter()
                    .map(|h| {
                        (
                            vec![
                                Point2::new(h[0], h[1]),
                                Point2::new(h[2], h[1]),
                                Point2::new(h[2], h[3]),
                                Point2::new(h[0], h[3]),
                            ],
                            Vec::new(),
                        )
                    })
                    .collect();
                let mut triangles = Vec::new();
                for (outer, inner) in crate::junctions::difference(&[shape], &cutters) {
                    let flat = crate::solid::with_holes(&outer, &inner);
                    // Along the wall, up, and out of the face: turned over
                    // so the faces look out.
                    for [a, b, c] in crate::solid::prism(&flat, 0.0, proud) {
                        let to = |q: [f64; 3]| {
                            let o = w.start + u * q[0] + n * (ext + q[2]);
                            [o.x, o.y, lv + q[1]]
                        };
                        triangles.push([to(a), to(c), to(b)]);
                    }
                }
                if !triangles.is_empty() {
                    out.push(crate::solid::Solid {
                        owner: crate::solid::SolidOwner::Wall(w.id),
                        material: p.material,
                        triangles,
                    });
                }
                continue;
            }
            let r = [
                p.at.0 + j,
                p.at.1 + j,
                p.at.0 + p.size.0 - j,
                p.at.1 + p.size.1 - j,
            ];
            let mut triangles = Vec::new();
            for q in subtract(r, &holes) {
                let o = w.start + u * q[0] + n * ext;
                crate::framing::cuboid(
                    &mut triangles,
                    [o.x, o.y, lv + q[1]],
                    [u.x * (q[2] - q[0]), u.y * (q[2] - q[0]), 0.0],
                    [n.x * proud, n.y * proud, 0.0],
                    [0.0, 0.0, q[3] - q[1]],
                );
            }
            if !triangles.is_empty() {
                out.push(crate::solid::Solid {
                    owner: crate::solid::SolidOwner::Wall(w.id),
                    material: p.material,
                    triangles,
                });
            }
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
        let Some(mut laying) = laying(model, l.material, l.thickness) else {
            continue;
        };
        let f = laying.format;
        // In plan, «upright» lays the boards' length along the slab's
        // short side; the usual format lies along its long side.
        if model.board_layout(l.material).is_none() {
            laying.upright = false;
        }
        let shift = if matches!(laying.joints, BoardJoints::HalfBond) {
            0.5 * (nth % 2) as f64
        } else {
            0.0
        };
        nth += 1;
        // Composed by hand in plan: the pieces its lines cut.
        if let Some(bp) = plan_host(host).and_then(|h| model.board_plan(h, i)) {
            let local_pt = |q: &Point2| {
                let (x, y) = local(q);
                Point2::new(x, y)
            };
            let joints: Vec<(Point2, Point2)> = bp
                .joints
                .iter()
                .map(|(a, b)| (local_pt(a), local_pt(b)))
                .collect();
            let mut k = 0;
            for piece in face_pieces(face, &joints) {
                let inside = clip_to_convex(&poly, &piece);
                let mut a = poly_area(&inside);
                for h in &holes {
                    a -= poly_area(&clip_to_convex(h, &piece));
                }
                if a < 0.005 || inside.len() < 3 {
                    continue;
                }
                k += 1;
                let r = bounds(&inside);
                let pm = bp
                    .marks
                    .iter()
                    .find(|m| inside_convex(&piece, local(&m.at)));
                out.push(BoardPiece {
                    mark: format!("{mark}·{}·{:02}", i + 1, k),
                    host,
                    layer: i,
                    material: pm.and_then(|m| m.material).unwrap_or(l.material),
                    thickness: l.thickness,
                    format: f,
                    at: (r[0], r[1]),
                    size: (r[2] - r[0], r[3] - r[1]),
                    area: a,
                    notched: (a - area(r)).abs() > 1e-4,
                    outline: inside,
                    on_site: pm.is_some_and(|m| m.on_site),
                });
            }
            continue;
        }
        let laid: Vec<Rect> = lay_with(face, &laying, shift, &[])
            .into_iter()
            .map(|p| p.rect)
            .collect();
        for (k, r) in laid.into_iter().enumerate() {
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
                outline: Vec::new(),
                on_site: false,
            });
        }
    }
    out
}

/// The slab or ceiling a host of boards in plan is.
fn plan_host(host: Host) -> Option<archisim_core::PlanHost> {
    match host {
        Host::Slab(id) => Some(archisim_core::PlanHost::Slab(archisim_core::SlabId(id))),
        Host::Ceiling(id) => Some(archisim_core::PlanHost::Ceiling(archisim_core::CeilingId(
            id,
        ))),
        _ => None,
    }
}

/// The rectangle that holds a polygon.
fn bounds(p: &[(f64, f64)]) -> Rect {
    let mut r = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for q in p {
        r = [r[0].min(q.0), r[1].min(q.1), r[2].max(q.0), r[3].max(q.1)];
    }
    r
}

/// A polygon (convex or not) clipped to a convex one, in the same axes.
fn clip_to_convex(subject: &[(f64, f64)], convex: &[(f64, f64)]) -> FacePoly {
    let n = convex.len();
    if n < 3 {
        return Vec::new();
    }
    let ccw = (0..n)
        .map(|i| convex[i].0 * convex[(i + 1) % n].1 - convex[(i + 1) % n].0 * convex[i].1)
        .sum::<f64>()
        > 0.0;
    let mut out = subject.to_vec();
    for i in 0..n {
        let (a, b) = (convex[i], convex[(i + 1) % n]);
        let (l, r) = split_convex(&out, a, (b.0 - a.0, b.1 - a.1));
        out = if ccw { l } else { r };
        if out.len() < 3 {
            return Vec::new();
        }
    }
    out
}

/// Whether a point is inside a polygon, convex or not.
pub fn inside_piece(poly: &[(f64, f64)], p: (f64, f64)) -> bool {
    let n = poly.len();
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < a.0 + (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) {
            inside = !inside;
        }
    }
    inside
}

/// The pieces a slab's or a ceiling's layer is cut in by lines drawn in
/// plan (as they are drawn, before they are kept), in plan: each piece of
/// the face as the lines cut it, clipped to the outline.
pub fn plan_pieces(
    model: &Model,
    host: archisim_core::PlanHost,
    joints: &[(Point2, Point2)],
) -> Vec<FacePoly> {
    let Some((outline, _)) = model.plan_host_outline(host) else {
        return Vec::new();
    };
    if outline.len() < 3 {
        return Vec::new();
    }
    let (o, u, v) = plan_axes(outline);
    let local = |p: &Point2| {
        let d = *p - o;
        (d.x * u.x + d.y * u.y, d.x * v.x + d.y * v.y)
    };
    let back = |q: (f64, f64)| {
        let p = o + u * q.0 + v * q.1;
        (p.x, p.y)
    };
    let poly: Vec<(f64, f64)> = outline.iter().map(local).collect();
    let face = bounds(&poly);
    let joints: Vec<(Point2, Point2)> = joints
        .iter()
        .map(|(a, b)| {
            let (a, b) = (local(a), local(b));
            (Point2::new(a.0, a.1), Point2::new(b.0, b.1))
        })
        .collect();
    face_pieces(face, &joints)
        .into_iter()
        .map(|piece| clip_to_convex(&poly, &piece))
        .filter(|p| p.len() >= 3 && poly_area(p) > 1e-3)
        .map(|p| p.into_iter().map(back).collect())
        .collect()
}

/// What a slab's or a ceiling's layer composed in plan warns of: pieces
/// too big for their board (red) or narrower than `MIN_PIECE` (amber),
/// measured along the slab's own axes. Outlines in plan.
pub fn plan_warnings(
    model: &Model,
    host: archisim_core::PlanHost,
    layer: usize,
    joints: &[(Point2, Point2)],
    marks: &[archisim_core::PieceMark],
) -> Vec<FaceWarning> {
    let mut out = Vec::new();
    let (Some(layers), Some((outline, _))) =
        (model.plan_host_layers(host), model.plan_host_outline(host))
    else {
        return out;
    };
    let Some(l) = layers.get(layer) else {
        return out;
    };
    let Some(lay) = laying(model, l.material, l.thickness) else {
        return out;
    };
    let (o, u, v) = plan_axes(outline);
    let local = |p: (f64, f64)| {
        let d = Point2::new(p.0, p.1) - o;
        (d.x * u.x + d.y * u.y, d.x * v.x + d.y * v.y)
    };
    let cm = |v: f64| format!("{:.0}", v * 100.0);
    for poly in plan_pieces(model, host, joints) {
        let material = marks
            .iter()
            .find(|m| inside_piece(&poly, (m.at.x, m.at.y)))
            .and_then(|m| m.material)
            .unwrap_or(l.material);
        let (fw, fl) = if material == l.material {
            (lay.format.width, lay.format.length)
        } else {
            model
                .board_layout(material)
                .map_or((lay.format.width, lay.format.length), |o| {
                    (o.width, o.length)
                })
        };
        let r = bounds(&poly.iter().map(|q| local(*q)).collect::<Vec<_>>());
        let (a, b) = (r[2] - r[0], r[3] - r[1]);
        let fits = |a: f64, b: f64| a <= fw + 1e-3 && b <= fl + 1e-3;
        if !fits(a, b) && !fits(b, a) {
            out.push(FaceWarning {
                kind: FaceWarningKind::TooBig,
                outline: poly.clone(),
                text: format!(
                    "Pieza de {}×{} cm: no sale de una placa de {}×{}",
                    cm(a),
                    cm(b),
                    cm(fw),
                    cm(fl)
                ),
            });
        }
        if a.min(b) < MIN_PIECE - 1e-3 {
            out.push(FaceWarning {
                kind: FaceWarningKind::TooSmall,
                outline: poly,
                text: format!(
                    "Pieza de {}×{} cm: más estrecha de {} cm",
                    cm(a),
                    cm(b),
                    cm(MIN_PIECE)
                ),
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
    // Boards: those of the factory; the ones fitted on site (M20) apart,
    // counted, to go with the box.
    let (on_site, boards): (Vec<BoardPiece>, Vec<BoardPiece>) =
        box_boards(model, id).into_iter().partition(|p| p.on_site);
    for p in &on_site {
        let name = model
            .material(p.material)
            .map_or(p.format.kind.to_string(), |m| m.name.clone());
        push(
            &mut out,
            "En obra",
            format!(
                "{name} {} mm, piezas para colocar en obra",
                (p.thickness * 1000.0).round()
            ),
            "ud",
            1.0,
        );
    }
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
    // The services: their joints at the box's joints and the grommets of
    // the holes in its studs (M6).
    for c in crate::box_services::joint_crossings(model)
        .into_iter()
        .filter(|c| c.a == id)
    {
        push(
            &mut out,
            "Instalaciones",
            format!("{} ({})", c.kind.label(), c.size),
            "ud",
            1.0,
        );
        push(
            &mut out,
            "Instalaciones",
            "Registro de la unión en la junta".into(),
            "ud",
            1.0,
        );
    }
    let walls = &model.box_members(id).walls;
    let grommets = crate::box_services::stud_holes(model)
        .iter()
        .filter(|h| walls.contains(&h.wall))
        .count();
    push(
        &mut out,
        "Instalaciones",
        "Pasamuros de goma para los pasos por montantes".into(),
        "ud",
        grommets as f64,
    );
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
    let mut s =
        String::from("Marca;Caja;Material;Espesor mm;Formato mm;Corte mm;Área m²;Pieza;Dónde\n");
    let mut owner: std::collections::HashMap<u64, String> = Default::default();
    for b in model.boxes() {
        for id in model.box_members(b.id).ids() {
            owner.insert(id, b.mark.clone());
        }
    }
    for p in all_boards(model) {
        let mm = |v: f64| format!("{:.0}", v * 1000.0);
        s.push_str(&format!(
            "{};{};{};{};{}×{};{}×{};{};{};{}\n",
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
            },
            if p.on_site { "en obra" } else { "fábrica" }
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
        plaster_wall_of(length, 2.60)
    }

    #[test]
    fn a_face_composed_by_hand_follows_its_lines() {
        let (mut m, w) = plaster_wall_of(3.6, 2.7);
        let wood = m.add_material("Lama de madera", "wood", [160, 110, 70], CutPattern::Solid);
        let p = |x: f64, y: f64| Point2::new(x, y);
        // Two upright joints, a level one only in the middle column (drawn
        // to the uprights), and one that stops short: it splits nothing.
        let face = archisim_core::BoardFace {
            wall: w,
            layer: 0,
            joints: vec![
                (p(1.2, 0.0), p(1.2, 2.7)),
                (p(2.4, 0.0), p(2.4, 2.7)),
                (p(1.2, 2.0), p(2.4, 2.0)),
                (p(0.2, 1.0), p(0.8, 1.0)),
            ],
            marks: vec![
                archisim_core::PieceMark {
                    at: p(1.8, 2.3),
                    material: Some(wood),
                    on_site: false,
                },
                archisim_core::PieceMark {
                    at: p(3.0, 1.0),
                    material: None,
                    on_site: true,
                },
            ],
            joint: None,
        };
        m.set_board_face(w, 0, Some(face)).unwrap();
        let pieces: Vec<_> = wall_boards(&m, w)
            .into_iter()
            .filter(|b| b.layer == 0)
            .collect();
        assert_eq!(pieces.len(), 4, "{pieces:?}");
        let accent = pieces.iter().find(|b| b.material == wood).unwrap();
        assert!((accent.size.0 - 1.2).abs() < 1e-6 && (accent.size.1 - 0.7).abs() < 1e-6);
        assert_eq!(accent.format.kind, "Acento");
        let site: Vec<_> = pieces.iter().filter(|b| b.on_site).collect();
        assert_eq!(site.len(), 1);
        assert!((site[0].at.0 - 2.4).abs() < 1e-6);
        let total: f64 = pieces.iter().map(|b| b.area).sum();
        assert!((total - 3.6 * 2.7).abs() < 1e-6);
        // The other layers keep their rule.
        assert!(wall_boards(&m, w)
            .iter()
            .any(|b| b.layer == 2 && b.outline.is_empty()));
    }

    #[test]
    fn a_joint_drawn_short_of_an_edge_cuts_nothing() {
        let face = [0.0, 0.0, 2.0, 2.0];
        let p = |x: f64, y: f64| Point2::new(x, y);
        assert_eq!(face_pieces(face, &[(p(0.5, 0.0), p(0.5, 1.5))]).len(), 1);
        assert_eq!(face_pieces(face, &[(p(0.5, -0.01), p(0.5, 2.01))]).len(), 2);
        // A slanted joint makes two triangles' worth of pieces.
        let slant = face_pieces(face, &[(p(0.0, 0.0), p(2.0, 2.0))]);
        assert_eq!(slant.len(), 2);
        let inset = inset_convex(&slant[0], 0.003);
        assert!(poly_area(&inset) < poly_area(&slant[0]));
    }

    #[test]
    fn a_face_drawn_from_its_rule_has_the_rules_pieces() {
        let (mut m, w) = plaster_wall_of(3.6, 2.7);
        let before: Vec<_> = wall_boards(&m, w)
            .into_iter()
            .filter(|b| b.layer == 0)
            .collect();
        let joints = rule_joints(&m, w, 0);
        assert!(!joints.is_empty());
        m.set_board_face(
            w,
            0,
            Some(archisim_core::BoardFace {
                wall: w,
                layer: 0,
                joints,
                marks: Vec::new(),
                joint: None,
            }),
        )
        .unwrap();
        let after: Vec<_> = wall_boards(&m, w)
            .into_iter()
            .filter(|b| b.layer == 0)
            .collect();
        assert_eq!(before.len(), after.len());
        let area = |v: &[BoardPiece]| v.iter().map(|b| b.area).sum::<f64>();
        assert!((area(&before) - area(&after)).abs() < 1e-6);
    }

    #[test]
    fn a_face_by_hand_warns_of_what_will_not_go() {
        let (m, w) = plaster_wall_of(3.6, 2.5);
        let p = |x: f64, y: f64| Point2::new(x, y);
        // Uprights at 1,2 and 1,4: a sliver of 20 cm; nothing at 3,6: the
        // last piece is 2,2 wide, more than a board.
        let joints = vec![(p(1.2, 0.0), p(1.2, 2.5)), (p(1.4, 0.0), p(1.4, 2.5))];
        let warn = face_warnings(&m, w, 0, &joints, &[]);
        let kinds: Vec<_> = warn.iter().map(|x| x.kind).collect();
        assert!(kinds.contains(&FaceWarningKind::TooSmall), "{warn:?}");
        assert!(kinds.contains(&FaceWarningKind::TooBig), "{warn:?}");
        // The inner layer's joints are those of its rule, every 1,20 from
        // the start: the joint at 1,20 is over one of them.
        assert!(kinds.contains(&FaceWarningKind::OverJoint) || kinds.len() >= 2);
    }

    #[test]
    fn a_slab_composed_by_hand_in_plan() {
        let mut m = Model::new("T");
        let level = m.add_level("PB", 0.0);
        let osb = m.add_material("Tablero OSB", "wood", [200, 170, 120], CutPattern::Solid);
        let st = m
            .add_slab_type(
                "Suelo",
                archisim_core::SlabFunction::Floor,
                vec![Layer {
                    material: osb,
                    thickness: 0.018,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let p = |x: f64, y: f64| Point2::new(x, y);
        let s = m
            .add_slab(archisim_core::NewSlab {
                slab_type: st,
                level,
                outline: vec![p(0.0, 0.0), p(4.0, 0.0), p(4.0, 3.0), p(0.0, 3.0)],
                top_offset: 0.0,
            })
            .unwrap();
        let host = archisim_core::PlanHost::Slab(s);
        let by_rule = slab_boards(&m, s).len();
        assert!(by_rule > 0);
        // One line across at x = 1,2 (it overshoots: it still cuts) and
        // one that stops short: two pieces, the narrow one on site.
        let joints = vec![(p(1.2, -0.5), p(1.2, 3.5)), (p(2.0, 1.0), p(3.0, 1.0))];
        assert_eq!(plan_pieces(&m, host, &joints).len(), 2);
        m.set_board_plan(
            host,
            0,
            Some(archisim_core::BoardPlan {
                host,
                layer: 0,
                joints: joints.clone(),
                marks: vec![archisim_core::PieceMark {
                    at: p(0.6, 1.5),
                    material: None,
                    on_site: true,
                }],
                joint: None,
            }),
        )
        .unwrap();
        let pieces = slab_boards(&m, s);
        assert_eq!(pieces.len(), 2, "{pieces:?}");
        assert_eq!(pieces.iter().filter(|b| b.on_site).count(), 1);
        let total: f64 = pieces.iter().map(|b| b.area).sum();
        assert!((total - 12.0).abs() < 1e-6);
        // 2,80 × 3,00 does not come out of a board of OSB.
        let warn = plan_warnings(&m, host, 0, &joints, &[]);
        assert!(
            warn.iter().any(|w| w.kind == FaceWarningKind::TooBig),
            "{warn:?}"
        );
    }

    fn plaster_wall_of(length: f64, height: f64) -> (Model, archisim_core::WallId) {
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
                height,
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
            outline: Vec::new(),
            on_site: false,
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

    #[test]
    fn a_zipper_puts_the_cut_piece_up_and_down_by_turns() {
        let (mut m, w) = plaster_wall_of(3.6, 3.58);
        let gyp = m.materials().find(|x| x.name.contains("yeso")).unwrap().id;
        m.set_board_layout(
            gyp,
            Some(archisim_core::BoardLayout {
                width: 1.2,
                length: 3.0,
                upright: true,
                joint: 0.0,
                joints: BoardJoints::Zipper,
                accent: None,
            }),
        )
        .unwrap();
        let p: Vec<_> = wall_boards(&m, w)
            .into_iter()
            .filter(|x| x.layer == 0)
            .collect();
        // Three columns, two pieces each: 3,00 + 0,58 and 0,58 + 3,00.
        assert_eq!(p.len(), 6);
        let first: Vec<_> = p.iter().filter(|x| x.at.0.abs() < 1e-9).collect();
        let second: Vec<_> = p.iter().filter(|x| (x.at.0 - 1.2).abs() < 1e-9).collect();
        assert!((first[0].size.1 - 3.0).abs() < 1e-9, "{first:?}");
        assert!((second[0].size.1 - 0.58).abs() < 1e-9, "{second:?}");
    }

    #[test]
    fn columns_frame_the_openings_and_leave_no_slivers() {
        let widths = |c: &[(f64, f64, bool)]| -> Vec<f64> {
            c.iter()
                .map(|x| ((x.1 - x.0) * 100.0).round() / 100.0)
                .collect()
        };
        // A blank face of 6,10: whole boards, the remnant at both corners.
        let c = layout_columns([0.0, 0.0, 6.1, 3.0], 1.2, &[], 0.0);
        assert_eq!(widths(&c), vec![0.65, 1.2, 1.2, 1.2, 1.2, 0.65]);
        // 3,70: a 0,05 remnant is shared with the last board, not left.
        let c = layout_columns([0.0, 0.0, 3.7, 3.0], 1.2, &[], 0.0);
        assert!(widths(&c).iter().all(|w| *w >= MIN_PIECE), "{c:?}");
        // A 3,00 window with a 0,30 frame each side, at 1,50–4,50 of 6,10.
        let c = layout_columns([0.0, 0.0, 6.1, 3.0], 1.2, &[[1.5, 0.9, 4.5, 2.5]], 0.3);
        assert_eq!(widths(&c), vec![1.2, 0.3, 1.0, 1.0, 1.0, 0.3, 0.65, 0.65]);
        assert_eq!(c.iter().filter(|x| x.2).count(), 2);
        // Joints at the jambs and the frames.
        for x in [1.2, 1.5, 4.5, 4.8] {
            assert!(c.iter().any(|k| (k.0 - x).abs() < 1e-9), "{x}");
        }
    }

    #[test]
    fn a_cladding_by_lines_frames_the_window_in_wood() {
        let (mut m, w) = plaster_wall(3.6);
        let gyp = m.materials().find(|x| x.name.contains("yeso")).unwrap().id;
        let wood = m.add_material("Madera", "wood", [160, 110, 70], CutPattern::Solid);
        m.set_board_layout(
            gyp,
            Some(archisim_core::BoardLayout {
                width: 1.2,
                length: 2.5,
                upright: true,
                joint: 0.0,
                joints: BoardJoints::Lines(vec![0.0, 2.0]),
                accent: Some(archisim_core::BoardAccent {
                    material: wood,
                    up_to: 2.0,
                    side: 0.0,
                }),
            }),
        )
        .unwrap();
        let board = m.add_material("Marco", "wood", [190, 150, 105], CutPattern::Solid);
        let ot = m
            .add_opening_type(NewOpeningType {
                name: "Ventana".into(),
                kind: OpeningKind::Door(DoorOperation::SingleSwing),
                width: 0.8,
                height: 1.0,
                frame_width: 0.05,
                frame_depth: 0.07,
                frame_material: board,
                panel_material: board,
            })
            .unwrap();
        // In the middle column.
        m.add_opening(NewOpening {
            opening_type: ot,
            wall: w,
            along: 1.8,
            sill: 0.9,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        })
        .unwrap();
        let p: Vec<_> = wall_boards(&m, w)
            .into_iter()
            .filter(|x| x.layer == 0)
            .collect();
        // Bands 0–2,0 and 2,0–2,6; the column under the window (its joints
        // at the jambs) is wood in the lower band.
        let wood_pieces: Vec<_> = p.iter().filter(|x| x.material == wood).collect();
        assert_eq!(wood_pieces.len(), 1, "{p:?}");
        assert!((wood_pieces[0].at.0 - 1.4).abs() < 1e-9);
        assert!((wood_pieces[0].size.1 - 2.0).abs() < 1e-9);
        assert!(p
            .iter()
            .any(|x| (x.at.1 - 2.0).abs() < 1e-9 && (x.size.1 - 0.6).abs() < 1e-9));
    }
}
