//! The grid and footing tools: what they draw, their properties and those
//! of selected grid lines and footings.
//!
//! A grid line is drawn by two clicks and takes the next name (numbers for
//! the lines running up, letters for those across). A footing is a pad
//! (a click on a column puts it under the column, which it then follows;
//! elsewhere, at the click), a strip (a click on a wall puts it under the
//! wall; elsewhere, two clicks) or a tie beam (two clicks).

use archisim_core::{Footing, FootingId, FootingKind, GlobalId, GridId, MaterialId, Model, Point2};
use eframe::egui::{self, ComboBox, Id, Ui};

use super::props::{self, choice, group, id_row, row, text_row};
use super::*;
use crate::editor::FootingMode;

/// A combo of the project's materials, by name.
fn material_combo(ui: &mut Ui, id: Id, model: &Model, value: MaterialId, width: f32) -> MaterialId {
    let mut v = value;
    let mut mats: Vec<(MaterialId, String)> =
        model.materials().map(|m| (m.id, m.name.clone())).collect();
    mats.sort_by_key(|a| a.1.to_lowercase());
    let shown = model
        .material(value)
        .map_or("—".to_string(), |m| m.name.clone());
    ComboBox::from_id_salt(id)
        .width(width - 8.0)
        .truncate()
        .selected_text(shown)
        .show_ui(ui, |ui| {
            for (mid, name) in &mats {
                ui.selectable_value(&mut v, *mid, name);
            }
        });
    v
}

impl ArchiSimApp {
    /// The point the line being drawn by the grid or footing tool starts
    /// from.
    pub(super) fn foundation_anchor(&self) -> Option<Point2> {
        match self.tool {
            Tool::Grid => self.grid_anchor,
            Tool::Footing => self.footing_anchor,
            _ => None,
        }
    }

    /// Esc or a right click: ends the line being drawn. Returns whether
    /// there was one.
    pub(super) fn foundation_finish(&mut self) -> bool {
        self.grid_anchor.take().is_some() | self.footing_anchor.take().is_some()
    }

    /// The column under `p`, on the active level.
    fn column_under(&self, p: Point2) -> Option<archisim_core::ColumnId> {
        let reach = self.view.to_metres(REACH);
        match pick(&self.ed, self.ed.level, p, reach) {
            Some(Element::Column(c)) => Some(c),
            _ => None,
        }
    }

    /// The straight wall under `p`, on the active level.
    fn wall_under(&self, p: Point2) -> Option<archisim_core::WallId> {
        let reach = self.view.to_metres(REACH);
        crate::opening_tool::wall_at(&self.ed, self.ed.level, p, reach)
            .filter(|w| self.ed.model().wall_centre_line(*w).is_some())
    }

    /// A click of the grid or footing tool at `p`.
    pub(super) fn foundation_click(&mut self, p: Point2) {
        match self.tool {
            Tool::Grid => match self.grid_anchor {
                None => self.grid_anchor = Some(p),
                Some(a) if a.approx_eq(p) => {}
                Some(a) => match self.ed.add_grid(a, p) {
                    Ok(id) => {
                        self.grid_anchor = None;
                        self.ed.selection = [Element::Grid(id)].into_iter().collect();
                        let name = self
                            .ed
                            .model()
                            .grid(id)
                            .map(|g| g.name.clone())
                            .unwrap_or_default();
                        self.say(format!("Eje {name} creado. Clic: el siguiente."));
                    }
                    Err(e) => self.complain(format!("No se puede crear el eje: {e}.")),
                },
            },
            Tool::Footing => self.footing_click(p),
            _ => {}
        }
    }

