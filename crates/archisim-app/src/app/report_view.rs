//! The results of a calculation drawn to be read, not as typed text: the
//! same blocks as the PDF annexes (`archisim_struct::report::Block`) — a
//! card with the verdict and its counts, each section a card of its own
//! with what fails in it, tables in stripes with their figures to the
//! right and what fails in red, and each check with a coloured pill.
//! «Ver como texto» keeps the typed text for copying.

use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke, Ui};

use archisim_struct::lightframe::Status;
use archisim_struct::report::Block;

use super::theme::Palette;

/// How many checks pass, ask to be reviewed and fail (a table's rows with
/// a failing cell count as failing).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    pub pass: usize,
    pub warn: usize,
    pub fail: usize,
}

impl Tally {
    pub fn total(&self) -> usize {
        self.pass + self.warn + self.fail
    }

    fn add(&mut self, o: Tally) {
        self.pass += o.pass;
        self.warn += o.warn;
        self.fail += o.fail;
    }
}

/// Whether a cell of a table says that something fails: «NO CUMPLE», or
/// a size drawn going up to another («100 × 100→150 × 100», «Ø20→26»).
pub fn cell_fails(c: &str) -> bool {
    c.contains("NO CUMPLE") || c.contains('→') || c.contains('↑')
}

/// The tally of some blocks: their checks, and their table rows with a
/// failing cell.
pub fn tally(blocks: &[Block]) -> Tally {
    let mut t = Tally::default();
    for b in blocks {
        match b {
            Block::Check { status, .. } => match status {
                Status::Pass => t.pass += 1,
                Status::Warn => t.warn += 1,
                Status::Fail => t.fail += 1,
                Status::Info => {}
            },
            // A row of a table is data; one with a failing cell, a
            // failure.
            Block::Table { rows, .. } => {
                t.fail += rows
                    .iter()
                    .filter(|r| r.iter().any(|c| cell_fails(c)))
                    .count();
            }
            _ => {}
        }
    }
    t
}

fn colours(p: &Palette, ui: &Ui) -> (Color32, Color32, Color32) {
    (p.good, ui.visuals().warn_fg_color, p.bad)
}

/// A small rounded label in a colour.
pub fn pill(ui: &mut Ui, text: &str, colour: Color32) -> egui::Response {
    egui::Frame::new()
        .fill(colour.gamma_multiply(0.14))
        .stroke(Stroke::new(1.0, colour.gamma_multiply(0.6)))
        .corner_radius(CornerRadius::same(9))
        .inner_margin(egui::Margin::symmetric(7, 1))
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(colour).size(11.5).strong());
        })
        .response
}

/// The verdict at the top: a card that says whether everything passes,
/// with the counts and a bar of how much passes, is to review and fails.
pub fn summary(ui: &mut Ui, t: Tally, what: &str) {
    let p = Palette::current(ui.ctx());
    let (good, warn, bad) = colours(&p, ui);
    let (colour, title) = if t.fail > 0 {
        (
            bad,
            format!(
                "{} {} no cumple{}",
                t.fail,
                if t.fail == 1 { "cosa" } else { "cosas" },
                if t.fail == 1 { "" } else { "n" }
            ),
        )
    } else if t.warn > 0 {
        (
            warn,
            format!(
                "Cumple, con {} cosa{} que revisar",
                t.warn,
                if t.warn == 1 { "" } else { "s" }
            ),
        )
    } else if t.total() > 0 {
        (good, "Todo cumple".to_string())
    } else {
        (p.muted, format!("Nada que comprobar en {what}"))
    };
    egui::Frame::new()
        .fill(colour.gamma_multiply(0.08))
        .stroke(Stroke::new(1.0, colour.gamma_multiply(0.5)))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let mark = if t.fail > 0 {
                    "✖"
                } else if t.warn > 0 {
                    "!"
                } else {
                    "✔"
                };
                ui.label(RichText::new(mark).size(24.0).color(colour).strong());
                ui.vertical(|ui| {
                    ui.label(RichText::new(title).size(17.0).strong().color(colour));
                    ui.horizontal(|ui| {
                        pill(
                            ui,
                            &format!("✔ {} cumple{}", t.pass, plural_n(t.pass)),
                            good,
                        );
                        if t.warn > 0 {
                            pill(ui, &format!("! {} a revisar", t.warn), warn);
                        }
                        if t.fail > 0 {
                            pill(
                                ui,
                                &format!("✖ {} no cumple{}", t.fail, plural_n(t.fail)),
                                bad,
                            );
                        }
                    });
                });
            });
            if t.total() > 0 {
                ui.add_space(6.0);
                bar(ui, t);
            }
        });
}

