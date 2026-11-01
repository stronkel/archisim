//! Urbanismo: the plot tool (a boundary by its corners, with its setback)
//! and the road tool (an axis by its points, with a cross-section), and
//! the properties of plots and roads.

use archisim_core::{
    MaterialId, Model, NewPlot, NewRoad, PlotId, Point2, RoadBand, RoadId, RoadSection,
};
use eframe::egui::{self, ComboBox, Id, Ui};

use super::props::{self, group, row, text_row};
use super::*;

/// Which line of a road the clicks draw (ArchiCAD's reference line of a
/// wall): its axis, or the outer edge of one sidewalk, left or right as
/// seen walking the way it is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoadReference {
    LeftEdge,
    #[default]
    Axis,
    RightEdge,
}

impl RoadReference {
    pub const ALL: [RoadReference; 3] = [
        RoadReference::LeftEdge,
        RoadReference::Axis,
        RoadReference::RightEdge,
    ];

    pub fn name(self) -> &'static str {
        match self {
            RoadReference::LeftEdge => "Borde izquierdo",
            RoadReference::Axis => "Eje",
            RoadReference::RightEdge => "Borde derecho",
        }
    }

    /// The road's axis from the line drawn.
    pub fn axis(self, drawn: &[Point2], section: &RoadSection) -> Option<Vec<Point2>> {
        let w = section.width() / 2.0;
        match self {
            RoadReference::Axis => Some(drawn.to_vec()),
            // The road lies to the right of a left edge.
            RoadReference::LeftEdge => archisim_core::offset_path(drawn, -w),
            RoadReference::RightEdge => archisim_core::offset_path(drawn, w),
        }
    }
}

/// What the road tool draws.
#[derive(Debug, Clone, PartialEq)]
pub struct RoadSettings {
    pub reference: RoadReference,
    pub section: RoadSection,
    pub top_offset: f64,
    pub pavement: MaterialId,
    pub kerb: MaterialId,
    pub sidewalk: MaterialId,
}

impl RoadSettings {
    pub fn new(model: &Model) -> Self {
        let by = |cats: &[&str]| {
            cats.iter()
                .find_map(|c| model.materials().find(|m| m.category == *c))
                .or_else(|| model.materials().next())
                .map_or(MaterialId(0), |m| m.id)
        };
        RoadSettings {
            reference: RoadReference::Axis,
            section: RoadSection::default(),
            top_offset: 0.0,
            pavement: by(&["asphalt", "concrete"]),
            kerb: by(&["stone", "concrete"]),
            sidewalk: by(&["ceramic", "stone", "concrete"]),
        }
    }
}

