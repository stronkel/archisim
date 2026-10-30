//! Starter library: the materials and types a new project begins with.
//!
//! This is ArchiSim's own "plantilla" (ArchiCAD template, Revit project
//! template): typical Spanish construction, written from scratch. Every new
//! project created in the application starts from [`new_project`], and the
//! demo house is built on the same library so both stay in step.

use archisim_core::{
    CurtainCaps, CurtainDesign, CurtainDivision, CurtainJoint, CurtainLouvres, CurtainPanel,
    CurtainProfile, CurtainRest, CurtainTypeId, CutPattern, DoorOperation, Layer, LayerFunction,
    MaterialId, MemberTypeId, MemberUsage, Model, ModelError, NewOpeningType, ObjectTypeId,
    OpeningKind, OpeningTypeId, Profile, SlabFunction, SlabTypeId, WallFunction, WallTypeId,
    WindowOperation,
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
    /// Folding door of four leaves, 160 × 205.
    pub folding_door: OpeningTypeId,
    /// Pocket sliding door (into a frame in the wall), 80 × 205.
    pub pocket_door: OpeningTypeId,
    pub window_two_leaves: OpeningTypeId,
    pub window_one_leaf: OpeningTypeId,
    /// Two-leaf window with a roller shutter and its box, 120 × 150.
    pub window_shutter: OpeningTypeId,
    pub balcony_door: OpeningTypeId,
    pub fixed_window: OpeningTypeId,
    /// A passage with no door ("hueco"), 90 × 210.
    pub void: OpeningTypeId,
    /// Reinforced concrete column, 30 × 30.
    pub column: MemberTypeId,
    /// Reinforced concrete downstand beam, 30 × 50.
    pub beam: MemberTypeId,
    /// Stick curtain wall, bays up to 120 and rows up to 150.
    pub curtain: CurtainTypeId,
}

/// A new, empty project: one level at ±0.00 ("Planta baja") and the
/// starter library.
pub fn new_project(name: impl Into<String>) -> Model {
    let mut model = Model::new(name);
    model.add_level("Planta baja", 0.0);
    add_library(&mut model).expect("the starter library is always valid");
    model
}

/// The view templates a project starts with (Revit's): «AD5» (everything
/// in white with black lines, walls with their air gap only, the garden
/// and the site in pastel), «Albañilería» (walls, doors and windows and
/// dimensions; no furniture, garden or installations) and «Superficies»
/// (the rooms with their floors and areas, in AD5's look).
pub fn view_templates() -> Vec<archisim_core::ViewTemplate> {
    use archisim_core::{Category as C, ViewStyle, ViewTemplate, ViewTemplateId};
    let template = |name: &str, style: ViewStyle| ViewTemplate {
        id: ViewTemplateId(0),
        name: name.into(),
        style,
    };
    let hide = |mut s: ViewStyle, cs: &[C]| {
        s.hidden.extend(cs.iter().copied());
        s
    };
    vec![
        template("AD5", ViewStyle::ad5()),
        template(
            "Albañilería",
            hide(
                ViewStyle::default(),
                &[
                    C::Objects,
                    C::Vegetation,
                    C::Services,
                    C::Terrain,
                    C::Roads,
                    C::Plots,
                    C::Rooms,
                    C::Tags,
                ],
            ),
        ),
        template(
            "Superficies",
            hide(
                ViewStyle::ad5(),
                &[
                    C::Objects,
                    C::Vegetation,
                    C::Services,
                    C::Dimensions,
                    C::Tags,
                ],
            ),
        ),
    ]
}

