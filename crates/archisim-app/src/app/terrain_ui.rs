//! The terrain tool ("Terreno"): survey points clicked one by one with
//! their height; the properties of a terrain (its points, contour lines,
//! depth); and the project's true north.

use super::widgets::SwitchExt as _;
use archisim_core::{SurveyPoint, TerrainId};
use eframe::egui::{self, Id, Ui};

use super::icons::Icon;
use super::props::{self, choice, group, row, text_row};
use super::widgets::segmented;
use super::*;

/// Colour of the survey points and contour lines drawn by hand (earth).
const TERRAIN_LINE: Color32 = Color32::from_rgb(140, 98, 52);

/// How close (screen points) a click must be to pick a survey point.
const POINT_REACH: f32 = 5.0;

/// Distance from `p` to the segment `a b`.
fn segment_distance(p: Point2, a: Point2, b: Point2) -> f64 {
    let ab = b - a;
    let len2 = ab.dot(ab);
    if len2 < 1e-18 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// A height as on a topographic plan: "+1,25".
fn height(z: f64) -> String {
    let s = format!("{:.2}", z.abs()).replace('.', ",");
    if z < -0.0005 {
        format!("-{s}")
    } else {
        format!("+{s}")
    }
}

impl ArchiSimApp {
    /// The terrain the terrain tool adds to: the selected one on this
    /// level, or the first one there.
    pub(super) fn level_terrain(&self) -> Option<TerrainId> {
        let level = self.ed.level;
        let selected = self.ed.selection.iter().find_map(|e| match e {
            Element::Terrain(id) => Some(*id),
            _ => None,
        });
        selected
            .filter(|id| {
                self.ed
                    .model()
                    .terrain(*id)
                    .is_some_and(|t| t.level == level)
            })
            .or_else(|| {
                self.ed
                    .model()
                    .terrains_on_level(level)
                    .map(|t| t.id)
                    .next()
            })
    }

    /// The terrain selected on its own, whose points can be picked.
    fn selected_terrain(&self) -> Option<TerrainId> {
        match self.ed.selection.iter().collect::<Vec<_>>().as_slice() {
            [Element::Terrain(id)] => Some(*id),
            _ => None,
        }
    }

    /// The point of a terrain nearest to `p`, within `reach`.
    fn terrain_point_at(&self, id: TerrainId, p: Point2, reach: f64) -> Option<usize> {
        let t = self.ed.model().terrain(id)?;
        t.points
            .iter()
            .enumerate()
            .map(|(i, q)| (i, q.at.distance(p)))
            .filter(|(_, d)| *d <= reach)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    /// The contour line drawn by hand passing within `reach` of `p`.
    fn terrain_line_at(&self, id: TerrainId, p: Point2, reach: f64) -> Option<usize> {
        let t = self.ed.model().terrain(id)?;
        t.lines.iter().position(|l| {
            l.windows(2)
                .any(|w| segment_distance(p, t.points[w[0]].at, t.points[w[1]].at) <= reach)
        })
    }

    pub(super) fn terrain_click(&mut self, p: Point2) {
        if self.terrain_settings.line {
            if self.terrain_line.last().is_some_and(|q| q.approx_eq(p)) {
                return;
            }
            self.terrain_line.push(p);
            self.say(format!(
                "Línea de nivel a {}: {} vértices · Intro o doble clic: terminarla.",
                height(self.terrain_settings.z),
                self.terrain_line.len()
            ));
            return;
        }
        let z = self.terrain_settings.z;
        // With a terrain on the level, a click goes straight into it: on
        // one of its points, it takes the height; elsewhere, a new point.
        if let Some(id) = self.level_terrain() {
            let reach = self.view.to_metres(REACH);
            let r = match self.terrain_point_at(id, p, reach) {
                Some(i) => self
                    .ed
                    .edit_terrain("Cota de punto", id, |t| t.points[i].z = z),
                None => self.ed.edit_terrain("Añadir punto al terreno", id, |t| {
                    t.points.push(SurveyPoint { at: p, z })
                }),
            };
            self.report(r, &format!("Punto a {}.", height(z)));
            return;
        }
        if self.terrain_points.iter().any(|q| q.at.approx_eq(p)) {
            return;
        }
        self.terrain_points.push(SurveyPoint { at: p, z });
        let n = self.terrain_points.len();
        self.say(format!(
            "{n} puntos. Cambia la cota en Propiedades antes de cada clic · Intro o doble clic: crear el terreno."
        ));
    }

    /// Esc or Backspace with the terrain tool: the last vertex of the line,
    /// or the last point marked. False if there was nothing to take.
    pub(super) fn terrain_back(&mut self) -> bool {
        if self.terrain_line.pop().is_some() {
            return true;
        }
        if self.terrain_points.pop().is_none() {
            return false;
        }
        let n = self.terrain_points.len();
        self.terrain_lines
            .retain(|l| l.iter().all(|&i| i < n) && l.len() >= 2);
        true
    }

    pub(super) fn finish_terrain(&mut self) {
        if !self.terrain_line.is_empty() {
            self.finish_terrain_line();
            return;
        }
        if self.terrain_points.is_empty() && self.level_terrain().is_some() {
            return;
        }
        let points = std::mem::take(&mut self.terrain_points);
        let lines = std::mem::take(&mut self.terrain_lines);
        if points.len() < 3 {
            self.complain("El terreno necesita al menos tres puntos.");
            self.terrain_points = points;
            self.terrain_lines = lines;
            return;
        }
        match self.ed.add_terrain(&self.terrain_settings, points, lines) {
            Ok(id) => {
                self.ed.selection = [Element::Terrain(id)].into_iter().collect();
                self.say("Terreno creado: sus curvas de nivel en planta; cambia las cotas en Propiedades.");
            }
            Err(e) => self.complain(format!("No se puede crear el terreno: {e}.")),
        }
    }

    /// Ends the contour line being drawn: into the level's terrain, or
    /// into the points being marked (the terrain is made as soon as they
    /// make a surface: a second line at another height).
    fn finish_terrain_line(&mut self) {
        let path = std::mem::take(&mut self.terrain_line);
        if path.len() < 2 {
            self.complain("La línea de nivel necesita dos vértices como mínimo.");
            return;
        }
        let z = self.terrain_settings.z;
        if let Some(id) = self.level_terrain() {
            let r = self.ed.edit_terrain("Línea de nivel", id, |t| {
                t.add_line(&path, z);
            });
            self.report(
                r,
                &format!("Línea de nivel a {} añadida al terreno.", height(z)),
            );
            return;
        }
        let line = archisim_core::push_line(&mut self.terrain_points, &path, z);
        self.terrain_lines.push(line);
        let pts: Vec<Point2> = self.terrain_points.iter().map(|p| p.at).collect();
        if archisim_core::delaunay(&pts).is_empty() {
            self.say(format!(
                "Línea de nivel a {}. Dibuja otra a otra cota (o marca puntos) para formar el terreno.",
                height(z)
            ));
            return;
        }
        self.finish_terrain();
    }

    /// A click with the arrow on the selected terrain: on one of its
    /// points, it picks it (Shift adds or takes it); on a contour line
    /// drawn by hand, all its points. False if it hit neither (the pick
    /// is cleared and the click selects as usual).
    pub(super) fn pick_terrain_points(&mut self, rect: Rect, pos: Pos2, shift: bool) -> bool {
        let Some(id) = self.selected_terrain() else {
            self.terrain_pick.clear();
            return false;
        };
        let p = self.view.to_model(rect, pos);
        let reach = self.view.to_metres(REACH);
        // A point needs a close click: the points of a line are near each
        // other, and a click on the line between them takes it whole.
        let hit: Vec<usize> = match self.terrain_point_at(id, p, self.view.to_metres(POINT_REACH)) {
            Some(i) => vec![i],
            None => match self.terrain_line_at(id, p, reach) {
                Some(l) => self.ed.model().terrain(id).unwrap().lines[l].clone(),
                None => {
                    self.terrain_pick.clear();
                    return false;
                }
            },
        };
        if shift {
            for i in hit {
                if let Some(k) = self.terrain_pick.iter().position(|&j| j == i) {
                    self.terrain_pick.remove(k);
                } else {
                    self.terrain_pick.push(i);
                }
            }
        } else {
            self.terrain_pick = hit;
        }
        let n = self.terrain_pick.len();
        self.say(format!(
            "{n} punto(s) elegido(s): escribe su cota en Propiedades · Supr: quitarlos · Mayús: añadir más."
        ));
        true
    }

    /// The picked points of the selected terrain, if any (stale picks are
    /// dropped).
    fn terrain_picked(&self) -> Option<(TerrainId, Vec<usize>)> {
        let id = self.selected_terrain()?;
        let n = self.ed.model().terrain(id)?.points.len();
        let pick: Vec<usize> = self
            .terrain_pick
            .iter()
            .copied()
            .filter(|&i| i < n)
            .collect();
        (!pick.is_empty()).then_some((id, pick))
    }

    /// Delete with points of the terrain picked: those points go, not the
    /// terrain.
    pub(super) fn delete_terrain_points(&mut self) -> bool {
        let Some((id, pick)) = self.terrain_picked() else {
            return false;
        };
        let r = self
            .ed
            .edit_terrain("Quitar puntos del terreno", id, |t| t.remove_points(&pick));
        match r {
            Ok(()) => {
                self.terrain_pick.clear();
                self.say(format!("{} punto(s) quitado(s) del terreno.", pick.len()));
            }
            Err(_) => self.complain(
                "No se pueden quitar: el terreno necesita al menos tres puntos que no estén en línea.",
            ),
        }
        true
    }

    /// Survey points with their heights, and the contour lines drawn by
    /// hand: of the terrain being drawn and, with the terrain tool, the
    /// level's terrain; of the selected terrain, with the points picked.
    pub(super) fn draw_terrain_preview(&self, painter: &egui::Painter, rect: Rect) {
        let font = FontId::proportional(11.0);
        let labels = self.view.zoom >= 25.0;
        let shown = match self.tool {
            Tool::Terrain => self.level_terrain(),
            Tool::Arrow => self.selected_terrain(),
            _ => None,
        };
        if let Some(t) = shown.and_then(|id| self.ed.model().terrain(id)) {
            let picked = |i: usize| self.tool == Tool::Arrow && self.terrain_pick.contains(&i);
            for l in &t.lines {
                let pts: Vec<Point2> = l.iter().map(|&i| t.points[i].at).collect();
                painter.add(Shape::line(
                    self.polygon(rect, &pts),
                    Stroke::new(2.0, TERRAIN_LINE),
                ));
            }
            let few = t.points.len() <= 400;
            // Heights only where they do not overlap one already written.
            let mut written: Vec<Pos2> = Vec::new();
            for (i, p) in t.points.iter().enumerate() {
                let s = self.view.to_screen(rect, p.at);
                if !rect.expand(20.0).contains(s) {
                    continue;
                }
                if picked(i) {
                    painter.circle_filled(s, 5.0, ACCENT);
                } else {
                    painter.circle(s, 3.0, Color32::WHITE, Stroke::new(1.2, TERRAIN_LINE));
                }
                let free = !written
                    .iter()
                    .any(|w| (w.x - s.x).abs() < 40.0 && (w.y - s.y).abs() < 14.0);
                if free && (picked(i) || (labels && few)) {
                    written.push(s);
                    painter.text(
                        s + Vec2::new(5.0, -5.0),
                        Align2::LEFT_BOTTOM,
                        height(p.z),
                        font.clone(),
                        if picked(i) { ACCENT } else { TERRAIN_LINE },
                    );
                }
            }
        }
        if self.tool != Tool::Terrain {
            return;
        }
        let ink = Stroke::new(1.0, ACCENT);
        let pts: Vec<Point2> = self.terrain_points.iter().map(|p| p.at).collect();
        for [a, b, c] in archisim_core::delaunay(&pts) {
            let tri = [pts[a], pts[b], pts[c]];
            painter.add(Shape::closed_line(self.polygon(rect, &tri), ink));
        }
        for l in &self.terrain_lines {
            let line: Vec<Point2> = l.iter().map(|&i| pts[i]).collect();
            painter.add(Shape::line(
                self.polygon(rect, &line),
                Stroke::new(2.0, ACCENT),
            ));
        }
        for p in &self.terrain_points {
            let s = self.view.to_screen(rect, p.at);
            painter.circle_filled(s, 3.0, ACCENT);
            painter.text(
                s + Vec2::new(5.0, -5.0),
                Align2::LEFT_BOTTOM,
                height(p.z),
                font.clone(),
                ACCENT,
            );
        }
        let mut line = self.terrain_line.clone();
        if let (false, Some(c)) = (line.is_empty(), self.cursor) {
            line.push(c.point);
        }
        if line.len() >= 2 {
            painter.add(Shape::line(
                self.polygon(rect, &line),
                Stroke::new(2.0, ACCENT),
            ));
        }
        if let Some(c) = self.cursor {
            let what = if self.terrain_settings.line {
                "Línea de nivel a"
            } else {
                "Punto a"
            };
            self.draw_label(
                painter,
                rect,
                c.point,
                format!("{what} {}", height(self.terrain_settings.z)),
            );
        }
    }

    pub(super) fn terrain_tool_properties(&mut self, ui: &mut Ui) {
        props::title(ui, "Herramienta Terreno · lo que vas a crear");
        group(ui, "tool-terrain", "Terreno", |ui| {
            ui.add_space(2.0);
            let before = self.terrain_settings.line;
            segmented(
                ui,
                "terrain-mode",
                &mut self.terrain_settings.line,
                &[
                    (false, "Puntos", "Un punto por clic, a la cota escrita"),
                    (
                        true,
                        "Línea de nivel",
                        "Una línea de varios clics: todos sus puntos a la cota escrita",
                    ),
                ],
            );
            if before != self.terrain_settings.line {
                self.terrain_line.clear();
            }
            ui.add_space(4.0);
            row(
                ui,
                if self.terrain_settings.line {
                    "Cota de la línea (mm)"
                } else {
                    "Cota del punto (mm)"
                },
                "Respecto al ±0,00 del proyecto",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique("tool-terrain-z"),
                        self.terrain_settings.z,
                        w - 8.0,
                    ) {
                        self.terrain_settings.z = v;
                    }
                },
            );
            row(
                ui,
                "Equidistancia (mm)",
                "Entre curvas de nivel",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique("tool-terrain-contour"),
                        self.terrain_settings.contour,
                        w - 8.0,
                    ) {
                        self.terrain_settings.contour = v.max(0.0);
                    }
                },
            );
            row(
                ui,
                "Profundidad (mm)",
                "Del terreno bajo su punto más bajo, para las secciones",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique("tool-terrain-depth"),
                        self.terrain_settings.depth,
                        w - 8.0,
                    ) {
                        self.terrain_settings.depth = v;
                    }
                },
            );
            match self
                .level_terrain()
                .and_then(|id| self.ed.model().terrain(id))
            {
                Some(t) => text_row(
                    ui,
                    "Se añade a",
                    "El terreno de esta planta",
                    &format!("Terreno de {} puntos", t.points.len()),
                ),
                None => text_row(
                    ui,
                    "Puntos",
                    "Marcados hasta ahora",
                    &self.terrain_points.len().to_string(),
                ),
            }
        });
        ui.add_space(6.0);
        props::note(
            ui,
            "Puntos: escribe la cota y haz clic en su sitio; sobre un punto que ya existe, le pone esa cota. Línea de nivel: escribe la cota y dibuja la curva con varios clics; Intro o doble clic la termina (sus puntos, cada metro como mucho). Con un terreno en la planta todo se añade a él; si no, se crea en cuanto los puntos forman superficie (Intro con tres puntos, o dos líneas a distinta cota). Esc quita el último vértice.",
        );
    }

    pub(super) fn terrain_properties(&mut self, ui: &mut Ui, id: TerrainId) {
        let Some(t) = self.ed.model().terrain(id).cloned() else {
            return;
        };
        props::title(ui, "Terreno · 1 seleccionado");
        super::properties::view_card(
            ui,
            Icon::Terrain,
            &format!("Terreno de {} puntos", t.points.len()),
            "IfcGeographicElement",
        );
        let (lo, hi) = t.range();
        group(ui, "terrain-data", "Terreno", |ui| {
            row(
                ui,
                "Equidistancia (mm)",
                "Entre curvas de nivel",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique(("terrain-contour", id.0)),
                        t.contour,
                        w - 8.0,
                    ) {
                        let r = self.ed.edit_terrain("Equidistancia", id, |t| t.contour = v);
                        self.report(r, "Equidistancia cambiada.");
                    }
                },
            );
            row(
                ui,
                "Profundidad (mm)",
                "Bajo su punto más bajo",
                |ui, w| {
                    if let Some(v) =
                        length_field(ui, Id::unique(("terrain-depth", id.0)), t.depth, w - 8.0)
                    {
                        let r = self
                            .ed
                            .edit_terrain("Profundidad del terreno", id, |t| t.depth = v);
                        self.report(r, "Profundidad cambiada.");
                    }
                },
            );
            text_row(
                ui,
                "Cotas",
                "Más baja y más alta",
                &format!("{} … {}", height(lo), height(hi)),
            );
        });
        self.terrain_pick_group(ui, id, &t);
        group(ui, "terrain-identity", "Datos de identidad", |ui| {
            props::id_row(ui, "GlobalId", &t.guid.to_string());
        });
    }

    /// The points picked in plan: how many, and one height for all.
    fn terrain_pick_group(&mut self, ui: &mut Ui, id: TerrainId, t: &archisim_core::Terrain) {
        let pick: Vec<usize> = self.terrain_picked().map(|(_, p)| p).unwrap_or_default();
        group(ui, "terrain-points", "Puntos elegidos", |ui| {
            text_row(
                ui,
                "Puntos",
                "Del terreno, y líneas de nivel dibujadas",
                &format!("{} · {} líneas", t.points.len(), t.lines.len()),
            );
            if pick.is_empty() {
                props::note(
                    ui,
                    "Haz clic en la planta sobre un punto para elegirlo (Mayús: añadir o quitar) o sobre una línea de nivel para elegir todos sus puntos; aquí les das la cota de una vez. Supr los quita del terreno; arrastrándolos se mueven.",
                );
                return;
            }
            let zs: Vec<f64> = pick.iter().map(|&i| t.points[i].z).collect();
            let (lo, hi) = zs
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), z| {
                    (a.min(*z), b.max(*z))
                });
            text_row(
                ui,
                "Elegidos",
                "Puntos elegidos en la planta",
                &pick.len().to_string(),
            );
            if hi - lo > 1e-9 {
                text_row(
                    ui,
                    "Cotas ahora",
                    "Distintas: la que escribas se pone a todos",
                    &format!("{} … {}", height(lo), height(hi)),
                );
            }
            row(
                ui,
                "Cota (mm)",
                "De todos los puntos elegidos, respecto al ±0,00",
                |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique(("terrain-pick-z", id.0, pick.len(), pick[0])),
                        lo,
                        w - 8.0,
                    ) {
                        let r = self.ed.edit_terrain("Cota de puntos", id, |t| {
                            for &i in &pick {
                                t.points[i].z = v;
                            }
                        });
                        self.report(r, &format!("{} punto(s) a {}.", pick.len(), height(v)));
                    }
                },
            );
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                if ui.button("Quitar del terreno").clicked() {
                    self.delete_terrain_points();
                }
                if ui
                    .button("Soltar")
                    .on_hover_text("No elegir ningún punto")
                    .clicked()
                {
                    self.terrain_pick.clear();
                }
            });
        });
    }

    /// The true north in the project's properties.
    pub(super) fn north_row(&mut self, ui: &mut Ui) {
        let north = self.ed.model().true_north().to_degrees();
        row(
            ui,
            "Norte (°)",
            "Ángulo del norte geográfico respecto al arriba de la planta, en sentido horario",
            |ui, w| {
                let shown = format!("{:.1}", north).replace('.', ",");
                if let Some(v) =
                    text_edit_committed(ui, Id::unique("project-north"), &shown, w - 8.0)
                {
                    match v
                        .trim()
                        .trim_end_matches('°')
                        .replace(',', ".")
                        .parse::<f64>()
                    {
                        Ok(d) if d.is_finite() => {
                            let r = self.ed.set_true_north(d);
                            self.report(
                                r,
                                "Norte cambiado: se ve en las plantas exportadas y en el IFC.",
                            );
                        }
                        _ => self.complain(format!("«{v}» no es un ángulo: escribe 12 o -7,5.")),
                    }
                }
            },
        );
    }
}

