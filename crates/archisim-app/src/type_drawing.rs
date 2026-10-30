//! Elevations and sections of types for the types dialog, as small vector
//! drawings: a door or window seen from the side its leaves open to, or cut
//! through its first leaf with the host wall above and below; the layers
//! of a wall or slab type cut vertically. Drawn from the same 3D body the
//! model, the 3D view and the IFC use (the family's parts), so what the
//! dialog shows is what is built.
//!
//! Coordinates are metres on the paper of the drawing: `x` to the right,
//! `y` up. The dialog paints them (`app/types_ui`) on white paper.

use archisim_core::{CutPattern, Layer, Model, OpeningType};

use crate::type_preview::opening_model;

/// Line weights, as the plan's pens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pen {
    /// Things seen beyond the cut, hatches.
    Thin,
    /// Outlines of what is seen in elevation.
    Medium,
    /// Outlines of what is cut.
    Thick,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// A rectangle, filled (`fill` with alpha) and outlined; `pattern` is
    /// the cut hatch of a material cut by the section.
    Rect {
        min: [f64; 2],
        max: [f64; 2],
        fill: [u8; 4],
        pattern: Option<CutPattern>,
        pen: Pen,
    },
    Line {
        from: [f64; 2],
        to: [f64; 2],
        pen: Pen,
        dashed: bool,
    },
    /// A closed outline (a piece of an arch), filled, and drawn round
    /// with `pen` if any.
    Polygon {
        points: Vec<[f64; 2]>,
        fill: [u8; 4],
        pen: Option<Pen>,
    },
}

/// A drawing and the region it covers.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawing {
    pub items: Vec<Item>,
    pub lo: [f64; 2],
    pub hi: [f64; 2],
    /// What is left and right of the drawing ("Exterior", "Interior"),
    /// empty when it does not apply.
    pub sides: (&'static str, &'static str),
}

impl Drawing {
    fn new() -> Self {
        Drawing {
            items: Vec::new(),
            lo: [f64::INFINITY; 2],
            hi: [f64::NEG_INFINITY; 2],
            sides: ("", ""),
        }
    }

    fn grow(&mut self, p: [f64; 2]) {
        self.lo = [self.lo[0].min(p[0]), self.lo[1].min(p[1])];
        self.hi = [self.hi[0].max(p[0]), self.hi[1].max(p[1])];
    }

    fn rect(
        &mut self,
        min: [f64; 2],
        max: [f64; 2],
        fill: [u8; 4],
        pattern: Option<CutPattern>,
        pen: Pen,
    ) {
        if max[0] - min[0] < 1e-6 || max[1] - min[1] < 1e-6 {
            return;
        }
        self.grow(min);
        self.grow(max);
        self.items.push(Item::Rect {
            min,
            max,
            fill,
            pattern,
            pen,
        });
    }

    fn polygon(&mut self, points: Vec<[f64; 2]>, fill: [u8; 4], pen: Option<Pen>) {
        if points.len() < 3 {
            return;
        }
        for p in &points {
            self.grow(*p);
        }
        self.items.push(Item::Polygon { points, fill, pen });
    }

    fn line(&mut self, from: [f64; 2], to: [f64; 2], pen: Pen, dashed: bool) {
        self.grow(from);
        self.grow(to);
        self.items.push(Item::Line {
            from,
            to,
            pen,
            dashed,
        });
    }

    /// A margin around everything drawn.
    fn pad(mut self, m: f64) -> Self {
        if self.lo[0].is_finite() {
            self.lo = [self.lo[0] - m, self.lo[1] - m];
            self.hi = [self.hi[0] + m, self.hi[1] + m];
        } else {
            self.lo = [0.0, 0.0];
            self.hi = [1.0, 1.0];
        }
        self
    }

    pub fn width(&self) -> f64 {
        self.hi[0] - self.lo[0]
    }

    pub fn height(&self) -> f64 {
        self.hi[1] - self.lo[1]
    }
}

/// How much of the host wall is shown around a door or window.
const SURROUND: f64 = 0.25;

