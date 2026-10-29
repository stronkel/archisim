//! Collections (revisión de Antonio del 23/10/2026): the pieces of the
//! library in one style, with its palette — nórdico cálido, mediterráneo,
//! años 50 and japandi, and since 26/10/2026 contemporáneo, rústica,
//! industrial and clásica. A collection does not list its own furniture: it
//! says what wood, fabrics, metal, ceramic and stone each piece takes, and
//! which model of the 3D some families take, so every piece of the library
//! (and of the project) has its version in each collection, named
//! «Sofá de 3 plazas · Japandi». The families of the paquete 1 (salón y
//! comedor, 2026-10) have a model per collection, named as it: a piece
//! whose model is its family's first takes its collection's.
//!
//! What is not furniture keeps its materials: sheets, lampshades, glass,
//! mirrors, chrome taps, porcelain, plants and what is on the tables.

use archisim_core::{
    FamilyId, MaterialId, Model, ModelError, ObjectTypeId, ParamKind, ParamScope, Value, Values,
};

/// A collection: its name and what each kind of material is in it (names
/// of [`crate::COZY_PALETTE`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Collection {
    pub name: &'static str,
    /// One line for the interface.
    pub hint: &'static str,
    pub wood: &'static str,
    /// The main fabric (upholstery, curtains, rugs).
    pub fabric: &'static str,
    /// A second, softer one (duvets, a second towel, the ground of a print).
    pub soft: &'static str,
    /// The colour of cushions, throws and bedspreads.
    pub accent: &'static str,
    pub metal: &'static str,
    pub ceramic: &'static str,
    pub stone: &'static str,
    /// Rattan, rush, bamboo: what is woven.
    pub weave: &'static str,
}

impl Collection {
    /// Its colours, for a strip in the interface: wood, fabric, soft,
    /// accent, metal, ceramic.
    pub fn swatches(&self) -> [&'static str; 6] {
        [
            self.wood,
            self.fabric,
            self.soft,
            self.accent,
            self.metal,
            self.ceramic,
        ]
    }
}

pub const COLLECTIONS: &[Collection] = &[
    Collection {
        name: "Nórdico cálido",
        hint: "Roble claro, bouclé y lino arena, salvia y cerámica blanca",
        wood: "Roble claro",
        fabric: "Lino arena",
        soft: "Bouclé crudo",
        accent: "Terciopelo salvia",
        metal: "Metal negro",
        ceramic: "Cerámica blanca",
        stone: "Mármol",
        weave: "Ratán",
    },
    Collection {
        name: "Mediterráneo",
        hint: "Madera encalada, lino blanco y añil, terracota, enea y microcemento",
        wood: "Madera encalada",
        fabric: "Lino blanco",
        soft: "Lino arena",
        accent: "Lino añil",
        metal: "Hierro forjado",
        ceramic: "Terracota",
        stone: "Microcemento",
        weave: "Enea",
    },
    Collection {
        name: "Años 50",
        hint: "Teca, lana oliva y terciopelo mostaza, latón, cerámica turquesa y terrazo",
        wood: "Teca",
        fabric: "Lana oliva",
        soft: "Lana cruda",
        accent: "Terciopelo mostaza",
        metal: "Latón",
        ceramic: "Cerámica turquesa",
        stone: "Terrazo",
        weave: "Ratán",
    },
    Collection {
        name: "Japandi",
        hint: "Roble ahumado, lino piedra y carbón, gres, bambú y piedra gris",
        wood: "Roble ahumado",
        fabric: "Lino piedra",
        soft: "Lino gris",
        accent: "Lana carbón",
        metal: "Metal negro",
        ceramic: "Gres",
        stone: "Piedra gris",
        weave: "Bambú",
    },
    Collection {
        name: "Contemporáneo",
        hint: "Lacado blanco, lino gris y bouclé, metal negro, terrazo y cerámica blanca",
        wood: "Roble claro",
        fabric: "Lino gris",
        soft: "Bouclé crudo",
        accent: "Lino piedra",
        metal: "Metal negro",
        ceramic: "Cerámica blanca",
        stone: "Terrazo",
        weave: "Ratán",
    },
    Collection {
        name: "Rústica",
        hint: "Pino viejo, lino crudo y lana teja, hierro forjado, terracota y piedra",
        wood: "Pino viejo",
        fabric: "Lino crudo",
        soft: "Lana cruda",
        accent: "Lana teja",
        metal: "Hierro forjado",
        ceramic: "Terracota",
        stone: "Piedra gris",
        weave: "Enea",
    },
    Collection {
        name: "Industrial",
        hint: "Roble ahumado, lino gris y lana carbón, metal negro, gres y microcemento",
        wood: "Roble ahumado",
        fabric: "Lino gris",
        soft: "Lana carbón",
        accent: "Lana teja",
        metal: "Metal negro",
        ceramic: "Gres",
        stone: "Microcemento",
        weave: "Enea",
    },
    Collection {
        name: "Clásica",
        hint: "Nogal, terciopelo verde botella y mostaza, latón, mármol y porcelana",
        wood: "Nogal",
        fabric: "Terciopelo verde botella",
        soft: "Lino crudo",
        accent: "Terciopelo mostaza",
        metal: "Latón",
        ceramic: "Cerámica blanca",
        stone: "Mármol",
        weave: "Ratán",
    },
];

