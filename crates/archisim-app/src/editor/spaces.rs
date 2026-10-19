//! Rooms: placing them (by clicking inside a room or by their outline) and
//! changing their number, name, use, height and stamp.

use archisim_core::{LevelId, Model, NewSpace, Point2, SpaceId};
use archisim_geom::{room_outline, Contour};

use super::{EditResult, Editor};

/// Settings of the room tool.
#[derive(Debug, Clone, PartialEq)]
pub struct SpaceSettings {
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

impl Default for SpaceSettings {
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
    if model.space_usages().is_empty() {
        USAGES.iter().map(|u| u.to_string()).collect()
    } else {
        model.space_usages().to_vec()
    }
}

impl Editor {
    /// Replaces the project's list of room uses.
    pub fn set_space_usages(&mut self, usages: Vec<String>) -> EditResult {
        self.transact("Usos de los locales", |m| m.set_space_usages(usages))
    }
}

/// The height a new room on `level` gets: the settings', or the height
/// from the level to the one above (3 m on the top level).
pub fn space_height(model: &Model, level: LevelId, s: &SpaceSettings) -> f64 {
    s.height.unwrap_or_else(|| {
        let z = model.level(level).map_or(0.0, |l| l.elevation);
        model
            .level_above(level)
            .and_then(|l| model.level(l))
            .map_or(3.0, |l| l.elevation - z)
    })
}

impl Editor {
    /// The room around `p` on the active level.
    pub fn add_space_in_room(&mut self, s: &SpaceSettings, p: Point2) -> EditResult<SpaceId> {
        let level = self.level;
        let outline = room_outline(self.model(), level, p, s.contour)
            .map_err(|e| crate::units::explain_room(&e))?;
        if self
            .model()
            .spaces_on_level(level)
            .any(|o| o.outline.len() == outline.len() && crate::pick::contains(&o.outline, p))
        {
            return Err("ese recinto ya tiene un local".into());
        }
        self.add_space(s, outline)
    }

    /// A room with the outline drawn, on the active level.
    pub fn add_space(&mut self, s: &SpaceSettings, outline: Vec<Point2>) -> EditResult<SpaceId> {
        let level = self.level;
        let number = self.model().next_space_number(level);
        let height = space_height(self.model(), level, s);
        let name = if s.name.trim().is_empty() {
            "Local".to_string()
        } else {
            s.name.trim().to_string()
        };
        self.transact("Crear local", |m| {
            m.add_space(NewSpace {
                level,
                number,
                name,
                usage: s.usage.clone(),
                outline,
                height,
            })
        })
    }

    pub fn edit_space(
        &mut self,
        label: &str,
        id: SpaceId,
        edit: impl FnOnce(&mut archisim_core::Space),
    ) -> EditResult {
        self.transact(label, |m| m.edit_space(id, edit))
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
        let s = SpaceSettings {
            name: "Estar".into(),
            usage: "Estar".into(),
            ..Default::default()
        };
        let id = ed.add_space_in_room(&s, Point2::new(3.0, 2.0)).unwrap();
        let sp = ed.model().space(id).unwrap();
        // 6 × 4 m outside, 26.5 cm walls: (6 − 0.53) × (4 − 0.53).
        assert!((sp.area() - 5.47 * 3.47).abs() < 1e-9, "{}", sp.area());
        assert_eq!(sp.number, "1");
        assert!((sp.height - 3.0).abs() < 1e-12);
        // Once only.
        assert!(ed.add_space_in_room(&s, Point2::new(2.0, 2.0)).is_err());
        assert!(ed.add_space_in_room(&s, Point2::new(-5.0, 2.0)).is_err());
        ed.edit_space("Renombrar local", id, |s| s.name = "Salón".into())
            .unwrap();
        assert_eq!(ed.model().space(id).unwrap().name, "Salón");
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
        ed.set_space_usages(v).unwrap();
        let u = usages_of(ed.model());
        assert_eq!(u.len(), USAGES.len() + 1, "trimmed, no repeats");
        assert_eq!(u.last().unwrap(), "Sala de reuniones");
        assert!(ed.set_space_usages(vec!["  ".into()]).is_err());
        ed.undo();
        assert_eq!(usages_of(ed.model()).len(), USAGES.len());
    }
}
