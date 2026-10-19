//! DXF (AutoCAD 2007 format, AC1021, text in UTF-8) of a plan or a sheet.
//!
//! A plan is written in model space in millimetres at real size, with the
//! line type scale of its drawing scale; a sheet, in paper millimetres. The
//! project's 2D drafting goes out natively, on its own layers (lines,
//! polylines with arcs, circles, arcs, ellipses, points, multiline texts
//! and hatches with their pattern); walls, slabs, doors and windows and
//! dimensions go out as the lines and fills of the drawing, on layers by
//! category ("A-MURO-CORTE", "A-CARPINTERIA"…).

use std::collections::BTreeMap;
use std::fmt::Write;

use archisim_core::{
    Category, DrawLayer, Hatch, LevelId, LineStyle, Model, Point2, Polyline, Shape, SheetId,
    TextAlign, ViewRef,
};
use archisim_plan::{render_plan, PlanOptions};

use crate::paths::{self, Item};

/// Lineweights DXF knows, hundredths of a millimetre.
const LINEWEIGHTS: [i32; 24] = [
    0, 5, 9, 13, 15, 18, 20, 25, 30, 35, 40, 50, 53, 60, 70, 80, 90, 100, 106, 120, 140, 158, 200,
    211,
];

/// The DXF of a level's plan with the given options (scale, cut plane,
/// what the view shows).
pub fn plan_dxf(model: &Model, level: LevelId, options: &PlanOptions) -> Result<String, String> {
    // The drafting goes natively; the rest from the drawing.
    let mut o = options.clone();
    o.style.hidden.insert(Category::Drafting);
    o.title = false;
    o.viewport = None;
    o.px_per_mm = None;
    let svg = render_plan(model, level, &o);
    let k = options.scale / 1000.0;
    let mut d = Dxf::new(options.scale);
    d.drawing(&svg, k, 1000.0)?;
    if options.style.shows(Category::Drafting) {
        for draft in model.drafts_in(ViewRef::Level(level)) {
            if model.draft_visible(draft) && options.style.shows_layer(draft.layer) {
                d.draft(model, draft, k);
            }
        }
    }
    Ok(d.finish(model))
}

/// The DXF of a section or elevation, in millimetres: along its line (x)
/// and in height (y), the ±0,00 at y = 0.
pub fn section_dxf(
    model: &Model,
    d: &archisim_plan::section::SectionDrawing,
    options: &PlanOptions,
) -> Result<String, String> {
    let mut o = options.clone();
    o.title = false;
    o.viewport = None;
    o.px_per_mm = None;
    let svg = archisim_plan::section::render_section(model, d, &o);
    let k = options.scale / 1000.0;
    let mut dx = Dxf::new(options.scale);
    dx.drawing(&svg, k, 1000.0)?;
    Ok(dx.finish(model))
}

/// The DXF of a sheet, in paper millimetres: its views (cut at their
/// frames), title block and what is drawn on it.
pub fn sheet_dxf(model: &Model, sheet: SheetId) -> Result<String, String> {
    let svg = archisim_plan::sheet::render_sheet(
        model,
        sheet,
        &archisim_plan::sheet::SheetOptions {
            without_drafts: true,
            ..Default::default()
        },
    );
    let mut d = Dxf::new(1.0);
    d.drawing(&svg, 0.001, 1000.0)?;
    for draft in model.drafts_in(ViewRef::Sheet(sheet)) {
        if model.draft_visible(draft) {
            d.draft(model, draft, 0.001);
        }
    }
    Ok(d.finish(model))
}

/// A closed boundary: points and bulges, millimetres.
type Loop = (Vec<(f64, f64)>, Vec<f64>);

/// A layer as it will be written.
#[derive(Debug, Clone)]
struct Layer {
    color: [u8; 3],
    weight: i32,
    line: LineStyle,
    off: bool,
    frozen_print: bool,
}

struct Dxf {
    next: u32,
    ltscale: f64,
    layers: BTreeMap<String, Layer>,
    lines_used: Vec<LineStyle>,
    entities: String,
    /// Fixed handles of the structure.
    model_record: u32,
}

/// Millimetres per drawing unit: model coordinates are metres.
const MM: f64 = 1000.0;

impl Dxf {
    fn new(ltscale: f64) -> Self {
        let mut d = Dxf {
            next: 0x30,
            ltscale,
            layers: BTreeMap::new(),
            lines_used: Vec::new(),
            entities: String::new(),
            model_record: 0x1F,
        };
        d.layer("0", [255, 255, 255], 25, LineStyle::Continuous);
        d
    }

    fn handle(&mut self) -> u32 {
        let h = self.next;
        self.next += 1;
        h
    }

    fn layer(&mut self, name: &str, color: [u8; 3], weight: i32, line: LineStyle) {
        self.layers.entry(name.to_string()).or_insert(Layer {
            color,
            weight,
            line,
            off: false,
            frozen_print: false,
        });
        self.use_line(line);
    }

    fn use_line(&mut self, line: LineStyle) {
        if line != LineStyle::Continuous && !self.lines_used.contains(&line) {
            self.lines_used.push(line);
        }
    }

