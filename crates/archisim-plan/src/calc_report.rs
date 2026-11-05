//! The calculation report of the structure on paper: A4 portrait pages
//! (SVG documents, millimetres) from the blocks of
//! `archisim_struct::report`, with a cover, numbered chapters, tables,
//! the checks marked by their result and a footer on every page.

use std::fmt::Write as _;

use archisim_core::{Model, Point2};
use archisim_struct::lightframe::Status;
use archisim_struct::report::Block;

use crate::lettering::{paint, text_path, text_width, Align};

const W: f64 = 210.0;
const H: f64 = 297.0;
const MARGIN: f64 = 20.0;
const BODY: f64 = 2.5;
const LEADING: f64 = 4.0;
const INK: &str = "#1a1a1a";
const GREY: &str = "#6b6b6b";
const RULE: &str = "#b9b9b9";

fn n(v: f64) -> String {
    let v = (v * 100.0).round() / 100.0;
    format!("{v}")
}

fn text(s: &mut String, t: &str, x: f64, y: f64, h: f64, colour: &str) {
    if t.trim().is_empty() {
        return;
    }
    let _ = writeln!(
        s,
        r#"<path d="{}" {}/>"#,
        text_path(t, Point2::new(x, -y), Point2::new(1.0, 0.0), h, Align::Left),
        paint(h, colour)
    );
}