pub(super) fn material_combo(
    ui: &mut Ui,
    id: Id,
    model: &Model,
    value: MaterialId,
    width: f32,
) -> MaterialId {
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

fn comma(v: f64, d: usize) -> String {
    format!("{v:.d$}").replace('.', ",")
}

/// The species planted in a road's planting strips: the Seville orange
/// of Andalusian streets, or the others usual in them.
pub const STREET_TREES: [&str; 5] = [
    "Naranjo",
    "Jacaranda",
    "Plátano de sombra",
    "Tipuana",
    "Palmera datilera",
];

fn band_combo(ui: &mut Ui, id: Id, value: RoadBand, w: f32) -> RoadBand {
    let mut v = value;
    ComboBox::from_id_salt(id)
        .width(w)
        .selected_text(value.name())
        .show_ui(ui, |ui| {
            for b in RoadBand::ALL {
                ui.selectable_value(&mut v, b, b.name());
            }
        });
    v
}

/// The fields of a road section (H6): its type, its strips across, its
/// parking, kerbs and pavement, its build-outs and crossings; returns the
/// section changed, if it was.
fn section_rows(ui: &mut Ui, salt: u64, s: &RoadSection) -> Option<RoadSection> {
    let mut out = s.clone();
    let mut changed = false;
    row(
        ui,
        "Tipo de vía",
        "Una sección de partida; luego se cambia franja a franja",
        |ui, w| {
            let presets = archisim_core::road_presets();
            let now = presets
                .iter()
                .position(|(_, p)| {
                    p.strips == s.strips
                        && p.single_level == s.single_level
                        && p.parking == s.parking
                })
                .map_or("A medida", |i| presets[i].0);
            ComboBox::from_id_salt(Id::unique(("road-preset", salt)))
                .width(w - 8.0)
                .truncate()
                .selected_text(now)
                .show_ui(ui, |ui| {
                    for (name, p) in &presets {
                        if ui.selectable_label(false, *name).clicked() {
                            let mut n = p.clone();
                            // What is along it and its materials stay.
                            n.buildouts = s.buildouts.clone();
                            n.crossings = s.crossings.clone();
                            n.planting = s.planting;
                            n.bike = s.bike;
                            n.markings = s.markings;
                            out = n;
                            changed = true;
                        }
                    }
                });
        },
    );
    props::note(
        ui,
        "Franjas de la sección, del borde izquierdo al derecho mirando en el sentido del eje:",
    );
    let mut remove = None;
    let mut up = None;
    for (i, st) in s.strips.iter().enumerate() {
        ui.horizontal(|ui| {
            let w = ui.available_width();
            let b = band_combo(
                ui,
                Id::unique(("road-band", salt, i)),
                st.band,
                (w * 0.45).max(80.0),
            );
            if b != st.band {
                out.strips[i].band = b;
                changed = true;
            }
            if let Some(v) = length_field(
                ui,
                Id::unique(("road-w", salt, i)),
                st.width,
                (w * 0.3).max(50.0),
            ) {
                if v > 0.0 {
                    out.strips[i].width = v;
                    changed = true;
                }
            }
            if i > 0
                && ui
                    .small_button("↑")
                    .on_hover_text("Más a la izquierda")
                    .clicked()
            {
                up = Some(i);
            }
            if s.strips.len() > 1
                && ui
                    .small_button("✕")
                    .on_hover_text("Quitar la franja")
                    .clicked()
            {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = up {
        out.strips.swap(i - 1, i);
        changed = true;
    }
    if let Some(i) = remove {
        out.strips.remove(i);
        changed = true;
    }
    if ui
        .button("Añadir franja")
        .on_hover_text("Una acera a la derecha; cámbiala después")
        .clicked()
    {
        out.strips
            .push(archisim_core::strip_of(RoadBand::Sidewalk, 2.0));
        changed = true;
    }
    text_row(ui, "Ancho total (mm)", "", &fmt_mm(out.width()));
    if s.strips.iter().any(|x| x.band == RoadBand::Parking) {
        row(
            ui,
            "Aparcamiento",
            "Cómo aparcan los coches; cambia el ancho de sus franjas al usual",
            |ui, w| {
                let mut v = s.parking;
                ComboBox::from_id_salt(Id::unique(("road-park", salt)))
                    .width(w - 8.0)
                    .selected_text(v.name())
                    .show_ui(ui, |ui| {
                        for m in archisim_core::ParkingMode::ALL {
                            ui.selectable_value(&mut v, m, m.name());
                        }
                    });
                if v != s.parking {
                    out.parking = v;
                    for st in out
                        .strips
                        .iter_mut()
                        .filter(|x| x.band == RoadBand::Parking)
                    {
                        st.width = v.width();
                    }
                    changed = true;
                }
            },
        );
    }
    let mut single = s.single_level;
    if ui
        .checkbox(&mut single, "Plataforma única (todo al mismo nivel)")
        .changed()
    {
        out.single_level = single;
        changed = true;
    }
    for (i, (name, hint, value)) in [
        (
            "Altura del bordillo (mm)",
            "Lo que sube la acera sobre la calzada",
            s.kerb_height,
        ),
        ("Firme (mm)", "Espesor del pavimento de la calzada", s.depth),
        (
            "Radio de esquina (mm)",
            "Radio del bordillo en las esquinas de los cruces con otras calles",
            s.corner_radius,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        row(ui, name, hint, |ui, w| {
            if let Some(v) = length_field(ui, Id::unique(("road-sec", salt, i)), value, w - 8.0) {
                match i {
                    0 => out.kerb_height = v.max(0.0),
                    1 => out.depth = v.max(0.01),
                    _ => out.corner_radius = v.clamp(0.0, 30.0),
                }
                changed = true;
            }
        });
    }
    // Build-outs and crossings along the axis.
    props::note(ui, "Orejas (la acera se ensancha sobre el aparcamiento) y pasos de peatones, a una distancia del inicio del eje:");
    let mut drop_out = None;
    for (i, b) in s.buildouts.iter().enumerate() {
        ui.horizontal(|ui| {
            let mut left = b.left;
            ComboBox::from_id_salt(Id::unique(("road-bo-side", salt, i)))
                .width(90.0)
                .selected_text(if left { "Izquierda" } else { "Derecha" })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut left, true, "Izquierda");
                    ui.selectable_value(&mut left, false, "Derecha");
                });
            if left != b.left {
                out.buildouts[i].left = left;
                changed = true;
            }
            for (k, v) in [b.from, b.to].into_iter().enumerate() {
                if let Some(n) = length_field(ui, Id::unique(("road-bo", salt, i, k)), v, 60.0) {
                    if k == 0 {
                        out.buildouts[i].from = n;
                    } else {
                        out.buildouts[i].to = n;
                    }
                    changed = true;
                }
            }
            if ui.small_button("✕").clicked() {
                drop_out = Some(i);
            }
        });
    }
    if let Some(i) = drop_out {
        out.buildouts.remove(i);
        changed = true;
    }
    let mut drop_cross = None;
    for (i, c) in s.crossings.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label("Paso en");
            if let Some(n) = length_field(ui, Id::unique(("road-cr", salt, i, 0)), c.at, 70.0) {
                out.crossings[i].at = n;
                changed = true;
            }
            ui.label("ancho");
            if let Some(n) = length_field(ui, Id::unique(("road-cr", salt, i, 1)), c.width, 60.0) {
                out.crossings[i].width = n.max(1.0);
                changed = true;
            }
            if ui.small_button("✕").clicked() {
                drop_cross = Some(i);
            }
        });
    }
    if let Some(i) = drop_cross {
        out.crossings.remove(i);
        changed = true;
    }
    ui.horizontal(|ui| {
        let at = out.crossings.last().map_or(10.0, |c| c.at + 20.0);
        if ui.button("Añadir paso de peatones").clicked() {
            out.crossings
                .push(archisim_core::Crossing { at, width: 4.0 });
            changed = true;
        }
        if s.strips.iter().any(|x| x.band == RoadBand::Parking)
            && ui
                .button("Añadir orejas")
                .on_hover_text(
                    "A los dos lados de un tramo de 8 m, donde está el último paso de peatones",
                )
                .clicked()
        {
            let c = out.crossings.last().map_or(10.0, |c| c.at);
            for left in [true, false] {
                out.buildouts.push(archisim_core::Buildout {
                    left,
                    from: c - 4.0,
                    to: c + 4.0,
                });
            }
            changed = true;
        }
    });
    changed.then_some(out)
}

