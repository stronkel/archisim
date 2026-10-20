//! Wall and slab types between projects. They are not made from families
//! (a multilayer is its layers), so they go to their own library file
//! (`.astipos`), a readable list of types with their layers by material
//! name and the materials they use:
//!
//! ```text
//! material "Ladrillo perforado" category "brick" color 196 120 90 pattern diagonal
//! wall_type "Fachada 26,5" guid "3Ax…" function exterior
//!   layer "Ladrillo perforado" 0.115 structure
//! slab_type "Forjado HA 30" guid "1Bc…" function floor
//!   layer "Hormigón armado" 0.30 structure
//! ```
//!
//! A type keeps its `GlobalId` from project to project, as family types do.

use super::{
    CutPattern, Layer, LayerFunction, Model, ModelError, OnConflict, SlabFunction, WallFunction,
};
use crate::ids::*;

/// A material a library type uses, by name.
#[derive(Debug, Clone, PartialEq)]
pub struct LayeredMaterial {
    pub name: String,
    pub category: String,
    pub color: [u8; 3],
    pub pattern: CutPattern,
}

/// Whether a library type is of walls or of slabs, with its function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayeredKind {
    Wall(WallFunction),
    Slab(SlabFunction),
}

/// A wall or slab type as a library file keeps it.
#[derive(Debug, Clone, PartialEq)]
pub struct LayeredType {
    pub name: String,
    pub guid: String,
    pub kind: LayeredKind,
    /// Material name, thickness (metres) and function of each layer, from
    /// the exterior (walls) or the top (slabs).
    pub layers: Vec<(String, f64, LayerFunction)>,
}

/// The contents of a `.astipos` file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LayeredCatalog {
    pub materials: Vec<LayeredMaterial>,
    pub types: Vec<LayeredType>,
}

/// A wall or slab type of the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LayeredRef {
    Wall(WallTypeId),
    Slab(SlabTypeId),
}

/// What loading a wall or slab type did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadedLayered {
    New(LayeredRef),
    AlreadyThere(LayeredRef),
    Replaced(LayeredRef),
}

impl LoadedLayered {
    pub fn type_ref(self) -> LayeredRef {
        match self {
            LoadedLayered::New(t) | LoadedLayered::AlreadyThere(t) | LoadedLayered::Replaced(t) => {
                t
            }
        }
    }
}

fn pattern_code(p: CutPattern) -> &'static str {
    match p {
        CutPattern::Solid => "solid",
        CutPattern::Diagonal => "diagonal",
        CutPattern::Cross => "cross",
        CutPattern::Insulation => "insulation",
        CutPattern::Dots => "dots",
        CutPattern::Empty => "empty",
    }
}

fn pattern_from(code: &str) -> Option<CutPattern> {
    Some(match code {
        "solid" => CutPattern::Solid,
        "diagonal" => CutPattern::Diagonal,
        "cross" => CutPattern::Cross,
        "insulation" => CutPattern::Insulation,
        "dots" => CutPattern::Dots,
        "empty" => CutPattern::Empty,
        _ => return None,
    })
}

fn function_code(f: LayerFunction) -> &'static str {
    match f {
        LayerFunction::Structure => "structure",
        LayerFunction::Substrate => "substrate",
        LayerFunction::Insulation => "insulation",
        LayerFunction::AirGap => "airgap",
        LayerFunction::Membrane => "membrane",
        LayerFunction::Finish => "finish",
    }
}

fn function_from(code: &str) -> Option<LayerFunction> {
    Some(match code {
        "structure" => LayerFunction::Structure,
        "substrate" => LayerFunction::Substrate,
        "insulation" => LayerFunction::Insulation,
        "airgap" => LayerFunction::AirGap,
        "membrane" => LayerFunction::Membrane,
        "finish" => LayerFunction::Finish,
        _ => return None,
    })
}

fn kind_code(k: LayeredKind) -> (&'static str, &'static str) {
    match k {
        LayeredKind::Wall(WallFunction::Exterior) => ("wall_type", "exterior"),
        LayeredKind::Wall(WallFunction::Interior) => ("wall_type", "interior"),
        LayeredKind::Slab(SlabFunction::Floor) => ("slab_type", "floor"),
        LayeredKind::Slab(SlabFunction::BaseSlab) => ("slab_type", "baseslab"),
        LayeredKind::Slab(SlabFunction::Roof) => ("slab_type", "roof"),
    }
}

fn quoted(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "'"))
}

