//! «Manzanas» (N7, `docs/TERRITORIO.md` §2.10): the blocks the streets
//! close, cut into plots along their façade line and filled with buildings
//! of the place by zone, as Zoning Adjuster, Ploppable RICO and Historical
//! Districts do in Cities: Skylines. A locked plot keeps what it has.

use archisim_core::{BlockZone, PlotId, Point2};
use archisim_geom::blocks;
use eframe::egui::{self, ComboBox, Id, Ui};

use super::props::{self, row};
use super::widgets::SwitchExt;
use super::*;

/// The panel «Manzanas».
#[derive(Debug, Clone)]
pub struct BlocksState {
    pub open: bool,
    /// The zone new plots take.
    pub zone: BlockZone,
    /// Frontage and depth of the plots; None: the zone's usual.
    pub frontage: Option<f64>,
    pub depth: Option<f64>,
    /// Picking a block on the plan to cut it into plots.
    pub picking: bool,
    /// Painting the zone over the plots with a brush (N7): its radius,
    /// metres, and the plots painted in the stroke so far.
    pub brush: bool,
    pub brush_radius: f64,
    pub stroke: Vec<archisim_core::PlotId>,
}

impl Default for BlocksState {
    fn default() -> Self {
        BlocksState {
            open: false,
            zone: BlockZone::OldTown,
            frontage: None,
            depth: None,
            picking: false,
            brush: false,
            brush_radius: 8.0,
            stroke: Vec::new(),
        }
    }
}

/// The zone chosen in a list, with its colour.
pub(super) fn zone_combo(ui: &mut Ui, id: Id, zone: &mut Option<BlockZone>, width: f32) -> bool {
    let mut changed = false;
    let text = zone.map_or("Sin zona", |z| z.label());
    ComboBox::from_id_salt(id)
        .width(width)
        .selected_text(text)
        .show_ui(ui, |ui| {
            changed |= ui.selectable_value(zone, None, "Sin zona").changed();
            for z in BlockZone::ALL {
                let [r, g, b] = z.rgb();
                let label = egui::RichText::new(format!("■ {}", z.label()))
                    .color(egui::Color32::from_rgb(r, g, b));
                changed |= ui.selectable_value(zone, Some(z), label).changed();
            }
        });
    changed
}

/// The block of the level under a point, and whether it already has plots.
fn block_at(
    model: &archisim_core::Model,
    level: archisim_core::LevelId,
    p: Point2,
) -> Option<(usize, Vec<Point2>, bool)> {
    let all = blocks::city_blocks(model, level);
    let (k, b) = all
        .into_iter()
        .enumerate()
        .find(|(_, b)| archisim_core::point_in_polygon(p, b))?;
    let has = model
        .plots_on_level(level)
        .any(|x| archisim_core::point_in_polygon(archisim_core::centroid(&x.outline), &b));
    Some((k + 1, b, has))
}

impl ArchiSimApp {
    pub(super) fn blocks_dialog(&mut self, ctx: &egui::Context) {
        if !self.blocks.open {
            return;
        }
        let mut open = true;
        super::widgets::side_sheet(ctx, "Manzanas", 400.0)
            .open(&mut open)
            .show(ctx, |ui| self.blocks_contents(ui));
        self.blocks.open = open;
        if !open {
            self.blocks.picking = false;
        }
    }