/// Colour of a material seen (not cut): its cut colour, lighter; glass
/// light blue and see-through.
fn seen(model: &Model, material: u64) -> [u8; 4] {
    match model.material(archisim_core::MaterialId(material)) {
        Some(m) if m.category == "glass" => [190, 220, 240, 150],
        Some(m) => {
            let [r, g, b] = m.cut_color;
            let l = |c: u8| (c as f64 + (255.0 - c as f64) * 0.35) as u8;
            [l(r), l(g), l(b), 255]
        }
        None => [220, 220, 220, 255],
    }
}

fn cut(model: &Model, material: u64) -> ([u8; 4], Option<CutPattern>) {
    match model.material(archisim_core::MaterialId(material)) {
        Some(m) if m.category == "glass" => ([190, 220, 240, 220], None),
        Some(m) => {
            let [r, g, b] = m.cut_color;
            ([r, g, b, 255], Some(m.cut_pattern))
        }
        None => ([200, 200, 200, 255], None),
    }
}

/// The body of a door or window (its parts, in the family's axes) and the
/// mini model with its materials and host wall.
struct Body {
    model: Model,
    parts: Vec<archisim_core::family::Part>,
    /// Host wall layers, from the exterior.
    layers: Vec<Layer>,
    width: f64,
    height: f64,
    shape: archisim_core::OpeningShape,
    door: bool,
    leaves: usize,
    /// The recipe's elevation marks (opening lines), in `x` and `z`.
    marks: Vec<archisim_core::family::Mark>,
}

fn body(source: &Model, t: &OpeningType) -> Option<Body> {
    let (model, _) = opening_model(source, t)?;
    let o = model.openings().next()?;
    let parts = model.evaluate_opening(o.id).ok()?.parts;
    let marks = model.opening_elevation_marks(o.id).unwrap_or_default();
    let wall = model.wall(o.wall)?;
    let layers = model.wall_type(wall.wall_type)?.layers.clone();
    let leaves = match t.kind {
        archisim_core::OpeningKind::Door(archisim_core::DoorOperation::DoubleSwing) => 2,
        _ => t.panels.len().max(1),
    };
    Some(Body {
        model,
        parts,
        layers,
        width: t.width,
        height: t.height,
        shape: t.shape,
        door: t.is_door(),
        leaves,
        marks,
    })
}

const WALL_SEEN: [u8; 4] = [236, 236, 236, 255];