    /// Starts an entity: its handle, owner, layer and look.
    fn start(&mut self, kind: &str, layer: &str, look: &Look) {
        let h = self.handle();
        let s = &mut self.entities;
        let _ = write!(
            s,
            "0\n{kind}\n5\n{h:X}\n330\n{:X}\n100\nAcDbEntity\n8\n{}\n",
            self.model_record,
            text(layer)
        );
        if let Some(c) = look.color {
            let _ = write!(s, "62\n{}\n", aci(c));
            if aci(c) != 7 {
                let _ = write!(s, "420\n{}\n", true_color(c));
            }
        }
        if let Some(l) = look.line {
            let _ = write!(s, "6\n{}\n", ltype_name(l));
        }
        if let Some(w) = look.weight {
            let _ = write!(s, "370\n{}\n", lineweight(w));
        }
    }

    fn line(&mut self, layer: &str, look: &Look, a: (f64, f64), b: (f64, f64)) {
        self.start("LINE", layer, look);
        let _ = write!(
            self.entities,
            "100\nAcDbLine\n10\n{}\n20\n{}\n30\n0\n11\n{}\n21\n{}\n31\n0\n",
            num(a.0),
            num(a.1),
            num(b.0),
            num(b.1)
        );
    }

    fn lwpolyline(
        &mut self,
        layer: &str,
        look: &Look,
        pts: &[(f64, f64)],
        bulges: &[f64],
        closed: bool,
        width: f64,
    ) {
        if pts.len() == 2 && !closed && bulges.iter().all(|b| b.abs() < 1e-12) && width == 0.0 {
            return self.line(layer, look, pts[0], pts[1]);
        }
        self.start("LWPOLYLINE", layer, look);
        let s = &mut self.entities;
        let _ = write!(
            s,
            "100\nAcDbPolyline\n90\n{}\n70\n{}\n",
            pts.len(),
            if closed { 1 } else { 0 }
        );
        if width > 0.0 {
            let _ = write!(s, "43\n{}\n", num(width));
        }
        for (i, p) in pts.iter().enumerate() {
            let _ = write!(s, "10\n{}\n20\n{}\n", num(p.0), num(p.1));
            let b = bulges.get(i).copied().unwrap_or(0.0);
            if b.abs() > 1e-12 && (closed || i + 1 < pts.len()) {
                let _ = write!(s, "42\n{}\n", num(b));
            }
        }
    }

    /// A solid fill, or a hatch with its pattern lines, bounded by
    /// polylines (even-odd).
    fn hatch(&mut self, layer: &str, look: &Look, loops: &[Loop], pattern: Option<HatchDef>) {
        if loops.is_empty() {
            return;
        }
        self.start("HATCH", layer, look);
        let s = &mut self.entities;
        let (name, solid) = match &pattern {
            Some(p) => (p.name.as_str(), 0),
            None => ("SOLID", 1),
        };
        let _ = write!(
            s,
            "100\nAcDbHatch\n10\n0\n20\n0\n30\n0\n210\n0\n220\n0\n230\n1\n2\n{name}\n70\n{solid}\n71\n0\n91\n{}\n",
            loops.len()
        );
        for (pts, bulges) in loops {
            let has_bulge = bulges.iter().any(|b| b.abs() > 1e-12);
            let _ = write!(
                s,
                "92\n2\n72\n{}\n73\n1\n93\n{}\n",
                if has_bulge { 1 } else { 0 },
                pts.len()
            );
            for (i, p) in pts.iter().enumerate() {
                let _ = write!(s, "10\n{}\n20\n{}\n", num(p.0), num(p.1));
                if has_bulge {
                    let _ = write!(s, "42\n{}\n", num(bulges.get(i).copied().unwrap_or(0.0)));
                }
            }
            s.push_str("97\n0\n");
        }
        // Odd parity (islands are holes), custom pattern.
        s.push_str("75\n0\n76\n2\n");
        if let Some(p) = pattern {
            let _ = write!(
                s,
                "52\n{}\n41\n{}\n77\n0\n78\n{}\n",
                num(p.angle),
                num(p.scale),
                p.lines.len()
            );
            for l in &p.lines {
                let _ = write!(
                    s,
                    "53\n{}\n43\n{}\n44\n{}\n45\n{}\n46\n{}\n79\n{}\n",
                    num(l.angle),
                    num(l.base.0),
                    num(l.base.1),
                    num(l.offset.0),
                    num(l.offset.1),
                    l.dashes.len()
                );
                for d in &l.dashes {
                    let _ = write!(s, "49\n{}\n", num(*d));
                }
            }
        }
        s.push_str("98\n0\n");
    }

    /// The paths of an SVG drawing: `k` user units per paper millimetre,
    /// `mm` millimetres per user unit (y is turned up).
    fn drawing(&mut self, svg: &str, k: f64, mm: f64) -> Result<(), String> {
        for it in paths::user_items(svg)? {
            self.item(&it, k, mm);
        }
        Ok(())
    }

