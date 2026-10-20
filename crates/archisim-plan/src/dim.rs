//! Elements drawn in grey: those a view's discipline focus leaves as
//! reference (`DisciplineFocus`). The drawing of such an element is made
//! as usual and its colours are then turned into light greys, so the
//! greyed plan prints, goes to PDF and to DXF like any other.

use std::cell::RefCell;
use std::collections::HashSet;

use archisim_core::{DisciplineFocus, Model, ViewStyle};

thread_local! {
    /// The ids of the elements the plan being drawn shows in grey.
    static DIMMED: RefCell<HashSet<u64>> = RefCell::new(HashSet::new());
    /// The ids of the elements the plan being drawn leaves out.
    static HIDDEN: RefCell<HashSet<u64>> = RefCell::new(HashSet::new());
}

/// Sets which elements of `model` are greyed for `focus`, for the drawing
/// that follows; returns a guard that clears them.
pub(crate) fn begin(model: &Model, style: &ViewStyle) -> Guard {
    let focus = style.focus;
    let ids = || {
        model
            .walls()
            .map(|w| w.id.0)
            .chain(model.slabs().map(|s| s.id.0))
            .chain(model.openings().map(|o| o.id.0))
            .chain(model.columns().map(|c| c.id.0))
            .chain(model.beams().map(|b| b.id.0))
            .chain(model.roofs().map(|r| r.id.0))
            .chain(model.stairs().map(|s| s.id.0))
            .chain(model.terrains().map(|t| t.id.0))
            .chain(model.curtain_walls().map(|c| c.id.0))
            .chain(model.railings().map(|r| r.id.0))
            .chain(model.objects().map(|o| o.id.0))
            .chain(model.footings().map(|f| f.id.0))
            .chain(model.plots().map(|p| p.id.0))
            .chain(model.roads().map(|r| r.id.0))
    };
    DIMMED.with(|d| {
        let mut d = d.borrow_mut();
        d.clear();
        if focus != DisciplineFocus::None {
            d.extend(ids().filter(|id| model.dimmed(focus, *id)));
        }
    });
    HIDDEN.with(|h| {
        let mut h = h.borrow_mut();
        h.clear();
        if !style.hidden_disciplines.is_empty() {
            h.extend(ids().filter(|id| model.hidden_by_discipline(&style.hidden_disciplines, *id)));
        }
    });
    Guard
}

pub(crate) struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        DIMMED.with(|d| d.borrow_mut().clear());
        HIDDEN.with(|h| h.borrow_mut().clear());
    }
}

pub(crate) fn is_dimmed(id: u64) -> bool {
    DIMMED.with(|d| d.borrow().contains(&id))
}

pub(crate) fn is_hidden(id: u64) -> bool {
    HIDDEN.with(|h| h.borrow().contains(&id))
}

/// Draws one element with `draw`: in grey if it is greyed, not at all if
/// its discipline is hidden.
pub(crate) fn element(s: &mut String, id: u64, draw: impl FnOnce(&mut String)) {
    if is_hidden(id) {
        return;
    }
    if is_dimmed(id) {
        let mut t = String::new();
        draw(&mut t);
        s.push_str(&grey(&t));
    } else {
        draw(s);
    }
}

/// A colour as a light grey of its lightness.
fn grey_of(r: u8, g: u8, b: u8) -> u8 {
    let lum = 0.3 * r as f64 + 0.59 * g as f64 + 0.11 * b as f64;
    // Towards white: dark lines become mid grey, fills very light.
    (255.0 - (255.0 - lum) * 0.38).round().clamp(0.0, 255.0) as u8
}