/// A door or window seen from the side its leaves open to, in its wall:
/// family `x` to the right, height up from the sill.
pub fn opening_elevation(source: &Model, t: &OpeningType) -> Option<Drawing> {
    let b = body(source, t)?;
    let mut d = Drawing::new();
    let (w, h) = (b.width / 2.0, b.height);
    let x0 = -w - SURROUND;
    let x1 = w + SURROUND;
    let bottom = if b.door { 0.0 } else { -SURROUND };
    // The wall around the opening.
    d.rect([x0, h], [x1, h + SURROUND], WALL_SEEN, None, Pen::Thin);
    d.rect([x0, bottom], [-w, h], WALL_SEEN, None, Pen::Thin);
    d.rect([w, bottom], [x1, h], WALL_SEEN, None, Pen::Thin);
    if !b.door {
        d.rect([-w, bottom], [w, 0.0], WALL_SEEN, None, Pen::Thin);
    }
    // Over an arch (or round an oculus) the wall fills the box back in.
    let shape = b.shape;
    if !shape.is_rect() {
        for (line, edge) in [(shape.upper(b.width, h), h), (shape.lower(b.width, h), 0.0)] {
            for q in line.windows(2) {
                d.polygon(
                    vec![q[0], q[1], [q[1][0], edge], [q[0][0], edge]],
                    WALL_SEEN,
                    None,
                );
            }
        }
    }
    // Farthest first: the viewer stands on +y.
    let mut parts = b.parts.clone();
    parts.sort_by(|a, b| a.max[1].total_cmp(&b.max[1]));
    // The pieces of an arch are drawn whole: filled, and outlined only
    // where a piece does not meet another of the same recipe line.
    let mut edges: Vec<([f64; 2], [f64; 2], usize)> = Vec::new();
    for p in parts.iter().filter(|p| p.profile.is_some()) {
        let pr = p.profile.as_ref().expect("filtered");
        for i in 0..pr.len() {
            edges.push((pr[i], pr[(i + 1) % pr.len()], p.line));
        }
    }
    let near = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6;
    for p in &parts {
        match &p.profile {
            Some(profile) => {
                d.polygon(profile.clone(), seen(&b.model, p.material), None);
                for i in 0..profile.len() {
                    let (a, c) = (profile[i], profile[(i + 1) % profile.len()]);
                    let shared = edges
                        .iter()
                        .any(|(x, y, l)| *l == p.line && near(*x, c) && near(*y, a));
                    if !shared {
                        d.line(a, c, Pen::Medium, false);
                    }
                }
            }
            None => d.rect(
                [p.min[0], p.min[2]],
                [p.max[0], p.max[2]],
                seen(&b.model, p.material),
                None,
                Pen::Medium,
            ),
        }
    }
    // The rough opening and, for doors, the floor.
    if shape.is_rect() {
        d.line([-w, 0.0], [-w, h], Pen::Medium, false);
        d.line([w, 0.0], [w, h], Pen::Medium, false);
        d.line([-w, h], [w, h], Pen::Medium, false);
    } else {
        let outline = shape.polygon(b.width, h);
        for i in 0..outline.len() {
            let (a, c) = (outline[i], outline[(i + 1) % outline.len()]);
            // Not along the sill (drawn below).
            if a[1].abs() > 1e-9 || c[1].abs() > 1e-9 {
                d.line(a, c, Pen::Medium, false);
            }
        }
    }
    if b.door {
        d.line([x0, 0.0], [x1, 0.0], Pen::Thick, false);
    } else {
        d.line([-w, 0.0], [w, 0.0], Pen::Medium, false);
    }
    // The opening lines the recipe draws in elevation.
    for m in &b.marks {
        elevation_mark(&mut d, m);
    }
    Some(d.pad(0.03))
}

/// One elevation mark of a recipe as lines of the drawing (arcs in short
/// segments; any dash pattern drawn dashed).
fn elevation_mark(d: &mut Drawing, m: &archisim_core::family::Mark) {
    use archisim_core::family::Mark;
    use archisim_core::LineStyle;
    match m {
        Mark::Line {
            from, to, style, ..
        } => d.line(*from, *to, Pen::Thin, *style != LineStyle::Continuous),
        Mark::Polygon { points, style, .. } => {
            for i in 0..points.len() {
                let a = points[i];
                let b = points[(i + 1) % points.len()];
                d.line(a, b, Pen::Thin, *style != LineStyle::Continuous);
            }
        }
        Mark::Arc {
            center,
            from,
            to,
            style,
            ..
        } => {
            let r = ((from[0] - center[0]).powi(2) + (from[1] - center[1]).powi(2)).sqrt();
            let a0 = (from[1] - center[1]).atan2(from[0] - center[0]);
            let mut a1 = (to[1] - center[1]).atan2(to[0] - center[0]);
            // The short way.
            let tau = std::f64::consts::TAU;
            while a1 - a0 > std::f64::consts::PI {
                a1 -= tau;
            }
            while a0 - a1 > std::f64::consts::PI {
                a1 += tau;
            }
            let n = 16;
            let at = |k: usize| {
                let a = a0 + (a1 - a0) * k as f64 / n as f64;
                [center[0] + r * a.cos(), center[1] + r * a.sin()]
            };
            for k in 0..n {
                d.line(at(k), at(k + 1), Pen::Thin, *style != LineStyle::Continuous);
            }
        }
    }
}

