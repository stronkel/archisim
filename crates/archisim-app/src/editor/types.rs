//! Editing the project's materials and types (ArchiCAD "Materiales de
//! construcción" and "Multicapas", Revit "Edit Type"): create by
//! duplicating, change and delete. Every change is one undoable step, and
//! changing a type changes every element of that type.
//!
//! Names are unique within each list (materials, wall types, slab types,
//! door and window types), as both programs require: IFC files and the
//! import match materials by name, and the tool menus show types by name.

use archisim_core::{
    CutPattern, Layer, MaterialId, Model, NewOpeningType, OpeningTypeId, SlabFunction, SlabTypeId,
    WallFunction, WallTypeId,
};

use super::{EditResult, Editor};

/// Which list a name must be unique in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Names {
    Material,
    WallType,
    SlabType,
    OpeningType,
}

fn names(m: &Model, list: Names) -> Vec<String> {
    match list {
        Names::Material => m.materials().map(|x| x.name.clone()).collect(),
        Names::WallType => m.wall_types().map(|x| x.name.clone()).collect(),
        Names::SlabType => m.slab_types().map(|x| x.name.clone()).collect(),
        Names::OpeningType => m.opening_types().map(|x| x.name.clone()).collect(),
    }
}

/// `«name» (copia)`, `«name» (copia 2)`… the first one not in `taken`.
pub fn copy_name(name: &str, taken: &[String]) -> String {
    // Copying a copy numbers it instead of adding "(copia) (copia)".
    let base = match name.rfind(" (copia") {
        Some(i) if name.ends_with(')') => &name[..i],
        _ => name,
    };
    (1..)
        .map(|n| match n {
            1 => format!("{base} (copia)"),
            n => format!("{base} (copia {n})"),
        })
        .find(|c| !taken.iter().any(|t| t == c))
        .unwrap_or_else(|| format!("{base} (copia)"))
}

/// The trimmed name, if it is not empty and no other item of the list (the
/// one being renamed has `current`) already has it.
fn checked_name(m: &Model, list: Names, name: &str, current: Option<&str>) -> EditResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("el nombre no puede quedar vacío".into());
    }
    if Some(name) != current && names(m, list).iter().any(|n| n == name) {
        let what = match list {
            Names::Material => "un material",
            Names::WallType => "un tipo de muro",
            Names::SlabType => "un tipo de forjado",
            Names::OpeningType => "un tipo de puerta o ventana",
        };
        return Err(format!("ya hay {what} llamado «{name}»"));
    }
    Ok(name.to_string())
}

