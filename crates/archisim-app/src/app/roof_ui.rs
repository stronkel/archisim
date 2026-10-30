//! The roof tool ("Cubierta") and the properties of a selected roof: its
//! shape (four, two or one slope), pitch, overhang, height and the pitch
//! or gable of each side.

use super::widgets::SwitchExt as _;
use archisim_core::{DormerKind, GutterShape, RoofId, SlabFunction, TileKind};
use eframe::egui::{self, Id, Ui};

use super::icons::Icon;
use super::props::{self, choice, group, row, text_row};
use super::*;
use crate::editor::{RoofKind, RoofSettings};
use crate::slab_tool::Outcome as PolyOutcome;

/// Shapes offered, with their Spanish names.
const KINDS: [(RoofKind, &str); 3] = [
    (RoofKind::Hip, "A cuatro aguas"),
    (RoofKind::Gable, "A dos aguas"),
    (RoofKind::Shed, "A un agua"),
];

/// The roof tool's settings for a model: its first pitched roof type (or
/// any roof type, or any slab type), 30 %, 50 cm of eaves.
pub fn default_roof_settings(model: &archisim_core::Model) -> RoofSettings {
    let types: Vec<_> = model.slab_types().collect();
    let roof_type = types
        .iter()
        .find(|t| t.function == SlabFunction::Roof && t.name.to_lowercase().contains("inclinada"))
        .or_else(|| types.iter().find(|t| t.function == SlabFunction::Roof))
        .or(types.first())
        .map_or(archisim_core::SlabTypeId(0), |t| t.id);
    RoofSettings {
        roof_type,
        kind: RoofKind::Hip,
        slope: 0.3,
        overhang: 0.5,
        base_offset: 0.0,
        level_above: true,
        wand: true,
        dormer: None,
    }
}

/// The types a roof may take: those of roofs (all, if there is none).
fn roof_types(model: &archisim_core::Model) -> Vec<(archisim_core::SlabTypeId, String)> {
    let roofs: Vec<_> = model
        .slab_types()
        .filter(|t| t.function == SlabFunction::Roof)
        .map(|t| (t.id, t.name.clone()))
        .collect();
    if roofs.is_empty() {
        model.slab_types().map(|t| (t.id, t.name.clone())).collect()
    } else {
        roofs
    }
}

/// A pitch typed as a percentage ("30", "27,5 %").
fn parse_percent(text: &str) -> Option<f64> {
    let v = text
        .trim()
        .trim_end_matches('%')
        .trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()?;
    v.is_finite().then_some(v / 100.0)
}

/// A pitch as a percentage with a decimal comma.
fn percent(v: f64) -> String {
    let r = (v * 1000.0).round() / 10.0;
    let t = format!("{r:.1}");
    t.trim_end_matches('0')
        .trim_end_matches('.')
        .replace('.', ",")
}

impl ArchiSimApp {
    /// Whether a click with the roof tool takes the building around it.
    pub(super) fn roof_wand(&self) -> bool {
        self.roof_settings.wand
            && !self.roof_tool.is_drawing()
            && self.roof_settings.dormer.is_none()
    }

    pub(super) fn roof_click(&mut self, p: Point2) {
        if let Some(kind) = self.roof_settings.dormer {
            let level = self.ed.level;
            match self.ed.add_dormer_at(level, kind, p) {
                Ok((roof, i)) => {
                    self.ed.selection = [Element::Dormer(roof, i as u16)].into_iter().collect();
                    self.say(format!(
                        "{} colocada. Sigue pinchando para poner más; sus medidas, en Propiedades.",
                        kind.name()
                    ));
                }
                Err(e) => self.complain(format!("No se puede colocar: {e}.")),
            }
            return;
        }
        if self.roof_wand() {
            match self.ed.add_roof_over(&self.roof_settings, p) {
                Ok(id) => self.roof_created(id),
                Err(e) => self.complain(format!("No se puede crear la cubierta: {e}.")),
            }
            return;
        }
        match self.roof_tool.click(p) {
            PolyOutcome::Build(outline) => self.build_roof(outline),
            PolyOutcome::Invalid => {
                self.complain("El contorno no puede cruzarse: corrige la última esquina.")
            }
            PolyOutcome::Nothing => {}
        }
    }