/// The value of an optional material: «—» for none.
fn optional_material(
    ui: &mut Ui,
    id: Id,
    model: &Model,
    value: Option<MaterialId>,
    w: f32,
) -> Option<Option<MaterialId>> {
    let mut v = value;
    let shown = value
        .and_then(|m| model.material(m))
        .map_or("— (el de la acera o la calzada)".to_string(), |m| {
            m.name.clone()
        });
    let mut mats: Vec<(MaterialId, String)> =
        model.materials().map(|m| (m.id, m.name.clone())).collect();
    mats.sort_by_key(|a| a.1.to_lowercase());
    ComboBox::from_id_salt(id)
        .width(w - 8.0)
        .truncate()
        .selected_text(shown)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut v, None, "—");
            for (mid, name) in &mats {
                ui.selectable_value(&mut v, Some(*mid), name);
            }
        });
    (v != value).then_some(v)
}

impl ArchiSimApp {
    // ---- Plot tool ----------------------------------------------------------

    pub(super) fn plot_click(&mut self, p: Point2) {
        match self.plot_tool.click(p) {
            SlabOutcome::Build(outline) => self.build_plot(outline),
            SlabOutcome::Invalid => {
                self.complain("La linde no puede cruzarse: corrige la última esquina.")
            }
            SlabOutcome::Nothing => {}
        }
    }

    pub(super) fn build_plot(&mut self, outline: Vec<Point2>) {
        let level = self.ed.level;
        let setback = self.plot_setback;
        let r = self.ed.transact("Crear parcela", |m| {
            m.add_plot(NewPlot {
                name: m.next_plot_name(),
                level,
                outline,
                setback,
                reference: String::new(),
            })
        });
        match r {
            Ok(id) => {
                self.plot_tool.accept();
                self.ed.selection = [Element::Plot(id)].into_iter().collect();
                let (name, area) = self
                    .ed
                    .model()
                    .plot(id)
                    .map(|p| (p.name.clone(), p.area()))
                    .unwrap_or_default();
                self.say(format!(
                    "{name}: {} m². La referencia catastral y el retranqueo, en Propiedades.",
                    comma(area, 2)
                ));
            }
            Err(e) => self.complain(format!("No se puede crear la parcela: {e}.")),
        }
    }

    // ---- Road tool ----------------------------------------------------------

    pub(super) fn road_click(&mut self, p: Point2) {
        if self.road_points.last().is_some_and(|l| l.approx_eq(p)) {
            self.finish_road();
        } else {
            self.road_points.push(p);
        }
    }

    pub(super) fn finish_road(&mut self) {
        let drawn = std::mem::take(&mut self.road_points);
        if drawn.len() < 2 {
            return;
        }
        let level = self.ed.level;
        let s = self.road_settings.clone();
        let Some(axis) = s.reference.axis(&drawn, &s.section) else {
            self.complain("No se puede crear el vial: el eje dibujado no tiene longitud.");
            return;
        };
        let r = self.ed.transact("Crear vial", |m| {
            m.add_road(NewRoad {
                name: m.next_road_name(),
                level,
                axis,
                section: s.section.clone(),
                top_offset: s.top_offset,
                profile: Vec::new(),

                changes: Vec::new(),
                pavement: s.pavement,
                kerb_material: s.kerb,
                sidewalk_material: s.sidewalk,
            })
        });
        match r {
            Ok(id) => {
                self.ed.selection = [Element::Road(id)].into_iter().collect();
                let (name, len) = self
                    .ed
                    .model()
                    .road(id)
                    .map(|r| (r.name.clone(), r.length()))
                    .unwrap_or_default();
                self.say(format!("{name}: {} m de eje.", comma(len, 2)));
            }
            Err(e) => self.complain(format!("No se puede crear el vial: {e}.")),
        }
    }

