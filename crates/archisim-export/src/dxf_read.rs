//! DXF drawings in: lines, polylines, circles, arcs, ellipses, splines
//! (as polylines), points, texts, hatches and blocks (exploded, also those
//! of dimensions) become 2D drafting on layers of the same names, with
//! their colours, weights and line types.

use std::collections::BTreeMap;
use std::f64::consts::{PI, TAU};

use archisim_core::{
    DrawLayer, Hatch, HatchPattern, LineStyle, Model, ModelError, NewDraft, Point2, Polyline, Rgb,
    Shape, Text, TextAlign, Vec2, ViewRef, LINE_WEIGHTS,
};
use archisim_draft::{transform_shape, Seg, Transform};

/// A DXF drawing read: its layers and its elements, in metres.
#[derive(Debug, Clone, Default)]
pub struct Drawing {
    pub layers: Vec<LayerDef>,
    pub items: Vec<ItemDef>,
    /// Entities of kinds not brought in, by kind.
    pub skipped: BTreeMap<String, usize>,
    /// The drawing's unit, in metres (from `$INSUNITS`; millimetres if it
    /// does not say).
    pub unit: f64,
    /// Whether the file says its unit (`$INSUNITS` other than 0).
    pub unit_stated: bool,
}

impl Drawing {
    /// Its extent, in metres: lower left and upper right corners.
    pub fn extent(&self) -> Option<(Point2, Point2)> {
        let mut out: Option<(Point2, Point2)> = None;
        for it in &self.items {
            let (a, b) = archisim_draft::bounds(&it.shape, 0.0);
            out = Some(match out {
                None => (a, b),
                Some((lo, hi)) => (
                    Point2::new(lo.x.min(a.x), lo.y.min(a.y)),
                    Point2::new(hi.x.max(b.x), hi.y.max(b.y)),
                ),
            });
        }
        out
    }

    /// The drawing in another unit (metres per drawing unit): every shape
    /// scaled about the origin.
    pub fn in_unit(&self, unit: f64) -> Drawing {
        let f = unit / self.unit;
        let t = Transform::scale(Point2::new(0.0, 0.0), f);
        let mut d = self.clone();
        d.unit = unit;
        for it in &mut d.items {
            it.shape = transform_shape(&it.shape, &t);
        }
        d
    }

    /// The unit the drawing is most likely drawn in, metres per unit: the
    /// one it says, unless it is millimetres (or nothing) and the whole
    /// drawing would then be a few centimetres across — then it is drawn
    /// in metres, as many are.
    pub fn likely_unit(&self) -> f64 {
        let Some((lo, hi)) = self.extent() else {
            return self.unit;
        };
        let raw = (hi.x - lo.x).max(hi.y - lo.y) / self.unit;
        let mm = (self.unit - 0.001).abs() < 1e-12;
        if mm && (raw < 100.0 || (!self.unit_stated && raw < 500.0)) {
            1.0
        } else {
            self.unit
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayerDef {
    pub name: String,
    pub color: Rgb,
    pub weight: f64,
    pub line: LineStyle,
    pub visible: bool,
}

/// One element: its layer, shape (texts with their height in drawing
/// metres in `Text::height`, turned into paper millimetres when added),
/// and what it changes from its layer.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemDef {
    pub layer: String,
    pub shape: Shape,
    pub color: Option<Rgb>,
    pub weight: Option<f64>,
    pub line: Option<LineStyle>,
    /// The block placed it came from (its number among the drawing's
    /// placed blocks, and the block's name), to keep it as a group.
    pub block: Option<(usize, String)>,
}

/// What a DXF brought in.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    pub elements: usize,
    /// Blocks kept as groups.
    pub groups: usize,
    pub layers_new: usize,
    pub skipped: BTreeMap<String, usize>,
}

impl Report {
    /// In words, for the person who imported it.
    pub fn describe(&self) -> String {
        let mut s = format!(
            "{} elementos de dibujo importados, {} capas nuevas.",
            self.elements, self.layers_new
        );
        if self.groups > 0 {
            s.push_str(&format!(" {} bloques, cada uno agrupado.", self.groups));
        }
        if !self.skipped.is_empty() {
            let parts: Vec<String> = self
                .skipped
                .iter()
                .map(|(k, n)| format!("{n} {k}"))
                .collect();
            s.push_str(&format!(" No importados: {}.", parts.join(", ")));
        }
        s
    }
}

type Codes = Vec<(i32, String)>;

/// One entity as read: its kind and group codes.
#[derive(Debug, Clone)]
struct Ent {
    kind: String,
    codes: Codes,
    /// Vertices of an old-style POLYLINE.
    vertices: Vec<Codes>,
}

impl Ent {
    fn get(&self, code: i32) -> Option<&str> {
        self.codes
            .iter()
            .find(|(c, _)| *c == code)
            .map(|(_, v)| v.as_str())
    }

    fn f(&self, code: i32) -> f64 {
        self.get(code)
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0.0)
    }

    fn f_or(&self, code: i32, d: f64) -> f64 {
        self.get(code)
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(d)
    }

    fn i(&self, code: i32) -> Option<i64> {
        self.get(code).and_then(|v| v.trim().parse().ok())
    }

    fn layer(&self) -> String {
        decode(self.get(8).unwrap_or("0"))
    }
}

/// One block placed in a drawing, on its own: a drawing of many trees,
/// people or cars as a library, one block each.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    /// The block's name.
    pub name: String,
    /// Where it is placed, in metres.
    pub at: Point2,
    /// What it draws, placed (in metres, as in the whole drawing).
    pub items: Vec<ItemDef>,
}

/// Reads a DXF file (ASCII; UTF-8, or Windows-1252 as older files are).
pub fn read(bytes: &[u8]) -> Result<Drawing, String> {
    read_with_blocks(bytes).map(|(d, _)| d)
}