    fn item(&mut self, it: &Item, k: f64, mm: f64) {
        let (name, color, weight) = category_layer(&it.group, it.nested);
        self.layer(name, color, weight, LineStyle::Continuous);
        // Whole thousandths of a millimetre (the drawing is read in single
        // precision).
        let round = |v: f64| (v * 1000.0).round() / 1000.0;
        let to = |p: &(f64, f64)| (round(p.0 * mm), round(-p.1 * mm));
        if let Some(f) = it.fill {
            // White and near-white fills (paper, slabs) are left out.
            if f.iter().any(|c| *c < 230) {
                let loops: Vec<Loop> = it
                    .subpaths
                    .iter()
                    .filter(|(p, closed)| *closed && p.len() >= 3)
                    .map(|(p, _)| (p.iter().map(to).collect(), Vec::new()))
                    .collect();
                let look = Look {
                    color: Some(f),
                    ..Look::default()
                };
                self.hatch(name, &look, &loops, None);
            }
        }
        if let Some((c, w)) = it.stroke {
            // Dark ink takes the layer's colour.
            let dark = c.iter().all(|v| *v < 60);
            let look = Look {
                color: (!dark).then_some(c),
                weight: Some(w / k),
                line: line_of(&it.dashes, k),
            };
            if let Some(l) = look.line {
                self.use_line(l);
            }
            for (pts, closed) in &it.subpaths {
                let pts: Vec<(f64, f64)> = pts.iter().map(to).collect();
                if pts.len() == 1 {
                    self.line(name, &look, pts[0], pts[0]);
                } else {
                    self.lwpolyline(name, &look, &pts, &[], *closed, 0.0);
                }
            }
        }
    }

    /// A drafting element of the project, natively. `k`: model metres per
    /// paper millimetre.
    fn draft(&mut self, model: &Model, d: &archisim_core::Draft, k: f64) {
        let zero = DrawLayer::zero();
        let layer = model.draw_layer(d.layer).unwrap_or(&zero);
        self.layers.entry(layer.name.clone()).or_insert(Layer {
            color: layer.color,
            weight: lineweight(layer.weight),
            line: layer.line,
            off: !layer.visible,
            frozen_print: !layer.printable,
        });
        self.use_line(layer.line);
        let look = Look {
            color: d.color,
            weight: d.weight,
            line: d.line,
        };
        if let Some(l) = d.line {
            self.use_line(l);
        }
        let name = layer.name.clone();
        let p = |q: Point2| (q.x * MM, q.y * MM);
        match &d.shape {
            Shape::Line { a, b } => self.line(&name, &look, p(*a), p(*b)),
            Shape::Polyline(pl) => {
                let pts: Vec<_> = pl.points.iter().map(|q| p(*q)).collect();
                self.lwpolyline(&name, &look, &pts, &pl.bulges, pl.closed, pl.width * MM);
            }
            Shape::Circle { centre, radius } => {
                self.start("CIRCLE", &name, &look);
                let c = p(*centre);
                let _ = write!(
                    self.entities,
                    "100\nAcDbCircle\n10\n{}\n20\n{}\n30\n0\n40\n{}\n",
                    num(c.0),
                    num(c.1),
                    num(radius * MM)
                );
            }
            Shape::Arc {
                centre,
                radius,
                start,
                sweep,
            } => {
                self.start("ARC", &name, &look);
                let c = p(*centre);
                let _ = write!(
                    self.entities,
                    "100\nAcDbCircle\n10\n{}\n20\n{}\n30\n0\n40\n{}\n100\nAcDbArc\n50\n{}\n51\n{}\n",
                    num(c.0),
                    num(c.1),
                    num(radius * MM),
                    num(start.to_degrees()),
                    num((start + sweep).to_degrees())
                );
            }
            Shape::Ellipse {
                centre,
                axis,
                ratio,
                start,
                sweep,
            } => {
                self.start("ELLIPSE", &name, &look);
                let c = p(*centre);
                let _ = write!(
                    self.entities,
                    "100\nAcDbEllipse\n10\n{}\n20\n{}\n30\n0\n11\n{}\n21\n{}\n31\n0\n210\n0\n220\n0\n230\n1\n40\n{}\n41\n{}\n42\n{}\n",
                    num(c.0),
                    num(c.1),
                    num(axis.x * MM),
                    num(axis.y * MM),
                    num(*ratio),
                    num(*start),
                    num(start + sweep)
                );
            }
            Shape::Point { at } => {
                self.start("POINT", &name, &look);
                let c = p(*at);
                let _ = write!(
                    self.entities,
                    "100\nAcDbPoint\n10\n{}\n20\n{}\n30\n0\n",
                    num(c.0),
                    num(c.1)
                );
            }
            Shape::Text(t) => {
                self.start("MTEXT", &name, &look);
                let c = p(t.at);
                let attach = TextAlign::ALL
                    .iter()
                    .position(|a| *a == t.align)
                    .unwrap_or(6)
                    + 1;
                let content = t
                    .content
                    .replace('\\', "\\\\")
                    .replace('{', "\\{")
                    .replace('}', "\\}")
                    .replace('\n', "\\P");
                let _ = write!(
                    self.entities,
                    "100\nAcDbMText\n10\n{}\n20\n{}\n30\n0\n40\n{}\n41\n0\n71\n{attach}\n72\n1\n1\n{}\n7\nStandard\n11\n{}\n21\n{}\n31\n0\n44\n{}\n",
                    num(c.0),
                    num(c.1),
                    num(t.height * k * MM),
                    text(&content),
                    num(t.angle.cos()),
                    num(t.angle.sin()),
                    num(archisim_core::LINE_SPACING / 1.666_666)
                );
            }
            Shape::Hatch(h) => {
                let loops: Vec<Loop> = h.loops.iter().map(|l| loop_points(l, MM)).collect();
                if let Some(bg) = h.background {
                    let back = Look {
                        color: Some(bg),
                        ..Look::default()
                    };
                    self.hatch(&name, &back, &loops, None);
                }
                let pattern = hatch_def(h, k);
                self.hatch(&name, &look, &loops, pattern);
            }
            Shape::Dim(m) => {
                // As its pieces: lines and arcs, solid arrowheads, the text.
                let dd = archisim_draft::dims::drawing(m, k, archisim_draft::dims::DIM_TEXT);
                let mut parts: Vec<Shape> =
                    dd.segs.iter().map(archisim_draft::ops::seg_shape).collect();
                for a in &dd.arrows {
                    parts.push(Shape::Hatch(Hatch {
                        loops: vec![Polyline::straight(a.to_vec(), true)],
                        pattern: archisim_core::HatchPattern::Solid,
                        scale: 1.0,
                        angle: 0.0,
                        background: None,
                    }));
                }
                parts.push(Shape::Text(dd.text));
                for p in parts {
                    let mut sub = d.clone();
                    sub.shape = p;
                    self.draft(model, &sub, k);
                }
            }
        }
    }