/// A number typed with a decimal comma or point; `None` if it is not one.
fn parse_number(t: &str) -> Option<f64> {
    let t: String = t
        .trim()
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '\u{a0}')
        .collect();
    // «4.474.270,5» or «4474270.5».
    let t = if t.contains(',') {
        t.replace('.', "").replace(',', ".")
    } else {
        t
    };
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Metres with two decimals and a decimal comma.
fn metres(v: f64) -> String {
    format!("{v:.2}").replace('.', ",")
}

impl ArchiSimApp {
    /// Where the project is on the map, in the project's properties: the
    /// coordinate system, the UTM coordinates of the ±0,00 at plan 0,0 and
    /// its height above sea level. They go to the IFC (`IfcMapConversion`,
    /// latitude and longitude of the site).
    pub(super) fn georeference_group(&mut self, ui: &mut Ui) {
        let current = self.ed.model().georeference().cloned();
        let mut new = current.clone();
        group(
            ui,
            "project-georef",
            "Emplazamiento (georreferencia)",
            |ui| {
                let mut on = current.is_some();
                row(
                    ui,
                    "Georreferenciado",
                    "Sitúa el proyecto en el mapa (coordenadas UTM del ±0,00)",
                    |ui, _| {
                        if ui.switch(&mut on, "").changed() {
                            new = on.then(|| archisim_core::Georeference {
                                crs: "EPSG:25830".into(),
                                eastings: 0.0,
                                northings: 0.0,
                                height: 0.0,
                            });
                        }
                    },
                );
                let Some(g) = current.clone() else {
                    return;
                };
                let mut edited = g.clone();
                row(ui, "Sistema", "Sistema de coordenadas (EPSG)", |ui, w| {
                    let mut opts: Vec<(&str, &str)> = archisim_core::CRS_LIST
                        .iter()
                        .map(|(c, n)| (*c, *n))
                        .collect();
                    if !opts.iter().any(|(c, _)| *c == g.crs) {
                        opts.push((g.crs.as_str(), g.crs.as_str()));
                    }
                    let picked = choice(ui, "georef-crs", w, g.crs.as_str(), &opts);
                    edited.crs = picked.to_string();
                });
                for (label, hint, which) in [
                    (
                        "X (este, m)",
                        "Coordenada X UTM del punto 0,0 de la planta",
                        0,
                    ),
                    (
                        "Y (norte, m)",
                        "Coordenada Y UTM del punto 0,0 de la planta",
                        1,
                    ),
                    ("Cota del ±0,00 (m)", "Altitud sobre el nivel del mar", 2),
                ] {
                    let value = [g.eastings, g.northings, g.height][which];
                    row(ui, label, hint, |ui, w| {
                        if let Some(t) = text_edit_committed(
                            ui,
                            Id::unique(("georef", which)),
                            &metres(value),
                            w - 8.0,
                        ) {
                            match parse_number(&t) {
                                Some(v) => match which {
                                    0 => edited.eastings = v,
                                    1 => edited.northings = v,
                                    _ => edited.height = v,
                                },
                                None => self.complain(format!("«{t}» no es un número.")),
                            }
                        }
                    });
                }
                match g.lat_lon() {
                Some((lat, lon)) => {
                    text_row(
                        ui,
                        "Latitud, longitud",
                        "Calculadas desde las coordenadas UTM",
                        &format!("{lat:.6}°, {lon:.6}°").replace('.', ","),
                    );
                    text_row(
                        ui,
                        "Convergencia (°)",
                        "Ángulo entre el norte geográfico y el norte de la cuadrícula UTM",
                        &format!("{:.3}", g.convergence().to_degrees()).replace('.', ","),
                    );
                }
                None => props::note(
                    ui,
                    "Sistema no UTM: el IFC lleva las coordenadas, pero no la latitud ni la longitud.",
                ),
            }
                if edited != g {
                    new = Some(edited);
                }
            },
        );
        if new != current {
            let r = self.ed.set_georeference(new);
            self.report(r, "Emplazamiento cambiado: va en el IFC.");
        }
    }
}

#[cfg(test)]
mod georef_tests {
    #[test]
    fn numbers_with_commas_and_points() {
        assert_eq!(super::parse_number("440230"), Some(440_230.0));
        assert_eq!(super::parse_number("4.474.270,5"), Some(4_474_270.5));
        assert_eq!(super::parse_number(" 650,25 "), Some(650.25));
        assert_eq!(super::parse_number("4474270.5"), Some(4_474_270.5));
        assert_eq!(super::parse_number("abc"), None);
    }
}
