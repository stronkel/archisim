//! 3D models from other programs (SketchUp, Blender, a manufacturer's
//! catalogue) turned into object types of the project: the model with its
//! lighter versions, a material per material of the model, and a family
//! written for it whose plan symbol is the model's outline seen from
//! above, scaled with the type's height, as for any other object.

use archisim_core::{
    CutPattern, MaterialId, MeshGroup, MeshLevel, MeshModel, Model, ObjectTypeId, Values,
};
use archisim_mesh::Mesh;

/// What kind of object a model is: its category, IFC class, folder of
/// the library and where it is inserted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelKind {
    Furniture,
    Sanitary,
    Kitchen,
    Vegetation,
    Street,
    People,
    Vehicle,
}

impl ModelKind {
    pub const ALL: [ModelKind; 7] = [
        ModelKind::Furniture,
        ModelKind::Kitchen,
        ModelKind::Sanitary,
        ModelKind::Vegetation,
        ModelKind::Street,
        ModelKind::People,
        ModelKind::Vehicle,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ModelKind::Furniture => "Mobiliario",
            ModelKind::Sanitary => "Sanitarios",
            ModelKind::Kitchen => "Cocina",
            ModelKind::Vegetation => "Vegetación",
            ModelKind::Street => "Mobiliario urbano",
            ModelKind::People => "Personas",
            ModelKind::Vehicle => "Vehículos",
        }
    }

    fn category(self) -> &'static str {
        match self {
            ModelKind::Furniture => "furniture",
            ModelKind::Sanitary => "sanitary",
            ModelKind::Kitchen => "kitchen",
            ModelKind::Vegetation => "site",
            ModelKind::Street => "street",
            ModelKind::People => "people",
            ModelKind::Vehicle => "vehicle",
        }
    }

    fn ifc(self) -> (&'static str, &'static str) {
        match self {
            ModelKind::Sanitary => ("IfcSanitaryTerminal", "USERDEFINED"),
            ModelKind::Vegetation => ("IfcGeographicElement", "VEGETATION"),
            ModelKind::People => ("IfcBuildingElementProxy", "USERDEFINED"),
            ModelKind::Vehicle => ("IfcVehicle", "VEHICLEWHEELED"),
            _ => ("IfcFurniture", "USERDEFINED"),
        }
    }

    /// Inserted by the middle of its base (trees, people, lamp posts) or,
    /// like the furniture of the library, by the middle of its back with
    /// its front towards +y.
    fn centred(self) -> bool {
        matches!(
            self,
            ModelKind::Vegetation | ModelKind::Street | ModelKind::People
        )
    }

    /// The kind a library folder suggests (by its name).
    pub fn from_folder(name: &str) -> ModelKind {
        let n = name.to_lowercase();
        let has = |w: &[&str]| w.iter().any(|x| n.contains(x));
        if has(&["vegeta", "árbol", "arbol", "planta", "jard", "entorno"]) {
            ModelKind::Vegetation
        } else if has(&["urbano", "calle", "farola", "street"]) {
            ModelKind::Street
        } else if has(&["gente", "persona", "people"]) {
            ModelKind::People
        } else if has(&["vehí", "vehi", "coche", "car"]) {
            ModelKind::Vehicle
        } else if has(&["sanit", "baño", "bano"]) {
            ModelKind::Sanitary
        } else if has(&["cocina", "kitchen"]) {
            ModelKind::Kitchen
        } else {
            ModelKind::Furniture
        }
    }
}

/// What was brought in, for the report.
#[derive(Debug, Clone, PartialEq)]
pub struct Imported {
    pub object_type: ObjectTypeId,
    /// Width, depth and height in metres.
    pub size: [f64; 3],
    /// Triangles of each version, the model's first.
    pub triangles: Vec<usize>,
    pub materials: usize,
}

fn fmt(x: f64) -> String {
    let s = format!("{x:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".into()
    } else {
        s.to_string()
    }
}

fn quote(s: &str) -> String {
    s.replace('"', "'")
}

/// A name not yet taken by a family (or a material) of the project.
fn free_name(taken: impl Fn(&str) -> bool, base: &str) -> String {
    if !taken(base) {
        return base.to_string();
    }
    (2..)
        .map(|i| format!("{base} ({i})"))
        .find(|n| !taken(n))
        .expect("some number is free")
}