    fn finish(mut self, model: &Model) -> String {
        let _ = model;
        // Handles of the structure.
        let ids = Ids {
            block_records: 0x10,
            layers: 0x11,
            ltypes: 0x12,
            styles: 0x13,
            views: 0x14,
            ucs: 0x15,
            appids: 0x16,
            dimstyles: 0x17,
            vports: 0x18,
            root: 0x19,
            groups: 0x1A,
            layouts: 0x1B,
            model_layout: 0x1C,
            paper_layout: 0x1D,
            paper_record: 0x1E,
            model_record: self.model_record,
            model_block: 0x20,
            model_end: 0x21,
            paper_block: 0x22,
            paper_end: 0x23,
            vport: 0x24,
            style: 0x25,
            appid: 0x26,
            dimstyle: 0x27,
            by_block: 0x28,
            by_layer: 0x29,
            continuous: 0x2A,
        };
        let mut tables = String::new();
        self.tables(&mut tables, &ids);
        let mut out = String::new();
        let seed = self.next + 1;
        let _ = write!(
            out,
            "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1021\n9\n$DWGCODEPAGE\n3\nANSI_1252\n9\n$HANDSEED\n5\n{seed:X}\n9\n$INSUNITS\n70\n4\n9\n$MEASUREMENT\n70\n1\n9\n$LUNITS\n70\n2\n9\n$LTSCALE\n40\n{}\n9\n$PSLTSCALE\n70\n1\n9\n$LWDISPLAY\n290\n1\n9\n$TEXTSTYLE\n7\nStandard\n9\n$CLAYER\n8\n0\n0\nENDSEC\n",
            num(self.ltscale)
        );
        out.push_str("0\nSECTION\n2\nCLASSES\n0\nENDSEC\n");
        out.push_str(&tables);
        let _ = write!(
            out,
            "0\nSECTION\n2\nBLOCKS\n\
0\nBLOCK\n5\n{:X}\n330\n{:X}\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockBegin\n2\n*Model_Space\n70\n0\n10\n0\n20\n0\n30\n0\n3\n*Model_Space\n1\n\n\
0\nENDBLK\n5\n{:X}\n330\n{:X}\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockEnd\n\
0\nBLOCK\n5\n{:X}\n330\n{:X}\n100\nAcDbEntity\n67\n1\n8\n0\n100\nAcDbBlockBegin\n2\n*Paper_Space\n70\n0\n10\n0\n20\n0\n30\n0\n3\n*Paper_Space\n1\n\n\
0\nENDBLK\n5\n{:X}\n330\n{:X}\n100\nAcDbEntity\n67\n1\n8\n0\n100\nAcDbBlockEnd\n0\nENDSEC\n",
            ids.model_block,
            ids.model_record,
            ids.model_end,
            ids.model_record,
            ids.paper_block,
            ids.paper_record,
            ids.paper_end,
            ids.paper_record
        );
        out.push_str("0\nSECTION\n2\nENTITIES\n");
        out.push_str(&self.entities);
        out.push_str("0\nENDSEC\n");
        objects(&mut out, &ids);
        out.push_str("0\nEOF\n");
        out
    }

