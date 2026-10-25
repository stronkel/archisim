//! Colaboración → Trabajo en equipo: sharing a project in a folder that
//! Google Drive (or a NAS) keeps in sync, opening it, the people in the top
//! bar, Publicar and Recibir, the Equipo window (people, reservations,
//! requests, history) and the reservations drawn in plan, each person in
//! their colour. The work itself (folder, changes, reservations) is in
//! `archisim-team`; the guard of the edits in `crate::team`.

use super::widgets::SwitchExt as _;
use std::path::{Path, PathBuf};

use archisim_core::Point2;
use archisim_team::{Member, Session};
use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke, Ui, Vec2};

use super::props::group;
use super::theme::Palette;
use super::widgets::strong_font;
use super::*;
use crate::team::{raw_id, Guard, Identity, ModelLocator, Team};

/// Something a button asks for, done once the window is drawn.
type Deferred = Box<dyn FnOnce(&mut ArchiSimApp)>;

/// The state of the team windows.
#[derive(Default)]
pub struct TeamUi {
    /// The Equipo window.
    pub open: bool,
    /// Note for the next publish.
    pub note: String,
    /// A note to someone.
    pub message: String,
    pub message_to: String,
    /// Name for the next zone.
    pub zone_name: String,
    /// First corner of the zone being drawn.
    pub zone_start: Option<Point2>,
    /// Sharing or opening a shared project.
    pub dialog: Option<TeamDialog>,
}

/// Asks for the person's name (and the folder, to share).
pub struct TeamDialog {
    /// Sharing the open project (else opening `folder`).
    pub share: bool,
    pub folder: Option<PathBuf>,
    pub name: String,
    /// People already in the project (to open as one of them).
    pub members: Vec<Member>,
}

fn color(m: &Member) -> Color32 {
    let [r, g, b] = m.rgb();
    Color32::from_rgb(r, g, b)
}

/// «hace 5 min».
fn ago(t: u64) -> String {
    let s = archisim_team::folder::now().saturating_sub(t);
    match s {
        0..=59 => "ahora".into(),
        60..=3599 => format!("hace {} min", s / 60),
        3600..=86_399 => format!("hace {} h", s / 3600),
        _ => format!(
            "hace {} día{}",
            s / 86_400,
            if s / 86_400 == 1 { "" } else { "s" }
        ),
    }
}

impl ArchiSimApp {
    // ---- Menu and starting -------------------------------------------

    /// The team items of the Colaboración menu.
    pub(super) fn team_menu_items(&mut self, ui: &mut Ui) {
        ui.separator();
        if self.team.is_none() {
            if ui
                .button("Compartir en equipo…")
                .on_hover_text("Pone el proyecto en una carpeta compartida (Google Drive, un NAS) para trabajar en él varias personas a la vez")
                .clicked()
            {
                self.team_ui.dialog = Some(TeamDialog {
                    share: true,
                    folder: None,
                    name: Identity::load().name,
                    members: Vec::new(),
                });
                ui.close();
            }
            if ui.button("Abrir proyecto de equipo…").clicked() {
                self.pick_team_folder();
                ui.close();
            }
        } else {
            if ui.button("Equipo…").clicked() {
                self.team_ui.open = true;
                ui.close();
            }
            if ui.button("Recibir").clicked() {
                self.team_sync(false);
                ui.close();
            }
            if ui.button("Publicar y recibir").clicked() {
                self.team_sync(true);
                ui.close();
            }
            if ui
                .button("Dejar el trabajo en equipo")
                .on_hover_text("Cierra el proyecto compartido; lo no publicado queda en tu copia de trabajo de la carpeta")
                .clicked()
            {
                self.leave_team();
                ui.close();
            }
        }
    }

    pub(super) fn pick_team_folder(&mut self) {
        let mut d = rfd::FileDialog::new()
            .set_title("Abrir proyecto de equipo (la carpeta compartida o su equipo.asequipo)")
            .add_filter("Proyecto de equipo", &[archisim_team::EXTENSION]);
        if let Some(dir) = self.start_dir() {
            d = d.set_directory(dir);
        }
        if let Some(p) = d.pick_file() {
            self.open_team_as(&p, true);
        }
    }

    /// Opens a shared project: straight away if this computer knows who
    /// is opening it, else asking.
    pub fn open_team(&mut self, path: &Path) {
        self.open_team_as(path, false);
    }

    /// Opens a shared project; with `ask`, always asking who opens it (to
    /// open it as someone else, as when trying it with two names on one
    /// computer).
    pub fn open_team_as(&mut self, path: &Path, ask: bool) {
        let Some(root) = archisim_team::team_folder_of(path) else {
            self.complain(format!("{} no es un proyecto de equipo.", path.display()));
            return;
        };
        let members = match archisim_team::members(&root) {
            Ok(m) => m,
            Err(e) => {
                self.complain(format!("No se puede abrir el proyecto de equipo: {e}."));
                return;
            }
        };
        let me = Identity::load();
        let known = !me.name.is_empty()
            && members
                .iter()
                .any(|m| m.slug == archisim_team::slug(&me.name));
        if known && !ask {
            self.start_team(&root, &me.name, false);
        } else {
            self.team_ui.dialog = Some(TeamDialog {
                share: false,
                folder: Some(root),
                name: me.name,
                members,
            });
        }
    }

