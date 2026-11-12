//! The factory drawings of the modular boxes (M16, `docs/MODULAR.md`): one
//! A3 sheet per box with its wall panels in elevation, dimensioned (overall
//! size, the studs from the panel's left end, the openings), and A4 sheets
//! of labels, one per piece, with its mark, box, panel, profile, length
//! and a QR code that carries them.

use std::fmt::Write as _;

use archisim_core::Model;
use archisim_geom::factory::{all_pieces, label_code, FactoryPiece};
use archisim_geom::framing::{panel_elevation, wall_panels, Role};
use qrcodegen::{QrCode, QrCodeEcc};

use crate::structure_sheets::{
    centred, line, n, page, rect, text, BLOCK_H, BLOCK_W, GREY, H, INK, MARGIN, W,
};

/// Scale of the panels on their sheet.
pub const PANEL_SCALE: f64 = 25.0;
const STEEL: &str = "#cfd3d8";

fn mm(v: f64) -> String {
    format!("{:.0}", v * 1000.0)
}

/// A dimension line along x at `y`, from `x0` to `x1` (paper mm), its text
/// above.
fn dim_x(s: &mut String, x0: f64, x1: f64, y: f64, label: &str) {
    line(s, (x0, y), (x1, y), 0.18, INK);
    for x in [x0, x1] {
        line(s, (x - 0.8, y + 0.8), (x + 0.8, y - 0.8), 0.25, INK);
        line(s, (x, y - 1.5), (x, y + 1.5), 0.13, INK);
    }
    if (x1 - x0).abs() > 4.0 {
        centred(s, label, (x0 + x1) / 2.0, y - 1.0, 1.8, INK);
    }
}

fn dim_y(s: &mut String, x: f64, y0: f64, y1: f64, label: &str) {
    line(s, (x, y0), (x, y1), 0.18, INK);
    for y in [y0, y1] {
        line(s, (x - 0.8, y + 0.8), (x + 0.8, y - 0.8), 0.25, INK);
    }
    let _ = writeln!(
        s,
        r#"<g transform="translate({} {}) rotate(-90)">"#,
        n(x - 1.0),
        n((y0 + y1) / 2.0)
    );
    centred(s, label, 0.0, 0.0, 1.8, INK);
    s.push_str("</g>\n");
}

fn frame(s: &mut String, model: &Model, code: &str, title: &str) {
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
    text(s, title, x + 4.0, y + 15.0, 3.0, INK);
    text(
        s,
        &format!("Fábrica de cajas · E 1:{PANEL_SCALE:.0}"),
        x + 4.0,
        y + 22.5,
        2.5,
        GREY,
    );
    centred(s, code, x + BLOCK_W - 15.0, y + 17.0, 6.0, INK);
}

/// One A3 sheet per box (or more, if its panels do not fit) with its wall
/// panels in elevation, seen from outside, dimensioned. Empty without
/// boxes or panels.
pub fn panel_pages(model: &Model) -> Vec<(String, String)> {
    let mut boxes: Vec<_> = model.boxes().collect();
    boxes.sort_by(|a, b| a.mark.cmp(&b.mark));
    let panels = wall_panels(model);
    let k = 1000.0 / PANEL_SCALE; // paper mm per m
    let mut out = Vec::new();
    for b in boxes {
        let held = model.box_members(b.id);
        let mine: Vec<_> = panels
            .iter()
            .filter(|p| held.walls.contains(&p.wall))
            .collect();
        if mine.is_empty() {
            continue;
        }
        let mut sheet = 1;
        let mut s = String::new();
        let (left, top) = (MARGIN + 12.0, MARGIN + 16.0);
        let (mut x, mut y, mut row_h) = (left, top, 0.0_f64);
        let flush = |s: &mut String, sheet: &mut usize, out: &mut Vec<(String, String)>| {
            let code = format!("F-{}-{}", b.mark.trim_start_matches("MOD-"), sheet);
            let mut body = String::new();
            frame(
                &mut body,
                model,
                &code,
                &format!("Caja {} · paneles", b.mark),
            );
            body.push_str(s);
            out.push((code, page(&body)));
            s.clear();
            *sheet += 1;
        };
        for p in mine {
            let (pw, ph) = (p.length() * k, p.height() * k);
            if x + pw > W - MARGIN - 8.0 && x > left {
                x = left;
                y += row_h + 22.0;
                row_h = 0.0;
            }
            if y + ph > H - MARGIN - BLOCK_H - 14.0 && y > top {
                flush(&mut s, &mut sheet, &mut out);
                x = left;
                y = top;
                row_h = 0.0;
            }
            // Its pieces, seen from outside.
            let y0 = y + ph; // the panel's bottom on paper
            let mut studs: Vec<f64> = Vec::new();
            for (role, a, c, w) in panel_elevation(p) {
                let (x0, x1) = (x + a[0].min(c[0]) * k, x + a[0].max(c[0]) * k);
                let (z0, z1) = (a[1].min(c[1]), a[1].max(c[1]));
                let upright = (c[1] - a[1]).abs() > (c[0] - a[0]).abs();
                let (rx, ry, rw, rh) = if upright {
                    let cx = (x0 + x1) / 2.0;
                    (cx - w * k / 2.0, y0 - z1 * k, w * k, (z1 - z0) * k)
                } else {
                    let cz = (z0 + z1) / 2.0;
                    (x0, y0 - cz * k - w * k / 2.0, x1 - x0, w * k)
                };
                let _ = writeln!(
                    s,
                    r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{INK}" stroke-width="0.18"/>"#,
                    n(rx),
                    n(ry),
                    n(rw.max(0.3)),
                    n(rh.max(0.3)),
                    if role == Role::Header {
                        "#9aa2ab"
                    } else {
                        STEEL
                    }
                );
                if upright {
                    studs.push((x0 + x1) / 2.0);
                }
            }
            // Mark, overall size and the studs from the left end.
            text(&mut s, &p.mark, x, y - 9.0, 3.0, INK);
            dim_x(&mut s, x, x + pw, y - 3.0, &mm(p.length()));
            dim_y(&mut s, x - 4.0, y, y0, &mm(p.height()));
            studs.sort_by(|a, b| a.total_cmp(b));
            studs.dedup_by(|a, b| (*a - *b).abs() < 0.2);
            let mut last = x;
            for sx in studs {
                if sx - last > 0.5 {
                    dim_x(&mut s, last, sx, y0 + 5.0, &mm((sx - last) / k));
                    last = sx;
                }
            }
            if x + pw - last > 0.5 {
                dim_x(&mut s, last, x + pw, y0 + 5.0, &mm((x + pw - last) / k));
            }
            x += pw + 18.0;
            row_h = row_h.max(ph);
        }
        flush(&mut s, &mut sheet, &mut out);
    }
    out
}