    pub(super) fn build_roof(&mut self, outline: Vec<Point2>) {
        match self.ed.add_roof(&self.roof_settings, outline) {
            Ok(id) => {
                self.roof_tool.accept();
                self.roof_created(id);
            }
            Err(e) => self.complain(format!("No se puede crear la cubierta: {e}.")),
        }
    }

    fn roof_created(&mut self, id: RoofId) {
        let (planes, top, level) = self
            .ed
            .model()
            .roof(id)
            .map(|r| {
                (
                    r.planes().len(),
                    r.top_range().1,
                    self.ed
                        .model()
                        .level(r.level)
                        .map(|l| l.name.clone())
                        .unwrap_or_default(),
                )
            })
            .unwrap_or_default();
        self.ed.selection = [Element::Roof(id)].into_iter().collect();
        self.say(format!(
            "Cubierta de {planes} faldones en «{level}», cumbrera a {} sobre la planta. Cambia la forma y cada lado en Propiedades.",
            crate::units::fmt_len(top)
        ));
    }

    /// The footprint being drawn (the building the wand would take is
    /// drawn by the wand preview).
    pub(super) fn draw_roof_preview(&self, painter: &egui::Painter, rect: Rect) {
        let (Tool::Roof, Some(c)) = (self.tool, self.cursor) else {
            return;
        };
        let corners = self.roof_tool.corners();
        if corners.is_empty() {
            return;
        }
        let ink = Stroke::new(1.5, ACCENT);
        if self.roof_tool.mode == SlabMode::Rectangle {
            if let Some(outline) = self.roof_tool.rectangle_to(c.point) {
                painter.add(Shape::closed_line(self.polygon(rect, &outline), ink));
            }
            return;
        }
        let mut pts = corners.to_vec();
        pts.push(self.roof_tool.typed_end(c.point).unwrap_or(c.point));
        let screen = self.polygon(rect, &pts);
        painter.add(Shape::line(screen.clone(), ink));
        if pts.len() >= 3 {
            dashed(painter, screen[screen.len() - 1], screen[0], ACCENT);
        }
    }