/// The text of a `.astipos` file.
pub fn layered_catalog_text(cat: &LayeredCatalog) -> String {
    let mut s = String::from("# Tipos de muro y de forjado de ArchiSim\n");
    for m in &cat.materials {
        let [r, g, b] = m.color;
        s.push_str(&format!(
            "material {} category {} color {r} {g} {b} pattern {}\n",
            quoted(&m.name),
            quoted(&m.category),
            pattern_code(m.pattern)
        ));
    }
    for t in &cat.types {
        let (kw, f) = kind_code(t.kind);
        s.push_str(&format!(
            "{kw} {} guid {} function {f}\n",
            quoted(&t.name),
            quoted(&t.guid)
        ));
        for (m, th, func) in &t.layers {
            s.push_str(&format!(
                "  layer {} {} {}\n",
                quoted(m),
                th,
                function_code(*func)
            ));
        }
    }
    s
}

/// Splits a line into words, quoted texts kept whole (without quotes).
fn words(line: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else if c == '"' {
            chars.next();
            let mut w = String::new();
            loop {
                match chars.next() {
                    Some('"') => break,
                    Some(c) => w.push(c),
                    None => return Err("comillas sin cerrar".into()),
                }
            }
            out.push(w);
        } else {
            let mut w = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    break;
                }
                w.push(c);
                chars.next();
            }
            out.push(w);
        }
    }
    Ok(out)
}

/// Reads a `.astipos` file. Errors say the line.
pub fn parse_layered_catalog(text: &str) -> Result<LayeredCatalog, String> {
    let mut cat = LayeredCatalog::default();
    for (i, raw) in text.lines().enumerate() {
        let no = i + 1;
        let line = raw.split('#').next().unwrap_or("").trim_end();
        if line.trim().is_empty() {
            continue;
        }
        let w = words(line).map_err(|e| format!("línea {no}: {e}"))?;
        let bad = |what: &str| format!("línea {no}: {what}");
        let field = |key: &str| -> Option<&String> {
            w.iter().position(|x| x == key).and_then(|k| w.get(k + 1))
        };
        match w[0].as_str() {
            "material" => {
                let name = w
                    .get(1)
                    .ok_or_else(|| bad("falta el nombre del material"))?;
                let pos = w
                    .iter()
                    .position(|x| x == "color")
                    .ok_or_else(|| bad("falta el color"))?;
                let rgb: Vec<u8> = w[pos + 1..]
                    .iter()
                    .take(3)
                    .map(|x| x.parse::<u8>())
                    .collect::<Result<_, _>>()
                    .map_err(|_| bad("el color son tres números de 0 a 255"))?;
                if rgb.len() != 3 {
                    return Err(bad("el color son tres números de 0 a 255"));
                }
                cat.materials.push(LayeredMaterial {
                    name: name.clone(),
                    category: field("category").cloned().unwrap_or_default(),
                    color: [rgb[0], rgb[1], rgb[2]],
                    pattern: field("pattern")
                        .and_then(|p| pattern_from(p))
                        .unwrap_or(CutPattern::Solid),
                });
            }
            "wall_type" | "slab_type" => {
                let name = w.get(1).ok_or_else(|| bad("falta el nombre del tipo"))?;
                let f = field("function").map(String::as_str).unwrap_or("");
                let kind = match (w[0].as_str(), f) {
                    ("wall_type", "interior") => LayeredKind::Wall(WallFunction::Interior),
                    ("wall_type", _) => LayeredKind::Wall(WallFunction::Exterior),
                    ("slab_type", "baseslab") => LayeredKind::Slab(SlabFunction::BaseSlab),
                    ("slab_type", "roof") => LayeredKind::Slab(SlabFunction::Roof),
                    _ => LayeredKind::Slab(SlabFunction::Floor),
                };
                if cat.types.iter().any(|t| t.name == *name) {
                    return Err(bad(&format!("ya hay un tipo «{name}»")));
                }
                cat.types.push(LayeredType {
                    name: name.clone(),
                    guid: field("guid").cloned().unwrap_or_default(),
                    kind,
                    layers: Vec::new(),
                });
            }
            "layer" => {
                let t = cat
                    .types
                    .last_mut()
                    .ok_or_else(|| bad("una capa va debajo de su tipo"))?;
                let (Some(m), Some(th)) = (w.get(1), w.get(2)) else {
                    return Err(bad("una capa es: layer \"Material\" espesor función"));
                };
                let th: f64 = th
                    .replace(',', ".")
                    .parse()
                    .map_err(|_| bad("el espesor es un número en metros"))?;
                let func = w
                    .get(3)
                    .map(|f| {
                        function_from(f)
                            .ok_or_else(|| bad(&format!("función de capa desconocida «{f}»")))
                    })
                    .transpose()?
                    .unwrap_or(LayerFunction::Structure);
                t.layers.push((m.clone(), th, func));
            }
            other => return Err(bad(&format!("línea desconocida «{other}»"))),
        }
    }
    Ok(cat)
}