    /// Shares or opens with a name.
    pub(super) fn start_team(&mut self, root: &Path, name: &str, share: bool) {
        let mut me = Identity::load();
        me.name = name.trim().to_string();
        me.save();
        let result = if share {
            let model = self.ed.model().clone();
            Session::create(root, &model, &me.name, &me.machine)
                .map(|(s, m)| (s, m, archisim_team::Report::default()))
        } else {
            Session::open(root, &me.name, &me.machine)
        };
        match result {
            Ok((session, model, report)) => {
                let first = !share && report.received.is_empty();
                let mut ed = Editor::with_model(model, None);
                ed.team = Some(Guard::of(&session));
                self.replace_editor(ed);
                let mut team = Team::new(session, self.ed.revision());
                let _ = team.session.touch(Some(self.ed.level.0), None, true);
                let name = team.session.team.name.clone();
                let others = team.session.members().len().saturating_sub(1);
                let elsewhere = team.session.open_elsewhere();
                self.team = Some(team);
                self.note_recent(&root.join(archisim_team::TEAM_FILE));
                self.team_ui.open = true;
                let mut text = if share {
                    format!(
                        "Proyecto compartido en {}. Los demás lo abren con Colaboración → Abrir proyecto de equipo.",
                        root.display()
                    )
                } else if first {
                    format!(
                        "Abierto el proyecto de equipo «{name}» ({others} persona{} más).",
                        if others == 1 { "" } else { "s" }
                    )
                } else {
                    format!("Abierto «{name}». {report}")
                };
                if elsewhere {
                    text.push_str(" Atención: también lo tienes abierto en otro ordenador; ciérralo allí para no pisar tu copia de trabajo.");
                }
                self.say(text);
            }
            Err(e) => self.complain(format!("No se puede trabajar en equipo: {e}.")),
        }
    }

    /// Ends the team session: the project stays open as an unsaved copy.
    pub(super) fn leave_team(&mut self) {
        if let Some(mut team) = self.team.take() {
            let model = self.ed.model().clone();
            let _ = team.session.save_work(&model);
            let _ = team.session.touch(None, None, false);
        }
        self.ed.team = None;
        self.say("Has dejado el trabajo en equipo. Lo no publicado sigue en tu copia de trabajo de la carpeta.");
    }

    /// Writes my working copy to the folder (Guardar, in a shared project).
    pub(super) fn team_save(&mut self) -> bool {
        let model = self.ed.model().clone();
        let Some(team) = self.team.as_mut() else {
            return false;
        };
        match team.session.save_work(&model) {
            Ok(()) => {
                self.ed.mark_saved();
                self.say("Copia de trabajo guardada en la carpeta del equipo (publica para que la vean los demás).");
                true
            }
            Err(e) => {
                self.complain(format!("No se pudo guardar la copia de trabajo: {e}."));
                false
            }
        }
    }

    /// On closing the program: the working copy saved, and gone from the
    /// people online.
    pub(super) fn team_closing(&mut self) {
        let model = self.ed.model().clone();
        if let Some(team) = self.team.as_mut() {
            if team.session.save_work(&model).is_ok() {
                self.ed.mark_saved();
            }
            let _ = team.session.touch(None, None, false);
        }
    }

    // ---- Publishing and receiving --------------------------------------

    /// Receives (and, with `publish`, publishes) and puts the result in
    /// place.
    pub(super) fn team_sync(&mut self, publish: bool) {
        let model = self.ed.model().clone();
        let note = std::mem::take(&mut self.team_ui.note);
        let Some(team) = self.team.as_mut() else {
            return;
        };
        let loc = ModelLocator::new(&model);
        let result = if publish {
            team.session.publish(&model, &note, &loc)
        } else {
            team.session.receive(&model, &loc)
        };
        match result {
            Ok((new, report)) => {
                let changed = new != model;
                let guard = Guard::of(&team.session);
                if changed {
                    self.ed.reset_model(new);
                } else {
                    self.ed.mark_saved();
                }
                self.ed.team = Some(guard);
                if let Some(team) = self.team.as_mut() {
                    team.synced_revision = self.ed.revision();
                    team.counted_revision = self.ed.revision();
                    team.local = 0;
                }
                self.plan = None;
                if report.dropped.is_empty() && report.skipped.is_empty() {
                    self.say(report.to_string());
                } else {
                    self.complain(report.to_string());
                }
            }
            Err(e) => {
                self.team_ui.note = note;
                self.complain(format!(
                    "No se pudo {}: {e}.",
                    if publish { "publicar" } else { "recibir" }
                ));
            }
        }
    }