    // ---- Preview ------------------------------------------------------------

    pub(super) fn draw_urban_preview(&self, painter: &egui::Painter, rect: Rect) {
        let Some(c) = self.cursor else {
            return;
        };
        let ink = Stroke::new(1.5, ACCENT);
        match self.tool {
            Tool::Plot => {
                let corners = self.plot_tool.corners();
                if corners.is_empty() {
                    return;
                }
                if self.plot_tool.mode == SlabMode::Rectangle {
                    if let Some(o) = self.plot_tool.rectangle_to(c.point) {
                        painter.add(Shape::closed_line(self.polygon(rect, &o), ink));
                        self.draw_label(
                            painter,
                            rect,
                            c.point,
                            format!("{} m²", comma(archisim_core::signed_area(&o).abs(), 2)),
                        );
                    }
                    return;
                }
                let mut pts = corners.to_vec();
                pts.push(self.plot_tool.typed_end(c.point).unwrap_or(c.point));
                let screen = self.polygon(rect, &pts);
                painter.add(Shape::line(screen.clone(), ink));
                if pts.len() >= 3 {
                    dashed(painter, screen[screen.len() - 1], screen[0], ACCENT);
                    self.draw_label(
                        painter,
                        rect,
                        c.point,
                        format!("{} m²", comma(archisim_core::signed_area(&pts).abs(), 2)),
                    );
                }
            }
            Tool::Road => {
                if self.road_points.is_empty() {
                    return;
                }
                let mut drawn = self.road_points.clone();
                drawn.push(c.point);
                let s = self.road_settings.clone();
                // The whole road as it will be: each band filled with its
                // colour, the kerbs drawn, and the line clicked.
                if let Some(axis) = s.reference.axis(&drawn, &s.section) {
                    self.paint_road_bands(painter, rect, &axis, &s.section);
                }
                painter.add(Shape::line(self.polygon(rect, &drawn), ink));
                let len: f64 = drawn.windows(2).map(|x| x[0].distance(x[1])).sum();
                self.draw_label(painter, rect, c.point, format!("L {} m", comma(len, 2)));
            }
            _ => {}
        }
    }

    /// The bands of a road along `axis`, filled segment by segment (each
    /// piece is convex, the whole strip may not be).
    pub(super) fn paint_road_bands(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        axis: &[Point2],
        section: &RoadSection,
    ) {
        for (band, a, b) in section.band_offsets() {
            let (Some(near), Some(far)) = (
                archisim_core::offset_path(axis, a),
                archisim_core::offset_path(axis, b),
            ) else {
                continue;
            };
            let fill = match band {
                RoadBand::Carriageway => Color32::from_rgba_unmultiplied(90, 96, 104, 90),
                RoadBand::Parking => Color32::from_rgba_unmultiplied(140, 146, 154, 80),
                RoadBand::Kerb => Color32::from_rgba_unmultiplied(235, 235, 230, 200),
                RoadBand::Sidewalk => Color32::from_rgba_unmultiplied(214, 196, 160, 110),
                RoadBand::BikeLane => Color32::from_rgba_unmultiplied(200, 110, 90, 100),
                RoadBand::Planting => Color32::from_rgba_unmultiplied(120, 160, 90, 110),
                RoadBand::Median => Color32::from_rgba_unmultiplied(214, 206, 180, 110),
            };
            for i in 0..near.len().saturating_sub(1) {
                let quad = [near[i], near[i + 1], far[i + 1], far[i]];
                painter.add(Shape::convex_polygon(
                    self.polygon(rect, &quad),
                    fill,
                    Stroke::new(0.8, ACCENT.gamma_multiply(0.5)),
                ));
            }
        }
    }

