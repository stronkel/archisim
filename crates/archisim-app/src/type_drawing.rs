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
        for k in 0..2 {
            self.lo[k] = self.lo[k].min(p[k]);
            self.hi[k] = self.hi[k].max(p[k]);
        }
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
    door: bool,
    leaves: usize,
}

fn body(source: &Model, t: &OpeningType) -> Option<Body> {
    let (model, _) = opening_model(source, t)?;
    let o = model.openings().next()?;
    let parts = model.evaluate_opening(o.id).ok()?.parts;
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
        door: t.is_door(),
        leaves,
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
    // Farthest first: the viewer stands on +y.
    let mut parts = b.parts.clone();
    parts.sort_by(|a, b| a.max[1].total_cmp(&b.max[1]));
    for p in &parts {
        d.rect(
            [p.min[0], p.min[2]],
            [p.max[0], p.max[2]],
            seen(&b.model, p.material),
            None,
            Pen::Medium,
        );
    }
    // The rough opening and, for doors, the floor.
    d.line([-w, 0.0], [-w, h], Pen::Medium, false);
    d.line([w, 0.0], [w, h], Pen::Medium, false);
    d.line([-w, h], [w, h], Pen::Medium, false);
    if b.door {
        d.line([x0, 0.0], [x1, 0.0], Pen::Thick, false);
    } else {
        d.line([-w, 0.0], [w, 0.0], Pen::Medium, false);
    }
    Some(d.pad(0.03))
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
    wall_band(&mut d, h, h + SURROUND);
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
    // Parts beyond the cut (towards the right jamb) seen, then those cut.
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
        let (fill, pattern) = cut(&b.model, p.material);
        d.rect(
            [p.min[1], p.min[2]],
            [p.max[1], p.max[2]],
            fill,
            pattern.filter(|_| p.max[1] - p.min[1] > 0.03),
            Pen::Thick,
        );
    }
    Some(d.pad(0.03))
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

#[cfg(test)]
mod tests {
    use super::*;

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
