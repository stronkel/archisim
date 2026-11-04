//! The buildings of the world put by hand, as in CityRT (tanda 16): a
//! gallery of cards with each building in 3D (the dwellings of the place,
//! its facilities, its shops, its squares and parks, what finishes a town
//! and its popular architecture), its usual measurements, and a click to
//! put it on the ground: with its front on the edge of the nearest street,
//! or turned with Space where there is none. With two clicks, the front is
//! drawn from one end to the other. What is put is a building of the
//! surroundings, changed afterwards like any other.

use std::collections::HashMap;

use archisim_core::{LevelId, Model, NewWorldBuilding, Point2, Vec2, WorldPick, WorldTypology};
use eframe::egui::{self, Color32, Id, Rect, Ui};

use super::props::{self, choice, group, row};
use super::thumbs::Thumb;
use super::widgets::SwitchExt as _;
use super::*;

/// How far from a street a building is put with its front on it.
const STREET_REACH: f64 = 25.0;

/// What the tool «Edificio del mundo» puts.
#[derive(Debug, Clone)]
pub struct WorldSettings {
    pub pick: WorldPick,
    /// Its front, its depth and its floors.
    pub width: f64,
    pub depth: f64,
    pub floors: u32,
    /// Where its front looks where there is no street near (radians from
    /// −y, the bottom of the plan).
    pub turn: f64,
    /// Its front on the nearest street.
    pub to_street: bool,
    /// The front drawn with two clicks, from one end to the other.
    pub two_clicks: bool,
    /// The first end of the front, with two clicks.
    pub first: Option<Point2>,
    /// The place was taken from the project.
    pub placed: bool,
}

impl Default for WorldSettings {
    fn default() -> Self {
        let pick = WorldPick {
            place: 0,
            typology: WorldTypology::Street,
            programme: 0,
        };
        let (width, depth, floors, _) = pick.usual();
        WorldSettings {
            pick,
            width,
            depth,
            floors,
            turn: 0.0,
            to_street: true,
            two_clicks: false,
            first: None,
            placed: false,
        }
    }
}

impl WorldSettings {
    /// Takes `pick` with its usual measurements.
    pub fn take(&mut self, pick: WorldPick) {
        self.pick = pick;
        let (w, d, f, _) = pick.usual();
        self.width = w;
        self.depth = d;
        self.floors = f;
        self.first = None;
    }

    /// Where the front looks without a street.
    pub fn facing(&self) -> Vec2 {
        let (s, c) = self.turn.sin_cos();
        Vec2::new(s, -c)
    }
}

/// The cards of the gallery: titles and the buildings under each.
pub fn gallery(place: u8) -> Vec<(&'static str, Vec<WorldPick>)> {
    let t = |typology| WorldPick {
        place,
        typology,
        programme: 0,
    };
    let f = |programme| WorldPick {
        place,
        typology: WorldTypology::Facility,
        programme,
    };
    let fs = |list: &[u8]| list.iter().map(|p| f(*p)).collect::<Vec<_>>();
    vec![
        (
            "Viviendas",
            vec![
                t(WorldTypology::Street),
                t(WorldTypology::Block),
                t(WorldTypology::House),
                t(WorldTypology::Tower),
                t(WorldTypology::Courtyard),
            ],
        ),
        ("Arquitectura popular", fs(&[33, 34, 35, 36, 37])),
        ("Equipamientos", fs(&[0, 1, 2, 3, 4, 29, 32])),
        ("Deporte", fs(&[5, 6, 7])),
        (
            "Comercio y hostelería",
            fs(&[9, 10, 11, 12, 13, 14, 15, 16, 30]),
        ),
        ("Plazas, parques y paseos", fs(&[17, 19, 20, 21, 22])),
        (
            "Transporte, puerto e industria",
            fs(&[27, 28, 25, 26, 31, 8]),
        ),
        ("Monumentos, culto y cementerio", fs(&[18, 24, 23])),
    ]
}

/// A small model with one building of the world alone, its front to +y
/// (as the pictures look at pieces).
pub fn world_model(pick: WorldPick) -> Option<(Model, LevelId)> {
    let mut m = Model::new("Edificio del mundo");
    let level = m.add_level("Planta baja", 0.0);
    let (w, d, _, _) = pick.usual();
    m.add_world_building(NewWorldBuilding {
        level,
        front: Point2::new(0.0, d / 2.0),
        facing: Vec2::new(0.0, 1.0),
        width: w,
        depth: d,
        floors: None,
        pick,
    })
    .ok()?;
    Some((m, level))
}

impl ArchiSimApp {
    /// The 3D pictures of the gallery's cards that are ready.
    fn world_thumbs(&mut self, ctx: &egui::Context, picks: &[WorldPick]) -> HashMap<String, Thumb> {
        self.thumbs3d.begin();
        let gpu = self.gpu.as_ref();
        let mut out = HashMap::new();
        for pick in picks {
            let key = format!(
                "world:{}:{}:{}",
                pick.place,
                pick.typology.code(),
                pick.programme
            );
            let p = *pick;
            if let Some(t) = self
                .thumbs3d
                .get_seen(ctx, gpu, key, [55.0, 26.0], move || world_model(p))
            {
                out.insert(pick.name(), t);
            }
        }
        out
    }