/// Brings `mesh` into `model` as a new object type called `name`, of
/// `kind`, read from the file `source`. The model is moved so that it is
/// inserted where `kind` says, and keeps its size (its type's height scales
/// it).
pub fn import_mesh(
    model: &mut Model,
    mut mesh: Mesh,
    name: &str,
    source: &str,
    kind: ModelKind,
) -> Result<Imported, String> {
    let name = name.trim();
    let name = if name.is_empty() { "Modelo 3D" } else { name };
    mesh.centre_on_base();
    if !kind.centred() {
        // Its back on the insertion point and its front towards +y: the
        // model's front looks to −y (glTF's and SketchUp's).
        for p in &mut mesh.positions {
            p[0] = -p[0];
            p[1] = -p[1];
        }
        let (lo, _) = mesh.bounds();
        for p in &mut mesh.positions {
            p[1] -= lo[1];
        }
    }
    let (lo, hi) = mesh.bounds();
    let size = [0, 1, 2].map(|k| (hi[k] - lo[k]) as f64);
    if size[2] < 1e-3 {
        return Err("el modelo es plano: no tiene altura".into());
    }
    if size.iter().any(|s| *s > 500.0) {
        return Err(format!(
            "el modelo mide {:.0} m: compruebe sus unidades (¿milímetros, centímetros o pulgadas?)",
            size.iter().fold(0.0f64, |a, b| a.max(*b))
        ));
    }
    // A material of the project for each of the model's.
    let mut materials: Vec<MaterialId> = Vec::new();
    for m in &mesh.materials {
        let full = free_name(
            |n| model.materials().any(|x| x.name == n),
            &format!("{name} · {}", m.name),
        );
        let [r, g, b] = m.color;
        let category = if m.alpha < 0.6 {
            "glass"
        } else if kind == ModelKind::Vegetation && g > r && g >= b {
            "vegetation"
        } else {
            "other"
        };
        materials.push(model.add_material(full, category, m.color, CutPattern::Solid));
    }
    let levels: Vec<MeshLevel> = archisim_mesh::levels_of_detail(&mesh)
        .into_iter()
        .map(|l| MeshLevel {
            positions: l.positions,
            colors: l.colors,
            groups: l
                .groups
                .into_iter()
                .map(|g| MeshGroup {
                    material: materials[g.material],
                    triangles: g.triangles,
                })
                .collect(),
        })
        .collect();
    let triangles = levels.iter().map(|l| l.triangle_count()).collect();
    // The material that shows most from above tints the plan in
    // presentation (a canopy's leaves).
    let main = mesh
        .groups
        .iter()
        .max_by_key(|g| g.triangles.len())
        .map_or(0, |g| g.material);
    let main_name = model
        .material(materials[main])
        .map(|m| m.name.clone())
        .unwrap_or_default();
    let tol = (size[0].max(size[1]) * 0.01).clamp(0.005, 0.05);
    let outline = archisim_mesh::outline(&mesh, tol);
    let family_name = free_name(|n| model.family_by_name(n).is_some(), name);
    let source_text = recipe(
        &family_name,
        kind,
        source,
        size,
        [lo[0] as f64, lo[1] as f64, hi[0] as f64, hi[1] as f64],
        &main_name,
        &outline,
    );
    let family = model
        .add_family(&source_text)
        .map_err(|e| format!("la familia del modelo: {e}"))?;
    let values: Values = model
        .family_defaults(family, archisim_core::ParamScope::Type)
        .map_err(|e| e.to_string())?;
    let mesh_id = model
        .add_mesh(MeshModel {
            id: archisim_core::MeshId(0),
            name: name.to_string(),
            source: source.to_string(),
            levels,
        })
        .map_err(|e| e.to_string())?;
    let object_type = model
        .add_object_type(name, family, values)
        .map_err(|e| e.to_string())?;
    model
        .set_object_type_mesh(object_type, Some(mesh_id))
        .map_err(|e| e.to_string())?;
    Ok(Imported {
        object_type,
        size,
        triangles,
        materials: materials.len(),
    })
}

