//! Modular boxes (M1) in plan: the outline of each box in a dash and dot
//! of their own colour over the walls, its mark in a tag at its middle
//! («MOD-03», and under it what box of the system it is) and, inside each
//! side, its letter and what it is when it is joined to another box («C ·
//! muro», «A · abierto»); the posts of its chassis, cut, and a ring with a
//! cross at each point it is lifted from (M3).

use std::fmt::Write as _;

use archisim_core::{LevelId, Model, ModularBox, Point2, SideState, SIDE_NAMES};

use crate::lettering::{text_path, text_width, Align};
use crate::{num, path, pens, pt};

/// The colour of the boxes' lines and tags: they are not built, they say
/// how the building is split.
pub const BOX_INK: &str = "#2a64b4";
/// Height of the mark, mm of paper.
pub const MARK: f64 = 2.5;
/// Height of the kind of box and of the sides' letters.
const SMALL: f64 = 1.6;

/// The tag of a box's mark: a rectangle round it at the middle of the box,
/// in metres. The app picks the box by it.
pub fn box_tag(b: &ModularBox, k: f64) -> Vec<Point2> {
    let h = MARK * k;
    let mut w = text_width(&b.mark, h);
    if !b.kind.is_empty() {
        w = w.max(text_width(&b.kind, SMALL * k));
    }
    let lines = if b.kind.is_empty() { 1.0 } else { 1.8 };
    let (hx, hy) = (w / 2.0 + 1.2 * k, h * lines / 2.0 + 1.0 * k);
    let c = b.centre();
    let (u, v) = (Point2::new(1.0, 0.0), Point2::new(0.0, 1.0));
    vec![
        c - u * hx - v * hy,
        c + u * hx - v * hy,
        c + u * hx + v * hy,
        c - u * hx + v * hy,
    ]
}

/// The boxes of a level.
pub fn draw_boxes(s: &mut String, model: &Model, level: LevelId, k: f64) {
    for b in model.boxes_on_level(level) {
        crate::dim::element(s, b.id.0, |s| draw_box(s, b, k));
    }
}

