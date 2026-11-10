//! The weights of the elements: the specific weight of each material
//! (DB-SE-AE, annex C, from its name or its kind) and the weight per m² of
//! the wall, floor and roof types, with their light frame if they have one.

use archisim_core::{FrameMaterial, Layer, LayerFunction, Material, Model};

use crate::actions::UseCategory;

/// Specific weight of a material, kN/m³ (annex C of DB-SE-AE), from its
/// name first and then its kind.
pub fn specific_weight(m: &Material) -> f64 {
    let name = m.name.to_lowercase();
    let has = |w: &[&str]| w.iter().any(|x| name.contains(x));
    if has(&["hormigón celular", "hormigon celular"]) {
        return 6.0;
    }
    if has(&["hormigón armado", "hormigon armado"]) {
        return 25.0;
    }
    if has(&["hormigón", "hormigon"]) {
        return 23.0;
    }
    if has(&["ladrillo hueco", "rasilla"]) {
        return 12.0;
    }
    if has(&["ladrillo perforado"]) {
        return 15.0;
    }
    if has(&["ladrillo macizo"]) {
        return 18.0;
    }
    if has(&["bloque de hormigón", "bloque de hormigon"]) {
        return 16.0;
    }
    if has(&["placa de yeso", "pladur"]) {
        return 9.0;
    }
    if has(&["yeso", "escayola"]) {
        return 12.0;
    }
    if has(&["mortero"]) {
        return 20.0;
    }
    if has(&["granito", "basalto"]) {
        return 28.0;
    }
    if has(&["mármol", "marmol", "caliza"]) {
        return 27.0;
    }
    // A composite slab (steel deck and concrete) drawn as one layer: its
    // weight spread over its depth (H90 + 30 in 120 mm, some 1,9 kN/m²).
    if has(&["colaborante"]) {
        return 16.0;
    }
    if has(&["acero"]) {
        return 78.5;
    }
    if has(&["aluminio"]) {
        return 27.0;
    }
    if has(&["xps", "eps", "poliestireno", "poliuretano"]) {
        return 0.4;
    }
    if has(&["lana mineral", "lana de roca", "lana de vidrio"]) {
        return 1.0;
    }
    if has(&["corcho"]) {
        return 1.5;
    }
    if has(&["osb", "contrachapado", "tablero"]) {
        return 6.0;
    }
    if has(&["clt", "contralaminada", "laminada"]) {
        return 5.0;
    }
    match m.category.as_str() {
        "concrete" => 24.0,
        "brick" | "masonry" => 15.0,
        "ceramic" | "tile" => 20.0,
        "stone" => 27.0,
        "mortar" | "plaster" | "render" => 18.0,
        "gravel" | "soil" | "earth" => 20.0,
        "asphalt" => 24.0,
        "steel" | "metal" => 78.5,
        "aluminium" => 27.0,
        "wood" | "timber" => 5.0,
        "glass" => 25.0,
        "insulation" => 0.5,
        "membrane" => 12.0,
        "air" | "gap" | "concept" | "vegetation" => 0.0,
        "water" => 10.0,
        _ => 15.0,
    }
}

/// A layer's weight, kN/m².
pub fn layer_weight(model: &Model, l: &Layer) -> f64 {
    if l.function == LayerFunction::AirGap {
        return 0.0;
    }
    model.material(l.material).map_or(15.0, specific_weight) * l.thickness
}

