//! The drawings of the faces composed by hand (M20, `docs/MODULAR.md`):
//! one A3 landscape page (an SVG document in millimetres) per wall layer
//! composed by hand, with its face seen from outside, every piece with its
//! mark, the doors and windows, the chains of joints along the bottom and
//! up the side, and a table of the pieces (mark, material, size, factory
//! or site). Codes «D-01», «D-02»…

use std::collections::BTreeMap;
use std::fmt::Write as _;

use archisim_core::Model;
use archisim_geom::boards::{wall_boards, BoardPiece};

use crate::structure_sheets::{
    centred, line, page, rect, table, text, BLOCK_H, BLOCK_W, GREY, H, INK, MARGIN, W,
};

/// The scales a face may be drawn at, the largest that fits.
const SCALES: [f64; 5] = [20.0, 25.0, 50.0, 75.0, 100.0];
/// Fill of the pieces fitted on site.
const ON_SITE: &str = "#f3c79a";

fn n(v: f64) -> String {
    crate::structure_sheets::n(v)
}

fn cm(v: f64) -> String {
    format!("{:.0}", v * 100.0)
}

/// The faces composed by hand, as (wall, layer), in the order of their
/// walls.
pub fn faces(model: &Model) -> Vec<(archisim_core::WallId, usize)> {
    let mut v: Vec<_> = model.board_faces().map(|f| (f.wall, f.layer)).collect();
    v.sort_by_key(|(w, l)| (w.0, *l));
    v
}

/// The pages: one per face composed by hand. Empty without any.
pub fn pages(model: &Model) -> Vec<String> {
    let _font = crate::lettering::use_font(model.graphics.font);
    faces(model)
        .into_iter()
        .enumerate()
        .filter_map(|(i, (w, l))| face_page(model, w, l, i + 1))
        .collect()
}

/// The codes of the pages, «D-01», «D-02»…
pub fn codes(count: usize) -> Vec<String> {
    (1..=count).map(|i| format!("D-{i:02}")).collect()
}

fn frame(s: &mut String, model: &Model, code: &str, title: &str, scale: f64) {
    rect(
        s,
        MARGIN,
        MARGIN,
        W - 2.0 * MARGIN,
        H - 2.0 * MARGIN,
        0.5,
        INK,
    );
    let (x, y) = (W - MARGIN - BLOCK_W, H - MARGIN - BLOCK_H);
    rect(s, x, y, BLOCK_W, BLOCK_H, 0.35, INK);
    line(
        s,
        (x + BLOCK_W - 30.0, y),
        (x + BLOCK_W - 30.0, y + BLOCK_H),
        0.25,
        INK,
    );
    text(s, &model.project_name, x + 4.0, y + 7.0, 3.5, INK);
    let mut t = title.to_string();
    while crate::lettering::text_width(&t, 3.0) > BLOCK_W - 40.0 && t.chars().count() > 4 {
        t.pop();
    }
    text(s, &t, x + 4.0, y + 15.0, 3.0, INK);
    text(
        s,
        &format!("Despiece a mano · vista desde fuera · E 1:{scale:.0}"),
        x + 4.0,
        y + 22.5,
        2.5,
        GREY,
    );
    centred(s, code, x + BLOCK_W - 15.0, y + 17.0, 6.0, INK);
}

