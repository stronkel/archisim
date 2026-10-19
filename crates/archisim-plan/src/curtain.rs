//! Curtain walls in plan: where the cut plane goes through them, their
//! mullions cut (filled as their material) and the glass between them as a
//! thin rectangle; above or below the cut, their outline seen (dashed if
//! above).

use std::fmt::Write as _;

use archisim_core::{CurtainPieceKind, CurtainWall, Model, Point2};

use crate::{fill_cut, num, path, CUT_LINE, INK, LAYER_LINE, VIEW_LINE};

/// Every point of the curtain wall the plan must show.
pub fn curtain_points(c: &CurtainWall) -> Vec<Point2> {
    c.outline()
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

pub fn draw_curtain_wall(s: &mut String, model: &Model, c: &CurtainWall, cut_plane: f64, k: f64) {
    let _ = writeln!(s, r#"<g class="curtain-wall">"#);
    let (z0, z1) = (c.base_offset, c.base_offset + c.height);
    if cut_plane > z0 && cut_plane < z1 {
        let pieces = c.pieces();
        // Panels of the first row stand for all of them.
        let first_row = pieces
            .iter()
            .filter(|p| p.kind == CurtainPieceKind::Panel)
            .map(|p| p.up.0)
            .fold(f64::INFINITY, f64::min);
        for p in pieces
            .iter()
            .filter(|p| p.kind == CurtainPieceKind::Panel && (p.up.0 - first_row).abs() < 1e-9)
        {
            outline(s, &c.piece_footprint(p), LAYER_LINE, false, k);
        }
        for (i, p) in pieces
            .iter()
            .filter(|p| p.kind == CurtainPieceKind::Mullion)
            .enumerate()
        {
            let poly = c.piece_footprint(p);
            if let Some(m) = model.material(c.frame_material) {
                fill_cut(s, m, &poly, &format!("cw{}-{i}", c.id.0), k);
            }
            outline(s, &poly, CUT_LINE, false, k);
        }
    } else {
        outline(s, &c.outline(), VIEW_LINE, cut_plane <= z0, k);
    }
    s.push_str("</g>\n");
}