    /// A click on the plan while picking a block: its plots.
    pub(super) fn blocks_click(&mut self, p: Point2) {
        let level = self.ed.level;
        let Some((number, block, has)) = block_at(self.ed.model(), level, p) else {
            self.complain("Ahí no hay una manzana: busca un hueco cerrado por calles.");
            return;
        };
        let (zone, f, d) = (self.blocks.zone, self.blocks.frontage, self.blocks.depth);
        let mut ids = Vec::new();
        let mut gone = 0;
        let what = if has {
            "Volver a parcelar manzana"
        } else {
            "Parcelar manzana"
        };
        let r = self.ed.transact(what, |m| {
            if has {
                // Cut again: what is not locked goes, with its buildings.
                (ids, gone) = blocks::replot_block(m, level, &block, number, zone, f, d)?;
            } else {
                ids = blocks::add_block_plots(m, level, &block, number, zone, f, d)?;
            }
            Ok::<(), archisim_core::ModelError>(())
        });
        let done = if has {
            format!(
                "Manzana {number} parcelada de nuevo: {gone} parcelas quitadas y {} de {}; las bloqueadas siguen.",
                ids.len(),
                zone.label().to_lowercase()
            )
        } else {
            format!(
                "Manzana {number}: {} parcelas de {}.",
                ids.len(),
                zone.label().to_lowercase()
            )
        };
        self.report(r, &done);
        self.ed.selection = ids.into_iter().map(Element::Plot).collect();
    }

