//! The extended library (D23): many more species of trees and shrubs,
//! street furniture, people and vehicles than the starter one, each with
//! its sizes, the colour of its leaves and how its canopy is drawn in
//! presentation plans. It is not in new projects: `archisim biblioteca
//! --ampliada` writes it as library files (`biblioteca-ampliada/` in the
//! repository, downloaded apart), and the Biblioteca window and the
//! galleries load from it. Its types always get the same `GlobalId`
//! (from their names), so loading them again finds them.

use archisim_core::{CutPattern, GlobalId, Model, ModelError, ParamScope, Value};

/// A species of tree: sizes in metres, leaf colour, canopy and fill.
pub struct Tree {
    pub name: &'static str,
    pub height: f64,
    pub crown: f64,
    pub trunk_h: f64,
    pub trunk: f64,
    pub leaves: [u8; 3],
    pub shape: &'static str,
    pub look: &'static str,
    pub opacity: f64,
}

#[allow(clippy::too_many_arguments)]
const fn tree(
    name: &'static str,
    height: f64,
    crown: f64,
    trunk_h: f64,
    trunk: f64,
    leaves: [u8; 3],
    shape: &'static str,
    look: &'static str,
    opacity: f64,
) -> Tree {
    Tree {
        name,
        height,
        crown,
        trunk_h,
        trunk,
        leaves,
        shape,
        look,
        opacity,
    }
}