/// A collection by its name.
pub fn collection(name: &str) -> Option<&'static Collection> {
    COLLECTIONS.iter().find(|c| c.name == name)
}

/// The name of a piece in a collection: «Sofá de 3 plazas · Japandi».
pub fn variant_name(base: &str, c: &Collection) -> String {
    format!("{base} · {}", c.name)
}

/// The name of a piece without its collection, and the collection.
pub fn split_name(name: &str) -> (&str, Option<&'static Collection>) {
    for c in COLLECTIONS {
        if let Some(base) = name
            .strip_suffix(c.name)
            .and_then(|b| b.strip_suffix(" · "))
        {
            return (base, Some(c));
        }
    }
    (name, None)
}

/// The material parameters a collection leaves alone: what is not the
/// piece's own (sheets, shades, glass, plants, what is on it).
const KEEP: &[&str] = &[
    "linen", "shade", "mount", "glass", "bulb", "flame", "leaves", "stem", "soil", "fruit",
    "fruit2", "books", "books2", "books3", "candle", "dark", "light", "screen", "water", "flowers",
    "flame2", "wood",
];

/// Those that take the accent, and those that take the soft fabric.
const ACCENT: &[&str] = &["pillow", "throw_mat", "accent", "spread_mat", "art2"];
const SOFT: &[&str] = &["duvet", "mat2", "art", "border"];

/// Materials a collection keeps wherever they are.
const KEEP_NAMES: &[&str] = &[
    "Porcelana",
    "Cromado",
    "Espejo",
    "Pantalla de lino",
    "Visillo",
    "Toalla blanca",
    "Libros",
    "Libros claros",
];

/// What is woven.
const WOVEN: &[&str] = &["Ratán", "Enea", "Bambú", "Paja", "Cañizo"];

/// The material of the palette a collection gives to a material parameter
/// `param` now of material `name` (of `category`); `None` when the
/// collection leaves it alone.
pub fn collection_material(
    param: &str,
    name: &str,
    category: &str,
    c: &Collection,
) -> Option<&'static str> {
    if KEEP.contains(&param) || KEEP_NAMES.contains(&name) {
        return None;
    }
    Some(if ACCENT.contains(&param) {
        c.accent
    } else if SOFT.contains(&param) && category == "fabric" {
        c.soft
    } else {
        match category {
            "fabric" => c.fabric,
            "wood" if WOVEN.contains(&name) => c.weave,
            "wood" => c.wood,
            "metal" => c.metal,
            "ceramic" => c.ceramic,
            "stone" => c.stone,
            _ => return None,
        }
    })
}