    /// The tool's panel: the place, the gallery, the measurements and how
    /// a click puts it.
    pub(super) fn world_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Edificio del mundo · lo que vas a poner");
        let mut s = self.world_settings.clone();
        if !s.placed {
            // The project's place, the first time.
            let mut pick = s.pick;
            pick.place = self.ed.model().world_place();
            s.take(pick);
            s.placed = true;
        }
        let list = gallery(s.pick.place);
        let picks: Vec<WorldPick> = list.iter().flat_map(|(_, l)| l.iter().copied()).collect();
        let thumbs = self.world_thumbs(ui.ctx(), &picks);
        if self.thumbs3d.pending {
            ui.ctx().request_repaint();
        }
        let groups: Vec<(String, Vec<String>)> = list
            .iter()
            .map(|(g, l)| (g.to_string(), l.iter().map(|p| p.name()).collect()))
            .collect();
        group(ui, "tool-world", "Edificio del mundo", |ui| {
            let places = archisim_core::world_places();
            let options: Vec<(u8, &str)> = places
                .iter()
                .enumerate()
                .map(|(i, n)| (i as u8, *n))
                .collect();
            row(
                ui,
                "Lugar",
                "La piel: cómo se construye en ese sitio",
                |ui, w| {
                    let c = choice(ui, "tool-world-place", w, s.pick.place, &options);
                    if c != s.pick.place {
                        let mut p = s.pick;
                        p.place = c;
                        s.take(p);
                    }
                },
            );
            let picked = super::cards::grouped_cards(
                ui,
                ui.available_width(),
                &groups,
                &s.pick.name(),
                &mut |ui, r, name| paint_card(ui, r, name, &thumbs),
            );
            if let Some(name) = picked {
                if let Some(p) = picks.iter().find(|p| p.name() == name) {
                    s.take(*p);
                }
            }
            row(
                ui,
                "Fachada",
                "El ancho de la fachada principal",
                |ui, w| {
                    if let Some(v) = length_field(ui, Id::unique("tool-world-w"), s.width, w - 8.0)
                    {
                        s.width = v.clamp(2.0, 300.0);
                    }
                },
            );
            row(ui, "Fondo", "Desde la fachada hacia dentro", |ui, w| {
                if let Some(v) = length_field(ui, Id::unique("tool-world-d"), s.depth, w - 8.0) {
                    s.depth = v.clamp(2.0, 300.0);
                }
            });
            row(ui, "Plantas", "Sobre rasante", |ui, _| {
                let mut f = s.floors;
                ui.add(egui::DragValue::new(&mut f).range(1..=60));
                s.floors = f;
            });
            if ui
                .button("Medidas habituales")
                .on_hover_text("Las del edificio en su lugar")
                .clicked()
            {
                s.take(s.pick);
            }
            ui.switch(&mut s.to_street, "La fachada a la calle más cercana");
            if ui
                .switch(&mut s.two_clicks, "La fachada con dos clics")
                .changed()
            {
                s.first = None;
            }
            props::note(
                ui,
                "Clic: pone el edificio con el centro de su fachada en el cursor (en el borde de la calle si hay una cerca). Espacio gira la fachada donde no hay calle. Después se cambia como cualquier edificio del entorno: sus vértices, su altura, su lugar y su programa.",
            );
        });
        self.world_settings = s;
    }

    /// Turns the front a quarter (Space).
    pub(super) fn world_turn(&mut self) {
        self.world_settings.turn += std::f64::consts::FRAC_PI_2;
    }

    /// What a click at `p` would put (none while the first end of a front
    /// of two clicks is awaited).
    pub(super) fn world_would(&self, p: Point2) -> Option<NewWorldBuilding> {
        let s = &self.world_settings;
        let level = self.ed.level;
        let model = self.ed.model();
        let (front, facing, width) = if s.two_clicks {
            let a = s.first?;
            let along = (p - a).normalized()?;
            let mid = a + (p - a) * 0.5;
            // The front looks to the street if there is one, else to the
            // right of the line drawn.
            let right = -along.perp_left();
            let facing = model
                .world_front_at(level, mid, STREET_REACH)
                .map(|(_, n)| if n.dot(right) >= 0.0 { right } else { -right })
                .unwrap_or(right);
            (mid, facing, a.distance(p))
        } else if s.to_street {
            let (f, n) = model
                .world_front_at(level, p, STREET_REACH)
                .unwrap_or((p, s.facing()));
            (f, n, s.width)
        } else {
            (p, s.facing(), s.width)
        };
        if width < 1.0 {
            return None;
        }
        Some(NewWorldBuilding {
            level,
            front,
            facing,
            width,
            depth: s.depth,
            floors: Some(s.floors),
            pick: s.pick,
        })
    }

    /// A click of the tool.
    pub(super) fn world_click(&mut self, p: Point2) {
        if self.world_settings.two_clicks && self.world_settings.first.is_none() {
            self.world_settings.first = Some(p);
            self.say("Ahora el otro extremo de la fachada.");
            return;
        }
        let Some(spec) = self.world_would(p) else {
            return;
        };
        self.world_settings.first = None;
        let name = spec.pick.name();
        let r = self
            .ed
            .transact("Edificio del mundo", |m| m.add_world_building(spec));
        if let Ok(id) = &r {
            self.ed.selection = [Element::Mass(*id)].into_iter().collect();
        }
        self.report(r.map(|_| ()), &format!("{name} puesto."));
    }

    /// Esc: forgets the first end of the front, or leaves the tool.
    pub(super) fn world_escape(&mut self) {
        if self.world_settings.first.take().is_none() {
            self.set_tool(Tool::Arrow);
        }
    }
}

/// A card: the building's picture if it is ready, else its name.
fn paint_card(ui: &Ui, r: Rect, name: &str, thumbs: &HashMap<String, Thumb>) {
    if let Some(t) = thumbs.get(name) {
        let k = (r.width() / t.size.x).min(r.height() / t.size.y);
        let size = t.size * k;
        let rect = Rect::from_center_size(r.center(), size);
        ui.painter().image(
            t.tex,
            rect,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        ui.painter().text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            name,
            egui::FontId::proportional(10.0),
            Color32::DARK_GRAY,
        );
    }
}
