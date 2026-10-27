//! 3D models from other programs (SketchUp, Blender, a manufacturer's
//! catalogue) turned into object types of the project: the model with its
//! lighter versions, a material per material of the model (of the kind
//! its name says: leather, fabric, wood, metal, glass…), and a family
//! written for it whose plan symbol is the model's outline seen from
//! above, scaled with the type's height, as for any other object.
//!
//! A design piece of a manufacturer (an LC2 of Cassina, a Togo of Ligne
//! Roset) keeps who made it ([`Maker`]): its type has the manufacturer,
//! the model, the reference, the designer, the year and the link, and the
//! IFC carries them in `Pset_ManufacturerTypeInformation`. Its file is the
//! manufacturer's, never ArchiSim's: each user keeps it in the «Diseño»
//! folder of their own library.

use archisim_core::{
    Appearance, CutPattern, MaterialId, MeshGroup, MeshLevel, MeshModel, Model, ObjectTypeId,
    TextureKind, Value, Values,
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

/// Who made a piece, and which one it is.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Maker {
    pub manufacturer: String,
    /// Its name in the catalogue («LC2», «Togo»).
    pub product: String,
    /// The manufacturer's reference (its article number).
    pub reference: String,
    pub designer: String,
    pub year: String,
    /// Its page on the manufacturer's site.
    pub url: String,
}

impl Maker {
    /// Its fields: type parameter, label, property of the IFC.
    pub const FIELDS: [(&'static str, &'static str, &'static str); 6] = [
        (
            "manufacturer",
            "Fabricante",
            "Pset_ManufacturerTypeInformation.Manufacturer",
        ),
        (
            "product",
            "Modelo",
            "Pset_ManufacturerTypeInformation.ModelLabel",
        ),
        (
            "reference",
            "Referencia",
            "Pset_ManufacturerTypeInformation.ModelReference",
        ),
        ("designer", "Diseño de", "ArchiSim_Design.Designer"),
        ("year", "Año", "ArchiSim_Design.Year"),
        ("url", "Enlace", "ArchiSim_Design.URL"),
    ];

    pub fn field(&self, i: usize) -> &str {
        match i {
            0 => &self.manufacturer,
            1 => &self.product,
            2 => &self.reference,
            3 => &self.designer,
            4 => &self.year,
            _ => &self.url,
        }
    }

    pub fn field_mut(&mut self, i: usize) -> &mut String {
        match i {
            0 => &mut self.manufacturer,
            1 => &mut self.product,
            2 => &mut self.reference,
            3 => &mut self.designer,
            4 => &mut self.year,
            _ => &mut self.url,
        }
    }

    pub fn is_empty(&self) -> bool {
        (0..6).all(|i| self.field(i).trim().is_empty())
    }

    /// The file kept next to a design piece in the library: its name, its
    /// kind and who made it.
    pub fn to_json(&self, name: &str, kind: ModelKind) -> String {
        let mut o = serde_json::Map::new();
        o.insert("nombre".into(), name.into());
        o.insert("tipo".into(), kind.label().into());
        for (i, (key, _, _)) in Self::FIELDS.iter().enumerate() {
            if !self.field(i).is_empty() {
                o.insert((*key).into(), self.field(i).into());
            }
        }
        serde_json::to_string_pretty(&serde_json::Value::Object(o)).unwrap_or_default() + "\n"
    }

    /// What [`to_json`](Self::to_json) wrote: who made it, the piece's
    /// name and its kind (if given).
    pub fn from_json(text: &str) -> Option<(Maker, Option<String>, Option<ModelKind>)> {
        let v: serde_json::Value = serde_json::from_str(text).ok()?;
        let o = v.as_object()?;
        let mut m = Maker::default();
        for (i, (key, _, _)) in Self::FIELDS.iter().enumerate() {
            if let Some(s) = o.get(*key).and_then(|x| x.as_str()) {
                *m.field_mut(i) = s.to_string();
            }
        }
        let name = o.get("nombre").and_then(|x| x.as_str()).map(String::from);
        let kind = o
            .get("tipo")
            .and_then(|x| x.as_str())
            .and_then(|t| ModelKind::ALL.into_iter().find(|k| k.label() == t));
        Some((m, name, kind))
    }
}

/// Who made the piece of an object type, if it says.
pub fn maker_of(model: &Model, object_type: ObjectTypeId) -> Option<Maker> {
    let t = model.object_type(object_type)?;
    let mut m = Maker::default();
    for (i, (key, _, _)) in Maker::FIELDS.iter().enumerate() {
        if let Some(Value::Text(s)) = t.values.get(*key) {
            *m.field_mut(i) = s.clone();
        }
    }
    (!m.is_empty()).then_some(m)
}

/// How a model is brought in.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportOptions {
    /// Quarter turns (anticlockwise seen from above) that bring its front
    /// to where the file's front is taken to be (−y).
    pub turns: u8,
    /// At most these faces in its nearest version (none: as it is).
    pub detail: Option<usize>,
    /// Who made it: a design piece.
    pub maker: Option<Maker>,
}

