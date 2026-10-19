//! The room tool: placing rooms (by clicking inside a room of walls or by
//! their outline) with a name, a use and a height chosen beforehand, and
//! the project's list of room uses.

use archisim_core::{label_center, Model, NewRoom, Point2, RoomId};
use archisim_geom::{room_outline, Contour};

use super::{EditResult, Editor};

/// Settings of the room tool.
#[derive(Debug, Clone, PartialEq)]
pub struct RoomSettings {
    pub name: String,
    pub usage: String,
    /// Clear height; `None`: from the level to the one above.
    pub height: Option<f64>,
    /// Which outline a click inside a room gives (the usable area is to
    /// the interior faces).
    pub contour: Contour,
    /// A click inside a room makes it (otherwise the outline is drawn).
    pub wand: bool,
}

impl Default for RoomSettings {
    fn default() -> Self {
        Self {
            name: "Local".into(),
            usage: String::new(),
            height: None,
            contour: Contour::InteriorFace,
            wand: true,
        }
    }
}

/// Uses of rooms offered in the lists (free text is allowed too).
pub const USAGES: &[&str] = &[
    "Estar",
    "Comedor",
    "Estar-comedor",
    "Cocina",
    "Dormitorio",
    "Baño",
    "Aseo",
    "Vestíbulo",
    "Pasillo",
    "Distribuidor",
    "Escalera",
    "Lavadero",
    "Despensa",
    "Almacén",
    "Garaje",
    "Terraza",
    "Porche",
    "Patio",
    "Instalaciones",
    "Oficina",
];

/// Sets of uses for other kinds of building, to add to a project's list:
/// (name, uses).
pub const USAGE_SETS: &[(&str, &[&str])] = &[
    ("Vivienda", USAGES),
    (
        "Oficinas",
        &[
            "Despacho",
            "Oficina abierta",
            "Sala de reuniones",
            "Recepción",
            "Archivo",
            "Office",
            "Aseo",
            "Almacén",
            "Instalaciones",
            "Vestíbulo",
            "Pasillo",
        ],
    ),
    (
        "Docente",
        &[
            "Aula",
            "Laboratorio",
            "Biblioteca",
            "Sala de profesores",
            "Despacho",
            "Gimnasio",
            "Comedor",
            "Aseo",
            "Vestuario",
            "Almacén",
            "Vestíbulo",
            "Pasillo",
        ],
    ),
    (
        "Sanitario",
        &[
            "Consulta",
            "Sala de espera",
            "Sala de curas",
            "Habitación",
            "Control de enfermería",
            "Almacén",
            "Aseo",
            "Vestuario",
            "Instalaciones",
        ],
    ),
    (
        "Comercial",
        &[
            "Sala de ventas",
            "Probador",
            "Caja",
            "Almacén",
            "Oficina",
            "Aseo",
            "Muelle de carga",
        ],
    ),
    (
        "Hostelería",
        &[
            "Comedor",
            "Barra",
            "Cocina industrial",
            "Cámara frigorífica",
            "Habitación",
            "Recepción",
            "Aseo",
            "Almacén",
        ],
    ),
    (
        "Industrial",
        &[
            "Nave",
            "Taller",
            "Almacén",
            "Muelle de carga",
            "Oficina",
            "Vestuario",
            "Aseo",
        ],
    ),
];

/// The uses offered for rooms: the project's list, or the default one.
pub fn usages_of(model: &Model) -> Vec<String> {
    if model.room_usages().is_empty() {
        USAGES.iter().map(|u| u.to_string()).collect()
    } else {
        model.room_usages().to_vec()
    }
}

impl Editor {
    /// Replaces the project's list of room uses.
    pub fn set_room_usages(&mut self, usages: Vec<String>) -> EditResult {
        self.transact("Usos de los locales", |m| m.set_room_usages(usages))
    }

    /// The room around `p` on the active level: it follows its walls, and
    /// its name goes where it was clicked.
    pub fn add_room_in_room(&mut self, s: &RoomSettings, p: Point2) -> EditResult<RoomId> {
        let level = self.level;
        let outline = room_outline(self.model(), level, p, s.contour)
            .map_err(|e| crate::units::explain_room(&e))?;
        if self.model().rooms_on_level(level).any(|o| o.contains(p)) {
            return Err("ese recinto ya tiene un local".into());
        }
        self.new_room(s, outline, p, true)
    }

