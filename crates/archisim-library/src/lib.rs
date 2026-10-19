//! Starter library: the materials and types a new project begins with.
//!
//! This is ArchiSim's own "plantilla" (ArchiCAD template, Revit project
//! template): typical Spanish construction, written from scratch. Every new
//! project created in the application starts from [`new_project`], and the
//! demo house is built on the same library so both stay in step.

use archisim_core::{
    CutPattern, DoorOperation, Layer, LayerFunction, MaterialId, MemberTypeId, MemberUsage, Model,
    ModelError, NewOpeningType, OpeningKind, OpeningTypeId, Profile, SlabFunction, SlabTypeId,
    WallFunction, WallTypeId, WindowOperation,
};

/// Ids of the types added by [`add_library`], to place elements in code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Library {
    /// Double-leaf façade, 26.5 cm.
    pub facade: WallTypeId,
    /// Hollow-brick partition, 10 cm.
    pub partition: WallTypeId,
    /// One-foot load-bearing brick wall, 27 cm.
    pub bearing_wall: WallTypeId,
    pub ground_slab: SlabTypeId,
    pub floor_slab: SlabTypeId,
    pub flat_roof: SlabTypeId,
    /// Tiled pitched roof on an inclined concrete slab.
    pub pitched_roof: SlabTypeId,
    pub entrance_door: OpeningTypeId,
    pub interior_door: OpeningTypeId,
    pub double_door: OpeningTypeId,
    pub sliding_door: OpeningTypeId,
    pub window_two_leaves: OpeningTypeId,
    pub window_one_leaf: OpeningTypeId,
    pub balcony_door: OpeningTypeId,
    pub fixed_window: OpeningTypeId,
    /// A passage with no door ("hueco"), 90 × 210.
    pub void: OpeningTypeId,
    /// Reinforced concrete column, 30 × 30.
    pub column: MemberTypeId,
    /// Reinforced concrete downstand beam, 30 × 50.
    pub beam: MemberTypeId,
}

/// A new, empty project: one level at ±0.00 ("Planta baja") and the
/// starter library.
pub fn new_project(name: impl Into<String>) -> Model {
    let mut model = Model::new(name);
    model.add_level("Planta baja", 0.0);
    add_library(&mut model).expect("the starter library is always valid");
    model
}