    fn footing_click(&mut self, p: Point2) {
        let s = self.footing_settings;
        let placed = |app: &mut Self, r: Result<FootingId, String>, what: &str| match r {
            Ok(id) => {
                app.ed.selection = [Element::Footing(id)].into_iter().collect();
                app.say(what.to_string());
            }
            Err(e) => app.complain(format!("No se puede colocar la cimentación: {e}.")),
        };
        match (s.mode, self.footing_anchor) {
            (FootingMode::Pad, _) => {
                let column = self.column_under(p);
                if let Some(c) = column {
                    if self.has_footing_under(Element::Column(c)) {
                        self.complain("Ese pilar ya tiene zapata.");
                        return;
                    }
                }
                let r = self.ed.add_footing(&s, s.pad(p, column));
                let what = if column.is_some() {
                    "Zapata colocada bajo el pilar: lo sigue si se mueve."
                } else {
                    "Zapata aislada colocada."
                };
                placed(self, r, what);
            }
            (FootingMode::Strip, None) => {
                if let Some(w) = self.wall_under(p) {
                    if self.has_footing_under(Element::Wall(w)) {
                        self.complain("Ese muro ya tiene zapata corrida.");
                        return;
                    }
                    let (a, b) = self
                        .ed
                        .model()
                        .wall_centre_line(w)
                        .unwrap_or((p, p + Point2::new(1.0, 0.0)));
                    let r = self.ed.add_footing(&s, s.between(a, b, Some(w)));
                    placed(self, r, "Zapata corrida bajo el muro: lo sigue si cambia.");
                } else {
                    self.footing_anchor = Some(p);
                }
            }
            (_, None) => self.footing_anchor = Some(p),
            (_, Some(a)) if a.approx_eq(p) => {}
            (mode, Some(a)) => {
                let r = self.ed.add_footing(&s, s.between(a, p, None));
                self.footing_anchor = Some(p);
                let what = format!(
                    "{} de {} mm. Clic: la siguiente desde aquí · Esc: terminar.",
                    mode.name(),
                    fmt_mm(a.distance(p))
                );
                placed(self, r, &what);
            }
        }
    }

    /// Whether a column or wall already has its footing.
    fn has_footing_under(&self, host: Element) -> bool {
        self.ed.model().footings().any(|f| match (f.kind, host) {
            (
                FootingKind::Pad {
                    column: Some(c), ..
                },
                Element::Column(x),
            ) => c == x,
            (FootingKind::Strip { wall: Some(w), .. }, Element::Wall(x)) => w == x,
            _ => false,
        })
    }

    /// The footing that would be placed, or the grid line being drawn.
    pub(super) fn draw_foundation_preview(&self, painter: &egui::Painter, rect: Rect) {
        let Some(c) = self.cursor else {
            return;
        };
        let model = self.ed.model();
        match self.tool {
            Tool::Grid => {
                let Some(a) = self.grid_anchor else {
                    return;
                };
                dashed(
                    painter,
                    self.view.to_screen(rect, a),
                    self.view.to_screen(rect, c.point),
                    ACCENT,
                );
                let d = c.point - a;
                self.draw_label(
                    painter,
                    rect,
                    c.point,
                    format!(
                        "Eje {}   L {} mm   {}",
                        model.next_grid_name(a, c.point),
                        fmt_mm(d.length()),
                        fmt_angle(d.y.atan2(d.x))
                    ),
                );
            }
            Tool::Footing => {
                let s = &self.footing_settings;
                let size = s.size();
                let kind = match (s.mode, self.footing_anchor) {
                    (FootingMode::Pad, _) => {
                        match self.column_under(c.point).and_then(|id| model.column(id)) {
                            Some(col) => FootingKind::Pad {
                                position: col.position,
                                length: size.length,
                                rotation: col.rotation,
                                column: None,
                            },
                            None => s.pad(c.point, None),
                        }
                    }
                    (FootingMode::Strip, None) => {
                        match self
                            .wall_under(c.point)
                            .and_then(|w| model.wall_centre_line(w))
                        {
                            Some((a, b)) => s.between(a, b, None),
                            None => return,
                        }
                    }
                    (_, None) => return,
                    (_, Some(a)) => s.between(a, c.point, None),
                };
                let probe = Footing {
                    id: FootingId(0),
                    guid: GlobalId::default(),
                    level: self.ed.level,
                    kind,
                    material: s.material,
                    width: size.width,
                    depth: size.depth,
                    top_offset: s.top_offset,
                };
                let outline = probe.outline();
                if outline.len() == 4 {
                    let pts = self.polygon(rect, &outline);
                    painter.add(Shape::convex_polygon(
                        pts.clone(),
                        ACCENT.gamma_multiply(0.15),
                        Stroke::NONE,
                    ));
                    painter.add(Shape::closed_line(pts, Stroke::new(1.5, ACCENT)));
                }
                let text = match kind {
                    FootingKind::Pad { length, .. } => format!(
                        "{} × {} × {} mm",
                        fmt_mm(size.width),
                        fmt_mm(length),
                        fmt_mm(size.depth)
                    ),
                    FootingKind::Strip { start, end, .. } | FootingKind::TieBeam { start, end } => {
                        format!(
                            "{}   L {} mm   {} × {} mm",
                            s.mode.name(),
                            fmt_mm(start.distance(end)),
                            fmt_mm(size.width),
                            fmt_mm(size.depth)
                        )
                    }
                };
                self.draw_label(painter, rect, c.point, text);
            }
            _ => {}
        }
    }

