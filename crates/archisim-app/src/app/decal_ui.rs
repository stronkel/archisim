//! Decals in the app (U8): the tool «Pegatina» (choose a PNG, then a
//! click on a face in 3D — any face — or, in plan, on the face of a wall
//! near the click or on the floor) and its properties (size, turn,
//! height, picture).

use archisim_core::{decal_image, DecalId, DecalImage, NewDecal, Point2};
use eframe::egui::{self, Id, Ui};

use super::props::{self, group, row, text_row};
use super::*;

/// What the tool sticks next.
#[derive(Debug, Clone, PartialEq)]
pub struct DecalSettings {
    pub image: Option<DecalImage>,
    /// Width (m); the height keeps the picture's proportion.
    pub width: f64,
    /// On a wall picked in plan, the height of its centre over the level.
    pub height: f64,
    /// Turn on the face (radians).
    pub rotation: f64,
}

impl Default for DecalSettings {
    fn default() -> Self {
        DecalSettings {
            image: None,
            width: 1.0,
            height: 1.6,
            rotation: 0.0,
        }
    }
}

/// How near a wall's face a plan click sticks to it, in metres at most.
const WALL_REACH: f64 = 0.6;

impl ArchiSimApp {
    /// Asks for the picture of the decals (a PNG). False if none was
    /// chosen.
    pub(super) fn choose_decal_image(&mut self) -> bool {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Imagen de la pegatina")
            .add_filter("Imagen PNG", &["png", "PNG"])
            .pick_file()
        else {
            return false;
        };
        match std::fs::read(&path) {
            Ok(bytes) => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                match decal_image(&name, bytes) {
                    Some(img) => {
                        self.decal_settings.image = Some(img);
                        true
                    }
                    None => {
                        self.complain("Ese fichero no es una imagen PNG.");
                        false
                    }
                }
            }
            Err(e) => {
                self.complain(format!("No se pudo leer la imagen: {e}"));
                false
            }
        }
    }

    /// Where a click puts a decal: in 3D, on the face under the cursor; in
    /// plan, on the nearest wall face or else on the floor. Its centre,
    /// outward normal and the element it is on.
    fn decal_spot(&self, p: Point2) -> ([f64; 3], [f64; 3], Option<u64>) {
        let level = self.ed.level;
        let z0 = self.ed.model().level(level).map_or(0.0, |l| l.elevation);
        if let (ViewMode::ThreeD, Some(c)) = (self.mode, self.cursor3d) {
            if let Some(n) = c.normal {
                let host = match c.owner {
                    Some(crate::view3d::scene::Owner::Model(e)) => Some(crate::team::raw_id(e)),
                    _ => None,
                };
                return (
                    [c.world.x as f64, c.world.y as f64, c.world.z as f64 - z0],
                    [n.x as f64, n.y as f64, n.z as f64],
                    host,
                );
            }
        }
        // In plan: the nearest face of a wall.
        let reach = self.view.to_metres(14.0).clamp(0.05, WALL_REACH);
        let model = self.ed.model();
        let mut best: Option<(f64, Point2, archisim_core::Vec2, u64)> = None;
        for w in model.walls_on_level(level) {
            let Some(g) = crate::geo::Geo::wall_geometry(&self.ed, w.id) else {
                continue;
            };
            let o = &g.outline;
            let ccw = archisim_core::signed_area(o) > 0.0;
            for i in 0..o.len() {
                let (a, b) = (o[i], o[(i + 1) % o.len()]);
                let Some(u) = (b - a).normalized() else {
                    continue;
                };
                let t = (p - a).dot(u).clamp(0.0, a.distance(b));
                let q = a + u * t;
                let dist = q.distance(p);
                // Outwards: right of the edge on a counter-clockwise
                // outline.
                let out = if ccw { -u.perp_left() } else { u.perp_left() };
                if dist <= reach && best.is_none_or(|b| dist < b.0) {
                    best = Some((dist, q, out, w.id.0));
                }
            }
        }
        if let Some((_, q, out, host)) = best {
            return (
                [q.x, q.y, self.decal_settings.height],
                [out.x, out.y, 0.0],
                Some(host),
            );
        }
        // On the floor, or the street or ground there.
        let z = model.surface_at(level, p);
        ([p.x, p.y, z], [0.0, 0.0, 1.0], None)
    }

    pub(super) fn decal_click(&mut self, p: Point2) {
        let Some(image) = self.decal_settings.image.clone() else {
            if !self.choose_decal_image() {
                self.say("Elige primero la imagen de la pegatina (PNG).");
            }
            return;
        };
        let (centre, normal, host) = self.decal_spot(p);
        let s = &self.decal_settings;
        let level = self.ed.level;
        let (width, rotation) = (s.width, s.rotation);
        let r = self.ed.transact("Pegatina", |m| {
            let name = m.next_decal_name();
            m.add_decal(NewDecal {
                name,
                level,
                centre,
                normal,
                rotation,
                width,
                image,
                host,
            })
        });
        if let Ok(id) = &r {
            self.ed.selection = [Element::Decal(*id)].into_iter().collect();
        }
        self.report(r.map(|_| ()), "Pegatina puesta.");
    }

    pub(super) fn decal_hint(&self) -> &'static str {
        if self.decal_settings.image.is_none() {
            "Pegatina: elige su imagen PNG en Propiedades"
        } else if self.mode == ViewMode::ThreeD {
            "Pegatina: clic en la cara donde va"
        } else {
            "Pegatina: clic junto a la cara de un muro, o en el suelo"
        }
    }

    pub(super) fn decal_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Pegatina · lo que vas a poner");
        let mut s = self.decal_settings.clone();
        let mut choose = false;
        group(ui, "tool-decal", "Pegatina", |ui| {
            text_row(
                ui,
                "Imagen",
                "",
                s.image.as_ref().map_or("—", |i| i.source.as_str()),
            );
            if ui
                .button("Elegir imagen PNG…")
                .on_hover_text("Un grafiti, un cartel, un rótulo, una mancha… con su transparencia")
                .clicked()
            {
                choose = true;
            }
            row(
                ui,
                "Ancho (m)",
                "El alto sigue la proporción de la imagen",
                |ui, w| {
                    if let Some(v) = length_field(ui, Id::unique("tool-decal-w"), s.width, w - 8.0)
                    {
                        s.width = v.clamp(archisim_core::MIN_DECAL, archisim_core::MAX_DECAL);
                    }
                },
            );
            row(
                ui,
                "Altura del centro (m)",
                "En un muro elegido en planta",
                |ui, w| {
                    if let Some(v) = length_field(ui, Id::unique("tool-decal-h"), s.height, w - 8.0)
                    {
                        s.height = v;
                    }
                },
            );
            row(ui, "Giro (°)", "Sobre la cara", |ui, w| {
                let mut deg = s.rotation.to_degrees().round() as i32;
                if ui
                    .add_sized(
                        [w - 8.0, 20.0],
                        egui::DragValue::new(&mut deg).range(-180..=180),
                    )
                    .changed()
                {
                    s.rotation = (deg as f64).to_radians();
                }
            });
            props::note(
                ui,
                "Sale en la vista 3D, en la imagen realista y en el modelo 3D (glTF). En 3D va sobre cualquier cara; en planta, sobre la cara de un muro o en el suelo.",
            );
        });
        self.decal_settings = s;
        if choose {
            self.choose_decal_image();
        }
    }

    pub(super) fn decal_properties(&mut self, ui: &mut Ui, id: DecalId) {
        let Some(d) = self.ed.model().decal(id).cloned() else {
            return;
        };
        props::title(ui, &format!("{} · 1 seleccionada", d.name));
        let mut edit: Option<archisim_core::Decal> = None;
        let mut replace = false;
        group(ui, "decal-data", "Pegatina", |ui| {
            let mut x = d.clone();
            let mut changed = false;
            text_row(ui, "Imagen", "", &d.image.source);
            if ui.button("Cambiar la imagen…").clicked() {
                replace = true;
            }
            row(ui, "Ancho (m)", "", |ui, w| {
                if let Some(v) = length_field(ui, Id::unique(("decal-w", id)), x.width, w - 8.0) {
                    x.width = v;
                    changed = true;
                }
            });
            text_row(
                ui,
                "Alto (m)",
                "Sigue la proporción de la imagen",
                &format!("{:.2}", d.height()).replace('.', ","),
            );
            row(ui, "Altura del centro (m)", "Sobre la planta", |ui, w| {
                if let Some(v) = length_field(ui, Id::unique(("decal-z", id)), x.centre[2], w - 8.0)
                {
                    x.centre[2] = v;
                    changed = true;
                }
            });
            row(ui, "Giro (°)", "Sobre la cara", |ui, w| {
                let mut deg = x.rotation.to_degrees().round() as i32;
                if ui
                    .add_sized(
                        [w - 8.0, 20.0],
                        egui::DragValue::new(&mut deg).range(-180..=180),
                    )
                    .changed()
                {
                    x.rotation = (deg as f64).to_radians();
                    changed = true;
                }
            });
            let on = if d.lying() {
                if d.normal[2] > 0.0 {
                    "Suelo"
                } else {
                    "Techo"
                }
            } else {
                "Pared"
            };
            text_row(ui, "Sobre", "", on);
            if changed {
                edit = Some(x);
            }
        });
        if replace {
            let before = self.decal_settings.image.clone();
            if self.choose_decal_image() {
                if let Some(img) = self.decal_settings.image.clone() {
                    let r = self.ed.transact("Imagen de la pegatina", |m| {
                        m.edit_decal(id, |x| x.image = img)
                    });
                    self.report(r, "Imagen cambiada.");
                }
            } else {
                self.decal_settings.image = before;
            }
        } else if let Some(x) = edit {
            let r = self
                .ed
                .transact("Pegatina", |m| m.edit_decal(id, |d| *d = x));
            self.report(r, "Pegatina cambiada.");
        }
    }
}