/// Labels on an A4 sheet: three across, eight down.
pub const LABELS_ACROSS: usize = 3;
pub const LABELS_DOWN: usize = 8;
const A4: (f64, f64) = (210.0, 297.0);
const LABEL: (f64, f64) = (70.0, 37.0);

fn qr(s: &mut String, code: &str, x: f64, y: f64, size: f64) {
    let Ok(q) = QrCode::encode_text(code, QrCodeEcc::Medium) else {
        return;
    };
    let n_mod = q.size();
    let cell = size / n_mod as f64;
    let mut d = String::new();
    for j in 0..n_mod {
        for i in 0..n_mod {
            if q.get_module(i, j) {
                let _ = write!(
                    d,
                    "M{} {}h{}v{}h-{}z",
                    n(x + i as f64 * cell),
                    n(y + j as f64 * cell),
                    n(cell),
                    n(cell),
                    n(cell)
                );
            }
        }
    }
    let _ = writeln!(s, r#"<path d="{d}" fill="{INK}"/>"#);
}

fn label(s: &mut String, model: &Model, p: &FactoryPiece, x: f64, y: f64) {
    rect(
        s,
        x + 1.0,
        y + 1.0,
        LABEL.0 - 2.0,
        LABEL.1 - 2.0,
        0.15,
        GREY,
    );
    qr(s, &label_code(model, p), x + 3.0, y + 3.5, 30.0);
    let tx = x + 36.0;
    text(s, &p.mark, tx, y + 8.0, 2.6, INK);
    text(s, &format!("Caja {}", p.box_mark), tx, y + 13.5, 2.2, INK);
    text(s, &format!("Panel {}", p.panel), tx, y + 18.0, 2.2, INK);
    text(s, &p.name, tx, y + 22.5, 1.9, GREY);
    text(s, &p.section, tx, y + 27.0, 1.9, INK);
    text(s, &format!("{} mm", mm(p.length)), tx, y + 32.5, 3.0, INK);
}

/// The labels of every piece, A4 sheets of 24, as SVG pages.
pub fn label_pages(model: &Model) -> Vec<String> {
    let pieces = all_pieces(model);
    let per = LABELS_ACROSS * LABELS_DOWN;
    let ox = (A4.0 - LABELS_ACROSS as f64 * LABEL.0) / 2.0;
    let oy = (A4.1 - LABELS_DOWN as f64 * LABEL.1) / 2.0;
    pieces
        .chunks(per)
        .map(|chunk| {
            let mut s = String::new();
            for (i, p) in chunk.iter().enumerate() {
                let (c, r) = (i % LABELS_ACROSS, i / LABELS_ACROSS);
                label(&mut s, model, p, ox + c as f64 * LABEL.0, oy + r as f64 * LABEL.1);
            }
            format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}mm" height="{}mm" viewBox="0 0 {} {}"><rect width="{}" height="{}" fill="white"/>
{s}</svg>
"#,
                A4.0, A4.1, A4.0, A4.1, A4.0, A4.1
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_label_s_code_is_drawn() {
        let mut s = String::new();
        qr(
            &mut s,
            "ARCHISIM;Casa;MOD-01·M1-1·01;C 100;Montante;2500",
            0.0,
            0.0,
            30.0,
        );
        assert!(s.contains("<path d=\"M"));
        // 25 modules or more, a few hundred dark ones.
        assert!(s.matches('M').count() > 100);
    }
}
