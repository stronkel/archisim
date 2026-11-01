//! Buildings of the surroundings in the app (U6): the tile «Edificio del
//! entorno» of Exterior (the mass tool by floors with a style: a click in
//! a plot fills its buildable area, or the contour is drawn), the style
//! chosen in cards painted as a little façade, and the properties of one
//! or many dressed masses (their style changed all at once).

use archisim_core::{BuildingStyle, Closure, Dress, EnvRoof, MassId, Point2};
use eframe::egui::{self, Color32, CornerRadius, Id, Rect, Stroke, Ui};

use super::props::{self, choice, group, row};
use super::*;
use crate::editor::{MassMode, MassWhat};

/// The style cards, by group.
const GROUPS: [(&str, &[BuildingStyle]); 3] = [
    (
        "Casas",
        &[
            BuildingStyle::Andalusian,
            BuildingStyle::Courtyard,
            BuildingStyle::Terraced,
            BuildingStyle::SemiDetached,
            BuildingStyle::Detached,
        ],
    ),
    (
        "Bloques",
        &[
            BuildingStyle::HistoricCentre,
            BuildingStyle::Ensanche,
            BuildingStyle::ShopAndFlats,
            BuildingStyle::Block1960s,
            BuildingStyle::CoastalHotel,
        ],
    ),
    (
        "Otros usos",
        &[
            BuildingStyle::Offices,
            BuildingStyle::Facility,
            BuildingStyle::Warehouse,
        ],
    ),
];

fn colour(rgb: [u8; 3]) -> Color32 {
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

/// A little façade of a style: its wall, windows, closures and roof in its
/// colours.
pub(super) fn paint_style(ui: &Ui, r: Rect, style: BuildingStyle) {
    let painter = ui.painter_at(r);
    let f = style.finishes();
    let [wall, trim, roof, joinery, glass, metal] = f.map(|x| colour(x.2));
    let d = Dress::usual(
        style,
        archisim_core::DressMaterials {
            wall: archisim_core::MaterialId(0),
            trim: archisim_core::MaterialId(0),
            roof: archisim_core::MaterialId(0),
            joinery: archisim_core::MaterialId(0),
            glass: archisim_core::MaterialId(0),
            metal: archisim_core::MaterialId(0),
        },
    );
    let (floors, _) = style.usual_floors();
    let floors = floors.min(8) as f32;
    let r = r.shrink(5.0);
    let tiled = d.roof != EnvRoof::Flat;
    let roof_h = if tiled { r.height() * 0.22 } else { 0.0 };
    let body = Rect::from_min_max(egui::pos2(r.left() + 6.0, r.top() + roof_h), r.max);
    let fh = body.height() / floors;
    painter.rect_filled(body, CornerRadius::ZERO, wall);
    if tiled {
        painter.add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(body.left() - 3.0, body.top()),
                egui::pos2(body.right() + 3.0, body.top()),
                egui::pos2(body.center().x, r.top()),
            ],
            roof,
            Stroke::NONE,
        ));
    }
    if d.plinth > 0.0 {
        let h = (fh * (d.plinth / 3.0) as f32).max(2.0);
        painter.rect_filled(
            Rect::from_min_max(egui::pos2(body.left(), body.bottom() - h), body.max),
            CornerRadius::ZERO,
            trim,
        );
    }
    if d.cornice {
        painter.rect_filled(
            Rect::from_min_size(
                egui::pos2(body.left() - 2.0, body.top()),
                egui::vec2(body.width() + 4.0, 2.5),
            ),
            CornerRadius::ZERO,
            trim,
        );
    }
    if d.closure == Closure::CurtainWall {
        painter.rect_filled(
            body.shrink2(egui::vec2(0.0, 2.0)),
            CornerRadius::ZERO,
            glass,
        );
        let k = 7;
        for j in 0..=k {
            let x = body.left() + body.width() * j as f32 / k as f32;
            painter.line_segment(
                [egui::pos2(x, body.top()), egui::pos2(x, body.bottom())],
                Stroke::new(1.0, joinery),
            );
        }
        for i in 1..floors as usize {
            let y = body.bottom() - fh * i as f32;
            painter.line_segment(
                [egui::pos2(body.left(), y), egui::pos2(body.right(), y)],
                Stroke::new(2.0, wall),
            );
        }
        return;
    }
    let bays = ((body.width() / (d.bay as f32 * 5.0)).floor() as usize).clamp(1, 6);
    let bw = body.width() / bays as f32;
    let ww = (bw * (d.window_width / d.bay) as f32).min(bw - 2.0);
    for i in 0..floors as usize {
        let y0 = body.bottom() - fh * (i + 1) as f32;
        let shop = i == 0 && d.shop;
        let balcony = i > 0 && matches!(d.closure, Closure::Balconies | Closure::Terraces);
        if d.closure == Closure::Terraces && i > 0 {
            painter.rect_filled(
                Rect::from_min_size(
                    egui::pos2(body.left() - 2.0, y0 + fh - 3.0),
                    egui::vec2(body.width() + 4.0, 3.0),
                ),
                CornerRadius::ZERO,
                if style == BuildingStyle::Block1960s {
                    wall
                } else {
                    metal
                },
            );
        }
        for b in 0..bays {
            let cx = body.left() + bw * (b as f32 + 0.5);
            let (w, top, bottom) = if shop {
                (bw - 3.0, y0 + fh * 0.25, y0 + fh)
            } else if balcony {
                (ww, y0 + fh * 0.2, y0 + fh - 1.0)
            } else {
                (ww, y0 + fh * 0.25, y0 + fh * 0.75)
            };
            let win = Rect::from_min_max(
                egui::pos2(cx - w / 2.0, top),
                egui::pos2(cx + w / 2.0, bottom),
            );
            painter.rect_filled(win, CornerRadius::ZERO, glass);
            painter.rect_stroke(
                win,
                CornerRadius::ZERO,
                Stroke::new(1.0, joinery),
                egui::StrokeKind::Inside,
            );
            if d.closure == Closure::Grilles {
                for k in 1..4 {
                    let x = win.left() + win.width() * k as f32 / 4.0;
                    painter.line_segment(
                        [egui::pos2(x, win.top()), egui::pos2(x, win.bottom())],
                        Stroke::new(0.8, metal),
                    );
                }
            }
            if balcony && d.closure == Closure::Balconies {
                painter.rect_filled(
                    Rect::from_min_max(
                        egui::pos2(win.left() - 2.0, win.bottom() - fh * 0.3),
                        egui::pos2(win.right() + 2.0, win.bottom() + 1.0),
                    ),
                    CornerRadius::ZERO,
                    metal.gamma_multiply(0.8),
                );
            }
        }
    }
}