    /// Looks at the folder now and then: others' changes, reservations,
    /// presence and messages; says I am here; keeps my working copy.
    pub(super) fn team_poll(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        let level = self.ed.level.0;
        let cursor = self.cursor.map(|c| [c.point.x, c.point.y]);
        let modified = self.ed.is_modified();
        let model_for_save = modified.then(|| self.ed.model().clone());
        // Elements edited that nobody had: reserved now.
        let auto: Vec<u64> = self
            .ed
            .team
            .as_mut()
            .map(|g| std::mem::take(&mut g.auto).into_iter().collect())
            .unwrap_or_default();
        let Some(team) = self.team.as_mut() else {
            return;
        };
        let mut problem = None;
        if !auto.is_empty() {
            let model = self.ed.model();
            if let Err(e) = team.session.reserve(&auto, &ModelLocator::new(model)) {
                problem = Some(e);
            }
        }
        let mut guard_stale = !auto.is_empty();
        if now - team.polled > 3.0 {
            team.polled = now;
            match team.session.refresh() {
                Ok(r) => {
                    guard_stale |= r.reservations || r.members;
                    if r.messages && team.session.unread() > 0 {
                        self.message = Some(Message {
                            text: "Tienes un mensaje del equipo (Colaboración → Equipo).".into(),
                            error: false,
                        });
                    }
                }
                Err(e) => problem = Some(e),
            }
            // A working copy at least every minute, in case of a crash.
            if let Some(m) = &model_for_save {
                if now - team.touched > 60.0 {
                    let _ = team.session.save_work(m);
                }
            }
        }
        let share = team.share_cursor;
        let interval = if share { 1.0 } else { 20.0 };
        let moved_level = team
            .session
            .presence_of(&team.session.me.slug.clone())
            .and_then(|p| p.level)
            != Some(level);
        if now - team.touched > interval || moved_level {
            team.touched = now;
            if let Err(e) = team
                .session
                .touch(Some(level), if share { cursor } else { None }, true)
            {
                problem = Some(e);
            }
        }
        if guard_stale {
            let mut g = Guard::of(&team.session);
            if let Some(old) = self.ed.team.as_ref() {
                g.auto = old.auto.clone();
            }
            self.ed.team = Some(g);
        }
        if let Some(e) = problem {
            self.complain(format!("Equipo: {e}."));
        }
        ctx.request_repaint_after(std::time::Duration::from_secs(if share { 1 } else { 3 }));
    }

    // ---- Top bar --------------------------------------------------------