/// Trees: the Mediterranean ones first, then street and garden trees.
pub const TREES: &[Tree] = &[
    tree(
        "Olivo",
        5.0,
        5.0,
        1.5,
        0.5,
        [150, 164, 122],
        "follaje",
        "transparente",
        0.6,
    ),
    tree(
        "Acebuche",
        4.0,
        4.0,
        1.2,
        0.35,
        [138, 152, 110],
        "follaje",
        "transparente",
        0.6,
    ),
    tree(
        "Encina",
        9.0,
        8.0,
        2.5,
        0.6,
        [86, 112, 74],
        "lóbulos",
        "transparente",
        0.6,
    ),
    tree(
        "Alcornoque",
        10.0,
        9.0,
        3.0,
        0.7,
        [96, 120, 80],
        "lóbulos",
        "transparente",
        0.55,
    ),
    tree(
        "Algarrobo",
        7.0,
        7.0,
        2.0,
        0.5,
        [74, 106, 70],
        "lóbulos",
        "rellena",
        0.6,
    ),
    tree(
        "Pino piñonero",
        14.0,
        11.0,
        7.0,
        0.6,
        [92, 128, 86],
        "acuarela",
        "transparente",
        0.6,
    ),
    tree(
        "Pino carrasco",
        12.0,
        7.0,
        4.0,
        0.45,
        [104, 136, 92],
        "conífera",
        "transparente",
        0.6,
    ),
    tree(
        "Ciprés",
        10.0,
        1.8,
        0.6,
        0.3,
        [52, 86, 64],
        "círculo",
        "rellena",
        1.0,
    ),
    tree(
        "Ciprés de Arizona",
        8.0,
        3.5,
        0.8,
        0.3,
        [110, 140, 130],
        "conífera",
        "rellena",
        0.8,
    ),
    tree(
        "Palmera datilera",
        15.0,
        7.0,
        12.0,
        0.6,
        [128, 158, 96],
        "palmera",
        "transparente",
        0.7,
    ),
    tree(
        "Palmera canaria",
        12.0,
        9.0,
        8.0,
        0.9,
        [104, 146, 84],
        "palmera",
        "transparente",
        0.7,
    ),
    tree(
        "Washingtonia",
        18.0,
        4.0,
        16.0,
        0.5,
        [120, 150, 90],
        "palmera",
        "transparente",
        0.7,
    ),
    tree(
        "Palmito",
        2.0,
        2.5,
        0.6,
        0.3,
        [130, 160, 100],
        "palmera",
        "rellena",
        0.8,
    ),
    tree(
        "Naranjo",
        4.0,
        3.5,
        1.0,
        0.25,
        [72, 120, 70],
        "círculo",
        "rellena",
        0.85,
    ),
    tree(
        "Limonero",
        4.0,
        3.5,
        1.0,
        0.25,
        [96, 138, 74],
        "círculo",
        "rellena",
        0.85,
    ),
    tree(
        "Almendro en flor",
        5.0,
        5.0,
        1.4,
        0.3,
        [240, 206, 214],
        "acuarela",
        "transparente",
        0.6,
    ),
    tree(
        "Jacaranda",
        9.0,
        8.0,
        2.5,
        0.4,
        [150, 132, 200],
        "acuarela",
        "transparente",
        0.6,
    ),
    tree(
        "Árbol del amor",
        6.0,
        6.0,
        1.8,
        0.3,
        [214, 136, 176],
        "acuarela",
        "transparente",
        0.6,
    ),
    tree(
        "Tipuana",
        12.0,
        12.0,
        3.0,
        0.6,
        [212, 196, 90],
        "follaje",
        "transparente",
        0.5,
    ),
    tree(
        "Plátano de sombra",
        18.0,
        14.0,
        3.5,
        0.7,
        [118, 156, 86],
        "ramas",
        "transparente",
        0.5,
    ),
    tree(
        "Almez",
        12.0,
        10.0,
        3.0,
        0.5,
        [104, 146, 84],
        "ramas",
        "transparente",
        0.55,
    ),
    tree(
        "Morera",
        8.0,
        9.0,
        2.2,
        0.5,
        [110, 158, 80],
        "lóbulos",
        "transparente",
        0.55,
    ),
    tree(
        "Ficus",
        12.0,
        14.0,
        2.5,
        0.9,
        [56, 100, 62],
        "lóbulos",
        "rellena",
        0.7,
    ),
    tree(
        "Magnolio",
        12.0,
        8.0,
        2.0,
        0.5,
        [64, 104, 70],
        "lóbulos",
        "rellena",
        0.75,
    ),
    tree(
        "Grevillea",
        14.0,
        7.0,
        3.0,
        0.45,
        [130, 150, 110],
        "follaje",
        "transparente",
        0.55,
    ),
    tree(
        "Sauce llorón",
        10.0,
        10.0,
        2.0,
        0.5,
        [150, 184, 108],
        "follaje",
        "transparente",
        0.5,
    ),
    tree(
        "Álamo blanco",
        18.0,
        9.0,
        3.5,
        0.6,
        [170, 192, 150],
        "lóbulos",
        "transparente",
        0.5,
    ),
    tree(
        "Chopo",
        20.0,
        6.0,
        3.0,
        0.5,
        [124, 162, 92],
        "círculo",
        "transparente",
        0.55,
    ),
    tree(
        "Fresno",
        15.0,
        10.0,
        3.0,
        0.5,
        [124, 164, 96],
        "ramas",
        "transparente",
        0.5,
    ),
    tree(
        "Olmo",
        16.0,
        12.0,
        3.0,
        0.6,
        [98, 140, 82],
        "lóbulos",
        "transparente",
        0.55,
    ),
    tree(
        "Abedul",
        14.0,
        6.0,
        2.5,
        0.3,
        [160, 190, 110],
        "follaje",
        "transparente",
        0.5,
    ),
    tree(
        "Arce rojo",
        12.0,
        8.0,
        2.5,
        0.4,
        [196, 84, 64],
        "follaje",
        "transparente",
        0.55,
    ),
    tree(
        "Liquidámbar",
        15.0,
        8.0,
        2.5,
        0.45,
        [214, 128, 70],
        "lóbulos",
        "transparente",
        0.55,
    ),
    tree(
        "Ginkgo",
        15.0,
        7.0,
        3.0,
        0.4,
        [226, 200, 92],
        "acuarela",
        "transparente",
        0.55,
    ),
    tree(
        "Castaño de Indias",
        18.0,
        12.0,
        3.0,
        0.7,
        [84, 128, 74],
        "lóbulos",
        "rellena",
        0.7,
    ),
    tree(
        "Tilo",
        16.0,
        11.0,
        3.0,
        0.6,
        [128, 168, 96],
        "lóbulos",
        "transparente",
        0.55,
    ),
    tree(
        "Drago",
        6.0,
        4.5,
        2.5,
        0.6,
        [90, 128, 96],
        "palmera",
        "rellena",
        0.8,
    ),
    tree(
        "Árbol joven",
        3.5,
        2.0,
        1.8,
        0.12,
        [132, 170, 100],
        "ramas",
        "transparente",
        0.45,
    ),
];

