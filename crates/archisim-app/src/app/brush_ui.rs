//! The Exterior brush on the screen (C5, one brush in C8): the head under
//! the cursor, in the plan and in 3D; the stroke while the button is held
//! (a dab every little distance, all in one step of Deshacer); and its
//! panel, as in Paralives: what it does (paint, rub out, raise, lower,
//! smooth, flatten, scatter), its head, its size and strength, and the
//! palette of covers or the type scattered. The panel is the same in
//! Propiedades (Técnica) and floating over the 3D (Maqueta).

use archisim_core::Point2;
use archisim_library::ground::{CoverGroup, GROUND_COVERS};
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, Ui, Vec2};

use super::glyphs::{self, Glyph};
use super::theme::Palette;
use super::widgets::SwitchExt;
use super::*;
use crate::brush::{self, BrushHead, BrushMode};

/// A stroke being made: where Deshacer starts it, the last dab, the
/// height it flattens to, how many dabs so far and what it made or took.
#[derive(Debug, Clone, Copy)]
pub struct BrushStroke {
    mark: usize,
    last: Point2,
    flat: f64,
    dabs: u64,
    things: usize,
}

/// The glyph of what the brush does.
pub(super) fn mode_glyph(m: BrushMode) -> Glyph {
    match m {
        BrushMode::Paint => Glyph::Brush,
        BrushMode::Erase => Glyph::Eraser,
        BrushMode::Raise => Glyph::Raise,
        BrushMode::Lower => Glyph::Lower,
        BrushMode::Smooth => Glyph::Smooth,
        BrushMode::Flatten => Glyph::Flatten,
        BrushMode::Scatter => Glyph::Scatter,
    }
}

fn head_glyph(h: BrushHead) -> Glyph {
    match h {
        BrushHead::Round => Glyph::HeadRound,
        BrushHead::Square => Glyph::HeadSquare,
        BrushHead::Soft => Glyph::HeadSoft,
        BrushHead::Splatter => Glyph::HeadSplatter,
    }
}

/// Width of the floating panel of the Maqueta.
const PANEL_W: f32 = 300.0;

