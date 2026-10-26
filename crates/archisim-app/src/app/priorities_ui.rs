//! The intersection priorities of the project ("Prioridades de
//! intersección"), as ArchiCAD shows them: one bar per material from 0 to
//! 999, longer wins. Where two elements meet (a wall butting into another,
//! a wall through a slab), each layer keeps its room against layers of
//! lower priority. A material without a priority of its own takes that of
//! the function of each layer it is in; those defaults are marked on the
//! scale.

use super::widgets::SwitchExt as _;
use archisim_core::{LayerFunction, MaterialId, MAX_PRIORITY};
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Rect, Sense, Stroke, Ui, Vec2};

use super::props;
use super::theme::Palette;
use super::ArchiSimApp;

/// The defaults by layer function, as marked on the scale.
const DEFAULTS: [(LayerFunction, &str); 5] = [
    (LayerFunction::Finish, "Acabado"),
    (LayerFunction::Membrane, "Lámina"),
    (LayerFunction::Insulation, "Aislante"),
    (LayerFunction::Substrate, "Soporte"),
    (LayerFunction::Structure, "Estructura"),
];

/// One material as the window lists it.
struct Row {
    id: MaterialId,
    name: String,
    color: [u8; 3],
    /// Its own priority, if it has one.
    own: Option<u16>,
    /// The defaults of the functions of the layers it is in.
    by_function: Vec<u16>,
    /// Where it is used: "Fachada (estructura)".
    uses: Vec<String>,
}

impl Row {
    /// What it ranks by: its own priority, or the highest of its layers'.
    fn rank(&self) -> u16 {
        self.own
            .or_else(|| self.by_function.iter().copied().max())
            .unwrap_or(0)
    }
}

fn function_name(f: LayerFunction) -> &'static str {
    match f {
        LayerFunction::Structure => "estructura",
        LayerFunction::Substrate => "soporte",
        LayerFunction::Insulation => "aislante",
        LayerFunction::AirGap => "cámara",
        LayerFunction::Membrane => "lámina",
        LayerFunction::Finish => "acabado",
    }
}

/// Widths of the columns (points): swatch, name and the bar's scale.
const SWATCH: f32 = 14.0;
const NAME: f32 = 200.0;
const BAR: f32 = 300.0;

impl ArchiSimApp {
    fn priority_rows(&self) -> Vec<Row> {
        let model = self.ed.model();
        let mut rows: Vec<Row> = model
            .materials()
            .map(|m| Row {
                id: m.id,
                name: m.name.clone(),
                color: m.cut_color,
                own: m.priority,
                by_function: Vec::new(),
                uses: Vec::new(),
            })
            .collect();
        let layers = model
            .wall_types()
            .map(|t| (t.name.clone(), t.layers.clone()))
            .chain(
                model
                    .slab_types()
                    .map(|t| (t.name.clone(), t.layers.clone())),
            );
        for (name, layers) in layers {
            for l in layers {
                if let Some(r) = rows.iter_mut().find(|r| r.id == l.material) {
                    let d = l.function.default_priority();
                    if !r.by_function.contains(&d) {
                        r.by_function.push(d);
                    }
                    let what = format!("{name} ({})", function_name(l.function));
                    if !r.uses.contains(&what) {
                        r.uses.push(what);
                    }
                }
            }
        }
        rows.sort_by(|a, b| b.rank().cmp(&a.rank()).then(a.name.cmp(&b.name)));
        rows
    }