    fn tables(&mut self, s: &mut String, ids: &Ids) {
        let table = |s: &mut String, name: &str, h: u32, n: usize| {
            let _ = write!(
                s,
                "0\nTABLE\n2\n{name}\n5\n{h:X}\n330\n0\n100\nAcDbSymbolTable\n70\n{n}\n"
            );
        };
        let record = |s: &mut String, kind: &str, h: u32, owner: u32, sub: &str| {
            let _ = write!(
                s,
                "0\n{kind}\n5\n{h:X}\n330\n{owner:X}\n100\nAcDbSymbolTableRecord\n100\n{sub}\n"
            );
        };
        s.push_str("0\nSECTION\n2\nTABLES\n");
        // Viewport: the whole drawing.
        table(s, "VPORT", ids.vports, 1);
        record(s, "VPORT", ids.vport, ids.vports, "AcDbViewportTableRecord");
        s.push_str("2\n*Active\n70\n0\n10\n0\n20\n0\n11\n1\n21\n1\n12\n0\n22\n0\n13\n0\n23\n0\n14\n10\n24\n10\n15\n10\n25\n10\n16\n0\n26\n0\n36\n1\n17\n0\n27\n0\n37\n0\n40\n1000\n41\n1.5\n42\n50\n43\n0\n44\n0\n50\n0\n51\n0\n71\n0\n72\n100\n73\n1\n74\n3\n75\n0\n76\n0\n77\n0\n78\n0\n");
        s.push_str("0\nENDTAB\n");
        // Line types.
        let lines = self.lines_used.clone();
        table(s, "LTYPE", ids.ltypes, 3 + lines.len());
        record(
            s,
            "LTYPE",
            ids.by_block,
            ids.ltypes,
            "AcDbLinetypeTableRecord",
        );
        s.push_str("2\nByBlock\n70\n0\n3\n\n72\n65\n73\n0\n40\n0\n");
        record(
            s,
            "LTYPE",
            ids.by_layer,
            ids.ltypes,
            "AcDbLinetypeTableRecord",
        );
        s.push_str("2\nByLayer\n70\n0\n3\n\n72\n65\n73\n0\n40\n0\n");
        record(
            s,
            "LTYPE",
            ids.continuous,
            ids.ltypes,
            "AcDbLinetypeTableRecord",
        );
        s.push_str("2\nContinuous\n70\n0\n3\nContinua\n72\n65\n73\n0\n40\n0\n");
        for l in &lines {
            let h = self.handle();
            record(s, "LTYPE", h, ids.ltypes, "AcDbLinetypeTableRecord");
            let pattern = l.pattern_mm();
            let total: f64 = pattern.iter().sum();
            let _ = write!(
                s,
                "2\n{}\n70\n0\n3\n{}\n72\n65\n73\n{}\n40\n{}\n",
                ltype_name(*l),
                text(l.label()),
                pattern.len(),
                num(total)
            );
            for (i, v) in pattern.iter().enumerate() {
                let v = if i % 2 == 0 { *v } else { -*v };
                let _ = write!(s, "49\n{}\n74\n0\n", num(v));
            }
        }
        s.push_str("0\nENDTAB\n");
        // Layers.
        let layers: Vec<(String, Layer)> = self
            .layers
            .iter()
            .map(|(a, b)| (a.clone(), b.clone()))
            .collect();
        table(s, "LAYER", ids.layers, layers.len());
        for (name, l) in &layers {
            let h = self.handle();
            record(s, "LAYER", h, ids.layers, "AcDbLayerTableRecord");
            let c = aci(l.color) as i32;
            // Black and white inks are colour 7 alone: black on paper,
            // white on a dark screen.
            let tc = if c == 7 {
                String::new()
            } else {
                format!("420\n{}\n", true_color(l.color))
            };
            let _ = write!(
                s,
                "2\n{}\n70\n0\n62\n{}\n{tc}6\n{}\n290\n{}\n370\n{}\n",
                text(name),
                if l.off { -c } else { c },
                ltype_name(l.line),
                if l.frozen_print { 0 } else { 1 },
                l.weight
            );
        }
        s.push_str("0\nENDTAB\n");
        table(s, "STYLE", ids.styles, 1);
        record(
            s,
            "STYLE",
            ids.style,
            ids.styles,
            "AcDbTextStyleTableRecord",
        );
        s.push_str("2\nStandard\n70\n0\n40\n0\n41\n1\n50\n0\n71\n0\n42\n2.5\n3\ntxt\n4\n\n");
        s.push_str("0\nENDTAB\n");
        table(s, "VIEW", ids.views, 0);
        s.push_str("0\nENDTAB\n");
        table(s, "UCS", ids.ucs, 0);
        s.push_str("0\nENDTAB\n");
        table(s, "APPID", ids.appids, 1);
        record(s, "APPID", ids.appid, ids.appids, "AcDbRegAppTableRecord");
        s.push_str("2\nACAD\n70\n0\n");
        s.push_str("0\nENDTAB\n");
        let _ = write!(
            s,
            "0\nTABLE\n2\nDIMSTYLE\n5\n{:X}\n330\n0\n100\nAcDbSymbolTable\n70\n1\n100\nAcDbDimStyleTable\n71\n0\n",
            ids.dimstyles
        );
        let _ = write!(
            s,
            "0\nDIMSTYLE\n105\n{:X}\n330\n{:X}\n100\nAcDbSymbolTableRecord\n100\nAcDbDimStyleTableRecord\n2\nStandard\n70\n0\n",
            ids.dimstyle, ids.dimstyles
        );
        s.push_str("0\nENDTAB\n");
        table(s, "BLOCK_RECORD", ids.block_records, 2);
        record(
            s,
            "BLOCK_RECORD",
            ids.model_record,
            ids.block_records,
            "AcDbBlockTableRecord",
        );
        let _ = write!(s, "2\n*Model_Space\n340\n{:X}\n", ids.model_layout);
        record(
            s,
            "BLOCK_RECORD",
            ids.paper_record,
            ids.block_records,
            "AcDbBlockTableRecord",
        );
        let _ = write!(s, "2\n*Paper_Space\n340\n{:X}\n", ids.paper_layout);
        s.push_str("0\nENDTAB\n0\nENDSEC\n");
    }
}