/// A shrub or bush: width and height in metres, leaf colour, fill.
pub struct Shrub {
    pub name: &'static str,
    pub width: f64,
    pub height: f64,
    pub leaves: [u8; 3],
    pub look: &'static str,
}

const fn shrub(
    name: &'static str,
    width: f64,
    height: f64,
    leaves: [u8; 3],
    look: &'static str,
) -> Shrub {
    Shrub {
        name,
        width,
        height,
        leaves,
        look,
    }
}

pub const SHRUBS: &[Shrub] = &[
    shrub("Adelfa", 2.0, 2.2, [96, 140, 88], "rellena"),
    shrub("Lentisco", 1.8, 1.5, [80, 116, 72], "rellena"),
    shrub("Romero", 0.9, 0.8, [120, 146, 120], "rellena"),
    shrub("Lavanda", 0.7, 0.6, [158, 140, 196], "rellena"),
    shrub("Pitosporo", 1.6, 1.8, [88, 132, 84], "rellena"),
    shrub("Boj", 0.8, 0.8, [70, 112, 66], "rellena"),
    shrub("Hibisco", 1.5, 1.8, [208, 96, 110], "rellena"),
    shrub("Buganvilla", 2.5, 2.0, [200, 80, 150], "transparente"),
    shrub("Agapanto", 0.6, 0.7, [120, 150, 200], "rellena"),
    shrub(
        "Gramínea ornamental",
        0.8,
        0.9,
        [196, 186, 128],
        "transparente",
    ),
    shrub("Aloe", 0.8, 0.6, [128, 160, 136], "rellena"),
    shrub("Pita", 2.0, 1.5, [140, 170, 150], "transparente"),
    shrub("Jazmín", 1.2, 1.5, [150, 186, 124], "transparente"),
    shrub("Arbusto grande", 3.0, 2.5, [100, 140, 90], "transparente"),
];