/// «N elementos» with the right plural.
fn count(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

impl Editor {
    // ---- Materials ----------------------------------------------------------

    /// A copy of a material, named «… (copia)».
    pub fn duplicate_material(&mut self, id: MaterialId) -> EditResult<MaterialId> {
        let m = self.model();
        let src = m.material(id).ok_or("el material no existe")?.clone();
        let name = copy_name(&src.name, &names(m, Names::Material));
        self.transact("Duplicar material", |m| {
            Ok(m.add_material(name, src.category, src.cut_color, src.cut_pattern))
        })
    }

    /// Changes a material: every layer, frame and leaf made of it follows.
    pub fn edit_material(
        &mut self,
        id: MaterialId,
        name: &str,
        category: &str,
        cut_color: [u8; 3],
        cut_pattern: CutPattern,
    ) -> EditResult {
        let m = self.model();
        let current = m.material(id).ok_or("el material no existe")?;
        let name = checked_name(m, Names::Material, name, Some(&current.name))?;
        let category = category.trim().to_string();
        self.transact("Modificar material", |m| {
            m.set_material(id, name, category, cut_color, cut_pattern)
        })
    }

    /// Deletes a material that no type uses; otherwise says which types do.
    pub fn remove_material(&mut self, id: MaterialId) -> EditResult {
        let users = self.model().material_users(id);
        if !users.is_empty() {
            return Err(format!(
                "lo usa{} {}: {}",
                if users.len() == 1 { "" } else { "n" },
                count(users.len(), "tipo", "tipos"),
                quoted_list(&users)
            ));
        }
        self.transact("Borrar material", |m| m.remove_material(id).map(|_| ()))
    }

    // ---- Wall types -----------------------------------------------------------

    /// A copy of a wall type with a new GlobalId (Revit "Duplicate").
    pub fn duplicate_wall_type(&mut self, id: WallTypeId) -> EditResult<WallTypeId> {
        let m = self.model();
        let src = m.wall_type(id).ok_or("el tipo de muro no existe")?.clone();
        let name = copy_name(&src.name, &names(m, Names::WallType));
        self.transact("Duplicar tipo de muro", |m| {
            m.add_wall_type(name, src.function, src.layers)
        })
    }

    /// Redefines a wall type; every wall of the type changes with it.
    pub fn edit_wall_type(
        &mut self,
        id: WallTypeId,
        name: &str,
        function: WallFunction,
        layers: Vec<Layer>,
    ) -> EditResult {
        let m = self.model();
        let current = m.wall_type(id).ok_or("el tipo de muro no existe")?;
        let name = checked_name(m, Names::WallType, name, Some(&current.name))?;
        self.transact("Modificar tipo de muro", |m| {
            m.edit_wall_type(id, name, function, layers)
        })
    }

    /// Deletes a wall type no wall uses. The wall tool falls back to
    /// another type if it was using it.
    pub fn remove_wall_type(&mut self, id: WallTypeId) -> EditResult {
        let n = self.model().wall_type_users(id);
        if n > 0 {
            return Err(format!(
                "lo usa{} {}",
                plural_n(n),
                count(n, "muro", "muros")
            ));
        }
        if self.model().wall_types().count() == 1 {
            return Err("el proyecto necesita al menos un tipo de muro".into());
        }
        self.transact("Borrar tipo de muro", |m| {
            m.remove_wall_type(id).map(|_| ())
        })
    }

    // ---- Slab types -----------------------------------------------------------

    pub fn duplicate_slab_type(&mut self, id: SlabTypeId) -> EditResult<SlabTypeId> {
        let m = self.model();
        let src = m
            .slab_type(id)
            .ok_or("el tipo de forjado no existe")?
            .clone();
        let name = copy_name(&src.name, &names(m, Names::SlabType));
        self.transact("Duplicar tipo de forjado", |m| {
            m.add_slab_type(name, src.function, src.layers)
        })
    }

    /// Redefines a slab type; every slab of the type changes with it.
    pub fn edit_slab_type(
        &mut self,
        id: SlabTypeId,
        name: &str,
        function: SlabFunction,
        layers: Vec<Layer>,
    ) -> EditResult {
        let m = self.model();
        let current = m.slab_type(id).ok_or("el tipo de forjado no existe")?;
        let name = checked_name(m, Names::SlabType, name, Some(&current.name))?;
        self.transact("Modificar tipo de forjado", |m| {
            m.edit_slab_type(id, name, function, layers)
        })
    }

    pub fn remove_slab_type(&mut self, id: SlabTypeId) -> EditResult {
        let n = self.model().slab_type_users(id);
        if n > 0 {
            return Err(format!(
                "lo usa{} {}",
                plural_n(n),
                count(n, "forjado", "forjados")
            ));
        }
        if self.model().slab_types().count() == 1 {
            return Err("el proyecto necesita al menos un tipo de forjado".into());
        }
        self.transact("Borrar tipo de forjado", |m| {
            m.remove_slab_type(id).map(|_| ())
        })
    }

    // ---- Door and window types ------------------------------------------------

    pub fn duplicate_opening_type(&mut self, id: OpeningTypeId) -> EditResult<OpeningTypeId> {
        let m = self.model();
        let src = m
            .opening_type(id)
            .ok_or("el tipo de puerta o ventana no existe")?
            .clone();
        let name = copy_name(&src.name, &names(m, Names::OpeningType));
        let label = if src.is_door() {
            "Duplicar tipo de puerta"
        } else {
            "Duplicar tipo de ventana"
        };
        self.transact(label, |m| m.duplicate_opening_type(id, name))
    }

    /// Changes the type mark ("P-03") and description of a door or window
    /// type. Marks are unique; an empty mark is allowed.
    pub fn set_opening_type_info(
        &mut self,
        id: OpeningTypeId,
        mark: &str,
        description: &str,
    ) -> EditResult {
        let current = self
            .model()
            .opening_type(id)
            .ok_or("el tipo de puerta o ventana no existe")?;
        let (name, door) = (current.name.clone(), current.is_door());
        let label = if door {
            "Modificar tipo de puerta"
        } else {
            "Modificar tipo de ventana"
        };
        let (mark, description) = (mark.trim().to_string(), description.trim().to_string());
        self.transact(label, |m| {
            m.set_opening_type_info(id, name, mark, description)
        })
    }

    /// Redefines a door or window type; every one of the type changes with
    /// it, keeping its centre and sill.
    pub fn edit_opening_type(&mut self, id: OpeningTypeId, mut spec: NewOpeningType) -> EditResult {
        let m = self.model();
        let current = m
            .opening_type(id)
            .ok_or("el tipo de puerta o ventana no existe")?;
        spec.name = checked_name(m, Names::OpeningType, &spec.name, Some(&current.name))?;
        let label = if current.is_door() {
            "Modificar tipo de puerta"
        } else {
            "Modificar tipo de ventana"
        };
        self.transact(label, |m| m.edit_opening_type(id, spec))
    }

    pub fn remove_opening_type(&mut self, id: OpeningTypeId) -> EditResult {
        let m = self.model();
        let door = m
            .opening_type(id)
            .ok_or("el tipo de puerta o ventana no existe")?
            .is_door();
        let n = m.opening_type_users(id);
        if n > 0 {
            let (one, many) = if door {
                ("puerta", "puertas")
            } else {
                ("ventana", "ventanas")
            };
            return Err(format!("lo usa{} {}", plural_n(n), count(n, one, many)));
        }
        if m.opening_types().filter(|t| t.is_door() == door).count() == 1 {
            return Err(if door {
                "el proyecto necesita al menos un tipo de puerta".into()
            } else {
                "el proyecto necesita al menos un tipo de ventana".into()
            });
        }
        let label = if door {
            "Borrar tipo de puerta"
        } else {
            "Borrar tipo de ventana"
        };
        self.transact(label, |m| m.remove_opening_type(id).map(|_| ()))
    }
}

fn plural_n(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "n"
    }
}