    pub(super) fn urban_hint(&self) -> &'static str {
        match self.tool {
            Tool::Plot if self.plot_tool.is_drawing() && self.plot_tool.mode == SlabMode::Rectangle => {
                "Clic: esquina opuesta · Esc: anular"
            }
            Tool::Plot if self.plot_tool.is_drawing() => {
                "Clic: siguiente esquina de la linde · Clic en la primera o Intro: cerrar · Esc: quitar la última"
            }
            Tool::Plot => "Clic: primera esquina de la linde · Esc: flecha",
            Tool::Road if !self.road_points.is_empty() => match self.road_settings.reference {
                RoadReference::Axis => "Clic: siguiente punto del eje · Doble clic o Intro: terminar · Esc: quitar el último",
                _ => "Clic: siguiente punto del borde · Doble clic o Intro: terminar · Esc: quitar el último",
            },
            Tool::Road => match self.road_settings.reference {
                RoadReference::Axis => "Clic: comienzo del eje del vial (eje o borde, en la barra de arriba; la sección, en Propiedades) · Esc: flecha",
                _ => "Clic: comienzo del borde de la acera (eje o borde, en la barra de arriba; la sección, en Propiedades) · Esc: flecha",
            },
            _ => "",
        }
    }

    // ---- Properties ---------------------------------------------------------

    pub(super) fn plot_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Parcela · lo que vas a dibujar");
        group(ui, "tool-plot", "Parcela", |ui| {
            row(
                ui,
                "Retranqueo (mm)",
                "Lo que el edificio se separa de la linde",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique("tool-plot-setback"),
                        self.plot_setback,
                        w - 8.0,
                    ) {
                        self.plot_setback = v.max(0.0);
                    }
                },
            );
            props::note(
                ui,
                "La superficie de las parcelas es la que usan las masas de Concepto para la edificabilidad y la ocupación.",
            );
        });
    }

    pub(super) fn road_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Vial · lo que vas a dibujar");
        let model = self.ed.model();
        let mut s = self.road_settings.clone();
        group(ui, "tool-road-ref", "Dibujo", |ui| {
            text_row(ui, "Línea dibujada", "", s.reference.name());
            props::note(
                ui,
                "Eje o bordes de las aceras: se elige en «Referencia», en la barra bajo las pestañas.",
            );
        });
        group(ui, "tool-road", "Sección tipo", |ui| {
            if let Some(sec) = section_rows(ui, 0, &s.section) {
                s.section = sec;
            }
            row(ui, "Rasante (mm)", "La calzada sobre la planta", |ui, w| {
                if let Some(v) =
                    length_field(ui, Id::unique("tool-road-top"), s.top_offset, w - 8.0)
                {
                    s.top_offset = v;
                }
            });
        });
        group(ui, "tool-road-mat", "Materiales", |ui| {
            row(ui, "Calzada", "", |ui, w| {
                s.pavement = material_combo(ui, Id::unique("tool-road-pav"), model, s.pavement, w);
            });
            row(ui, "Bordillo", "", |ui, w| {
                s.kerb = material_combo(ui, Id::unique("tool-road-kerb"), model, s.kerb, w);
            });
            row(ui, "Acera", "", |ui, w| {
                s.sidewalk = material_combo(ui, Id::unique("tool-road-side"), model, s.sidewalk, w);
            });
        });
        self.road_settings = s;
    }

    pub(super) fn plot_properties(&mut self, ui: &mut Ui, id: PlotId) {
        let Some(p) = self.ed.model().plot(id).cloned() else {
            return;
        };
        props::title(ui, &format!("{} · 1 seleccionada", p.name));
        group(ui, "plot-data", "Parcela", |ui| {
            row(ui, "Nombre", "", |ui, w| {
                if let Some(t) =
                    text_edit_committed(ui, Id::unique(("plot-name", id)), &p.name, w - 8.0)
                {
                    let r = self.ed.transact("Nombre de la parcela", |m| {
                        m.edit_plot(id, |x| x.name = t.trim().to_string())
                    });
                    self.report(r, "Nombre cambiado.");
                }
            });
            row(ui, "Referencia catastral", "", |ui, w| {
                if let Some(t) =
                    text_edit_committed(ui, Id::unique(("plot-ref", id)), &p.reference, w - 8.0)
                {
                    let r = self.ed.transact("Referencia catastral", |m| {
                        m.edit_plot(id, |x| x.reference = t.trim().to_string())
                    });
                    self.report(r, "Referencia cambiada.");
                }
            });
            row(
                ui,
                "Retranqueo (mm)",
                "Lo que el edificio se separa de la linde",
                |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("plot-setback", id)), p.setback, w - 8.0)
                    {
                        let r = self.ed.transact("Retranqueo", |m| {
                            m.edit_plot(id, |x| x.setback = v.max(0.0))
                        });
                        self.report(r, "Retranqueo cambiado.");
                    }
                },
            );
        });
        group(ui, "plot-figures", "Superficies", |ui| {
            text_row(ui, "Superficie (m²)", "", &comma(p.area(), 2));
            text_row(ui, "Perímetro (m)", "", &comma(p.perimeter(), 2));
            text_row(
                ui,
                "Área edificable (m²)",
                "Dentro del retranqueo",
                &p.buildable()
                    .map_or("— (el retranqueo no cabe)".into(), |b| {
                        comma(archisim_core::signed_area(&b).abs(), 2)
                    }),
            );
            let built: f64 = self
                .ed
                .model()
                .masses()
                .filter(|m| {
                    self.ed
                        .model()
                        .plot_under(&m.outline())
                        .is_some_and(|x| x.id == id)
                })
                .map(|m| m.figures(None).built)
                .sum();
            if built > 0.0 {
                text_row(
                    ui,
                    "Edificabilidad de sus masas",
                    "Lo construido en sus masas de Concepto entre su superficie",
                    &format!("{} m²/m²", comma(built / p.area(), 2)),
                );
            }
        });
        group(ui, "plot-identity", "Datos de identidad", |ui| {
            text_row(ui, "Clase IFC", "", "IfcSite (dentro del emplazamiento)");
        });
    }

    pub(super) fn road_properties(&mut self, ui: &mut Ui, id: RoadId) {
        let model = self.ed.model();
        let Some(r) = model.road(id).cloned() else {
            return;
        };
        props::title(ui, &format!("{} · 1 seleccionado", r.name));
        group(ui, "road-data", "Vial", |ui| {
            row(ui, "Nombre", "", |ui, w| {
                if let Some(t) =
                    text_edit_committed(ui, Id::unique(("road-name", id)), &r.name, w - 8.0)
                {
                    let res = self.ed.transact("Nombre del vial", |m| {
                        m.edit_road(id, |x| x.name = t.trim().to_string())
                    });
                    self.report(res, "Nombre cambiado.");
                }
            });
            self.level_row(
                ui,
                ("road-level", id),
                "Planta",
                Some(r.level),
                &[Element::Road(id)],
            );
            row(ui, "Rasante (mm)", "La calzada sobre la planta", |ui, w| {
                if let Some(v) =
                    length_field(ui, Id::unique(("road-top", id)), r.top_offset, w - 8.0)
                {
                    let res = self.ed.transact("Rasante del vial", |m| {
                        m.edit_road(id, |x| x.top_offset = v)
                    });
                    self.report(res, "Rasante cambiada.");
                }
            });
        });
        group(ui, "road-section", "Sección tipo", |ui| {
            if let Some(sec) = section_rows(ui, id.0, &r.section) {
                let res = self.ed.transact("Sección del vial", |m| {
                    m.edit_road(id, |x| x.section = sec)
                });
                self.report(res, "Sección cambiada.");
            }
        });
        self.road_changes_group(ui, id);
        self.road_profile_group(ui, id);
        group(ui, "road-figures", "Mediciones", |ui| {
            text_row(ui, "Longitud del eje (m)", "", &comma(r.length(), 2));
            for band in RoadBand::ALL {
                let a = r.area_of(band);
                if a > 0.0 {
                    text_row(ui, &format!("{} (m²)", band.name()), "", &comma(a, 2));
                }
            }
        });
        let (p, k, s) = (r.pavement, r.kerb_material, r.sidewalk_material);
        let sec = r.section.clone();
        group(ui, "road-mat", "Materiales", |ui| {
            for (i, (name, v)) in [
                ("Alcorques", sec.planting),
                ("Carril bici", sec.bike),
                ("Marcas viales", sec.markings),
            ]
            .into_iter()
            .enumerate()
            {
                row(ui, name, "", |ui, w| {
                    if let Some(n) = optional_material(
                        ui,
                        Id::unique(("road-opt", id, i)),
                        self.ed.model(),
                        v,
                        w,
                    ) {
                        let res = self.ed.transact("Material del vial", |m| {
                            m.edit_road(id, |x| match i {
                                0 => x.section.planting = n,
                                1 => x.section.bike = n,
                                _ => x.section.markings = n,
                            })
                        });
                        self.report(res, "Material cambiado.");
                    }
                });
            }
            for (i, (name, v)) in [("Calzada", p), ("Bordillo", k), ("Acera", s)]
                .into_iter()
                .enumerate()
            {
                row(ui, name, "", |ui, w| {
                    let n =
                        material_combo(ui, Id::unique(("road-mat", id, i)), self.ed.model(), v, w);
                    if n != v {
                        let res = self.ed.transact("Material del vial", |m| {
                            m.edit_road(id, |x| match i {
                                0 => x.pavement = n,
                                1 => x.kerb_material = n,
                                _ => x.sidewalk_material = n,
                            })
                        });
                        self.report(res, "Material cambiado.");
                    }
                });
            }
        });
        // Trees in its planting strips.
        if sec.strips.iter().any(|x| x.band == RoadBand::Planting) {
            group(ui, "road-trees", "Arbolado", |ui| {
                ui.horizontal_wrapped(|ui| {
                    for name in STREET_TREES {
                        if ui
                            .button(name)
                            .on_hover_text(
                                "Una plantación por cada franja de alcorques, un árbol cada 6 m",
                            )
                            .clicked()
                        {
                            let res = self
                                .ed
                                .transact("Árboles del vial", |m| plant_road_trees(m, id, name));
                            self.report(res.map(drop), "Árboles plantados.");
                        }
                    }
                });
            });
        }
        group(ui, "road-identity", "Datos de identidad", |ui| {
            text_row(
                ui,
                "Clase IFC",
                "",
                "IfcRoad con IfcPavement e IfcKerb (IFC 4.3)",
            );
        });
    }

    /// Whether a mass keeps the setback of the plot it is on: «Dentro»,
    /// «Se sale del retranqueo», or nothing off any plot.
    pub(super) fn mass_setback(&self, outline: &[Point2]) -> Option<&'static str> {
        let model = self.ed.model();
        let plot = model.plot_under(outline)?;
        let b = plot.buildable()?;
        Some(
            if outline.iter().all(|p| {
                crate::pick::contains(&b, *p) || crate::pick::distance_to_outline(&b, *p) < 1e-6
            }) {
                "Dentro del retranqueo"
            } else {
                "Se sale del retranqueo"
            },
        )
    }
}

