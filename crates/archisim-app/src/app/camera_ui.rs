//! Saved cameras and the walk through the house (Presentación,
//! `docs/INTERFAZ.md` §12.4): the Cámara tile keeps the 3D view's point of
//! view in the project (they are listed in the navigator, under
//! «Cámaras»), and Recorrido puts the eyes at a person's height to walk
//! the rooms with W A S D and look round with the right button.

use archisim_core::{CameraId, SavedCamera};
use eframe::egui::{self, Ui};

use super::tabs::TabKind;
use super::{ArchiSimApp, ViewMode};
use crate::view3d::camera::Camera;

/// The height of the eyes over the floor, metres.
pub const EYE_HEIGHT: f32 = 1.6;

/// A saved camera as the 3D view's.
pub fn to_view(c: &SavedCamera) -> Camera {
    Camera {
        target: glam::Vec3::new(c.target[0] as f32, c.target[1] as f32, c.target[2] as f32),
        yaw: c.yaw as f32,
        pitch: c.pitch as f32,
        distance: c.distance as f32,
        perspective: c.perspective,
    }
}

/// The 3D view's camera, to be saved (unnamed).
pub fn from_view(c: &Camera) -> SavedCamera {
    SavedCamera {
        id: CameraId(0),
        name: String::new(),
        target: [c.target.x as f64, c.target.y as f64, c.target.z as f64],
        pitch: c.pitch as f64,
        yaw: c.yaw as f64,
        distance: c.distance as f64,
        perspective: c.perspective,
    }
}

impl ArchiSimApp {
    /// The Cámara tile: in the 3D view, keeps its point of view; elsewhere,
    /// opens the 3D view to choose one first.
    pub(super) fn save_camera(&mut self) {
        if self.mode != ViewMode::ThreeD {
            self.open_view(TabKind::ThreeD);
            self.say("Encuadra la vista 3D y vuelve a pulsar Cámara para guardarla.");
            return;
        }
        let c = from_view(&self.v3d.camera);
        match self.ed.add_camera(c) {
            Ok(id) => {
                let name = self
                    .ed
                    .model()
                    .camera(id)
                    .map(|c| c.name.clone())
                    .unwrap_or_default();
                self.say(format!(
                    "Guardada «{name}»: está en el navegador, en Cámaras; un clic vuelve a ella."
                ));
            }
            Err(e) => self.complain(e),
        }
    }

    /// Opens the 3D view looking from a saved camera.
    pub(super) fn go_to_camera(&mut self, id: CameraId) {
        let Some(c) = self.ed.model().camera(id).cloned() else {
            return;
        };
        if self.mode != ViewMode::ThreeD {
            self.open_view(TabKind::ThreeD);
        }
        self.v3d.walk_eye = None;
        self.v3d.camera = to_view(&c);
    }

    /// Recorrido: the eyes at a person's height over the active level,
    /// where the view looks, looking level; again (or Esc), back to the
    /// view as it was before.
    pub(super) fn toggle_walk(&mut self) {
        if self.v3d.walk_eye.take().is_some() {
            if let Some(c) = self.walk_return.take() {
                self.v3d.camera = c;
            }
            self.say("Fuera del recorrido.");
            return;
        }
        if self.mode != ViewMode::ThreeD {
            self.open_view(TabKind::ThreeD);
        }
        let floor = self
            .ed
            .model()
            .level(self.ed.level)
            .map_or(0.0, |l| l.elevation) as f32;
        let eye_z = floor + EYE_HEIGHT;
        self.walk_return = Some(self.v3d.camera);
        let c = &mut self.v3d.camera;
        // Stand where the view looked, facing the way it faced.
        let at = glam::Vec3::new(c.target.x, c.target.y, eye_z);
        let ahead = glam::Vec3::new(-c.yaw.cos(), -c.yaw.sin(), 0.0);
        c.perspective = true;
        c.pitch = 0.0;
        c.distance = 0.5;
        c.target = at + ahead * 0.5;
        self.v3d.walk_eye = Some(eye_z);
        self.say("Recorrido: W A S D andan, Mayús corre, el botón derecho mira alrededor; Cámara guarda el punto de vista; Esc sale.");
    }

    /// The cameras of the project in the navigator: a click looks from
    /// one; its menu renames it, moves it to the view or deletes it.
    pub(super) fn cameras_rows(&mut self, ui: &mut Ui) {
        let cameras: Vec<(CameraId, String)> = self
            .ed
            .model()
            .cameras()
            .map(|c| (c.id, c.name.clone()))
            .collect();
        if cameras.is_empty() {
            return;
        }
        let mut go = None;
        let mut rename = None;
        let mut update = None;
        let mut delete = None;
        for (id, name) in &cameras {
            let r = super::widgets::list_row_draggable(
                ui,
                Some(super::icons::Icon::ThreeD),
                None,
                name,
                "",
                false,
                22.0,
            )
            .on_hover_text(
                "Mirar desde esta cámara; clic derecho para renombrarla, actualizarla o borrarla",
            );
            if r.clicked() {
                go = Some(*id);
            }
            r.context_menu(|ui| {
                let edit_id = egui::Id::unique(("rename-camera", id.0));
                let mut text: String = ui
                    .ctx()
                    .data(|d| d.get_temp(edit_id))
                    .unwrap_or_else(|| name.clone());
                ui.label("Nombre:");
                let resp = ui.text_edit_singleline(&mut text);
                ui.ctx().data_mut(|d| d.insert_temp(edit_id, text.clone()));
                if (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    || ui.button("Renombrar").clicked()
                {
                    rename = Some((*id, text.clone()));
                    ui.ctx().data_mut(|d| d.remove_temp::<String>(edit_id));
                    ui.close();
                }
                ui.separator();
                if ui
                    .button("Actualizar con la vista")
                    .on_hover_text("La cámara pasa a mirar como la vista 3D ahora")
                    .clicked()
                {
                    update = Some(*id);
                    ui.close();
                }
                if ui.button("Borrar cámara").clicked() {
                    delete = Some(*id);
                    ui.close();
                }
            });
        }
        if let Some(id) = go {
            self.go_to_camera(id);
        }
        if let Some((id, name)) = rename {
            let r = self.ed.rename_camera(id, &name);
            self.report(r, "Cámara renombrada.");
        }
        if let Some(id) = update {
            let c = from_view(&self.v3d.camera);
            let r = self.ed.update_camera(id, c);
            self.report(r, "Cámara actualizada con la vista.");
        }
        if let Some(id) = delete {
            let r = self.ed.remove_camera(id);
            self.report(r, "Cámara borrada.");
        }
    }
}