/// The 3D model of an object type as a `.glb` (its model, at the type's
/// size, facing as the file it came from), and its kind, to keep it in the
/// library folder. `None` if the type has no model.
pub fn export_glb(model: &Model, object_type: ObjectTypeId) -> Option<(Vec<u8>, ModelKind)> {
    let t = model.object_type(object_type)?;
    let mesh = model.mesh(t.mesh?)?;
    let level = mesh.levels.first()?;
    let category = model.family(t.family)?.def().category.clone();
    let kind = ModelKind::ALL
        .into_iter()
        .find(|k| k.category() == category)
        .unwrap_or(ModelKind::Furniture);
    let natural = mesh.size()[2];
    let mut values = model
        .family_defaults(t.family, archisim_core::ParamScope::Type)
        .ok()?;
    values.extend(t.values.clone());
    let k = match values.get("height") {
        Some(archisim_core::Value::Number(h)) if natural > 1e-6 => h / natural,
        _ => 1.0,
    } as f32;
    let turn = if kind.centred() { 1.0 } else { -1.0 };
    let mut materials = Vec::new();
    let mut groups = Vec::new();
    for g in &level.groups {
        let mat = model.material(g.material)?;
        // Without the type's name in front, as it was in the file.
        let name = mat
            .name
            .split_once(" · ")
            .map_or(mat.name.as_str(), |(_, n)| n)
            .to_string();
        materials.push(archisim_mesh::Material {
            name,
            color: mat.cut_color,
            alpha: if mat.category == "glass" { 0.4 } else { 1.0 },
            texture: None,
        });
        groups.push(archisim_mesh::Group {
            material: materials.len() - 1,
            triangles: g.triangles.clone(),
        });
    }
    let m = Mesh {
        positions: level
            .positions
            .iter()
            .map(|p| [p[0] * k * turn, p[1] * k * turn, p[2] * k])
            .collect(),
        colors: level.colors.clone(),
        materials,
        groups,
    };
    Some((archisim_mesh::write_glb(&m, &t.name), kind))
}

type Ring = Vec<[f64; 2]>;