/// Other objects: a library entry by its family and the values that are
/// not the family's defaults (numbers; texts with `T`; colours with `M`,
/// a material made with that colour).
pub enum V {
    N(&'static str, f64),
    T(&'static str, &'static str),
    M(&'static str, &'static str, [u8; 3]),
}

pub struct Other {
    pub name: &'static str,
    pub family: &'static str,
    pub values: &'static [V],
}

const fn other(name: &'static str, family: &'static str, values: &'static [V]) -> Other {
    Other {
        name,
        family,
        values,
    }
}

pub const OTHERS: &[Other] = &[
    other(
        "Farola clásica 3,5 m",
        "Farola",
        &[
            V::T("model3d", "Fernandina"),
            V::N("height", 3.5),
            V::N("arm", 0.0),
            V::N("pole", 0.14),
            V::N("lamp", 0.4),
            V::N("glow", 3.0),
            V::M("mat", "Fundición negra", [40, 42, 46]),
        ],
    ),
    other(
        "Farola de vial 12 m",
        "Farola",
        &[
            V::N("height", 12.0),
            V::N("arm", 2.0),
            V::N("pole", 0.22),
            V::N("lamp", 0.8),
            V::N("glow", 9.0),
        ],
    ),
    other(
        "Farola de paseo 5 m",
        "Farola",
        &[V::N("height", 5.0), V::N("arm", 0.6), V::N("glow", 4.0)],
    ),
    other(
        "Baliza de suelo",
        "Farola",
        &[
            V::T("model3d", "Baliza"),
            V::N("height", 0.6),
            V::N("arm", 0.0),
            V::N("pole", 0.18),
            V::N("lamp", 0.18),
            V::N("glow", 1.0),
        ],
    ),
    other(
        "Banco de madera 2 m",
        "Banco",
        &[
            V::N("width", 2.0),
            V::M("mat", "Madera tratada", [150, 110, 72]),
        ],
    ),
    other(
        "Banco de hormigón 2,4 m",
        "Banco",
        &[
            V::T("model3d", "Hormigón y madera"),
            V::N("width", 2.4),
            V::N("depth", 0.6),
            V::M("mat", "Hormigón prefabricado", [196, 192, 184]),
        ],
    ),
    other(
        "Papelera de acero 60 l",
        "Papelera",
        &[V::N("width", 0.45), V::N("height", 0.85)],
    ),
    other(
        "Papelera de madera",
        "Papelera",
        &[
            V::T("model3d", "Listones de madera"),
            V::M("mat", "Madera tratada", [150, 110, 72]),
        ],
    ),
    other(
        "Bolardo de hormigón",
        "Bolardo",
        &[
            V::T("model3d", "Hormigón"),
            V::N("width", 0.35),
            V::N("height", 0.5),
            V::M("mat", "Hormigón prefabricado", [196, 192, 184]),
        ],
    ),
    other(
        "Bolardo de fundición",
        "Bolardo",
        &[
            V::N("width", 0.12),
            V::N("height", 0.9),
            V::M("mat", "Fundición negra", [40, 42, 46]),
        ],
    ),
    other(
        "Contenedor amarillo",
        "Contenedor de residuos",
        &[V::M("mat", "Plástico amarillo", [232, 196, 52])],
    ),
    other(
        "Contenedor azul",
        "Contenedor de residuos",
        &[V::M("mat", "Plástico azul", [52, 100, 170])],
    ),
    other(
        "Contenedor verde vidrio",
        "Contenedor de residuos",
        &[
            V::T("model3d", "Iglú de vidrio"),
            V::N("width", 1.2),
            V::N("depth", 1.2),
            V::N("height", 1.6),
            V::M("mat", "Plástico verde", [62, 112, 74]),
        ],
    ),
    other(
        "Contenedor gris",
        "Contenedor de residuos",
        &[V::M("mat", "Plástico gris", [110, 114, 118])],
    ),
    other(
        "Contenedor marrón orgánico",
        "Contenedor de residuos",
        &[V::M("mat", "Plástico marrón", [120, 84, 56])],
    ),
    other(
        "Señal grande 90",
        "Señal de tráfico",
        &[V::N("plate", 0.9), V::N("height", 2.9)],
    ),
    other(
        "Jardinera larga 3 m",
        "Jardinera",
        &[V::N("width", 3.0), V::N("depth", 0.7)],
    ),
    other(
        "Jardinera cuadrada 80",
        "Jardinera",
        &[V::N("width", 0.8), V::N("depth", 0.8), V::N("height", 0.6)],
    ),
    other("Alcorque 150", "Alcorque", &[V::N("width", 1.5)]),
    other(
        "Aparcabicis 10 plazas",
        "Aparcabicis",
        &[V::N("count", 10.0)],
    ),
    other(
        "Buzón grande",
        "Buzón",
        &[V::N("width", 0.6), V::N("depth", 0.45), V::N("height", 1.3)],
    ),
    other(
        "Fuente de beber de piedra",
        "Fuente de beber",
        &[V::M("mat", "Piedra caliza", [214, 204, 180])],
    ),
    other(
        "Seto 6 m",
        "Seto",
        &[V::N("width", 6.0), V::N("depth", 0.8), V::N("height", 1.5)],
    ),
    other(
        "Seto bajo 2 m",
        "Seto",
        &[V::N("width", 2.0), V::N("depth", 0.4), V::N("height", 0.5)],
    ),
    other("Adulto alto", "Persona", &[V::N("height", 1.85)]),
    other(
        "Adulta",
        "Persona",
        &[
            V::N("height", 1.65),
            V::M("mat", "Figura clara", [196, 172, 160]),
        ],
    ),
    other(
        "Niña",
        "Persona",
        &[
            V::N("height", 1.10),
            V::M("mat", "Figura clara", [196, 172, 160]),
        ],
    ),
    other(
        "Monovolumen",
        "Turismo",
        &[
            V::N("length", 4.7),
            V::N("width", 1.9),
            V::N("height", 1.75),
            V::M("body_mat", "Carrocería azul", [70, 96, 140]),
        ],
    ),
    other(
        "Utilitario",
        "Turismo",
        &[
            V::N("length", 3.8),
            V::N("width", 1.7),
            V::N("height", 1.5),
            V::M("body_mat", "Carrocería roja", [170, 64, 58]),
        ],
    ),
    other(
        "Coche blanco",
        "Turismo",
        &[V::M("body_mat", "Carrocería blanca", [232, 232, 228])],
    ),
    other(
        "Camión pequeño",
        "Turismo",
        &[
            V::N("length", 6.0),
            V::N("width", 2.2),
            V::N("height", 2.2),
            V::M("body_mat", "Carrocería blanca", [232, 232, 228]),
        ],
    ),
];

fn material(m: &mut Model, name: &str, category: &str, color: [u8; 3]) -> u64 {
    super::material_named(m, name, category, color, CutPattern::Solid).0
}

/// A `GlobalId` that is always the same for the same type.
pub(crate) fn stable_guid(name: &str) -> GlobalId {
    // FNV-1a, 128 bits.
    let mut h: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
    for b in format!("ArchiSim biblioteca ampliada · {name}").bytes() {
        h ^= b as u128;
        h = h.wrapping_mul(0x0000_0000_0100_0000_0000_0000_0000_013B);
    }
    GlobalId::from_u128(h)
}

/// Adds the extended library to a model: its materials and types. Returns
/// how many types.
pub fn add_extended_library(m: &mut Model) -> Result<usize, ModelError> {
    super::object_materials(m);
    let mut n = 0;
    let mut add = |m: &mut Model,
                   name: &str,
                   family: &str,
                   set: &dyn Fn(&mut Model, &mut archisim_core::Values)|
     -> Result<(), ModelError> {
        let f = m.system_family(family)?;
        let mut values = m.family_defaults(f, ParamScope::Type)?;
        set(m, &mut values);
        let t = m.add_object_type(name, f, values)?;
        m.set_object_type_guid(t, stable_guid(name))?;
        n += 1;
        Ok(())
    };
    for t in TREES {
        add(m, t.name, "Árbol", &|m, v| {
            v.insert("height".into(), Value::Number(t.height));
            v.insert("crown".into(), Value::Number(t.crown));
            v.insert("trunk_h".into(), Value::Number(t.trunk_h));
            v.insert("trunk".into(), Value::Number(t.trunk));
            let leaves = material(
                m,
                &format!("Copa de {}", t.name.to_lowercase()),
                "vegetation",
                t.leaves,
            );
            v.insert("leaves".into(), Value::Material(leaves));
            v.insert("shape".into(), Value::Text(t.shape.into()));
            v.insert("look".into(), Value::Text(t.look.into()));
            v.insert("opacity".into(), Value::Number(t.opacity));
        })?;
    }
    for s in SHRUBS {
        add(m, s.name, "Arbusto", &|m, v| {
            v.insert("width".into(), Value::Number(s.width));
            v.insert("height".into(), Value::Number(s.height));
            let leaves = material(
                m,
                &format!("Follaje de {}", s.name.to_lowercase()),
                "vegetation",
                s.leaves,
            );
            v.insert("leaves".into(), Value::Material(leaves));
            v.insert("look".into(), Value::Text(s.look.into()));
        })?;
    }
    for o in OTHERS {
        add(m, o.name, o.family, &|m, v| {
            for x in o.values {
                match x {
                    V::N(k, n) => {
                        v.insert((*k).into(), Value::Number(*n));
                    }
                    V::T(k, t) => {
                        v.insert((*k).into(), Value::Text((*t).into()));
                    }
                    V::M(k, name, c) => {
                        let id = material(m, name, "other", *c);
                        v.insert((*k).into(), Value::Material(id));
                    }
                }
            }
        })?;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_extended_library_is_valid_and_stable() {
        let mut m = Model::new("Ampliada");
        let level = m.add_level("PB", 0.0);
        let n = add_extended_library(&mut m).unwrap();
        assert_eq!(n, TREES.len() + SHRUBS.len() + OTHERS.len());
        assert!(n >= 80, "{n}");
        // Each type draws in plan and in presentation.
        let types: Vec<_> = m.object_types().map(|t| (t.id, t.guid.clone())).collect();
        for (t, _) in &types {
            let o = m
                .add_object(archisim_core::NewObject {
                    level,
                    object_type: *t,
                    position: archisim_core::Point2::new(0.0, 0.0),
                    rotation: 0.0,
                    base_offset: 0.0,
                })
                .unwrap();
            assert!(!m.object_plan_symbol(o, 50.0).unwrap().is_empty());
            if let Some(p) = m.object_presentation_symbol(o, 50.0) {
                assert!(!p.unwrap().is_empty());
            }
        }
        // The same GlobalIds every time.
        let mut again = Model::new("Otra vez");
        add_extended_library(&mut again).unwrap();
        let guids: Vec<_> = again.object_types().map(|t| t.guid.clone()).collect();
        assert_eq!(
            guids,
            types.iter().map(|(_, g)| g.clone()).collect::<Vec<_>>()
        );
    }
}