/// The weight per m² of a type's layers, kN/m². With a light frame, its
/// structure layer counts the frame (studs or joists at their spacing,
/// plus a tenth for tracks, noggins and blocking), the boards over it and
/// the insulation inside, in place of the layer's own material.
pub fn type_weight(model: &Model, type_id: u64, layers: &[Layer]) -> f64 {
    let frame = model.type_framing(type_id);
    let framed = frame.and(Model::frame_layer(layers));
    // A one-way or waffle floor weighs as its ribs, topping and blocks,
    // whatever its structure layer's material.
    let system = model.floor_system(archisim_core::SlabTypeId(type_id));
    let floor = system.and(Model::frame_layer(layers));
    let mut g = 0.0;
    for (i, l) in layers.iter().enumerate() {
        if Some(i) == floor {
            g += system.expect("floor").weight();
        } else if Some(i) == framed {
            let f = frame.expect("framed");
            let members = f.stud.kg_per_m() / f.spacing * 1.1 * 9.81 / 1000.0;
            let infill = 0.5 * l.thickness;
            let boards = f.sheathing.as_ref().map_or(0.0, |s| 6.0 * s.thickness);
            g += members + infill + boards;
        } else {
            g += layer_weight(model, l);
        }
    }
    g
}

/// The weight per m² of a wall type's face, kN/m².
pub fn wall_type_weight(model: &Model, id: archisim_core::WallTypeId) -> f64 {
    model
        .wall_type(id)
        .map_or(0.0, |t| type_weight(model, id.0, &t.layers))
}

/// The weight per m² of a slab (or roof) type, kN/m².
pub fn slab_type_weight(model: &Model, id: archisim_core::SlabTypeId) -> f64 {
    model
        .slab_type(id)
        .map_or(0.0, |t| type_weight(model, id.0, &t.layers))
}

/// Tiles and battens on a roof that has them, kN/m² (annex C: curved
/// ceramic tile).
pub const TILES: f64 = 0.5;

/// Light partitions not drawn, as a uniform load (DB-SE-AE 2.1.4): only
/// where the code allows it, and here only when the floor has no
/// partitions drawn on it.
pub const PARTITIONS: f64 = 1.0;

/// The live load of a roof by its pitch and how it is reached (table 3.1):
/// G1 under 20°, nothing over 40°, a straight line between; walkable flat
/// roofs take F.
pub fn roof_live(pitch_deg: f64, walkable: bool) -> (UseCategory, f64) {
    if walkable && pitch_deg < 5.0 {
        return (UseCategory::F, UseCategory::F.uniform());
    }
    let q = if pitch_deg <= 20.0 {
        1.0
    } else if pitch_deg >= 40.0 {
        0.0
    } else {
        (40.0 - pitch_deg) / 20.0
    };
    let cat = if pitch_deg >= 40.0 {
        UseCategory::G2
    } else {
        UseCategory::G1
    };
    (cat, q)
}

/// The weight of one member of a frame, kN/m.
pub fn member_weight(section: &archisim_core::FrameSection) -> f64 {
    section.kg_per_m() * 9.81 / 1000.0
}

/// The specific weight of a frame's material, kN/m³.
pub fn frame_material_weight(m: FrameMaterial) -> f64 {
    match m {
        FrameMaterial::Steel => 78.5,
        FrameMaterial::Timber => 4.2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roofs_lose_their_live_load_as_they_steepen() {
        assert_eq!(roof_live(10.0, false).1, 1.0);
        assert!((roof_live(30.0, false).1 - 0.5).abs() < 1e-9);
        assert_eq!(roof_live(45.0, false), (UseCategory::G2, 0.0));
        assert_eq!(roof_live(2.0, true).0, UseCategory::F);
    }

    #[test]
    fn the_template_types_weigh_what_they_should() {
        let mut m = archisim_core::Model::new("Pesos");
        let lib = archisim_library::add_library(&mut m).unwrap();
        for t in m.slab_types() {
            let g = slab_type_weight(&m, t.id);
            assert!(g.is_finite() && g >= 0.0, "{}: {g}", t.name);
        }
        // The concrete floor of the template weighs more than 2 kN/m², and
        // a one-foot brick wall about 4.
        let floor = slab_type_weight(&m, lib.floor_slab);
        assert!(floor > 2.0, "{floor}");
        let wall = wall_type_weight(&m, lib.bearing_wall);
        assert!((3.0..6.0).contains(&wall), "{wall}");
    }
}
