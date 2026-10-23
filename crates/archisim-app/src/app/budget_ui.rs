//! The budget in the Mediciones tab (bloque 7 de la revisión del
//! 22/10/2026): the items of the measurements by chapters, each with its
//! price — from a price bank in BC3 opened here, or typed — and its amount;
//! the BC3 exported carries them to Presto. Each price can show how it is
//! made up (its pieces, from the bank, whose yields and prices can be
//! changed here), and the end of the table goes from the execution budget
//! to the contract one with the overheads, profit and VAT of the project.

use archisim_core::PriceConcept;
use archisim_export::bc3_read::{read_bank, unit_key, Bank};
use eframe::egui::{self, Ui};

use super::*;

/// The bank opened and the item being given a concept.
#[derive(Default)]
pub struct BudgetState {
    pub bank: Option<Bank>,
    /// The bank of the preferences was tried already.
    tried_saved: bool,
    pub pick: Option<Pick>,
    /// Items whose decomposition is shown, by key.
    pub open: std::collections::BTreeSet<String>,
}

/// Choosing a concept of the bank for an item.
#[derive(Debug, Clone, PartialEq)]
pub struct Pick {
    pub key: String,
    pub unit: String,
    pub summary: String,
    pub query: String,
}

/// `1234.5` as `1.234,50`.
pub fn euros(v: f64) -> String {
    let cents = (v * 100.0).round() as i64;
    let (neg, cents) = (cents < 0, cents.abs());
    let whole = (cents / 100).to_string();
    let mut grouped = String::new();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(c);
    }
    format!("{}{grouped},{:02}", if neg { "-" } else { "" }, cents % 100)
}

fn quantity(v: f64) -> String {
    format!("{:.2}", v).replace('.', ",")
}