/// The presentation styles a project starts with: the studio's («AD5»:
/// fine black lines, walls cut black, the drawing's own colours) and four
/// with a palette of their own, after presentation plans of other
/// studios (solid walls in one colour, soft tones, a tinted paper).
pub fn presentation_styles() -> Vec<archisim_core::PresentationStyle> {
    use archisim_core::PresentationStyle;
    let style = |name: &str, bg: [u8; 3], palette: &[[u8; 3]], poche: [u8; 3], lines: f64| {
        let mut s = PresentationStyle::new(name);
        s.background = bg;
        s.palette = palette.to_vec();
        s.poche = Some(poche);
        s.line_scale = lines;
        // Soft shadows on the coloured ones.
        s.shadows = !palette.is_empty() && name != "Monocromo";
        s
    };
    vec![
        style("AD5", [255, 255, 255], &[], [28, 28, 28], 0.7),
        style(
            "Terracota",
            [247, 242, 235],
            &[
                [74, 44, 34],
                [178, 96, 70],
                [226, 192, 164],
                [247, 242, 235],
            ],
            [176, 92, 66],
            0.8,
        ),
        style(
            "Bosque",
            [244, 242, 234],
            &[
                [34, 52, 44],
                [88, 122, 94],
                [172, 192, 162],
                [244, 242, 234],
            ],
            [47, 79, 62],
            0.8,
        ),
        style(
            "Azul",
            [245, 247, 250],
            &[
                [28, 40, 64],
                [72, 106, 160],
                [172, 192, 216],
                [245, 247, 250],
            ],
            [52, 84, 140],
            0.8,
        ),
        style(
            "Monocromo",
            [255, 255, 255],
            &[
                [24, 24, 24],
                [120, 120, 120],
                [205, 205, 205],
                [255, 255, 255],
            ],
            [64, 64, 64],
            0.9,
        ),
    ]
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
    // The volumes of the concept phase.
    mat(
        "Volumen de concepto",
        "concept",
        [150, 182, 222],
        CutPattern::Solid,
    );
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
    m.add_slab_type(
        "Losa de cimentación HA 50",
        SlabFunction::BaseSlab,
        vec![
            layer(hormigon, 0.500, LayerFunction::Structure),
            layer(hormigon, 0.100, LayerFunction::Substrate),
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

    // The materials the system families ask for by name that the
    // template does not have (the hardware's steel, a grille's iron): from
    // the palette, before any type is made, so that their parameters do
    // not fall back on the first material.
    family_materials(m);

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
    let folding_door = opening(
        m,
        "Puerta plegable 4 hojas 160×205",
        Door(D::Folding),
        1.60,
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
    let pocket_door = {
        let family = m.system_family(archisim_core::family::POCKET_DOOR)?;
        let mut values = m.family_defaults(family, archisim_core::ParamScope::Type)?;
        values.insert(
            "frame_material".into(),
            archisim_core::Value::Material(madera.0),
        );
        values.insert(
            "panel_material".into(),
            archisim_core::Value::Material(madera.0),
        );
        m.add_family_type(archisim_core::NewFamilyType {
            family,
            name: "Puerta corredera empotrada 80×205".into(),
            mark: None,
            description: String::new(),
            values,
        })?
    };
    let window_shutter = opening(
        m,
        "Ventana practicable 2 hojas con persiana 120×150",
        Window(W::DoubleCasement),
        1.20,
        1.50,
        0.06,
        0.07,
        aluminio,
        vidrio,
    )?;
    {
        let t = m
            .opening_type(window_shutter)
            .ok_or(ModelError::UnknownOpeningType(window_shutter))?;
        let (family, mut values) = (t.family, t.values.clone());
        values.insert("shutter".into(), archisim_core::Value::Bool(true));
        m.edit_family_type(window_shutter, family, values)?;
    }
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
    add_objects(m)?;
    railings::add_railing_types(m)?;
    let curtain = add_curtain_types(m)?;
    add_studio_types(m)?;

    for style in presentation_styles() {
        m.add_presentation_style(style)?;
    }
    // Finishes to paint volumes and faces with (v0.39), seen with their
    // texture in 3D, like the other materials seen on faces.
    for (name, category, color, pattern) in [
        (
            "Ladrillo visto",
            "brick",
            [176, 98, 72],
            CutPattern::Diagonal,
        ),
        ("Piedra", "stone", [196, 186, 166], CutPattern::Diagonal),
        (
            "Tarima de madera",
            "wood",
            [168, 122, 80],
            CutPattern::Solid,
        ),
        ("Césped", "vegetation", [110, 156, 78], CutPattern::Solid),
    ] {
        m.add_material(name, category, color, pattern);
    }
    add_appearances(m);
    // View templates (v0.42), last so older ids stay put.
    for t in view_templates() {
        m.add_view_template(t)?;
    }
    // Façades laid piece by piece (E9), last so older ids stay put.
    facades::add_facade_types(m)?;
    Ok(Library {
        curtain,
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
        folding_door,
        pocket_door,
        window_two_leaves,
        window_one_leaf,
        window_shutter,
        balcony_door,
        fixed_window,
        column,
        beam,
    })
}

/// Door and window types usual in a Spanish studio (v0.31, a first
/// proposal for AD5's catalogue): passage doors of 72,5 and 82,5 cm
/// leaves, an entrance door with a transom and subframe, a two-leaf door
/// with an active leaf, windows with subframe, sill and shutter, a
/// three-leaf sliding window and a casement window with a transom.
pub const STUDIO_TYPES: [StudioType; 16] = [
    (
        "Puerta de paso 72,5 (baño)",
        "Puerta abatible",
        &[("width", N(0.825)), ("height", N(2.10))],
    ),
    (
        "Puerta de paso 82,5",
        "Puerta abatible",
        &[("width", N(0.925)), ("height", N(2.10))],
    ),
    (
        "Puerta de entrada con tarja 100×250",
        "Puerta abatible",
        &[
            ("width", N(1.06)),
            ("height", N(2.53)),
            ("transom", B(true)),
            ("transom_height", N(0.40)),
            ("subframe", B(true)),
            ("frame_width", N(0.06)),
            ("frame_depth", N(0.10)),
        ],
    ),
    (
        "Puerta de 2 hojas desiguales 120×210",
        "Puerta abatible",
        &[
            ("width", N(1.30)),
            ("height", N(2.10)),
            ("leaves", N(2.0)),
            ("unequal", B(true)),
            ("active_width", N(0.825)),
        ],
    ),
    (
        "Ventana oscilobatiente 1 hoja 80×120 con premarco",
        "Ventana practicable",
        &[
            ("width", N(0.80)),
            ("height", N(1.20)),
            ("leaves", N(1.0)),
            ("leaf_1", T("oscilobatiente")),
            ("subframe", B(true)),
            ("sill", B(true)),
        ],
    ),
    (
        "Ventana practicable 2 hojas con persiana y vierteaguas 140×150",
        "Ventana practicable",
        &[
            ("width", N(1.40)),
            ("height", N(1.50)),
            ("shutter", B(true)),
            ("subframe", B(true)),
            ("sill", B(true)),
        ],
    ),
    (
        "Balconera practicable 2 hojas con persiana 140×240",
        "Ventana practicable",
        &[
            ("width", N(1.40)),
            ("height", N(2.40)),
            ("shutter", B(true)),
            ("subframe", B(true)),
        ],
    ),
    (
        "Ventana corredera 3 hojas 240×150 con premarco",
        "Ventana corredera",
        &[
            ("width", N(2.40)),
            ("height", N(1.50)),
            ("leaves", N(3.0)),
            ("subframe", B(true)),
            ("sill", B(true)),
        ],
    ),
    (
        "Ventana practicable 2 hojas con tarja 120×200",
        "Ventana practicable",
        &[
            ("width", N(1.20)),
            ("height", N(2.00)),
            ("transom", B(true)),
            ("transom_height", N(0.50)),
        ],
    ),
    // v0.33: the families asked for on 20/10.
    (
        "Puerta de entrada con fijo lateral 140×215",
        "Puerta abatible",
        &[
            ("width", N(1.40)),
            ("height", N(2.15)),
            ("side_left", B(true)),
            ("side_width", N(0.40)),
            ("side_base", N(0.40)),
            ("frame_width", N(0.06)),
            ("frame_depth", N(0.10)),
        ],
    ),
    (
        "Puerta pivotante de entrada 120×240",
        "Puerta pivotante",
        &[("width", N(1.20)), ("height", N(2.40)), ("pivot", N(0.25))],
    ),
    (
        "Puerta de garaje seccional 250×220",
        "Puerta de garaje",
        &[("width", N(2.50)), ("height", N(2.20))],
    ),
    (
        "Puerta de garaje enrollable 300×250",
        "Puerta de garaje",
        &[
            ("width", N(3.00)),
            ("height", N(2.50)),
            ("kind", T("enrollable")),
        ],
    ),
    (
        "Ventana de guillotina 90×150",
        "Ventana de guillotina",
        &[("width", N(0.90)), ("height", N(1.50))],
    ),
    (
        "Ventana pivotante 1 hoja 80×100",
        "Ventana practicable",
        &[
            ("width", N(0.80)),
            ("height", N(1.00)),
            ("leaves", N(1.0)),
            ("leaf_1", T("pivotante")),
        ],
    ),
    (
        "Ventana basculante 1 hoja 100×60",
        "Ventana practicable",
        &[
            ("width", N(1.00)),
            ("height", N(0.60)),
            ("leaves", N(1.0)),
            ("leaf_1", T("basculante")),
        ],
    ),
];

/// A type of [`STUDIO_TYPES`]: name, family and the values that differ
/// from the family's.
pub type StudioType = (
    &'static str,
    &'static str,
    &'static [(&'static str, StudioValue)],
);

/// A value of [`STUDIO_TYPES`].
#[derive(Debug, Clone, Copy)]
pub enum StudioValue {
    N(f64),
    B(bool),
    T(&'static str),
    /// A value chosen by name (a choice parameter).
    C(&'static str),
}
use StudioValue::{B, C, N, T};

pub mod collections;
pub mod extended;
pub mod facades;
pub mod flora;
pub mod ground;
pub mod imported;
pub mod railings;
pub mod workshop;

/// Adds the studio's usual door and window types, with the materials they
/// need (a subframe of galvanised steel, a stone sill).
pub fn add_studio_types(m: &mut Model) -> Result<Vec<OpeningTypeId>, ModelError> {
    for (name, category, color, pattern) in [
        (
            "Acero galvanizado",
            "metal",
            [170, 176, 180],
            CutPattern::Solid,
        ),
        ("Piedra natural", "stone", [226, 220, 204], CutPattern::Dots),
    ] {
        if !m.materials().any(|x| x.name == name) {
            m.add_material(name, category, color, pattern);
        }
    }
    let mut out = Vec::new();
    for (name, family, values) in STUDIO_TYPES {
        let family = m.system_family(family)?;
        let mut v = m.family_defaults(family, archisim_core::ParamScope::Type)?;
        for (k, x) in values {
            v.insert(
                (*k).into(),
                match x {
                    N(n) => archisim_core::Value::Number(*n),
                    B(b) => archisim_core::Value::Bool(*b),
                    T(t) | C(t) => archisim_core::Value::Text((*t).into()),
                },
            );
        }
        out.push(m.add_family_type(archisim_core::NewFamilyType {
            family,
            name: name.into(),
            mark: None,
            description: String::new(),
            values: v,
        })?);
    }
    Ok(out)
}

/// An object type of the library: its name, its family and the sizes
/// that differ from the family's.
pub type CatalogueEntry = (&'static str, &'static str, &'static [(&'static str, f64)]);

/// The object types of the library (v0.25, more in v0.26), in the order of
/// the gallery.
pub const OBJECT_CATALOGUE: [CatalogueEntry; 292] = [
    ("Mesa de comedor 160×90", "Mesa", &[]),
    ("Silla", "Silla", &[]),
    ("Sofá de 3 plazas", "Sofá", &[]),
    ("Sofá rinconera 260×170", "Sofá rinconera", &[]),
    (
        "Sofá rinconera 300×200",
        "Sofá rinconera",
        &[
            ("width", 3.0),
            ("length2", 2.0),
            ("places", 3.0),
            ("places2", 2.0),
        ],
    ),
    (
        "Sillón",
        "Sofá",
        &[("width", 1.0), ("places", 1.0), ("depth", 0.85)],
    ),
    ("Cama doble 150×200", "Cama", &[]),
    (
        "Cama individual 90×190",
        "Cama",
        &[("width", 0.9), ("depth", 1.9)],
    ),
    ("Armario 120", "Armario", &[]),
    ("Armario empotrado 180", "Armario empotrado", &[]),
    (
        "Armario empotrado corredero 240",
        "Armario empotrado",
        &[("width", 2.4), ("doors", 3.0)],
    ),
    ("Estantería 80", "Estantería", &[]),
    (
        "Librería 160",
        "Estantería",
        &[("width", 1.6), ("height", 2.2), ("shelves", 5.0)],
    ),
    ("Muebles de cocina 240", "Encimera de cocina", &[]),
    ("Muebles altos de cocina 120", "Mueble alto de cocina", &[]),
    // The modular kitchen (concepto de interfaz del 24/10/2026): modules
    // that join their worktops, each with its own drawers and fronts.
    ("Bajo cajonera 60", "Módulo bajo de cocina", &[]),
    (
        "Bajo cajonera 40",
        "Módulo bajo de cocina",
        &[("width", 0.4)],
    ),
    (
        "Bajo con puerta 45",
        "Módulo bajo de cocina",
        &[("width", 0.45)],
    ),
    (
        "Bajo con dos puertas 80",
        "Módulo bajo de cocina",
        &[("width", 0.8), ("doors", 2.0)],
    ),
    (
        "Bajo fregadero 80",
        "Módulo bajo de cocina",
        &[("width", 0.8), ("doors", 2.0)],
    ),
    ("Bajo horno 60", "Módulo bajo de cocina", &[]),
    ("Bajo lavavajillas 60", "Módulo bajo de cocina", &[]),
    (
        "Bajo placa 90",
        "Módulo bajo de cocina",
        &[("width", 0.9), ("drawers", 2.0)],
    ),
    ("Botellero 15", "Módulo bajo de cocina", &[("width", 0.15)]),
    (
        "Bajo de esquina 100",
        "Módulo bajo de cocina",
        &[("width", 1.0)],
    ),
    ("Columna despensa 60", "Columna de cocina", &[]),
    ("Columna horno y microondas 60", "Columna de cocina", &[]),
    ("Columna frigorífico 60", "Columna de cocina", &[]),
    (
        "Alto con puerta 40",
        "Mueble alto de cocina",
        &[("width", 0.4), ("doors", 1.0)],
    ),
    (
        "Alto vitrina 80",
        "Mueble alto de cocina",
        &[("width", 0.8), ("doors", 2.0)],
    ),
    ("Frigorífico 60", "Frigorífico", &[]),
    ("Lavadora 60", "Lavadora", &[]),
    ("Inodoro", "Inodoro", &[]),
    ("Bidé", "Bidé", &[]),
    ("Lavabo 60", "Lavabo", &[]),
    ("Plato de ducha 90×90", "Plato de ducha", &[]),
    (
        "Plato de ducha 120×80",
        "Plato de ducha",
        &[("width", 1.2), ("depth", 0.8)],
    ),
    ("Bañera 170×70", "Bañera", &[]),
    ("Árbol de hoja caduca 6 m", "Árbol", &[]),
    (
        "Árbol grande 12 m",
        "Árbol",
        &[
            ("height", 12.0),
            ("crown", 8.0),
            ("trunk_h", 3.5),
            ("trunk", 0.5),
        ],
    ),
    (
        "Palmera",
        "Árbol",
        &[
            ("height", 8.0),
            ("crown", 3.0),
            ("trunk_h", 6.5),
            ("trunk", 0.35),
        ],
    ),
    (
        "Árbol en flor 5 m",
        "Árbol",
        &[("height", 5.0), ("crown", 4.5), ("trunk_h", 1.8)],
    ),
    (
        "Árbol otoñal 8 m",
        "Árbol",
        &[
            ("height", 8.0),
            ("crown", 6.0),
            ("trunk_h", 2.5),
            ("trunk", 0.4),
        ],
    ),
    (
        "Pino 10 m",
        "Árbol",
        &[
            ("height", 10.0),
            ("crown", 5.0),
            ("trunk_h", 4.0),
            ("trunk", 0.4),
        ],
    ),
    (
        "Olivo 5 m",
        "Árbol",
        &[
            ("height", 5.0),
            ("crown", 5.0),
            ("trunk_h", 1.6),
            ("trunk", 0.45),
        ],
    ),
    (
        "Ciprés 8 m",
        "Árbol",
        &[
            ("height", 8.0),
            ("crown", 1.4),
            ("trunk_h", 0.6),
            ("trunk", 0.25),
        ],
    ),
    ("Arbusto 120", "Arbusto", &[]),
    ("Seto 3 m", "Seto", &[]),
    ("Banco con respaldo", "Banco", &[]),
    ("Persona", "Persona", &[]),
    ("Niño", "Persona", &[("height", 1.20)]),
    ("Turismo", "Turismo", &[]),
    (
        "Furgoneta",
        "Turismo",
        &[("length", 5.20), ("width", 2.0), ("height", 2.0)],
    ),
    ("Bicicleta", "Bicicleta", &[]),
    ("Farola 4,5 m", "Farola", &[]),
    (
        "Farola de vial 9 m",
        "Farola",
        &[
            ("height", 9.0),
            ("arm", 1.6),
            ("pole", 0.18),
            ("lamp", 0.7),
            ("glow", 7.0),
        ],
    ),
    (
        "Baliza de jardín",
        "Farola",
        &[
            ("height", 0.9),
            ("arm", 0.0),
            ("pole", 0.15),
            ("lamp", 0.15),
            ("glow", 1.2),
        ],
    ),
    ("Buzón", "Buzón", &[]),
    ("Papelera", "Papelera", &[]),
    ("Bolardo", "Bolardo", &[]),
    ("Fuente de beber", "Fuente de beber", &[]),
    ("Aparcabicis 5 plazas", "Aparcabicis", &[]),
    ("Contenedor de residuos", "Contenedor de residuos", &[]),
    ("Señal de tráfico", "Señal de tráfico", &[]),
    ("Jardinera 120", "Jardinera", &[]),
    ("Alcorque 120", "Alcorque", &[]),
    // More models of each object (v0.40), their 3D made by ArchiSim.
    ("Sofá clásico de 3 plazas", "Sofá", &[]),
    (
        "Sofá nórdico de 2 plazas",
        "Sofá",
        &[("width", 1.6), ("places", 2.0), ("depth", 0.85)],
    ),
    ("Silla metálica", "Silla", &[]),
    (
        "Silla tapizada",
        "Silla",
        &[("width", 0.48), ("depth", 0.55)],
    ),
    (
        "Mesa de caballetes 200×100",
        "Mesa",
        &[("width", 2.0), ("depth", 1.0), ("chairs", 8.0)],
    ),
    (
        "Mesa de patas metálicas 140×80",
        "Mesa",
        &[("width", 1.4), ("depth", 0.8), ("chairs", 4.0)],
    ),
    ("Mesa de centro", "Mesa de centro", &[]),
    (
        "Mesa de centro de vidrio",
        "Mesa de centro",
        &[("width", 1.2), ("depth", 0.7)],
    ),
    ("Mesita de noche", "Mesita de noche", &[]),
    ("Mueble TV 180", "Mueble TV", &[]),
    (
        "Cama de madera 160×200",
        "Cama",
        &[("width", 1.6), ("head", 0.9)],
    ),
    (
        "Cama tarima 150×200",
        "Cama",
        &[("height", 0.4), ("head", 0.6)],
    ),
    ("Inodoro suspendido", "Inodoro", &[("depth", 0.54)]),
    ("Bidé suspendido", "Bidé", &[]),
    (
        "Lavabo con mueble 80",
        "Lavabo",
        &[("width", 0.8), ("depth", 0.46)],
    ),
    ("Lavabo suspendido 60", "Lavabo", &[]),
    (
        "Ducha con mampara 120×80",
        "Plato de ducha",
        &[("width", 1.2), ("depth", 0.8)],
    ),
    // Bloque 5 de la revisión: drawers, desks and bathroom and kitchen
    // accessories.
    ("Cómoda 4 cajones", "Cómoda", &[]),
    (
        "Cómoda de patas 2×3 cajones",
        "Cómoda",
        &[("width", 1.2), ("drawers", 3.0), ("columns", 2.0)],
    ),
    ("Escritorio 140", "Escritorio", &[]),
    ("Escritorio metálico 120", "Escritorio", &[("width", 1.2)]),
    (
        "Encimera con cajonera 300",
        "Encimera de cocina",
        &[("width", 3.0), ("drawers", 3.0)],
    ),
    ("Espejo 80×100", "Espejo", &[]),
    (
        "Espejo redondo 60",
        "Espejo",
        &[("width", 0.6), ("height", 0.6), ("elevation", 1.2)],
    ),
    ("Toallero 60", "Toallero", &[]),
    (
        "Radiador toallero 50",
        "Toallero",
        &[("width", 0.5), ("elevation", 0.3)],
    ),
    ("Portarrollos", "Portarrollos", &[]),
    ("Barra de apoyo fija 70", "Barra de apoyo", &[]),
    (
        "Barra de apoyo abatible 75",
        "Barra de apoyo",
        &[("width", 0.75)],
    ),
    ("Campana decorativa 60", "Campana extractora", &[]),
    ("Campana integrada 60", "Campana extractora", &[]),
    ("Balda 80", "Balda", &[]),
    ("Mujer", "Persona", &[("height", 1.65)]),
    (
        "Todoterreno",
        "Turismo",
        &[("length", 4.6), ("width", 1.9), ("height", 1.7)],
    ),
    // The cosy pieces (v0.49): living room…
    (
        "Sofá acogedor de 3 plazas",
        "Sofá",
        &[("width", 2.2), ("depth", 0.95), ("pillows", 3.0)],
    ),
    (
        "Sofá nórdico con cojines",
        "Sofá",
        &[("width", 1.8), ("places", 2.0), ("pillows", 2.0)],
    ),
    ("Butaca de orejas", "Butaca", &[]),
    (
        "Butaca redonda",
        "Butaca",
        &[("width", 0.85), ("height", 0.8)],
    ),
    (
        "Butaca de ratán",
        "Butaca",
        &[("width", 0.75), ("height", 0.85)],
    ),
    (
        "Butaca baja de patas",
        "Butaca",
        &[
            ("width", 0.75),
            ("depth", 0.85),
            ("height", 0.8),
            ("seat", 0.38),
        ],
    ),
    ("Puf redondo", "Puf", &[]),
    ("Puf de punto", "Puf", &[("width", 0.55), ("height", 0.38)]),
    ("Puf cubo", "Puf", &[("width", 0.45), ("height", 0.42)]),
    ("Mesa auxiliar redonda", "Mesa auxiliar", &[]),
    (
        "Mesa auxiliar de tres patas",
        "Mesa auxiliar",
        &[("width", 0.45), ("height", 0.5)],
    ),
    (
        "Mesas nido",
        "Mesa auxiliar",
        &[("width", 0.55), ("depth", 0.4)],
    ),
    ("Aparador 160", "Aparador", &[]),
    (
        "Aparador de lamas 180",
        "Aparador",
        &[("width", 1.8), ("height", 0.7)],
    ),
    // …and what makes a house lived in.
    ("Alfombra 200×140", "Alfombra", &[]),
    (
        "Alfombra redonda 160",
        "Alfombra",
        &[("width", 1.6), ("depth", 1.6)],
    ),
    (
        "Alfombra de rayas 240×170",
        "Alfombra",
        &[("width", 2.4), ("depth", 1.7)],
    ),
    (
        "Alfombra bereber 300×200",
        "Alfombra",
        &[("width", 3.0), ("depth", 2.0)],
    ),
    ("Lámpara de pie", "Lámpara", &[]),
    (
        "Lámpara de mesa",
        "Lámpara",
        &[("height", 0.55), ("diameter", 0.32)],
    ),
    (
        "Lámpara colgante",
        "Lámpara",
        &[("height", 1.9), ("diameter", 0.42), ("drop", 0.6)],
    ),
    (
        "Lámpara de arco",
        "Lámpara",
        &[("height", 1.95), ("diameter", 0.38)],
    ),
    ("Costilla de Adán", "Planta de interior", &[]),
    (
        "Ficus en maceta",
        "Planta de interior",
        &[("height", 1.6), ("pot", 0.36)],
    ),
    (
        "Kentia",
        "Planta de interior",
        &[("height", 0.7), ("pot", 0.24)],
    ),
    (
        "Helecho",
        "Planta de interior",
        &[("height", 0.45), ("pot", 0.26)],
    ),
    (
        "Sansevieria",
        "Planta de interior",
        &[("height", 0.75), ("pot", 0.26)],
    ),
    ("Cortinas de lino 160", "Cortina", &[]),
    ("Visillo 160", "Cortina", &[("open", 0.0)]),
    (
        "Jarrón con ramas",
        "Decoración",
        &[("width", 0.22), ("height", 0.55)],
    ),
    ("Frutero", "Decoración", &[]),
    ("Libros y vela", "Decoración", &[("width", 0.26)]),
    ("Cerámicas", "Decoración", &[("width", 0.32)]),
    ("Tetera y cuencos", "Decoración", &[("width", 0.45)]),
    // The collections' pieces (v0.50): bedroom…
    (
        "Cama con cabecero tapizado 160",
        "Cama",
        &[("width", 1.6), ("head", 1.2), ("cushions", 2.0)],
    ),
    (
        "Cama de lamas 160",
        "Cama",
        &[
            ("width", 1.6),
            ("height", 0.4),
            ("head", 0.9),
            ("cushions", 1.0),
        ],
    ),
    ("Mesita de patas cónicas", "Mesita de noche", &[]),
    ("Mesita flotante", "Mesita de noche", &[("height", 0.5)]),
    // …kitchen…
    ("Taburete de madera", "Taburete", &[]),
    ("Taburete tapizado", "Taburete", &[("width", 0.42)]),
    ("Taburete de enea", "Taburete", &[("width", 0.36)]),
    ("Isla de cocina 180", "Isla de cocina", &[]),
    (
        "Mesa de trabajo de cocina 140",
        "Isla de cocina",
        &[("width", 1.4), ("depth", 0.7), ("overhang", 0.0)],
    ),
    (
        "Isla con fregadero 240",
        "Isla de cocina",
        &[("width", 2.4), ("depth", 1.0)],
    ),
    ("Tablas y aceite", "Cosas de la encimera", &[]),
    (
        "Botes de cristal",
        "Cosas de la encimera",
        &[("width", 0.45)],
    ),
    (
        "Cafetera y tazas",
        "Cosas de la encimera",
        &[("width", 0.4)],
    ),
    (
        "Hierbas aromáticas",
        "Cosas de la encimera",
        &[("width", 0.42)],
    ),
    // …bathroom…
    ("Toallas dobladas", "Toallas", &[]),
    ("Toallas colgadas", "Toallas", &[("width", 0.6)]),
    ("Toallas enrolladas", "Toallas", &[("width", 0.36)]),
    ("Cesto de ropa", "Cesta", &[]),
    (
        "Cesta con rollos",
        "Cesta",
        &[("width", 0.3), ("height", 0.18)],
    ),
    (
        "Cesta con asas",
        "Cesta",
        &[("width", 0.45), ("height", 0.3)],
    ),
    // …walls, floor cushions and blankets…
    (
        "Cuadro 50×70",
        "Cuadro o espejo",
        &[("width", 0.5), ("height", 0.7), ("elevation", 1.3)],
    ),
    (
        "Tríptico",
        "Cuadro o espejo",
        &[("width", 1.2), ("height", 0.5), ("elevation", 1.4)],
    ),
    (
        "Espejo de arco",
        "Cuadro o espejo",
        &[("width", 0.7), ("height", 1.5), ("elevation", 0.5)],
    ),
    (
        "Espejo sol",
        "Cuadro o espejo",
        &[("width", 0.7), ("height", 0.7), ("elevation", 1.3)],
    ),
    (
        "Reloj de pared",
        "Cuadro o espejo",
        &[("width", 0.35), ("height", 0.35), ("elevation", 1.8)],
    ),
    ("Cojín de suelo", "Cojín de suelo", &[]),
    (
        "Zafu",
        "Cojín de suelo",
        &[("width", 0.4), ("height", 0.15)],
    ),
    (
        "Pila de cojines de suelo",
        "Cojín de suelo",
        &[("width", 0.6), ("height", 0.12)],
    ),
    ("Manta doblada", "Manta", &[]),
    ("Pila de mantas", "Manta", &[("width", 0.5)]),
    ("Escalera con mantas", "Manta", &[]),
    // …and the garden and the terrace.
    ("Tumbona de teca", "Tumbona", &[]),
    ("Tumbona de aluminio", "Tumbona", &[("width", 0.66)]),
    (
        "Cama de exterior",
        "Tumbona",
        &[("width", 1.4), ("depth", 2.0)],
    ),
    ("Sombrilla redonda 250", "Sombrilla", &[]),
    (
        "Sombrilla descentrada 300",
        "Sombrilla",
        &[("width", 3.0), ("height", 2.5)],
    ),
    (
        "Sombrilla de paja 200",
        "Sombrilla",
        &[("width", 2.0), ("height", 2.3)],
    ),
    ("Macetón con olivo", "Macetón", &[]),
    (
        "Ánfora de barro",
        "Macetón",
        &[("width", 0.5), ("height", 0.9)],
    ),
    (
        "Macetón con gramíneas",
        "Macetón",
        &[("width", 0.5), ("height", 1.2)],
    ),
    (
        "Cuenco con lavanda",
        "Macetón",
        &[("width", 0.7), ("height", 0.6)],
    ),
    // Rocks and other natural things of the site (C8).
    ("Roca angulosa 80", "Roca", &[]),
    (
        "Roca angulosa 80 (2)",
        "Roca",
        &[
            ("variant", 2.0),
            ("width", 0.9),
            ("depth", 0.55),
            ("height", 0.45),
        ],
    ),
    (
        "Roca angulosa 80 (3)",
        "Roca",
        &[
            ("variant", 3.0),
            ("width", 0.7),
            ("depth", 0.65),
            ("height", 0.55),
        ],
    ),
    (
        "Canto rodado 50",
        "Roca",
        &[("width", 0.5), ("depth", 0.4), ("height", 0.3)],
    ),
    (
        "Canto rodado 50 (2)",
        "Roca",
        &[
            ("variant", 2.0),
            ("width", 0.55),
            ("depth", 0.35),
            ("height", 0.25),
        ],
    ),
    (
        "Laja 120",
        "Roca",
        &[("width", 1.2), ("depth", 0.8), ("height", 0.15)],
    ),
    (
        "Peña 2 m",
        "Roca",
        &[
            ("variant", 4.0),
            ("width", 2.0),
            ("depth", 1.6),
            ("height", 1.4),
        ],
    ),
    (
        "Roca con musgo 1 m",
        "Roca",
        &[
            ("variant", 5.0),
            ("width", 1.0),
            ("depth", 0.8),
            ("height", 0.6),
        ],
    ),
    ("Grupo de rocas", "Grupo de rocas", &[]),
    (
        "Grupo de cantos rodados",
        "Grupo de rocas",
        &[
            ("variant", 2.0),
            ("width", 1.6),
            ("depth", 1.2),
            ("height", 0.5),
        ],
    ),
    (
        "Rocas con musgo",
        "Grupo de rocas",
        &[
            ("variant", 3.0),
            ("width", 2.4),
            ("depth", 1.8),
            ("height", 1.1),
        ],
    ),
    ("Cantos rodados 1,2 m", "Cantos rodados", &[]),
    ("Tronco caído 2,4 m", "Tronco caído", &[]),
    ("Tocón 60", "Tocón", &[]),
    ("Pérgola de madera 3×3", "Pérgola", &[]),
    ("Pérgola de lamas 4×3", "Pérgola", &[("width", 4.0)]),
    ("Pérgola con cañizo 3×3", "Pérgola", &[]),
    // Street furniture in more models (v0.52)…
    (
        "Banco de hormigón y madera",
        "Banco",
        &[("width", 2.0), ("depth", 0.55)],
    ),
    ("Banco de acero sin respaldo", "Banco", &[("width", 1.6)]),
    (
        "Farola fernandina 3,5 m",
        "Farola",
        &[
            ("height", 3.5),
            ("arm", 0.0),
            ("pole", 0.14),
            ("lamp", 0.45),
            ("glow", 4.0),
        ],
    ),
    (
        "Farola de globo 4 m",
        "Farola",
        &[
            ("height", 4.0),
            ("arm", 0.0),
            ("pole", 0.12),
            ("lamp", 0.5),
            ("glow", 4.0),
        ],
    ),
    (
        "Papelera de listones",
        "Papelera",
        &[("width", 0.5), ("height", 0.85)],
    ),
    (
        "Papelera doble de reciclaje",
        "Papelera",
        &[("width", 0.7), ("height", 0.95)],
    ),
    (
        "Bolardo de acero",
        "Bolardo",
        &[("width", 0.12), ("height", 0.9)],
    ),
    (
        "Bolardo cónico de hormigón",
        "Bolardo",
        &[("width", 0.25), ("height", 0.55)],
    ),
    (
        "Buzón sobre poste",
        "Buzón",
        &[("width", 0.4), ("depth", 0.3), ("height", 1.2)],
    ),
    (
        "Buzones de portal",
        "Buzón",
        &[("width", 0.9), ("depth", 0.3), ("height", 1.5)],
    ),
    (
        "Iglú de vidrio",
        "Contenedor de residuos",
        &[("width", 1.4), ("depth", 1.4), ("height", 1.7)],
    ),
    (
        "Contenedor soterrado",
        "Contenedor de residuos",
        &[("width", 1.5), ("depth", 1.5), ("height", 1.2)],
    ),
    (
        "Fuente de piedra",
        "Fuente de beber",
        &[("width", 0.5), ("depth", 0.4), ("height", 0.95)],
    ),
    (
        "Estanque con surtidor",
        "Fuente de beber",
        &[("width", 3.0), ("depth", 3.0), ("height", 1.4)],
    ),
    (
        "Jardinera de madera con flores",
        "Jardinera",
        &[("width", 1.6), ("depth", 0.6), ("height", 0.55)],
    ),
    (
        "Jardinera de acero corten",
        "Jardinera",
        &[("width", 2.0), ("depth", 0.5), ("height", 0.6)],
    ),
    ("Alcorque de pletinas", "Alcorque", &[("width", 1.2)]),
    ("Alcorque de piedra y grava", "Alcorque", &[("width", 1.4)]),
    (
        "Aparcabicis de onda",
        "Aparcabicis",
        &[("count", 5.0), ("step", 0.7)],
    ),
    (
        "Aparcabicis de rueda",
        "Aparcabicis",
        &[("count", 6.0), ("step", 0.6), ("height", 0.6)],
    ),
    ("Señal de peligro", "Señal de tráfico", &[]),
    ("Panel informativo", "Señal de tráfico", &[("plate", 0.8)]),
    ("Bicicleta de carretera", "Bicicleta", &[("length", 1.7)]),
    ("Bicicleta de carga", "Bicicleta", &[("length", 2.4)]),
    // …and more of what a house has.
    ("Chimenea de obra", "Chimenea", &[]),
    (
        "Estufa de leña",
        "Chimenea",
        &[("width", 0.8), ("depth", 0.7)],
    ),
    (
        "Chimenea moderna lineal",
        "Chimenea",
        &[("width", 2.0), ("depth", 0.35)],
    ),
    ("Perchero de pie", "Perchero", &[]),
    ("Recibidor con banco", "Perchero", &[("width", 1.2)]),
    ("Silla de oficina", "Silla de oficina", &[]),
    (
        "Silla de oficina de malla",
        "Silla de oficina",
        &[("width", 0.66)],
    ),
    ("Mesa de bistró con dos sillas", "Mesa de exterior", &[]),
    (
        "Mesa de teca para seis",
        "Mesa de exterior",
        &[("width", 1.9), ("depth", 0.95)],
    ),
    (
        "Sofá de exterior con mesa",
        "Mesa de exterior",
        &[("width", 2.0)],
    ),
    ("Silla colgante de ratán", "Columpio y hamaca", &[]),
    (
        "Hamaca con bastidor",
        "Columpio y hamaca",
        &[("width", 3.0)],
    ),
    ("Radiador de aluminio 80", "Radiador", &[]),
    (
        "Radiador de panel 120",
        "Radiador",
        &[("width", 1.2), ("height", 0.6)],
    ),
    ("Aire acondicionado de pared", "Aire acondicionado", &[]),
    ("Ventilador de techo", "Ventilador de techo", &[]),
    ("Ventilador de techo de ratán", "Ventilador de techo", &[]),
    ("Piano de pared", "Piano", &[]),
    ("Piano de cola", "Piano", &[("width", 1.5)]),
    ("Cama infantil 90×190", "Cama infantil", &[]),
    ("Litera 90×190", "Litera", &[]),
    (
        "Estantería con objetos 120",
        "Estantería",
        &[("width", 1.2), ("height", 1.9), ("shelves", 3.0)],
    ),
    ("Vitrina 90", "Vitrina", &[]),
    (
        "Chimenea colgante",
        "Chimenea",
        &[("width", 1.2), ("depth", 0.6), ("height", 2.7)],
    ),
    // The workshop packages 2–8, approved by Antonio.
    ("Mesa de terraza", "Mesa de terraza", &[]),
    ("Silla de exterior", "Silla de exterior", &[]),
    ("Sofá de exterior", "Sofá de exterior", &[]),
    ("Banco de pie de cama", "Banco de pie de cama", &[]),
    ("Cuna", "Cuna", &[]),
    ("Espejo de pie", "Espejo de pie", &[]),
    ("Galán de noche", "Galán de noche", &[]),
    ("Tocador", "Tocador", &[]),
    ("Aplique de pared", "Aplique", &[]),
    ("Colgante de comedor", "Lámpara colgante", &[]),
    ("Lámpara de mesilla", "Lámpara de mesa", &[]),
    ("Lámpara de pie de lectura", "Lámpara de pie", &[]),
    ("Plafón de techo", "Plafón", &[]),
    ("Barra de bar", "Barra de bar", &[]),
    ("Contrabarra con botellas", "Contrabarra", &[]),
    ("Mesa de bar", "Mesa de bar", &[]),
    ("Silla de bar", "Silla de bar", &[]),
    ("Banco corrido", "Banco corrido", &[]),
    ("Puestos de trabajo", "Puestos de trabajo", &[]),
    ("Mesa de reuniones", "Mesa de reuniones", &[]),
    ("Mostrador de recepción", "Mostrador de recepción", &[]),
    ("Armario de oficina", "Armario de oficina", &[]),
    ("Mampara acústica", "Mampara acústica", &[]),
    ("Mostrador de tienda", "Mostrador de tienda", &[]),
    ("Estantería de tienda", "Estantería de tienda", &[]),
    ("Mesa expositora", "Mesa expositora", &[]),
    ("Burro de ropa", "Burro de ropa", &[]),
    ("Probador", "Probador", &[]),
    ("Piscina", "Piscina", &[]),
    ("Ducha de piscina", "Ducha de piscina", &[]),
    ("Canasta de baloncesto", "Canasta de baloncesto", &[]),
    ("Portería", "Portería", &[]),
    ("Mesa de ping-pong", "Mesa de ping-pong", &[]),
    ("Cama elástica", "Cama elástica", &[]),
    ("Cinta de correr", "Cinta de correr", &[]),
    ("Bicicleta estática", "Bicicleta estática", &[]),
    ("Banco de pesas", "Banco de pesas", &[]),
    ("Mancuernero", "Mancuernero", &[]),
    ("Espaldera", "Espaldera", &[]),
    ("Esterilla de yoga", "Esterilla de yoga", &[]),
    ("Pista de pádel", "Pista de pádel", &[]),
    ("Pista de tenis", "Pista de tenis", &[]),
    ("Pista de baloncesto", "Pista de baloncesto", &[]),
    ("Pista de fútbol sala", "Pista de fútbol sala", &[]),
    ("Campo de fútbol", "Campo de fútbol", &[]),
    ("Grada", "Grada", &[]),
    ("Columpio", "Columpio", &[]),
    ("Tobogán", "Tobogán", &[]),
    ("Balancín", "Balancín", &[]),
    ("Balancín de muelle", "Balancín de muelle", &[]),
    ("Estación de calistenia", "Estación de calistenia", &[]),
    ("Elíptica de parque", "Elíptica de parque", &[]),
];

/// Choices and materials of the library's types that are not the family's
/// (the cosy ones): by parameter, a yes/no or a material's name.
const LOOKS: &[(&str, &[(&str, StudioValue)])] = &[
    ("Armario empotrado corredero 240", &[("sliding", B(true))]),
    ("Peña 2 m", &[("mat", T("Roca granito"))]),
    (
        "Persona",
        &[
            ("mat", T("Pana azul petróleo")),
            ("mat2", T("Lino arena")),
            ("skin", T("Piel")),
            ("hair", T("Pelo castaño")),
        ],
    ),
    (
        "Mujer",
        &[
            ("mat", T("Lana teja")),
            ("mat2", T("Lana carbón")),
            ("skin", T("Piel morena")),
            ("hair", T("Pelo negro")),
        ],
    ),
    (
        "Niño",
        &[
            ("mat", T("Terciopelo mostaza")),
            ("mat2", T("Vaquero")),
            ("skin", T("Piel")),
            ("hair", T("Pelo castaño")),
        ],
    ),
    ("Turismo", &[("body_mat", T("Chapa salvia"))]),
    ("Todoterreno", &[("body_mat", T("Chapa crema"))]),
    ("Furgoneta", &[("body_mat", T("Chapa teja"))]),
    ("Bajo con puerta 45", &[("unit", C("Puertas"))]),
    ("Bajo con dos puertas 80", &[("unit", C("Puertas"))]),
    ("Bajo fregadero 80", &[("unit", C("Fregadero"))]),
    ("Bajo horno 60", &[("unit", C("Horno"))]),
    ("Bajo lavavajillas 60", &[("unit", C("Lavavajillas"))]),
    ("Bajo placa 90", &[("unit", C("Placa"))]),
    ("Botellero 15", &[("unit", C("Botellero"))]),
    ("Bajo de esquina 100", &[("unit", C("Esquina"))]),
    (
        "Columna horno y microondas 60",
        &[("tall", C("Horno y microondas"))],
    ),
    (
        "Columna frigorífico 60",
        &[("tall", C("Frigorífico integrado"))],
    ),
    (
        "Sofá acogedor de 3 plazas",
        &[
            ("mat", T("Lino arena")),
            ("cushion", T("Lino arena")),
            ("frame", T("Roble claro")),
            ("pillow", T("Terciopelo mostaza")),
            ("throw", B(true)),
        ],
    ),
    (
        "Sofá nórdico con cojines",
        &[
            ("mat", T("Terciopelo salvia")),
            ("cushion", T("Terciopelo salvia")),
            ("frame", T("Roble claro")),
            ("pillow", T("Lana teja")),
        ],
    ),
    (
        "Butaca redonda",
        &[("mat", T("Bouclé crudo")), ("cushion", T("Bouclé crudo"))],
    ),
    (
        "Butaca de ratán",
        &[("weave", T("Ratán")), ("cushion", T("Lino arena"))],
    ),
    (
        "Butaca baja de patas",
        &[
            ("mat", T("Lana teja")),
            ("cushion", T("Lana teja")),
            ("frame", T("Nogal")),
        ],
    ),
    ("Puf de punto", &[("mat", T("Lana teja"))]),
    ("Puf cubo", &[("mat", T("Terciopelo mostaza"))]),
    (
        "Mesa auxiliar de tres patas",
        &[("top", T("Nogal")), ("frame", T("Nogal"))],
    ),
    ("Mesas nido", &[("top", T("Nogal")), ("metal", T("Latón"))]),
    (
        "Aparador de lamas 180",
        &[("front", T("Nogal")), ("frame", T("Nogal"))],
    ),
    (
        "Alfombra redonda 160",
        &[("mat", T("Lana teja")), ("mat2", T("Lana cruda"))],
    ),
    (
        "Alfombra de rayas 240×170",
        &[("mat", T("Lana cruda")), ("mat2", T("Pana azul petróleo"))],
    ),
    ("Alfombra bereber 300×200", &[("mat2", T("Lana cruda"))]),
    ("Lámpara de mesa", &[("frame", T("Cerámica esmaltada"))]),
    ("Lámpara colgante", &[("shade", T("Ratán"))]),
    (
        "Lámpara de arco",
        &[("shade", T("Latón")), ("frame", T("Mármol"))],
    ),
    ("Cama con cabecero tapizado 160", &[("spread", B(true))]),
    ("Cama de lamas 160", &[("spread", B(true))]),
    ("Taburete de enea", &[("frame", T("Nogal"))]),
    (
        "Isla de cocina 180",
        &[("mat", T("Lacado salvia")), ("top", T("Mármol"))],
    ),
    (
        "Mesa de trabajo de cocina 140",
        &[("mat", T("Roble claro"))],
    ),
    (
        "Isla con fregadero 240",
        &[("mat", T("Roble claro")), ("top", T("Microcemento"))],
    ),
    ("Botes de cristal", &[("mat", T("Lino arena"))]),
    ("Cafetera y tazas", &[("mat", T("Cerámica esmaltada"))]),
    ("Toallas colgadas", &[("mat", T("Terciopelo salvia"))]),
    ("Toallas enrolladas", &[("mat2", T("Lino arena"))]),
    ("Cesta con rollos", &[("mat", T("Cerámica blanca"))]),
    ("Cesta con asas", &[("mat", T("Lino gris"))]),
    ("Zafu", &[("mat", T("Lana teja"))]),
    (
        "Pila de cojines de suelo",
        &[("mat2", T("Terciopelo salvia"))],
    ),
    ("Pila de mantas", &[("mat", T("Lana cruda"))]),
    ("Tumbona de teca", &[("pad", B(true))]),
    ("Señal de peligro", &[("face", T("Cerámica blanca"))]),
    (
        "Panel informativo",
        &[("face", T("Lino añil")), ("face2", T("Cerámica blanca"))],
    ),
    ("Bicicleta de carretera", &[("frame_mat", T("Lana teja"))]),
    (
        "Bicicleta de carga",
        &[("frame_mat", T("Terciopelo salvia")), ("frame", T("Teca"))],
    ),
    ("Chimenea moderna lineal", &[("mat", T("Microcemento"))]),
    ("Mesa de teca para seis", &[("frame", T("Teca"))]),
    (
        "Sofá de exterior con mesa",
        &[("mat", T("Teca")), ("frame", T("Teca"))],
    ),
    (
        "Hamaca con bastidor",
        &[("frame", T("Teca")), ("cushion", T("Lino añil"))],
    ),
    ("Banco de acero sin respaldo", &[("back", B(false))]),
];

/// The model of the 3D of the library's types that are not the family's
/// first (its «Modelo 3D»).
const MODELS: &[(&str, &str)] = &[
    ("Costilla de Adán", "Años 50"),
    ("Macetón con olivo", "Contemporáneo"),
    ("Farola de vial 9 m", "Industrial"),
    ("Sillón", "Contemporáneo"),
    ("Sofá clásico de 3 plazas", "Clásica"),
    ("Silla metálica", "Industrial"),
    ("Silla tapizada", "Contemporáneo"),
    ("Mesa de caballetes 200×100", "Rústica"),
    ("Mesa de patas metálicas 140×80", "Industrial"),
    ("Mesa de centro de vidrio", "Contemporáneo"),
    ("Cama de madera 160×200", "Rústica"),
    ("Cama tarima 150×200", "Japandi"),
    ("Inodoro suspendido", "Contemporáneo"),
    ("Bidé suspendido", "Contemporáneo"),
    ("Lavabo con mueble 80", "Nórdico cálido"),
    ("Lavabo suspendido 60", "Contemporáneo"),
    ("Ducha con mampara 120×80", "Japandi"),
    ("Mujer", "Mujer"),
    ("Cómoda de patas 2×3 cajones", "Años 50"),
    ("Escritorio metálico 120", "Industrial"),
    ("Espejo redondo 60", "Nórdico cálido"),
    ("Radiador toallero 50", "Clásica"),
    ("Barra de apoyo abatible 75", "Abatible"),
    ("Campana integrada 60", "Japandi"),
    ("Furgoneta", "Todoterreno"),
    ("Todoterreno", "Todoterreno"),
    ("Turismo", "Berlina"),
    ("Butaca de orejas", "Clásica"),
    ("Butaca de ratán", "Mediterráneo"),
    ("Butaca baja de patas", "Años 50"),
    ("Puf de punto", "Japandi"),
    ("Puf cubo", "Contemporáneo"),
    ("Mesa auxiliar de tres patas", "Años 50"),
    ("Mesas nido", "Contemporáneo"),
    ("Aparador de lamas 180", "Japandi"),
    ("Alfombra redonda 160", "Mediterráneo"),
    ("Alfombra de rayas 240×170", "Rústica"),
    ("Alfombra bereber 300×200", "Industrial"),
    ("Lámpara de mesa", "De mesa"),
    ("Lámpara colgante", "Colgante"),
    ("Lámpara de arco", "De arco"),
    ("Ficus en maceta", "Nórdico cálido"),
    ("Helecho", "Rústica"),
    ("Sansevieria", "Japandi"),
    ("Visillo 160", "Contemporáneo"),
    ("Frutero", "Años 50"),
    ("Libros y vela", "Industrial"),
    ("Cerámicas", "Mediterráneo"),
    ("Cama con cabecero tapizado 160", "Clásica"),
    ("Cama de lamas 160", "Años 50"),
    ("Mesita de patas cónicas", "Años 50"),
    ("Mesita flotante", "Japandi"),
    ("Taburete tapizado", "Años 50"),
    ("Taburete de enea", "Mediterráneo"),
    ("Mesa de trabajo de cocina 140", "Mesa de trabajo"),
    ("Isla con fregadero 240", "Con fregadero"),
    ("Botes de cristal", "Botes de cristal"),
    ("Cafetera y tazas", "Cafetera y tazas"),
    ("Hierbas aromáticas", "Hierbas aromáticas"),
    ("Toallas colgadas", "Colgadas"),
    ("Toallas enrolladas", "Enrolladas"),
    ("Cesta con rollos", "Con rollos"),
    ("Cesta con asas", "Con asas"),
    ("Tríptico", "Contemporáneo"),
    ("Espejo de arco", "Rústica"),
    ("Espejo sol", "Años 50"),
    ("Reloj de pared", "Industrial"),
    ("Zafu", "Zafu"),
    ("Pila de cojines de suelo", "Pila"),
    ("Pila de mantas", "Pila de mantas"),
    ("Escalera con mantas", "En escalera"),
    ("Tumbona de aluminio", "Contemporáneo"),
    ("Cama de exterior", "Japandi"),
    ("Sombrilla descentrada 300", "Contemporáneo"),
    ("Sombrilla de paja 200", "Mediterráneo"),
    ("Ánfora de barro", "Mediterráneo"),
    ("Canto rodado 50", "Canto rodado"),
    ("Canto rodado 50 (2)", "Canto rodado"),
    ("Laja 120", "Laja"),
    ("Roca con musgo 1 m", "Roca con musgo"),
    ("Grupo de cantos rodados", "Cantos rodados"),
    ("Rocas con musgo", "Con musgo"),
    ("Macetón con gramíneas", "Nórdico cálido"),
    ("Cuenco con lavanda", "Años 50"),
    ("Pérgola de lamas 4×3", "Contemporáneo"),
    ("Pérgola con cañizo 3×3", "Mediterráneo"),
    ("Banco de hormigón y madera", "Contemporáneo"),
    ("Banco de acero sin respaldo", "Industrial"),
    ("Farola fernandina 3,5 m", "Clásica"),
    ("Farola de globo 4 m", "Años 50"),
    ("Baliza de jardín", "Contemporáneo"),
    ("Papelera de listones", "Japandi"),
    ("Papelera doble de reciclaje", "Industrial"),
    ("Bolardo de acero", "Contemporáneo"),
    ("Bolardo cónico de hormigón", "Rústica"),
    ("Buzón sobre poste", "Sobre poste"),
    ("Buzones de portal", "Buzones de portal"),
    ("Iglú de vidrio", "Iglú de vidrio"),
    ("Contenedor soterrado", "Soterrado"),
    ("Fuente de piedra", "Rústica"),
    ("Estanque con surtidor", "Años 50"),
    ("Jardinera de madera con flores", "Rústica"),
    ("Jardinera de acero corten", "Industrial"),
    ("Alcorque de pletinas", "Pletinas"),
    ("Alcorque de piedra y grava", "Bordillo y grava"),
    ("Aparcabicis de onda", "Contemporáneo"),
    ("Aparcabicis de rueda", "Japandi"),
    ("Señal de peligro", "Triangular"),
    ("Panel informativo", "Rectangular"),
    ("Bicicleta de carretera", "Carretera"),
    ("Bicicleta de carga", "De carga"),
    ("Chimenea de obra", "Mediterráneo"),
    ("Chimenea moderna lineal", "Contemporáneo"),
    ("Chimenea colgante", "Colgante"),
    ("Recibidor con banco", "Recibidor con banco"),
    ("Silla de oficina de malla", "Contemporáneo"),
    ("Mesa de teca para seis", "Teca para seis"),
    ("Sofá de exterior con mesa", "Sofá de exterior"),
    ("Hamaca con bastidor", "Hamaca"),
    ("Radiador de panel 120", "Panel de acero"),
    ("Ventilador de techo de ratán", "Aspas de ratán"),
    ("Piano de cola", "De cola"),
    ("Estantería con objetos 120", "Contemporáneo"),
    ("Inodoro", "Mediterráneo"),
    ("Bidé", "Mediterráneo"),
    ("Lavabo 60", "Años 50"),
    ("Espejo 80×100", "Rústica"),
    ("Toallero 60", "Contemporáneo"),
    ("Kentia", "Clásica"),
    ("Tetera y cuencos", "Japandi"),
    ("Bolardo", "Clásica"),
    ("Fuente de beber", "Clásica"),
    ("Jardinera 120", "Contemporáneo"),
];

/// The leaves of each species (a material of [`object_materials`]) and
/// how the canopy is drawn in presentation plans, where it is not the
/// family's own.
const SPECIES: &[(&str, &str, &str, &str)] = &[
    ("Árbol grande 12 m", "Vegetación", "ramas", "transparente"),
    ("Palmera", "Copa clara", "palmera", "transparente"),
    (
        "Árbol en flor 5 m",
        "Copa en flor",
        "acuarela",
        "transparente",
    ),
    ("Árbol otoñal 8 m", "Copa otoñal", "follaje", "transparente"),
    ("Pino 10 m", "Copa oscura", "conífera", "rellena"),
    ("Olivo 5 m", "Copa olivo", "follaje", "transparente"),
    ("Ciprés 8 m", "Copa oscura", "círculo", "rellena"),
];

/// How the starter materials look in 3D: the texture of what they are
/// (bricks, tiles, concrete, boards…), for the ones seen on faces.
pub fn add_appearances(m: &mut Model) {
    use archisim_core::{Appearance, TextureKind};
    let looks: Vec<(MaterialId, TextureKind)> = m
        .materials()
        .filter(|x| m.appearance(x.id).is_plain())
        .filter_map(|x| {
            // The furniture's fabrics, woods, fibres and stones.
            if COZY_PALETTE.iter().any(|e| e.0 == x.name) {
                return TextureKind::suggested(&x.name, &x.category).map(|t| (x.id, t));
            }
            let t = match (x.name.as_str(), x.category.as_str()) {
                ("Ladrillo visto", _) => TextureKind::Brick,
                ("Tarima de madera", _) => TextureKind::Planks,
                ("Césped", _) => TextureKind::Grass,
                ("Mortero monocapa" | "Enlucido de yeso", _) => TextureKind::Plaster,
                (_, "ceramic") if x.name.contains("Baldosa") => TextureKind::Tile,
                (_, "concrete") => TextureKind::Concrete,
                (_, "stone") => TextureKind::Stone,
                (_, "gravel") => TextureKind::Gravel,
                (_, "water") => TextureKind::Water,
                (_, "wood") => TextureKind::Wood,
                _ => return None,
            };
            Some((x.id, t))
        })
        .collect();
    for (id, t) in looks {
        let _ = m.set_appearance(
            id,
            Appearance {
                color: None,
                texture: Some(t),
                scale: (t == TextureKind::Tile).then_some(0.33),
                growth: None,
            },
        );
    }
}

/// The materials the library's objects ask for by name, added if the
/// project has none of that name.
fn object_materials(m: &mut Model) {
    for (name, category, color) in [
        ("Metal lacado", "metal", [86, 92, 100]),
        ("Hormigón prefabricado", "concrete", [196, 192, 184]),
        ("Plástico verde", "plastic", [62, 112, 74]),
        ("Agua", "water", [128, 192, 222]),
        ("Luz", "other", [255, 226, 150]),
        ("Copa clara", "vegetation", [176, 204, 120]),
        ("Copa oscura", "vegetation", [70, 112, 82]),
        ("Copa otoñal", "vegetation", [214, 150, 74]),
        ("Copa en flor", "vegetation", [226, 160, 182]),
        ("Copa olivo", "vegetation", [150, 164, 122]),
        ("Porcelana", "ceramic", [244, 244, 240]),
        ("Madera de pino", "wood", [196, 160, 112]),
        ("Vegetación", "vegetation", [126, 164, 98]),
        ("Corteza", "wood", [112, 86, 62]),
        ("Roca caliza", "plaster", [178, 172, 160]),
        ("Roca granito", "plaster", [140, 137, 132]),
        ("Musgo", "vegetation", [98, 124, 58]),
        ("Madera de tronco", "wood", [204, 168, 118]),
        ("Figura", "other", [150, 158, 172]),
        ("Carrocería", "metal", [164, 172, 184]),
        ("Neumático", "rubber", [52, 52, 56]),
        // The parts of the models ArchiSim makes (v0.40).
        ("Tela", "fabric", [146, 152, 160]),
        ("Tela clara", "fabric", [236, 232, 224]),
        ("Edredón", "fabric", [200, 210, 218]),
        ("Cromado", "metal", [204, 208, 212]),
        ("Negro mate", "other", [40, 40, 44]),
        ("Libros", "other", [150, 62, 54]),
        ("Libros claros", "other", [222, 198, 144]),
        ("Encimera", "stone", [226, 224, 218]),
        ("Piloto", "other", [196, 40, 32]),
        ("Vidrio doble", "glass", [190, 222, 236]),
    ] {
        material_named(m, name, category, color, CutPattern::Solid);
    }
    for (name, category, color) in COZY_PALETTE {
        material_named(m, name, category, *color, CutPattern::Solid);
    }
}

/// The cosy palette (v0.49): warm fabrics, woods, metals and ceramics, the
/// colours the Maqueta offers for each piece of a piece of furniture.
pub const COZY_PALETTE: &[(&str, &str, [u8; 3])] = &[
    ("Lino arena", "fabric", [214, 198, 172]),
    ("Bouclé crudo", "fabric", [236, 228, 212]),
    ("Lana cruda", "fabric", [226, 216, 196]),
    ("Lana teja", "fabric", [190, 104, 72]),
    ("Terciopelo salvia", "fabric", [150, 168, 136]),
    ("Terciopelo mostaza", "fabric", [214, 160, 58]),
    ("Terciopelo rosa", "fabric", [214, 160, 156]),
    ("Pana azul petróleo", "fabric", [52, 96, 108]),
    ("Lino gris", "fabric", [168, 166, 160]),
    ("Pantalla de lino", "fabric", [244, 232, 206]),
    ("Visillo", "glass", [246, 244, 238]),
    ("Roble claro", "wood", [206, 170, 122]),
    ("Nogal", "wood", [120, 82, 56]),
    ("Ratán", "wood", [200, 160, 104]),
    ("Latón", "metal", [196, 160, 88]),
    ("Mármol", "stone", [234, 232, 226]),
    ("Terracota", "ceramic", [196, 112, 78]),
    ("Cerámica esmaltada", "ceramic", [92, 128, 150]),
    ("Cerámica blanca", "ceramic", [240, 238, 232]),
    ("Hoja verde", "vegetation", [76, 132, 76]),
    ("Tierra", "other", [94, 70, 52]),
    ("Naranja", "other", [236, 140, 40]),
    ("Manzana verde", "other", [150, 190, 70]),
    ("Cera", "other", [246, 240, 226]),
    // People and cars in the same style (C1).
    ("Piel", "other", [226, 188, 154]),
    ("Piel morena", "other", [172, 124, 92]),
    ("Pelo castaño", "other", [98, 70, 48]),
    ("Pelo negro", "other", [46, 42, 40]),
    ("Vaquero", "fabric", [88, 114, 150]),
    ("Chapa salvia", "metal", [146, 170, 146]),
    ("Chapa crema", "metal", [232, 220, 194]),
    ("Chapa teja", "metal", [184, 96, 70]),
    // The collections' (v0.50): Mediterranean…
    ("Lino blanco", "fabric", [242, 238, 228]),
    ("Lino añil", "fabric", [58, 86, 140]),
    ("Madera encalada", "wood", [224, 214, 196]),
    ("Hierro forjado", "metal", [54, 52, 50]),
    ("Microcemento", "stone", [212, 204, 190]),
    ("Enea", "wood", [196, 166, 106]),
    // …fifties…
    ("Teca", "wood", [150, 98, 60]),
    ("Lana oliva", "fabric", [130, 128, 74]),
    ("Cerámica turquesa", "ceramic", [72, 150, 150]),
    ("Terrazo", "stone", [228, 220, 206]),
    // …japandi…
    ("Roble ahumado", "wood", [106, 84, 66]),
    ("Lino piedra", "fabric", [196, 188, 172]),
    ("Lana carbón", "fabric", [72, 72, 74]),
    ("Metal negro", "metal", [46, 46, 48]),
    ("Gres", "ceramic", [178, 166, 150]),
    ("Piedra gris", "stone", [152, 150, 144]),
    ("Bambú", "wood", [206, 180, 126]),
    // …and what the new pieces are made of.
    ("Toalla blanca", "fabric", [242, 240, 234]),
    ("Tela de exterior", "fabric", [232, 226, 212]),
    ("Paja", "wood", [214, 184, 124]),
    ("Cañizo", "wood", [204, 178, 122]),
    ("Espejo", "metal", [206, 214, 220]),
    ("Aceite", "other", [196, 170, 60]),
    ("Lavanda", "vegetation", [150, 120, 190]),
    ("Lacado salvia", "wood", [164, 180, 150]),
    // The workshop's (M1): plants, a sink and its tap, a lamp post.
    ("Hoja verde tierna", "vegetation", [122, 166, 84]),
    ("Tallo verde", "vegetation", [104, 140, 76]),
    ("Pétalo blanco", "other", [244, 242, 232]),
    ("Espádice", "other", [232, 214, 152]),
    ("Acero inoxidable", "metal", [192, 194, 194]),
    ("Fundición", "metal", [50, 54, 56]),
    ("Cristal esmerilado", "other", [250, 234, 196]),
    // The new collections (paquete 1a): contemporary, rustic, industrial
    // and classic.
    ("Tela gris", "fabric", [150, 150, 148]),
    ("Pino viejo", "wood", [178, 132, 84]),
    ("Corteza de pino", "wood", [104, 80, 58]),
    // The terraces' plants (paquete 5a).
    ("Hoja de olivo", "vegetation", [126, 146, 104]),
    ("Envés de olivo", "vegetation", [178, 188, 162]),
    ("Corteza de olivo", "wood", [112, 102, 88]),
    ("Gramínea", "vegetation", [158, 170, 104]),
    ("Espiga", "vegetation", [214, 196, 150]),
    ("Hoja de lavanda", "vegetation", [150, 168, 140]),
    ("Corteza de buganvilla", "wood", [96, 80, 62]),
    ("Hoja de buganvilla", "vegetation", [64, 118, 58]),
    ("Buganvilla", "vegetation", [200, 44, 128]),
    // The street furniture's (paquete 5, tanda 5b).
    ("Acero corten", "metal", [150, 84, 52]),
    ("Luz de farola", "other", [255, 226, 150]),
    ("Agua de fuente", "water", [128, 192, 222]),
    // The house plants and decoration's (paquete 6, tanda 6a).
    ("Limón", "other", [238, 208, 62]),
    ("Libro crema", "other", [232, 222, 200]),
    ("Libro azul", "other", [62, 84, 122]),
    ("Libro teja", "other", [172, 92, 64]),
    ("Llama", "other", [255, 214, 140]),
    ("Papel", "other", [242, 238, 228]),
    ("Oro viejo", "metal", [186, 152, 82]),
    // The bars' (paquete 7, tanda 7a).
    ("Vidrio verde botella", "glass", [64, 104, 70]),
    ("Vidrio ámbar", "glass", [164, 98, 38]),
    ("Luz cálida", "other", [255, 220, 160]),
    // The offices' (paquete 7, tanda 7b).
    ("Fieltro gris", "fabric", [150, 152, 150]),
    ("Fieltro salvia", "fabric", [148, 164, 140]),
    ("Fieltro teja", "fabric", [176, 112, 86]),
    ("Archivador azul", "other", [56, 86, 140]),
    ("Archivador rojo", "other", [160, 56, 50]),
    ("Archivador blanco", "other", [236, 234, 226]),
    // The shops' (paquete 7, tanda 7c).
    ("Caja kraft", "other", [196, 160, 112]),
    ("Caja salvia", "other", [130, 152, 124]),
    // The pools' (paquete 8, tanda 8a).
    ("Gresite blanco", "stone", [222, 232, 234]),
    ("Gresite azul", "stone", [70, 140, 186]),
    ("Gresite verde", "stone", [104, 150, 140]),
    ("Pizarra", "stone", [62, 66, 70]),
    ("Luz de piscina", "other", [196, 232, 255]),
    ("Goma negra", "other", [38, 38, 40]),
    ("Red blanca", "fabric", [236, 236, 230]),
    ("Red negra", "fabric", [40, 40, 42]),
    ("Línea blanca", "other", [244, 244, 240]),
    ("Pintura roja", "other", [185, 45, 40]),
    ("Tablero azul", "other", [30, 75, 130]),
    ("Tablero verde", "other", [35, 100, 65]),
    ("Acero gris", "metal", [140, 144, 148]),
    ("Acero galvanizado", "metal", [170, 175, 178]),
    ("Lona negra", "fabric", [32, 32, 34]),
    ("Plástico azul", "other", [45, 115, 195]),
    ("Plástico gris oscuro", "other", [72, 74, 78]),
    ("Polipiel negra", "other", [30, 30, 32]),
    ("Caucho salvia", "other", [140, 162, 136]),
    ("Caucho teja", "other", [192, 112, 82]),
    ("Corcho", "other", [196, 156, 108]),
    ("Plástico rojo", "other", [200, 50, 45]),
    ("Plástico amarillo", "other", [235, 190, 45]),
    ("Plástico verde", "other", [70, 150, 80]),
    ("Caucho de seguridad", "other", [178, 88, 64]),
    ("Césped artificial", "other", [72, 142, 72]),
    ("Pista azul", "other", [42, 92, 150]),
    ("Pista verde", "other", [52, 112, 82]),
    ("Pista terracota", "other", [186, 96, 66]),
    ("Pintura verde", "other", [55, 115, 70]),
    ("Pintura amarilla", "other", [230, 180, 40]),
    ("Luz de foco", "other", [250, 248, 235]),
    ("Césped natural", "vegetation", [72, 136, 64]),
    ("Césped natural claro", "vegetation", [94, 158, 80]),
    ("Vidrio claro", "glass", [206, 224, 228]),
    ("Lacado blanco", "wood", [234, 230, 222]),
    ("Lacado arena", "wood", [214, 200, 178]),
    ("Fresno oscuro", "wood", [92, 70, 54]),
    ("Lomo granate", "other", [128, 50, 46]),
    ("Lomo azul", "other", [54, 80, 108]),
    ("Lomo ocre", "other", [196, 152, 76]),
    ("Lomo verde", "other", [86, 106, 80]),
    ("Lomo crema", "other", [226, 214, 190]),
    ("Pantalla de televisor", "other", [22, 24, 28]),
    ("Cal blanca", "stone", [238, 234, 226]),
    ("Piedra caliza", "stone", [198, 184, 156]),
    ("Ladrillo visto", "brick", [176, 98, 72]),
    ("Llama", "other", [255, 140, 40]),
    ("Llama clara", "other", [255, 214, 120]),
    ("Lino crudo", "fabric", [222, 208, 184]),
    ("Cuero coñac", "other", [146, 84, 48]),
    // The kitchens (paquete 2a).
    ("Vitrocerámica", "other", [30, 30, 33]),
    ("Granito negro", "stone", [44, 44, 46]),
    ("Hormigón pulido", "stone", [150, 147, 140]),
    ("Lacado azul celeste", "wood", [150, 194, 204]),
    ("Lacado azul añil", "wood", [64, 94, 140]),
    ("Lacado marfil", "wood", [232, 223, 202]),
    ("Terciopelo verde botella", "fabric", [44, 82, 64]),
    // The lamps of the collections (paquete 4).
    ("Papel de arroz", "other", [246, 240, 226]),
];

/// A material of the cosy palette, added to the project if it has none of
/// that name.
pub fn palette_material(m: &mut Model, name: &str) -> Option<MaterialId> {
    let (name, category, color) = COZY_PALETTE.iter().find(|e| e.0 == name)?;
    let id = material_named(m, name, category, *color, CutPattern::Solid);
    if m.appearance(id).is_plain() {
        if let Some(t) = archisim_core::TextureKind::suggested(name, category) {
            let _ = m.set_appearance(
                id,
                archisim_core::Appearance {
                    color: None,
                    texture: Some(t),
                    scale: None,
                    growth: None,
                },
            );
        }
    }
    Some(id)
}

/// Adds from the palette each material the doors' and windows' system
/// families name as a default (`material = "…"`) that the project does not have yet.
pub fn family_materials(m: &mut Model) {
    for (_, source) in archisim_core::family::system_families() {
        for line in source.lines() {
            let Some(at) = line.find(" material = \"") else {
                continue;
            };
            let rest = &line[at + " material = \"".len()..];
            let Some(end) = rest.find('"') else {
                continue;
            };
            let name = &rest[..end];
            if !m.materials().any(|x| x.name == name) {
                palette_material(m, name);
            }
        }
    }
}

/// A material like `base` in any colour (the free colour wheel of the
/// Maqueta): same kind and texture (wood stays grain, fabric weave), its
/// colour `rgb`. Named «<base> #RRGGBB»: the same colour again finds it.
pub fn colour_material(m: &mut Model, base: MaterialId, rgb: [u8; 3]) -> Option<MaterialId> {
    let b = m.material(base)?.clone();
    // The base name without a colour it was given before.
    let stem = match b.name.rsplit_once(" #") {
        Some((s, hex)) if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) => s,
        _ => b.name.as_str(),
    };
    let name = format!("{stem} #{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
    let id = material_named(m, &name, &b.category, rgb, b.cut_pattern);
    let look = m.appearance(base);
    let _ = m.set_appearance(
        id,
        archisim_core::Appearance {
            color: Some(rgb),
            texture: look.texture,
            scale: look.scale,
            growth: look.growth,
        },
    );
    Some(id)
}

/// Adds one object type of the library (by its name in
/// [`OBJECT_CATALOGUE`]), or finds the project's type of that name.
pub fn add_object_entry(m: &mut Model, name: &str) -> Result<ObjectTypeId, ModelError> {
    if let Some(t) = m.object_types().find(|t| t.name == name) {
        return Ok(t.id);
    }
    let Some((f, values)) = entry_values(m, name)? else {
        // A species of the plants ArchiSim grows.
        return flora::add_flora_entry(m, name)?
            .ok_or_else(|| ModelError::NotAnObject(name.to_string()));
    };
    m.add_object_type(name, f, values)
}

/// The family and values of an object type of the library (by its name in
/// [`OBJECT_CATALOGUE`]), its materials added to the project; `None` if
/// the catalogue has no type of that name.
pub(crate) fn entry_values(
    m: &mut Model,
    name: &str,
) -> Result<Option<(archisim_core::FamilyId, archisim_core::Values)>, ModelError> {
    use archisim_core::{ParamScope, Value};
    let Some((name, family, sizes)) = OBJECT_CATALOGUE.iter().find(|e| e.0 == name) else {
        return Ok(None);
    };
    object_materials(m);
    let f = m.system_family(family)?;
    let mut values = m.family_defaults(f, ParamScope::Type)?;
    for (k, v) in sizes.iter() {
        values.insert((*k).into(), Value::Number(*v));
    }
    if let Some((_, model)) = MODELS.iter().find(|e| e.0 == *name) {
        values.insert("model3d".into(), Value::Text((*model).into()));
    }
    if let Some((_, looks)) = LOOKS.iter().find(|e| e.0 == *name) {
        for (k, v) in looks.iter() {
            let v = match v {
                N(n) => Value::Number(*n),
                B(b) => Value::Bool(*b),
                // A material, by its name.
                T(t) => match m.materials().find(|x| x.name == *t) {
                    Some(mat) => Value::Material(mat.id.0),
                    None => continue,
                },
                C(t) => Value::Text((*t).into()),
            };
            values.insert((*k).into(), v);
        }
    }
    if let Some((_, leaves, shape, look)) = SPECIES.iter().find(|e| e.0 == *name) {
        if let Some(mat) = m.materials().find(|x| x.name == *leaves) {
            values.insert("leaves".into(), Value::Material(mat.id.0));
        }
        values.insert("shape".into(), Value::Text((*shape).into()));
        values.insert("look".into(), Value::Text((*look).into()));
    }
    Ok(Some((f, values)))
}

/// Whether a type of the catalogue chooses its own model of the 3D (and
/// a collection should not change it).
pub(crate) fn entry_has_model(name: &str) -> bool {
    MODELS.iter().any(|e| e.0 == name)
}

/// Adds the object types of the library: furniture, sanitary ware,
/// kitchen units, vegetation and street furniture, people and vehicles,
/// each from its system family with these sizes.
pub fn add_objects(m: &mut Model) -> Result<Vec<(String, ObjectTypeId)>, ModelError> {
    let mut out = Vec::new();
    for (name, _, _) in OBJECT_CATALOGUE {
        out.push((name.to_string(), add_object_entry(m, name)?));
    }
    Ok(out)
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

/// Adds the curtain wall types of the library (and their materials, those
/// of the same name reused). Returns the plain stick curtain wall.
pub fn add_curtain_types(m: &mut Model) -> Result<CurtainTypeId, ModelError> {
    let aluminium = material_named(
        m,
        "Aluminio lacado",
        "aluminium",
        [120, 124, 128],
        CutPattern::Solid,
    );
    let glass = material_named(
        m,
        "Vidrio doble",
        "glass",
        [190, 222, 236],
        CutPattern::Solid,
    );
    let spandrel = material_named(
        m,
        "Panel sándwich lacado",
        "metal",
        [96, 100, 106],
        CutPattern::Solid,
    );
    let profile = |width, depth| CurtainProfile {
        width,
        depth,
        material: aluminium,
    };
    let glazing = |thickness| CurtainPanel {
        name: "Vidrio".into(),
        thickness,
        material: glass,
    };
    let stick = CurtainDesign {
        columns: CurtainDivision::max_spacing(1.2),
        rows: CurtainDivision::max_spacing(1.5),
        border: profile(0.06, 0.15),
        mullion: profile(0.05, 0.15),
        transom: profile(0.05, 0.12),
        joint: CurtainJoint::Mullions,
        panels: vec![glazing(0.024)],
        row_panels: Vec::new(),
        caps: None,
        louvres: None,
    };
    let plain = m.add_curtain_type("Montante y travesaño 120 × 150", stick.clone())?;
    m.add_curtain_type(
        "Fachada con antepecho opaco",
        CurtainDesign {
            rows: CurtainDivision {
                fixed: vec![1.0],
                rest: CurtainRest::MaxSpacing(2.4),
            },
            panels: vec![
                glazing(0.024),
                CurtainPanel {
                    name: "Antepecho".into(),
                    thickness: 0.06,
                    material: spandrel,
                },
            ],
            row_panels: vec![1, 0],
            ..stick.clone()
        },
    )?;
    m.add_curtain_type(
        "Con tapajuntas 150",
        CurtainDesign {
            columns: CurtainDivision::max_spacing(1.5),
            caps: Some(CurtainCaps {
                width: 0.06,
                depth: 0.025,
                material: aluminium,
            }),
            ..stick.clone()
        },
    )?;
    m.add_curtain_type(
        "Ventanal corrido",
        CurtainDesign {
            columns: CurtainDivision::max_spacing(1.8),
            rows: CurtainDivision {
                fixed: Vec::new(),
                rest: CurtainRest::Count(1),
            },
            border: profile(0.07, 0.12),
            mullion: profile(0.06, 0.12),
            transom: profile(0.06, 0.12),
            joint: CurtainJoint::Transoms,
            ..stick.clone()
        },
    )?;
    m.add_curtain_type(
        "Con lamas horizontales",
        CurtainDesign {
            columns: CurtainDivision::max_spacing(1.5),
            louvres: Some(CurtainLouvres {
                depth: 0.2,
                thickness: 0.03,
                spacing: 0.35,
                offset: 0.1,
                material: aluminium,
            }),
            ..stick
        },
    )?;
    Ok(plain)
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
        "Pilar tubo 100×100×5",
        MemberUsage::Column,
        acero,
        Profile::RectHollow {
            width: 0.100,
            depth: 0.100,
            wall: 0.005,
        },
    )?;
    m.add_member_type(
        "Pilar tubo Ø114,3×5",
        MemberUsage::Column,
        acero,
        Profile::CircleHollow {
            diameter: 0.1143,
            wall: 0.005,
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
        "Viga UPN 200",
        MemberUsage::Beam,
        acero,
        Profile::UShape {
            width: 0.075,
            depth: 0.200,
            web: 0.0085,
            flange: 0.0115,
        },
    )?;
    m.add_member_type(
        "Viga T 100",
        MemberUsage::Beam,
        acero,
        Profile::TShape {
            width: 0.100,
            depth: 0.100,
            web: 0.011,
            flange: 0.011,
        },
    )?;
    m.add_member_type(
        "Angular L 80×8",
        MemberUsage::Beam,
        acero,
        Profile::LShape {
            width: 0.080,
            depth: 0.080,
            thickness: 0.008,
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

    /// Every piece of the catalogue whose model ArchiSim makes has it,
    /// with each role in a material of the project.
    #[test]
    fn every_piece_has_its_model() {
        let m = new_project("Prueba");
        for (name, _, _) in OBJECT_CATALOGUE {
            let t = m
                .object_types()
                .find(|t| t.name == name)
                .unwrap_or_else(|| panic!("{name}"));
            if !m.type_has_prop(t.id) {
                continue;
            }
            let (mesh, parts) = m
                .type_prop(t.id)
                .unwrap_or_else(|| panic!("{name}: no model"));
            assert!(mesh.triangle_count() > 20, "{name}");
            assert!(!parts.is_empty(), "{name}");
        }
    }

    #[test]
    fn cosy_pieces_have_their_models_and_materials() {
        let m = new_project("Prueba");
        for name in [
            "Sofá acogedor de 3 plazas",
            "Butaca de ratán",
            "Puf de punto",
            "Mesas nido",
            "Aparador 160",
            "Alfombra bereber 300×200",
            "Lámpara de pie",
            "Lámpara colgante",
            "Costilla de Adán",
            "Cortinas de lino 160",
            "Libros y vela",
        ] {
            let t = m
                .object_types()
                .find(|t| t.name == name)
                .unwrap_or_else(|| panic!("{name}"));
            let (mesh, parts) = m
                .type_prop(t.id)
                .unwrap_or_else(|| panic!("{name}: no model"));
            assert!(mesh.triangle_count() > 50, "{name}");
            assert!(!parts.is_empty(), "{name}");
        }
        // A plant's leaves are green (the costilla de Adán of the años 50,
        // in its cone pot on legs).
        let t = m
            .object_types()
            .find(|t| t.name == "Costilla de Adán")
            .unwrap();
        let (_, parts) = m.type_prop(t.id).unwrap();
        let names: Vec<&str> = parts
            .iter()
            .filter_map(|(id, _)| m.material(*id).map(|x| x.name.as_str()))
            .collect();
        assert!(
            names.contains(&"Hoja verde") && names.contains(&"Metal negro"),
            "{names:?}"
        );
        // The cosy sofa: linen, mustard cushions and a throw.
        let t = m
            .object_types()
            .find(|t| t.name == "Sofá acogedor de 3 plazas")
            .unwrap();
        let (_, parts) = m.type_prop(t.id).unwrap();
        let names: Vec<&str> = parts
            .iter()
            .filter_map(|(id, _)| m.material(*id).map(|x| x.name.as_str()))
            .collect();
        assert!(
            names.contains(&"Terciopelo mostaza") && names.contains(&"Lana teja"),
            "{names:?}"
        );
    }

    #[test]
    fn new_project_has_a_level_and_the_library() {
        let m = new_project("Casa");
        assert_eq!(m.project_name, "Casa");
        assert_eq!(m.levels().len(), 1);
        assert_eq!(m.levels()[0].elevation, 0.0);
        // The three of always and the eight façades laid piece by piece (E9).
        assert_eq!(m.wall_types().count(), 3 + facades::FACADE_TYPES.len());
        assert_eq!(m.slab_types().count(), 5);
        // With the studio's: 8 doors and 8 windows more.
        assert_eq!(m.opening_types().filter(|t| t.is_door()).count(), 14);
        assert_eq!(m.opening_types().filter(|t| t.is_window()).count(), 13);
        let entrance = m
            .opening_types()
            .find(|t| t.name == "Puerta de entrada con tarja 100×250")
            .unwrap();
        // The subframe narrows the clear width: 3 cm + 6 cm each side.
        assert!((entrance.frame_width - 0.09).abs() < 1e-12);
        assert_eq!(
            m.member_types()
                .filter(|t| t.usage == MemberUsage::Column)
                .count(),
            8
        );
        assert_eq!(
            m.member_types()
                .filter(|t| t.usage == MemberUsage::Beam)
                .count(),
            7
        );
        let facade = m
            .wall_types()
            .find(|t| t.name.starts_with("Fachada"))
            .unwrap();
        assert!((facade.thickness() - 0.265).abs() < 1e-12);
        assert_eq!(facade.function, WallFunction::Exterior);
        assert_eq!(m.curtain_types().count(), 5);
    }
}
