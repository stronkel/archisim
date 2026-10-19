//! The «Colaboración» menu and the «Coordinación» window: linked models,
//! clash detection, issues (BCF) and requirements (IDS), as in
//! Navisworks, Solibri or BIMcollab but inside the modelling program.
//! The logic is in `crate::coord`; this is what the person sees.

use std::path::PathBuf;

use archisim_core::Point2;
use eframe::egui::{self, Color32, RichText, Stroke, Ui};
use glam::Vec3;

use super::ArchiSimApp;
use crate::coord::{guid_of, CLASH_AMBER, CLASH_RED};
use crate::view3d::camera::{Camera, FOV_Y};

/// Tabs of the coordination window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CoordTab {
    #[default]
    Links,
    Clashes,
    Issues,
    Requirements,
}

/// What the window keeps between frames.
#[derive(Default)]
pub struct CoordWindow {
    pub open: bool,
    pub tab: CoordTab,
    /// Title of the issue being created from the 3D view.
    new_title: String,
    /// Comment being written on the selected issue.
    new_comment: String,
    /// Plan sections of the links: (serial, cut height) and segments.
    plan_cache: Option<((u64, u64), Vec<crate::coord::PlanSegment>)>,
}

/// A BCF camera from the 3D view's.
pub fn camera_to_bcf(c: &Camera, aspect: f32) -> archisim_bcf::Camera {
    let eye = c.eye();
    let dir = (c.target - eye).normalize_or(Vec3::NEG_Y);
    let right = dir.cross(Vec3::Z).normalize_or(Vec3::X);
    let up = right.cross(dir);
    let v = |v: Vec3| [v.x as f64, v.y as f64, v.z as f64];
    if c.perspective {
        archisim_bcf::Camera::Perspective {
            position: v(eye),
            direction: v(dir),
            up: v(up),
            field_of_view: (FOV_Y as f64).to_degrees(),
            aspect_ratio: aspect as f64,
        }
    } else {
        archisim_bcf::Camera::Orthogonal {
            position: v(eye),
            direction: v(dir),
            up: v(up),
            view_to_world_scale: 2.0 * (c.distance * (FOV_Y / 2.0).tan()) as f64,
            aspect_ratio: aspect as f64,
        }
    }
}

/// The 3D view's camera from a BCF one, looking at `focus` if given (the
/// middle of the elements of the issue), else 8 m ahead.
pub fn camera_from_bcf(b: &archisim_bcf::Camera, focus: Option<Vec3>) -> Camera {
    let f = |a: [f64; 3]| Vec3::new(a[0] as f32, a[1] as f32, a[2] as f32);
    let eye = f(b.position());
    let dir = f(b.direction()).normalize_or(Vec3::NEG_Y);
    let (perspective, distance) = match b {
        archisim_bcf::Camera::Orthogonal {
            view_to_world_scale,
            ..
        } => (
            false,
            (*view_to_world_scale as f32 / 2.0) / (FOV_Y / 2.0).tan(),
        ),
        archisim_bcf::Camera::Perspective { .. } => {
            // Along the line of sight, as far as the focus is.
            let d = focus.map_or(8.0, |p| (p - eye).dot(dir).max(1.0));
            (true, d)
        }
    };
    let back = -dir;
    Camera {
        target: eye + dir * distance,
        yaw: back.y.atan2(back.x),
        pitch: back.z.clamp(-1.0, 1.0).asin(),
        distance: distance.max(0.5),
        perspective,
    }
}