    pub(super) fn roof_tool_properties(&mut self, ui: &mut Ui) {
        if let Some(kind) = self.roof_settings.dormer {
            props::title(ui, "Herramienta Cubierta · buhardillas");
            props::note(
                ui,
                "Pincha en el faldón de una cubierta, en planta o en 3D: la buhardilla se coloca por el centro de su frente y mira aguas abajo. Sus medidas, al seleccionarla. Para volver a dibujar cubiertas, elige Polígono, Rectángulo o Edificio arriba.",
            );
            group(ui, "tool-dormer", "Buhardilla", |ui| {
                row(ui, "Clase", "Qué se coloca con cada clic", |ui, w| {
                    let opts: Vec<_> = DormerKind::ALL.iter().map(|k| (*k, k.name())).collect();
                    let k = choice(ui, "tool-dormer-kind", w, kind, &opts);
                    self.roof_settings.dormer = Some(k);
                });
            });
            return;
        }
        props::title(ui, "Herramienta Cubierta · lo que vas a crear");
        let level = self.ed.roof_level(&self.roof_settings);
        let level_name = self
            .ed
            .model()
            .level(level)
            .map(|l| l.name.clone())
            .unwrap_or_default();
        group(ui, "tool-roof-shape", "Forma", |ui| {
            row(ui, "Forma", "Qué lados llevan faldón", |ui, w| {
                self.roof_settings.kind =
                    choice(ui, "tool-roof-kind", w, self.roof_settings.kind, &KINDS);
            });
            row(ui, "Pendiente (%)", "La de todos los faldones", |ui, w| {
                let shown = percent(self.roof_settings.slope);
                if let Some(v) =
                    text_edit_committed(ui, Id::unique("tool-roof-slope"), &shown, w - 8.0)
                {
                    match parse_percent(&v) {
                        Some(s) if s > 0.0 => self.roof_settings.slope = s,
                        _ => {
                            self.complain(format!("«{v}» no es una pendiente: escribe 30 o 27,5."))
                        }
                    }
                }
            });
            row(ui, "Alero (mm)", "Vuelo de aleros y piñones", |ui, w| {
                if let Some(v) = length_field(
                    ui,
                    Id::unique("tool-roof-overhang"),
                    self.roof_settings.overhang,
                    w - 8.0,
                ) {
                    self.roof_settings.overhang = v.max(0.0);
                }
            });
        });
        group(ui, "tool-roof-place", "Restricciones", |ui| {
            row(
                ui,
                "Planta de encima",
                "Dibujada sobre los muros de una planta, la cubierta pertenece a la de encima",
                |ui, _| {
                    ui.switch(&mut self.roof_settings.level_above, "");
                },
            );
            text_row(ui, "Planta", "", &level_name);
            row(
                ui,
                "Arranque (mm)",
                "Altura de la cara superior en el borde del contorno, sobre su planta",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique("tool-roof-base"),
                        self.roof_settings.base_offset,
                        w - 8.0,
                    ) {
                        self.roof_settings.base_offset = v;
                    }
                },
            );
        });
        let types = roof_types(self.ed.model());
        group(ui, "tool-roof-type", "Tipo", |ui| {
            row(ui, "Composición", "Capas, de arriba abajo", |ui, w| {
                let opts: Vec<_> = types.iter().map(|(id, n)| (*id, n.as_str())).collect();
                self.roof_settings.roof_type =
                    choice(ui, "tool-roof-type", w, self.roof_settings.roof_type, &opts);
            });
        });
    }

    pub(super) fn roof_properties(&mut self, ui: &mut Ui, id: RoofId) {
        let model = self.ed.model();
        let Some(r) = model.roof(id).cloned() else {
            return;
        };
        let type_name = model
            .slab_type(r.roof_type)
            .map(|t| t.name.clone())
            .unwrap_or_default();
        let thickness = model.slab_type(r.roof_type).map_or(0.0, |t| t.thickness());
        props::title(ui, "Cubierta · 1 seleccionada");
        let sketch = super::sketch_ui::edit_contour_button(ui);
        super::properties::view_card(ui, Icon::Roof, &type_name, "IfcRoof");
        if sketch {
            self.start_sketch(crate::sketch::SketchTarget::Roof(id));
            return;
        }
        let types = roof_types(model);
        props::note(
            ui,
            "En la planta, los puntos azules donde se juntan limatesas, limahoyas y cumbrera se arrastran: los faldones que llegan a ellos cambian de pendiente para pasar por el punto nuevo (geometrías especiales).",
        );
        group(ui, "roof-shape", "Forma", |ui| {
            let pitched: Vec<f64> = r.slopes.iter().flatten().copied().collect();
            let slope = pitched.first().copied().unwrap_or(0.3);
            let current = KINDS
                .iter()
                .map(|k| k.0)
                .find(|k| crate::editor::slopes_for(*k, &r.outline, slope) == r.slopes);
            row(ui, "Forma", "Qué lados llevan faldón", |ui, w| {
                let mut opts: Vec<(Option<RoofKind>, &str)> =
                    KINDS.iter().map(|(k, n)| (Some(*k), *n)).collect();
                if current.is_none() {
                    opts.push((None, "A medida"));
                }
                if let Some(k) = choice(ui, ("roof-kind", id.0), w, current, &opts) {
                    if Some(k) != current {
                        let res = self.ed.set_roof_kind(id, k, slope);
                        self.report(res, "Forma cambiada.");
                    }
                }
            });
            let same = pitched.iter().all(|s| (s - slope).abs() < 1e-9);
            row(ui, "Pendiente (%)", "La de todos los faldones", |ui, w| {
                let shown = if same {
                    percent(slope)
                } else {
                    "varias".to_string()
                };
                if let Some(v) =
                    text_edit_committed(ui, Id::unique(("roof-slope", id.0)), &shown, w - 8.0)
                {
                    match parse_percent(&v) {
                        Some(s) => {
                            let res = self.ed.edit_roof("Pendiente de la cubierta", id, |r| {
                                for x in r.slopes.iter_mut().flatten() {
                                    *x = s;
                                }
                            });
                            self.report(res, "Pendiente cambiada.");
                        }
                        None => {
                            self.complain(format!("«{v}» no es una pendiente: escribe 30 o 27,5."))
                        }
                    }
                }
            });
            row(ui, "Alero (mm)", "Vuelo de aleros y piñones", |ui, w| {
                if let Some(v) =
                    length_field(ui, Id::unique(("roof-overhang", id.0)), r.overhang, w - 8.0)
                {
                    let res = self
                        .ed
                        .edit_roof("Alero de la cubierta", id, |r| r.overhang = v);
                    self.report(res, "Alero cambiado.");
                }
            });
        });
        group(ui, "roof-sides", "Lados", |ui| {
            let n = r.outline.len();
            for i in 0..n {
                let len = r.outline[i].distance(r.outline[(i + 1) % n]);
                let label = format!("Lado {} · {} m", i + 1, fmt_m(len));
                row(
                    ui,
                    &label,
                    "Faldón con su pendiente, o piñón",
                    |ui, w| {
                        let shown = r.slopes[i].map_or("piñón".to_string(), percent);
                        if let Some(v) = text_edit_committed(
                            ui,
                            Id::unique(("roof-side", id.0, i)),
                            &shown,
                            w - 8.0,
                        ) {
                            let t = v.trim().to_lowercase();
                            let new = if t.is_empty() || t.starts_with('p') || t == "-" {
                                Some(None)
                            } else {
                                parse_percent(&t).map(Some)
                            };
                            match new {
                                Some(x) => {
                                    let res = self
                                        .ed
                                        .edit_roof("Lado de la cubierta", id, |r| r.slopes[i] = x);
                                    self.report(res, "Lado cambiado.");
                                }
                                None => self.complain(format!(
                                    "«{v}» no es una pendiente: escribe 30, o «piñón»."
                                )),
                            }
                        }
                    },
                );
            }
        });
        group(ui, "roof-constraints", "Restricciones", |ui| {
            self.level_row(
                ui,
                ("roof-level", id),
                "Planta",
                Some(r.level),
                &[Element::Roof(id)],
            );
            row(
                ui,
                "Arranque (mm)",
                "Altura de la cara superior en el borde del contorno, sobre la planta",
                |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("roof-base", id.0)), r.base_offset, w - 8.0)
                    {
                        let res = self
                            .ed
                            .edit_roof("Altura de la cubierta", id, |r| r.base_offset = v);
                        self.report(res, "Altura cambiada.");
                    }
                },
            );
        });
        group(ui, "roof-type", "Tipo", |ui| {
            row(ui, "Composición", "Capas, de arriba abajo", |ui, w| {
                let opts: Vec<_> = types.iter().map(|(t, n)| (*t, n.as_str())).collect();
                let t = choice(ui, ("roof-type", id.0), w, r.roof_type, &opts);
                if t != r.roof_type {
                    let res = self
                        .ed
                        .edit_roof("Tipo de cubierta", id, |r| r.roof_type = t);
                    self.report(res, "Tipo cambiado.");
                }
            });
            text_row(ui, "Canto (mm)", "Del tipo", &fmt_mm(thickness));
        });
        group(ui, "roof-quantities", "Superficies", |ui| {
            let (lo, hi) = r.top_range();
            text_row(
                ui,
                "Superficie inclinada",
                "Sobre la pendiente, aleros incluidos",
                &fmt_area(r.surface_area()),
            );
            text_row(
                ui,
                "En planta",
                "Proyección, aleros incluidos",
                &fmt_area(archisim_core::signed_area(&r.eaves()).abs()),
            );
            text_row(
                ui,
                "Cumbrera (mm)",
                "Punto más alto de la cara superior, sobre la planta",
                &fmt_mm(hi),
            );
            text_row(
                ui,
                "Alero (mm)",
                "Punto más bajo de la cara superior, sobre la planta",
                &fmt_mm(lo),
            );
        });
        group(ui, "roof-walls", "Muros", |ui| {
            let trimmed = self
                .ed
                .model()
                .walls()
                .filter(|w| self.ed.model().wall_roof(w.id) == Some(id))
                .count();
            text_row(
                ui,
                "Recortados debajo",
                "Muros que suben hasta la cara inferior de esta cubierta",
                &trimmed.to_string(),
            );
            if ui
                .button("Recortar los muros de debajo")
                .on_hover_text("Los muros rectos de esta planta y de la de debajo que quedan bajo la cubierta suben hasta ella: los piñones salen solos")
                .clicked()
            {
                match self.ed.trim_walls_under(id) {
                    Ok(0) => self.complain("No hay muros rectos bajo esta cubierta."),
                    Ok(n) => self.say(format!("{n} muros recortados bajo la cubierta.")),
                    Err(e) => self.complain(format!("No se puede: {e}.")),
                }
            }
        });
        self.covering_group(ui, id, &r);
        group(ui, "roof-dormers", "Buhardillas", |ui| {
            if r.dormers.is_empty() {
                props::note(
                    ui,
                    "Sin buhardillas. Herramienta Cubierta → Buhardilla o Ventana de cubierta (barra de arriba) y clic en un faldón.",
                );
            }
            for (i, d) in r.dormers.iter().enumerate() {
                let fits = r.dormer_shape(d).is_some();
                let label = format!("{}{}", d.kind.name(), if fits { "" } else { " · no cabe" });
                row(ui, &label, "Seleccionarla", |ui, _| {
                    if ui.small_button("Seleccionar").clicked() {
                        self.ed.selection = [Element::Dormer(id, i as u16)].into_iter().collect();
                    }
                });
            }
        });
        self.phase_group(ui, "roof-phase", &[Element::Roof(id)]);
        group(ui, "roof-identity", "Datos de identidad", |ui| {
            props::id_row(ui, "GlobalId", &r.guid.to_string());
        });
    }

    /// Tiles and gutters of a roof.
    fn covering_group(&mut self, ui: &mut Ui, id: RoofId, r: &archisim_core::Roof) {
        let materials: Vec<(archisim_core::MaterialId, String)> = self
            .ed
            .model()
            .materials()
            .map(|m| (m.id, m.name.clone()))
            .collect();
        let default_gutter = self.ed.model().default_gutter();
        let mut c = r.covering.clone();
        let before = c.clone();
        group(ui, "roof-covering", "Cubrición", |ui| {
            row(
                ui,
                "Tejas",
                "La piel exterior: hiladas, cobijas y caballetes en 3D, rayado en planta",
                |ui, w| {
                    let mut opts: Vec<(Option<TileKind>, &str)> =
                        vec![(None, "Ninguna (la capa del tipo)")];
                    opts.extend(TileKind::ALL.iter().map(|k| (Some(*k), k.name())));
                    c.tiles = choice(ui, ("roof-tiles", id.0), w, c.tiles, &opts);
                },
            );
            if c.tiles.is_some() {
                row(
                    ui,
                    "Material de las tejas",
                    "Vacío: el de la capa superior del tipo",
                    |ui, w| {
                        let mut opts: Vec<(Option<archisim_core::MaterialId>, &str)> =
                            vec![(None, "El de la capa superior")];
                        opts.extend(materials.iter().map(|(m, n)| (Some(*m), n.as_str())));
                        c.tile_material =
                            choice(ui, ("roof-tile-mat", id.0), w, c.tile_material, &opts);
                    },
                );
            }
            row(
                ui,
                "Canalón",
                "En los aleros, con sus bajantes hasta la planta más baja",
                |ui, _| {
                    let mut on = c.gutter.is_some();
                    if ui.switch(&mut on, "").changed() {
                        c.gutter = if on { default_gutter.clone() } else { None };
                    }
                },
            );
            if let Some(g) = c.gutter.as_mut() {
                row(ui, "Forma", "", |ui, w| {
                    g.shape = choice(
                        ui,
                        ("roof-gutter-shape", id.0),
                        w,
                        g.shape,
                        &[
                            (GutterShape::HalfRound, "Media caña"),
                            (GutterShape::Box, "Cuadrado"),
                        ],
                    );
                });
                row(ui, "Ancho (mm)", "Del canalón", |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("roof-gutter-size", id.0)), g.size, w - 8.0)
                    {
                        g.size = v;
                    }
                });
                row(
                    ui,
                    "Bajante Ø (mm)",
                    "Diámetro de las bajantes",
                    |ui, w| {
                        if let Some(v) = length_field(
                            ui,
                            Id::unique(("roof-gutter-pipe", id.0)),
                            g.downpipe,
                            w - 8.0,
                        ) {
                            g.downpipe = v;
                        }
                    },
                );
                row(
                    ui,
                    "Bajante cada (mm)",
                    "Una en cada extremo del alero y más si es más largo",
                    |ui, w| {
                        if let Some(v) = length_field(
                            ui,
                            Id::unique(("roof-gutter-gap", id.0)),
                            g.spacing,
                            w - 8.0,
                        ) {
                            g.spacing = v;
                        }
                    },
                );
                row(ui, "Material", "", |ui, w| {
                    let opts: Vec<_> = materials.iter().map(|(m, n)| (*m, n.as_str())).collect();
                    g.material = choice(ui, ("roof-gutter-mat", id.0), w, g.material, &opts);
                });
            }
        });
        if c != before {
            let res = self.ed.set_roof_covering(id, c);
            self.report(res, "Cubrición cambiada.");
        }
    }

    /// A selected dormer or roof window.
    pub(super) fn dormer_properties(&mut self, ui: &mut Ui, id: RoofId, index: usize) {
        let model = self.ed.model();
        let Some(r) = model.roof(id).cloned() else {
            return;
        };
        let Some(d) = r.dormers.get(index).cloned() else {
            return;
        };
        let materials: Vec<(archisim_core::MaterialId, String)> =
            model.materials().map(|m| (m.id, m.name.clone())).collect();
        let shape = r.dormer_shape(&d);
        props::title(ui, &format!("{} · 1 seleccionada", d.kind.name()));
        let class = if d.kind == DormerKind::Skylight {
            "IfcWindow SKYLIGHT"
        } else {
            "IfcElementAssembly"
        };
        super::properties::view_card(ui, Icon::Roof, d.kind.name(), class);
        props::note(
            ui,
            "Vive en su cubierta: se mueve arrastrándola por el faldón, y sigue a la cubierta si esta cambia. Si deja de caber, no se dibuja hasta que vuelva a caber.",
        );
        let mut nd = d.clone();
        group(ui, "dormer-shape", "Forma", |ui| {
            row(ui, "Clase", "", |ui, w| {
                let opts: Vec<_> = DormerKind::ALL.iter().map(|k| (*k, k.name())).collect();
                nd.kind = choice(ui, ("dormer-kind", id.0, index), w, nd.kind, &opts);
            });
            row(ui, "Ancho (mm)", "Por fuera", |ui, w| {
                if let Some(v) =
                    length_field(ui, Id::unique(("dormer-w", id.0, index)), d.width, w - 8.0)
                {
                    nd.width = v;
                }
            });
            let height = if d.kind == DormerKind::Skylight {
                ("Largo (mm)", "En la pendiente del faldón")
            } else {
                (
                    "Altura del frente (mm)",
                    "Del faldón al alero de la buhardilla",
                )
            };
            row(ui, height.0, height.1, |ui, w| {
                if let Some(v) =
                    length_field(ui, Id::unique(("dormer-h", id.0, index)), d.height, w - 8.0)
                {
                    nd.height = v;
                }
            });
            if matches!(d.kind, DormerKind::Gable | DormerKind::Shed) {
                row(ui, "Pendiente (%)", "La de su propia cubierta", |ui, w| {
                    if let Some(v) = text_edit_committed(
                        ui,
                        Id::unique(("dormer-p", id.0, index)),
                        &percent(d.pitch),
                        w - 8.0,
                    ) {
                        match parse_percent(&v) {
                            Some(p) => nd.pitch = p,
                            None => self
                                .complain(format!("«{v}» no es una pendiente: escribe 30 o 27,5.")),
                        }
                    }
                });
            }
        });
        if d.kind != DormerKind::Skylight {
            group(ui, "dormer-window", "Ventana del frente", |ui| {
                row(ui, "Ancho (mm)", "", |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique(("dormer-ww", id.0, index)),
                        d.window_width,
                        w - 8.0,
                    ) {
                        nd.window_width = v;
                    }
                });
                row(ui, "Alto (mm)", "", |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique(("dormer-wh", id.0, index)),
                        d.window_height,
                        w - 8.0,
                    ) {
                        nd.window_height = v;
                    }
                });
            });
        }
        group(ui, "dormer-materials", "Materiales", |ui| {
            let opts: Vec<_> = materials.iter().map(|(m, n)| (*m, n.as_str())).collect();
            if d.kind != DormerKind::Skylight {
                row(ui, "Frente y laterales", "", |ui, w| {
                    nd.wall_material =
                        choice(ui, ("dormer-wm", id.0, index), w, nd.wall_material, &opts);
                });
            }
            row(ui, "Carpintería", "", |ui, w| {
                nd.frame_material =
                    choice(ui, ("dormer-fm", id.0, index), w, nd.frame_material, &opts);
            });
            row(ui, "Vidrio", "", |ui, w| {
                nd.glass_material =
                    choice(ui, ("dormer-gm", id.0, index), w, nd.glass_material, &opts);
            });
        });
        if let Some(s) = &shape {
            group(ui, "dormer-measures", "Medidas", |ui| {
                text_row(
                    ui,
                    "Frente a (mm)",
                    "Sobre la planta de la cubierta, donde el frente pisa el faldón",
                    &fmt_mm(s.frame.z0),
                );
                if d.kind != DormerKind::Skylight {
                    text_row(
                        ui,
                        "Alero (mm)",
                        "Sobre la planta de la cubierta",
                        &fmt_mm(s.eave_z),
                    );
                    text_row(
                        ui,
                        "Cumbrera (mm)",
                        "Sobre la planta de la cubierta",
                        &fmt_mm(s.ridge_z),
                    );
                }
                text_row(
                    ui,
                    "Fondo en planta (mm)",
                    "Del frente a donde muere en el faldón",
                    &fmt_mm(s.ridge_back),
                );
            });
        } else {
            props::note(
                ui,
                "No cabe en el faldón tal como está la cubierta: no se dibuja.",
            );
        }
        group(ui, "dormer-roof", "Cubierta", |ui| {
            row(ui, "Su cubierta", "", |ui, _| {
                if ui.small_button("Seleccionar la cubierta").clicked() {
                    self.ed.selection = [Element::Roof(id)].into_iter().collect();
                }
            });
        });
        group(ui, "dormer-identity", "Datos de identidad", |ui| {
            props::id_row(ui, "GlobalId", &d.guid.to_string());
        });
        if nd.kind != d.kind {
            // Another kind: its usual sizes, where it was.
            let fresh = archisim_core::Dormer::new(
                nd.kind,
                nd.wall_material,
                nd.frame_material,
                nd.glass_material,
            );
            nd = archisim_core::Dormer {
                guid: d.guid.clone(),
                side: d.side,
                along: d.along,
                up: d.up,
                ..fresh
            };
        }
        if nd != d {
            let label = format!("{}: medidas", d.kind.name());
            let res = self.ed.edit_dormer(&label, id, index, move |x| *x = nd);
            self.report(res, "Buhardilla cambiada.");
        }
    }
}