    fn blocks_contents(&mut self, ui: &mut Ui) {
        props::note(
            ui,
            "Las calles que se cierran forman manzanas. Cada manzana se parte en parcelas por la línea de fachada y se llena con edificios del lugar según su zona. Las parcelas bloqueadas no cambian.",
        );
        let level = self.ed.level;
        let model = self.ed.model();
        let st = &mut self.blocks;
        let mut zone = Some(st.zone);
        row(
            ui,
            "Zona",
            "Lo que se construye en las parcelas nuevas",
            |ui, w| {
                if zone_combo(ui, Id::unique("blocks-zone"), &mut zone, w - 8.0) {
                    if let Some(z) = zone {
                        st.zone = z;
                    }
                }
            },
        );
        let usual_f = blocks::zone_frontage(model, st.zone);
        let usual_d = blocks::zone_depth(model, st.zone);
        row(
            ui,
            "Frente de parcela (mm)",
            "Ancho de cada parcela a la calle",
            |ui, w| {
                if let Some(v) = length_field(
                    ui,
                    Id::unique("blocks-frontage"),
                    st.frontage.unwrap_or(usual_f),
                    w - 8.0,
                ) {
                    st.frontage = Some(v.max(3.0));
                }
            },
        );
        row(
            ui,
            "Fondo de parcela (mm)",
            "Desde la línea de fachada hacia el patio de manzana",
            |ui, w| {
                if let Some(v) = length_field(
                    ui,
                    Id::unique("blocks-depth"),
                    st.depth.unwrap_or(usual_d),
                    w - 8.0,
                ) {
                    st.depth = Some(v.max(3.0));
                }
            },
        );
        if (st.frontage.is_some() || st.depth.is_some())
            && ui.small_button("Las habituales de la zona").clicked()
        {
            st.frontage = None;
            st.depth = None;
        }
        let mut brush_on = false;
        ui.horizontal_wrapped(|ui| {
            if ui
                .selectable_label(st.brush, "Pincel de zona")
                .on_hover_text(
                    "Arrastra sobre las parcelas: toman la zona elegida (las bloqueadas no)",
                )
                .clicked()
            {
                brush_on = true;
            }
            ui.label("Radio");
            ui.add(
                egui::DragValue::new(&mut st.brush_radius)
                    .range(1.0..=100.0)
                    .speed(0.5)
                    .suffix(" m"),
            );
        });
        let plots: Vec<_> = model.plots_on_level(level).collect();
        let zoned = plots.iter().filter(|p| p.zone.is_some()).count();
        let locked = plots.iter().filter(|p| p.locked).count();
        props::note(
            ui,
            &format!(
                "En esta planta: {} parcelas, {zoned} con zona y {locked} bloqueadas.",
                plots.len()
            ),
        );
        let mut act = None;
        ui.horizontal_wrapped(|ui| {
            if ui
                .selectable_label(st.picking, "Parcelar una manzana")
                .on_hover_text(
                    "Clic dentro de una manzana; si ya tiene parcelas, se vuelve a parcelar sin tocar las bloqueadas",
                )
                .clicked()
            {
                act = Some(0);
            }
            if ui
                .button("Parcelar todas")
                .on_hover_text("Todas las manzanas de la planta que aún no tienen parcelas")
                .clicked()
            {
                act = Some(1);
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .button("Llenar las vacías")
                .on_hover_text("Un edificio de su zona en cada parcela sin edificio y sin bloquear")
                .clicked()
            {
                act = Some(2);
            }
            if ui
                .button("Rehacer")
                .on_hover_text(
                    "Quita los edificios de las parcelas sin bloquear y las vuelve a llenar",
                )
                .clicked()
            {
                act = Some(3);
            }
            if ui
                .button("Vaciar")
                .on_hover_text("Quita los edificios de las parcelas sin bloquear")
                .clicked()
            {
                act = Some(4);
            }
        });
        if brush_on {
            let on = !self.blocks.brush;
            self.set_tool(Tool::Arrow);
            self.blocks.picking = false;
            self.blocks.brush = on;
            if on {
                self.say("Arrastra el pincel sobre las parcelas.");
            }
        }
        match act {
            Some(0) => {
                let on = !self.blocks.picking;
                self.set_tool(Tool::Arrow);
                self.blocks.picking = on;
                if on {
                    self.say("Clic dentro de una manzana.");
                }
            }
            Some(1) => {
                let (zone, f, d) = (self.blocks.zone, self.blocks.frontage, self.blocks.depth);
                let mut n = (0, 0);
                let r = self.ed.transact("Parcelar manzanas", |m| {
                    for (k, b) in blocks::city_blocks(m, level).into_iter().enumerate() {
                        let has = m.plots_on_level(level).any(|x| {
                            archisim_core::point_in_polygon(archisim_core::centroid(&x.outline), &b)
                        });
                        if !has {
                            n.0 += 1;
                            n.1 += blocks::add_block_plots(m, level, &b, k + 1, zone, f, d)?.len();
                        }
                    }
                    Ok::<(), archisim_core::ModelError>(())
                });
                let done = match n {
                    (0, _) => "No hay manzanas sin parcelas.".to_string(),
                    (1, p) => format!("1 manzana partida en {p} parcelas."),
                    (b, p) => format!("{b} manzanas partidas en {p} parcelas."),
                };
                self.report(r, &done);
            }
            Some(k) => {
                let mut n = (0, 0);
                let r = self.ed.transact("Llenar parcelas", |m| {
                    if k >= 3 {
                        n.0 = blocks::clear_plots(m, level, None)?;
                    }
                    if k <= 3 {
                        n.1 = blocks::fill_plots(m, level, None, None)?;
                    }
                    Ok::<(), archisim_core::ModelError>(())
                });
                let done = match k {
                    2 => format!("{} edificios nuevos.", n.1),
                    3 => format!("{} edificios quitados y {} puestos.", n.0, n.1),
                    _ => format!("{} edificios quitados.", n.0),
                };
                self.report(r, &done);
            }
            None => {}
        }
    }

    /// The plot's zone and lock, and filling or clearing it alone.
    pub(super) fn plot_zone_group(&mut self, ui: &mut Ui, id: PlotId) {
        let Some(p) = self.ed.model().plot(id).cloned() else {
            return;
        };
        props::group(ui, "plot-zone", "Zona y llenado", |ui| {
            let mut zone = p.zone;
            row(
                ui,
                "Zona",
                "Lo que se construye en ella al llenarla",
                |ui, w| {
                    if zone_combo(ui, Id::unique(("plot-zone", id)), &mut zone, w - 8.0) {
                        let r = self
                            .ed
                            .transact("Zona de la parcela", |m| m.edit_plot(id, |x| x.zone = zone));
                        self.report(r, "Zona cambiada.");
                    }
                },
            );
            let mut locked = p.locked;
            if ui
                .switch(&mut locked, "Bloqueada")
                .on_hover_text("Llenar, rehacer y vaciar las parcelas no la tocan")
                .changed()
            {
                let r = self.ed.transact("Bloquear parcela", |m| {
                    m.edit_plot(id, |x| x.locked = locked)
                });
                self.report(
                    r,
                    if locked {
                        "Parcela bloqueada."
                    } else {
                        "Parcela desbloqueada."
                    },
                );
            }
            let built = blocks::plot_built(self.ed.model(), &p);
            ui.add_enabled_ui(!p.locked, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!built && p.zone.is_some(), egui::Button::new("Llenar"))
                        .clicked()
                    {
                        let mut put = None;
                        let r = self.ed.transact("Llenar parcela", |m| {
                            put = blocks::fill_plot(m, id, None)?;
                            Ok::<(), archisim_core::ModelError>(())
                        });
                        let done = if put.is_some() {
                            "Edificio puesto."
                        } else {
                            "No cabe un edificio de su zona."
                        };
                        self.report(r, done);
                    }
                    if ui.add_enabled(built, egui::Button::new("Vaciar")).clicked() {
                        let r = self.ed.transact("Vaciar parcela", |m| {
                            blocks::clear_plots(m, p.level, Some(&[id]))?;
                            Ok::<(), archisim_core::ModelError>(())
                        });
                        self.report(r, "Edificio quitado.");
                    }
                });
            });
        });
    }

    /// The plots under the brush at `p` (N7): taken into the stroke.
    pub(super) fn zone_brush_at(&mut self, p: Point2) {
        let level = self.ed.level;
        let r = self.blocks.brush_radius;
        let under: Vec<archisim_core::PlotId> = self
            .ed
            .model()
            .plots_on_level(level)
            .filter(|x| !x.locked)
            .filter(|x| {
                archisim_core::point_in_polygon(p, &x.outline)
                    || x.outline.iter().any(|q| q.distance(p) <= r)
                    || archisim_core::centroid(&x.outline).distance(p) <= r
            })
            .map(|x| x.id)
            .collect();
        for id in under {
            if !self.blocks.stroke.contains(&id) {
                self.blocks.stroke.push(id);
            }
        }
    }

    /// The stroke ends: its plots take the zone, in one step.
    pub(super) fn zone_brush_done(&mut self) {
        let ids = std::mem::take(&mut self.blocks.stroke);
        if ids.is_empty() {
            return;
        }
        let zone = self.blocks.zone;
        let n = ids.len();
        let r = self.ed.transact("Pincel de zona", |m| {
            for id in &ids {
                m.edit_plot(*id, |x| x.zone = Some(zone))?;
            }
            Ok::<(), archisim_core::ModelError>(())
        });
        self.report(
            r,
            &format!(
                "{n} parcela{} con zona {}. «Rehacer» cambia sus edificios.",
                if n == 1 { "" } else { "s" },
                zone.label().to_lowercase()
            ),
        );
    }

    /// The brush and the plots of its stroke, over the plan.
    pub(super) fn draw_zone_brush(&self, painter: &egui::Painter, rect: egui::Rect) {
        if !self.blocks.brush {
            return;
        }
        let rgb = self.blocks.zone.rgb();
        let ink = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
        let model = self.ed.model();
        for id in &self.blocks.stroke {
            if let Some(x) = model.plot(*id) {
                let pts: Vec<egui::Pos2> = x
                    .outline
                    .iter()
                    .map(|q| self.view.to_screen(rect, *q))
                    .collect();
                painter.add(egui::Shape::convex_polygon(
                    pts.clone(),
                    ink.gamma_multiply(0.35),
                    egui::Stroke::NONE,
                ));
                painter.add(egui::Shape::closed_line(pts, egui::Stroke::new(1.5, ink)));
            }
        }
        if let Some(c) = self.cursor {
            let at = self.view.to_screen(rect, c.point);
            let r = (self.blocks.brush_radius * self.view.zoom) as f32;
            painter.circle_stroke(at, r, egui::Stroke::new(1.5, ink));
        }
    }
}