/// [`read`], and each block placed in the model space on its own, in
/// the order of the file; a block placed again is given once.
pub fn read_with_blocks(bytes: &[u8]) -> Result<(Drawing, Vec<Placed>), String> {
    let text = match std::str::from_utf8(bytes) {
        Ok(t) => t.to_string(),
        Err(_) => bytes.iter().map(|b| cp1252(*b)).collect(),
    };
    if text.starts_with("AutoCAD Binary DXF") {
        return Err("es un DXF binario: guárdalo como DXF de texto (ASCII)".into());
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut pairs: Vec<(i32, String)> = Vec::with_capacity(lines.len() / 2);
    let mut i = 0;
    while i + 1 < lines.len() {
        let code: i32 = lines[i]
            .trim()
            .parse()
            .map_err(|_| format!("línea {}: se esperaba un código de grupo", i + 1))?;
        pairs.push((code, lines[i + 1].trim_end_matches('\r').to_string()));
        i += 2;
    }
    let mut d = Drawing {
        unit: 0.001,
        ..Default::default()
    };
    let mut blocks: BTreeMap<String, (Point2, Vec<Ent>)> = BTreeMap::new();
    let mut entities: Vec<Ent> = Vec::new();
    let mut i = 0;
    while i < pairs.len() {
        if pairs[i] == (0, "SECTION".into()) && i + 1 < pairs.len() {
            let name = pairs[i + 1].1.trim().to_string();
            i += 2;
            let end = (i..pairs.len())
                .find(|&j| pairs[j] == (0, "ENDSEC".into()))
                .unwrap_or(pairs.len());
            let sec = &pairs[i..end];
            match name.as_str() {
                "HEADER" => header(sec, &mut d),
                "TABLES" => layers(sec, &mut d),
                "BLOCKS" => read_blocks(sec, &mut blocks),
                "ENTITIES" => entities = split_entities(sec),
                _ => {}
            }
            i = end + 1;
        } else {
            i += 1;
        }
    }
    let unit = d.unit;
    let mut out = Vec::new();
    let mut skipped = BTreeMap::new();
    let mut inserts = 0;
    for e in &entities {
        let first = out.len();
        convert(
            e,
            &blocks,
            &Affine::scale(unit),
            None,
            0,
            &mut out,
            &mut skipped,
        );
        // A block placed, of more than one element: kept together.
        if e.kind == "INSERT" && out.len() > first + 1 {
            let name = decode(e.get(2).unwrap_or(""));
            for it in &mut out[first..] {
                it.block = Some((inserts, name.clone()));
            }
            inserts += 1;
        }
    }
    d.items = out;
    d.skipped = skipped;
    let mut placed: Vec<Placed> = Vec::new();
    for e in entities.iter().filter(|e| e.kind == "INSERT") {
        let name = decode(e.get(2).unwrap_or(""));
        if placed.iter().any(|p| p.name == name) || !blocks.contains_key(&name) {
            continue;
        }
        let mut items = Vec::new();
        let mut ignored = BTreeMap::new();
        convert(
            e,
            &blocks,
            &Affine::scale(unit),
            None,
            0,
            &mut items,
            &mut ignored,
        );
        if !items.is_empty() {
            placed.push(Placed {
                name,
                at: Point2::new(e.f(10) * unit, e.f(20) * unit),
                items,
            });
        }
    }
    Ok((d, placed))
}

fn header(sec: &[(i32, String)], d: &mut Drawing) {
    for w in sec.windows(2) {
        if w[0] == (9, "$INSUNITS".into()) {
            let code = w[1].1.trim().parse::<i32>().unwrap_or(0);
            d.unit_stated = code != 0;
            d.unit = match code {
                1 => 0.0254,
                2 => 0.3048,
                5 => 0.01,
                6 => 1.0,
                7 => 1000.0,
                14 => 0.1,
                _ => 0.001,
            };
        }
    }
}

fn layers(sec: &[(i32, String)], d: &mut Drawing) {
    let mut i = 0;
    while i < sec.len() {
        if sec[i] == (0, "LAYER".into()) {
            let end = (i + 1..sec.len())
                .find(|&j| sec[j].0 == 0)
                .unwrap_or(sec.len());
            let e = Ent {
                kind: "LAYER".into(),
                codes: sec[i + 1..end].to_vec(),
                vertices: Vec::new(),
            };
            let aci_code = e.i(62).unwrap_or(7);
            let color = match e.i(420) {
                Some(tc) => rgb_of(tc),
                None => aci_rgb(aci_code.unsigned_abs() as u8),
            };
            let weight = e
                .i(370)
                .filter(|w| *w >= 0)
                .map_or(0.25, |w| w as f64 / 100.0);
            d.layers.push(LayerDef {
                name: decode(e.get(2).unwrap_or("0")),
                color,
                weight: snap_weight(weight),
                line: line_style(e.get(6).unwrap_or("Continuous")),
                visible: aci_code >= 0,
            });
            i = end;
        } else {
            i += 1;
        }
    }
}

fn split_entities(sec: &[(i32, String)]) -> Vec<Ent> {
    let mut out: Vec<Ent> = Vec::new();
    let mut i = 0;
    while i < sec.len() {
        if sec[i].0 != 0 {
            i += 1;
            continue;
        }
        let kind = sec[i].1.trim().to_string();
        let end = (i + 1..sec.len())
            .find(|&j| sec[j].0 == 0)
            .unwrap_or(sec.len());
        let codes = sec[i + 1..end].to_vec();
        if kind == "VERTEX" {
            if let Some(p) = out.last_mut().filter(|p| p.kind == "POLYLINE") {
                p.vertices.push(codes);
            }
        } else if kind != "SEQEND" {
            out.push(Ent {
                kind,
                codes,
                vertices: Vec::new(),
            });
        }
        i = end;
    }
    out
}

fn read_blocks(sec: &[(i32, String)], blocks: &mut BTreeMap<String, (Point2, Vec<Ent>)>) {
    let ents = split_entities(sec);
    let mut cur: Option<(String, Point2, Vec<Ent>)> = None;
    for e in ents {
        match e.kind.as_str() {
            "BLOCK" => {
                let name = decode(e.get(2).unwrap_or(""));
                cur = Some((name, Point2::new(e.f(10), e.f(20)), Vec::new()));
            }
            "ENDBLK" => {
                if let Some((n, b, v)) = cur.take() {
                    blocks.insert(n, (b, v));
                }
            }
            _ => {
                if let Some(c) = cur.as_mut() {
                    c.2.push(e);
                }
            }
        }
    }
}

/// An affine map of the plane (blocks may scale unevenly).
#[derive(Debug, Clone, Copy)]
struct Affine {
    m: [[f64; 2]; 2],
    t: Vec2,
}

impl Affine {
    fn scale(s: f64) -> Self {
        Affine {
            m: [[s, 0.0], [0.0, s]],
            t: Vec2::new(0.0, 0.0),
        }
    }

    fn apply(&self, p: Point2) -> Point2 {
        Point2::new(
            self.m[0][0] * p.x + self.m[0][1] * p.y + self.t.x,
            self.m[1][0] * p.x + self.m[1][1] * p.y + self.t.y,
        )
    }

    /// `self` after `inner`: p ↦ self(inner(p)).
    fn of(&self, inner: &Affine) -> Affine {
        let a = &self.m;
        let b = &inner.m;
        let m = [
            [
                a[0][0] * b[0][0] + a[0][1] * b[1][0],
                a[0][0] * b[0][1] + a[0][1] * b[1][1],
            ],
            [
                a[1][0] * b[0][0] + a[1][1] * b[1][0],
                a[1][0] * b[0][1] + a[1][1] * b[1][1],
            ],
        ];
        let t = self.apply(Point2::new(inner.t.x, inner.t.y));
        Affine {
            m,
            t: Vec2::new(t.x, t.y),
        }
    }

    fn det(&self) -> f64 {
        self.m[0][0] * self.m[1][1] - self.m[0][1] * self.m[1][0]
    }

    /// The same map as a similarity, if it is one (no uneven scaling).
    fn similarity(&self) -> Option<Transform> {
        let (a, b, c, d) = (self.m[0][0], self.m[0][1], self.m[1][0], self.m[1][1]);
        let s1 = (a * a + c * c).sqrt();
        let s2 = (b * b + d * d).sqrt();
        if (s1 - s2).abs() > 1e-9 * s1.max(1.0) || (a * b + c * d).abs() > 1e-9 * s1 * s1 {
            return None;
        }
        let o = Point2::new(0.0, 0.0);
        let mut t = Transform::identity();
        if self.det() < 0.0 {
            // Mirror in x first, then the rest is a rotation.
            t = Transform::mirror(o, Point2::new(0.0, 1.0));
            // After x ↦ -x, the matrix [a b; c d] becomes [-a b; -c d].
            let rot = (-c).atan2(-a);
            t = t
                .then(&Transform::scale(o, s1))
                .then(&Transform::rotate(o, rot));
        } else {
            let rot = c.atan2(a);
            t = t
                .then(&Transform::scale(o, s1))
                .then(&Transform::rotate(o, rot));
        }
        Some(t.then(&Transform::translate(self.t)))
    }
}

#[allow(clippy::too_many_arguments)]
fn convert(
    e: &Ent,
    blocks: &BTreeMap<String, (Point2, Vec<Ent>)>,
    t: &Affine,
    // Layer of the block reference, for elements on layer "0" inside it.
    parent_layer: Option<&str>,
    depth: usize,
    out: &mut Vec<ItemDef>,
    skipped: &mut BTreeMap<String, usize>,
) {
    let mut layer = e.layer();
    if layer == "0" {
        if let Some(p) = parent_layer {
            layer = p.to_string();
        }
    }
    let color = match (e.i(420), e.i(62)) {
        (Some(tc), _) => Some(rgb_of(tc)),
        (None, Some(c)) if (1..=255).contains(&c) => Some(aci_rgb(c as u8)),
        _ => None,
    };
    let weight = e
        .i(370)
        .filter(|w| *w >= 0)
        .map(|w| snap_weight(w as f64 / 100.0));
    let line = e
        .get(6)
        .filter(|n| !n.eq_ignore_ascii_case("ByLayer") && !n.eq_ignore_ascii_case("ByBlock"))
        .map(line_style);
    let mirrored_ocs = e.f_or(230, 1.0) < 0.0;
    let ocs = |p: Point2| {
        if mirrored_ocs {
            Point2::new(-p.x, p.y)
        } else {
            p
        }
    };
    let p = |x: i32, y: i32| Point2::new(e.f(x), e.f(y));
    let shape: Option<Shape> = match e.kind.as_str() {
        "LINE" => Some(Shape::Line {
            a: p(10, 20),
            b: p(11, 21),
        }),
        "POINT" => Some(Shape::Point { at: p(10, 20) }),
        "CIRCLE" => Some(Shape::Circle {
            centre: ocs(p(10, 20)),
            radius: e.f(40),
        }),
        "ARC" => {
            let (mut a0, mut a1) = (e.f(50).to_radians(), e.f(51).to_radians());
            if mirrored_ocs {
                (a0, a1) = (PI - a1, PI - a0);
            }
            let mut sweep = (a1 - a0).rem_euclid(TAU);
            if sweep < 1e-9 {
                sweep = TAU;
            }
            Some(Shape::Arc {
                centre: ocs(p(10, 20)),
                radius: e.f(40),
                start: a0.rem_euclid(TAU),
                sweep: sweep.min(TAU - 1e-9),
            })
        }
        "ELLIPSE" => {
            let axis = Vec2::new(e.f(11), e.f(21));
            let (a0, a1) = (e.f_or(41, 0.0), e.f_or(42, TAU));
            let mut sweep = (a1 - a0).rem_euclid(TAU);
            if sweep < 1e-9 {
                sweep = TAU;
            }
            let ratio = e.f_or(40, 1.0).clamp(1e-6, 1.0);
            Some(Shape::Ellipse {
                centre: p(10, 20),
                axis,
                ratio,
                start: a0,
                sweep,
            })
        }
        "LWPOLYLINE" => {
            let mut pts = Vec::new();
            let mut bulges = Vec::new();
            let mut x = None;
            for (c, v) in &e.codes {
                let v: f64 = v.trim().parse().unwrap_or(0.0);
                match c {
                    10 => x = Some(v),
                    20 => {
                        if let Some(x) = x.take() {
                            pts.push(ocs(Point2::new(x, v)));
                            bulges.push(0.0);
                        }
                    }
                    42 => {
                        if let Some(b) = bulges.last_mut() {
                            *b = if mirrored_ocs { -v } else { v };
                        }
                    }
                    _ => {}
                }
            }
            polyline(pts, bulges, e.i(70).unwrap_or(0) & 1 == 1, e.f(43))
        }
        "POLYLINE" => {
            let flags = e.i(70).unwrap_or(0);
            if flags & (16 | 64) != 0 {
                // Meshes and polyface meshes are 3D.
                None
            } else {
                let mut pts = Vec::new();
                let mut bulges = Vec::new();
                for v in &e.vertices {
                    let ve = Ent {
                        kind: "VERTEX".into(),
                        codes: v.clone(),
                        vertices: Vec::new(),
                    };
                    if ve.i(70).unwrap_or(0) & 16 != 0 {
                        // Spline frame control point.
                        continue;
                    }
                    pts.push(ocs(Point2::new(ve.f(10), ve.f(20))));
                    let b = ve.f(42);
                    bulges.push(if mirrored_ocs { -b } else { b });
                }
                polyline(pts, bulges, flags & 1 == 1, e.f(40))
            }
        }
        "SPLINE" => {
            // Through its fit points if it has them, else along its
            // control polygon smoothed.
            let fit = points_of(e, 11, 21);
            let ctrl = points_of(e, 10, 20);
            let pts = if fit.len() >= 2 {
                fit
            } else {
                bspline(&ctrl, e.i(71).unwrap_or(3) as usize)
            };
            polyline(
                pts.clone(),
                vec![0.0; pts.len()],
                e.i(70).unwrap_or(0) & 1 == 1,
                0.0,
            )
        }
        "TEXT" | "ATTRIB" => {
            let h = e.f_or(40, 2.5);
            let (hz, vt) = (e.i(72).unwrap_or(0), e.i(73).unwrap_or(0));
            let at = if hz != 0 || vt != 0 {
                p(11, 21)
            } else {
                p(10, 20)
            };
            let align = match (hz, vt) {
                (1 | 4, 3) => TextAlign::TopCentre,
                (2, 3) => TextAlign::TopRight,
                (0, 3) => TextAlign::TopLeft,
                (4, _) | (1, 2) => TextAlign::Middle,
                (2, 2) => TextAlign::MiddleRight,
                (0, 2) => TextAlign::MiddleLeft,
                (1, _) => TextAlign::BottomCentre,
                (2, _) => TextAlign::BottomRight,
                _ => TextAlign::BottomLeft,
            };
            let content = plain_text(e.get(1).unwrap_or(""));
            (!content.trim().is_empty()).then(|| {
                Shape::Text(Text {
                    at: ocs(at),
                    content,
                    height: h,
                    angle: e.f(50).to_radians(),
                    align,
                })
            })
        }
        "MTEXT" => {
            let mut raw = String::new();
            for (c, v) in &e.codes {
                if *c == 3 {
                    raw.push_str(v);
                }
            }
            raw.push_str(e.get(1).unwrap_or(""));
            let content = mtext_plain(&raw);
            let attach = e.i(71).unwrap_or(1).clamp(1, 9) as usize;
            let angle = if e.get(11).is_some() {
                e.f(21).atan2(e.f(11))
            } else {
                e.f(50).to_radians()
            };
            (!content.trim().is_empty()).then(|| {
                Shape::Text(Text {
                    at: p(10, 20),
                    content,
                    height: e.f_or(40, 2.5),
                    angle,
                    align: TextAlign::ALL[attach - 1],
                })
            })
        }
        "HATCH" => hatch(e),
        "INSERT" | "DIMENSION" => {
            let name = decode(e.get(2).unwrap_or(""));
            if depth < 8 {
                if let Some((base, ents)) = blocks.get(&name) {
                    let inner = if e.kind == "INSERT" {
                        let (sx, sy) = (e.f_or(41, 1.0), e.f_or(42, 1.0));
                        let rot = e.f(50).to_radians();
                        let (s, c) = rot.sin_cos();
                        let ins = ocs(p(10, 20));
                        // p ↦ ins + R·S·(p - base)
                        let m = [[c * sx, -s * sy], [s * sx, c * sy]];
                        let tb = Point2::new(
                            m[0][0] * base.x + m[0][1] * base.y,
                            m[1][0] * base.x + m[1][1] * base.y,
                        );
                        Affine {
                            m,
                            t: Vec2::new(ins.x - tb.x, ins.y - tb.y),
                        }
                    } else {
                        Affine::scale(1.0)
                    };
                    if e.kind == "INSERT" {
                        // Rows and columns of a block array.
                        let (cols, rows) = (
                            e.i(70).unwrap_or(1).clamp(1, 100) as usize,
                            e.i(71).unwrap_or(1).clamp(1, 100) as usize,
                        );
                        let (dc, dr) = (e.f(44), e.f(45));
                        for r in 0..rows {
                            for c in 0..cols {
                                let shift = Affine {
                                    m: [[1.0, 0.0], [0.0, 1.0]],
                                    t: Vec2::new(c as f64 * dc, r as f64 * dr),
                                };
                                let at = t.of(&shift.of(&inner));
                                for be in ents {
                                    convert(be, blocks, &at, Some(&layer), depth + 1, out, skipped);
                                }
                            }
                        }
                    } else {
                        let at = t.of(&inner);
                        for be in ents {
                            convert(be, blocks, &at, Some(&layer), depth + 1, out, skipped);
                        }
                    }
                }
            }
            return;
        }
        "VIEWPORT" | "ATTDEF" | "SEQEND" | "ENDBLK" => return,
        other => {
            *skipped.entry(other.to_string()).or_insert(0) += 1;
            return;
        }
    };
    let Some(shape) = shape else {
        return;
    };
    let shape = match t.similarity() {
        Some(sim) => transform_shape(&shape, &sim),
        None => uneven(&shape, t),
    };
    out.push(ItemDef {
        layer,
        shape,
        color,
        weight,
        line,
        block: None,
    });
}

fn points_of(e: &Ent, cx: i32, cy: i32) -> Vec<Point2> {
    let mut v = Vec::new();
    let mut x = None;
    for (c, val) in &e.codes {
        let f: f64 = val.trim().parse().unwrap_or(0.0);
        if *c == cx {
            x = Some(f);
        } else if *c == cy {
            if let Some(x) = x.take() {
                v.push(Point2::new(x, f));
            }
        }
    }
    v
}

/// A uniform B-spline of a degree through its control points, as a
/// polyline (enough for drawings from other programs).
fn bspline(ctrl: &[Point2], degree: usize) -> Vec<Point2> {
    let n = ctrl.len();
    if n <= degree || degree == 0 {
        return ctrl.to_vec();
    }
    // Clamped uniform knots.
    let m = n + degree + 1;
    let knots: Vec<f64> = (0..m)
        .map(|i| {
            if i <= degree {
                0.0
            } else if i >= n {
                1.0
            } else {
                (i - degree) as f64 / (n - degree) as f64
            }
        })
        .collect();
    let steps = (n * 8).min(400);
    (0..=steps)
        .map(|s| {
            let u = (s as f64 / steps as f64).min(1.0 - 1e-12);
            de_boor(ctrl, &knots, degree, u)
        })
        .collect()
}

fn de_boor(ctrl: &[Point2], knots: &[f64], p: usize, u: f64) -> Point2 {
    let n = ctrl.len();
    let mut k = p;
    while k + 1 < n && knots[k + 1] <= u {
        k += 1;
    }
    let mut d: Vec<Point2> = (0..=p).map(|j| ctrl[j + k - p]).collect();
    for r in 1..=p {
        for j in (r..=p).rev() {
            let i = j + k - p;
            let den = knots[i + p + 1 - r] - knots[i];
            let a = if den.abs() < 1e-12 {
                0.0
            } else {
                (u - knots[i]) / den
            };
            d[j] = d[j - 1] + (d[j] - d[j - 1]) * a;
        }
    }
    d[p]
}

fn polyline(pts: Vec<Point2>, mut bulges: Vec<f64>, closed: bool, width: f64) -> Option<Shape> {
    if pts.len() < 2 {
        return None;
    }
    bulges.resize(pts.len(), 0.0);
    Some(Shape::Polyline(Polyline {
        points: pts,
        bulges,
        closed,
        width: width.max(0.0),
    }))
}

/// A hatch: its boundaries (polylines, or lines and arcs) and whether it is
/// solid.
fn hatch(e: &Ent) -> Option<Shape> {
    let solid = e.i(70) == Some(1) || e.get(2).is_some_and(|n| n.eq_ignore_ascii_case("SOLID"));
    let mut loops = Vec::new();
    let c = &e.codes;
    let mut i = c.iter().position(|(k, _)| *k == 91)? + 1;
    let num = |s: &str| s.trim().parse::<f64>().unwrap_or(0.0);
    while i < c.len() {
        if c[i].0 != 92 {
            if matches!(c[i].0, 75 | 76 | 98) {
                break;
            }
            i += 1;
            continue;
        }
        let flags = num(&c[i].1) as i64;
        i += 1;
        if flags & 2 != 0 {
            // Polyline boundary.
            let mut pts = Vec::new();
            let mut bulges = Vec::new();
            let mut x = None;
            while i < c.len() && !matches!(c[i].0, 92 | 97 | 75 | 76) {
                match c[i].0 {
                    10 => x = Some(num(&c[i].1)),
                    20 => {
                        if let Some(x) = x.take() {
                            pts.push(Point2::new(x, num(&c[i].1)));
                            bulges.push(0.0);
                        }
                    }
                    42 => {
                        if let Some(b) = bulges.last_mut() {
                            *b = num(&c[i].1);
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            if pts.len() >= 3 {
                loops.push(Polyline {
                    points: pts,
                    bulges,
                    closed: true,
                    width: 0.0,
                });
            }
        } else {
            // Edges: lines (1) and arcs (2); others as their end points.
            let mut segs: Vec<Seg> = Vec::new();
            let n_edges = c
                .get(i)
                .filter(|x| x.0 == 93)
                .map_or(0, |x| num(&x.1) as usize);
            i += 1;
            for _ in 0..n_edges {
                let Some(kind) = c.get(i).filter(|x| x.0 == 72).map(|x| num(&x.1) as i64) else {
                    break;
                };
                i += 1;
                let mut vals: BTreeMap<i32, f64> = BTreeMap::new();
                while i < c.len() && !matches!(c[i].0, 72 | 92 | 97 | 75) {
                    vals.entry(c[i].0).or_insert(num(&c[i].1));
                    i += 1;
                }
                let g = |k: i32| vals.get(&k).copied().unwrap_or(0.0);
                match kind {
                    1 => segs.push(Seg::Line {
                        a: Point2::new(g(10), g(20)),
                        b: Point2::new(g(11), g(21)),
                    }),
                    2 => {
                        let (centre, r) = (Point2::new(g(10), g(20)), g(40));
                        let (mut a0, mut a1) = (g(50).to_radians(), g(51).to_radians());
                        let ccw = g(73) != 0.0;
                        if !ccw {
                            (a0, a1) = (-a0, -a1);
                        }
                        let mut sweep = (a1 - a0).rem_euclid(TAU);
                        if sweep < 1e-9 {
                            sweep = TAU - 1e-6;
                        }
                        segs.push(Seg::Arc {
                            centre,
                            radius: r,
                            start: a0,
                            sweep: if ccw { sweep } else { -sweep },
                        });
                    }
                    _ => {}
                }
            }
            if let Some(pl) = archisim_draft::polyline_from_segs(&segs, true, 0.0) {
                loops.push(pl);
            } else if !segs.is_empty() {
                let pts: Vec<Point2> = segs.iter().map(|s| s.start()).collect();
                if pts.len() >= 3 {
                    loops.push(Polyline::straight(pts, true));
                }
            }
        }
        while i < c.len() && c[i].0 == 97 {
            let n = num(&c[i].1) as usize;
            i += 1 + n;
        }
    }
    if loops.is_empty() {
        return None;
    }
    Some(Shape::Hatch(Hatch {
        loops,
        pattern: if solid {
            HatchPattern::Solid
        } else {
            HatchPattern::Diagonal
        },
        scale: 1.0,
        angle: 0.0,
        background: None,
        opacity: 1.0,
    }))
}

/// A shape under an uneven scaling: curves as polylines.
fn uneven(shape: &Shape, t: &Affine) -> Shape {
    match shape {
        Shape::Text(tx) => {
            let mut tx = tx.clone();
            tx.at = t.apply(tx.at);
            tx.height *= t.det().abs().sqrt();
            Shape::Text(tx)
        }
        Shape::Point { at } => Shape::Point { at: t.apply(*at) },
        Shape::Hatch(h) => Shape::Hatch(Hatch {
            loops: h.loops.iter().map(|l| flat(l, t)).collect(),
            ..h.clone()
        }),
        Shape::Polyline(pl) => Shape::Polyline(flat(pl, t)),
        other => {
            let pts: Vec<Point2> = archisim_draft::shape_segs(other)
                .iter()
                .flat_map(|s| s.tessellate(1e-3))
                .map(|p| t.apply(p))
                .collect();
            Shape::Polyline(Polyline::straight(pts, false))
        }
    }
}

fn flat(pl: &Polyline, t: &Affine) -> Polyline {
    let mut pts: Vec<Point2> = Vec::new();
    for s in archisim_draft::polyline_segs(pl) {
        let mut p = s.tessellate(1e-3);
        if !pts.is_empty() {
            p.remove(0);
        }
        pts.extend(p);
    }
    if pl.closed && pts.len() > 1 && pts.first() == pts.last() {
        pts.pop();
    }
    Polyline::straight(pts.into_iter().map(|p| t.apply(p)).collect(), pl.closed)
}

/// The text of a TEXT entity: `%%c`, `%%d`, `%%p` and escapes.
fn plain_text(s: &str) -> String {
    let s = decode(s);
    s.replace("%%c", "Ø")
        .replace("%%C", "Ø")
        .replace("%%d", "°")
        .replace("%%D", "°")
        .replace("%%p", "±")
        .replace("%%P", "±")
        .replace("%%u", "")
        .replace("%%U", "")
        .replace("%%o", "")
        .replace("%%O", "")
        .replace("%%%", "%")
}

/// The plain text of an MTEXT: lines, special characters, no formatting.
fn mtext_plain(s: &str) -> String {
    let s = decode(s);
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '{' | '}' => i += 1,
            '\\' if i + 1 < chars.len() => {
                let n = chars[i + 1];
                match n {
                    'P' | 'n' => {
                        out.push('\n');
                        i += 2;
                    }
                    '~' => {
                        out.push(' ');
                        i += 2;
                    }
                    '\\' | '{' | '}' => {
                        out.push(n);
                        i += 2;
                    }
                    'L' | 'l' | 'O' | 'o' | 'K' | 'k' | 'N' => i += 2,
                    'S' => {
                        // Stacked text "a^b" or "a/b": as "a/b".
                        i += 2;
                        while i < chars.len() && chars[i] != ';' {
                            out.push(if chars[i] == '^' || chars[i] == '#' {
                                '/'
                            } else {
                                chars[i]
                            });
                            i += 1;
                        }
                        i += 1;
                    }
                    _ => {
                        // \f…; \H…; \C…; \A…; \W…; \Q…; \T…; \p…;
                        i += 2;
                        while i < chars.len() && chars[i] != ';' {
                            i += 1;
                        }
                        i += 1;
                    }
                }
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    plain_text(&out)
}

/// `\U+XXXX` escapes as characters.
fn decode(s: &str) -> String {
    if !s.contains("\\U+") && !s.contains("\\u+") {
        return s.to_string();
    }
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("\\U+").or_else(|| rest.find("\\u+")) {
        out.push_str(&rest[..i]);
        let hex = rest.get(i + 3..i + 7).unwrap_or("");
        match u32::from_str_radix(hex, 16).ok().and_then(char::from_u32) {
            Some(c) if hex.len() == 4 => {
                out.push(c);
                rest = &rest[i + 7..];
            }
            _ => {
                out.push_str(&rest[i..i + 3]);
                rest = &rest[i + 3..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn cp1252(b: u8) -> char {
    match b {
        0x80 => '€',
        0x91 => '‘',
        0x92 => '’',
        0x93 => '“',
        0x94 => '”',
        0x96 => '–',
        0x97 => '—',
        _ => b as char,
    }
}

fn rgb_of(v: i64) -> Rgb {
    [(v >> 16 & 255) as u8, (v >> 8 & 255) as u8, (v & 255) as u8]
}

/// An approximate colour for a colour number: 1–9 the basic colours,
/// 10–249 by hue (24 hues, five shades, full and half saturation), 250–255
/// greys.
fn aci_rgb(i: u8) -> Rgb {
    match i {
        1 => [255, 0, 0],
        2 => [255, 255, 0],
        3 => [0, 255, 0],
        4 => [0, 255, 255],
        5 => [0, 0, 255],
        6 => [255, 0, 255],
        8 => [128, 128, 128],
        9 => [192, 192, 192],
        10..=249 => {
            let hue = ((i / 10 - 1) as f64) * 15.0;
            let shade = [1.0, 1.0, 0.8, 0.8, 0.6, 0.6, 0.5, 0.5, 0.3, 0.3][(i % 10) as usize];
            let sat = if i.is_multiple_of(2) { 1.0 } else { 0.5 };
            hsv(hue, sat, shade)
        }
        250..=255 => {
            let g = 51 + (i - 250) as u16 * 40;
            let g = g.min(255) as u8;
            [g, g, g]
        }
        // 7 (white on black, black on paper) and anything else: black.
        _ => [0, 0, 0],
    }
}

fn hsv(h: f64, s: f64, v: f64) -> Rgb {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let f = |v: f64| ((v + m) * 255.0).round() as u8;
    [f(r), f(g), f(b)]
}

fn snap_weight(w: f64) -> f64 {
    *LINE_WEIGHTS
        .iter()
        .min_by(|a, b| (*a - w).abs().total_cmp(&(*b - w).abs()))
        .unwrap_or(&0.25)
}

/// The line type of a DXF line type name (by what its name says).
fn line_style(name: &str) -> LineStyle {
    let n = name.to_uppercase();
    let has = |s: &str| n.contains(s);
    if n == "CONTINUOUS" || n.is_empty() {
        LineStyle::Continuous
    } else if has("DOS_PUNTOS") || has("DIVIDE") {
        LineStyle::DashDotDot
    } else if has("TRAZO_PUNTO") || has("DASHDOT") || has("CENTER") || has("CENTRE") || has("EJE") {
        LineStyle::DashDot
    } else if has("LARGO") || has("LONG") || has("PHANTOM") {
        LineStyle::LongDash
    } else if has("PUNT") || has("DOT") {
        LineStyle::Dotted
    } else if has("DISCON") || has("DASH") || has("HIDDEN") || has("OCULT") || has("TRAZO") {
        LineStyle::Dashed
    } else {
        LineStyle::Continuous
    }
}

/// Adds a drawing to a view, moved by `offset` (metres): layers by name
/// (new ones created with the DXF's look), elements with their look. `k`:
/// drawing metres per paper millimetre of the view (texts keep their size
/// on paper).
pub fn add_to(
    model: &mut Model,
    view: ViewRef,
    drawing: &Drawing,
    offset: Vec2,
    k: f64,
) -> Result<Report, ModelError> {
    let mut report = Report {
        skipped: drawing.skipped.clone(),
        ..Default::default()
    };
    let mut ids = BTreeMap::new();
    let defs: BTreeMap<&str, &LayerDef> = drawing
        .layers
        .iter()
        .map(|l| (l.name.as_str(), l))
        .collect();
    let move_by = Transform::translate(offset);
    let mut groups: BTreeMap<usize, (String, Vec<archisim_core::DraftId>)> = BTreeMap::new();
    for item in &drawing.items {
        let id = match ids.get(&item.layer) {
            Some(id) => *id,
            None => {
                let id = match model.draw_layer_by_name(&item.layer) {
                    Some(l) => l.id,
                    None => {
                        let d = defs.get(item.layer.as_str());
                        let mut l = DrawLayer::new(
                            item.layer.clone(),
                            d.map_or([0, 0, 0], |d| d.color),
                            d.map_or(0.25, |d| d.weight),
                            d.map_or(LineStyle::Continuous, |d| d.line),
                        );
                        l.visible = d.is_none_or(|d| d.visible);
                        report.layers_new += 1;
                        model.add_draw_layer(l)?
                    }
                };
                ids.insert(item.layer.clone(), id);
                id
            }
        };
        let mut shape = transform_shape(&item.shape, &move_by);
        if let Shape::Text(t) = &mut shape {
            t.height = (t.height / k).max(0.1);
        }
        // Degenerate pieces (a zero radius, a polyline on one point) are
        // left out rather than stopping the import.
        if archisim_core::check_shape(&shape).is_err() {
            *report.skipped.entry("degenerados".into()).or_insert(0) += 1;
            continue;
        }
        let mut d = NewDraft::new(view, id, shape);
        d.color = item.color;
        d.weight = item.weight;
        d.line = item.line;
        let id = model.add_draft(d)?;
        if let Some((n, name)) = &item.block {
            groups
                .entry(*n)
                .or_insert_with(|| (name.clone(), Vec::new()))
                .1
                .push(id);
        }
        report.elements += 1;
    }
    for (name, ids) in groups.into_values() {
        if ids.len() > 1 {
            model.group_drafts(&ids, &name)?;
            report.groups += 1;
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_it_writes_and_more() {
        // A small drawing as another program would write it.
        let dxf = "0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n\
0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLAYER\n0\nLAYER\n2\nMUROS\n70\n0\n62\n1\n6\nCONTINUOUS\n370\n50\n0\nLAYER\n2\nEJES\n70\n0\n62\n-3\n6\nCENTER\n0\nENDTAB\n0\nENDSEC\n\
0\nSECTION\n2\nBLOCKS\n0\nBLOCK\n2\nPILAR\n10\n0\n20\n0\n0\nLWPOLYLINE\n8\n0\n90\n4\n70\n1\n10\n-150\n20\n-150\n10\n150\n20\n-150\n10\n150\n20\n150\n10\n-150\n20\n150\n0\nENDBLK\n0\nENDSEC\n\
0\nSECTION\n2\nENTITIES\n\
0\nLINE\n8\nMUROS\n10\n0\n20\n0\n11\n5000\n21\n0\n\
0\nARC\n8\nMUROS\n10\n0\n20\n0\n40\n1000\n50\n0\n51\n90\n\
0\nINSERT\n8\nEJES\n2\nPILAR\n10\n2000\n20\n1000\n41\n2\n42\n2\n50\n90\n\
0\nMTEXT\n8\nTEXTOS\n10\n100\n20\n200\n40\n125\n71\n7\n1\n{\\fArial|b1;Sal\\U+00F3n}\\P20 m2\n\
0\nTEXT\n8\nTEXTOS\n10\n0\n20\n0\n40\n250\n1\n%%c 12\n\
0\n3DSOLID\n8\nMUROS\n\
0\nENDSEC\n0\nEOF\n";
        let d = read(dxf.as_bytes()).unwrap();
        assert_eq!(d.unit, 0.001);
        assert_eq!(d.layers.len(), 2);
        assert_eq!(d.layers[1].line, LineStyle::DashDot);
        assert!(!d.layers[1].visible);
        assert_eq!(d.layers[0].weight, 0.5);
        assert_eq!(d.skipped.get("3DSOLID"), Some(&1));
        assert_eq!(d.items.len(), 5, "{:?}", d.items);
        // In metres.
        assert_eq!(
            d.items[0].shape,
            Shape::Line {
                a: Point2::new(0.0, 0.0),
                b: Point2::new(5.0, 0.0)
            }
        );
        // The block: scaled ×2, turned 90°, at (2, 1) m, on the INSERT's
        // layer.
        let pilar = &d.items[2];
        assert_eq!(pilar.layer, "EJES");
        let Shape::Polyline(pl) = &pilar.shape else {
            panic!()
        };
        assert!(
            pl.points[0].distance(Point2::new(2.3, 0.7)) < 1e-9,
            "{:?}",
            pl.points
        );
        let Shape::Text(t) = &d.items[3].shape else {
            panic!()
        };
        assert_eq!(t.content, "Salón\n20 m2");
        assert_eq!(t.align, TextAlign::BottomLeft);
        let Shape::Text(t) = &d.items[4].shape else {
            panic!()
        };
        assert_eq!(t.content, "Ø 12");
        // Into a plan at 1:50: layers by name, texts 125 mm → 2.5 mm.
        let mut m = Model::new("P");
        let lvl = m.add_level("PB", 0.0);
        let r = add_to(&mut m, ViewRef::Level(lvl), &d, Vec2::new(0.0, 0.0), 0.05).unwrap();
        assert_eq!(r.elements, 5);
        assert_eq!(r.layers_new, 3);
        let text = m
            .drafts()
            .find_map(|d| match &d.shape {
                Shape::Text(t) if t.content.starts_with("Sal") => Some(t.clone()),
                _ => None,
            })
            .unwrap();
        assert!((text.height - 2.5).abs() < 1e-9);
        assert!(r.describe().contains("1 3DSOLID"));
    }

    #[test]
    fn plan_out_and_back_in() {
        let mut m = Model::new("P");
        let lvl = m.add_level("PB", 0.0);
        let l = m
            .add_draw_layer(DrawLayer::new(
                "Ejes",
                [255, 0, 0],
                0.13,
                LineStyle::DashDot,
            ))
            .unwrap();
        let view = ViewRef::Level(lvl);
        for s in [
            Shape::Line {
                a: Point2::new(0.0, 0.0),
                b: Point2::new(3.0, 4.0),
            },
            Shape::Arc {
                centre: Point2::new(1.0, 1.0),
                radius: 2.0,
                start: 0.5,
                sweep: 1.0,
            },
            Shape::Polyline(Polyline {
                points: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(2.0, 0.0),
                    Point2::new(2.0, 2.0),
                ],
                bulges: vec![0.0, 0.4, 0.0],
                closed: true,
                width: 0.0,
            }),
            Shape::Ellipse {
                centre: Point2::new(5.0, 5.0),
                axis: Vec2::new(2.0, 0.0),
                ratio: 0.5,
                start: 0.0,
                sweep: TAU,
            },
        ] {
            m.add_draft(NewDraft::new(view, l, s)).unwrap();
        }
        let dxf = crate::dxf::plan_dxf(&m, lvl, &archisim_plan::PlanOptions::default()).unwrap();
        let d = read(dxf.as_bytes()).unwrap();
        let ejes = d.layers.iter().find(|x| x.name == "Ejes").unwrap();
        assert_eq!(
            (ejes.color, ejes.weight, ejes.line),
            ([255, 0, 0], 0.13, LineStyle::DashDot)
        );
        let ours: Vec<&ItemDef> = d.items.iter().filter(|i| i.layer == "Ejes").collect();
        assert_eq!(ours.len(), 4);
        for (a, b) in ours.iter().zip(m.drafts()) {
            let close = match (&a.shape, &b.shape) {
                (
                    Shape::Arc {
                        start: s1,
                        sweep: w1,
                        ..
                    },
                    Shape::Arc {
                        start: s2,
                        sweep: w2,
                        ..
                    },
                ) => (s1 - s2).abs() < 1e-6 && (w1 - w2).abs() < 1e-6,
                (x, y) => {
                    format!("{x:?}").len() > 10
                        && std::mem::discriminant(x) == std::mem::discriminant(y)
                }
            };
            assert!(close, "{:?} vs {:?}", a.shape, b.shape);
        }
    }

    #[test]
    fn blocks_stay_together_and_metres_are_guessed() {
        // A library of two trees drawn in metres, with no unit said: each
        // tree a block of two circles, placed twice.
        let dxf = "0\nSECTION\n2\nBLOCKS\n\
0\nBLOCK\n2\nARBOL\n10\n0\n20\n0\n\
0\nCIRCLE\n8\n0\n10\n0\n20\n0\n40\n2.5\n\
0\nCIRCLE\n8\n0\n10\n0\n20\n0\n40\n0.2\n\
0\nENDBLK\n0\nENDSEC\n\
0\nSECTION\n2\nENTITIES\n\
0\nINSERT\n8\nARBOLES\n2\nARBOL\n10\n0\n20\n0\n\
0\nINSERT\n8\nARBOLES\n2\nARBOL\n10\n8\n20\n0\n\
0\nLINE\n8\nSUELO\n10\n-3\n20\n-3\n11\n11\n21\n-3\n\
0\nENDSEC\n0\nEOF\n";
        let d = read(dxf.as_bytes()).unwrap();
        assert!(!d.unit_stated);
        assert_eq!(d.unit, 0.001);
        // 14 units across: millimetres would make it 14 mm wide.
        assert_eq!(d.likely_unit(), 1.0);
        let d = d.in_unit(1.0);
        let (lo, hi) = d.extent().unwrap();
        assert!((hi.x - lo.x - 14.0).abs() < 1e-9, "{lo:?} {hi:?}");
        // The two trees, each a block of two elements; the line on its own.
        assert_eq!(d.items[0].block, Some((0, "ARBOL".into())));
        assert_eq!(d.items[2].block, Some((1, "ARBOL".into())));
        assert_eq!(d.items[4].block, None);
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let r = add_to(&mut m, ViewRef::Level(l), &d, Vec2::new(0.0, 0.0), 0.05).unwrap();
        assert_eq!(r.groups, 2);
        let groups: std::collections::BTreeSet<u64> = m
            .drafts()
            .filter_map(|x| x.group.as_ref().map(|g| g.id))
            .collect();
        assert_eq!(groups.len(), 2);
        let g = *groups.iter().next().unwrap();
        assert_eq!(m.draft_group(g).count(), 2);
        assert_eq!(m.draft_group(g).next().unwrap().group.as_ref().unwrap().name, "ARBOL");
        // A drawing that says millimetres and is a building's size stays.
        let big = "0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n\
0\nSECTION\n2\nENTITIES\n0\nLINE\n8\n0\n10\n0\n20\n0\n11\n12000\n21\n0\n0\nENDSEC\n0\nEOF\n";
        let d = read(big.as_bytes()).unwrap();
        assert!(d.unit_stated);
        assert_eq!(d.likely_unit(), 0.001);
    }
}