fn draw_box(s: &mut String, b: &ModularBox, k: f64) {
    let w = num(pens().cut * k);
    let dash = format!(
        "{} {} {} {}",
        num(6.0 * k),
        num(1.5 * k),
        num(0.6 * k),
        num(1.5 * k)
    );
    // The sides: free ones in dash and dot, joined ones in a finer dash.
    for (i, name) in SIDE_NAMES.iter().enumerate() {
        let (a, e) = b.side(i);
        let (stroke_w, dashes) = match b.sides[i] {
            SideState::Free => (w.clone(), dash.clone()),
            _ => (
                num(pens().view * k),
                format!("{} {}", num(2.0 * k), num(1.0 * k)),
            ),
        };
        let _ = writeln!(
            s,
            r#"<path class="box-side" d="M{} L{}" fill="none" stroke="{BOX_INK}" stroke-width="{stroke_w}" stroke-dasharray="{dashes}"/>"#,
            pt(a),
            pt(e)
        );
        // Its letter, inside, along it.
        let dir = (e - a).normalized().unwrap_or(Point2::new(1.0, 0.0));
        let inward = dir.perp_left();
        let h = SMALL * k;
        let text = match b.sides[i] {
            SideState::Free => name.to_string(),
            SideState::Wall => format!("{name} · muro"),
            SideState::Open => format!("{name} · abierto"),
        };
        // Inside the wall that may stand on it, read from below or from
        // the right, as dimensions.
        let off = archisim_core::SIDE_BAND * 0.7 + 1.0 * k;
        // A quarter along, clear of the tag in the middle.
        let at = a + (e - a) * 0.25;
        let (dir, at) = if dir.x < -1e-9 || (dir.x.abs() <= 1e-9 && dir.y < 0.0) {
            (-dir, at + inward * (off + h))
        } else {
            (dir, at + inward * off)
        };
        let _ = writeln!(
            s,
            r#"<path d="{}" {}/>"#,
            text_path(&text, at, dir, h, Align::Centre),
            crate::lettering::paint(h, BOX_INK)
        );
    }
    // The chassis' posts, cut (M3), and where it is lifted from.
    for m in archisim_geom::boxes::chassis_members(b)
        .iter()
        .filter(|m| m.role.is_post())
    {
        let corners: Vec<Point2> = [
            [m.lo[0], m.lo[1]],
            [m.hi[0], m.lo[1]],
            [m.hi[0], m.hi[1]],
            [m.lo[0], m.hi[1]],
        ]
        .iter()
        .map(|p| archisim_geom::boxes::box_point(b, [p[0], p[1], 0.0]))
        .collect();
        let _ = writeln!(
            s,
            r#"<path class="box-post" d="{}" fill="{BOX_INK}" stroke="none"/>"#,
            path(&corners)
        );
    }
    for p in archisim_geom::boxes::lift_points(b) {
        // A small ring with a cross, inside the corner.
        let c = archisim_geom::boxes::box_point(b, p);
        let r = 1.2 * k;
        let inward = (b.centre() - c)
            .normalized()
            .unwrap_or(Point2::new(0.0, 1.0));
        let c = c + inward * (r * 2.5);
        let _ = writeln!(
            s,
            r#"<circle class="box-lift" cx="{}" cy="{}" r="{}" fill="none" stroke="{BOX_INK}" stroke-width="{}"/>"#,
            num(c.x),
            num(-c.y),
            num(r),
            num(pens().view * k)
        );
        let _ = writeln!(
            s,
            r#"<path d="M{} L{} M{} L{}" stroke="{BOX_INK}" stroke-width="{}"/>"#,
            pt(c - Point2::new(r, 0.0)),
            pt(c + Point2::new(r, 0.0)),
            pt(c - Point2::new(0.0, r)),
            pt(c + Point2::new(0.0, r)),
            num(pens().view * k)
        );
    }
    // The tag.
    let tag = box_tag(b, k);
    let _ = writeln!(
        s,
        r#"<path class="box-tag" d="{}" fill="white" stroke="{BOX_INK}" stroke-width="{}"/>"#,
        path(&tag),
        num(pens().view * k)
    );
    let c = b.centre();
    let h = MARK * k;
    let right = Point2::new(1.0, 0.0);
    if b.kind.is_empty() {
        let _ = writeln!(
            s,
            r#"<path d="{}" {}/>"#,
            text_path(
                &b.mark,
                c - Point2::new(0.0, h / 2.0),
                right,
                h,
                Align::Centre
            ),
            crate::lettering::paint(h, BOX_INK)
        );
    } else {
        let small = SMALL * k;
        let _ = writeln!(
            s,
            r#"<path d="{}" {}/>"#,
            text_path(
                &b.mark,
                c + Point2::new(0.0, small * 0.25),
                right,
                h,
                Align::Centre
            ),
            crate::lettering::paint(h, BOX_INK)
        );
        let _ = writeln!(
            s,
            r#"<path d="{}" {}/>"#,
            text_path(
                &b.kind,
                c - Point2::new(0.0, small * 1.25),
                right,
                small,
                Align::Centre
            ),
            crate::lettering::paint(small, BOX_INK)
        );
    }
    // Where it is (M9), under the tag, in the stage's colour.
    if b.progress.stage != archisim_core::BoxStage::Design {
        let small = SMALL * k;
        let low = tag.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let mut t = b.progress.stage.short().to_string();
        if b.progress.stage == archisim_core::BoxStage::Factory && !b.progress.station.is_empty() {
            t.push_str(&format!(" · {}", b.progress.station));
        }
        let [r, g, bl] = b.progress.stage.rgb();
        let colour = format!("#{r:02x}{g:02x}{bl:02x}");
        let _ = writeln!(
            s,
            r#"<path class="box-stage" d="{}" {}/>"#,
            text_path(
                &t,
                Point2::new(c.x, low - small * 1.4),
                right,
                small,
                Align::Centre
            ),
            crate::lettering::paint(small, &colour)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::NewBox;

    #[test]
    fn a_box_draws_its_sides_and_mark() {
        let mut m = Model::new("P");
        let lv = m.add_level("PB", 0.0);
        let a = m
            .add_box(NewBox::usual(lv, Point2::new(0.0, 0.0), 0.0))
            .unwrap();
        m.edit_box(a, |b| {
            b.sides[2] = SideState::Wall;
            b.kind = "Caja 6,20 · dormitorio".into();
        })
        .unwrap();
        let mut s = String::new();
        draw_boxes(&mut s, &m, lv, 0.05);
        assert_eq!(s.matches("box-side").count(), 4);
        assert_eq!(s.matches("box-tag").count(), 1);
        let tag = box_tag(m.modular_box(a).unwrap(), 0.05);
        assert!(archisim_core::point_in_polygon(
            m.modular_box(a).unwrap().centre(),
            &tag
        ));
        assert!(!s.contains("box-stage"));
        m.edit_box(a, |b| {
            b.progress.stage = archisim_core::BoxStage::Factory;
            b.progress.station = "Puesto 2".into();
        })
        .unwrap();
        let mut s = String::new();
        draw_boxes(&mut s, &m, lv, 0.05);
        assert!(s.contains("box-stage"));
        // Its posts are drawn, cut (and the parapet's, but on C, the joint),
        // and where it is lifted from.
        assert_eq!(s.matches("box-post").count(), 4 + 8);
        assert_eq!(s.matches("box-lift").count(), 4);
    }
}