/// A number typed in Spanish («1.234,5»), or `None`.
fn parse_es(text: &str) -> Option<f64> {
    let t = text.trim().trim_end_matches('%').trim();
    let t = if t.contains(',') {
        t.replace('.', "").replace(',', ".")
    } else {
        t.to_string()
    };
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

fn percent(v: f64) -> String {
    let p = (v * 1000.0).round() / 10.0;
    format!("{p}").replace('.', ",")
}

impl ArchiSimApp {
    /// Opens a price bank and remembers it.
    pub(super) fn open_price_bank(&mut self, path: &std::path::Path) {
        match std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|b| read_bank(&b).map_err(|e| e.to_string()))
        {
            Ok(bank) => {
                let items = bank.search("", None, usize::MAX).len();
                self.say(format!(
                    "Banco de precios «{}»: {items} partidas.",
                    bank.name
                ));
                self.budget.bank = Some(bank);
                self.prefs.price_bank = Some(path.display().to_string());
            }
            Err(e) => self.complain(format!("No se pudo leer el banco de precios: {e}.")),
        }
    }

    fn choose_price_bank(&mut self) {
        let mut d = rfd::FileDialog::new()
            .set_title("Abrir banco de precios (BC3)")
            .add_filter("BC3 (FIEBDC-3)", &["bc3", "BC3"]);
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        if let Some(path) = d.pick_file() {
            self.open_price_bank(&path);
        }
    }

    /// Gives an item a concept (or a price), or takes it away.
    pub(super) fn set_item_price(&mut self, key: &str, concept: Option<PriceConcept>) {
        let label = if concept.is_some() {
            "Precio de partida"
        } else {
            "Quitar precio"
        };
        let key = key.to_string();
        let r = self.ed.transact(label, |m| m.set_price(&key, concept));
        if let Err(e) = r {
            self.complain(e);
        }
    }

    /// The budget: chapters, items, prices and amounts.
    pub(super) fn budget_section(&mut self, ui: &mut Ui) {
        let p = super::theme::Palette::current(ui.ctx());
        if !self.budget.tried_saved {
            self.budget.tried_saved = true;
            if let Some(path) = self.prefs.price_bank.clone() {
                let path = std::path::PathBuf::from(path);
                if path.is_file() {
                    if let Ok(bank) = std::fs::read(&path)
                        .map_err(|e| e.to_string())
                        .and_then(|b| read_bank(&b).map_err(|e| e.to_string()))
                    {
                        self.budget.bank = Some(bank);
                    }
                }
            }
        }
        let shown = self.ed.shown();
        let model: &archisim_core::Model = &shown;
        let chapters = archisim_plan::schedules::measurements(model);
        let mut actions: Vec<(String, Option<PriceConcept>)> = Vec::new();
        let mut open_bank = false;
        let mut pick = None;
        let mut toggles: Vec<String> = Vec::new();
        let mut budget_change: Option<archisim_core::BudgetSettings> = None;
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Presupuesto")
                    .font(super::widgets::strong_font(13.0))
                    .color(p.text),
            );
            ui.add_space(10.0);
            match &self.budget.bank {
                Some(b) => {
                    ui.label(egui::RichText::new(format!("Banco: {}", b.name)).color(p.muted));
                }
                None => {
                    ui.label(egui::RichText::new("Sin banco de precios").color(p.muted));
                }
            }
            if ui
                .button("Abrir banco BC3…")
                .on_hover_text(
                    "Un banco de precios en BC3 (los de las comunidades autónomas, el de tu estudio, uno exportado de Presto) para elegir de él la partida de cada tipo",
                )
                .clicked()
            {
                open_bank = true;
            }
        });
        ui.add_space(4.0);
        let mut grand = 0.0;
        egui::Grid::new("budget-grid")
            .striped(true)
            .min_col_width(30.0)
            .spacing([14.0, 4.0])
            .show(ui, |ui| {
                for h in [
                    "Código",
                    "Ud",
                    "Resumen",
                    "Cantidad",
                    "Precio (€)",
                    "Importe (€)",
                    "",
                ] {
                    ui.label(egui::RichText::new(h).strong());
                }
                ui.end_row();
                for (code, title, items) in &chapters {
                    let mut chapter_total = 0.0;
                    ui.label(egui::RichText::new(code.to_string()).strong());
                    ui.label("");
                    ui.label(egui::RichText::new(*title).strong());
                    ui.end_row();
                    for it in items {
                        let concept = model.price_of(&it.key);
                        let qty = it.total();
                        let price = concept.map_or(0.0, |c| c.price);
                        let amount = qty * price;
                        chapter_total += amount;
                        let code = concept
                            .map(|c| c.code.as_str())
                            .filter(|c| !c.is_empty())
                            .unwrap_or(&it.code);
                        ui.label(code);
                        ui.label(it.unit);
                        let summary = concept
                            .map(|c| c.summary.as_str())
                            .filter(|s| !s.is_empty())
                            .unwrap_or(&it.summary);
                        let r = ui.label(summary);
                        if let Some(c) = concept.filter(|c| !c.text.is_empty()) {
                            r.on_hover_text(format!(
                                "{}\n\nTipo del modelo: {}",
                                c.text, it.summary
                            ));
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(quantity(qty))
                        });
                        let shown_price = if concept.is_some() {
                            format!("{price:.2}").replace('.', ",")
                        } else {
                            String::new()
                        };
                        if let Some(text) = super::text_edit_committed(
                            ui,
                            egui::Id::unique(("budget-price", &it.key)),
                            &shown_price,
                            70.0,
                        ) {
                            let t = text.trim().replace('.', "").replace(',', ".");
                            if t.is_empty() {
                                actions.push((it.key.clone(), None));
                            } else if let Ok(v) = t.parse::<f64>() {
                                let mut c = concept.cloned().unwrap_or_else(|| PriceConcept {
                                    unit: it.unit.to_string(),
                                    ..Default::default()
                                });
                                c.price = v.max(0.0);
                                // A price typed by hand has no pieces.
                                c.pieces.clear();
                                actions.push((it.key.clone(), Some(c)));
                            }
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(if concept.is_some() {
                                euros(amount)
                            } else {
                                String::new()
                            })
                        });
                        ui.horizontal(|ui| {
                            let unit_differs = concept.is_some_and(|c| {
                                !c.unit.is_empty() && unit_key(&c.unit) != unit_key(it.unit)
                            });
                            if unit_differs {
                                ui.label(egui::RichText::new("⚠").color(p.bad))
                                    .on_hover_text(format!(
                                        "La partida se mide en {} y el modelo da {}",
                                        concept.map_or("", |c| c.unit.as_str()),
                                        it.unit
                                    ));
                            }
                            if let Some(c) = concept.filter(|c| !c.pieces.is_empty()) {
                                let open = self.budget.open.contains(&it.key);
                                if ui
                                    .small_button(if open { "▾" } else { "▸" })
                                    .on_hover_text(format!(
                                        "Descomposición: {} piezas",
                                        c.pieces.len()
                                    ))
                                    .clicked()
                                {
                                    toggles.push(it.key.clone());
                                }
                            }
                            if ui
                                .small_button("Partida…")
                                .on_hover_text("Elegir la partida del banco de precios")
                                .clicked()
                            {
                                pick = Some(Pick {
                                    key: it.key.clone(),
                                    unit: it.unit.to_string(),
                                    summary: it.summary.clone(),
                                    query: String::new(),
                                });
                            }
                            if concept.is_some()
                                && ui
                                    .small_button("✕")
                                    .on_hover_text("Quitar el precio")
                                    .clicked()
                            {
                                actions.push((it.key.clone(), None));
                            }
                        });
                        ui.end_row();
                        // Its pieces, under it.
                        let Some(c) = concept
                            .filter(|c| !c.pieces.is_empty() && self.budget.open.contains(&it.key))
                        else {
                            continue;
                        };
                        let amounts = archisim_core::piece_amounts(&c.pieces);
                        let small =
                            |t: &str| egui::RichText::new(t.to_string()).small().color(p.muted);
                        for (i, (piece, a)) in c.pieces.iter().zip(&amounts).enumerate() {
                            ui.label(small(&format!("   {}", piece.code)));
                            ui.label(small(&piece.unit));
                            ui.label(small(&format!("{} · {}", piece.kind_name(), piece.summary)));
                            let mut edited: Option<archisim_core::PricePiece> = None;
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if let Some(t) = super::text_edit_committed(
                                        ui,
                                        egui::Id::unique(("piece-q", &it.key, i)),
                                        &format!("{}", (piece.quantity * 1000.0).round() / 1000.0)
                                            .replace('.', ","),
                                        60.0,
                                    ) {
                                        if let Some(v) = parse_es(&t) {
                                            edited = Some(archisim_core::PricePiece {
                                                quantity: v.max(0.0),
                                                ..piece.clone()
                                            });
                                        }
                                    }
                                },
                            );
                            if piece.is_percent() {
                                ui.label(small("%"));
                            } else if let Some(t) = super::text_edit_committed(
                                ui,
                                egui::Id::unique(("piece-p", &it.key, i)),
                                &format!("{:.2}", piece.price).replace('.', ","),
                                70.0,
                            ) {
                                if let Some(v) = parse_es(&t) {
                                    edited = Some(archisim_core::PricePiece {
                                        price: v.max(0.0),
                                        ..piece.clone()
                                    });
                                }
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| ui.label(small(&euros(*a))),
                            );
                            ui.label("");
                            ui.end_row();
                            if let Some(np) = edited {
                                let mut nc = c.clone();
                                nc.pieces[i] = np;
                                nc.reprice();
                                actions.push((it.key.clone(), Some(nc)));
                            }
                        }
                        let total: f64 = amounts.iter().sum();
                        if (total - c.price).abs() >= 0.005 {
                            ui.label("");
                            ui.label("");
                            ui.label(
                                egui::RichText::new(format!(
                                    "   La descomposición suma {} € y el precio es {} €",
                                    euros(total),
                                    euros(c.price)
                                ))
                                .small()
                                .color(p.bad),
                            );
                            ui.end_row();
                        }
                    }
                    grand += chapter_total;
                    ui.label("");
                    ui.label("");
                    ui.label(egui::RichText::new(format!("Total {title}")).italics());
                    ui.label("");
                    ui.label("");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(euros(chapter_total)).italics())
                    });
                    ui.end_row();
                }
                // From the execution budget to the contract one.
                let settings = model.budget_settings();
                let t = settings.totals(grand);
                let total_row = |ui: &mut Ui, name: &str, v: f64, strong: bool| {
                    ui.label("");
                    ui.label("");
                    let n = egui::RichText::new(name.to_string());
                    ui.label(if strong { n.strong() } else { n });
                    ui.label("");
                    ui.label("");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let e = egui::RichText::new(euros(v));
                        ui.label(if strong { e.strong() } else { e })
                    });
                    ui.end_row();
                };
                total_row(
                    ui,
                    "Presupuesto de ejecución material (PEM)",
                    t.execution,
                    true,
                );
                let mut new_settings = settings;
                for (i, (name, frac, v)) in [
                    ("Gastos generales", settings.overheads, t.overheads),
                    ("Beneficio industrial", settings.profit, t.profit),
                ]
                .into_iter()
                .enumerate()
                {
                    ui.label("");
                    ui.label("");
                    ui.label(name);
                    ui.label("");
                    ui.horizontal(|ui| {
                        if let Some(text) = super::text_edit_committed(
                            ui,
                            egui::Id::unique(("budget-pct", i)),
                            &percent(frac),
                            44.0,
                        ) {
                            if let Some(x) = parse_es(&text) {
                                let x = (x / 100.0).clamp(0.0, 1.0);
                                if i == 0 {
                                    new_settings.overheads = x;
                                } else {
                                    new_settings.profit = x;
                                }
                            }
                        }
                        ui.label("%");
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(euros(v))
                    });
                    ui.end_row();
                }
                total_row(
                    ui,
                    "Presupuesto de ejecución por contrata (PEC)",
                    t.contract,
                    true,
                );
                ui.label("");
                ui.label("");
                ui.label("IVA");
                ui.label("");
                ui.horizontal(|ui| {
                    if let Some(text) = super::text_edit_committed(
                        ui,
                        egui::Id::unique("budget-vat"),
                        &percent(settings.vat),
                        44.0,
                    ) {
                        if let Some(x) = parse_es(&text) {
                            new_settings.vat = (x / 100.0).clamp(0.0, 1.0);
                        }
                    }
                    ui.label("%");
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(euros(t.vat))
                });
                ui.end_row();
                total_row(ui, "Presupuesto total (PEC con IVA)", t.total, true);
                if new_settings != settings {
                    budget_change = Some(new_settings);
                }
            });
        if chapters.is_empty() {
            ui.label(
                egui::RichText::new("Nada que medir todavía.")
                    .small()
                    .color(p.muted),
            );
        }
        drop(shown);
        for key in toggles {
            if !self.budget.open.remove(&key) {
                self.budget.open.insert(key);
            }
        }
        for (key, c) in actions {
            self.set_item_price(&key, c);
        }
        if let Some(b) = budget_change {
            let r = self.ed.transact("Gastos generales, beneficio e IVA", |m| {
                m.set_budget_settings(b)
            });
            if let Err(e) = r {
                self.complain(e);
            }
        }
        if open_bank {
            self.choose_price_bank();
        }
        if let Some(pk) = pick {
            if self.budget.bank.is_none() {
                self.choose_price_bank();
            }
            if self.budget.bank.is_some() {
                self.budget.pick = Some(pk);
            }
        }
    }

    /// The window to choose a concept of the bank.
    pub(super) fn price_pick_window(&mut self, ctx: &egui::Context) {
        let Some(mut pk) = self.budget.pick.clone() else {
            return;
        };
        let Some(bank) = &self.budget.bank else {
            self.budget.pick = None;
            return;
        };
        let mut open = true;
        let mut chosen: Option<PriceConcept> = None;
        egui::Window::new("Partida del banco de precios")
            .open(&mut open)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .default_size([680.0, 460.0])
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(format!("Para: {} (medido en {})", pk.summary, pk.unit));
                ui.horizontal(|ui| {
                    ui.label("Buscar");
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut pk.query)
                            .hint_text("p. ej. ladrillo hueco doble")
                            .desired_width(320.0),
                    );
                    if !r.has_focus() && pk.query.is_empty() {
                        r.request_focus();
                    }
                });
                ui.add_space(4.0);
                let found = bank.search(&pk.query, Some(&pk.unit), 200);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Grid::new("bank-results")
                            .striped(true)
                            .spacing([12.0, 4.0])
                            .show(ui, |ui| {
                                for c in found {
                                    ui.label(&c.code);
                                    let same = unit_key(&c.unit) == unit_key(&pk.unit);
                                    let unit = egui::RichText::new(&c.unit);
                                    ui.label(if same { unit.strong() } else { unit });
                                    let path = bank.path_of(&c.code).join(" › ");
                                    let mut hover = c.text.clone();
                                    if !path.is_empty() {
                                        hover = format!("{path}\n\n{hover}");
                                    }
                                    ui.label(&c.summary).on_hover_text(hover);
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| ui.label(euros(c.price)),
                                    );
                                    if ui.small_button("Asignar").clicked() {
                                        chosen = bank.price_concept(&c.code);
                                    }
                                    ui.end_row();
                                }
                            });
                    });
            });
        if let Some(c) = chosen {
            let key = pk.key.clone();
            self.budget.pick = None;
            self.set_item_price(&key, Some(c));
        } else if open {
            self.budget.pick = Some(pk);
        } else {
            self.budget.pick = None;
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn euros_are_written_the_spanish_way() {
        assert_eq!(super::euros(1234.5), "1.234,50");
        assert_eq!(super::euros(0.0), "0,00");
        assert_eq!(super::euros(1234567.891), "1.234.567,89");
        assert_eq!(super::euros(12.0), "12,00");
    }
}