impl ArchiSimApp {
    /// Who signs issues and comments.
    fn author(&self) -> String {
        std::env::var("ARCHISIM_AUTOR")
            .or_else(|_| std::env::var("USER"))
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "ArchiSim".into())
    }

    /// The «Colaboración» menu.
    pub(super) fn coord_menu(&mut self, ui: &mut Ui) {
        ui.menu_button("Colaboración", |ui| {
            if ui.button("Vincular IFC…").clicked() {
                self.link_ifc_dialog();
                ui.close();
            }
            if ui.button("Vínculos y coordinación…").clicked() {
                self.coord_window.open = true;
                self.coord_window.tab = CoordTab::Links;
                ui.close();
            }
            ui.separator();
            if ui.button("Comprobar interferencias").clicked() {
                self.coord_window.open = true;
                self.coord_window.tab = CoordTab::Clashes;
                self.run_clash_check();
                ui.close();
            }
            if ui.button("Incidencias (BCF)…").clicked() {
                self.coord_window.open = true;
                self.coord_window.tab = CoordTab::Issues;
                ui.close();
            }
            if ui.button("Comprobar requisitos (IDS)…").clicked() {
                self.coord_window.open = true;
                self.coord_window.tab = CoordTab::Requirements;
                self.requirements_dialog();
                ui.close();
            }
        });
    }

    fn link_ifc_dialog(&mut self) {
        let mut d = rfd::FileDialog::new()
            .set_title("Vincular modelos IFC")
            .add_filter("IFC", &["ifc"]);
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        for path in d.pick_files().unwrap_or_default() {
            match self.coord.add_link(&path) {
                Ok(s) => self.say(s),
                Err(e) => self.complain(e),
            }
        }
    }

    fn run_clash_check(&mut self) {
        match self.coord.check_clashes(self.ed.model()) {
            Ok(s) => self.say(s),
            Err(e) => self.complain(e),
        }
    }

    fn requirements_dialog(&mut self) {
        let mut d = rfd::FileDialog::new()
            .set_title("Requisitos de información (IDS)")
            .add_filter("IDS", &["ids", "xml"]);
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        if let Some(path) = d.pick_file() {
            self.check_requirements(path);
        }
    }

    fn check_requirements(&mut self, path: PathBuf) {
        match self.coord.check_requirements(self.ed.model(), &path) {
            Ok(s) => self.say(s),
            Err(e) => self.complain(format!("No se pueden comprobar los requisitos: {e}")),
        }
    }

    /// New versions of linked files are read again on their own.
    pub(super) fn coord_poll(&mut self, ctx: &egui::Context) {
        if self.coord.links.is_empty() {
            return;
        }
        let now = ctx.input(|i| i.time);
        for r in self.coord.poll(now) {
            match r {
                Ok(s) => self.say(s),
                Err(e) => self.complain(e),
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_secs(3));
    }

    /// Linked models cut by the plan's cut plane, over the plan picture.
    pub(super) fn draw_links_in_plan(&mut self, painter: &egui::Painter, rect: egui::Rect) {
        if self.coord.links.is_empty() {
            return;
        }
        let base = self
            .ed
            .model()
            .level(self.ed.level)
            .map_or(0.0, |l| l.elevation);
        let z = base + self.cut_plane;
        let key = (self.coord.serial, z.to_bits());
        if self.coord_window.plan_cache.as_ref().map(|c| c.0) != Some(key) {
            self.coord_window.plan_cache = Some((key, self.coord.plan_cut(z)));
        }
        let Some((_, segments)) = &self.coord_window.plan_cache else {
            return;
        };
        for (seg, c) in segments {
            let a = self.view.to_screen(rect, Point2::new(seg[0][0], seg[0][1]));
            let b = self.view.to_screen(rect, Point2::new(seg[1][0], seg[1][1]));
            if !rect.contains(a)
                && !rect.contains(b)
                && !rect.intersects(egui::Rect::from_two_pos(a, b))
            {
                continue;
            }
            // Darker than in 3D: lines on white paper.
            let ink = Color32::from_rgb(c[0] / 2 + 20, c[1] / 2 + 20, c[2] / 2 + 20);
            painter.line_segment([a, b], Stroke::new(1.6, ink));
        }
    }

    /// Shows a point of the model: the 3D view looking at it.
    fn look_at(&mut self, point: [f64; 3], distance: f32) {
        self.set_mode(super::ViewMode::ThreeD);
        let cam = &mut self.v3d.camera;
        cam.target = Vec3::new(point[0] as f32, point[1] as f32, point[2] as f32);
        cam.distance = distance;
        self.v3d.fit_requested = false;
    }

    /// Selects the project's elements with these GlobalIds and paints the
    /// linked ones; returns how many of each were found.
    fn select_guids(&mut self, guids: &[String]) -> (usize, usize) {
        self.ed.selection.clear();
        let mut ours = 0;
        for g in guids {
            if let Some(e) = crate::coord::element_by_guid(self.ed.model(), g) {
                self.ed.selection.insert(e);
                ours += 1;
            }
        }
        self.coord.highlight_guids(guids, CLASH_RED);
        let theirs = self.coord.highlight.len();
        (ours, theirs)
    }

    /// Applies a BCF viewpoint: camera, selection and colours.
    fn apply_viewpoint(&mut self, vp: &archisim_bcf::Viewpoint) {
        let mut guids = vp.selection.clone();
        for (_, g) in &vp.coloring {
            guids.extend(g.iter().cloned());
        }
        guids.sort();
        guids.dedup();
        let (ours, theirs) = self.select_guids(&guids);
        self.coord.set_isolate(ours + theirs > 0);
        // The middle of what the issue is about, to orbit around it.
        let focus = self.focus_of(&guids);
        if let Some(cam) = &vp.camera {
            self.set_mode(super::ViewMode::ThreeD);
            self.v3d.camera = camera_from_bcf(cam, focus);
            self.v3d.fit_requested = false;
        } else if let Some(f) = focus {
            self.look_at([f.x as f64, f.y as f64, f.z as f64], 10.0);
        }
        let missing = guids.len().saturating_sub(ours + theirs);
        if missing > 0 {
            self.say(format!(
                "{missing} elementos de la incidencia no están en el proyecto ni en los vínculos."
            ));
        }
    }

    fn focus_of(&self, guids: &[String]) -> Option<Vec3> {
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        let mut any = false;
        let mut add = |b: Option<([f64; 3], [f64; 3])>| {
            if let Some((a, c)) = b {
                any = true;
                lo = lo.min(Vec3::new(a[0] as f32, a[1] as f32, a[2] as f32));
                hi = hi.max(Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32));
            }
        };
        for l in &self.coord.links {
            for e in &l.elements.elements {
                if guids.contains(&e.global_id) {
                    add(e.bounds);
                }
            }
        }
        // Project elements: through their solids.
        for solid in archisim_geom::solid::model_solids(self.ed.model()) {
            let el = match solid.owner {
                archisim_geom::solid::SolidOwner::Wall(id) => crate::pick::Element::Wall(id),
                archisim_geom::solid::SolidOwner::Slab(id) => crate::pick::Element::Slab(id),
                archisim_geom::solid::SolidOwner::Opening(id) => crate::pick::Element::Opening(id),
            };
            if !self.ed.selection.contains(&el) {
                continue;
            }
            add(archisim_coord::mesh::bounds_of(
                solid.triangles.iter().flatten().copied(),
            ));
        }
        any.then(|| (lo + hi) / 2.0)
    }

    /// The coordination window.
    pub(super) fn coord_dialog(&mut self, ctx: &egui::Context) {
        if !self.coord_window.open {
            return;
        }
        let mut open = true;
        egui::Window::new("Coordinación")
            .open(&mut open)
            .default_width(620.0)
            .default_height(460.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let tab = &mut self.coord_window.tab;
                    ui.selectable_value(tab, CoordTab::Links, "Vínculos");
                    ui.selectable_value(tab, CoordTab::Clashes, "Interferencias");
                    ui.selectable_value(tab, CoordTab::Issues, "Incidencias (BCF)");
                    ui.selectable_value(tab, CoordTab::Requirements, "Requisitos (IDS)");
                });
                ui.separator();
                match self.coord_window.tab {
                    CoordTab::Links => self.links_tab(ui),
                    CoordTab::Clashes => self.clashes_tab(ui),
                    CoordTab::Issues => self.issues_tab(ui),
                    CoordTab::Requirements => self.requirements_tab(ui),
                }
            });
        self.coord_window.open = open;
    }

    fn links_tab(&mut self, ui: &mut Ui) {
        ui.label(
            RichText::new(
                "Modelos de otras disciplinas como referencia: se ven en planta (cortados por el \
                 plano de corte) y en 3D, no se importan ni se exportan. Si el fichero cambia, se \
                 vuelve a leer solo y se colorean los cambios (verde: nuevos, naranja: modificados).",
            )
            .weak(),
        );
        ui.add_space(4.0);
        if ui.button("＋ Vincular IFC…").clicked() {
            self.link_ifc_dialog();
        }
        ui.add_space(6.0);
        let mut action: Option<(usize, &'static str)> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, l) in self.coord.links.iter().enumerate() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        let mut visible = l.visible;
                        if ui
                            .checkbox(&mut visible, "")
                            .on_hover_text("Visible")
                            .changed()
                        {
                            action = Some((i, if visible { "show" } else { "hide" }));
                        }
                        let [r, g, b] = l.color;
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(rect, 2.0, Color32::from_rgb(r, g, b));
                        ui.strong(l.name());
                        ui.label(
                            RichText::new(format!(
                                "{} · {} elementos",
                                l.elements.main_discipline().name(),
                                l.elements.elements.len()
                            ))
                            .weak(),
                        );
                    });
                    ui.horizontal(|ui| {
                        let mut t = l.transparent;
                        if ui.checkbox(&mut t, "Transparente").changed() {
                            action = Some((i, if t { "glass" } else { "solid" }));
                        }
                        if ui
                            .button("Recargar")
                            .on_hover_text("Leer de nuevo y ver qué ha cambiado")
                            .clicked()
                        {
                            action = Some((i, "reload"));
                        }
                        if ui.button("Quitar").clicked() {
                            action = Some((i, "remove"));
                        }
                    });
                    if l.elements.approximate > 0 || !l.elements.without_body.is_empty() {
                        ui.label(
                            RichText::new(format!(
                                "{} con geometría aproximada; {} sin geometría legible",
                                l.elements.approximate,
                                l.elements.without_body.values().sum::<usize>()
                            ))
                            .weak()
                            .small(),
                        );
                    }
                    if let Some(r) = &l.changes {
                        ui.horizontal(|ui| {
                            ui.label(format!(
                                "Última versión: {} cambios, {} sin cambios.",
                                r.changes.len(),
                                r.unchanged
                            ));
                            let mut show = l.show_changes;
                            if ui.checkbox(&mut show, "Colorear cambios").changed() {
                                action = Some((i, if show { "changes" } else { "nochanges" }));
                            }
                        });
                        egui::CollapsingHeader::new("Ver cambios")
                            .id_salt(("changes", i))
                            .show(ui, |ui| {
                                for c in r.changes.iter().take(300) {
                                    ui.label(
                                        RichText::new(format!(
                                            "{} «{}» — {}",
                                            archisim_ids::model::class_display(&c.class),
                                            c.name,
                                            c.describe()
                                        ))
                                        .small(),
                                    );
                                }
                            });
                    }
                });
            }
        });
        if let Some((i, what)) = action {
            match what {
                "show" => self.coord.set_visible(i, true),
                "hide" => self.coord.set_visible(i, false),
                "glass" => self.coord.set_transparent(i, true),
                "solid" => self.coord.set_transparent(i, false),
                "changes" => self.coord.set_show_changes(i, true),
                "nochanges" => self.coord.set_show_changes(i, false),
                "reload" => match self.coord.reload(i) {
                    Ok(s) => self.say(s),
                    Err(e) => self.complain(e),
                },
                "remove" => self.coord.remove_link(i),
                _ => {}
            }
        }
    }

    fn clashes_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Tolerancia (mm)");
            ui.add(
                egui::DragValue::new(&mut self.coord.tolerance_mm)
                    .range(0.5..=200.0)
                    .speed(0.5),
            )
            .on_hover_text("Lo que entra menos que esto no cuenta (contactos, redondeos)");
            if ui.button("Comprobar").clicked() {
                self.run_clash_check();
            }
            if self.coord.isolate
                && ui
                    .button("Ver todo")
                    .on_hover_text("El modelo entero, sin aislar la interferencia")
                    .clicked()
            {
                self.coord.set_isolate(false);
            }
            if !self.coord.clashes.is_empty() && ui.button("Pasar a incidencias").clicked() {
                let author = self.author();
                match self.coord.clashes_to_issues(self.ed.model(), &author) {
                    Ok(s) => {
                        self.say(s);
                        self.coord_window.tab = CoordTab::Issues;
                    }
                    Err(e) => self.complain(e),
                }
            }
        });
        ui.label(
            RichText::new(
                "Compara el proyecto con los vínculos visibles. No cuenta lo que solo se toca ni lo \
                 que pasa por un hueco. Un clic en una fila la muestra en 3D.",
            )
            .weak(),
        );
        match &self.coord.clashes_checked {
            None => {
                ui.label("Aún no se ha comprobado.");
                return;
            }
            Some(when) if self.coord.clashes.is_empty() => {
                ui.label(format!("✔ Sin interferencias ({when})."));
                return;
            }
            Some(_) => {}
        }
        let mut go: Option<usize> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("clashes")
                .num_columns(5)
                .striped(true)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.strong("#");
                    ui.strong("Proyecto");
                    ui.strong("Vínculo");
                    ui.strong("Entra");
                    ui.strong("Planta");
                    ui.end_row();
                    for (i, c) in self.coord.clashes.iter().enumerate() {
                        if ui.link(format!("{}", i + 1)).clicked() {
                            go = Some(i);
                        }
                        if ui.link(&c.ours_label).clicked() {
                            go = Some(i);
                        }
                        let link = self
                            .coord
                            .links
                            .get(c.link)
                            .map(|l| l.name())
                            .unwrap_or_default();
                        ui.label(format!("{} ({link})", c.theirs_label));
                        ui.label(archisim_coord::versions::metres(c.depth));
                        ui.label(c.storey.clone().unwrap_or_default());
                        ui.end_row();
                    }
                });
        });
        if let Some(i) = go {
            let c = self.coord.clashes[i].clone();
            self.ed.selection.clear();
            if let Some(e) = crate::coord::element_by_guid(self.ed.model(), &c.ours) {
                self.ed.selection.insert(e);
            }
            self.coord.highlight.clear();
            for other in &self.coord.clashes {
                self.coord
                    .highlight
                    .insert((other.link, other.theirs), CLASH_RED);
            }
            self.coord.highlight.insert((c.link, c.theirs), CLASH_AMBER);
            self.coord.selected_linked = None;
            self.coord.isolate = true;
            self.coord.highlight_changed();
            self.look_at(c.point, 6.0);
            self.say(format!(
                "{} × {}: {}.",
                c.ours_label,
                c.theirs_label,
                archisim_coord::versions::metres(c.depth)
            ));
        }
    }

    fn issues_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button("Abrir BCF…").clicked() {
                let mut d = rfd::FileDialog::new()
                    .set_title("Abrir incidencias")
                    .add_filter("BCF", &["bcf", "bcfzip"]);
                if let Some(dir) = self.start_dir() {
                    d = d.set_directory(dir);
                }
                if let Some(p) = d.pick_file() {
                    match self.coord.open_issues(&p) {
                        Ok(s) => self.say(s),
                        Err(e) => self.complain(format!("No se puede abrir: {e}")),
                    }
                }
            }
            if self.coord.issues.is_none() && ui.button("Nueva lista").clicked() {
                self.coord.issues = Some(crate::coord::Issues {
                    path: None,
                    bcf: archisim_bcf::Bcf {
                        extensions: archisim_bcf::Extensions::archisim(),
                        ..Default::default()
                    },
                    modified: true,
                    selected: None,
                });
            }
            if let Some(issues) = &self.coord.issues {
                let has_path = issues.path.is_some();
                if ui.button("Guardar").clicked() {
                    self.save_issues(!has_path);
                }
                if ui.button("Guardar como…").clicked() {
                    self.save_issues(true);
                }
            }
            if let Some(issues) = &mut self.coord.issues {
                // BCF 2.1 for programs that do not read 3.0 yet.
                let v = &mut issues.bcf.version;
                let before = *v;
                egui::ComboBox::from_id_salt("bcf-version")
                    .selected_text(format!("BCF {}", v.id()))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(v, archisim_bcf::BcfVersion::V3_0, "BCF 3.0");
                        ui.selectable_value(v, archisim_bcf::BcfVersion::V2_1, "BCF 2.1");
                    })
                    .response
                    .on_hover_text(
                        "Versión con la que se guarda (2.1 para programas que aún no leen la 3.0)",
                    );
                if *v != before {
                    issues.modified = true;
                }
            }
        });
        // New issue from the 3D view.
        let mut create = false;
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.coord_window.new_title)
                    .hint_text("Título de una incidencia nueva")
                    .desired_width(300.0),
            );
            create = ui
                .button("＋ Desde la vista 3D")
                .on_hover_text("Guarda la cámara, lo seleccionado y una imagen de la vista 3D")
                .clicked();
        });
        if create {
            self.new_issue_from_view();
        }
        let Some(issues) = &self.coord.issues else {
            ui.label(
                RichText::new(
                    "Abre un fichero BCF de Revit, ArchiCAD, Solibri o BIMcollab, o pasa aquí las \
                     interferencias o los requisitos incumplidos.",
                )
                .weak(),
            );
            return;
        };
        ui.label(
            RichText::new(format!(
                "{}{} · BCF {} · {} incidencias",
                issues
                    .path
                    .as_ref()
                    .map_or("Sin guardar".to_string(), |p| p.display().to_string()),
                if issues.modified {
                    " (con cambios)"
                } else {
                    ""
                },
                issues.bcf.version.id(),
                issues.bcf.topics.len()
            ))
            .weak(),
        );
        ui.separator();
        let mut select: Option<usize> = None;
        let selected = issues.selected;
        egui::ScrollArea::vertical()
            .id_salt("topics")
            .max_height(180.0)
            .show(ui, |ui| {
                for (i, t) in issues.bcf.topics.iter().enumerate() {
                    let text = format!(
                        "{}  [{}]  {}{}",
                        t.index.map(|n| format!("#{n}")).unwrap_or_default(),
                        t.status.as_deref().unwrap_or("?"),
                        t.title,
                        t.assigned_to
                            .as_deref()
                            .map(|a| format!("  → {a}"))
                            .unwrap_or_default()
                    );
                    if ui.selectable_label(selected == Some(i), text).clicked() {
                        select = Some(i);
                    }
                }
            });
        if let Some(i) = select {
            if let Some(issues) = &mut self.coord.issues {
                issues.selected = Some(i);
            }
            let vp = self
                .coord
                .issues
                .as_ref()
                .and_then(|is| is.bcf.topics.get(i))
                .and_then(|t| t.viewpoints.first().cloned());
            if let Some(vp) = vp {
                self.apply_viewpoint(&vp);
            }
        }
        ui.separator();
        self.topic_detail(ui);
    }

    fn topic_detail(&mut self, ui: &mut Ui) {
        let author = self.author();
        let Some(issues) = &mut self.coord.issues else {
            return;
        };
        let Some(i) = issues.selected else {
            ui.label(
                RichText::new("Elige una incidencia para ver su vista y sus comentarios.").weak(),
            );
            return;
        };
        let statuses = if issues.bcf.extensions.topic_statuses.is_empty() {
            archisim_bcf::Extensions::archisim().topic_statuses
        } else {
            issues.bcf.extensions.topic_statuses.clone()
        };
        let Some(t) = issues.bcf.topics.get_mut(i) else {
            return;
        };
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.strong(&t.title);
        });
        ui.horizontal(|ui| {
            ui.label("Estado");
            let current = t.status.clone().unwrap_or_default();
            egui::ComboBox::from_id_salt("status")
                .selected_text(&current)
                .show_ui(ui, |ui| {
                    for s in &statuses {
                        if ui.selectable_label(*s == current, s).clicked() {
                            t.status = Some(s.clone());
                            changed = true;
                        }
                    }
                });
            ui.label("Asignada a");
            let mut who = t.assigned_to.clone().unwrap_or_default();
            if ui
                .add(egui::TextEdit::singleline(&mut who).desired_width(180.0))
                .lost_focus()
                && Some(&who) != t.assigned_to.as_ref()
            {
                t.assigned_to = (!who.is_empty()).then_some(who);
                changed = true;
            }
        });
        if let Some(d) = &t.description {
            ui.label(d);
        }
        egui::ScrollArea::vertical()
            .id_salt("comments")
            .max_height(140.0)
            .show(ui, |ui| {
                for c in &t.comments {
                    ui.label(
                        RichText::new(format!("{} · {}", c.author, c.date))
                            .weak()
                            .small(),
                    );
                    ui.label(&c.text);
                    ui.add_space(3.0);
                }
            });
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.coord_window.new_comment)
                    .hint_text("Comentario")
                    .desired_width(380.0),
            );
            if ui.button("Comentar").clicked() && !self.coord_window.new_comment.trim().is_empty() {
                t.comments.push(archisim_bcf::Comment {
                    guid: archisim_bcf::new_guid(),
                    date: archisim_bcf::now(),
                    author: author.clone(),
                    text: std::mem::take(&mut self.coord_window.new_comment),
                    viewpoint: None,
                    modified_date: None,
                    modified_author: None,
                });
                changed = true;
            }
        });
        if changed {
            t.modified_date = Some(archisim_bcf::now());
            t.modified_author = Some(author);
            issues.modified = true;
        }
    }

    fn save_issues(&mut self, ask: bool) {
        let path = if ask {
            let mut d = rfd::FileDialog::new()
                .set_title("Guardar incidencias")
                .add_filter("BCF", &["bcf"])
                .set_file_name(format!("{} - incidencias.bcf", self.file_name_stem()));
            if let Some(dir) = self.start_dir() {
                d = d.set_directory(dir);
            }
            match d.save_file() {
                Some(p) => Some(p),
                None => return,
            }
        } else {
            None
        };
        match self.coord.save_issues(path.as_deref()) {
            Ok(s) => self.say(s),
            Err(e) => self.complain(e),
        }
    }

    /// A new issue with the 3D view's camera, the selection and a picture.
    fn new_issue_from_view(&mut self) {
        let title = self.coord_window.new_title.trim().to_string();
        if title.is_empty() {
            self.complain("Escribe primero el título de la incidencia.");
            return;
        }
        let aspect = {
            let r = self.v3d.canvas;
            if r.height() > 1.0 {
                r.width() / r.height()
            } else {
                1.6
            }
        };
        let mut vp = archisim_bcf::Viewpoint::new(archisim_bcf::new_guid());
        vp.camera = Some(camera_to_bcf(&self.v3d.camera, aspect));
        let model = self.ed.model();
        vp.selection = self
            .ed
            .selection
            .iter()
            .filter_map(|e| guid_of(model, *e))
            .collect();
        if let Some((li, ei)) = self.coord.selected_linked {
            if let Some(e) = self
                .coord
                .links
                .get(li)
                .and_then(|l| l.elements.elements.get(ei))
            {
                vp.selection.push(e.global_id.clone());
            }
        }
        vp.snapshot = self.v3d.picture().and_then(|(w, h, px)| {
            resvg::tiny_skia::IntSize::from_wh(w, h)
                .and_then(|size| resvg::tiny_skia::Pixmap::from_vec(px, size))
                .and_then(|p| p.encode_png().ok())
        });
        let author = self.author();
        let model_file = format!("{}.ifc", self.file_name_stem());
        let issues = self
            .coord
            .issues
            .get_or_insert_with(|| crate::coord::Issues {
                path: None,
                bcf: archisim_bcf::Bcf {
                    extensions: archisim_bcf::Extensions::archisim(),
                    ..Default::default()
                },
                modified: true,
                selected: None,
            });
        let index = issues
            .bcf
            .topics
            .iter()
            .filter_map(|t| t.index)
            .max()
            .unwrap_or(0)
            + 1;
        let first = |l: &[String], or: &str| l.first().cloned().unwrap_or_else(|| or.into());
        issues.bcf.topics.push(archisim_bcf::Topic {
            guid: archisim_bcf::new_guid(),
            topic_type: Some(first(&issues.bcf.extensions.topic_types, "Consulta")),
            status: Some(first(&issues.bcf.extensions.topic_statuses, "Abierta")),
            title,
            priority: Some("Normal".into()),
            index: Some(index),
            creation_date: archisim_bcf::now(),
            creation_author: author,
            files: vec![archisim_bcf::FileRef {
                is_external: true,
                filename: Some(model_file),
                ..Default::default()
            }],
            viewpoints: vec![vp],
            ..Default::default()
        });
        issues.selected = Some(issues.bcf.topics.len() - 1);
        issues.modified = true;
        self.coord_window.new_title.clear();
        self.say(format!("Incidencia #{index} creada desde la vista 3D."));
    }

    fn requirements_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button("Comprobar requisitos (IDS)…").clicked() {
                self.requirements_dialog();
            }
            let again = self.coord.requirements.as_ref().map(|r| r.path.clone());
            if let Some(p) = again {
                if ui
                    .button("Repetir")
                    .on_hover_text(p.display().to_string())
                    .clicked()
                {
                    self.check_requirements(p);
                }
                if ui.button("Pasar a incidencias").clicked() {
                    let author = self.author();
                    match self.coord.requirements_to_issues(self.ed.model(), &author) {
                        Ok(s) => {
                            self.say(s);
                            self.coord_window.tab = CoordTab::Issues;
                        }
                        Err(e) => self.complain(e),
                    }
                }
            }
        });
        let Some(req) = &self.coord.requirements else {
            ui.label(
                RichText::new(
                    "Un fichero IDS (buildingSMART) dice qué información pide el cliente: clases, \
                     propiedades, materiales, clasificaciones. Se comprueba el proyecto tal como \
                     sale en su IFC.",
                )
                .weak(),
            );
            return;
        };
        let r = &req.report;
        ui.label(format!(
            "«{}»: cumple {} de {} especificaciones.",
            if r.title.is_empty() {
                "Requisitos"
            } else {
                &r.title
            },
            r.specifications.iter().filter(|s| s.passed).count(),
            r.specifications.len()
        ));
        let mut pick: Option<String> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, s) in r.specifications.iter().enumerate() {
                let head = format!(
                    "{} {}  ({} afectados, {} incumplen)",
                    if s.passed { "✔" } else { "✘" },
                    s.name,
                    s.applicable,
                    s.failures.len()
                );
                let color = if s.passed {
                    Color32::from_rgb(40, 150, 70)
                } else {
                    Color32::from_rgb(200, 50, 50)
                };
                egui::CollapsingHeader::new(RichText::new(head).color(color))
                    .id_salt(("spec", i))
                    .show(ui, |ui| {
                        if let Some(n) = &s.note {
                            ui.label(RichText::new(n).weak());
                        }
                        for f in s.failures.iter().take(200) {
                            let label = format!("{}: {}", f.element, f.reasons.join("; "));
                            if ui.link(RichText::new(label).small()).clicked() {
                                pick = f.global_id.clone();
                            }
                        }
                    });
            }
        });
        if let Some(g) = pick {
            let (ours, _) = self.select_guids(std::slice::from_ref(&g));
            if ours == 0 {
                self.say("Ese elemento no se puede seleccionar en el proyecto.");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::Editor;
    use eframe::egui::{Pos2, RawInput, Rect, Vec2};

    fn frame(ctx: &egui::Context, app: &mut ArchiSimApp, t: &mut f64) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1600.0, 1000.0))),
            time: Some(*t),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| app.show(ui));
        out.textures_delta.clear();
        *t += 0.05;
    }

    /// Link, clash check, issues, a viewpoint applied, a new issue from
    /// the 3D view and every tab drawn, without a screen.
    #[test]
    fn coordination_window_flow() {
        let dir = std::env::temp_dir().join(format!("archisim-coordui-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let link = dir.join("estructura.ifc");
        // A beam through the room's west wall, cut by the plan's cut plane.
        std::fs::write(
            &link,
            "ISO-10303-21;HEADER;FILE_DESCRIPTION((''),'2;1');FILE_NAME('e.ifc','',(''),(''),'','Prueba','');\
             FILE_SCHEMA(('IFC4'));ENDSEC;DATA;\
             #1=IFCPROJECT('0p000000000000000000p1',$,'Estructura',$,$,$,$,$,#2);\
             #2=IFCUNITASSIGNMENT((#3));#3=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);#4=IFCDIRECTION((0.,0.,1.));\
             #10=IFCCARTESIANPOINT((0.,2.,0.9));#11=IFCAXIS2PLACEMENT3D(#10,$,$);#12=IFCLOCALPLACEMENT($,#11);\
             #13=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,2.,0.3);#14=IFCEXTRUDEDAREASOLID(#13,$,#4,0.4);\
             #15=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#14));#16=IFCPRODUCTDEFINITIONSHAPE($,$,(#15));\
             #20=IFCBEAM('0b000000000000000000b1',$,'Viga',$,$,#12,#16,$,$);\
             ENDSEC;END-ISO-10303-21;",
        )
        .unwrap();
        let (model, _, _) = crate::pick::tests::room();
        let mut app = ArchiSimApp::new(Editor::with_model(model, None));
        let ctx = egui::Context::default();
        let mut t = 0.0;
        frame(&ctx, &mut app, &mut t);
        app.coord.add_link(&link).unwrap();
        app.coord_window.open = true;
        for tab in [CoordTab::Links, CoordTab::Clashes] {
            app.coord_window.tab = tab;
            frame(&ctx, &mut app, &mut t);
        }
        // The plan shows the beam cut at 1,10 m.
        assert!(app
            .coord_window
            .plan_cache
            .as_ref()
            .is_some_and(|(_, segs)| !segs.is_empty()));

        app.run_clash_check();
        assert_eq!(app.coord.clashes.len(), 1, "{:?}", app.message);
        frame(&ctx, &mut app, &mut t);
        let author = app.author();
        app.coord
            .clashes_to_issues(app.ed.model(), &author)
            .unwrap();
        app.coord_window.tab = CoordTab::Issues;
        frame(&ctx, &mut app, &mut t);

        // Going to the issue: 3D, the wall selected, the beam painted.
        let vp = app.coord.issues.as_ref().unwrap().bcf.topics[0].viewpoints[0].clone();
        app.apply_viewpoint(&vp);
        assert_eq!(app.mode, super::super::ViewMode::ThreeD);
        assert!(app
            .ed
            .selection
            .iter()
            .any(|e| matches!(e, crate::pick::Element::Wall(_))));
        assert_eq!(app.coord.highlight.get(&(0, 0)), Some(&CLASH_RED));
        frame(&ctx, &mut app, &mut t);

        // A new issue from the 3D view keeps the camera and the selection.
        app.coord_window.new_title = "Revisar apoyo".into();
        app.new_issue_from_view();
        let issues = app.coord.issues.as_ref().unwrap();
        let topic = issues.bcf.topics.last().unwrap();
        assert_eq!(topic.title, "Revisar apoyo");
        assert!(topic.viewpoints[0].camera.is_some());
        assert!(!topic.viewpoints[0].selection.is_empty());
        // Requirements: the sample IDS against the room.
        let ids = dir.join("requisitos.ids");
        std::fs::write(
            &ids,
            include_str!("../../../archisim-ids/tests/fixtures/vivienda.ids"),
        )
        .unwrap();
        app.check_requirements(ids);
        let report = &app.coord.requirements.as_ref().unwrap().report;
        assert_eq!(report.specifications.len(), 7);
        app.coord_window.tab = CoordTab::Requirements;
        frame(&ctx, &mut app, &mut t);
        let author = app.author();
        app.coord
            .requirements_to_issues(app.ed.model(), &author)
            .unwrap();
        // Every clash topic is still open (none resolved by mistake).
        let issues = app.coord.issues.as_ref().unwrap();
        assert!(issues
            .bcf
            .topics
            .iter()
            .all(|t| t.status.as_deref() == Some("Abierta")));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn camera_survives_bcf() {
        let cam = Camera {
            target: Vec3::new(5.0, 3.0, 1.5),
            yaw: 0.7,
            pitch: 0.4,
            distance: 12.0,
            perspective: true,
        };
        let b = camera_to_bcf(&cam, 1.6);
        let back = camera_from_bcf(&b, Some(cam.target));
        assert!((back.target - cam.target).length() < 1e-3);
        assert!((back.yaw - cam.yaw).abs() < 1e-4);
        assert!((back.pitch - cam.pitch).abs() < 1e-4);
        assert!((back.distance - cam.distance).abs() < 1e-3);
        let ortho = Camera {
            perspective: false,
            ..cam
        };
        let back = camera_from_bcf(&camera_to_bcf(&ortho, 1.6), None);
        assert!(!back.perspective);
        assert!((back.distance - ortho.distance).abs() < 1e-3);
        assert!((back.target - ortho.target).length() < 1e-3);
    }
}
