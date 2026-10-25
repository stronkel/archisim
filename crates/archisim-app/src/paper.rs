//! What the document sheets share (the carpentry sheets, the carpentry
//! report and the table of areas): A3 landscape paper in millimetres (y
//! down), the frame and the title block, ArchiSim's lettering, wrapped
//! paragraphs and plain tables. Each sheet is an SVG document that prints
//! at its size and turns into PDF like the plans.

use std::fmt::Write as _;

use archisim_core::Point2;
use archisim_plan::lettering::{self, Align};

/// A3 landscape, in millimetres.
pub const PAPER: (f64, f64) = (420.0, 297.0);
/// Margins and the height of the title block.
pub const MARGIN: f64 = 10.0;
pub const BLOCK: f64 = 22.0;
pub const INK: &str = "#1e1e1e";

/// The title block's texts.
pub struct TitleBlock<'a> {
    pub project: &'a str,
    pub title: &'a str,
    /// "E 1:50", or empty.
    pub scale: &'a str,
    pub sheet: usize,
    pub sheets: usize,
}

/// A sheet with its frame and title block; returns the SVG so far (to
/// close with `</svg>`) and the top of the title block, below which
/// nothing else goes.
pub fn sheet(b: &TitleBlock) -> (String, f64) {
    let (w, h) = PAPER;
    let mut s = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}mm" height="{h}mm" viewBox="0 0 {w} {h}"><rect width="{w}" height="{h}" fill="white"/>"#
    );
    let _ = write!(
        s,
        r#"<rect x="{MARGIN}" y="{MARGIN}" width="{}" height="{}" fill="none" stroke="{INK}" stroke-width="0.5"/>"#,
        w - 2.0 * MARGIN,
        h - 2.0 * MARGIN
    );
    let by = h - MARGIN - BLOCK;
    let _ = write!(
        s,
        r#"<line x1="{MARGIN}" y1="{by}" x2="{}" y2="{by}" stroke="{INK}" stroke-width="0.35"/>"#,
        w - MARGIN
    );
    text(&mut s, b.project, MARGIN + 6.0, by + 9.0, 4.0, Align::Left);
    text(&mut s, b.title, MARGIN + 6.0, by + 16.0, 2.8, Align::Left);
    if !b.scale.is_empty() {
        text(
            &mut s,
            b.scale,
            w - MARGIN - 70.0,
            by + 9.0,
            3.5,
            Align::Left,
        );
    }
    text(
        &mut s,
        &format!("Hoja {} de {}", b.sheet, b.sheets),
        w - MARGIN - 70.0,
        by + 16.0,
        2.8,
        Align::Left,
    );
    let _ = write!(
        s,
        r#"<line x1="{x}" y1="{by}" x2="{x}" y2="{}" stroke="{INK}" stroke-width="0.35"/>"#,
        h - MARGIN,
        x = w - MARGIN - 76.0
    );
    (s, by)
}