impl ImportOptions {
    /// The same, for a model already [`prepare`]d.
    pub fn prepared(&self) -> ImportOptions {
        ImportOptions {
            turns: 0,
            detail: None,
            maker: self.maker.clone(),
        }
    }
}

/// How much detail is kept of a heavy model: its name and the faces of its
/// nearest version (none: all).
pub const DETAILS: [(&str, Option<usize>); 4] = [
    ("Muy alto (150.000 caras)", Some(150_000)),
    ("Alto (50.000 caras)", Some(50_000)),
    ("Medio (15.000 caras)", Some(15_000)),
    ("Todo, sin aligerar", None),
];

/// The detail of [`DETAILS`] for a model brought in without choosing.
pub const DEFAULT_DETAIL: usize = 1;

/// What a material of a model is by its name (manufacturers name them:
/// «Leather_Cognac», «Oak natural», «Chrome»…), in Spanish, English,
/// Italian, German or French: its category and its texture in 3D.
pub fn finish_of(name: &str) -> (&'static str, Option<TextureKind>) {
    let n = name
        .to_lowercase()
        .replace(['_', '-', '.'], " ")
        .replace('á', "a")
        .replace('é', "e")
        .replace('í', "i")
        .replace('ó', "o")
        .replace('ú', "u");
    // Short words at the start of a word («lack» is not in «black»), the
    // others anywhere («nogal» in «nogalcanaletto»).
    let words: Vec<&str> = n.split_whitespace().collect();
    let has = |w: &[&str]| {
        w.iter().any(|x| {
            if x.chars().count() <= 4 {
                words.iter().any(|t| t.starts_with(x))
            } else {
                n.contains(x)
            }
        })
    };
    if has(&["glass", "vidrio", "cristal", "vetro", "glas", "verre"]) {
        ("glass", None)
    } else if has(&["boucle"]) {
        ("fabric", Some(TextureKind::Boucle))
    } else if has(&["velvet", "terciopelo", "velluto", "samt", "velours"]) {
        ("fabric", Some(TextureKind::Velvet))
    } else if has(&[
        "wool", "lana", "knit", "punto", "wolle", "laine", "cashmere",
    ]) {
        ("fabric", Some(TextureKind::Knit))
    } else if has(&[
        "leather", "cuero", "piel", "pelle", "leder", "cuir", "nappa", "aniline", "suede",
    ]) {
        ("fabric", None)
    } else if has(&[
        "fabric",
        "tela",
        "textil",
        "tejido",
        "tessuto",
        "stoff",
        "tissu",
        "linen",
        "lino",
        "cotton",
        "algodon",
        "cotone",
        "upholster",
        "tapiz",
        "cushion",
        "cojin",
        "cuscino",
        "kvadrat",
        "felt",
        "fieltro",
    ]) {
        ("fabric", Some(TextureKind::Linen))
    } else if has(&[
        "rattan", "ratan", "cane", "wicker", "mimbre", "rush", "enea", "bamboo", "bambu", "straw",
        "paja", "paglia", "vienna", "weave",
    ]) {
        ("wood", Some(TextureKind::Wicker))
    } else if has(&["marble", "marmol", "marmo", "marmor", "marbre"]) {
        ("stone", Some(TextureKind::Marble))
    } else if has(&["terrazzo", "terrazo"]) {
        ("stone", Some(TextureKind::Terrazzo))
    } else if has(&[
        "stone",
        "piedra",
        "pietra",
        "stein",
        "pierre",
        "granite",
        "granito",
        "travertin",
        "slate",
        "pizarra",
    ]) {
        ("stone", Some(TextureKind::Stone))
    } else if has(&[
        "lacquer", "lacado", "laccato", "lack", "laque", "painted", "pintado",
    ]) {
        ("wood", None)
    } else if has(&[
        "wood",
        "madera",
        "legno",
        "holz",
        "bois",
        "oak",
        "roble",
        "rovere",
        "eiche",
        "chene",
        "walnut",
        "nogal",
        "noce",
        "nuss",
        "noyer",
        "ash",
        "fresno",
        "frassino",
        "esche",
        "beech",
        "haya",
        "faggio",
        "buche",
        "teak",
        "teca",
        "pine",
        "pino",
        "maple",
        "arce",
        "cherry",
        "cerezo",
        "elm",
        "olmo",
        "birch",
        "abedul",
        "veneer",
        "chapa",
        "plywood",
        "contrachapado",
        "multiplex",
    ]) {
        ("wood", Some(TextureKind::Wood))
    } else if has(&[
        "chrome", "cromo", "chrom", "steel", "acero", "acciaio", "stahl", "acier", "inox",
        "alumin", "brass", "laton", "ottone", "messing", "copper", "cobre", "rame", "bronze",
        "bronce", "iron", "hierro", "ferro", "eisen", "metal", "nickel", "niquel",
    ]) {
        ("metal", None)
    } else if has(&["concrete", "hormigon", "cemento", "beton"]) {
        ("concrete", Some(TextureKind::Concrete))
    } else if has(&["ceramic", "ceramica", "porcelain", "porcelana", "keramik"]) {
        ("ceramic", None)
    } else if has(&[
        "plastic",
        "plastico",
        "polypropylene",
        "polipropileno",
        "abs",
        "resin",
        "resina",
        "acrylic",
        "acrilico",
        "polyamide",
    ]) {
        ("plastic", None)
    } else if has(&["rubber", "goma", "caucho", "gomma"]) {
        ("rubber", None)
    } else if has(&[
        "leaf", "leaves", "hoja", "foliage", "plant", "planta", "grass", "cesped",
    ]) {
        ("vegetation", None)
    } else {
        ("other", None)
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
    mesh: Mesh,
    name: &str,
    source: &str,
    kind: ModelKind,
) -> Result<Imported, String> {
    import_mesh_with(model, mesh, name, source, kind, &ImportOptions::default())
}

/// The model turned and lightened as `options` say (the slow part of
/// bringing a heavy model in, which the application does aside); with
/// [`ImportOptions::prepared`] it is not done again.
pub fn prepare(mut mesh: Mesh, options: &ImportOptions) -> Mesh {
    for _ in 0..options.turns % 4 {
        for p in &mut mesh.positions {
            *p = [-p[1], p[0], p[2]];
        }
    }
    if let Some(n) = options.detail {
        if mesh.triangle_count() > n {
            mesh = archisim_mesh::decimate(&mesh, n)
                .unwrap_or_else(|| archisim_mesh::simplify(&mesh, n));
        }
    }
    mesh
}

/// [`import_mesh`] turned, lightened and with who made it, as `options`
/// say.
pub fn import_mesh_with(
    model: &mut Model,
    mut mesh: Mesh,
    name: &str,
    source: &str,
    kind: ModelKind,
    options: &ImportOptions,
) -> Result<Imported, String> {
    let name = name.trim();
    let name = if name.is_empty() { "Modelo 3D" } else { name };
    mesh = prepare(mesh, options);
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
        let (category, texture) = finish_of(&m.name);
        let category = if m.alpha < 0.6 {
            "glass"
        } else if kind == ModelKind::Vegetation && g > r && g >= b {
            "vegetation"
        } else {
            category
        };
        let id = model.add_material(full, category, m.color, CutPattern::Solid);
        if let Some(t) = texture.filter(|_| category != "glass") {
            let _ = model.set_appearance(
                id,
                Appearance {
                    color: None,
                    texture: Some(t),
                    scale: None,
                    growth: None,
                },
            );
        }
        materials.push(id);
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
        (
            size,
            [lo[0] as f64, lo[1] as f64, hi[0] as f64, hi[1] as f64],
        ),
        &main_name,
        &outline,
        options.maker.as_ref().filter(|m| !m.is_empty()),
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
    (size, rect): ([f64; 3], [f64; 4]),
    main: &str,
    outline: &[(Ring, Vec<Ring>)],
    maker: Option<&Maker>,
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
    s.push_str(&format!(
        "  folder \"{}\"\n\n",
        if maker.is_some() {
            "Diseño"
        } else {
            kind.label()
        }
    ));
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
        "    mat material = \"{}\" label \"Color en planta\"\n",
        quote(main)
    ));
    if let Some(m) = maker {
        s.push_str("  section \"Fabricante\"\n");
        for (i, (key, label, ifc)) in Maker::FIELDS.iter().enumerate() {
            s.push_str(&format!(
                "    {key} text = \"{}\" label \"{label}\" ifc \"{ifc}\"\n",
                quote(m.field(i))
            ));
        }
    }
    s.push('\n');
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

    #[test]
    fn a_design_piece_from_a_manufacturers_zip() {
        let mut m = crate::new_project("Prueba");
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../archisim-mesh/tests/fixtures/butaca.zip");
        let loaded = archisim_mesh::load(&p, None, Default::default()).unwrap();
        let full = loaded.mesh.triangle_count();
        let (lo, hi) = loaded.mesh.bounds();
        let maker = Maker {
            manufacturer: "Muebles Ejemplo".into(),
            product: "Butaca Uno".into(),
            reference: "BU-01".into(),
            designer: "Ana Pérez".into(),
            year: "1958".into(),
            url: "https://example.com/butaca-uno".into(),
        };
        let options = ImportOptions {
            turns: 1,
            detail: Some(120),
            maker: Some(maker.clone()),
        };
        let r = import_mesh_with(
            &mut m,
            loaded.mesh,
            "Butaca Uno",
            "butaca.zip",
            ModelKind::Furniture,
            &options,
        )
        .unwrap();
        // Lightened, and turned a quarter: width and depth swap.
        assert!(
            r.triangles[0] <= 120 && r.triangles[0] < full,
            "{:?}",
            r.triangles
        );
        assert!(
            (r.size[0] - (hi[1] - lo[1]) as f64).abs() < 0.02,
            "{:?}",
            r.size
        );
        assert!(
            (r.size[1] - (hi[0] - lo[0]) as f64).abs() < 0.02,
            "{:?}",
            r.size
        );
        // Who made it, in its type and its folder.
        assert_eq!(maker_of(&m, r.object_type), Some(maker.clone()));
        let t = m.object_type(r.object_type).unwrap();
        let fam = m.family(t.family).unwrap();
        assert_eq!(fam.def().folder, "Diseño");
        // Its materials by their names.
        let mat = |n: &str| {
            m.materials()
                .find(|x| x.name == format!("Butaca Uno · {n}"))
                .unwrap()
                .clone()
        };
        assert_eq!(mat("Vidrio").category, "glass");
        assert_eq!(mat("Nogal").category, "wood");
        assert_eq!(
            m.appearance(mat("Nogal").id).texture,
            Some(TextureKind::Wood)
        );
        assert_eq!(mat("Cuero coñac").category, "fabric");
        // The file kept next to it in the library.
        let json = maker.to_json("Butaca Uno", ModelKind::Furniture);
        let (back, name, kind) = Maker::from_json(&json).unwrap();
        assert_eq!(back, maker);
        assert_eq!(name.as_deref(), Some("Butaca Uno"));
        assert_eq!(kind, Some(ModelKind::Furniture));
    }

    #[test]
    fn finishes_by_their_names() {
        assert_eq!(finish_of("Leather_Cognac_01").0, "fabric");
        assert_eq!(finish_of("Oak natural"), ("wood", Some(TextureKind::Wood)));
        assert_eq!(
            finish_of("Rovere naturale"),
            ("wood", Some(TextureKind::Wood))
        );
        assert_eq!(finish_of("Chrome"), ("metal", None));
        assert_eq!(finish_of("Stahl verchromt"), ("metal", None));
        assert_eq!(
            finish_of("Bouclé blanco"),
            ("fabric", Some(TextureKind::Boucle))
        );
        assert_eq!(
            finish_of("Mármol Carrara"),
            ("stone", Some(TextureKind::Marble))
        );
        assert_eq!(
            finish_of("Vienna straw"),
            ("wood", Some(TextureKind::Wicker))
        );
        assert_eq!(finish_of("Lacado negro"), ("wood", None));
        assert_eq!(finish_of("Material #25"), ("other", None));
        assert_eq!(finish_of("Black oak"), ("wood", Some(TextureKind::Wood)));
        assert_eq!(finish_of("Eiche lackiert"), ("wood", None));
        assert_eq!(finish_of("Black leather"), ("fabric", None));
    }
}