/// The values of a type of `family` in a collection: its materials and,
/// unless `own_model`, its model of the 3D.
pub fn restyle(
    m: &mut Model,
    family: FamilyId,
    values: &Values,
    c: &Collection,
    own_model: bool,
) -> Result<Values, ModelError> {
    let defaults = m.family_defaults(family, ParamScope::Type)?;
    let Some(f) = m.family(family) else {
        return Ok(values.clone());
    };
    let def = f.def().clone();
    let mut out = values.clone();
    for param in def.params_of(ParamScope::Type) {
        match &param.kind {
            ParamKind::Material => {}
            ParamKind::Choice(options) if param.name == "model3d" => {
                let chosen = values.get("model3d").or(defaults.get("model3d"));
                let is_default = chosen == defaults.get("model3d");
                if own_model || !is_default {
                    continue;
                }
                // Its model named as the collection.
                let model = options
                    .iter()
                    .find(|o| o.as_str() == c.name)
                    .map(String::as_str);
                if let Some(model) = model {
                    out.insert("model3d".into(), Value::Text(model.into()));
                }
                continue;
            }
            _ => continue,
        }
        if KEEP.contains(&param.name.as_str()) {
            continue;
        }
        let Some(Value::Material(id)) = values.get(&param.name).or(defaults.get(&param.name))
        else {
            continue;
        };
        let Some(mat) = m.material(MaterialId(*id)) else {
            continue;
        };
        if KEEP_NAMES.contains(&mat.name.as_str()) {
            continue;
        }
        let Some(to) = collection_material(&param.name, &mat.name, &mat.category, c) else {
            continue;
        };
        if let Some(new) = crate::palette_material(m, to) {
            out.insert(param.name.clone(), Value::Material(new.0));
        }
    }
    Ok(out)
}

/// The type of a piece of the library (by its name in
/// [`crate::OBJECT_CATALOGUE`]) in a collection: the project's, or added.
pub fn add_collection_entry(
    m: &mut Model,
    name: &str,
    c: &Collection,
) -> Result<ObjectTypeId, ModelError> {
    let variant = variant_name(name, c);
    if let Some(t) = m.object_types().find(|t| t.name == variant) {
        return Ok(t.id);
    }
    let Some((f, values)) = crate::entry_values(m, name)? else {
        return Err(ModelError::NotAnObject(name.to_string()));
    };
    let values = restyle(m, f, &values, c, crate::entry_has_model(name))?;
    m.add_object_type(variant, f, values)
}

/// The type an object of type `t` takes in a collection (`None`: the
/// piece as it is in no collection): found in the project, made from the
/// library's piece of that name, or from `t` itself.
pub fn type_in_collection(
    m: &mut Model,
    t: ObjectTypeId,
    c: Option<&Collection>,
) -> Result<ObjectTypeId, ModelError> {
    let Some(ty) = m.object_type(t).cloned() else {
        return Err(ModelError::NotAnObject(format!("{t:?}")));
    };
    let (base, now) = split_name(&ty.name);
    let base = base.to_string();
    if now.map(|x| x.name) == c.map(|x| x.name) {
        return Ok(t);
    }
    let in_library = crate::OBJECT_CATALOGUE.iter().any(|e| e.0 == base);
    let Some(c) = c else {
        // Back to the piece itself.
        if let Some(b) = m.object_types().find(|x| x.name == base) {
            return Ok(b.id);
        }
        if in_library {
            return crate::add_object_entry(m, &base);
        }
        return Ok(t);
    };
    let variant = variant_name(&base, c);
    if let Some(v) = m.object_types().find(|x| x.name == variant) {
        return Ok(v.id);
    }
    if in_library {
        return add_collection_entry(m, &base, c);
    }
    // A piece of the project's own: from its type without a collection,
    // if it is there.
    let source = m
        .object_types()
        .find(|x| x.name == base)
        .cloned()
        .unwrap_or(ty);
    let values = restyle(m, source.family, &source.values, c, false)?;
    let new = m.add_object_type(variant, source.family, values)?;
    m.set_object_type_mesh(new, source.mesh)?;
    Ok(new)
}