/// A door or window cut vertically through the middle of its first leaf
/// (from the left, as seen in elevation): the exterior on the left, the
/// side its leaves open to on the right; the host wall cut above the head
/// and below the sill, its reveal seen beyond.
pub fn opening_section(source: &Model, t: &OpeningType) -> Option<Drawing> {
    let b = body(source, t)?;
    let mut d = Drawing::new();
    d.sides = ("Exterior", "Interior");
    let leaf = b.width / b.leaves as f64;
    let x_cut = if b.leaves > 1 {
        -b.width / 2.0 + leaf / 2.0
    } else {
        0.0
    };
    let thickness: f64 = b.layers.iter().map(|l| l.thickness).sum();
    let h = b.height;
    // Family y = 0 is the middle of the frame, centred in the wall; +y the
    // interior (the preview opens to the interior).
    let wall_band = |d: &mut Drawing, z0: f64, z1: f64| {
        let mut y = -thickness / 2.0;
        for l in &b.layers {
            let (fill, pattern) = cut(&b.model, l.material.0);
            d.rect([y, z0], [y + l.thickness, z1], fill, pattern, Pen::Thin);
            y += l.thickness;
        }
        d.rect(
            [-thickness / 2.0, z0],
            [thickness / 2.0, z1],
            [0, 0, 0, 0],
            None,
            Pen::Thick,
        );
    };
    // Above the shape where the cut runs (under an arch, its intrados).
    let top_at_cut = if b.shape.is_rect() {
        h
    } else {
        z_range(&b.shape.polygon(b.width, h), x_cut).map_or(h, |r| r.1)
    };
    wall_band(&mut d, top_at_cut, h + SURROUND);
    if b.door {
        let reach = thickness / 2.0 + 0.25;
        d.line([-reach, 0.0], [reach, 0.0], Pen::Thick, false);
    } else {
        wall_band(&mut d, -SURROUND, 0.0);
    }
    // The reveals beyond the cut.
    d.line(
        [-thickness / 2.0, 0.0],
        [-thickness / 2.0, h],
        Pen::Thin,
        false,
    );
    d.line(
        [thickness / 2.0, 0.0],
        [thickness / 2.0, h],
        Pen::Thin,
        false,
    );
    // Parts beyond the cut (towards the right jamb) seen, then those cut
    // (a piece of an arch, where the cut crosses it).
    for p in b.parts.iter().filter(|p| p.min[0] > x_cut) {
        d.rect(
            [p.min[1], p.min[2]],
            [p.max[1], p.max[2]],
            [0, 0, 0, 0],
            None,
            Pen::Thin,
        );
    }
    for p in b
        .parts
        .iter()
        .filter(|p| p.min[0] <= x_cut && p.max[0] >= x_cut)
    {
        let (z0, z1) = match &p.profile {
            Some(profile) => match z_range(profile, x_cut) {
                Some(r) => r,
                None => continue,
            },
            None => (p.min[2], p.max[2]),
        };
        let (fill, pattern) = cut(&b.model, p.material);
        d.rect(
            [p.min[1], z0],
            [p.max[1], z1],
            fill,
            pattern.filter(|_| p.max[1] - p.min[1] > 0.03),
            Pen::Thick,
        );
    }
    Some(d.pad(0.03))
}

/// The heights a convex outline (in `x` and `z`) covers at `x`.
fn z_range(poly: &[[f64; 2]], x: f64) -> Option<(f64, f64)> {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let (xa, xb) = (a[0].min(b[0]), a[0].max(b[0]));
        if x < xa - 1e-12 || x > xb + 1e-12 {
            continue;
        }
        let z = if (b[0] - a[0]).abs() < 1e-12 {
            lo = lo.min(a[1].min(b[1]));
            hi = hi.max(a[1].max(b[1]));
            continue;
        } else {
            a[1] + (b[1] - a[1]) * (x - a[0]) / (b[0] - a[0])
        };
        lo = lo.min(z);
        hi = hi.max(z);
    }
    (hi > lo).then_some((lo, hi))
}

/// The layers of a wall type cut vertically (exterior on the left), or of
/// a slab type (top face up), a short piece of each.
pub fn layers_section(model: &Model, layers: &[Layer], slab: bool) -> Drawing {
    let mut d = Drawing::new();
    let total: f64 = layers.iter().map(|l| l.thickness).sum();
    let piece = (total * 2.0).clamp(0.5, 0.9);
    let mut at = 0.0;
    for l in layers {
        let (fill, pattern) = cut(model, l.material.0);
        if slab {
            d.rect(
                [0.0, total - at - l.thickness],
                [piece, total - at],
                fill,
                pattern,
                Pen::Thin,
            );
        } else {
            d.rect(
                [at, 0.0],
                [at + l.thickness, piece],
                fill,
                pattern,
                Pen::Thin,
            );
        }
        at += l.thickness;
    }
    let max = if slab { [piece, total] } else { [total, piece] };
    d.rect([0.0, 0.0], max, [0, 0, 0, 0], None, Pen::Thick);
    d.sides = if slab {
        ("", "")
    } else {
        ("Exterior", "Interior")
    };
    d.pad(0.02)
}