impl Model {
    /// A wall or slab type as a library file keeps it, with the materials
    /// it uses.
    pub fn layered_entry(
        &self,
        t: LayeredRef,
    ) -> Result<(LayeredType, Vec<LayeredMaterial>), ModelError> {
        let (name, guid, kind, layers) = match t {
            LayeredRef::Wall(id) => {
                let w = self.wall_type(id).ok_or(ModelError::UnknownWallType(id))?;
                (&w.name, &w.guid, LayeredKind::Wall(w.function), &w.layers)
            }
            LayeredRef::Slab(id) => {
                let s = self.slab_type(id).ok_or(ModelError::UnknownSlabType(id))?;
                (&s.name, &s.guid, LayeredKind::Slab(s.function), &s.layers)
            }
        };
        let mut materials: Vec<LayeredMaterial> = Vec::new();
        let mut out = Vec::new();
        for l in layers {
            let m = self
                .material(l.material)
                .ok_or(ModelError::UnknownMaterial(l.material))?;
            if !materials.iter().any(|x| x.name == m.name) {
                materials.push(LayeredMaterial {
                    name: m.name.clone(),
                    category: m.category.clone(),
                    color: m.cut_color,
                    pattern: m.cut_pattern,
                });
            }
            out.push((m.name.clone(), l.thickness, l.function));
        }
        Ok((
            LayeredType {
                name: name.clone(),
                guid: guid.as_str().to_string(),
                kind,
                layers: out,
            },
            materials,
        ))
    }