impl ArchiSimApp {
    /// What a click does with the brush, for the bar.
    pub(super) fn brush_hint(&self) -> &'static str {
        match self.brush.mode {
            BrushMode::Paint => {
                "Arrastra: pinta sobre el terreno lo elegido en la paleta (sin terreno, se hace uno llano) · Esc: flecha"
            }
            BrushMode::Erase => "Arrastra: borra lo pintado · Esc: flecha",
            BrushMode::Raise | BrushMode::Lower => {
                "Arrastra sobre el suelo: sube o baja, más en el centro · Tamaño y fuerza en el panel · Esc: flecha"
            }
            BrushMode::Smooth => "Arrastra: suaviza el terreno · Esc: flecha",
            BrushMode::Flatten => {
                "Arrastra: deja el terreno a la altura del punto donde empiezas · Esc: flecha"
            }
            BrushMode::Scatter => {
                "Arrastra: esparce plantas o rocas del tipo elegido en el panel · Esc: flecha"
            }
        }
    }

    /// Takes the brush (and the tool), doing `mode`.
    pub(super) fn take_brush(&mut self, mode: BrushMode) {
        self.set_tool(Tool::Brush);
        self.set_brush_mode(mode);
    }

    /// Changes what the brush does, with the strength that suits it.
    pub(super) fn set_brush_mode(&mut self, mode: BrushMode) {
        if self.brush.mode != mode {
            self.brush.strength = match mode {
                BrushMode::Raise | BrushMode::Lower => 0.15,
                BrushMode::Scatter => 1.0,
                _ => 0.5,
            };
            if mode == BrushMode::Scatter && self.brush.radius < 3.0 {
                self.brush.radius = 4.0;
            }
        }
        self.brush.mode = mode;
    }

    /// The type the brush scatters: the one chosen, or the first plant of
    /// the project, or an olive tree from the library.
    fn brush_species(&mut self) -> Option<archisim_core::ObjectTypeId> {
        if let Some(t) = self
            .brush
            .species
            .filter(|t| self.ed.model().object_type(*t).is_some())
        {
            return Some(t);
        }
        // A grown species first (they look like trees), then any plant.
        let first = |model: &archisim_core::Model, grown: bool| {
            model
                .object_types()
                .find(|t| {
                    t.own.is_none()
                        && brush::is_plant(model, t.id)
                        && model.type_grows(t.id) == grown
                })
                .map(|t| t.id)
        };
        let t = match first(self.ed.model(), true) {
            Some(t) => t,
            None => {
                let olive = self
                    .ed
                    .transact("Cargar especie del vivero", |m| {
                        archisim_library::flora::add_flora_entry(m, "Olivo")
                    })
                    .ok()
                    .flatten();
                olive.or_else(|| first(self.ed.model(), false))?
            }
        };
        self.brush.species = Some(t);
        Some(t)
    }

    /// The mouse with the brush: a dab when the button goes down and every
    /// little way while it is held; on letting go, the stroke is one step.
    pub(super) fn brush_clicks(&mut self, ctx: &egui::Context, response: &egui::Response) {
        let down = response.is_pointer_button_down_on() && ctx.input(|i| i.pointer.primary_down());
        let Some(at) = self.cursor.map(|c| c.point) else {
            if !down {
                self.end_brush_stroke();
            }
            return;
        };
        if !down {
            self.end_brush_stroke();
            return;
        }
        let r = self.brush.radius.max(0.2);
        match self.brush_stroke {
            None => {
                let mark = self.ed.undo_mark();
                let level = self.ed.level;
                let flat = self
                    .ed
                    .model()
                    .terrains_on_level(level)
                    .find_map(|t| t.height_at(at))
                    .unwrap_or_else(|| self.ed.model().level(level).map_or(0.0, |l| l.elevation));
                self.brush_stroke = Some(BrushStroke {
                    mark,
                    last: at,
                    flat,
                    dabs: 0,
                    things: 0,
                });
                self.brush_dab(at);
            }
            Some(s) if s.last.distance(at) >= r * brush::DAB_STEP => {
                // Every step of the way from the last dab (the mouse may
                // have gone far in one frame): a continuous stroke.
                let step = r * brush::DAB_STEP;
                let d = at - s.last;
                let n = (d.length() / step).floor().clamp(1.0, 200.0) as usize;
                for i in 1..=n {
                    let q = if i == n {
                        at
                    } else {
                        s.last + d * (i as f64 / n as f64)
                    };
                    self.brush_dab(q);
                }
            }
            Some(_) => {}
        }
        ctx.request_repaint();
    }

    /// One dab of the brush at `at`.
    fn brush_dab(&mut self, at: Point2) {
        let Some(mut s) = self.brush_stroke else {
            return;
        };
        let b = self.brush;
        let level = self.ed.level;
        let types = if b.mode == BrushMode::Scatter {
            match self.brush_species() {
                Some(t) => brush::scatter_types(self.ed.model(), t, b.vary),
                None => Vec::new(),
            }
        } else {
            Vec::new()
        };
        let seed = 0x9E37_79B9 ^ s.dabs.wrapping_mul(0x2545_F491) ^ (at.x * 1000.0) as u64;
        let dab = brush::dab_shape(b.head, at, b.radius, seed);
        let r = self.ed.transact(b.mode.undo_label(), |m| match b.mode {
            BrushMode::Paint => {
                let t = brush::ensure_terrain(m, level, at)?;
                let mat = brush::cover_material(m, b.cover);
                brush::paint(m, t, mat, dab).map(|_| 0)
            }
            BrushMode::Erase => brush::erase(m, level, dab, b.erase_plants),
            BrushMode::Scatter => {
                brush::ensure_terrain(m, level, at)?;
                brush::scatter(m, level, &types, &dab, at, b.radius, b.strength, seed)
                    .map(|v| v.len())
            }
            mode => {
                let t = brush::ensure_terrain(m, level, at)?;
                brush::sculpt(m, t, mode, at, b.radius, b.strength, s.flat).map(|_| 0)
            }
        });
        match r {
            Ok(n) => {
                s.things += n;
                s.dabs += 1;
                s.last = at;
                self.brush_stroke = Some(s);
            }
            Err(e) => {
                self.brush_stroke = Some(BrushStroke { last: at, ..s });
                self.complain(format!("No se puede: {e}."));
            }
        }
    }

    /// Ends the stroke being made: all its dabs, one step of Deshacer.
    pub(super) fn end_brush_stroke(&mut self) {
        let Some(s) = self.brush_stroke.take() else {
            return;
        };
        if s.dabs == 0 {
            return;
        }
        let label = match self.brush.mode {
            BrushMode::Paint => format!(
                "Pintar {}",
                GROUND_COVERS[self.brush.cover.min(GROUND_COVERS.len() - 1)]
                    .name
                    .to_lowercase()
            ),
            m => m.undo_label().to_string(),
        };
        self.ed.join_since(s.mark, &label);
        match self.brush.mode {
            BrushMode::Scatter => self.say(format!("{} esparcidos.", s.things)),
            BrushMode::Erase if s.things > 0 => {
                self.say(format!("Borrado, con {} plantas o rocas.", s.things))
            }
            _ => self.say(format!("{label}.")),
        }
    }

    /// The outline of the head round `centre` (in plan), as points to draw.
    fn head_outline(&self, centre: Point2) -> Vec<Vec<Point2>> {
        let r = self.brush.radius;
        if self.brush.mode.has_head() && self.brush.head == BrushHead::Square {
            let c = |dx: f64, dy: f64| centre + Point2::new(dx * r, dy * r);
            return vec![vec![
                c(-1.0, -1.0),
                c(1.0, -1.0),
                c(1.0, 1.0),
                c(-1.0, 1.0),
                c(-1.0, -1.0),
            ]];
        }
        let ring = |k: f64| -> Vec<Point2> {
            (0..=48)
                .map(|i| {
                    let a = i as f64 / 48.0 * std::f64::consts::TAU;
                    centre + Point2::new(a.cos(), a.sin()) * (r * k)
                })
                .collect()
        };
        let inner = match (self.brush.mode.has_head(), self.brush.head) {
            (false, _) => 0.5,
            (true, BrushHead::Soft) => 0.72,
            _ => 0.0,
        };
        let mut v = vec![ring(1.0)];
        if inner > 0.0 {
            v.push(ring(inner));
        }
        v
    }

    /// The brush's head in the plan.
    pub(super) fn draw_brush_plan(&self, painter: &egui::Painter, rect: Rect) {
        if self.tool != Tool::Brush {
            return;
        }
        let Some(c) = self.cursor else {
            return;
        };
        let colour = brush_colour(self.brush.mode);
        for (i, ring) in self.head_outline(c.point).into_iter().enumerate() {
            let pts: Vec<Pos2> = ring.iter().map(|p| self.view.to_screen(rect, *p)).collect();
            painter.add(egui::Shape::line(
                pts,
                Stroke::new(if i == 0 { 1.5 } else { 0.8 }, colour),
            ));
        }
    }

    /// The brush's head in 3D, on the ground under it.
    pub(super) fn draw_brush_3d(&self, painter: &egui::Painter, rect: Rect) {
        if self.tool != Tool::Brush {
            return;
        }
        let Some(c) = self.cursor3d else {
            return;
        };
        let model = self.ed.model();
        let level = self.ed.level;
        let z0 = model.level(level).map_or(0.0, |l| l.elevation);
        let terrain = model.terrains_on_level(level).next();
        let size = glam::Vec2::new(rect.width(), rect.height());
        let centre = Point2::new(c.world.x as f64, c.world.y as f64);
        let colour = brush_colour(self.brush.mode);
        for (i, ring) in self.head_outline(centre).into_iter().enumerate() {
            let pts: Vec<Pos2> = ring
                .iter()
                .filter_map(|p| {
                    let z = terrain.and_then(|t| t.height_at(*p)).unwrap_or(z0) + 0.03;
                    self.v3d
                        .camera
                        .project_to_screen(glam::Vec3::new(p.x as f32, p.y as f32, z as f32), size)
                        .map(|s| Pos2::new(rect.min.x + s.x, rect.min.y + s.y))
                })
                .collect();
            painter.add(egui::Shape::line(
                pts,
                Stroke::new(if i == 0 { 2.0 } else { 1.0 }, colour),
            ));
        }
    }

    /// Size and strength, on the bar after the modes.
    pub(super) fn brush_fields(&mut self, ui: &mut Ui) {
        if self.tool != Tool::Brush {
            return;
        }
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Tamaño").small());
        ui.add(
            egui::DragValue::new(&mut self.brush.radius)
                .range(0.2..=30.0)
                .speed(0.05)
                .suffix(" m"),
        );
        if let Some((label, speed, max, suffix)) = strength_of(self.brush.mode) {
            ui.add_space(4.0);
            ui.label(egui::RichText::new(label).small());
            ui.add(
                egui::DragValue::new(&mut self.brush.strength)
                    .range(0.01..=max)
                    .speed(speed)
                    .suffix(suffix),
            );
        }
    }

    /// The brush in Propiedades (Técnica).
    pub(super) fn brush_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Pincel de Exterior");
        ui.add_space(4.0);
        self.brush_panel(ui);
    }

    /// The brush's panel floating over the 3D of the Maqueta, at its
    /// left, while the brush is in hand.
    pub(super) fn brush_panel_maquette(&mut self, ctx: &egui::Context, top: f32, bottom: f32) {
        if self.tool != Tool::Brush {
            return;
        }
        let p = Palette::current(ctx);
        egui::Area::new(egui::Id::unique("brush-panel"))
            .order(egui::Order::Foreground)
            .fixed_pos(Pos2::new(14.0, top + 14.0))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(p.panel)
                    .stroke(Stroke::new(1.0, p.line))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.set_width(PANEL_W);
                        let h = (bottom - top - 50.0).max(200.0);
                        egui::ScrollArea::vertical()
                            .max_height(h)
                            .min_scrolled_height(h)
                            .show(ui, |ui| self.brush_panel(ui));
                    });
            });
    }

    /// What the brush does, its head, size and strength, and the palette
    /// (or the type scattered).
    pub(super) fn brush_panel(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        // What it does.
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(3.0, 3.0);
            for m in BrushMode::ALL {
                let on = self.brush.mode == m;
                if glyph_button(ui, mode_glyph(m), on, p)
                    .on_hover_text(format!("{} — {}", m.name(), m.hint()))
                    .clicked()
                {
                    self.set_brush_mode(m);
                }
            }
        });
        ui.label(
            egui::RichText::new(self.brush.mode.name())
                .strong()
                .color(p.text),
        );
        // The head.
        if self.brush.mode.has_head() {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(3.0, 3.0);
                ui.label(egui::RichText::new("Cabezal").small().color(p.muted));
                for h in BrushHead::ALL {
                    if glyph_button(ui, head_glyph(h), self.brush.head == h, p)
                        .on_hover_text(h.name())
                        .clicked()
                    {
                        self.brush.head = h;
                    }
                }
            });
        }
        // Size and strength.
        ui.add_space(4.0);
        egui::Grid::new("brush-sliders")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Tamaño").small().color(p.muted));
                ui.add(
                    egui::Slider::new(&mut self.brush.radius, 0.2..=15.0)
                        .logarithmic(true)
                        .suffix(" m")
                        .max_decimals(2),
                );
                ui.end_row();
                if let Some((label, _, max, suffix)) = strength_of(self.brush.mode) {
                    ui.label(egui::RichText::new(label).small().color(p.muted));
                    ui.add(
                        egui::Slider::new(&mut self.brush.strength, 0.01..=max)
                            .suffix(suffix)
                            .max_decimals(2),
                    );
                    ui.end_row();
                }
            });
        ui.add_space(6.0);
        match self.brush.mode {
            BrushMode::Paint => self.cover_palette(ui),
            BrushMode::Erase => {
                ui.switch(
                    &mut self.brush.erase_plants,
                    "Borrar también plantas y rocas",
                );
            }
            BrushMode::Scatter => self.scatter_choice(ui),
            _ => {
                ui.label(
                    egui::RichText::new(self.brush.mode.hint())
                        .small()
                        .color(p.muted),
                );
            }
        }
    }

    /// The covers to paint, as swatches: lawns and meadows, then grounds.
    fn cover_palette(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        for (group, title) in [
            (CoverGroup::Grass, "Césped y praderas"),
            (CoverGroup::Ground, "Suelos"),
        ] {
            ui.label(egui::RichText::new(title).small().color(p.muted));
            let side = ((ui.available_width() - 2.0 * 6.0) / 3.0).clamp(56.0, 92.0);
            let picks: Vec<usize> = (0..GROUND_COVERS.len())
                .filter(|i| GROUND_COVERS[*i].group == group)
                .collect();
            for row in picks.chunks(3) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for &i in row {
                        let tex = cover_swatch(ui.ctx(), i);
                        let (rect, resp) =
                            ui.allocate_exact_size(Vec2::splat(side), Sense::click());
                        let on = self.brush.cover == i;
                        let painter = ui.painter();
                        painter.image(
                            tex.id(),
                            rect.shrink(2.0),
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                        painter.rect_stroke(
                            rect.shrink(1.0),
                            CornerRadius::same(8),
                            if on {
                                Stroke::new(3.0, p.accent)
                            } else {
                                Stroke::new(1.5, Color32::WHITE)
                            },
                            egui::StrokeKind::Inside,
                        );
                        if resp.on_hover_text(GROUND_COVERS[i].name).clicked() {
                            self.brush.cover = i;
                        }
                    }
                });
                ui.add_space(4.0);
            }
        }
        ui.label(
            egui::RichText::new(GROUND_COVERS[self.brush.cover.min(GROUND_COVERS.len() - 1)].name)
                .strong(),
        );
    }

    /// The plant or rock scattered, and whether its variants go in turn.
    fn scatter_choice(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        let current = self.brush_species();
        // The natural things of the library, there to choose too.
        let model = self.ed.model();
        let mut items: Vec<(Option<archisim_core::ObjectTypeId>, String, String)> = model
            .object_types()
            .filter(|t| t.own.is_none() && brush::is_scattered(model, t.id))
            .map(|t| {
                let kind = if brush::is_plant(model, t.id) {
                    "Plantas"
                } else {
                    "Rocas y troncos"
                };
                (Some(t.id), t.name.clone(), kind.to_string())
            })
            .collect();
        for name in ROCKS {
            if !items.iter().any(|i| i.1 == *name) {
                items.push((None, (*name).to_string(), "Rocas y troncos".to_string()));
            }
        }
        items.sort_by(|a, b| (&a.2, &a.1).cmp(&(&b.2, &b.1)));
        let shown = current
            .and_then(|t| items.iter().find(|i| i.0 == Some(t)))
            .map_or("—".to_string(), |i| i.1.clone());
        let mut chosen: Option<(Option<archisim_core::ObjectTypeId>, String)> = None;
        ui.label(egui::RichText::new("Qué se esparce").small().color(p.muted));
        egui::ComboBox::from_id_salt("brush-species")
            .width(ui.available_width() - 8.0)
            .selected_text(shown)
            .show_ui(ui, |ui| {
                let mut group = "";
                for (id, name, kind) in &items {
                    if kind != group {
                        group = kind;
                        ui.label(egui::RichText::new(kind.as_str()).small().color(p.muted));
                    }
                    if ui
                        .selectable_label(current.is_some() && *id == current, name)
                        .clicked()
                    {
                        chosen = Some((*id, name.clone()));
                    }
                }
            });
        if let Some((id, name)) = chosen {
            let id = match id {
                Some(id) => Some(id),
                None => self
                    .ed
                    .transact("Cargar de la biblioteca", |m| {
                        archisim_library::add_object_entry(m, &name)
                    })
                    .ok(),
            };
            if id.is_some() {
                self.brush.species = id;
            }
        }
        ui.switch(
            &mut self.brush.vary,
            "Alternar sus variantes (otros tipos de la misma familia)",
        );
    }
}