fn plural_n(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "n"
    }
}

/// A bar of how much passes (green), is to review (amber) and fails (red).
pub fn bar(ui: &mut Ui, t: Tally) {
    let p = Palette::current(ui.ctx());
    let (good, warn, bad) = colours(&p, ui);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 6.0), egui::Sense::hover());
    let total = t.total().max(1) as f32;
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(3), p.line);
    let mut x = rect.left();
    for (n, c) in [(t.pass, good), (t.warn, warn), (t.fail, bad)] {
        if n == 0 {
            continue;
        }
        let w = rect.width() * n as f32 / total;
        let r =
            egui::Rect::from_min_max(egui::pos2(x, rect.top()), egui::pos2(x + w, rect.bottom()));
        painter.rect_filled(r, CornerRadius::same(3), c);
        x += w;
    }
}

/// The blocks split into sections: what goes before the first section,
/// then each level-2 heading with its blocks (a level-1 heading stays a
/// title of its own).
fn sections(blocks: &[Block]) -> Vec<(Option<String>, Vec<&Block>)> {
    let mut out: Vec<(Option<String>, Vec<&Block>)> = vec![(None, Vec::new())];
    for b in blocks {
        match b {
            Block::Heading(2, t) => out.push((Some(t.clone()), Vec::new())),
            Block::Heading(_, t) => out.push((Some(format!("# {t}")), Vec::new())),
            Block::Cover { .. } | Block::PageBreak => {}
            other => out.last_mut().expect("one at least").1.push(other),
        }
    }
    out.retain(|(h, b)| h.is_some() || !b.is_empty());
    out
}

/// Draws the blocks: their sections as cards, open while they have
/// something failing or are short.
pub fn show(ui: &mut Ui, blocks: &[Block], salt: &str) {
    let p = Palette::current(ui.ctx());
    let (_, warn, bad) = colours(&p, ui);
    for (i, (heading, items)) in sections(blocks).into_iter().enumerate() {
        let owned: Vec<Block> = items.iter().map(|b| (*b).clone()).collect();
        let t = tally(&owned);
        match heading {
            None => body(ui, &items),
            Some(h) if h.starts_with("# ") => {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(h.trim_start_matches("# "))
                        .size(18.0)
                        .strong(),
                );
                ui.add_space(2.0);
                body(ui, &items);
            }
            Some(h) => {
                ui.add_space(4.0);
                egui::Frame::new()
                    .fill(p.panel)
                    .stroke(Stroke::new(
                        1.0,
                        if t.fail > 0 {
                            bad.gamma_multiply(0.6)
                        } else {
                            p.line
                        },
                    ))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(egui::Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let id = ui.make_persistent_id((salt, i));
                        let open_default = t.fail > 0 || items.len() <= 6;
                        egui::collapsing_header::CollapsingState::load_with_default_open(
                            ui.ctx(),
                            id,
                            open_default,
                        )
                        .show_header(ui, |ui| {
                            ui.label(RichText::new(&h).strong().size(14.0));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if t.fail > 0 {
                                        pill(ui, &format!("✖ {}", t.fail), bad);
                                    }
                                    if t.warn > 0 {
                                        pill(ui, &format!("! {}", t.warn), warn);
                                    }
                                    if t.fail == 0 && t.warn == 0 && t.pass > 0 {
                                        pill(ui, "✔", p.good);
                                    }
                                },
                            );
                        })
                        .body(|ui| body(ui, &items));
                    });
            }
        }
    }
}

fn body(ui: &mut Ui, items: &[&Block]) {
    for (k, b) in items.iter().enumerate() {
        match b {
            Block::Para(t) => {
                ui.label(
                    RichText::new(t)
                        .color(Palette::current(ui.ctx()).muted)
                        .size(12.0),
                );
            }
            Block::Check { status, text } => check(ui, *status, text),
            Block::Table { columns, rows } => table(ui, columns, rows, k),
            Block::Heading(_, t) => {
                ui.label(RichText::new(t).strong());
            }
            Block::Cover { .. } | Block::PageBreak => {}
        }
    }
}

/// One check: its pill and its text.
pub fn check(ui: &mut Ui, status: Status, text: &str) {
    let p = Palette::current(ui.ctx());
    let (good, warn, bad) = colours(&p, ui);
    let (label, colour) = match status {
        Status::Pass => ("Cumple", good),
        Status::Warn => ("Revisar", warn),
        Status::Fail => ("No cumple", bad),
        Status::Info => ("Dato", p.muted),
    };
    ui.horizontal_top(|ui| {
        ui.add_sized(egui::vec2(78.0, 18.0), |ui: &mut Ui| {
            pill(ui, label, colour)
        });
        ui.add(egui::Label::new(RichText::new(text).size(12.5)).wrap());
    });
}