struct Ids {
    block_records: u32,
    layers: u32,
    ltypes: u32,
    styles: u32,
    views: u32,
    ucs: u32,
    appids: u32,
    dimstyles: u32,
    vports: u32,
    root: u32,
    groups: u32,
    layouts: u32,
    model_layout: u32,
    paper_layout: u32,
    paper_record: u32,
    model_record: u32,
    model_block: u32,
    model_end: u32,
    paper_block: u32,
    paper_end: u32,
    vport: u32,
    style: u32,
    appid: u32,
    dimstyle: u32,
    by_block: u32,
    by_layer: u32,
    continuous: u32,
}

/// The objects every drawing has: the named dictionary with groups and
/// layouts (model and one sheet).
fn objects(s: &mut String, ids: &Ids) {
    let _ = write!(
        s,
        "0\nSECTION\n2\nOBJECTS\n\
0\nDICTIONARY\n5\n{:X}\n330\n0\n100\nAcDbDictionary\n281\n1\n3\nACAD_GROUP\n350\n{:X}\n3\nACAD_LAYOUT\n350\n{:X}\n\
0\nDICTIONARY\n5\n{:X}\n330\n{:X}\n100\nAcDbDictionary\n281\n1\n\
0\nDICTIONARY\n5\n{:X}\n330\n{:X}\n100\nAcDbDictionary\n281\n1\n3\nLayout1\n350\n{:X}\n3\nModel\n350\n{:X}\n",
        ids.root,
        ids.groups,
        ids.layouts,
        ids.groups,
        ids.root,
        ids.layouts,
        ids.root,
        ids.paper_layout,
        ids.model_layout
    );
    for (h, name, order, record) in [
        (ids.model_layout, "Model", 0, ids.model_record),
        (ids.paper_layout, "Layout1", 1, ids.paper_record),
    ] {
        let _ = write!(
            s,
            "0\nLAYOUT\n5\n{h:X}\n330\n{:X}\n100\nAcDbPlotSettings\n1\n\n4\nA3\n6\n\n40\n7.5\n41\n20\n42\n7.5\n43\n20\n44\n420\n45\n297\n46\n0\n47\n0\n48\n0\n49\n0\n140\n0\n141\n0\n142\n1\n143\n1\n70\n{}\n72\n1\n73\n0\n74\n5\n7\n\n75\n16\n147\n1\n148\n0\n149\n0\n\
100\nAcDbLayout\n1\n{name}\n70\n1\n71\n{order}\n10\n0\n20\n0\n11\n420\n21\n297\n12\n0\n22\n0\n32\n0\n14\n0\n24\n0\n34\n0\n15\n0\n25\n0\n35\n0\n146\n0\n13\n0\n23\n0\n33\n0\n16\n1\n26\n0\n36\n0\n17\n0\n27\n1\n37\n0\n76\n0\n330\n{record:X}\n",
            ids.layouts,
            if order == 0 { 1712 } else { 688 }
        );
    }
    s.push_str("0\nENDSEC\n");
}

/// How an entity differs from its layer.
#[derive(Debug, Clone, Copy, Default)]
struct Look {
    color: Option<[u8; 3]>,
    /// Paper millimetres.
    weight: Option<f64>,
    line: Option<LineStyle>,
}

/// A hatch pattern as DXF writes it: lines in drawing millimetres.
struct HatchDef {
    name: String,
    angle: f64,
    scale: f64,
    lines: Vec<PatternLine>,
}

struct PatternLine {
    angle: f64,
    base: (f64, f64),
    offset: (f64, f64),
    dashes: Vec<f64>,
}

