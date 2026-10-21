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
    let curtain = add_curtain_types(m)?;
    add_studio_types(m)?;

    for style in presentation_styles() {
        m.add_presentation_style(style)?;
    }
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
}
use StudioValue::{B, N, T};

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
                    T(t) => archisim_core::Value::Text((*t).into()),
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
pub const OBJECT_CATALOGUE: [CatalogueEntry; 30] = [
    ("Mesa de comedor 160×90", "Mesa", &[]),
    ("Silla", "Silla", &[]),
    ("Sofá de 3 plazas", "Sofá", &[]),
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
    ("Estantería 80", "Estantería", &[]),
    (
        "Librería 160",
        "Estantería",
        &[("width", 1.6), ("height", 2.2), ("shelves", 5.0)],
    ),
    ("Muebles de cocina 240", "Encimera de cocina", &[]),
    ("Muebles altos de cocina 120", "Mueble alto de cocina", &[]),
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
];

/// The materials the library's objects ask for by name, added if the
/// project has none of that name.
fn object_materials(m: &mut Model) {
    for (name, category, color) in [
        ("Porcelana", "ceramic", [244, 244, 240]),
        ("Madera de pino", "wood", [196, 160, 112]),
        ("Vegetación", "vegetation", [126, 164, 98]),
        ("Corteza", "wood", [112, 86, 62]),
        ("Figura", "other", [150, 158, 172]),
        ("Carrocería", "metal", [164, 172, 184]),
        ("Neumático", "rubber", [52, 52, 56]),
    ] {
        material_named(m, name, category, color, CutPattern::Solid);
    }
}

/// Adds one object type of the library (by its name in
/// [`OBJECT_CATALOGUE`]), or finds the project's type of that name.
pub fn add_object_entry(m: &mut Model, name: &str) -> Result<ObjectTypeId, ModelError> {
    use archisim_core::{ParamScope, Value};
    if let Some(t) = m.object_types().find(|t| t.name == name) {
        return Ok(t.id);
    }
    let Some((name, family, sizes)) = OBJECT_CATALOGUE.iter().find(|e| e.0 == name) else {
        return Err(ModelError::NotAnObject(name.to_string()));
    };
    object_materials(m);
    let f = m.system_family(family)?;
    let mut values = m.family_defaults(f, ParamScope::Type)?;
    for (k, v) in sizes.iter() {
        values.insert((*k).into(), Value::Number(*v));
    }
    m.add_object_type(*name, f, values)
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

    #[test]
    fn new_project_has_a_level_and_the_library() {
        let m = new_project("Casa");
        assert_eq!(m.project_name, "Casa");
        assert_eq!(m.levels().len(), 1);
        assert_eq!(m.levels()[0].elevation, 0.0);
        assert_eq!(m.wall_types().count(), 3);
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