/// A square of a material cut: its colour and hatch.
pub fn material_swatch(model: &Model, material: archisim_core::MaterialId) -> Drawing {
    let mut d = Drawing::new();
    let (fill, pattern) = cut(model, material.0);
    d.rect([0.0, 0.0], [0.3, 0.3], fill, pattern, Pen::Medium);
    d.pad(0.01)
}

// ---- Curtain walls --------------------------------------------------------

/// The curtain wall of a preview model (`type_preview::curtain_model`),
/// its design and pieces.
fn curtain_of(
    model: &Model,
) -> Option<(
    &archisim_core::CurtainWall,
    &archisim_core::CurtainDesign,
    Vec<archisim_core::CurtainPiece>,
)> {
    let c = model.curtain_walls().next()?;
    let d = model.curtain_design(c.id)?;
    Some((c, d, c.pieces(d)))
}

/// A curtain wall seen from outside: `x` along it from its end (its start
/// is on the right, as seen from outside), `y` up from its base. Farther
/// things first, so the caps and louvres cover what is behind them.
/// Returns the drawing and, to label it, where the grid lines fall on the
/// paper (`x` of each mullion line, `y` of each transom line).
pub fn curtain_elevation(model: &Model) -> Option<(Drawing, Vec<f64>, Vec<f64>)> {
    let (c, d, mut pieces) = curtain_of(model)?;
    let l = c.length();
    pieces.sort_by(|a, b| a.across.0.total_cmp(&b.across.0));
    let mut out = Drawing::new();
    for p in &pieces {
        let pen = match p.kind {
            archisim_core::CurtainPieceKind::Panel(_) => Pen::Thin,
            _ => Pen::Medium,
        };
        out.rect(
            [l - p.along.1, p.up.0],
            [l - p.along.0, p.up.1],
            seen(model, p.material.0),
            None,
            pen,
        );
    }
    // The ground line.
    out.line([-0.2, 0.0], [l + 0.2, 0.0], Pen::Thick, false);
    let g = c.grid(d);
    let xs = g.mullions.iter().map(|x| l - x).collect();
    Some((out.pad(0.05), xs, g.transoms))
}

/// A curtain wall cut in plan at mid-height of the row its cut plane
/// (1,10 m, or half the height if lower) goes through: the exterior on
/// top, from its start; only its corner and first mullion (a detail), with what is seen
/// below (louvres) thin.
pub fn curtain_plan_detail(model: &Model) -> Option<Drawing> {
    let (c, d, pieces) = curtain_of(model)?;
    let g = c.grid(d);
    let cut = 1.1_f64.min(c.height / 2.0);
    let end = g.mullions.get(1).copied().unwrap_or(c.length()) + 0.35;
    let end = end.min(c.length());
    let mut out = Drawing::new();
    for p in &pieces {
        let (a0, a1) = (p.along.0.max(0.0), p.along.1.min(end));
        if a1 - a0 < 1e-6 {
            continue;
        }
        let (min, max) = ([a0, p.across.0], [a1, p.across.1]);
        match p.kind {
            archisim_core::CurtainPieceKind::Louvre => {
                if p.up.1 <= cut {
                    out.rect(min, max, [0, 0, 0, 0], None, Pen::Thin);
                }
            }
            _ if p.up.0 <= cut && p.up.1 >= cut => {
                let (fill, pattern) = cut_fill(model, p.material.0);
                out.rect(min, max, fill, pattern, Pen::Thick);
            }
            _ => {}
        }
    }
    if end < c.length() - 1e-6 {
        // Broken off: the wall goes on.
        let (lo, hi) = (out.lo[1], out.hi[1]);
        out.line([end, lo - 0.03], [end, hi + 0.03], Pen::Thin, true);
    }
    out.sides = ("Exterior arriba", "");
    Some(out.pad(0.03))
}