    /// The people (a circle each, in their colour; faded if away) and
    /// Recibir and Publicar, in the top bar (right to left).
    pub(super) fn team_top_bar(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        let revision = self.ed.revision();
        let model = self.ed.model().clone();
        let Some(team) = self.team.as_mut() else {
            return;
        };
        let local = team.local_changes(&model, revision);
        let incoming = team.session.unreceived();
        let mut publish = false;
        let mut receive = false;
        let label = if local > 0 {
            format!("Publicar ({local})")
        } else {
            "Publicar".into()
        };
        if ui
            .add_enabled(
                local > 0,
                egui::Button::new(egui::RichText::new(label).font(strong_font(12.0))),
            )
            .on_hover_text("Envía tus cambios al equipo (y recibe los suyos)")
            .clicked()
        {
            publish = true;
        }
        let label = if incoming > 0 {
            format!("Recibir ({incoming})")
        } else {
            "Recibir".into()
        };
        let r = ui
            .add(egui::Button::new(
                egui::RichText::new(label).color(if incoming > 0 { p.accent } else { p.text }),
            ))
            .on_hover_text("Trae los cambios que han publicado los demás");
        if r.clicked() {
            receive = true;
        }
        // People, newest first from the right.
        let members: Vec<Member> = team.session.members().to_vec();
        let mut open_window = false;
        for m in members.iter().rev() {
            let online = team.session.is_online(&m.slug);
            let me = m.slug == team.session.me.slug;
            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::click());
            let c = color(m);
            let fill = if online { c } else { c.gamma_multiply(0.35) };
            ui.painter().circle_filled(rect.center(), 11.0, fill);
            if me {
                ui.painter()
                    .circle_stroke(rect.center(), 11.5, Stroke::new(1.5, p.text));
            }
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                m.initials(),
                egui::FontId::proportional(10.0),
                Color32::WHITE,
            );
            let reserved = team
                .session
                .reserved_by()
                .get(&m.slug)
                .map_or(0, |s| s.len());
            let state = if me {
                "tú".to_string()
            } else if online {
                "conectado".into()
            } else {
                team.session.presence_of(&m.slug).map_or_else(
                    || "sin conectar".into(),
                    |pr| format!("visto {}", ago(pr.time)),
                )
            };
            if resp
                .on_hover_text(format!(
                    "{} · {state} · {reserved} elementos reservados",
                    m.name
                ))
                .clicked()
            {
                open_window = true;
            }
        }
        ui.label(egui::RichText::new("Equipo").small().color(p.muted));
        if open_window {
            self.team_ui.open = true;
        }
        if publish {
            self.team_ui.open = true;
            if self.team_ui.note.trim().is_empty() {
                self.team_ui.note = "Cambios".into();
            }
            self.team_sync(true);
        } else if receive {
            self.team_sync(false);
        }
    }

    // ---- Dialog and window ------------------------------------------------

    pub(super) fn team_windows(&mut self, ctx: &egui::Context) {
        self.team_dialog(ctx);
        if self.team.is_some() && self.team_ui.open {
            let mut open = true;
            super::widgets::dialog("Equipo")
                .open(&mut open)
                .default_width(380.0)
                .default_height(520.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| self.team_contents(ui));
                });
            self.team_ui.open = open;
        }
    }

    fn team_dialog(&mut self, ctx: &egui::Context) {
        let Some(d) = self.team_ui.dialog.as_mut() else {
            return;
        };
        let mut go = None;
        let mut cancel = false;
        egui::Modal::new(egui::Id::unique("team-dialog")).show(ctx, |ui| {
            ui.set_width(420.0);
            if d.share {
                ui.heading("Compartir en equipo");
                ui.label("El proyecto se copia en una carpeta que todos veáis: una carpeta de Google Drive sincronizada en el ordenador, de Dropbox o de un NAS. Cada persona trabaja en su copia y publica sus cambios; nadie toca lo que otro ha reservado.");
            } else {
                ui.heading("Abrir proyecto de equipo");
                if !d.members.is_empty() {
                    ui.label("Ya trabajan en él:");
                    ui.horizontal_wrapped(|ui| {
                        for m in &d.members {
                            if ui
                                .add(egui::Button::new(egui::RichText::new(&m.name).color(color(m))))
                                .on_hover_text("Soy yo")
                                .clicked()
                            {
                                d.name = m.name.clone();
                            }
                        }
                    });
                }
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("Tu nombre");
                ui.add(egui::TextEdit::singleline(&mut d.name).hint_text("Nombre y apellido"));
            });
            if d.share {
                ui.horizontal(|ui| {
                    if ui.button("Elegir carpeta…").clicked() {
                        if let Some(dir) = rfd::FileDialog::new()
                            .set_title("Carpeta compartida del proyecto (vacía)")
                            .pick_folder()
                        {
                            d.folder = Some(dir);
                        }
                    }
                    ui.label(
                        d.folder
                            .as_ref()
                            .map_or_else(|| "ninguna".into(), |f| f.display().to_string()),
                    );
                });
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let ready = !d.name.trim().is_empty() && d.folder.is_some();
                let label = if d.share { "Compartir" } else { "Abrir" };
                if ui.add_enabled(ready, egui::Button::new(label)).clicked() {
                    go = Some((d.folder.clone().unwrap_or_default(), d.name.clone(), d.share));
                }
                if ui.button("Cancelar").clicked() {
                    cancel = true;
                }
            });
        });
        if cancel {
            self.team_ui.dialog = None;
        }
        if let Some((folder, name, share)) = go {
            self.team_ui.dialog = None;
            self.start_team(&folder, &name, share);
        }
    }

    fn team_contents(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        let model = self.ed.model().clone();
        let selection: Vec<u64> = self.ed.selection.iter().map(|e| raw_id(*e)).collect();
        let level = self.ed.level;
        let Some(team) = self.team.as_mut() else {
            return;
        };
        let s = &team.session;
        ui.label(egui::RichText::new(&s.team.name).font(strong_font(14.0)));
        ui.label(
            egui::RichText::new(s.root.display().to_string())
                .small()
                .color(p.muted),
        );
        let mut action: Option<Deferred> = None;

        group(ui, "team-people", "Personas", |ui| {
            let reserved = s.reserved_by();
            for m in s.members() {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
                    ui.painter().circle_filled(r.center(), 5.5, color(m));
                    let me = m.slug == s.me.slug;
                    ui.label(
                        egui::RichText::new(if me {
                            format!("{} (tú)", m.name)
                        } else {
                            m.name.clone()
                        })
                        .color(p.text),
                    );
                    let online = s.is_online(&m.slug);
                    let pr = s.presence_of(&m.slug);
                    let mut state = if me || online {
                        let lvl = pr
                            .and_then(|p| p.level)
                            .and_then(|l| model.level(archisim_core::LevelId(l)))
                            .map(|l| format!(" · {}", l.name))
                            .unwrap_or_default();
                        format!("conectado{lvl}")
                    } else {
                        pr.map_or_else(
                            || "sin conectar".into(),
                            |p| format!("visto {}", ago(p.time)),
                        )
                    };
                    let n = reserved.get(&m.slug).map_or(0, |x| x.len());
                    let z = s.zones().iter().filter(|(o, _)| o.slug == m.slug).count();
                    if n > 0 || z > 0 {
                        state.push_str(&format!(
                            " · {n} reservado{}",
                            if n == 1 { "" } else { "s" }
                        ));
                        if z > 0 {
                            state.push_str(&format!(", {z} zona{}", if z == 1 { "" } else { "s" }));
                        }
                    }
                    ui.label(egui::RichText::new(state).small().color(p.muted));
                });
            }
        });

        group(ui, "team-publish", "Publicar y recibir", |ui| {
            let local = team.local;
            let incoming = s.unreceived();
            ui.label(format!(
                "{} sin publicar · {} por recibir",
                if local == 0 {
                    "Nada".to_string()
                } else {
                    format!("{local} elemento{}", if local == 1 { "" } else { "s" })
                },
                incoming
            ));
            ui.add(
                egui::TextEdit::singleline(&mut self.team_ui.note)
                    .hint_text("Qué has cambiado (p. ej. Cocina: tabiques y puertas)"),
            );
            ui.horizontal(|ui| {
                if ui.button("Publicar").clicked() {
                    action = Some(Box::new(|app| app.team_sync(true)));
                }
                if ui.button("Recibir").clicked() {
                    action = Some(Box::new(|app| app.team_sync(false)));
                }
            });
            let lost = s.lost_claims();
            if !lost.is_empty() {
                ui.colored_label(
                    p.bad,
                    format!(
                        "{} elemento{} que reservaste a la vez que otra persona se lo queda{} quien lo reservó antes ({}).",
                        lost.len(),
                        if lost.len() == 1 { "" } else { "s" },
                        if lost.len() == 1 { "" } else { "n" },
                        lost[0].1
                    ),
                );
            }
        });

        group(ui, "team-reserve", "Reservas", |ui| {
            ui.label(egui::RichText::new("Para cambiar un elemento tiene que ser tuyo: lo reservas, o se reserva solo al cambiarlo si nadie lo tiene. Lo que dibujas nuevo es tuyo.").small().color(p.muted));
            let mine = s.reserved_by().get(&s.me.slug).map_or(0, |x| x.len());
            ui.label(format!(
                "Tienes {mine} elemento{} y {} zona{}.",
                if mine == 1 { "" } else { "s" },
                s.my_zones().len(),
                if s.my_zones().len() == 1 { "" } else { "s" }
            ));
            ui.horizontal_wrapped(|ui| {
                let some = !selection.is_empty();
                if ui
                    .add_enabled(some, egui::Button::new("Reservar selección"))
                    .clicked()
                {
                    let ids = selection.clone();
                    action = Some(Box::new(move |app| app.team_reserve(&ids)));
                }
                if ui
                    .add_enabled(some, egui::Button::new("Liberar selección"))
                    .clicked()
                {
                    let ids = selection.clone();
                    action = Some(Box::new(move |app| {
                        app.team_release(Some(&ids));
                    }));
                }
                if ui.button("Liberar todo").clicked() {
                    action = Some(Box::new(|app| {
                        app.team_release(None);
                    }));
                }
            });
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.team_ui.zone_name)
                        .hint_text("Nombre de la zona")
                        .desired_width(150.0),
                );
                if ui
                    .button("Dibujar zona")
                    .on_hover_text("Dos esquinas en la planta: todo lo que quede dentro es tuyo")
                    .clicked()
                {
                    action = Some(Box::new(|app| {
                        app.set_tool(Tool::TeamZone);
                        app.team_ui.zone_start = None;
                        app.say("Zona: clic en una esquina y en la opuesta.");
                    }));
                }
                if ui.button("Toda la planta").clicked() {
                    let name = model
                        .level(level)
                        .map_or("Planta".into(), |l| l.name.clone());
                    action = Some(Box::new(move |app| {
                        app.team_reserve_zone(&name, Vec::new())
                    }));
                }
            });
            let mut release = None;
            for (i, z) in s.my_zones().iter().enumerate() {
                ui.horizontal(|ui| {
                    let lvl = model
                        .level(archisim_core::LevelId(z.level))
                        .map_or("?".into(), |l| l.name.clone());
                    ui.label(format!(
                        "{} · {lvl}{}",
                        z.name,
                        if z.polygon.is_empty() { " entera" } else { "" }
                    ));
                    if ui.small_button("Liberar").clicked() {
                        release = Some(i);
                    }
                });
            }
            if let Some(i) = release {
                action = Some(Box::new(move |app| app.team_release_zone(i)));
            }
        });

        group(ui, "team-messages", "Mensajes", |ui| {
            let inbox = s.inbox();
            if inbox.is_empty() {
                ui.label(egui::RichText::new("Ninguno. Para pedir algo reservado por otro, selecciónalo y pulsa «Pedir» en Propiedades.").small().color(p.muted));
            }
            for (from, msg, unread) in inbox.iter().take(12) {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new(&from.name).color(color(from)).strong());
                    let t = egui::RichText::new(&msg.text);
                    ui.label(if *unread { t.strong() } else { t });
                    ui.label(egui::RichText::new(ago(msg.time)).small().color(p.muted));
                    if !msg.ids.is_empty()
                        && ui
                            .small_button("Liberar")
                            .on_hover_text("Libera lo que te pide")
                            .clicked()
                    {
                        let ids = msg.ids.clone();
                        let to = from.slug.clone();
                        action = Some(Box::new(move |app| {
                            if app.team_release(Some(&ids)) {
                                app.team_send(&to, "Liberado lo que pediste.", &[]);
                            }
                        }));
                    }
                });
            }
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("team-to")
                    .selected_text(
                        s.member(&self.team_ui.message_to)
                            .map_or("Para…".into(), |m| m.name.clone()),
                    )
                    .show_ui(ui, |ui| {
                        for m in s.members().iter().filter(|m| m.slug != s.me.slug) {
                            ui.selectable_value(
                                &mut self.team_ui.message_to,
                                m.slug.clone(),
                                &m.name,
                            );
                        }
                    });
                ui.add(egui::TextEdit::singleline(&mut self.team_ui.message).desired_width(160.0));
                if ui
                    .add_enabled(
                        !self.team_ui.message_to.is_empty()
                            && !self.team_ui.message.trim().is_empty(),
                        egui::Button::new("Enviar"),
                    )
                    .clicked()
                {
                    let to = self.team_ui.message_to.clone();
                    let text = std::mem::take(&mut self.team_ui.message);
                    action = Some(Box::new(move |app| app.team_send(&to, &text, &[])));
                }
            });
            if s.unread() > 0 && ui.small_button("Marcar como leídos").clicked() {
                action = Some(Box::new(|app| {
                    if let Some(t) = app.team.as_mut() {
                        let _ = t.session.mark_read();
                    }
                }));
            }
        });

        group(ui, "team-log", "Historial", |ui| {
            let log = s.log();
            if log.is_empty() {
                ui.label(
                    egui::RichText::new("Nadie ha publicado todavía.")
                        .small()
                        .color(p.muted),
                );
            }
            for (who, t, note, n) in log.iter().take(30) {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new(who).strong());
                    ui.label(note);
                    ui.label(
                        egui::RichText::new(format!("{n} elem. · {}", ago(*t)))
                            .small()
                            .color(p.muted),
                    );
                });
            }
        });

        group(ui, "team-options", "Opciones", |ui| {
            ui.switch(&mut team.share_cursor, "Enseñar mi cursor a los demás");
            ui.switch(&mut team.show_cursors, "Ver los cursores de los demás");
            ui.label(
                egui::RichText::new(
                    "Con Google Drive, lo de los demás llega con unos segundos de retraso.",
                )
                .small()
                .color(p.muted),
            );
        });

        if let Some(a) = action {
            a(self);
        }
    }

    // ---- Reservations -------------------------------------------------

    pub(super) fn team_reserve(&mut self, ids: &[u64]) {
        let model = self.ed.model().clone();
        let Some(team) = self.team.as_mut() else {
            return;
        };
        match team.session.reserve(ids, &ModelLocator::new(&model)) {
            Ok(refused) => {
                self.ed.team = Some(Guard::of(&team.session));
                if refused.is_empty() {
                    self.say(format!(
                        "Reservado{}: {} elemento{}.",
                        if ids.len() == 1 { "" } else { "s" },
                        ids.len(),
                        if ids.len() == 1 { "" } else { "s" }
                    ));
                } else {
                    let who = refused[0].1.clone();
                    self.complain(format!(
                        "{} de {} ya los tiene otra persona ({who}): pídeselos desde Propiedades.",
                        refused.len(),
                        ids.len()
                    ));
                }
            }
            Err(e) => self.complain(format!("No se pudo reservar: {e}.")),
        }
    }

    /// Releases some elements (all, with `None`). Not those with changes
    /// not yet published. Returns whether it released.
    pub(super) fn team_release(&mut self, ids: Option<&[u64]>) -> bool {
        let model = self.ed.model().clone();
        let Some(team) = self.team.as_mut() else {
            return false;
        };
        let pending = archisim_team::diff::touched(&team.session.local_ops(&model));
        let blocked = match ids {
            Some(ids) => ids.iter().any(|i| pending.contains(i)),
            None => !pending.is_empty(),
        };
        if blocked {
            self.complain(
                "Publica antes de liberar: tienes cambios sin publicar en lo que quieres liberar.",
            );
            return false;
        }
        let r = match ids {
            Some(ids) => team.session.release(ids),
            None => team.session.release_all(),
        };
        match r {
            Ok(()) => {
                self.ed.team = Some(Guard::of(&team.session));
                self.say("Liberado: ya lo pueden reservar los demás.");
                true
            }
            Err(e) => {
                self.complain(format!("No se pudo liberar: {e}."));
                false
            }
        }
    }

    fn team_reserve_zone(&mut self, name: &str, polygon: Vec<[f64; 2]>) {
        let level = self.ed.level.0;
        let Some(team) = self.team.as_mut() else {
            return;
        };
        let name = if name.trim().is_empty() {
            format!("Zona {}", team.session.my_zones().len() + 1)
        } else {
            name.trim().to_string()
        };
        match team.session.reserve_zone(&name, level, polygon) {
            Ok(()) => {
                self.ed.team = Some(Guard::of(&team.session));
                self.say(format!(
                    "Zona «{name}» reservada: lo que está dentro es tuyo."
                ));
            }
            Err(e) => self.complain(format!("No se puede reservar la zona: {e}.")),
        }
    }

    fn team_release_zone(&mut self, index: usize) {
        let Some(team) = self.team.as_mut() else {
            return;
        };
        match team.session.release_zone(index) {
            Ok(()) => {
                self.ed.team = Some(Guard::of(&team.session));
                self.say("Zona liberada.");
            }
            Err(e) => self.complain(format!("No se pudo liberar la zona: {e}.")),
        }
    }

    pub(super) fn team_send(&mut self, to: &str, text: &str, ids: &[u64]) {
        let Some(team) = self.team.as_mut() else {
            return;
        };
        let name = team
            .session
            .member(to)
            .map_or(to.to_string(), |m| m.name.clone());
        match team.session.send(to, text, ids) {
            Ok(()) => self.say(format!("Mensaje enviado a {name}.")),
            Err(e) => self.complain(format!("No se pudo enviar: {e}.")),
        }
    }

    /// A click of the zone tool: the first corner, then the opposite one.
    pub(super) fn team_zone_click(&mut self, p: Point2) {
        match self.team_ui.zone_start.take() {
            None => self.team_ui.zone_start = Some(p),
            Some(a) => {
                if (a.x - p.x).abs() < 0.05 || (a.y - p.y).abs() < 0.05 {
                    self.complain("La zona necesita ancho y fondo.");
                    return;
                }
                let poly = vec![[a.x, a.y], [p.x, a.y], [p.x, p.y], [a.x, p.y]];
                let name = std::mem::take(&mut self.team_ui.zone_name);
                self.team_reserve_zone(&name, poly);
                self.set_tool(Tool::Arrow);
            }
        }
    }

    // ---- Properties -----------------------------------------------------

    /// Who has the selected elements, with Reservar, Pedir or Liberar.
    pub(super) fn team_selection_strip(&mut self, ui: &mut Ui, selection: &[Element]) {
        if selection.is_empty() {
            return;
        }
        let p = Palette::current(ui.ctx());
        let model = self.ed.model().clone();
        let Some(team) = self.team.as_ref() else {
            return;
        };
        let loc = ModelLocator::new(&model);
        let ids: Vec<u64> = selection.iter().map(|e| raw_id(*e)).collect();
        let mut mine = 0;
        let mut free = 0;
        let mut theirs: std::collections::BTreeMap<String, (String, Color32, Vec<u64>)> =
            Default::default();
        for &id in &ids {
            match team.session.owner(id, &loc) {
                Some(m) if m.slug == team.session.me.slug => mine += 1,
                Some(m) => theirs
                    .entry(m.slug.clone())
                    .or_insert_with(|| (m.name.clone(), color(m), Vec::new()))
                    .2
                    .push(id),
                None => free += 1,
            }
        }
        let history = if let [one] = ids.as_slice() {
            team.session.history(*one).into_iter().next()
        } else {
            None
        };
        let mut action: Option<Deferred> = None;
        egui::Frame::new()
            .fill(p.raised)
            .inner_margin(egui::Margin::symmetric(8, 6))
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                if mine == ids.len() {
                    ui.label(egui::RichText::new("Tuyo: lo puedes cambiar").color(p.good));
                } else if free == ids.len() {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Libre: se reserva al cambiarlo").color(p.muted),
                        );
                        if ui.small_button("Reservar").clicked() {
                            let ids = ids.clone();
                            action = Some(Box::new(move |app| app.team_reserve(&ids)));
                        }
                    });
                } else if mine + free > 0 {
                    ui.label(
                        egui::RichText::new(format!("{mine} tuyos, {free} libres")).color(p.muted),
                    );
                }
                for (slug, (name, c, owned)) in &theirs {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("Reservado por {name}"))
                                .color(*c)
                                .strong(),
                        );
                        if ui
                            .small_button("Pedir")
                            .on_hover_text(format!("Pide a {name} que lo libere"))
                            .clicked()
                        {
                            let (slug, owned, n) = (slug.clone(), owned.clone(), owned.len());
                            action = Some(Box::new(move |app| {
                                let text = format!(
                                    "¿Me liberas {n} elemento{}?",
                                    if n == 1 { "" } else { "s" }
                                );
                                app.team_send(&slug, &text, &owned);
                            }));
                        }
                    });
                }
                if mine > 0 && ui.small_button("Liberar").clicked() {
                    let ids = ids.clone();
                    action = Some(Box::new(move |app| {
                        app.team_release(Some(&ids));
                    }));
                }
                if let Some((who, t, note)) = &history {
                    ui.label(
                        egui::RichText::new(format!("Último cambio: {who}, {} · {note}", ago(*t)))
                            .small()
                            .color(p.muted),
                    );
                }
            });
        ui.add_space(4.0);
        if let Some(a) = action {
            a(self);
        }
    }

    // ---- Plan -------------------------------------------------------------

    /// Reservations in their owners' colours, zones with their names, the
    /// zone being drawn and the others' cursors.
    pub(super) fn draw_team(&self, painter: &egui::Painter, rect: Rect) {
        let Some(team) = self.team.as_ref() else {
            return;
        };
        let model = self.ed.model();
        let s = &team.session;
        let level = self.ed.level;
        let by: std::collections::BTreeMap<u64, &Member> = s
            .reserved_by()
            .into_iter()
            .filter_map(|(who, ids)| s.member(&who).map(|m| (m, ids)))
            .flat_map(|(m, ids)| ids.into_iter().map(move |i| (i, m)))
            .collect();
        if !by.is_empty() {
            for e in crate::pick::elements_on_level(model, level) {
                let Some(m) = by.get(&raw_id(e)) else {
                    continue;
                };
                let Some(poly) = crate::pick::outline(&self.ed, e) else {
                    continue;
                };
                let c = color(m);
                painter.add(Shape::closed_line(
                    self.polygon(rect, &poly),
                    Stroke::new(2.5, c.gamma_multiply(0.75)),
                ));
            }
        }
        for (m, z) in s.zones() {
            if z.level != level.0 {
                continue;
            }
            let c = color(m);
            if z.polygon.len() >= 3 {
                let pts: Vec<Pos2> = z
                    .polygon
                    .iter()
                    .map(|q| self.view.to_screen(rect, Point2::new(q[0], q[1])))
                    .collect();
                let mut closed = pts.clone();
                closed.push(pts[0]);
                painter.add(Shape::convex_polygon(
                    pts.clone(),
                    c.gamma_multiply(0.06),
                    Stroke::NONE,
                ));
                painter.extend(Shape::dashed_line(&closed, Stroke::new(1.5, c), 8.0, 5.0));
                let top_left = pts.iter().fold(Pos2::new(f32::MAX, f32::MAX), |a, b| {
                    Pos2::new(a.x.min(b.x), a.y.min(b.y))
                });
                painter.text(
                    top_left + Vec2::new(4.0, 4.0),
                    egui::Align2::LEFT_TOP,
                    format!("{} · {}", z.name, m.name),
                    egui::FontId::proportional(11.0),
                    c,
                );
            } else {
                painter.text(
                    rect.left_top() + Vec2::new(10.0, 10.0),
                    egui::Align2::LEFT_TOP,
                    format!("Planta reservada por {} ({})", m.name, z.name),
                    egui::FontId::proportional(12.0),
                    c,
                );
            }
        }
        if self.tool == Tool::TeamZone {
            if let (Some(a), Some(c)) = (self.team_ui.zone_start, self.cursor) {
                let r = Rect::from_two_pos(
                    self.view.to_screen(rect, a),
                    self.view.to_screen(rect, c.point),
                );
                let me = color(&s.me);
                painter.rect_filled(r, 0.0, me.gamma_multiply(0.08));
                painter.rect_stroke(r, 0.0, Stroke::new(1.5, me), egui::StrokeKind::Inside);
            }
        }
        if team.show_cursors {
            for m in s.members() {
                if m.slug == s.me.slug || !s.is_online(&m.slug) {
                    continue;
                }
                let Some(pr) = s.presence_of(&m.slug) else {
                    continue;
                };
                let (Some(l), Some(c)) = (pr.level, pr.cursor) else {
                    continue;
                };
                if l != level.0 {
                    continue;
                }
                let at = self.view.to_screen(rect, Point2::new(c[0], c[1]));
                let col = color(m);
                painter.add(Shape::convex_polygon(
                    vec![at, at + Vec2::new(0.0, 14.0), at + Vec2::new(10.0, 10.0)],
                    col,
                    Stroke::new(1.0, Color32::WHITE),
                ));
                painter.text(
                    at + Vec2::new(12.0, 10.0),
                    egui::Align2::LEFT_TOP,
                    &m.name,
                    egui::FontId::proportional(11.0),
                    col,
                );
            }
        }
    }
}