/// The rocks of the library offered to scatter before the project has
/// any.
const ROCKS: &[&str] = &[
    "Roca angulosa 80",
    "Canto rodado 50",
    "Laja 120",
    "Roca con musgo 1 m",
    "Cantos rodados 1,2 m",
    "Tronco caído 2,4 m",
    "Tocón 60",
];

/// The label, step, largest value and unit of the strength of a mode.
fn strength_of(m: BrushMode) -> Option<(&'static str, f64, f64, &'static str)> {
    match m {
        BrushMode::Raise | BrushMode::Lower => Some(("Fuerza", 0.01, 2.0, " m")),
        BrushMode::Smooth | BrushMode::Flatten => Some(("Fuerza", 0.01, 1.0, "")),
        BrushMode::Scatter => Some(("Densidad", 0.05, 20.0, " /10 m²")),
        BrushMode::Paint | BrushMode::Erase => None,
    }
}

/// A square button with a glyph, lit when on.
fn glyph_button(ui: &mut Ui, g: Glyph, on: bool, p: Palette) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
    let fill = if on {
        p.accent
    } else if resp.hovered() {
        p.accent_soft
    } else {
        p.raised
    };
    ui.painter().rect_filled(rect, CornerRadius::same(8), fill);
    glyphs::paint(
        ui.painter(),
        rect.shrink(7.0),
        g,
        if on { Color32::WHITE } else { p.text },
    );
    resp
}