    /// The hint of the grid and footing tools, in the bottom bar.
    pub(super) fn foundation_hint(&self) -> &'static str {
        match (self.tool, self.footing_settings.mode) {
            (Tool::Grid, _) if self.grid_anchor.is_some() => {
                "Clic: final del eje · Mayús: 45° · Esc: anular"
            }
            (Tool::Grid, _) => "Clic: comienzo del eje (se nombra 1, 2… o A, B… según su dirección) · Esc: flecha",
            (_, FootingMode::Pad) => {
                "Clic en un pilar: zapata bajo él (lo sigue) · Clic en otro sitio: zapata en ese punto · Esc: flecha"
            }
            (_, _) if self.footing_anchor.is_some() => {
                "Clic: final (y comienzo de la siguiente) · Mayús: 45° · Clic derecho o Esc: terminar"
            }
            (_, FootingMode::Strip) => {
                "Clic en un muro: zapata corrida bajo él (lo sigue) · Clic fuera: comienzo de una zapata libre · Esc: flecha"
            }
            (_, FootingMode::TieBeam) => "Clic: comienzo de la viga de atado · Esc: flecha",
        }
    }

    /// Changes the kind of footing the tool places.
    pub(super) fn set_footing_mode(&mut self, mode: FootingMode) {
        if self.footing_settings.mode != mode {
            self.footing_settings.mode = mode;
            self.footing_anchor = None;
        }
    }

    /// Pads under every column and strips under every straight wall of the
    /// level that have none.
    pub(super) fn found_level(&mut self) {
        match self.ed.found_level(&self.footing_settings) {
            Ok(0) => self.say("Todos los pilares y muros de la planta ya tienen cimentación."),
            Ok(n) => self.say(format!(
                "{n} {} bajo los pilares y muros de la planta.",
                if n == 1 {
                    "zapata colocada"
                } else {
                    "zapatas colocadas"
                }
            )),
            Err(e) => self.complain(format!("No se puede cimentar la planta: {e}.")),
        }
    }

    // ---- Properties ---------------------------------------------------------

    pub(super) fn grid_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Eje · lo que vas a dibujar");
        group(ui, "tool-grid", "Eje", |ui| {
            let name = self
                .ed
                .model()
                .next_grid_name(Point2::new(0.0, 0.0), Point2::new(0.0, 1.0));
            text_row(
                ui,
                "Siguiente vertical",
                "Los ejes que suben se numeran",
                &name,
            );
            let name = self
                .ed
                .model()
                .next_grid_name(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0));
            text_row(
                ui,
                "Siguiente horizontal",
                "Los que cruzan llevan letras",
                &name,
            );
            props::note(
                ui,
                "Los ejes se ven en todas las plantas y en las secciones que los cortan. El nombre se cambia en las propiedades del eje.",
            );
        });
    }

    pub(super) fn footing_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Cimentación · lo que vas a colocar");
        let model = self.ed.model();
        let mut s = self.footing_settings;
        group(ui, "tool-footing", "Cimentación", |ui| {
            row(
                ui,
                "Tipo",
                "Zapata aislada, corrida o viga de atado",
                |ui, w| {
                    let options: Vec<(FootingMode, &str)> =
                        FootingMode::ALL.iter().map(|m| (*m, m.name())).collect();
                    s.mode = choice(ui, "tool-footing-mode", w, s.mode, &options);
                },
            );
            row(ui, "Material", "", |ui, w| {
                s.material =
                    material_combo(ui, Id::unique("tool-footing-mat"), model, s.material, w);
            });
            row(
                ui,
                "Cara superior (mm)",
                "Sobre la planta; negativa, bajo el suelo (bajo la solera)",
                |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique("tool-footing-top"), s.top_offset, w - 8.0)
                    {
                        s.top_offset = v;
                    }
                },
            );
        });
        group(ui, "tool-footing-size", "Dimensiones", |ui| {
            let pad = s.mode == FootingMode::Pad;
            let size = *s.size_mut();
            let mut out = size;
            row(
                ui,
                if pad { "Lado x (mm)" } else { "Ancho (mm)" },
                "",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique(("tool-footing-w", s.mode as u8)),
                        size.width,
                        w - 8.0,
                    ) {
                        if v > 0.0 {
                            out.width = v;
                        }
                    }
                },
            );
            if pad {
                row(ui, "Lado y (mm)", "", |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique("tool-footing-l"), size.length, w - 8.0)
                    {
                        if v > 0.0 {
                            out.length = v;
                        }
                    }
                });
            }
            row(ui, "Canto (mm)", "Bajo la cara superior", |ui, w| {
                if let Some(v) = length_field(
                    ui,
                    Id::unique(("tool-footing-d", s.mode as u8)),
                    size.depth,
                    w - 8.0,
                ) {
                    if v > 0.0 {
                        out.depth = v;
                    }
                }
            });
            *s.size_mut() = out;
            if pad {
                row(
                    ui,
                    "Giro",
                    "De una zapata libre; bajo un pilar, toma el suyo",
                    |ui, w| {
                        if let Some(v) = super::structure_ui::angle_field(
                            ui,
                            Id::unique("tool-footing-rot"),
                            s.rotation,
                            w - 8.0,
                        ) {
                            s.rotation = v;
                        }
                    },
                );
            }
        });
        self.footing_settings = s;
        ui.add_space(4.0);
        if ui
            .button("Cimentar la planta")
            .on_hover_text("Zapatas aisladas bajo los pilares y corridas bajo los muros rectos de esta planta que no la tengan")
            .clicked()
        {
            self.found_level();
        }
    }

    pub(super) fn footing_properties(&mut self, ui: &mut Ui, id: FootingId) {
        let model = self.ed.model();
        let Some(f) = model.footing(id).cloned() else {
            return;
        };
        props::title(ui, &format!("{} · 1 seleccionada", f.kind_name()));
        let host = match f.kind {
            FootingKind::Pad {
                column: Some(c), ..
            } => model.column(c).map(|_| format!("Bajo el pilar {}", c.0)),
            FootingKind::Strip { wall: Some(w), .. } => {
                model.wall(w).map(|_| format!("Bajo el muro {}", w.0))
            }
            _ => None,
        };
        let mat = f.material;
        group(ui, "footing-constraints", "Restricciones", |ui| {
            self.level_row(
                ui,
                ("footing-level", id),
                "Planta de referencia",
                Some(f.level),
                &[Element::Footing(id)],
            );
            row(
                ui,
                "Cara superior (mm)",
                "Sobre la planta; negativa, bajo el suelo",
                |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("footing-top", id)), f.top_offset, w - 8.0)
                    {
                        let r =
                            self.ed
                                .edit_footings("Cara superior de la cimentación", &[id], |f| {
                                    f.top_offset = v
                                });
                        self.report(r, "Cara superior cambiada.");
                    }
                },
            );
            text_row(
                ui,
                "Sigue a",
                "Bajo un pilar o un muro, la cimentación se mueve con él; movida sola, queda libre",
                host.as_deref().unwrap_or("—"),
            );
        });
        group(ui, "footing-dimensions", "Dimensiones", |ui| {
            let pad = matches!(f.kind, FootingKind::Pad { .. });
            row(
                ui,
                if pad { "Lado x (mm)" } else { "Ancho (mm)" },
                "",
                |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("footing-w", id)), f.width, w - 8.0)
                    {
                        let r = self
                            .ed
                            .edit_footings("Ancho de la cimentación", &[id], |f| f.width = v);
                        self.report(r, "Ancho cambiado.");
                    }
                },
            );
            if let FootingKind::Pad { length, .. } = f.kind {
                row(ui, "Lado y (mm)", "", |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("footing-l", id)), length, w - 8.0)
                    {
                        let r = self.ed.edit_footings("Lado de la zapata", &[id], |f| {
                            if let FootingKind::Pad { length, .. } = &mut f.kind {
                                *length = v;
                            }
                        });
                        self.report(r, "Lado cambiado.");
                    }
                });
            }
            row(ui, "Canto (mm)", "", |ui, w| {
                if let Some(v) = length_field(ui, Id::unique(("footing-d", id)), f.depth, w - 8.0) {
                    let r = self
                        .ed
                        .edit_footings("Canto de la cimentación", &[id], |f| f.depth = v);
                    self.report(r, "Canto cambiado.");
                }
            });
            match f.kind {
                FootingKind::Strip { start, end, .. } | FootingKind::TieBeam { start, end } => {
                    text_row(ui, "Longitud (mm)", "", &fmt_mm(start.distance(end)));
                }
                FootingKind::Pad { .. } => {}
            }
            text_row(
                ui,
                "Volumen (m³)",
                "",
                &format!("{:.3}", f.volume()).replace('.', ","),
            );
        });
        group(ui, "footing-material", "Material", |ui| {
            row(ui, "Material", "", |ui, w| {
                let v =
                    material_combo(ui, Id::unique(("footing-mat", id)), self.ed.model(), mat, w);
                if v != mat {
                    let r = self
                        .ed
                        .edit_footings("Material de la cimentación", &[id], |f| f.material = v);
                    self.report(r, "Material cambiado.");
                }
            });
        });
        group(ui, "footing-identity", "Datos de identidad", |ui| {
            text_row(ui, "Clase IFC", "", "IfcFooting");
            id_row(ui, "GlobalId", &f.guid.to_string());
        });
    }

    pub(super) fn grid_properties(&mut self, ui: &mut Ui, id: GridId) {
        let Some(g) = self.ed.model().grid(id).cloned() else {
            return;
        };
        props::title(ui, &format!("Eje {} · 1 seleccionado", g.name));
        group(ui, "grid-data", "Eje", |ui| {
            row(ui, "Nombre", "Lo que se lee en sus burbujas", |ui, w| {
                if let Some(t) =
                    text_edit_committed(ui, Id::unique(("grid-name", id)), &g.name, w - 8.0)
                {
                    let t = t.trim().to_string();
                    if t.is_empty() {
                        self.complain("El eje necesita un nombre.");
                    } else {
                        let r = self.ed.edit_grid("Nombre del eje", id, |g| g.name = t);
                        self.report(r, "Nombre cambiado.");
                    }
                }
            });
            row(
                ui,
                "Longitud (mm)",
                "Desde el inicio; el final se desplaza",
                |ui, w| {
                    let len = g.start.distance(g.end);
                    if let Some(v) = length_field(ui, Id::unique(("grid-len", id)), len, w - 8.0) {
                        if v > 0.0 {
                            let r = self.ed.edit_grid("Longitud del eje", id, |g| {
                                if let Some(u) = (g.end - g.start).normalized() {
                                    g.end = g.start + u * v;
                                }
                            });
                            self.report(r, "Longitud cambiada.");
                        } else {
                            self.complain("La longitud tiene que ser mayor que cero.");
                        }
                    }
                },
            );
            let d = g.end - g.start;
            text_row(ui, "Dirección", "", &fmt_angle(d.y.atan2(d.x)));
        });
        group(ui, "grid-identity", "Datos de identidad", |ui| {
            text_row(
                ui,
                "Clase IFC",
                "",
                "IfcGridAxis (en el IfcGrid del proyecto)",
            );
        });
    }
}
