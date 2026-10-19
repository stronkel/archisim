//! Columns and beams in plan.
//!
//! - A column crossed by the cut plane is drawn cut, on top of the walls
//!   (the concrete goes through the brickwork): filled with its material
//!   and outlined with the heavy cut line. Columns of other levels that
//!   reach through the cut plane are drawn the same way. A column entirely
//!   below the cut plane is drawn with the projection line; one entirely
//!   above it, dashed.
//! - Beams of this level and of the level above (the beams of the ceiling,
//!   as usual in Spanish plans) are drawn dashed when above the cut plane,
//!   and cut when it crosses them. Beams of this level below the cut plane
//!   (under the floor) are drawn dashed too.

use std::fmt::Write as _;

use archisim_core::{Beam, Column, LevelId, Model, Point2};

use crate::{fill_cut, num, path, CUT_LINE, INK, VIEW_LINE};

/// How a column or beam meets the cut plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberView {
    Cut,
    Below,
    Above,
}

fn view(bottom: f64, top: f64, cut: f64) -> MemberView {
    if top <= cut {
        MemberView::Below
    } else if bottom < cut {
        MemberView::Cut
    } else {
        MemberView::Above
    }
}

/// Segments used for round columns in plan.
const ROUND: usize = 48;

/// The columns a plan of `level` shows, with how each is seen: those of the
/// level, and those of other levels that its cut plane crosses.
pub fn plan_columns(model: &Model, level: LevelId, cut_plane: f64) -> Vec<(&Column, MemberView)> {
    let Some(elev) = model.level(level).map(|l| l.elevation) else {
        return Vec::new();
    };
    let cut = elev + cut_plane;
    model
        .columns()
        .filter_map(|c| {
            let base = model.level(c.level)?.elevation + c.base_offset;
            let v = view(base, base + c.height, cut);
            (c.level == level || v == MemberView::Cut).then_some((c, v))
        })
        .collect()
}

/// The beams a plan of `level` shows: those of the level and of the level
/// above, with how each is seen.
pub fn plan_beams(model: &Model, level: LevelId, cut_plane: f64) -> Vec<(&Beam, MemberView)> {
    let Some(elev) = model.level(level).map(|l| l.elevation) else {
        return Vec::new();
    };
    let above = model.level_above(level);
    let cut = elev + cut_plane;
    model
        .beams()
        .filter(|b| b.level == level || Some(b.level) == above)
        .filter_map(|b| {
            let t = model.member_type(b.member_type)?;
            let top = model.level(b.level)?.elevation + b.top_offset;
            let v = view(top - t.profile.size().1, top, cut);
            Some((b, v))
        })
        .collect()
}

/// A column's outline in plan.
pub fn column_outline(model: &Model, c: &Column) -> Vec<Point2> {
    model
        .member_type(c.member_type)
        .map(|t| c.plan_outline(t.profile, ROUND))
        .unwrap_or_default()
}

/// A beam's outline in plan: a band as wide as its section along its axis.
pub fn beam_outline(model: &Model, b: &Beam) -> Vec<Point2> {
    let Some(t) = model.member_type(b.member_type) else {
        return Vec::new();
    };
    let Some(u) = (b.end - b.start).normalized() else {
        return Vec::new();
    };
    let n = u.perp_left() * (t.profile.size().0 / 2.0);
    vec![b.start - n, b.end - n, b.end + n, b.start + n]
}

/// Points the plan must frame.
pub(crate) fn member_points(model: &Model, level: LevelId, cut_plane: f64) -> Vec<Point2> {
    let mut v: Vec<Point2> = plan_columns(model, level, cut_plane)
        .iter()
        .flat_map(|(c, _)| column_outline(model, c))
        .collect();
    v.extend(
        plan_beams(model, level, cut_plane)
            .iter()
            .flat_map(|(b, _)| beam_outline(model, b)),
    );
    v
}

fn outline(s: &mut String, poly: &[Point2], weight: f64, dashed: bool, k: f64) {
    let dash = if dashed {
        format!(r#" stroke-dasharray="{} {}""#, num(3.0 * k), num(1.5 * k))
    } else {
        String::new()
    };
    let _ = writeln!(
        s,
        r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-linejoin="miter"{dash}/>"#,
        path(poly),
        num(weight * k)
    );
}

fn draw_cut(
    s: &mut String,
    model: &Model,
    material: archisim_core::MaterialId,
    poly: &[Point2],
    id: &str,
    k: f64,
) {
    if let Some(m) = model.material(material) {
        fill_cut(s, m, poly, id, k);
    }
    outline(s, poly, CUT_LINE, false, k);
}

/// Draws the beams (under) and columns (over) of a plan.
pub(crate) fn draw_members(
    s: &mut String,
    model: &Model,
    level: LevelId,
    cut_plane: f64,
    k: f64,
    visible: &dyn Fn(&[Point2]) -> bool,
) {
    s.push_str("<g id=\"structure\">\n");
    for (b, v) in plan_beams(model, level, cut_plane) {
        let poly = beam_outline(model, b);
        if poly.is_empty() || !visible(&poly) {
            continue;
        }
        match v {
            MemberView::Cut => {
                let mat = model.member_type(b.member_type).map(|t| t.material);
                if let Some(mat) = mat {
                    draw_cut(s, model, mat, &poly, &format!("cut-beam-{}", b.id.0), k);
                }
            }
            MemberView::Above | MemberView::Below => outline(s, &poly, VIEW_LINE, true, k),
        }
    }
    for (c, v) in plan_columns(model, level, cut_plane) {
        let poly = column_outline(model, c);
        if poly.is_empty() || !visible(&poly) {
            continue;
        }
        match v {
            MemberView::Cut => {
                let mat = model.member_type(c.member_type).map(|t| t.material);
                if let Some(mat) = mat {
                    draw_cut(s, model, mat, &poly, &format!("cut-column-{}", c.id.0), k);
                }
                // A tube: its hole, empty, with its cut line.
                let hole = model
                    .member_type(c.member_type)
                    .and_then(|t| c.plan_hole(t.profile, ROUND));
                if let Some(hole) = hole {
                    let _ = writeln!(
                        s,
                        r#"<path d="{}" fill="white" stroke="{INK}" stroke-width="{}"/>"#,
                        path(&hole),
                        num(CUT_LINE * k)
                    );
                }
            }
            MemberView::Below => outline(s, &poly, VIEW_LINE, false, k),
            MemberView::Above => outline(s, &poly, VIEW_LINE, true, k),
        }
    }
    s.push_str("</g>\n");
}
