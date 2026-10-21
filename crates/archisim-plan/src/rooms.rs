//! Rooms and tags in plan: the stamp of each room (name, number, area)
//! and the tags of doors, windows and walls, written from the elements
//! themselves.

use std::fmt::Write;

use archisim_core::{Model, Point2, Room, Tag, TagTarget, Text, TextAlign};
use archisim_draft::font::{text_frame, text_strokes, STROKE};

use crate::{num, pens, pt};

const INK: &str = "#1a1a1a";

/// An area as rooms write it: `24,50 m²`.
pub fn format_area(a: f64) -> String {
    format!("{:.2} m²", (a * 100.0).round() / 100.0).replace('.', ",")
}

/// The stamp of a room: name, number and area, centred on its label.
pub fn room_stamp(model: &Model, r: &Room, k: f64) -> Vec<Text> {
    let h = pens().text;
    let small = h * 0.8;
    let mut lines: Vec<(String, f64)> = Vec::new();
    if !r.name.trim().is_empty() {
        lines.push((r.name.to_uppercase(), h));
    }
    if !r.number.trim().is_empty() {
        lines.push((r.number.clone(), small));
    }
    lines.push((format_area(r.area()), small));
    let _ = model;
    // Stacked from the top, centred on the label.
    let gap = 0.6 * h;
    let total: f64 = lines.iter().map(|l| l.1).sum::<f64>() + gap * (lines.len() - 1) as f64;
    let mut y = r.label.y + total / 2.0 * k;
    lines
        .into_iter()
        .map(|(content, height)| {
            let t = Text {
                at: Point2::new(r.label.x, y),
                content,
                height,
                angle: 0.0,
                align: TextAlign::TopCentre,
            };
            y -= (height + gap) * k;
            t
        })
        .collect()
}

/// What a tag says: a door's or window's mark (its type name if it has
/// none) and its size; a wall's type; a room's name and area.
pub fn tag_lines(model: &Model, t: &Tag) -> Vec<String> {
    match t.target {
        TagTarget::Opening(o) => {
            let Some(ty) = model
                .opening(o)
                .and_then(|o| model.opening_type(o.opening_type))
            else {
                return Vec::new();
            };
            let mark = if ty.mark.trim().is_empty() {
                ty.name.clone()
            } else {
                ty.mark.clone()
            };
            let size = format!(
                "{}×{}",
                crate::annotate::format_dimension(ty.width),
                crate::annotate::format_dimension(ty.height)
            );
            vec![mark, size]
        }
        TagTarget::Wall(w) => model
            .wall(w)
            .and_then(|w| model.wall_type(w.wall_type))
            .map(|t| vec![t.name.clone()])
            .unwrap_or_default(),
        TagTarget::Room(r) => model
            .room(r)
            .map(|r| vec![r.name.clone(), format_area(r.area())])
            .unwrap_or_default(),
    }
}

/// The point of an element a tag's leader goes to: a door's or window's
/// middle on its wall, a wall's middle, a room's label.
pub fn tag_anchor(model: &Model, target: TagTarget) -> Option<Point2> {
    match target {
        TagTarget::Opening(o) => {
            let g = archisim_geom::opening_geometry(model, o)?;
            Some(g.point((g.t0 + g.t1) / 2.0, 0.0))
        }
        TagTarget::Wall(w) => model.wall(w).map(|w| w.midpoint()),
        TagTarget::Room(r) => model.room(r).map(|r| r.label),
    }
}

/// A tag laid out: its texts, the frame around its first line (doors and
/// windows) and its leader.
#[derive(Debug, Clone, PartialEq)]
pub struct TagLayout {
    pub texts: Vec<Text>,
    pub frame: Option<[Point2; 4]>,
    pub leader: Option<(Point2, Point2)>,
}

pub fn tag_layout(model: &Model, t: &Tag, k: f64) -> TagLayout {
    let lines = tag_lines(model, t);
    let h = pens().text;
    let small = h * 0.8;
    let mut texts = Vec::new();
    let mut frame = None;
    let gap = 0.6 * k;
    // The lines stacked about the tag's point: the mark, the size below.
    let heights: Vec<f64> = (0..lines.len())
        .map(|i| if i == 0 { h } else { small })
        .collect();
    let total: f64 = heights.iter().sum::<f64>() * k + gap * (lines.len().max(1) - 1) as f64;
    let mut y = t.at.y + total / 2.0;
    for (line, height) in lines.iter().zip(&heights) {
        texts.push(Text {
            at: Point2::new(t.at.x, y),
            content: line.clone(),
            height: *height,
            angle: 0.0,
            align: TextAlign::TopCentre,
        });
        y -= height * k + gap;
    }
    if matches!(t.target, TagTarget::Opening(_)) && !texts.is_empty() {
        // A box around the lines, 1 mm clear.
        let pad = 1.0 * k;
        let (mut lo, mut hi) = (
            Point2::new(f64::INFINITY, f64::INFINITY),
            Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        );
        for tx in &texts {
            for p in text_frame(tx, k) {
                lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
                hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
            }
        }
        frame = Some([
            Point2::new(lo.x - pad, lo.y - pad),
            Point2::new(hi.x + pad, lo.y - pad),
            Point2::new(hi.x + pad, hi.y + pad),
            Point2::new(lo.x - pad, hi.y + pad),
        ]);
    }
    let leader = if t.leader {
        tag_anchor(model, t.target).map(|a| {
            // From the edge of the frame (or the text) towards the element.
            let from = match frame {
                Some(f) => edge_towards(&f, t.at, a),
                None => t.at,
            };
            (from, a)
        })
    } else {
        None
    };
    TagLayout {
        texts,
        frame,
        leader,
    }
}