/// Style cards: the one chosen as a wide button, all of them under it by
/// group. The style picked.
pub(super) fn style_cards(
    ui: &mut Ui,
    width: f32,
    current: BuildingStyle,
) -> Option<BuildingStyle> {
    let groups: Vec<(String, Vec<String>)> = GROUPS
        .iter()
        .map(|(g, list)| {
            (
                g.to_string(),
                list.iter().map(|s| s.name().to_string()).collect(),
            )
        })
        .collect();
    let picked =
        super::cards::grouped_cards(ui, width, &groups, current.name(), &mut |ui, r, n| {
            if let Some(s) = BuildingStyle::ALL.into_iter().find(|s| s.name() == n) {
                paint_style(ui, r, s);
            }
        })?;
    BuildingStyle::ALL.into_iter().find(|s| s.name() == picked)
}

impl ArchiSimApp {
    /// The tile «Edificio del entorno»: the mass tool by floors with a
    /// style.
    pub(super) fn env_building_pick(&mut self) {
        let style = self
            .mass_settings
            .style
            .unwrap_or(BuildingStyle::Andalusian);
        self.set_env_style(style);
        let s = &mut self.mass_settings;
        s.what = MassWhat::Mass;
        if s.mode == MassMode::Path || s.mode.is_form() {
            s.mode = MassMode::Polygon;
        }
        self.concept.finish();
        self.set_tool(Tool::Mass);
    }

    /// Whether the tool in use is the tile «Edificio del entorno».
    pub(super) fn env_building_picked(&self) -> bool {
        self.tool == Tool::Mass
            && self.mass_settings.what == MassWhat::Mass
            && self.mass_settings.style.is_some()
    }

    fn set_env_style(&mut self, style: BuildingStyle) {
        let (floors, fh) = style.usual_floors();
        let s = &mut self.mass_settings;
        s.style = Some(style);
        s.floors = floors as usize;
        s.floor_height = fh;
    }