/// Every `#rrggbb`, `#rgb` and `rgb(r,g,b)` of an SVG fragment, in grey.
pub fn grey(svg: &str) -> String {
    let b = svg.as_bytes();
    let mut out = String::with_capacity(svg.len());
    let mut i = 0;
    let hex = |c: u8| (c as char).to_digit(16).map(|v| v as u8);
    while i < b.len() {
        if b[i] == b'#' {
            let six = (1..=6).all(|k| b.get(i + k).and_then(|c| hex(*c)).is_some());
            let after = |n: usize| b.get(i + n).is_none_or(|c| !c.is_ascii_alphanumeric());
            if six && after(7) {
                let v = |k: usize| hex(b[i + k]).unwrap() * 16 + hex(b[i + k + 1]).unwrap();
                let g = grey_of(v(1), v(3), v(5));
                out.push_str(&format!("#{g:02x}{g:02x}{g:02x}"));
                i += 7;
                continue;
            }
            let three = (1..=3).all(|k| b.get(i + k).and_then(|c| hex(*c)).is_some());
            if three && after(4) {
                let v = |k: usize| hex(b[i + k]).unwrap() * 17;
                let g = grey_of(v(1), v(2), v(3));
                out.push_str(&format!("#{g:02x}{g:02x}{g:02x}"));
                i += 4;
                continue;
            }
        }
        if svg[i..].starts_with("rgb(") {
            if let Some(end) = svg[i..].find(')') {
                let inside = &svg[i + 4..i + end];
                let parts: Vec<u8> = inside
                    .split(',')
                    .filter_map(|p| p.trim().parse::<f64>().ok())
                    .map(|v| v.clamp(0.0, 255.0) as u8)
                    .collect();
                if parts.len() == 3 {
                    let g = grey_of(parts[0], parts[1], parts[2]);
                    out.push_str(&format!("rgb({g},{g},{g})"));
                    i += end + 1;
                    continue;
                }
            }
        }
        if svg[i..].starts_with("\"black\"") {
            let g = grey_of(0, 0, 0);
            out.push_str(&format!("\"#{g:02x}{g:02x}{g:02x}\""));
            i += 7;
            continue;
        }
        let ch = svg[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_turn_grey_and_ids_stay() {
        let s = grey(
            r##"<path id="cut-beam-12" fill="#ff0000" stroke="#1a1a1a"/><rect fill="rgb(200,100,50)" stroke="#fff"/>"##,
        );
        assert!(s.contains("id=\"cut-beam-12\""));
        assert!(!s.contains("#ff0000"));
        assert!(!s.contains("#1a1a1a"));
        assert!(s.contains("stroke=\"#ffffff\""));
        assert!(!s.contains("rgb(200,100,50)"));
        // Every colour left is a grey.
        for part in s.split('#').skip(1) {
            let h = &part[..6];
            assert_eq!(&h[0..2], &h[2..4]);
            assert_eq!(&h[2..4], &h[4..6]);
        }
        // Ink becomes a mid grey: still visible on white.
        let ink = grey_of(0x1a, 0x1a, 0x1a);
        assert!(ink > 120 && ink < 190, "{ink}");
    }

    fn two_walls() -> (Model, archisim_core::LevelId) {
        use archisim_core::{
            CutPattern, Layer, LayerFunction, NewWall, Point2, ReferenceLine, WallFunction, WallTop,
        };
        let mut m = Model::new("Disciplinas");
        let l = m.add_level("PB", 0.0);
        for (i, (cat, rgb)) in [("brick", [230, 120, 90]), ("concrete", [40, 160, 220])]
            .into_iter()
            .enumerate()
        {
            let mat = m.add_material(cat, cat, rgb, CutPattern::Solid);
            let t = m
                .add_wall_type(
                    format!("Muro {cat}"),
                    WallFunction::Exterior,
                    vec![Layer {
                        material: mat,
                        thickness: 0.25,
                        function: LayerFunction::Structure,
                    }],
                )
                .unwrap();
            let y = i as f64 * 4.0;
            m.add_wall(NewWall {
                wall_type: t,
                level: l,
                start: Point2::new(0.0, y),
                end: Point2::new(5.0, y),
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        }
        (m, l)
    }

    /// A structural plan: the brick wall in grey, the concrete one in its
    /// colour; and a discipline hidden leaves its walls out.
    #[test]
    fn plans_grey_and_hide_by_discipline() {
        use archisim_core::ElementDiscipline;
        let (m, l) = two_walls();
        let mut o = crate::PlanOptions::default();
        let plain = crate::render_plan(&m, l, &o);
        assert!(plain.contains("rgb(230,120,90)") && plain.contains("rgb(40,160,220)"));
        o.style.focus = DisciplineFocus::Only(ElementDiscipline::Structure);
        let structural = crate::render_plan(&m, l, &o);
        assert!(
            !structural.contains("rgb(230,120,90)"),
            "the brick wall in grey"
        );
        assert!(
            structural.contains("rgb(40,160,220)"),
            "the concrete wall in its colour"
        );
        o.style.focus = DisciplineFocus::None;
        o.style
            .hidden_disciplines
            .insert(ElementDiscipline::Structure);
        let without = crate::render_plan(&m, l, &o);
        assert!(without.contains("rgb(230,120,90)"));
        assert!(!without.contains("rgb(40,160,220)"));
        // Nothing left behind for the next drawing.
        assert!(!is_dimmed(m.walls().next().unwrap().id.0));
    }
}