/// Where the line from the centre of a box to `to` leaves it.
fn edge_towards(f: &[Point2; 4], c: Point2, to: Point2) -> Point2 {
    let d = to - c;
    let (hw, hh) = ((f[1].x - f[0].x) / 2.0, (f[2].y - f[1].y) / 2.0);
    let tx = if d.x.abs() > 1e-12 {
        hw / d.x.abs()
    } else {
        f64::INFINITY
    };
    let ty = if d.y.abs() > 1e-12 {
        hh / d.y.abs()
    } else {
        f64::INFINITY
    };
    let t = tx.min(ty).min(1.0);
    c + d * t
}

fn text_path(s: &mut String, t: &Text, k: f64) {
    let mut d = String::new();
    for st in text_strokes(t, k) {
        for (i, p) in st.iter().enumerate() {
            let _ = write!(d, "{}{} ", if i == 0 { "M" } else { "L" }, pt(*p));
        }
    }
    let _ = writeln!(
        s,
        r#"<path d="{}" stroke="{INK}" stroke-width="{}" stroke-linecap="round" fill="none"/>"#,
        d.trim_end(),
        num(STROKE * t.height * k)
    );
}

/// The stamps of the rooms of a level.
pub fn draw_rooms(s: &mut String, model: &Model, level: archisim_core::LevelId, k: f64) {
    let rooms: Vec<&Room> = model.rooms_on_level(level).collect();
    if rooms.is_empty() {
        return;
    }
    s.push_str("<g id=\"rooms\">\n");
    for r in rooms {
        for t in room_stamp(model, r, k) {
            text_path(s, &t, k);
        }
    }
    s.push_str("</g>\n");
}

/// The floors of the rooms of a level that have a floor fill: under
/// everything cut, like a slab.
pub fn draw_floor_fills(s: &mut String, model: &Model, level: archisim_core::LevelId, k: f64) {
    let rooms: Vec<&Room> = model
        .rooms_on_level(level)
        .filter(|r| r.floor_fill.is_some())
        .collect();
    if rooms.is_empty() {
        return;
    }
    s.push_str("<g id=\"floor-fills\" fill=\"none\">\n");
    let width = num(pens().hatch * k);
    for r in rooms {
        let Some(f) = r.floor_fill else { continue };
        let ring = |p: &[Point2]| archisim_core::Polyline::straight(p.to_vec(), true);
        let h = archisim_core::Hatch {
            loops: std::iter::once(ring(&r.outline))
                .chain(r.holes.iter().map(|h| ring(h)))
                .collect(),
            pattern: f.pattern,
            scale: f.scale,
            angle: f.angle,
            background: f.background,
            opacity: 1.0,
        };
        let color = format!("#{:02x}{:02x}{:02x}", f.color[0], f.color[1], f.color[2]);
        crate::drafting::draw_hatch(s, &h, &color, &width, k);
    }
    s.push_str("</g>\n");
}

/// The tags of a level.
pub fn draw_tags(s: &mut String, model: &Model, level: archisim_core::LevelId, k: f64) {
    let tags: Vec<&Tag> = model.tags_on_level(level).collect();
    if tags.is_empty() {
        return;
    }
    let w = num(pens().dimension * k);
    s.push_str("<g id=\"tags\">\n");
    for t in tags {
        let l = tag_layout(model, t, k);
        if let Some(f) = l.frame {
            let _ = writeln!(
                s,
                r#"<path d="M{} L{} L{} L{} Z" fill="white" stroke="{INK}" stroke-width="{w}"/>"#,
                pt(f[0]),
                pt(f[1]),
                pt(f[2]),
                pt(f[3])
            );
        }
        if let Some((a, b)) = l.leader {
            let _ = writeln!(
                s,
                r#"<path d="M{} L{}" stroke="{INK}" stroke-width="{w}"/>"#,
                pt(a),
                pt(b)
            );
            let r = 0.4 * k;
            let _ = writeln!(
                s,
                r#"<circle cx="{}" cy="{}" r="{}" fill="{INK}"/>"#,
                num(b.x),
                num(-b.y),
                num(r)
            );
        }
        for tx in &l.texts {
            text_path(s, tx, k);
        }
    }
    s.push_str("</g>\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn areas_as_written() {
        assert_eq!(format_area(24.504), "24,50 m²");
        assert_eq!(format_area(3.0), "3,00 m²");
    }
}