    /// Loads a wall or slab type of a library file, with its `GlobalId`;
    /// its materials are the project's of the same name (made if missing,
    /// with the file's colour and hatch). If the project already has the
    /// type with the same layers nothing changes; with others,
    /// `on_conflict` decides, as for family types.
    pub fn load_layered_type(
        &mut self,
        t: &LayeredType,
        materials: &[LayeredMaterial],
        on_conflict: OnConflict,
    ) -> Result<LoadedLayered, ModelError> {
        let mut layers = Vec::new();
        for (name, thickness, function) in &t.layers {
            let found = self.materials().find(|m| m.name == *name).map(|m| m.id);
            let id = match found {
                Some(id) => id,
                None => {
                    let m = materials.iter().find(|m| m.name == *name);
                    self.add_material(
                        name.clone(),
                        m.map_or_else(String::new, |m| m.category.clone()),
                        m.map_or([200, 200, 200], |m| m.color),
                        m.map_or(CutPattern::Solid, |m| m.pattern),
                    )
                }
            };
            layers.push(Layer {
                material: id,
                thickness: *thickness,
                function: *function,
            });
        }
        let guid = GlobalId::parse(&t.guid);
        let existing = guid.as_ref().and_then(|g| match t.kind {
            LayeredKind::Wall(_) => self
                .wall_types()
                .find(|w| w.guid == *g)
                .map(|w| LayeredRef::Wall(w.id)),
            LayeredKind::Slab(_) => self
                .slab_types()
                .find(|s| s.guid == *g)
                .map(|s| LayeredRef::Slab(s.id)),
        });
        let mut keep_guid = guid;
        if let Some(r) = existing {
            let same = match r {
                LayeredRef::Wall(id) => self
                    .wall_type(id)
                    .is_some_and(|w| w.layers == layers && LayeredKind::Wall(w.function) == t.kind),
                LayeredRef::Slab(id) => self
                    .slab_type(id)
                    .is_some_and(|s| s.layers == layers && LayeredKind::Slab(s.function) == t.kind),
            };
            if same {
                return Ok(LoadedLayered::AlreadyThere(r));
            }
            match on_conflict {
                OnConflict::Ask => return Err(ModelError::TypeConflict(t.name.clone())),
                OnConflict::Replace => {
                    match (r, t.kind) {
                        (LayeredRef::Wall(id), LayeredKind::Wall(f)) => {
                            let name = self
                                .wall_type(id)
                                .map(|w| w.name.clone())
                                .unwrap_or_default();
                            self.edit_wall_type(id, name, f, layers)?
                        }
                        (LayeredRef::Slab(id), LayeredKind::Slab(f)) => {
                            let name = self
                                .slab_type(id)
                                .map(|s| s.name.clone())
                                .unwrap_or_default();
                            self.edit_slab_type(id, name, f, layers)?
                        }
                        _ => return Err(ModelError::TypeConflict(t.name.clone())),
                    }
                    return Ok(LoadedLayered::Replaced(r));
                }
                OnConflict::Rename => keep_guid = None,
            }
        }
        let taken: Vec<String> = match t.kind {
            LayeredKind::Wall(_) => self.wall_types().map(|w| w.name.clone()).collect(),
            LayeredKind::Slab(_) => self.slab_types().map(|s| s.name.clone()).collect(),
        };
        let mut name = t.name.clone();
        let mut n = 2;
        while taken.contains(&name) {
            name = format!("{} ({n})", t.name);
            n += 1;
        }
        let r = match t.kind {
            LayeredKind::Wall(f) => {
                let id = self.add_wall_type(name, f, layers)?;
                if let (Some(g), Some(w)) = (keep_guid, self.wall_types.get_mut(&id)) {
                    w.guid = g;
                }
                LayeredRef::Wall(id)
            }
            LayeredKind::Slab(f) => {
                let id = self.add_slab_type(name, f, layers)?;
                if let (Some(g), Some(s)) = (keep_guid, self.slab_types.get_mut(&id)) {
                    s.guid = g;
                }
                LayeredRef::Slab(id)
            }
        };
        Ok(LoadedLayered::New(r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> Model {
        let mut m = Model::new("p");
        let brick = m.add_material(
            "Ladrillo perforado",
            "brick",
            [196, 120, 90],
            CutPattern::Diagonal,
        );
        let plaster = m.add_material(
            "Enlucido de yeso",
            "plaster",
            [240, 240, 240],
            CutPattern::Dots,
        );
        m.add_wall_type(
            "Fachada \"doble\"",
            WallFunction::Exterior,
            vec![
                Layer {
                    material: brick,
                    thickness: 0.115,
                    function: LayerFunction::Structure,
                },
                Layer {
                    material: plaster,
                    thickness: 0.015,
                    function: LayerFunction::Finish,
                },
            ],
        )
        .unwrap();
        m
    }

    #[test]
    fn a_wall_type_goes_to_another_project_the_same() {
        let a = project();
        let wt = a.wall_types().next().unwrap().clone();
        let (entry, mats) = a.layered_entry(LayeredRef::Wall(wt.id)).unwrap();
        let text = layered_catalog_text(&LayeredCatalog {
            materials: mats,
            types: vec![entry],
        });
        let cat = parse_layered_catalog(&text).unwrap();
        assert_eq!(cat.types.len(), 1);
        assert_eq!(cat.materials.len(), 2);
        let mut b = Model::new("b");
        let loaded = b
            .load_layered_type(&cat.types[0], &cat.materials, OnConflict::Ask)
            .unwrap();
        let LoadedLayered::New(LayeredRef::Wall(id)) = loaded else {
            panic!("{loaded:?}");
        };
        let t = b.wall_type(id).unwrap();
        assert_eq!(t.guid, wt.guid);
        assert!((t.thickness() - 0.13).abs() < 1e-12);
        let mat = b.material(t.layers[0].material).unwrap();
        assert_eq!(
            (mat.cut_color, mat.cut_pattern),
            ([196, 120, 90], CutPattern::Diagonal)
        );
        // Again: nothing new.
        assert_eq!(
            b.load_layered_type(&cat.types[0], &cat.materials, OnConflict::Ask),
            Ok(LoadedLayered::AlreadyThere(LayeredRef::Wall(id)))
        );
        // Changed in the project: asks, replaces or loads another.
        let mut other = cat.types[0].clone();
        other.layers[0].1 = 0.24;
        assert_eq!(
            b.load_layered_type(&other, &cat.materials, OnConflict::Ask),
            Err(ModelError::TypeConflict(other.name.clone()))
        );
        let r = b
            .load_layered_type(&other, &cat.materials, OnConflict::Rename)
            .unwrap();
        assert!(matches!(r, LoadedLayered::New(_)));
        assert_eq!(b.wall_types().count(), 2);
        b.load_layered_type(&other, &cat.materials, OnConflict::Replace)
            .unwrap();
        assert!((b.wall_type(id).unwrap().thickness() - 0.255).abs() < 1e-12);
    }

    #[test]
    fn bad_lines_say_where() {
        assert!(parse_layered_catalog("layer \"x\" 0.1 structure\n")
            .unwrap_err()
            .contains("línea 1"));
        assert!(
            parse_layered_catalog("wall_type \"a\"\n  layer \"x\" 0,1 bricks\n")
                .unwrap_err()
                .contains("bricks")
        );
        let ok =
            parse_layered_catalog("slab_type \"F\" function roof\n  layer \"H\" 0,3\n").unwrap();
        assert_eq!(ok.types[0].kind, LayeredKind::Slab(SlabFunction::Roof));
        assert_eq!(ok.types[0].layers[0].1, 0.3);
    }
}