/// Plantings along the middle of each planting strip of a road: a tree of
/// that species every 6 m.
pub(super) fn plant_road_trees(
    m: &mut Model,
    road: RoadId,
    species: &str,
) -> Result<usize, archisim_core::ModelError> {
    let r = m
        .road(road)
        .cloned()
        .ok_or(archisim_core::ModelError::UnknownElement(road.0))?;
    let Some(t) = archisim_library::flora::add_flora_entry(m, species)? else {
        return Ok(0);
    };
    let mut n = 0;
    for (k, line) in r.strip_lines(RoadBand::Planting).into_iter().enumerate() {
        let len: f64 = line.windows(2).map(|w| w[0].distance(w[1])).sum();
        let count = ((len / 6.0).floor() as usize).max(1);
        m.add_planting(archisim_core::NewPlanting {
            name: format!("{} · {} {}", r.name, species, k + 1),
            level: r.level,
            path: line,
            closed: false,
            smooth: false,
            arrangement: archisim_core::Arrangement::Line,
            count: count as u32,
            species: vec![t],
            size_variation: 0.15,
            shape_variation: 0.3,
            spread: 0.0,
            seed: 7 + k as u64,
            base_offset: r.top_offset + r.section.kerb_height,
        })?;
        n += 1;
    }
    Ok(n)
}