    /// A room with the outline drawn, on the active level, its name in the
    /// middle.
    pub fn add_room_outline(
        &mut self,
        s: &RoomSettings,
        outline: Vec<Point2>,
    ) -> EditResult<RoomId> {
        let label = label_center(&outline);
        self.new_room(s, outline, label, false)
    }

    fn new_room(
        &mut self,
        s: &RoomSettings,
        outline: Vec<Point2>,
        label: Point2,
        bounded: bool,
    ) -> EditResult<RoomId> {
        let level = self.level;
        let number = self.model().next_room_number(level);
        let name = if s.name.trim().is_empty() {
            "Local".to_string()
        } else {
            s.name.trim().to_string()
        };
        let (usage, height) = (s.usage.clone(), s.height);
        self.transact("Crear local", |m| {
            let id = m.add_room(NewRoom {
                level,
                number,
                name,
                outline,
                holes: Vec::new(),
                label,
                bounded,
            })?;
            let mut r = m
                .room(id)
                .cloned()
                .ok_or(archisim_core::ModelError::UnknownRoom(id))?;
            r.usage = usage;
            r.height = height;
            m.set_room(r)?;
            Ok(id)
        })
    }

    /// Changes a room.
    pub fn edit_room(
        &mut self,
        label: &str,
        id: RoomId,
        edit: impl FnOnce(&mut archisim_core::Room),
    ) -> EditResult {
        let Some(mut r) = self.model().room(id).cloned() else {
            return Err("el local ya no existe".into());
        };
        edit(&mut r);
        self.set_room(r, label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pick::tests::room;

    #[test]
    fn rooms_by_clicking_inside() {
        let (m, _, _) = room();
        let mut ed = Editor::with_model(m, None);
        let s = RoomSettings {
            name: "Estar".into(),
            usage: "Estar".into(),
            ..Default::default()
        };
        let id = ed.add_room_in_room(&s, Point2::new(3.0, 2.0)).unwrap();
        let r = ed.model().room(id).unwrap();
        // 6 × 4 m outside, 26.5 cm walls: (6 − 0.53) × (4 − 0.53).
        assert!((r.area() - 5.47 * 3.47).abs() < 1e-9, "{}", r.area());
        assert_eq!(r.number, "1");
        assert_eq!(r.usage, "Estar");
        assert!(r.bounded);
        // No level above: the default clear height.
        assert!((ed.model().room_height(r) - archisim_core::DEFAULT_ROOM_HEIGHT).abs() < 1e-12);
        // Once only.
        assert!(ed.add_room_in_room(&s, Point2::new(2.0, 2.0)).is_err());
        assert!(ed.add_room_in_room(&s, Point2::new(-5.0, 2.0)).is_err());
        ed.edit_room("Renombrar local", id, |r| r.name = "Salón".into())
            .unwrap();
        assert_eq!(ed.model().room(id).unwrap().name, "Salón");
        // Drawn by hand: number 2, its name in the middle.
        let id = ed
            .add_room_outline(
                &s,
                vec![
                    Point2::new(1.0, 1.0),
                    Point2::new(2.0, 1.0),
                    Point2::new(2.0, 2.0),
                    Point2::new(1.0, 2.0),
                ],
            )
            .unwrap();
        let r = ed.model().room(id).unwrap();
        assert_eq!(r.number, "2");
        assert!(r.label.distance(Point2::new(1.5, 1.5)) < 1e-9);
        assert!(!r.bounded);
    }
}

#[cfg(test)]
mod usage_tests {
    use super::*;

    #[test]
    fn projects_have_their_own_uses() {
        let mut ed = Editor::new_project();
        assert_eq!(usages_of(ed.model()).len(), USAGES.len());
        let mut v = usages_of(ed.model());
        v.push(" Sala de reuniones ".into());
        v.push("Estar".into());
        ed.set_room_usages(v).unwrap();
        let u = usages_of(ed.model());
        assert_eq!(u.len(), USAGES.len() + 1, "trimmed, no repeats");
        assert_eq!(u.last().unwrap(), "Sala de reuniones");
        assert!(ed.set_room_usages(vec!["  ".into()]).is_err());
        ed.undo();
        assert_eq!(usages_of(ed.model()).len(), USAGES.len());
    }
}