/// A curtain wall cut vertically through the middle of its first bay:
/// the exterior on the left, what the cut goes through thick.
pub fn curtain_section(model: &Model) -> Option<Drawing> {
    let (c, d, pieces) = curtain_of(model)?;
    let g = c.grid(d);
    let at = (g.mullions[0] + g.mullions[1]) / 2.0;
    let mut out = Drawing::new();
    for p in &pieces {
        if p.along.0 > at || p.along.1 < at {
            continue;
        }
        let (fill, pattern) = cut_fill(model, p.material.0);
        out.rect(
            [-p.across.1, p.up.0],
            [-p.across.0, p.up.1],
            fill,
            pattern.filter(|_| p.across.1 - p.across.0 > 0.03),
            Pen::Thick,
        );
    }
    // The mullion behind the cut, seen.
    if let Some(m) = pieces
        .iter()
        .find(|p| p.kind.is_frame() && p.is_vertical() && p.along.1 <= at && p.across.0 < 0.0)
    {
        out.rect(
            [-m.across.1, m.up.0],
            [-m.across.0, m.up.1],
            [0, 0, 0, 0],
            None,
            Pen::Thin,
        );
    }
    out.line([-0.4, 0.0], [0.4, 0.0], Pen::Thick, false);
    out.sides = ("Exterior", "Interior");
    Some(out.pad(0.05))
}

fn cut_fill(model: &Model, material: u64) -> ([u8; 4], Option<CutPattern>) {
    cut(model, material)
}