/// Whether a cell is a figure (to be set to the right).
fn numeric(c: &str) -> bool {
    let t = c.trim();
    !t.is_empty()
        && t.chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit() || ch == '-' || ch == 'Ø')
        && t.chars()
            .filter(|ch| ch.is_alphabetic() && *ch != 'Ø' && *ch != 'x')
            .count()
            <= 3
}

/// A table in stripes, its columns by their widths, figures to the
/// right, the rows that fail with a red mark and their failing cells red.
fn table(ui: &mut Ui, columns: &[(String, f64)], rows: &[Vec<String>], k: usize) {
    let p = Palette::current(ui.ctx());
    let bad = p.bad;
    let total: f64 = columns.iter().map(|c| c.1).sum::<f64>().max(1e-9);
    let width = (ui.available_width() - 16.0).max(200.0);
    let widths: Vec<f32> = columns
        .iter()
        .map(|c| (width as f64 * c.1 / total) as f32 - 8.0)
        .collect();
    egui::ScrollArea::horizontal()
        .id_salt(("report-table", k, ui.scope_id()))
        .show(ui, |ui| {
            egui::Grid::new(ui.scope_id().with(("table", k)))
                .striped(true)
                .spacing(egui::vec2(8.0, 3.0))
                .show(ui, |ui| {
                    ui.label("");
                    for (c, w) in columns.iter().zip(&widths) {
                        ui.allocate_ui_with_layout(
                            egui::vec2(*w, 16.0),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.set_width(*w);
                                ui.add(
                                    egui::Label::new(RichText::new(&c.0).strong().size(11.5))
                                        .truncate(),
                                )
                                .on_hover_text(&c.0);
                            },
                        );
                    }
                    ui.end_row();
                    for r in rows {
                        let fails = r.iter().any(|c| cell_fails(c));
                        ui.label(
                            RichText::new(if fails { "✖" } else { " " })
                                .color(bad)
                                .size(11.0),
                        );
                        for (j, c) in r.iter().enumerate() {
                            let w = widths.get(j).copied().unwrap_or(60.0);
                            let mut text = RichText::new(c).size(12.0);
                            if cell_fails(c) {
                                text = text.color(bad).strong();
                            }
                            let label = egui::Label::new(text).truncate();
                            if numeric(c) {
                                ui.allocate_ui_with_layout(
                                    egui::vec2(w, 16.0),
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.set_width(w);
                                        ui.add(label)
                                    },
                                )
                                .response
                                .on_hover_text(c);
                            } else {
                                ui.allocate_ui_with_layout(
                                    egui::vec2(w, 16.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.set_width(w);
                                        ui.add(label)
                                    },
                                )
                                .response
                                .on_hover_text(c);
                            }
                        }
                        ui.end_row();
                    }
                });
        });
}

/// The whole result: the summary card and the blocks, or the typed text.
pub fn result(
    ui: &mut Ui,
    blocks: &[Block],
    text: &str,
    as_text: &mut bool,
    what: &str,
    salt: &str,
    fails: Option<usize>,
) {
    let mut t = Tally::default();
    t.add(tally(blocks));
    // What the calculation itself counts as failing, when it counts more
    // finely than its blocks (each short passage, not one line for all).
    if let Some(n) = fails {
        t.fail = t.fail.max(n);
    }
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            super::widgets::switch(ui, as_text, "Ver como texto");
        });
    });
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .id_salt(("report", salt))
        .show(ui, |ui| {
            if *as_text {
                ui.add(
                    egui::Label::new(RichText::new(text).monospace().size(11.0))
                        .wrap_mode(egui::TextWrapMode::Extend),
                );
            } else {
                summary(ui, t, what);
                ui.add_space(6.0);
                show(ui, blocks, salt);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_fails_is_counted() {
        let blocks = vec![
            Block::Heading(2, "1 Tramos".into()),
            Block::Table {
                columns: vec![("Tramo".into(), 1.0), ("Basta".into(), 1.0)],
                rows: vec![
                    vec!["V1".into(), "150 × 150".into()],
                    vec!["V2".into(), "100 × 100→150 × 100".into()],
                ],
            },
            Block::Check {
                status: Status::Warn,
                text: "Revisar".into(),
            },
            Block::Check {
                status: Status::Info,
                text: "Dato".into(),
            },
        ];
        let t = tally(&blocks);
        assert_eq!((t.pass, t.warn, t.fail), (0, 1, 1));
        assert!(numeric("24,0") && numeric("Ø150") && !numeric("Cocina"));
        assert_eq!(sections(&blocks).len(), 1);
    }
}
