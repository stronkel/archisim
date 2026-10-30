//! Railing types of the template (E12): one per collection of the
//! «Barandilla» family, each with its materials, and a few more of the
//! usual ones (glass with clamps, perforated sheet, mesh).

use archisim_core::{Model, ModelError, ObjectTypeId, ParamScope, Value};

use crate::palette_material;

/// Name, collection, what is chosen apart from the collection, and the
/// structure, handrail and infill materials (by their palette names).
type Spec = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
    [&'static str; 3],
);

pub const RAILING_TYPES: &[Spec] = &[
    (
        "Barandilla · Nórdico cálido",
        "Nórdico cálido",
        &[],
        ["Roble claro", "Roble claro", "Roble claro"],
    ),
    (
        "Barandilla · Mediterráneo",
        "Mediterráneo",
        &[],
        ["Hierro forjado", "Hierro forjado", "Hierro forjado"],
    ),
    (
        "Barandilla · Años 50",
        "Años 50",
        &[],
        ["Metal negro", "Nogal", "Metal negro"],
    ),
    (
        "Barandilla · Japandi",
        "Japandi",
        &[],
        ["Roble claro", "Roble claro", "Roble claro"],
    ),
    (
        "Barandilla · Contemporáneo",
        "Contemporáneo",
        &[],
        ["Acero inoxidable", "Acero inoxidable", "Vidrio claro"],
    ),
    (
        "Barandilla · Rústica",
        "Rústica",
        &[],
        ["Roble ahumado", "Roble ahumado", "Roble ahumado"],
    ),
    (
        "Barandilla · Industrial",
        "Industrial",
        &[],
        ["Metal negro", "Metal negro", "Acero inoxidable"],
    ),
    (
        "Barandilla · Clásica",
        "Clásica",
        &[],
        ["Piedra caliza", "Piedra caliza", "Piedra caliza"],
    ),
    (
        "Barandilla de vidrio con pinzas",
        "Contemporáneo",
        &[
            ("rail_post", "Redondo"),
            ("rail_infill", "Vidrio con pinzas"),
        ],
        ["Acero inoxidable", "Acero inoxidable", "Vidrio claro"],
    ),
    (
        "Barandilla de chapa perforada",
        "Industrial",
        &[
            ("rail_post", "Cuadrado"),
            ("rail_top", "Rectangular"),
            ("rail_infill", "Chapa perforada"),
        ],
        ["Metal negro", "Metal negro", "Metal negro"],
    ),
    (
        "Barandilla de malla",
        "Industrial",
        &[("rail_infill", "Malla")],
        ["Metal negro", "Metal negro", "Acero inoxidable"],
    ),
];

/// Adds the railing types to the model (those it has by name are kept);
/// returns them in the order of [`RAILING_TYPES`].
pub fn add_railing_types(m: &mut Model) -> Result<Vec<ObjectTypeId>, ModelError> {
    let family = m.system_family("Barandilla")?;
    let hardware = palette_material(m, "Acero inoxidable");
    let mut out = Vec::new();
    for (name, collection, choices, [structure, rail, infill]) in RAILING_TYPES {
        if let Some(t) = m.object_types().find(|t| t.name == *name) {
            out.push(t.id);
            continue;
        }
        let mut values = m.family_defaults(family, ParamScope::Type)?;
        values.insert("model3d".into(), Value::Text((*collection).into()));
        for (k, v) in choices.iter() {
            values.insert((*k).into(), Value::Text((*v).into()));
        }
        for (key, material) in [
            ("structure_material", palette_material(m, structure)),
            ("rail_material", palette_material(m, rail)),
            ("infill_material", palette_material(m, infill)),
            ("hardware_material", hardware),
        ] {
            if let Some(id) = material {
                values.insert(key.into(), Value::Material(id.0));
            }
        }
        out.push(m.add_object_type(*name, family, values)?);
    }
    Ok(out)
}