/// The pattern of a hatch, as the same families of lines ArchiSim draws.
/// `None` for a solid fill.
fn hatch_def(h: &Hatch, k: f64) -> Option<HatchDef> {
    let families = archisim_draft::hatch::families(h.pattern);
    if families.is_empty() {
        return None;
    }
    let unit = if h.pattern.is_model_scale() {
        h.scale
    } else {
        k * h.scale
    } * MM;
    let (s, c) = h.angle.sin_cos();
    let turn = |x: f64, y: f64| (x * c - y * s, x * s + y * c);
    let lines = families
        .iter()
        .map(|f| {
            let a = f.angle.to_radians() + h.angle;
            let (u, n) = ((a.cos(), a.sin()), (-a.sin(), a.cos()));
            let base = turn(f.origin.0 * unit, f.origin.1 * unit);
            let offset = (
                u.0 * f.delta.0 * unit + n.0 * f.delta.1 * unit,
                u.1 * f.delta.0 * unit + n.1 * f.delta.1 * unit,
            );
            PatternLine {
                angle: a.to_degrees(),
                base,
                offset,
                dashes: f.dashes.iter().map(|d| d * unit).collect(),
            }
        })
        .collect();
    Some(HatchDef {
        name: format!("AS_{}", h.pattern.code().to_uppercase()),
        angle: h.angle.to_degrees(),
        scale: h.scale,
        lines,
    })
}

/// A closed polyline's points and bulges in millimetres.
fn loop_points(l: &Polyline, mm: f64) -> Loop {
    (
        l.points.iter().map(|p| (p.x * mm, p.y * mm)).collect(),
        l.bulges.clone(),
    )
}

/// The layer of what the drawing shows, by its group.
fn category_layer(group: &str, nested: bool) -> (&'static str, [u8; 3], i32) {
    match group {
        "slabs" => ("A-FORJADO", [128, 128, 128], 18),
        "poche" => ("A-MURO-MATERIAL", [192, 192, 192], 9),
        "cut-lines" => ("A-MURO-CORTE", [255, 255, 255], 50),
        "openings" => ("A-CARPINTERIA", [0, 255, 255], 25),
        "annotations" => ("A-COTA", [0, 255, 0], 13),
        "drafting" => ("A-DIBUJO", [255, 255, 255], 18),
        "view-titles" => ("A-TITULO-VISTA", [255, 255, 255], 25),
        "rooms" => ("A-LOCAL-NOMBRE", [255, 255, 255], 18),
        "tags" => ("A-ETIQUETA", [255, 255, 255], 18),
        "tables" => ("A-TABLA", [255, 255, 255], 18),
        "title-block" => ("A-CAJETIN", [255, 255, 255], 25),
        "structure" => ("A-ESTRUCTURA", [255, 0, 0], 50),
        "objects" => ("A-MOBILIARIO", [255, 191, 0], 18),
        "curtain-walls" => ("A-MURO-CORTINA", [0, 255, 255], 25),
        "railings" => ("A-BARANDILLA", [255, 0, 255], 18),
        "section-marks" => ("A-SECCION", [255, 255, 255], 25),
        "north" => ("A-NORTE", [255, 255, 255], 18),
        _ if nested => ("A-VISTA", [255, 255, 255], 18),
        _ => ("A-OTROS", [255, 255, 255], 18),
    }
}

/// The line type whose dashes these are (user units, `k` per paper mm).
fn line_of(dashes: &[f64], k: f64) -> Option<LineStyle> {
    if dashes.is_empty() {
        return None;
    }
    LineStyle::ALL.into_iter().find(|l| {
        let p = l.pattern_mm();
        p.len() == dashes.len()
            && p.iter()
                .zip(dashes)
                .all(|(a, b)| (a - b / k).abs() <= 0.05 * a.max(0.2))
    })
}

fn ltype_name(l: LineStyle) -> &'static str {
    match l {
        LineStyle::Continuous => "Continuous",
        LineStyle::Dashed => "AS_DISCONTINUA",
        LineStyle::Dotted => "AS_PUNTOS",
        LineStyle::DashDot => "AS_TRAZO_PUNTO",
        LineStyle::DashDotDot => "AS_TRAZO_DOS_PUNTOS",
        LineStyle::LongDash => "AS_TRAZOS_LARGOS",
    }
}

/// The nearest lineweight DXF knows, from paper millimetres.
fn lineweight(mm: f64) -> i32 {
    let v = (mm * 100.0).round() as i32;
    *LINEWEIGHTS
        .iter()
        .min_by_key(|w| (**w - v).abs())
        .unwrap_or(&25)
}

/// The nearest of the basic colour numbers (1–9).
fn aci(c: [u8; 3]) -> u8 {
    const BASIC: [(u8, [u8; 3]); 9] = [
        (1, [255, 0, 0]),
        (2, [255, 255, 0]),
        (3, [0, 255, 0]),
        (4, [0, 255, 255]),
        (5, [0, 0, 255]),
        (6, [255, 0, 255]),
        (7, [255, 255, 255]),
        (8, [128, 128, 128]),
        (9, [192, 192, 192]),
    ];
    // Black and dark inks print as colour 7 (black on white paper).
    if c.iter().all(|v| *v < 60) {
        return 7;
    }
    let d = |a: [u8; 3], b: [u8; 3]| {
        (0..3)
            .map(|i| (a[i] as i32 - b[i] as i32).pow(2))
            .sum::<i32>()
    };
    BASIC
        .iter()
        .min_by_key(|(_, b)| d(c, *b))
        .map_or(7, |(n, _)| *n)
}