    pub(super) fn priorities_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_priorities {
            return;
        }
        let mut open = true;
        super::widgets::dialog("Prioridades de intersección")
            .open(&mut open)
            .default_width(680.0)
            .default_height(560.0)
            .resizable(true)
            .show(ctx, |ui| self.priorities_table(ui));
        self.show_priorities = open && self.show_priorities;
    }

    fn priorities_table(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        ui.label(
            egui::RichText::new(
                "Donde dos elementos se encuentran, cada capa se queda el sitio frente a las capas de menor prioridad (barra más larga, manda):\n· en una T, las capas del muro que llega atraviesan las del otro muro que pierden;\n· entre muro y forjado, la capa del muro entra en el forjado mientras gane y se para en la primera que pierde; a igualdad, gana el forjado.\nUn material sin prioridad propia (barra gris) toma la de la función de cada capa en que está: las marcas de la escala.",
            )
            .color(p.muted),
        );
        ui.add_space(6.0);
        let rows = self.priority_rows();
        let mut set: Option<(MaterialId, Option<u16>)> = None;
        let mut reset_all = false;
        ui.horizontal(|ui| {
            ui.switch(
                &mut self.priorities_all,
                "Ver también los materiales sin usar",
            );
            ui.add_space(12.0);
            let any_own = rows.iter().any(|r| r.own.is_some());
            if ui
                .add_enabled(
                    any_own,
                    egui::Button::new("Todas según la función de la capa"),
                )
                .on_hover_text("Quita las prioridades propias: cada capa toma la de su función.")
                .clicked()
            {
                reset_all = true;
            }
        });
        ui.add_space(4.0);
        // The scale, with the defaults by function.
        ui.horizontal(|ui| {
            // Laid out as the rows: swatch, name, then the bar.
            ui.allocate_exact_size(Vec2::new(SWATCH, 1.0), Sense::hover());
            ui.allocate_exact_size(Vec2::new(NAME, 1.0), Sense::hover());
            let (rect, _) = ui.allocate_exact_size(Vec2::new(BAR, 30.0), Sense::hover());
            let painter = ui.painter_at(rect.expand(30.0));
            for (f, name) in DEFAULTS {
                let v = f.default_priority();
                let x = rect.left() + BAR * v as f32 / MAX_PRIORITY as f32;
                painter.line_segment(
                    [
                        egui::pos2(x, rect.bottom() - 8.0),
                        egui::pos2(x, rect.bottom()),
                    ],
                    Stroke::new(1.0, p.faint),
                );
                painter.text(
                    egui::pos2(x, rect.bottom() - 10.0),
                    Align2::CENTER_BOTTOM,
                    format!("{name}\n{v}"),
                    FontId::proportional(10.0),
                    p.muted,
                );
            }
        });
        egui::ScrollArea::vertical().show(ui, |ui| {
            for r in &rows {
                if r.uses.is_empty() && !self.priorities_all {
                    continue;
                }
                if let Some(change) = self.priority_row(ui, r, p) {
                    set = Some((r.id, change));
                }
                ui.add_space(2.0);
            }
        });
        if reset_all {
            let r = self.ed.reset_material_priorities();
            self.report(
                r,
                "Prioridades según la función de cada capa: las uniones y los encuentros se rehacen.",
            );
        }
        if let Some((id, v)) = set {
            let r = self.ed.set_material_priority(id, v);
            self.report(
                r,
                "Prioridad cambiada: las uniones en T y los encuentros con forjados se rehacen con ella.",
            );
        }
    }

    /// One material: swatch, name and uses, its bar (drag or click to set
    /// it) and its value; `↺` gives it back to its layers' functions.
    /// Returns the change to make, once the drag ends.
    fn priority_row(&mut self, ui: &mut Ui, r: &Row, p: Palette) -> Option<Option<u16>> {
        let mut change = None;
        ui.horizontal(|ui| {
            props::swatch(ui, r.color, SWATCH);
            // The name in a column of its own width, cut short if long.
            let (name_rect, name) = ui.allocate_exact_size(Vec2::new(NAME, 18.0), Sense::hover());
            let color = if r.uses.is_empty() { p.faint } else { p.text };
            let mut job = egui::text::LayoutJob::simple_singleline(
                r.name.clone(),
                egui::TextStyle::Body.resolve(ui.style()),
                color,
            );
            job.wrap = egui::text::TextWrapping::truncate_at_width(NAME - 6.0);
            let galley = ui.painter().layout_job(job);
            let at = egui::pos2(
                name_rect.left(),
                name_rect.center().y - galley.size().y / 2.0,
            );
            ui.painter().galley(at, galley, color);
            let uses = if r.uses.is_empty() {
                "No lo usa ningún tipo de muro ni de forjado.".to_string()
            } else {
                format!("Se usa en: {}", r.uses.join(", "))
            };
            name.on_hover_text(uses);
            let (rect, resp) =
                ui.allocate_exact_size(Vec2::new(BAR, 18.0), Sense::click_and_drag());
            let at = |x: f32| -> u16 {
                let t = ((x - rect.left()) / BAR).clamp(0.0, 1.0);
                ((t * MAX_PRIORITY as f32 / 10.0).round() * 10.0).min(MAX_PRIORITY as f32) as u16
            };
            let dragging = self
                .priority_drag
                .filter(|(id, _)| *id == r.id)
                .map(|(_, v)| v);
            if let Some(pos) = resp.interact_pointer_pos() {
                if resp.dragged() || resp.drag_started() {
                    self.priority_drag = Some((r.id, at(pos.x)));
                }
                if resp.clicked() {
                    change = Some(Some(at(pos.x)));
                }
            }
            if resp.drag_stopped() {
                if let Some(v) = dragging {
                    change = Some(Some(v));
                }
                self.priority_drag = None;
            }
            let shown = dragging.or(r.own);
            let painter = ui.painter_at(rect);
            painter.rect_filled(
                rect.shrink2(Vec2::new(0.0, 4.0)),
                CornerRadius::same(3),
                p.raised,
            );
            for (f, _) in DEFAULTS {
                let x = rect.left() + BAR * f.default_priority() as f32 / MAX_PRIORITY as f32;
                painter.line_segment(
                    [
                        egui::pos2(x, rect.top() + 2.0),
                        egui::pos2(x, rect.bottom() - 2.0),
                    ],
                    Stroke::new(1.0, p.line),
                );
            }
            let bar = |v: u16, color: Color32| {
                let w = BAR * v as f32 / MAX_PRIORITY as f32;
                if w > 0.5 {
                    let r = Rect::from_min_size(
                        rect.left_top() + Vec2::new(0.0, 4.0),
                        Vec2::new(w, rect.height() - 8.0),
                    );
                    painter.rect_filled(r, CornerRadius::same(3), color);
                }
            };
            match shown {
                Some(v) => bar(v, p.accent),
                None => {
                    // By function: a grey bar to each function's default.
                    let mut vs = r.by_function.clone();
                    vs.sort_unstable();
                    for v in vs.iter().rev() {
                        bar(*v, p.faint.gamma_multiply(0.6));
                    }
                }
            }
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            resp.on_hover_text("Arrastra o haz clic para darle prioridad propia (de 10 en 10).");
            let text = match (shown, r.by_function.as_slice()) {
                (Some(v), _) => v.to_string(),
                (None, []) => "—".to_string(),
                (None, [v]) => format!("{v} (función)"),
                (None, vs) => format!(
                    "{}–{} (según capa)",
                    vs.iter().min().unwrap_or(&0),
                    vs.iter().max().unwrap_or(&0)
                ),
            };
            ui.add_sized(
                [110.0, 18.0],
                egui::Label::new(egui::RichText::new(text).color(if shown.is_some() {
                    p.text
                } else {
                    p.muted
                })),
            );
            if r.own.is_some()
                && ui
                    .small_button("↺")
                    .on_hover_text("Según la función de cada capa")
                    .clicked()
            {
                change = Some(None);
            }
        });
        change
    }
}