/// «a», «b» y «c» (at most five, then «y N más»).
fn quoted_list(items: &[String]) -> String {
    let shown: Vec<String> = items.iter().take(5).map(|s| format!("«{s}»")).collect();
    let rest = items.len().saturating_sub(5);
    let mut text = match shown.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] if rest == 0 => format!("{} y {last}", init.join(", ")),
        all => all.join(", "),
    };
    if rest > 0 {
        text.push_str(&format!(" y {rest} más"));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{LayerFunction, NewWall, Point2, ReferenceLine};

    #[test]
    fn copies_get_free_names() {
        let taken = vec!["Tabique".to_string(), "Tabique (copia)".to_string()];
        assert_eq!(copy_name("Fachada", &taken), "Fachada (copia)");
        assert_eq!(copy_name("Tabique", &taken), "Tabique (copia 2)");
        assert_eq!(copy_name("Tabique (copia)", &taken), "Tabique (copia 2)");
        assert_eq!(
            quoted_list(&["a".into(), "b".into(), "c".into()]),
            "«a», «b» y «c»"
        );
        let many: Vec<String> = (1..=7).map(|i| i.to_string()).collect();
        assert_eq!(quoted_list(&many), "«1», «2», «3», «4», «5» y 2 más");
    }

    #[test]
    fn door_and_window_types_have_marks_and_a_family() {
        let mut ed = Editor::new_project();
        let door = ed
            .model()
            .opening_types()
            .find(|t| t.is_door())
            .unwrap()
            .clone();
        assert_eq!(door.mark, "P-01");
        assert_eq!(
            ed.model().family(door.family).unwrap().name(),
            "Puerta abatible"
        );
        let copy = ed.duplicate_opening_type(door.id).unwrap();
        let c = ed.model().opening_type(copy).unwrap().clone();
        assert_eq!(c.family, door.family);
        assert_eq!(c.values, door.values);
        // The template has four door types: the copy takes the next mark.
        assert_eq!(c.mark, "P-05");

        ed.set_opening_type_info(copy, " P-10 ", "Baños").unwrap();
        assert_eq!(ed.undo_label(), Some("Modificar tipo de puerta"));
        let c = ed.model().opening_type(copy).unwrap();
        assert_eq!((c.mark.as_str(), c.description.as_str()), ("P-10", "Baños"));
        let err = ed.set_opening_type_info(door.id, "P-10", "").unwrap_err();
        assert_eq!(err, "la marca «P-10» ya la tiene otro tipo");
        ed.undo();
        assert_eq!(ed.model().opening_type(copy).unwrap().mark, "P-05");

        // The family's own rule, in its own words: two jambs and the
        // mullion of 620 mm do not fit in 1800 mm.
        let window = ed
            .model()
            .opening_types()
            .find(|t| t.name.starts_with("Ventana practicable 2 hojas"))
            .unwrap()
            .clone();
        let mut spec = NewOpeningType {
            name: window.name.clone(),
            kind: window.kind,
            width: window.width,
            height: window.height,
            frame_width: 0.62,
            frame_depth: window.frame_depth,
            frame_material: window.frame_material,
            panel_material: window.panel_material,
        };
        let err = ed.edit_opening_type(window.id, spec.clone()).unwrap_err();
        assert_eq!(err, "El marco y el montante no dejan hueco para el vidrio");
        spec.frame_width = 0.06;
        ed.edit_opening_type(window.id, spec).unwrap();
    }

    #[test]
    fn duplicate_edit_and_delete_are_single_steps() {
        let mut ed = Editor::new_project();
        let lib_wall = ed.model().wall_types().next().unwrap().clone();
        let copy = ed.duplicate_wall_type(lib_wall.id).unwrap();
        assert_eq!(ed.undo_label(), Some("Duplicar tipo de muro"));
        let t = ed.model().wall_type(copy).unwrap().clone();
        assert_eq!(t.name, format!("{} (copia)", lib_wall.name));
        assert_eq!(t.layers, lib_wall.layers);
        assert_ne!(t.guid, lib_wall.guid, "a duplicate is a new IFC type");

        // Rename to an existing name or to nothing: refused.
        let err = ed
            .edit_wall_type(copy, &lib_wall.name, t.function, t.layers.clone())
            .unwrap_err();
        assert!(err.contains("ya hay un tipo de muro"), "{err}");
        assert!(ed
            .edit_wall_type(copy, "  ", t.function, t.layers.clone())
            .is_err());
        // Thicker structure: one step, then undone.
        let mut layers = t.layers.clone();
        layers[1].thickness = 0.24;
        ed.edit_wall_type(copy, " Fachada de 1 pie ", t.function, layers)
            .unwrap();
        assert_eq!(ed.model().wall_type(copy).unwrap().name, "Fachada de 1 pie");
        assert_eq!(ed.undo_label(), Some("Modificar tipo de muro"));
        ed.undo();
        assert_eq!(ed.model().wall_type(copy).unwrap(), &t);

        // In use: refused with the count; unused: deleted.
        let level = ed.level;
        ed.transact("Crear muro", |m| {
            m.add_wall(NewWall {
                wall_type: copy,
                level,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(3.0, 0.0),
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
            })
        })
        .unwrap();
        assert_eq!(ed.remove_wall_type(copy).unwrap_err(), "lo usa 1 muro");
        ed.undo();
        ed.remove_wall_type(copy).unwrap();
        assert!(ed.model().wall_type(copy).is_none());
    }

    #[test]
    fn materials_in_use_say_where() {
        let mut ed = Editor::new_project();
        let yeso = ed
            .model()
            .materials()
            .find(|m| m.name == "Enlucido de yeso")
            .unwrap()
            .id;
        let err = ed.remove_material(yeso).unwrap_err();
        assert!(err.starts_with("lo usan "), "{err}");
        assert!(err.contains("«Fachada doble hoja 26,5»"), "{err}");
        let copy = ed.duplicate_material(yeso).unwrap();
        assert_eq!(
            ed.model().material(copy).unwrap().name,
            "Enlucido de yeso (copia)"
        );
        ed.edit_material(
            copy,
            "Yeso proyectado",
            "plaster",
            [250, 250, 240],
            CutPattern::Dots,
        )
        .unwrap();
        assert!(ed
            .edit_material(
                copy,
                "Enlucido de yeso",
                "plaster",
                [0; 3],
                CutPattern::Solid
            )
            .is_err());
        // Keeping its own name is fine.
        ed.edit_material(
            copy,
            "Yeso proyectado",
            "plaster",
            [0; 3],
            CutPattern::Solid,
        )
        .unwrap();
        ed.remove_material(copy).unwrap();
        assert_eq!(ed.undo_label(), Some("Borrar material"));
    }

    #[test]
    fn the_last_type_of_each_kind_stays() {
        let mut ed = Editor::new_project();
        let walls: Vec<_> = ed.model().wall_types().map(|t| t.id).collect();
        for w in &walls[1..] {
            ed.remove_wall_type(*w).unwrap();
        }
        assert!(ed.remove_wall_type(walls[0]).is_err());
        let doors: Vec<_> = ed
            .model()
            .opening_types()
            .filter(|t| t.is_door())
            .map(|t| t.id)
            .collect();
        for d in &doors[1..] {
            ed.remove_opening_type(*d).unwrap();
        }
        assert_eq!(
            ed.remove_opening_type(doors[0]).unwrap_err(),
            "el proyecto necesita al menos un tipo de puerta"
        );
        // A slab type with a new structural layer.
        let st = ed.model().slab_types().next().unwrap().clone();
        let mut layers = st.layers.clone();
        layers.push(Layer {
            material: layers[0].material,
            thickness: 0.01,
            function: LayerFunction::Finish,
        });
        ed.edit_slab_type(st.id, &st.name, st.function, layers)
            .unwrap();
        assert_eq!(
            ed.model().slab_type(st.id).unwrap().layers.len(),
            st.layers.len() + 1
        );
    }
}