/// A text in ArchiSim's lettering at `(x, y)` of the paper (y down, the
/// baseline at `y`).
pub fn text(s: &mut String, t: &str, x: f64, y: f64, h: f64, align: Align) {
    if t.is_empty() {
        return;
    }
    let d = lettering::text_path(t, Point2::new(x, -y), Point2::new(1.0, 0.0), h, align);
    let _ = write!(s, r#"<path d="{d}" {}/>"#, lettering::paint(h, INK));
}

/// A text that ends at `x`.
pub fn text_right(s: &mut String, t: &str, x: f64, y: f64, h: f64) {
    text(s, t, x - lettering::text_width(t, h), y, h, Align::Left);
}

/// `t` cut into lines no wider than `width` at height `h`, by words.
pub fn wrap(t: &str, h: f64, width: f64) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in t.split_whitespace() {
        let next = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if !line.is_empty() && lettering::text_width(&next, h) > width {
            lines.push(std::mem::take(&mut line));
            line = word.to_string();
        } else {
            line = next;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// Metres with two decimals and a comma.
pub fn metres(v: f64) -> String {
    format!("{v:.2}").replace('.', ",")
}

/// A line from `(x1, y1)` to `(x2, y2)`.
pub fn line(s: &mut String, x1: f64, y1: f64, x2: f64, y2: f64, pen: f64) {
    let _ = write!(
        s,
        r#"<line x1="{x1:.2}" y1="{y1:.2}" x2="{x2:.2}" y2="{y2:.2}" stroke="{INK}" stroke-width="{pen}"/>"#
    );
}

/// A plain table for the sheets: a title over it, headings, rows and an
/// optional row of totals; the columns marked numeric are aligned right.
pub struct PaperTable {
    pub title: String,
    pub columns: Vec<(String, bool)>,
    pub rows: Vec<Vec<String>>,
    pub totals: Option<Vec<String>>,
}

/// Height of a row of a table, millimetres.
pub const ROW: f64 = 5.0;
const CELL_TEXT: f64 = 2.2;

impl PaperTable {
    /// Its width, given the width of the first text column, and the width
    /// of each column.
    pub fn widths(&self, total: f64) -> Vec<f64> {
        // The numbers and short columns as wide as they need; the rest
        // shares what is left.
        let need: Vec<f64> = (0..self.columns.len())
            .map(|i| {
                let head = lettering::text_width(&self.columns[i].0, CELL_TEXT);
                self.rows
                    .iter()
                    .chain(self.totals.iter())
                    .filter_map(|r| r.get(i))
                    .map(|c| lettering::text_width(c, CELL_TEXT))
                    .fold(head, f64::max)
                    + 4.0
            })
            .collect();
        let sum: f64 = need.iter().sum();
        if sum >= total {
            return need.iter().map(|w| w * total / sum).collect();
        }
        // Spare room to the text columns.
        let texts: f64 = (0..need.len())
            .filter(|i| !self.columns[*i].1)
            .map(|i| need[i])
            .sum();
        need.iter()
            .enumerate()
            .map(|(i, w)| {
                if self.columns[i].1 || texts <= 0.0 {
                    *w
                } else {
                    w + (total - sum) * w / texts
                }
            })
            .collect()
    }

    /// Its height on paper: title, headings, rows and totals.
    pub fn height(&self) -> f64 {
        ROW * (2 + self.rows.len() + usize::from(self.totals.is_some())) as f64
    }

    /// Draws it with its top left at `(x, y)` and width `w`.
    pub fn draw(&self, s: &mut String, x: f64, y: f64, w: f64) {
        let widths = self.widths(w);
        text(s, &self.title, x, y + ROW - 1.4, 2.6, Align::Left);
        let top = y + ROW;
        let n = 1 + self.rows.len() + usize::from(self.totals.is_some());
        let bottom = top + ROW * n as f64;
        let _ = write!(
            s,
            r#"<rect x="{x:.2}" y="{top:.2}" width="{w:.2}" height="{:.2}" fill="none" stroke="{INK}" stroke-width="0.35"/>"#,
            bottom - top
        );
        line(s, x, top + ROW, x + w, top + ROW, 0.35);
        for i in 2..n {
            let yy = top + ROW * i as f64;
            let pen = if self.totals.is_some() && i == n - 1 {
                0.35
            } else {
                0.13
            };
            line(s, x, yy, x + w, yy, pen);
        }
        let mut cx = x;
        for (i, cw) in widths.iter().enumerate() {
            if i > 0 {
                line(s, cx, top, cx, bottom, 0.13);
            }
            let cell = |s: &mut String, r: usize, v: &str| {
                let base = top + ROW * r as f64 + ROW - 1.5;
                if self.columns[i].1 && r > 0 {
                    text_right(s, v, cx + cw - 2.0, base, CELL_TEXT);
                } else {
                    // Long texts cut to their cell.
                    let mut v = v.to_string();
                    while v.chars().count() > 1 && lettering::text_width(&v, CELL_TEXT) > cw - 3.0 {
                        v.pop();
                    }
                    text(s, &v, cx + 2.0, base, CELL_TEXT, Align::Left);
                }
            };
            cell(s, 0, &self.columns[i].0);
            for (r, row) in self.rows.iter().chain(self.totals.iter()).enumerate() {
                if let Some(v) = row.get(i) {
                    cell(s, r + 1, v);
                }
            }
            cx += cw;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraphs_wrap_by_words() {
        let lines = wrap("una puerta de paso de una hoja abatible", 2.4, 30.0);
        assert!(lines.len() > 1);
        assert!(lines
            .iter()
            .all(|l| lettering::text_width(l, 2.4) <= 30.0 || !l.contains(' ')));
        assert_eq!(lines.join(" "), "una puerta de paso de una hoja abatible");
    }

    #[test]
    fn tables_fill_their_width() {
        let t = PaperTable {
            title: "Planta baja".into(),
            columns: vec![("Local".into(), false), ("m²".into(), true)],
            rows: vec![vec!["Salón".into(), "24,50".into()]],
            totals: Some(vec!["Total".into(), "24,50".into()]),
        };
        let w = t.widths(120.0);
        assert!((w.iter().sum::<f64>() - 120.0).abs() < 1e-9);
        assert_eq!(t.height(), 4.0 * ROW);
        let mut s = String::new();
        t.draw(&mut s, 10.0, 10.0, 120.0);
        assert!(s.contains("<rect"));
    }
}