    /// With a style, a click in a plot puts the building on its buildable
    /// area. True if it did (or tried).
    pub(super) fn env_building_click(&mut self, p: Point2) -> bool {
        let s = self.mass_settings;
        let Some(style) = s.style else {
            return false;
        };
        if s.what != MassWhat::Mass || !self.env_on_plots {
            return false;
        }
        let level = self.ed.level;
        let plot = self
            .ed
            .model()
            .plots_on_level(level)
            .find(|x| archisim_core::point_in_polygon(p, &x.outline))
            .map(|x| x.id);
        let Some(plot) = plot else {
            self.say(
                "Haz clic dentro de una parcela, o apaga «En la parcela» para dibujar el contorno.",
            );
            return true;
        };
        let Some(outline) = self.ed.model().plot_outline_for_building(plot) else {
            self.complain("La parcela no deja sitio para edificar con su retranqueo.");
            return true;
        };
        let r = self.ed.transact("Edificio del entorno", |m| {
            crate::editor::add_mass_in(m, &s, level, outline)
        });
        if let Ok(id) = &r {
            self.ed.selection = [Element::Mass(*id)].into_iter().collect();
        }
        self.report(r.map(|_| ()), &format!("{} en la parcela.", style.name()));
        true
    }

    /// The tool's group: the style, and whether a click fills a plot.
    pub(super) fn env_tool_group(&mut self, ui: &mut Ui) {
        let Some(style) = self.mass_settings.style else {
            return;
        };
        let mut picked = None;
        let mut on = self.env_on_plots;
        group(ui, "tool-env", "Edificio del entorno", |ui| {
            row(
                ui,
                "Estilo",
                "Cómo son sus fachadas y su cubierta",
                |ui, w| {
                    picked = style_cards(ui, w - 8.0, style);
                },
            );
            row(
                ui,
                "En la parcela",
                "Un clic en una parcela lo pone en su superficie edificable; apagado, se dibuja el contorno",
                |ui, w| {
                    on = choice(
                        ui,
                        "tool-env-plot",
                        w,
                        on,
                        &[(true, "Clic en la parcela"), (false, "Dibujar el contorno")],
                    );
                },
            );
            if ui
                .button("Volumen sin estilo")
                .on_hover_text("Una masa por plantas de Concepto, sin fachadas")
                .clicked()
            {
                self.mass_settings.style = None;
            }
        });
        self.env_on_plots = on;
        if let Some(s) = picked {
            self.set_env_style(s);
        }
    }