/// A project with every piece of the library in a collection (for the
/// pictures of the catalog).
pub fn collection_library(c: &Collection) -> Model {
    let mut m = crate::new_project("Colección");
    for (name, _, _) in crate::OBJECT_CATALOGUE {
        let _ = add_collection_entry(&mut m, name, c);
    }
    m
}

/// Puts objects in a collection (`None`: in none), each with its piece's
/// type in it. Returns how many changed.
pub fn apply_collection(
    m: &mut Model,
    objects: &[archisim_core::ObjectId],
    c: Option<&Collection>,
) -> Result<usize, ModelError> {
    let mut changed = 0;
    for &id in objects {
        let Some(o) = m.object(id) else {
            continue;
        };
        let t = o.object_type;
        if !m.type_has_prop(t) && m.object_type(t).is_some_and(|x| x.mesh.is_some()) {
            // A model brought from another program has its own colours.
            continue;
        }
        let new = type_in_collection(m, t, c)?;
        if new != t {
            m.edit_object(id, |o| o.object_type = new)?;
            changed += 1;
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_split() {
        let j = collection("Japandi").unwrap();
        assert_eq!(
            variant_name("Sofá de 3 plazas", j),
            "Sofá de 3 plazas · Japandi"
        );
        assert_eq!(
            split_name("Sofá de 3 plazas · Japandi"),
            ("Sofá de 3 plazas", Some(j))
        );
        assert_eq!(split_name("Sofá · Lana teja").1, None);
    }

    #[test]
    fn every_colour_of_every_collection_is_in_the_palette() {
        for c in COLLECTIONS {
            for name in c.swatches().into_iter().chain([c.stone, c.weave]) {
                assert!(
                    crate::COZY_PALETTE.iter().any(|e| e.0 == name),
                    "{}: {name}",
                    c.name
                );
            }
        }
    }

    #[test]
    fn a_sofa_in_each_collection() {
        let mut m = crate::new_project("Prueba");
        let base = m
            .object_types()
            .find(|t| t.name == "Sofá acogedor de 3 plazas")
            .unwrap()
            .id;
        for c in COLLECTIONS {
            let t = type_in_collection(&mut m, base, Some(c)).unwrap();
            let ty = m.object_type(t).unwrap().clone();
            assert_eq!(ty.name, format!("Sofá acogedor de 3 plazas · {}", c.name));
            let mat = |k: &str| match ty.values.get(k) {
                Some(Value::Material(id)) => m.material(MaterialId(*id)).unwrap().name.clone(),
                other => panic!("{k}: {other:?}"),
            };
            assert_eq!(mat("mat"), c.fabric, "{}", c.name);
            assert_eq!(mat("frame"), c.wood, "{}", c.name);
            assert_eq!(mat("pillow"), c.accent, "{}", c.name);
            // Asked again, the same type; and back to the piece itself.
            assert_eq!(type_in_collection(&mut m, base, Some(c)).unwrap(), t);
            assert_eq!(type_in_collection(&mut m, t, None).unwrap(), base);
        }
        // A bed takes the collection's model; the porcelain of a WC stays.
        let j = collection("Japandi").unwrap();
        let bed = add_collection_entry(&mut m, "Cama doble 150×200", j).unwrap();
        assert_eq!(
            m.object_type(bed).unwrap().values.get("model3d"),
            Some(&Value::Text("Japandi".into()))
        );
        // …but a piece that chose its model keeps it.
        let own = add_collection_entry(&mut m, "Cama con cabecero tapizado 160", j).unwrap();
        assert_eq!(
            m.object_type(own).unwrap().values.get("model3d"),
            Some(&Value::Text("Clásica".into()))
        );
        let wc = add_collection_entry(&mut m, "Inodoro", j).unwrap();
        let ty = m.object_type(wc).unwrap();
        let names: Vec<String> = ty
            .values
            .values()
            .filter_map(|v| match v {
                Value::Material(id) => m.material(MaterialId(*id)).map(|x| x.name.clone()),
                _ => None,
            })
            .collect();
        assert!(names.iter().any(|n| n == "Porcelana"), "{names:?}");
        assert!(!names.iter().any(|n| n == "Gres"), "{names:?}");
    }
}