fn true_color(c: [u8; 3]) -> u32 {
    ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32
}

/// Text for the file (UTF-8; no line breaks).
fn text(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect()
}

fn num(v: f64) -> String {
    let v = if v.abs() < 1e-9 { 0.0 } else { v };
    let s = format!("{v:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" {
        "0".into()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{NewDraft, Text};

    /// Group code and value pairs.
    fn pairs(dxf: &str) -> Vec<(i32, String)> {
        let lines: Vec<&str> = dxf.lines().collect();
        lines
            .chunks(2)
            .map(|c| (c[0].trim().parse().unwrap(), c[1].to_string()))
            .collect()
    }

    #[test]
    fn drafting_goes_out_natively_on_its_layers() {
        let mut m = Model::new("Prueba");
        let lvl = m.add_level("Planta baja", 0.0);
        let ejes = m
            .add_draw_layer(DrawLayer::new(
                "Ejes",
                [255, 0, 0],
                0.13,
                LineStyle::DashDot,
            ))
            .unwrap();
        let view = ViewRef::Level(lvl);
        m.add_draft(NewDraft::new(
            view,
            ejes,
            Shape::Line {
                a: Point2::new(0.0, 0.0),
                b: Point2::new(5.0, 0.0),
            },
        ))
        .unwrap();
        m.add_draft(NewDraft::new(
            view,
            archisim_core::LAYER_ZERO,
            Shape::Text(Text {
                at: Point2::new(1.0, 2.0),
                content: "Salón\n20 m²".into(),
                height: 2.5,
                angle: 0.0,
                align: TextAlign::BottomLeft,
            }),
        ))
        .unwrap();
        m.add_draft(NewDraft::new(
            view,
            archisim_core::LAYER_ZERO,
            Shape::Hatch(Hatch {
                loops: vec![Polyline::straight(
                    vec![
                        Point2::new(0.0, 0.0),
                        Point2::new(3.0, 0.0),
                        Point2::new(3.0, 3.0),
                        Point2::new(0.0, 3.0),
                    ],
                    true,
                )],
                pattern: archisim_core::HatchPattern::Tiles,
                scale: 1.0,
                angle: 0.0,
                background: None,
            }),
        ))
        .unwrap();
        let dxf = plan_dxf(&m, lvl, &PlanOptions::default()).unwrap();
        let p = pairs(&dxf);
        assert_eq!(p.last().unwrap(), &(0, "EOF".to_string()));
        // The line, in millimetres, on its layer.
        let i = p
            .iter()
            .enumerate()
            .position(|(i, x)| {
                x == &(0, "LINE".into()) && p[i..i + 6].contains(&(8, "Ejes".into()))
            })
            .unwrap();
        let after: Vec<_> = p[i..i + 20].to_vec();
        assert!(after.contains(&(8, "Ejes".into())), "{after:?}");
        assert!(after.contains(&(11, "5000".into())), "{after:?}");
        // The layer with its colour, weight and line type.
        assert!(dxf.contains(
            "2\nEjes\n70\n0\n62\n1\n420\n16711680\n6\nAS_TRAZO_PUNTO\n290\n1\n370\n13\n"
        ));
        assert!(dxf.contains("2\nAS_TRAZO_PUNTO\n"));
        // The text, two lines, accents escaped; 2.5 mm at 1:50 = 125 mm.
        assert!(dxf.contains("Salón\\P20 m²"), "{dxf}");
        assert!(dxf.contains("40\n125\n"));
        // The hatch with its tiles: lines every 300 mm both ways.
        assert!(dxf.contains("2\nAS_TILES\n70\n0\n"));
        assert!(
            dxf.contains("53\n90\n43\n0\n44\n0\n45\n-300\n46\n0\n"),
            "{dxf}"
        );
        // Handles are unique.
        let mut handles: Vec<&String> = p
            .iter()
            .filter(|x| x.0 == 5 || x.0 == 105)
            .map(|x| &x.1)
            .collect();
        let n = handles.len();
        handles.sort();
        handles.dedup();
        assert_eq!(handles.len(), n);
    }

    #[test]
    fn the_rest_of_the_plan_as_lines_by_category() {
        let (m, lvl) = demo_room();
        let dxf = plan_dxf(&m, lvl, &PlanOptions::default()).unwrap();
        for layer in ["A-MURO-CORTE", "A-MURO-MATERIAL"] {
            assert!(dxf.contains(&format!("8\n{layer}\n")), "{layer}");
        }
    }

    fn demo_room() -> (Model, LevelId) {
        use archisim_core::{NewWall, ReferenceLine};
        let mut m = Model::new("Prueba");
        let lvl = m.add_level("Planta baja", 0.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        m.add_wall(NewWall {
            wall_type: lib.facade,
            level: lvl,
            start: Point2::new(0.0, 0.0),
            end: Point2::new(5.0, 0.0),
            reference: ReferenceLine::ExteriorFace,
            height: 3.0,
            base_offset: 0.0,
            bulge: 0.0,
            top: archisim_core::WallTop::Unconnected,
        })
        .unwrap();
        (m, lvl)
    }
}