    /// The group of one or many dressed masses: their style, and the
    /// measurements of the first (given to all).
    pub(super) fn env_building_group(&mut self, ui: &mut Ui, ids: &[MassId]) {
        let model = self.ed.model();
        let Some(first) = ids.iter().find_map(|i| model.mass(*i)) else {
            return;
        };
        let undressed = first.dress.is_none();
        let d = first.dress.unwrap_or_else(|| {
            Dress::usual(
                BuildingStyle::ShopAndFlats,
                archisim_core::DressMaterials {
                    wall: first.material,
                    trim: first.material,
                    roof: first.material,
                    joinery: first.material,
                    glass: first.material,
                    metal: first.material,
                },
            )
        });
        let mut style = None;
        let mut edit: Option<Dress> = None;
        let mut undress = false;
        let title = if ids.len() > 1 {
            format!("Edificio del entorno · {} a la vez", ids.len())
        } else {
            "Edificio del entorno".to_string()
        };
        group(ui, "env-building", &title, |ui| {
            row(
                ui,
                "Estilo",
                "Cambia fachadas, cubierta y acabados de todos los seleccionados",
                |ui, w| {
                    if undressed {
                        if ui
                            .add_sized([w - 8.0, 22.0], egui::Button::new("Darle un estilo…"))
                            .clicked()
                        {
                            style = Some(BuildingStyle::ShopAndFlats);
                        }
                    } else {
                        style = style_cards(ui, w - 8.0, d.style);
                    }
                },
            );
            if undressed {
                return;
            }
            let mut x = d;
            let mut changed = false;
            let who = ids[0];
            let metres = |ui: &mut Ui, label: &str, hint: &str, key: &str, v: &mut f64| {
                let mut c = false;
                row(ui, label, hint, |ui, w| {
                    if let Some(n) = length_field(ui, Id::unique(("env", key, who)), *v, w - 8.0) {
                        *v = n;
                        c = true;
                    }
                });
                c
            };
            changed |= metres(
                ui,
                "Ritmo de huecos (mm)",
                "Distancia entre ejes de ventanas",
                "bay",
                &mut x.bay,
            );
            changed |= metres(ui, "Ancho de ventana (mm)", "", "ww", &mut x.window_width);
            changed |= metres(ui, "Alto de ventana (mm)", "", "wh", &mut x.window_height);
            changed |= metres(
                ui,
                "Antepecho (mm)",
                "Sobre cada planta",
                "sill",
                &mut x.sill,
            );
            row(
                ui,
                "Huecos",
                "Lo que cierra las ventanas por fuera",
                |ui, w| {
                    let c = choice(
                        ui,
                        "env-closure",
                        w,
                        x.closure,
                        &Closure::ALL.map(|c| (c, c.name())),
                    );
                    if c != x.closure {
                        x.closure = c;
                        changed = true;
                    }
                },
            );
            if matches!(x.closure, Closure::Balconies | Closure::Terraces) {
                changed |= metres(
                    ui,
                    "Vuelo (mm)",
                    "De balcones y terrazas",
                    "depth",
                    &mut x.balcony_depth,
                );
            }
            row(ui, "Cubierta", "", |ui, w| {
                let c = choice(
                    ui,
                    "env-roof",
                    w,
                    x.roof,
                    &EnvRoof::ALL.map(|c| (c, c.name())),
                );
                if c != x.roof {
                    x.roof = c;
                    changed = true;
                }
            });
            if x.roof != EnvRoof::Flat {
                row(ui, "Pendiente (%)", "", |ui, w| {
                    let mut p = (x.pitch * 100.0).round() as i32;
                    if ui
                        .add_sized([w - 8.0, 20.0], egui::DragValue::new(&mut p).range(5..=120))
                        .changed()
                    {
                        x.pitch = p as f64 / 100.0;
                        changed = true;
                    }
                });
            }
            changed |= metres(
                ui,
                "Zócalo (mm)",
                "Cero: sin zócalo",
                "plinth",
                &mut x.plinth,
            );
            row(
                ui,
                "Bajo comercial",
                "Escaparates y rótulo en la planta baja",
                |ui, w| {
                    let c = choice(ui, "env-shop", w, x.shop, &[(true, "Sí"), (false, "No")]);
                    if c != x.shop {
                        x.shop = c;
                        changed = true;
                    }
                },
            );
            row(ui, "Cornisa", "", |ui, w| {
                let c = choice(
                    ui,
                    "env-cornice",
                    w,
                    x.cornice,
                    &[(true, "Sí"), (false, "No")],
                );
                if c != x.cornice {
                    x.cornice = c;
                    changed = true;
                }
            });
            changed |= metres(
                ui,
                "Patio (mm)",
                "Fondo de las crujías alrededor del patio; cero: sin patio",
                "patio",
                &mut x.patio,
            );
            if changed {
                edit = Some(x);
            }
            props::note(
                ui,
                "Las fachadas que tocan otro volumen son medianeras: ciegas, y la cubierta no vierte sobre ellas.",
            );
            if ui
                .button("Quitar el estilo")
                .on_hover_text("Vuelve a ser una masa por plantas de Concepto")
                .clicked()
            {
                undress = true;
            }
        });
        let list = ids.to_vec();
        if let Some(s) = style {
            let r = self
                .ed
                .transact("Estilo del edificio", |m| m.restyle_masses(&list, s));
            self.report(r.map(|_| ()), &format!("Estilo: {}.", s.name()));
        } else if let Some(x) = edit {
            let r = self.ed.transact("Edificio del entorno", |m| {
                for id in &list {
                    // The measurements to all, each keeping its finishes
                    // and style.
                    let Some(own) = m.mass(*id).and_then(|b| b.dress) else {
                        continue;
                    };
                    m.set_mass_dress(
                        *id,
                        Some(Dress {
                            style: own.style,
                            materials: own.materials,
                            ..x
                        }),
                    )?;
                }
                Ok::<_, archisim_core::ModelError>(())
            });
            self.report(r, "Edificio cambiado.");
        } else if undress {
            let r = self.ed.transact("Quitar el estilo", |m| {
                for id in &list {
                    m.set_mass_dress(*id, None)?;
                }
                Ok::<_, archisim_core::ModelError>(())
            });
            self.report(r, "Vuelve a ser un volumen de Concepto.");
        }
    }

    /// The properties of many masses at once.
    pub(super) fn masses_properties(&mut self, ui: &mut Ui, ids: &[MassId]) {
        props::title(ui, &format!("Volúmenes · {} seleccionados", ids.len()));
        self.env_building_group(ui, ids);
    }
}