fn face_page(
    model: &Model,
    wall: archisim_core::WallId,
    layer: usize,
    nth: usize,
) -> Option<String> {
    let w = model.wall(wall)?;
    let t = model.wall_type(w.wall_type)?;
    let l = t.layers.get(layer)?;
    let pieces: Vec<BoardPiece> = wall_boards(model, wall)
        .into_iter()
        .filter(|p| p.layer == layer)
        .collect();
    let material = |m| {
        model
            .material(m)
            .map_or("?".to_string(), |x| x.name.clone())
    };
    let len = w.length();
    let (z0, z1) = (w.base_offset, w.base_offset + w.height);
    // The drawing on the left two thirds, the table on the right.
    let area_w = (W - 2.0 * MARGIN) * 0.62 - 30.0;
    let area_h = H - 2.0 * MARGIN - 60.0;
    let scale = SCALES
        .into_iter()
        .find(|k| len * 1000.0 / k <= area_w && (z1 - z0) * 1000.0 / k <= area_h)
        .unwrap_or(200.0);
    let k = 1000.0 / scale;
    let left = MARGIN + 30.0;
    let bottom = MARGIN + 25.0 + (z1 - z0) * k;
    // Seen from outside: the wall's start on the right.
    let px = |x: f64| left + (len - x) * k;
    let py = |z: f64| bottom - (z - z0) * k;
    let mut s = String::new();
    let code = format!("D-{nth:02}");
    frame(
        &mut s,
        model,
        &code,
        &format!(
            "Muro {} · capa {} · {}",
            wall.0,
            layer + 1,
            material(l.material)
        ),
        scale,
    );
    // The pieces.
    let mut rows = Vec::new();
    for p in &pieces {
        let outline: Vec<(f64, f64)> = if p.outline.is_empty() {
            let (x0, y0) = p.at;
            let (x1, y1) = (x0 + p.size.0, y0 + p.size.1);
            vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
        } else {
            p.outline.clone()
        };
        let fill = if p.on_site {
            ON_SITE.to_string()
        } else if p.material != l.material {
            let c = model.surface_color(p.material);
            format!("rgb({},{},{})", c[0], c[1], c[2])
        } else {
            "none".to_string()
        };
        let d: Vec<String> = outline
            .iter()
            .map(|(x, z)| format!("{},{}", n(px(*x)), n(py(*z))))
            .collect();
        let _ = writeln!(
            s,
            r#"<polygon points="{}" fill="{fill}" fill-opacity="0.55" stroke="{INK}" stroke-width="0.3"/>"#,
            d.join(" ")
        );
        let cx = outline.iter().map(|q| q.0).sum::<f64>() / outline.len() as f64;
        let cz = outline.iter().map(|q| q.1).sum::<f64>() / outline.len() as f64;
        let short = p.mark.rsplit('·').next().unwrap_or(&p.mark).to_string();
        if p.size.0 * k > 6.0 && p.size.1 * k > 4.0 {
            centred(&mut s, &short, px(cx), py(cz) + 1.0, 2.5, INK);
        }
        rows.push(vec![
            p.mark.clone(),
            material(p.material),
            format!("{}×{}", cm(p.size.0), cm(p.size.1)),
            if p.on_site { "en obra" } else { "fábrica" }.to_string(),
        ]);
    }
    // The doors and windows, dashed.
    for o in archisim_geom::openings_in_wall(model, wall) {
        let (a, b) = (px(o.t0), px(o.t1));
        let (c, d) = (py(o.sill), py(o.head));
        let _ = writeln!(
            s,
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="white" stroke="{GREY}" stroke-width="0.35" stroke-dasharray="2 1.2"/>"#,
            n(a.min(b)),
            n(c.min(d)),
            n((a - b).abs()),
            n((c - d).abs())
        );
    }
    // The face's outline.
    rect(&mut s, px(len), py(z1), len * k, (z1 - z0) * k, 0.5, INK);
    // The joints along the bottom and up the left side (the wall's end).
    let mut xs: Vec<f64> = vec![0.0, len];
    let mut zs: Vec<f64> = vec![z0, z1];
    for p in &pieces {
        xs.push(p.at.0);
        xs.push(p.at.0 + p.size.0);
        zs.push(p.at.1);
        zs.push(p.at.1 + p.size.1);
    }
    let tidy = |v: &mut Vec<f64>, lo: f64, hi: f64| {
        v.retain(|x| *x >= lo - 1e-6 && *x <= hi + 1e-6);
        v.sort_by(f64::total_cmp);
        v.dedup_by(|a, b| (*a - *b).abs() < 0.005);
    };
    tidy(&mut xs, 0.0, len);
    tidy(&mut zs, z0, z1);
    let yd = bottom + 8.0;
    line(&mut s, (px(len), yd), (px(0.0), yd), 0.25, INK);
    for pair in xs.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        line(&mut s, (px(a), yd - 1.5), (px(a), yd + 1.5), 0.25, INK);
        if (b - a) * k > 6.0 {
            centred(
                &mut s,
                &cm(b - a),
                (px(a) + px(b)) / 2.0,
                yd - 1.2,
                2.2,
                INK,
            );
        }
    }
    line(&mut s, (px(len), yd - 1.5), (px(len), yd + 1.5), 0.25, INK);
    centred(
        &mut s,
        &format!("{} cm", cm(len)),
        (px(0.0) + px(len)) / 2.0,
        yd + 6.0,
        2.6,
        INK,
    );
    let xd = px(len) - 8.0;
    line(&mut s, (xd, py(z0)), (xd, py(z1)), 0.25, INK);
    for pair in zs.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        line(&mut s, (xd - 1.5, py(a)), (xd + 1.5, py(a)), 0.25, INK);
        if (b - a) * k > 5.0 {
            let _ = writeln!(
                s,
                r#"<g transform="translate({},{}) rotate(-90)">"#,
                n(xd - 1.2),
                n((py(a) + py(b)) / 2.0)
            );
            centred(&mut s, &cm(b - a), 0.0, 0.0, 2.2, INK);
            let _ = writeln!(s, "</g>");
        }
    }
    line(&mut s, (xd - 1.5, py(z1)), (xd + 1.5, py(z1)), 0.25, INK);
    text(
        &mut s,
        "Inicio del muro →",
        px(0.0) - 26.0,
        py(z1) - 3.0,
        2.4,
        GREY,
    );
    // The table, and the totals by material and place.
    let tx = MARGIN + (W - 2.0 * MARGIN) * 0.62 + 4.0;
    let max_rows = ((H - 2.0 * MARGIN - BLOCK_H - 30.0) / 4.2) as usize;
    let shown: Vec<Vec<String>> = rows.iter().take(max_rows).cloned().collect();
    table(
        &mut s,
        tx,
        MARGIN + 6.0,
        &[36.0, 60.0, 25.0, 20.0],
        &["Marca", "Material", "cm", "Dónde"],
        &shown,
        4.2,
    );
    if rows.len() > shown.len() {
        text(
            &mut s,
            &format!(
                "… y {} piezas más (en tableros.csv)",
                rows.len() - shown.len()
            ),
            tx,
            MARGIN + 10.0 + 4.2 * (shown.len() + 1) as f64,
            2.5,
            GREY,
        );
    }
    let mut totals: BTreeMap<(String, &str), usize> = BTreeMap::new();
    for p in &pieces {
        *totals
            .entry((
                material(p.material),
                if p.on_site { "en obra" } else { "fábrica" },
            ))
            .or_default() += 1;
    }
    let mut y = H - MARGIN - BLOCK_H - 4.0 - 4.0 * totals.len() as f64;
    for ((m, place), count) in totals {
        text(&mut s, &format!("{count} × {m} ({place})"), tx, y, 2.6, INK);
        y += 4.0;
    }
    Some(page(&s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_face_composed_by_hand_has_its_sheet() {
        let mut m = archisim_core::Model::new("Despiece");
        assert!(pages(&m).is_empty());
        let lv = m.add_level("PB", 0.0);
        let cer = m.add_material(
            "Placa cerámica",
            "ceramic",
            [190, 120, 90],
            archisim_core::CutPattern::Solid,
        );
        let wt = m
            .add_wall_type(
                "Fachada",
                archisim_core::WallFunction::Exterior,
                vec![archisim_core::Layer {
                    material: cer,
                    thickness: 0.012,
                    function: archisim_core::LayerFunction::Finish,
                }],
            )
            .unwrap();
        let w = m
            .add_wall(archisim_core::NewWall {
                wall_type: wt,
                level: lv,
                start: archisim_core::Point2::new(0.0, 0.0),
                end: archisim_core::Point2::new(4.8, 0.0),
                bulge: 0.0,
                reference: archisim_core::ReferenceLine::ExteriorFace,
                height: 2.5,
                base_offset: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap();
        let p = archisim_core::Point2::new;
        m.set_board_face(
            w,
            0,
            Some(archisim_core::BoardFace {
                wall: w,
                layer: 0,
                joints: vec![(p(1.2, 0.0), p(1.2, 2.5)), (p(2.4, 0.0), p(2.4, 2.5))],
                marks: vec![archisim_core::PieceMark {
                    at: p(0.5, 1.0),
                    material: None,
                    on_site: true,
                }],
                joint: None,
            }),
        )
        .unwrap();
        let v = pages(&m);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].matches("<polygon").count(), 3);
        assert_eq!(v[0].matches(ON_SITE).count(), 1);
        assert!(roxmltree_like(&v[0]));
    }

    /// A rough check that the page is well formed: as many opening as
    /// closing groups.
    fn roxmltree_like(s: &str) -> bool {
        s.starts_with("<svg") && s.matches("<g ").count() == s.matches("</g>").count()
    }
}