impl ArchiSimApp {
    /// The road's longitudinal profile («Rasante», U1): its points along
    /// the axis with their heights and vertical curves, the grades between
    /// them, and taking it from the terrain.
    fn road_profile_group(&mut self, ui: &mut Ui, id: RoadId) {
        let Some(r) = self.ed.model().road(id).cloned() else {
            return;
        };
        let terrain = self.ed.model().terrains_on_level(r.level).next().is_some();
        group(ui, "road-profile", "Rasante", |ui| {
            props::note(
                ui,
                "Cota del eje de la calzada a lo largo de la calle (sobre la planta y su desfase) y el acuerdo vertical en cada punto. Sin puntos, la calle es horizontal.",
            );
            let mut p = r.profile.clone();
            p.sort_by(|a, b| a.at.total_cmp(&b.at));
            let mut out = p.clone();
            let mut changed = false;
            let mut drop = None;
            for (i, q) in p.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label("en");
                    if let Some(v) = length_field(ui, Id::unique(("prof-at", id, i)), q.at, 60.0) {
                        out[i].at = v.clamp(0.0, r.length());
                        changed = true;
                    }
                    ui.label("cota");
                    if let Some(v) = length_field(ui, Id::unique(("prof-z", id, i)), q.z, 60.0) {
                        out[i].z = v;
                        changed = true;
                    }
                    ui.label("acuerdo");
                    if let Some(v) = length_field(ui, Id::unique(("prof-k", id, i)), q.curve, 50.0)
                    {
                        out[i].curve = v.max(0.0);
                        changed = true;
                    }
                    if ui.small_button("✕").clicked() {
                        drop = Some(i);
                    }
                });
                if let Some(w) = p.get(i + 1) {
                    if w.at - q.at > 1e-6 {
                        let g = (w.z - q.z) / (w.at - q.at) * 100.0;
                        ui.label(
                            egui::RichText::new(format!(
                                "    pendiente {} %",
                                format!("{g:.2}").replace('.', ",")
                            ))
                            .small(),
                        );
                    }
                }
            }
            if let Some(i) = drop {
                out.remove(i);
                changed = true;
            }
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(terrain, egui::Button::new("Tomar del terreno"))
                    .on_hover_text("Un punto cada 20 m con la cota del terreno bajo el eje, con acuerdos de 20 m")
                    .clicked()
                {
                    let res = self.ed.transact("Rasante del terreno", |m| {
                        let prof = m
                            .road_profile_from_terrain(id, 20.0)
                            .ok_or(archisim_core::ModelError::InvalidShape("no hay terreno bajo la calle"))?;
                        m.edit_road(id, |x| x.profile = prof)
                    });
                    self.report(res, "Rasante tomada del terreno.");
                }
                if ui.button("Añadir punto").clicked() {
                    let len = r.length();
                    let at = match (out.first(), out.last()) {
                        (None, _) => {
                            out.push(archisim_core::ProfilePoint { at: 0.0, z: 0.0, curve: 0.0 });
                            len
                        }
                        (Some(_), Some(l)) if l.at < len - 1.0 => len,
                        _ => len / 2.0,
                    };
                    let z = r.profile_at(at);
                    out.push(archisim_core::ProfilePoint { at, z, curve: 0.0 });
                    changed = true;
                }
                if !out.is_empty() && ui.button("Horizontal").clicked() {
                    out.clear();
                    changed = true;
                }
            });
            if changed {
                let res = self
                    .ed
                    .transact("Rasante del vial", |m| m.edit_road(id, |x| x.profile = out));
                self.report(res, "Rasante cambiada.");
            }
        });
    }
}