/// The family written for a model: its height (which scales it), a box
/// for what knows of the model only its size, and its outline in plan.
fn recipe(
    name: &str,
    kind: ModelKind,
    source: &str,
    size: [f64; 3],
    rect: [f64; 4],
    main: &str,
    outline: &[(Ring, Vec<Ring>)],
) -> String {
    let h = size[2];
    let (ifc_class, ifc_type) = kind.ifc();
    let mut s = String::new();
    s.push_str(&format!("family \"{}\"\n", quote(name)));
    s.push_str(&format!("  category {}\n", kind.category()));
    s.push_str("  host none\n  version 1\n");
    s.push_str(&format!(
        "  description \"Modelo 3D importado de {}\"\n",
        quote(source)
    ));
    s.push_str(&format!("  folder \"{}\"\n\n", kind.label()));
    s.push_str("type_params\n  section \"Dimensiones\"\n");
    s.push_str(&format!(
        "    height length = {} range {}..{} label \"Altura (escala el modelo)\"\n",
        fmt(h),
        fmt((h * 0.05).max(0.01)),
        fmt(h * 20.0)
    ));
    if kind == ModelKind::Vegetation {
        // As the trees of the library: drawn with their colour in every
        // view unless told otherwise.
        s.push_str("  section \"Presentación\"\n");
        s.push_str("    plan_drawing choice(\"presentación\", \"según el estilo\", \"técnico\") = \"presentación\" label \"Dibujo en planta\"\n");
    }
    s.push_str("  section \"Materiales\"\n");
    s.push_str(&format!(
        "    mat material = \"{}\" label \"Color en planta\"\n\n",
        quote(main)
    ));
    let k = "height / ".to_string() + &fmt(h);
    s.push_str("body\n");
    s.push_str(&format!("  let k = {k}\n"));
    s.push_str(&format!(
        "  box ({} * k, {} * k, 0) to ({} * k, {} * k, height) material mat\n\n",
        fmt(rect[0]),
        fmt(rect[1]),
        fmt(rect[2]),
        fmt(rect[3])
    ));
    let polygon = |ring: &Ring, tail: &str| -> String {
        let pts: Vec<String> = ring
            .iter()
            .map(|p| format!("({} * k, {} * k)", fmt(p[0]), fmt(p[1])))
            .collect();
        format!("  polygon {} {tail}\n", pts.join(" "))
    };
    let box_ring: Ring = vec![
        [rect[0], rect[1]],
        [rect[2], rect[1]],
        [rect[2], rect[3]],
        [rect[0], rect[3]],
    ];
    let rings: Vec<(&Ring, bool)> = if outline.is_empty() {
        vec![(&box_ring, false)]
    } else {
        outline
            .iter()
            .flat_map(|(o, holes)| {
                std::iter::once((o, false)).chain(holes.iter().map(|h| (h, true)))
            })
            .collect()
    };
    s.push_str("plan 1:50\n");
    s.push_str(&format!("  let k = {k}\n"));
    for (r, _) in &rings {
        s.push_str(&polygon(r, "pen view"));
    }
    s.push_str("\nplan presentation\n");
    s.push_str(&format!("  let k = {k}\n"));
    for (r, hole) in &rings {
        if *hole {
            s.push_str(&polygon(r, "pen thin"));
        } else {
            s.push_str(&polygon(r, "pen thin tint mat opacity 0.75"));
        }
    }
    s.push_str("\noutputs\n");
    s.push_str(&format!("  ifc_class = \"{ifc_class}\"\n"));
    s.push_str(&format!("  ifc_type = \"{ifc_type}\"\n"));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    trait Positions {
        fn object_mesh_level0(&self, t: ObjectTypeId) -> Vec<[f32; 3]>;
    }

    impl Positions for Model {
        /// The corners of the model's triangles, in order.
        fn object_mesh_level0(&self, t: ObjectTypeId) -> Vec<[f32; 3]> {
            let mesh = self
                .mesh(self.object_type(t).unwrap().mesh.unwrap())
                .unwrap();
            let l = &mesh.levels[0];
            l.groups
                .iter()
                .flat_map(|g| {
                    g.triangles
                        .iter()
                        .flatten()
                        .map(|&i| l.positions[i as usize])
                })
                .collect()
        }
    }

    fn fixture(name: &str) -> Mesh {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../archisim-mesh/tests/fixtures")
            .join(name);
        archisim_mesh::read_file(&p, Default::default()).unwrap()
    }

    #[test]
    fn a_tree_and_a_chair_become_object_types() {
        let mut m = crate::new_project("Prueba");
        let level = m.levels()[0].id;
        let tree = import_mesh(
            &mut m,
            fixture("arbol.dae"),
            "Árbol de SketchUp",
            "arbol.dae",
            ModelKind::Vegetation,
        )
        .unwrap();
        assert!((tree.size[2] - 4.0).abs() < 1e-3);
        let t = m.object_type(tree.object_type).unwrap().clone();
        assert!(t.mesh.is_some());
        let fam = m.family(t.family).unwrap();
        assert_eq!(fam.def().category, "site");
        // Placed, it is centred on its trunk; its plan symbol is the
        // canopy's outline.
        let o = m
            .add_object(archisim_core::NewObject {
                level,
                object_type: tree.object_type,
                position: archisim_core::Point2::new(5.0, 5.0),
                rotation: 0.0,
                base_offset: 0.0,
            })
            .unwrap();
        let marks = m.object_plan_symbol(o, 50.0).unwrap();
        assert!(!marks.is_empty());
        let outline = m.object_outline(o);
        let w = outline[0].distance(outline[1]);
        assert!((w - 2.5).abs() < 0.05, "{w}");
        // Twice as tall: twice as wide.
        let mut v = t.values.clone();
        v.insert("height".into(), archisim_core::Value::Number(8.0));
        m.set_object_type_values(tree.object_type, v).unwrap();
        let (_, k, _) = m.object_mesh(o).unwrap();
        assert!((k - 2.0).abs() < 1e-6);
        // The chair: its back on the insertion point, its front to +y.
        let chair = import_mesh(
            &mut m,
            fixture("silla.obj"),
            "Silla de catálogo",
            "silla.obj",
            ModelKind::Furniture,
        )
        .unwrap();
        let (lo, hi) = m.object_type_box(chair.object_type).unwrap();
        assert!(
            lo[1].abs() < 1e-6 && (hi[1] - 0.45).abs() < 1e-3,
            "{lo:?} {hi:?}"
        );
        // Kept as a .glb, it comes back the same size and facing the same
        // way.
        let (glb, kind) = export_glb(&m, chair.object_type).unwrap();
        assert_eq!(kind, ModelKind::Furniture);
        let back =
            archisim_mesh::read(&glb, "glb", &archisim_mesh::NoResources, Default::default())
                .unwrap();
        let again = import_mesh(&mut m, back, "Silla devuelta", "silla.glb", kind).unwrap();
        let a = m.object_mesh_level0(chair.object_type);
        let b = m.object_mesh_level0(again.object_type);
        assert_eq!(a.len(), b.len());
        for (p, q) in a.iter().zip(&b) {
            for k in 0..3 {
                assert!((p[k] - q[k]).abs() < 1e-4, "{p:?} {q:?}");
            }
        }
        // The same name again: a family of its own.
        let again = import_mesh(
            &mut m,
            fixture("silla.obj"),
            "Silla de catálogo",
            "silla.obj",
            ModelKind::Furniture,
        )
        .unwrap();
        let fa = m.object_type(again.object_type).unwrap().family;
        assert_ne!(fa, m.object_type(chair.object_type).unwrap().family);
    }
}