/// Adds the starter materials and types to a model.
pub fn add_library(m: &mut Model) -> Result<Library, ModelError> {
    let mut mat = |name: &str, category: &str, color: [u8; 3], pattern| -> MaterialId {
        m.add_material(name, category, color, pattern)
    };
    let monocapa = mat(
        "Mortero monocapa",
        "plaster",
        [236, 232, 222],
        CutPattern::Dots,
    );
    let perforado = mat(
        "Ladrillo perforado",
        "brick",
        [232, 196, 178],
        CutPattern::Diagonal,
    );
    let lana = mat(
        "Lana mineral",
        "insulation",
        [250, 243, 205],
        CutPattern::Insulation,
    );
    let lhd = mat(
        "Ladrillo hueco doble",
        "brick",
        [240, 214, 200],
        CutPattern::Diagonal,
    );
    let yeso = mat(
        "Enlucido de yeso",
        "plaster",
        [245, 245, 245],
        CutPattern::Solid,
    );
    let baldosa = mat(
        "Baldosa cerámica",
        "ceramic",
        [214, 206, 190],
        CutPattern::Solid,
    );
    let mortero = mat(
        "Mortero de cemento",
        "mortar",
        [222, 222, 214],
        CutPattern::Dots,
    );
    let hormigon = mat(
        "Hormigón armado",
        "concrete",
        [205, 205, 205],
        CutPattern::Cross,
    );
    let lamina = mat(
        "Lámina impermeabilizante",
        "membrane",
        [70, 70, 70],
        CutPattern::Solid,
    );
    let grava = mat("Grava", "gravel", [210, 204, 190], CutPattern::Dots);
    let xps = mat(
        "Poliestireno extruido (XPS)",
        "insulation",
        [214, 236, 214],
        CutPattern::Insulation,
    );
    let celular = mat(
        "Hormigón celular",
        "concrete",
        [228, 224, 214],
        CutPattern::Dots,
    );
    let madera = mat("Madera de pino", "wood", [196, 160, 112], CutPattern::Solid);
    let aluminio = mat(
        "Aluminio lacado",
        "aluminium",
        [120, 124, 128],
        CutPattern::Solid,
    );
    let vidrio = mat("Vidrio doble", "glass", [190, 222, 236], CutPattern::Solid);
    let teja = mat(
        "Teja cerámica curva",
        "ceramic",
        [196, 104, 72],
        CutPattern::Solid,
    );

    let layer = |material, thickness, function| Layer {
        material,
        thickness,
        function,
    };

    let facade = m.add_wall_type(
        "Fachada doble hoja 26,5",
        WallFunction::Exterior,
        vec![
            layer(monocapa, 0.015, LayerFunction::Finish),
            layer(perforado, 0.115, LayerFunction::Structure),
            layer(lana, 0.050, LayerFunction::Insulation),
            layer(lhd, 0.070, LayerFunction::Substrate),
            layer(yeso, 0.015, LayerFunction::Finish),
        ],
    )?;
    let partition = m.add_wall_type(
        "Tabique LHD 10",
        WallFunction::Interior,
        vec![
            layer(yeso, 0.015, LayerFunction::Finish),
            layer(lhd, 0.070, LayerFunction::Substrate),
            layer(yeso, 0.015, LayerFunction::Finish),
        ],
    )?;
    let bearing_wall = m.add_wall_type(
        "Muro de carga ladrillo perforado 1 pie 27",
        WallFunction::Interior,
        vec![
            layer(yeso, 0.015, LayerFunction::Finish),
            layer(perforado, 0.240, LayerFunction::Structure),
            layer(yeso, 0.015, LayerFunction::Finish),
        ],
    )?;

    let ground_slab = m.add_slab_type(
        "Solera HA 15 con encachado",
        SlabFunction::BaseSlab,
        vec![
            layer(baldosa, 0.015, LayerFunction::Finish),
            layer(mortero, 0.030, LayerFunction::Substrate),
            layer(hormigon, 0.150, LayerFunction::Structure),
            layer(lamina, 0.002, LayerFunction::Membrane),
            layer(grava, 0.150, LayerFunction::Substrate),
        ],
    )?;
    let floor_slab = m.add_slab_type(
        "Forjado de planta HA 30",
        SlabFunction::Floor,
        vec![
            layer(baldosa, 0.015, LayerFunction::Finish),
            layer(mortero, 0.030, LayerFunction::Substrate),
            layer(hormigon, 0.300, LayerFunction::Structure),
            layer(yeso, 0.015, LayerFunction::Finish),
        ],
    )?;
    let flat_roof = m.add_slab_type(
        "Cubierta plana invertida con grava",
        SlabFunction::Roof,
        vec![
            layer(grava, 0.050, LayerFunction::Finish),
            layer(xps, 0.080, LayerFunction::Insulation),
            layer(lamina, 0.003, LayerFunction::Membrane),
            layer(celular, 0.100, LayerFunction::Substrate),
            layer(hormigon, 0.300, LayerFunction::Structure),
            layer(yeso, 0.015, LayerFunction::Finish),
        ],
    )?;
    let pitched_roof = m.add_slab_type(
        "Cubierta inclinada de teja sobre losa HA",
        SlabFunction::Roof,
        vec![
            layer(teja, 0.030, LayerFunction::Finish),
            layer(mortero, 0.020, LayerFunction::Substrate),
            layer(xps, 0.060, LayerFunction::Insulation),
            layer(hormigon, 0.180, LayerFunction::Structure),
            layer(yeso, 0.015, LayerFunction::Finish),
        ],
    )?;

    #[allow(clippy::too_many_arguments)]
    fn opening(
        m: &mut Model,
        name: &str,
        kind: OpeningKind,
        width: f64,
        height: f64,
        frame_width: f64,
        frame_depth: f64,
        frame_material: MaterialId,
        panel_material: MaterialId,
    ) -> Result<OpeningTypeId, ModelError> {
        m.add_opening_type(NewOpeningType {
            name: name.into(),
            kind,
            width,
            height,
            frame_width,
            frame_depth,
            frame_material,
            panel_material,
        })
    }
    use DoorOperation as D;
    use OpeningKind::{Door, Window};
    use WindowOperation as W;
    let entrance_door = opening(
        m,
        "Puerta de entrada 1 hoja 100×215",
        Door(D::SingleSwing),
        1.00,
        2.15,
        0.07,
        0.10,
        madera,
        madera,
    )?;
    let interior_door = opening(
        m,
        "Puerta de paso 1 hoja 80×205",
        Door(D::SingleSwing),
        0.80,
        2.05,
        0.05,
        0.07,
        madera,
        madera,
    )?;
    let double_door = opening(
        m,
        "Puerta de paso 2 hojas 140×205",
        Door(D::DoubleSwing),
        1.40,
        2.05,
        0.05,
        0.07,
        madera,
        madera,
    )?;
    let sliding_door = opening(
        m,
        "Puerta corredera 1 hoja 90×205",
        Door(D::Sliding),
        0.90,
        2.05,
        0.05,
        0.07,
        madera,
        madera,
    )?;
    let window_two_leaves = opening(
        m,
        "Ventana practicable 2 hojas 180×130",
        Window(W::DoubleCasement),
        1.80,
        1.30,
        0.06,
        0.07,
        aluminio,
        vidrio,
    )?;
    let window_one_leaf = opening(
        m,
        "Ventana practicable 1 hoja 100×120",
        Window(W::SingleCasement),
        1.00,
        1.20,
        0.06,
        0.07,
        aluminio,
        vidrio,
    )?;
    let balcony_door = opening(
        m,
        "Balconera corredera 2 hojas 240×220",
        Window(W::Sliding),
        2.40,
        2.20,
        0.06,
        0.07,
        aluminio,
        vidrio,
    )?;
    let fixed_window = opening(
        m,
        "Ventana fija 60×60",
        Window(W::Fixed),
        0.60,
        0.60,
        0.05,
        0.07,
        aluminio,
        vidrio,
    )?;

    let void = opening(
        m,
        "Hueco de paso 90×210",
        OpeningKind::Void,
        0.90,
        2.10,
        0.0,
        0.0,
        madera,
        madera,
    )?;

    // Structure (v0.25), added last so older ids stay put.
    let (column, beam) = add_structure_with(m, hormigon)?;

    Ok(Library {
        void,
        facade,
        partition,
        bearing_wall,
        ground_slab,
        floor_slab,
        flat_roof,
        pitched_roof,
        entrance_door,
        interior_door,
        double_door,
        sliding_door,
        window_two_leaves,
        window_one_leaf,
        balcony_door,
        fixed_window,
        column,
        beam,
    })
}