/// The colour of the brush's outline.
fn brush_colour(mode: BrushMode) -> Color32 {
    match mode {
        BrushMode::Paint => Color32::from_rgb(70, 140, 60),
        BrushMode::Erase => Color32::from_rgb(200, 70, 60),
        BrushMode::Scatter => Color32::from_rgb(40, 120, 70),
        _ => Color32::from_rgb(150, 100, 50),
    }
}

/// The picture of a cover in the palette: its texture in its colour, with
/// its grass and flowers; made once.
fn cover_swatch(ctx: &egui::Context, i: usize) -> egui::TextureHandle {
    #[derive(Clone, Default)]
    struct Swatches(std::collections::HashMap<usize, egui::TextureHandle>);
    let id = egui::Id::unique("brush-swatches");
    if let Some(t) = ctx.data(|d| {
        d.get_temp::<Swatches>(id)
            .and_then(|s| s.0.get(&i).cloned())
    }) {
        return t;
    }
    let img = swatch_image(i, 72);
    let tex = ctx.load_texture(format!("cover-{i}"), img, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Swatches>(id)
            .0
            .insert(i, tex.clone())
    });
    tex
}

/// The swatch of a cover, `n` pixels a side: 1,2 m of it seen from above.
pub(super) fn swatch_image(i: usize, n: usize) -> egui::ColorImage {
    let c = &GROUND_COVERS[i.min(GROUND_COVERS.len() - 1)];
    let tile = archisim_render::textures::tile(c.texture);
    let scale = c.scale.unwrap_or(c.texture.default_scale());
    let side = 1.2;
    let mut px = vec![Color32::BLACK; n * n];
    for y in 0..n {
        for x in 0..n {
            let w = [
                x as f64 / n as f64 * side / scale,
                (n - y) as f64 / n as f64 * side / scale,
            ];
            let col = tile.apply(c.rgb.map(|v| v as f64 / 255.0), w);
            px[y * n + x] = Color32::from_rgb(
                (col[0].clamp(0.0, 1.0) * 255.0) as u8,
                (col[1].clamp(0.0, 1.0) * 255.0) as u8,
                (col[2].clamp(0.0, 1.0) * 255.0) as u8,
            );
        }
    }
    // Its grass and flowers, as seen from above.
    let mut seed = 0x2545_F491_u64 ^ (i as u64 * 7919);
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let dot = |px: &mut Vec<Color32>, cx: f64, cy: f64, r: f64, col: Color32| {
        let (x0, x1) = (
            (cx - r).floor().max(0.0) as usize,
            ((cx + r).ceil() as usize).min(n - 1),
        );
        let (y0, y1) = (
            (cy - r).floor().max(0.0) as usize,
            ((cy + r).ceil() as usize).min(n - 1),
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d = ((x as f64 - cx).powi(2) + (y as f64 - cy).powi(2)).sqrt();
                if d <= r {
                    px[y * n + x] = col;
                }
            }
        }
    };
    if let Some((h, density)) = c.grass {
        let blades = (density * side * side * 3.0 * (h * 4.0).clamp(0.6, 2.5)) as usize;
        for _ in 0..blades.min(600) {
            let (x, y) = (rnd() * n as f64, rnd() * n as f64);
            let len = (2.0 + h * 14.0) * (0.6 + 0.6 * rnd());
            let k = 0.6 + 0.6 * rnd();
            let col = Color32::from_rgb(
                (c.rgb[0] as f64 * k).min(255.0) as u8,
                (c.rgb[1] as f64 * k).min(255.0) as u8,
                (c.rgb[2] as f64 * k).min(255.0) as u8,
            );
            let lean = (rnd() - 0.5) * 0.8;
            for t in 0..(len as usize).max(1) {
                let (px_, py_) = (x + lean * t as f64, y - t as f64);
                if px_ >= 0.0 && py_ >= 0.0 && (px_ as usize) < n && (py_ as usize) < n {
                    px[py_ as usize * n + px_ as usize] = col;
                }
            }
        }
    }
    if !c.flowers.is_empty() {
        let count = (c.bloom * side * side * 2.0) as usize;
        for _ in 0..count.min(200) {
            let col = c.flowers[(rnd() * c.flowers.len() as f64) as usize % c.flowers.len()];
            let (x, y) = (rnd() * n as f64, rnd() * n as f64);
            dot(
                &mut px,
                x,
                y,
                1.6 + rnd(),
                Color32::from_rgb(col[0], col[1], col[2]),
            );
        }
    }
    egui::ColorImage::new([n, n], px)
}