impl Drawing {
    /// The drawing as an SVG on white paper, `px_per_m` pixels per metre,
    /// with the pens of the screen (to rasterise it into a picture).
    pub fn svg(&self, px_per_m: f64) -> String {
        use std::fmt::Write as _;
        let (w, h) = (self.width() * px_per_m, self.height() * px_per_m);
        let x = |v: f64| (v - self.lo[0]) * px_per_m;
        let y = |v: f64| (self.hi[1] - v) * px_per_m;
        let pen = |p: Pen| match p {
            Pen::Thin => 0.6,
            Pen::Medium => 1.0,
            Pen::Thick => 1.6,
        };
        let mut s = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w:.1}" height="{h:.1}" viewBox="0 0 {w:.2} {h:.2}"><rect width="100%" height="100%" fill="white"/>"#
        );
        for item in &self.items {
            match item {
                Item::Rect {
                    min,
                    max,
                    fill,
                    pen: p,
                    ..
                } => {
                    let [r, g, b, a] = *fill;
                    let _ = write!(
                        s,
                        r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="rgb({r},{g},{b})" fill-opacity="{:.3}" stroke="rgb(30,30,30)" stroke-width="{}"/>"#,
                        x(min[0]),
                        y(max[1]),
                        (max[0] - min[0]) * px_per_m,
                        (max[1] - min[1]) * px_per_m,
                        a as f64 / 255.0,
                        pen(*p)
                    );
                }
                Item::Polygon {
                    points,
                    fill,
                    pen: p,
                } => {
                    let [r, g, b, a] = *fill;
                    let pts: Vec<String> = points
                        .iter()
                        .map(|q| format!("{:.2},{:.2}", x(q[0]), y(q[1])))
                        .collect();
                    let _ = write!(
                        s,
                        r#"<polygon points="{}" fill="rgb({r},{g},{b})" fill-opacity="{:.3}" stroke="{}" stroke-width="{}"/>"#,
                        pts.join(" "),
                        a as f64 / 255.0,
                        if p.is_some() { "rgb(30,30,30)" } else { "none" },
                        p.map_or(0.0, pen)
                    );
                }
                Item::Line {
                    from,
                    to,
                    pen: p,
                    dashed,
                } => {
                    let dash = if *dashed {
                        r#" stroke-dasharray="4 3""#
                    } else {
                        ""
                    };
                    let _ = write!(
                        s,
                        r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="rgb(30,30,30)" stroke-width="{}"{dash}/>"#,
                        x(from[0]),
                        y(from[1]),
                        x(to[0]),
                        y(to[1]),
                        pen(*p)
                    );
                }
            }
        }
        s.push_str("</svg>");
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_library_curtain_wall_has_its_drawings() {
        let m = archisim_library::new_project("p");
        for t in m.curtain_types() {
            let model = crate::type_preview::curtain_model(&m, &t.design, 4.8, 3.0)
                .unwrap_or_else(|| panic!("{}", t.name));
            let (e, xs, ys) = curtain_elevation(&model).unwrap();
            assert!(e.width() > 4.8 && e.height() > 3.0, "{}", t.name);
            assert_eq!(
                xs.len(),
                model
                    .curtain_walls()
                    .next()
                    .unwrap()
                    .grid(&t.design)
                    .mullions
                    .len()
            );
            assert!(ys.len() >= 2);
            let p = curtain_plan_detail(&model).unwrap();
            assert!(p.items.iter().any(|i| matches!(
                i,
                Item::Rect {
                    pen: Pen::Thick,
                    ..
                }
            )));
            let s = curtain_section(&model).unwrap();
            assert!(s.height() > 3.0, "{}", t.name);
            assert!(e.svg(40.0).starts_with("<svg"));
        }
    }

    #[test]
    fn swing_doors_and_casements_show_their_opening_lines() {
        let m = archisim_library::new_project("p");
        let lines = |t: &OpeningType| {
            opening_elevation(&m, t)
                .unwrap()
                .items
                .iter()
                .filter(|i| matches!(i, Item::Line { pen: Pen::Thin, .. }))
                .count()
        };
        for t in m.opening_types() {
            let family = m.family(t.family).unwrap().name().to_string();
            let n = lines(t);
            match family.as_str() {
                // Two lines per leaf that turns.
                "Puerta abatible" => assert!(n >= 2, "{}: {n}", t.name),
                "Ventana practicable" => {
                    let turning = t.panels.iter().filter(|p| *p != "FIXEDCASEMENT").count();
                    assert!(n >= 2 * turning, "{}: {n}", t.name);
                }
                "Hueco" => assert_eq!(n, 0, "{}", t.name),
                _ => {}
            }
        }
    }

    #[test]
    fn every_library_door_and_window_has_an_elevation_and_a_section() {
        let m = archisim_library::new_project("p");
        for t in m.opening_types() {
            let e = opening_elevation(&m, t).unwrap_or_else(|| panic!("{}", t.name));
            // The wall around the opening frames it.
            assert!(e.width() > t.width + 2.0 * SURROUND, "{}", t.name);
            assert!(e.height() > t.height, "{}", t.name);
            let s = opening_section(&m, t).unwrap_or_else(|| panic!("{}", t.name));
            assert!(s.height() > t.height, "{}", t.name);
            // Something of the door or window is cut, unless it is a void.
            let thick = s
                .items
                .iter()
                .filter(|i| {
                    matches!(
                        i,
                        Item::Rect {
                            pen: Pen::Thick,
                            ..
                        }
                    )
                })
                .count();
            let walls = if t.is_door() { 1 } else { 2 };
            if t.is_void() {
                assert_eq!(thick, walls, "{}", t.name);
            } else {
                assert!(thick > walls, "{}", t.name);
            }
        }
    }

    #[test]
    fn layers_are_drawn_in_order() {
        let m = archisim_library::new_project("p");
        let wt = m.wall_types().next().unwrap();
        let d = layers_section(&m, &wt.layers, false);
        let firsts: Vec<f64> = d
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Rect {
                    min,
                    pen: Pen::Thin,
                    ..
                } => Some(min[0]),
                _ => None,
            })
            .collect();
        assert_eq!(firsts.len(), wt.layers.len());
        assert!(firsts.windows(2).all(|w| w[0] < w[1]));
        assert!((d.width() - wt.thickness() - 0.04).abs() < 1e-9);
        let st = m.slab_types().next().unwrap();
        let d = layers_section(&m, &st.layers, true);
        assert!((d.height() - st.thickness() - 0.04).abs() < 1e-9);
    }
}