impl ArchiSimApp {
    /// Where the road's strips change along it («Cambios de sección», U2):
    /// from a distance, with a transition, other strips — of a street type
    /// or changed one by one.
    fn road_changes_group(&mut self, ui: &mut Ui, id: RoadId) {
        let Some(r) = self.ed.model().road(id).cloned() else {
            return;
        };
        group(ui, "road-changes", "Cambios de sección", |ui| {
            props::note(
                ui,
                "Desde una distancia del inicio del eje, la calle pasa a otra sección. Con las mismas franjas, sus anchos cambian poco a poco a lo largo de la transición; si no, cambia de golpe en su mitad.",
            );
            let mut out = r.changes.clone();
            out.sort_by(|a, b| a.at.total_cmp(&b.at));
            let before = out.clone();
            let mut drop = None;
            for (i, c) in before.iter().enumerate() {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Desde");
                    if let Some(v) = length_field(ui, Id::unique(("chg-at", id, i)), c.at, 60.0) {
                        out[i].at = v.clamp(0.0, r.length());
                    }
                    ui.label("transición");
                    if let Some(v) =
                        length_field(ui, Id::unique(("chg-tr", id, i)), c.transition, 50.0)
                    {
                        out[i].transition = v.max(0.0);
                    }
                    if ui
                        .small_button("✕")
                        .on_hover_text("Quitar el cambio")
                        .clicked()
                    {
                        drop = Some(i);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Tipo");
                    ComboBox::from_id_salt(Id::unique(("chg-preset", id, i)))
                        .width(ui.available_width() - 8.0)
                        .truncate()
                        .selected_text(format!(
                            "{} franjas · {}",
                            c.strips.len(),
                            fmt_mm(c.strips.iter().map(|x| x.width).sum())
                        ))
                        .show_ui(ui, |ui| {
                            for (name, p) in archisim_core::road_presets() {
                                if ui.selectable_label(false, name).clicked() {
                                    out[i].strips = p.strips.clone();
                                }
                            }
                        });
                });
                let mut remove = None;
                for (k, st) in c.strips.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let w = ui.available_width();
                        let b = band_combo(
                            ui,
                            Id::unique(("chg-band", id, i, k)),
                            st.band,
                            (w * 0.45).max(80.0),
                        );
                        if b != st.band {
                            out[i].strips[k].band = b;
                        }
                        if let Some(v) = length_field(
                            ui,
                            Id::unique(("chg-w", id, i, k)),
                            st.width,
                            (w * 0.3).max(50.0),
                        ) {
                            if v > 0.0 {
                                out[i].strips[k].width = v;
                            }
                        }
                        if c.strips.len() > 1 && ui.small_button("✕").clicked() {
                            remove = Some(k);
                        }
                    });
                }
                if let Some(k) = remove {
                    out[i].strips.remove(k);
                }
            }
            if let Some(i) = drop {
                out.remove(i);
            }
            if ui
                .button("Añadir cambio de sección")
                .on_hover_text(
                    "A mitad de la calle, con la sección que tiene allí; cámbiala después",
                )
                .clicked()
            {
                let at = (r.length() / 2.0).round();
                out.push(archisim_core::SectionChange {
                    at,
                    transition: 10.0,
                    strips: r.strips_at(at),
                });
            }
            let mut to_plots = false;
            if ui
                .button("Aceras hasta las parcelas")
                .on_hover_text(
                    "Lleva las aceras de los dos lados hasta la línea de fachada de las parcelas: un cambio de sección donde la línea se mueve más de 25 cm. La calzada no se mueve. Sustituye los cambios que haya.",
                )
                .clicked()
            {
                to_plots = true;
            }
            if to_plots {
                match self.ed.model().sidewalks_to_plots(id) {
                    Some(changes) => {
                        let n = changes.len();
                        let res = self.ed.transact("Aceras hasta las parcelas", |m| {
                            m.edit_road(id, |x| x.changes = changes)
                        });
                        self.report(
                            res,
                            &format!("Aceras hasta las parcelas: {n} cambios de sección."),
                        );
                    }
                    None => self
                        .complain("No hay parcelas junto a la calle, o no tiene acera por fuera."),
                }
                return;
            }
            if out != before {
                let res = self.ed.transact("Cambios de sección", |m| {
                    m.edit_road(id, |x| x.changes = out)
                });
                self.report(res, "Cambios de sección guardados.");
            }
        });
    }
}
