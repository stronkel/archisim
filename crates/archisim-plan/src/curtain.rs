//! Curtain walls in plan: where the cut plane goes through them, their
//! mullions cut (filled as their material) and the glass between them as a
//! thin rectangle; above or below the cut, their outline seen (dashed if
//! above).

use std::fmt::Write as _;

use archisim_core::{CurtainPieceKind, CurtainWall, Model, Point2};

use crate::{fill_cut, num, path, CUT_LINE, INK, LAYER_LINE, VIEW_LINE};

/// Every point of the curtain wall the plan must show.
pub fn curtain_points(model: &Model, c: &CurtainWall) -> Vec<Point2> {
    model
        .curtain_design(c.id)
        .map(|d| c.outline(d))
        .unwrap_or_default()
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
    let Some(d) = model.curtain_design(c.id) else {
        return;
    };
    let _ = writeln!(s, r#"<g class="curtain-wall">"#);
    let (z0, z1) = (c.base_offset, c.base_offset + c.height);
    let pieces = c.pieces(d);
    // Louvres below the cut are seen from above.
    let cut = cut_plane - c.base_offset;
    if let Some(l) = pieces
        .iter()
        .find(|p| p.kind == CurtainPieceKind::Louvre && p.up.1 <= cut)
    {
        outline(s, &c.piece_footprint(l), VIEW_LINE, false, k);
    }
    if cut_plane > z0 && cut_plane < z1 {
        // What the cut goes through; where it falls on a transom, the
        // panels of the row under it stand for their row.
        let row = pieces
            .iter()
            .filter(|p| matches!(p.kind, CurtainPieceKind::Panel(_)))
            .filter(|p| p.up.0 <= cut)
            .map(|p| p.up.0)
            .fold(f64::NEG_INFINITY, f64::max);
        for (i, p) in pieces.iter().enumerate() {
            let hit = match p.kind {
                CurtainPieceKind::Panel(_) => (p.up.0 - row).abs() < 1e-9,
                CurtainPieceKind::Louvre => false,
                _ => p.is_vertical() && p.up.0 <= cut && p.up.1 >= cut,
            };
            if !hit {
                continue;
            }
            let poly = c.piece_footprint(p);
            let material = model.material(p.material);
            let glass = material.is_some_and(|m| m.category == "glass");
            if let (Some(m), false) = (material, glass) {
                fill_cut(s, m, &poly, &format!("cw{}-{i}", c.id.0), k);
            }
            let weight = if matches!(p.kind, CurtainPieceKind::Panel(_)) {
                LAYER_LINE
            } else {
                CUT_LINE
            };
            outline(s, &poly, weight, false, k);
        }
    } else {
        outline(s, &c.outline(d), VIEW_LINE, cut_plane <= z0, k);
    }
    s.push_str("</g>\n");
}