/// `t` broken into lines no wider than `width` at height `h`.
fn wrap(t: &str, width: f64, h: f64) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in t.split_whitespace() {
        let trial = if cur.is_empty() {
            word.to_string()
        } else {
            format!("{cur} {word}")
        };
        if text_width(&trial, h) > width && !cur.is_empty() {
            lines.push(std::mem::take(&mut cur));
            cur = word.to_string();
        } else {
            cur = trial;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

struct Pages {
    pages: Vec<String>,
    y: f64,
}

impl Pages {
    fn new() -> Self {
        Pages {
            pages: vec![String::new()],
            y: MARGIN,
        }
    }

    fn page(&mut self) -> &mut String {
        self.pages.last_mut().expect("a page")
    }

    fn new_page(&mut self) {
        self.pages.push(String::new());
        self.y = MARGIN;
    }

    /// Room for `h` mm more on this page, or the next one.
    fn room(&mut self, h: f64) {
        if self.y + h > H - MARGIN - 8.0 && self.y > MARGIN + 1.0 {
            self.new_page();
        }
    }
}

/// The report's pages.
pub fn pages(model: &Model, blocks: &[Block]) -> Vec<String> {
    let mut p = Pages::new();
    let body_w = W - 2.0 * MARGIN;
    for b in blocks {
        match b {
            Block::Cover {
                title,
                project,
                lines,
            } => {
                p.y = 90.0;
                let y = p.y;
                text(p.page(), title, MARGIN, y, 7.0, INK);
                let y = y + 14.0;
                text(p.page(), project, MARGIN, y, 5.0, INK);
                let mut y = y + 16.0;
                for l in lines {
                    for line in wrap(l, body_w, 3.2) {
                        text(p.page(), &line, MARGIN, y, 3.2, GREY);
                        y += 5.5;
                    }
                    y += 2.0;
                }
                p.y = y;
            }
            Block::PageBreak => p.new_page(),
            Block::Heading(level, t) => {
                let h = if *level == 1 { 4.2 } else { 3.2 };
                p.room(h * 4.0);
                p.y += if *level == 1 { 6.0 } else { 3.0 };
                let y = p.y + h;
                text(p.page(), t, MARGIN, y, h, INK);
                if *level == 1 {
                    let yy = y + 1.8;
                    let _ = writeln!(
                        p.page(),
                        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{RULE}" stroke-width="0.25"/>"#,
                        n(MARGIN),
                        n(yy),
                        n(W - MARGIN),
                        n(yy)
                    );
                }
                p.y = y + if *level == 1 { 5.0 } else { 3.0 };
            }
            Block::Para(t) => {
                for line in wrap(t, body_w, BODY) {
                    p.room(LEADING);
                    let y = p.y + BODY;
                    text(p.page(), &line, MARGIN, y, BODY, INK);
                    p.y += LEADING;
                }
                p.y += 1.5;
            }
            Block::Check { status, text: t } => {
                let (mark, colour) = match status {
                    Status::Pass => ("cumple", "#2e7d4f"),
                    Status::Warn => ("revisar", "#b7791f"),
                    Status::Fail => ("NO CUMPLE", "#b3261e"),
                    Status::Info => ("dato", GREY),
                };
                let indent = 22.0;
                let lines = wrap(t, body_w - indent, BODY);
                p.room(LEADING * lines.len().min(3) as f64);
                let y0 = p.y + BODY;
                text(p.page(), mark, MARGIN, y0, BODY, colour);
                for line in lines {
                    p.room(LEADING);
                    let y = p.y + BODY;
                    text(p.page(), &line, MARGIN + indent, y, BODY, INK);
                    p.y += LEADING;
                }
                p.y += 1.0;
            }
            Block::Table { columns, rows } => {
                let total: f64 = columns.iter().map(|c| c.1).sum::<f64>().max(1e-9);
                let widths: Vec<f64> = columns.iter().map(|c| c.1 / total * body_w).collect();
                let h = 2.2;
                let lead = 3.4;
                let row_height = |cells: &[String]| -> f64 {
                    cells
                        .iter()
                        .zip(&widths)
                        .map(|(c, w)| wrap(c, w - 2.0, h).len())
                        .max()
                        .unwrap_or(1) as f64
                        * lead
                        + 1.6
                };
                let draw_row = |p: &mut Pages, cells: &[String], header: bool| {
                    let row_h = row_height(cells);
                    let top = p.y;
                    if header {
                        let _ = writeln!(
                            p.page(),
                            r##"<rect x="{}" y="{}" width="{}" height="{}" fill="#eeeeee"/>"##,
                            n(MARGIN),
                            n(top),
                            n(body_w),
                            n(row_h)
                        );
                    }
                    let mut x = MARGIN;
                    for (cell, w) in cells.iter().zip(&widths) {
                        let mut y = top + 0.8 + h;
                        for line in wrap(cell, w - 2.0, h) {
                            text(
                                p.page(),
                                &line,
                                x + 1.0,
                                y,
                                h,
                                if header { GREY } else { INK },
                            );
                            y += lead;
                        }
                        x += w;
                    }
                    let yy = top + row_h;
                    let _ = writeln!(
                        p.page(),
                        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{RULE}" stroke-width="0.18"/>"#,
                        n(MARGIN),
                        n(yy),
                        n(W - MARGIN),
                        n(yy)
                    );
                    p.y = yy;
                };
                let header: Vec<String> = columns.iter().map(|c| c.0.clone()).collect();
                p.room(row_height(&header) + row_height(rows.first().unwrap_or(&header)));
                draw_row(&mut p, &header, true);
                for r in rows {
                    // A table that goes on to the next page repeats its
                    // header there.
                    if p.y + row_height(r) > H - MARGIN - 8.0 {
                        p.new_page();
                        draw_row(&mut p, &header, true);
                    }
                    draw_row(&mut p, r, false);
                }
                p.y += 3.0;
            }
        }
    }
    let count = p.pages.len();
    p.pages
        .into_iter()
        .enumerate()
        .map(|(i, body)| {
            let mut foot = String::new();
            text(
                &mut foot,
                &format!(
                    "{} · Anejo de cálculo de la estructura · hoja {}/{}",
                    model.project_name,
                    i + 1,
                    count
                ),
                MARGIN,
                H - 10.0,
                2.2,
                GREY,
            );
            format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}mm" height="{h}mm" viewBox="0 0 {w} {h}"><rect width="{w}" height="{h}" fill="white"/>
{body}{foot}</svg>"#,
                w = n(W),
                h = n(H),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_is_laid_on_pages() {
        let blocks = vec![
            Block::Cover {
                title: "Anejo".into(),
                project: "Casa".into(),
                lines: vec!["Programa".into()],
            },
            Block::PageBreak,
            Block::Heading(1, "1. Normativa".into()),
            Block::Para("palabra ".repeat(400)),
            Block::Table {
                columns: vec![("A".into(), 1.0), ("B".into(), 2.0)],
                rows: (0..120).map(|i| vec![format!("{i}"), "x".into()]).collect(),
            },
            Block::Check {
                status: Status::Fail,
                text: "Algo no cumple".into(),
            },
        ];
        let m = Model::new("Casa");
        let ps = pages(&m, &blocks);
        assert!(ps.len() >= 4, "{}", ps.len());
        assert!(ps.iter().all(|p| p.starts_with("<svg")));
        // Every page has its footer (paths of lettering).
        assert!(ps.iter().all(|p| p.matches("<path").count() >= 1));
    }
}