/// Adds the column and beam types of the library (and their steel and
/// timber materials) to a model that has none, e.g. an imported one.
/// Materials already in the model with the same name are reused.
/// Returns the default column and beam types.
pub fn add_structure(m: &mut Model) -> Result<(MemberTypeId, MemberTypeId), ModelError> {
    let concrete = material_named(
        m,
        "Hormigón armado",
        "concrete",
        [205, 205, 205],
        CutPattern::Cross,
    );
    add_structure_with(m, concrete)
}

fn material_named(
    m: &mut Model,
    name: &str,
    category: &str,
    color: [u8; 3],
    pattern: CutPattern,
) -> MaterialId {
    let found = m.materials().find(|x| x.name == name).map(|x| x.id);
    match found {
        Some(id) => id,
        None => m.add_material(name, category, color, pattern),
    }
}

fn add_structure_with(
    m: &mut Model,
    hormigon: MaterialId,
) -> Result<(MemberTypeId, MemberTypeId), ModelError> {
    let acero = material_named(m, "Acero S275", "steel", [120, 126, 134], CutPattern::Solid);
    let glulam = material_named(
        m,
        "Madera laminada GL24h",
        "wood",
        [214, 176, 120],
        CutPattern::Solid,
    );
    let rect = |width, depth| Profile::Rectangle { width, depth };
    let column = m.add_member_type(
        "Pilar HA 30×30",
        MemberUsage::Column,
        hormigon,
        rect(0.30, 0.30),
    )?;
    m.add_member_type(
        "Pilar HA 25×25",
        MemberUsage::Column,
        hormigon,
        rect(0.25, 0.25),
    )?;
    m.add_member_type(
        "Pilar HA 40×40",
        MemberUsage::Column,
        hormigon,
        rect(0.40, 0.40),
    )?;
    m.add_member_type(
        "Pilar HA Ø30",
        MemberUsage::Column,
        hormigon,
        Profile::Circle { diameter: 0.30 },
    )?;
    m.add_member_type(
        "Pilar HEB 160",
        MemberUsage::Column,
        acero,
        Profile::IShape {
            width: 0.160,
            depth: 0.160,
            web: 0.008,
            flange: 0.013,
        },
    )?;
    m.add_member_type(
        "Pilar madera GL24h 20×20",
        MemberUsage::Column,
        glulam,
        rect(0.20, 0.20),
    )?;
    let beam = m.add_member_type(
        "Viga HA 30×50",
        MemberUsage::Beam,
        hormigon,
        rect(0.30, 0.50),
    )?;
    m.add_member_type(
        "Viga plana HA 60×30",
        MemberUsage::Beam,
        hormigon,
        rect(0.60, 0.30),
    )?;
    m.add_member_type(
        "Viga IPE 300",
        MemberUsage::Beam,
        acero,
        Profile::IShape {
            width: 0.150,
            depth: 0.300,
            web: 0.0071,
            flange: 0.0107,
        },
    )?;
    m.add_member_type(
        "Viga madera GL24h 16×32",
        MemberUsage::Beam,
        glulam,
        rect(0.16, 0.32),
    )?;

    Ok((column, beam))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_project_has_a_level_and_the_library() {
        let m = new_project("Casa");
        assert_eq!(m.project_name, "Casa");
        assert_eq!(m.levels().len(), 1);
        assert_eq!(m.levels()[0].elevation, 0.0);
        assert_eq!(m.wall_types().count(), 3);
        assert_eq!(m.slab_types().count(), 4);
        assert_eq!(m.opening_types().filter(|t| t.is_door()).count(), 4);
        assert_eq!(m.opening_types().filter(|t| t.is_window()).count(), 4);
        assert_eq!(
            m.member_types()
                .filter(|t| t.usage == MemberUsage::Column)
                .count(),
            6
        );
        assert_eq!(
            m.member_types()
                .filter(|t| t.usage == MemberUsage::Beam)
                .count(),
            4
        );
        let facade = m
            .wall_types()
            .find(|t| t.name.starts_with("Fachada"))
            .unwrap();
        assert!((facade.thickness() - 0.265).abs() < 1e-12);
        assert_eq!(facade.function, WallFunction::Exterior);
    }
}
