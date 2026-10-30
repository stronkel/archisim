//! Façades in full detail (E9): the wall types of the template whose
//! exterior finish is laid piece by piece — face brick in several bonds
//! and patterns, exposed concrete block, precast panels, a ventilated
//! façade and stone cladding — with their materials and the pieces over
//! the slab edges.

use archisim_core::{
    Appearance, Bond, CutPattern, Facade, FacadeSystem, Layer, LayerFunction, Model, ModelError,
    Pattern, SlabFront, TextureKind, WallFunction, WallTypeId,
};

/// A material for façades: added if the project has none of that name,
/// with the texture it shows from afar.
fn material(
    m: &mut Model,
    name: &str,
    category: &str,
    color: [u8; 3],
    pattern: CutPattern,
    texture: Option<TextureKind>,
) -> archisim_core::MaterialId {
    let id = super::material_named(m, name, category, color, pattern);
    if let Some(t) = texture {
        if m.appearance(id).is_plain() {
            let _ = m.set_appearance(
                id,
                Appearance {
                    texture: Some(t),
                    ..Default::default()
                },
            );
        }
    }
    id
}

/// The materials of the façades, by name: the mortar of the joints and the
/// joint of the ventilated façades are the first two.
pub fn facade_material(m: &mut Model, name: &str) -> Option<archisim_core::MaterialId> {
    let (category, color, pattern, texture) = match name {
        "Mortero de juntas" => ("mortar", [196, 190, 178], CutPattern::Dots, None),
        "Junta abierta" => ("other", [52, 52, 54], CutPattern::Empty, None),
        "Ladrillo cara vista rojo" => (
            "brick",
            [168, 86, 60],
            CutPattern::Diagonal,
            Some(TextureKind::Brick),
        ),
        "Ladrillo cara vista oscuro" => (
            "brick",
            [96, 58, 46],
            CutPattern::Diagonal,
            Some(TextureKind::Brick),
        ),
        "Ladrillo cara vista blanco" => (
            "brick",
            [226, 218, 200],
            CutPattern::Diagonal,
            Some(TextureKind::Brick),
        ),
        "Plaqueta cerámica" => (
            "brick",
            [150, 78, 56],
            CutPattern::Diagonal,
            Some(TextureKind::Brick),
        ),
        "Bloque de hormigón visto" => (
            "concrete",
            [168, 166, 160],
            CutPattern::Cross,
            Some(TextureKind::Tile),
        ),
        "Panel prefabricado de hormigón" => (
            "concrete",
            [204, 198, 186],
            CutPattern::Cross,
            Some(TextureKind::Concrete),
        ),
        "Placa de fachada ventilada" => (
            "ceramic",
            [214, 206, 192],
            CutPattern::Solid,
            Some(TextureKind::Tile),
        ),
        "Aplacado de piedra caliza" => (
            "stone",
            [206, 194, 166],
            CutPattern::Diagonal,
            Some(TextureKind::Stone),
        ),
        "Cámara de aire" => ("air", [240, 244, 248], CutPattern::Empty, None),
        _ => return None,
    };
    Some(material(m, name, category, color, pattern, texture))
}

fn existing(m: &Model, name: &str) -> Option<archisim_core::MaterialId> {
    m.materials().find(|x| x.name == name).map(|x| x.id)
}

/// The façade types of the template: (name, system, bond, pattern, piece
/// material, accent material, slab front: plaquetas or soldiers).
type FacadeEntry = (
    &'static str,
    FacadeSystem,
    Bond,
    Pattern,
    &'static str,
    Option<&'static str>,
    Option<bool>,
);

pub const FACADE_TYPES: &[FacadeEntry] = &[
    (
        "Fachada ladrillo visto a soga 30",
        FacadeSystem::Brick,
        Bond::Stretcher,
        Pattern::Plain,
        "Ladrillo cara vista rojo",
        None,
        Some(true),
    ),
    (
        "Fachada ladrillo visto inglés 30",
        FacadeSystem::Brick,
        Bond::English,
        Pattern::Plain,
        "Ladrillo cara vista rojo",
        None,
        Some(false),
    ),
    (
        "Fachada ladrillo visto flamenco 30",
        FacadeSystem::Brick,
        Bond::Flemish,
        Pattern::Headers,
        "Ladrillo cara vista rojo",
        Some("Ladrillo cara vista oscuro"),
        Some(false),
    ),
    (
        "Fachada ladrillo visto con rombos 30",
        FacadeSystem::Brick,
        Bond::Stretcher,
        Pattern::Diamonds(3),
        "Ladrillo cara vista blanco",
        Some("Ladrillo cara vista oscuro"),
        Some(true),
    ),
    (
        "Fachada bloque de hormigón visto 30",
        FacadeSystem::Block,
        Bond::Stretcher,
        Pattern::Plain,
        "Bloque de hormigón visto",
        None,
        None,
    ),
    (
        "Fachada paneles prefabricados 30",
        FacadeSystem::Panel,
        Bond::Stack,
        Pattern::Plain,
        "Panel prefabricado de hormigón",
        None,
        None,
    ),
    (
        "Fachada ventilada de placas 32",
        FacadeSystem::Ventilated,
        Bond::Stack,
        Pattern::Plain,
        "Placa de fachada ventilada",
        None,
        None,
    ),
    (
        "Fachada aplacado de piedra 30",
        FacadeSystem::Cladding,
        Bond::Stretcher,
        Pattern::Plain,
        "Aplacado de piedra caliza",
        None,
        None,
    ),
];

/// Adds the façade types (those it has not already), and returns them in
/// the order of [`FACADE_TYPES`].
pub fn add_facade_types(m: &mut Model) -> Result<Vec<WallTypeId>, ModelError> {
    let mut out = Vec::new();
    let lana = existing(m, "Lana mineral");
    let lhd = existing(m, "Ladrillo hueco doble");
    let yeso = existing(m, "Enlucido de yeso");
    let perforado = existing(m, "Ladrillo perforado");
    let mortero = existing(m, "Mortero de cemento");
    let (Some(lana), Some(lhd), Some(yeso), Some(perforado), Some(mortero)) =
        (lana, lhd, yeso, perforado, mortero)
    else {
        return Ok(out);
    };
    let layer = |material, thickness, function| Layer {
        material,
        thickness,
        function,
    };
    for (name, system, bond, pattern, piece, accent, front) in FACADE_TYPES {
        if let Some(t) = m.wall_types().find(|t| t.name == *name) {
            out.push(t.id);
            continue;
        }
        let piece_m = facade_material(m, piece).expect("a façade material");
        let accent = accent.and_then(|a| facade_material(m, a));
        let mortar = facade_material(m, "Mortero de juntas");
        let open = facade_material(m, "Junta abierta");
        let air = facade_material(m, "Cámara de aire").expect("air");
        let c = system.coursing();
        let (layers, joint) = match system {
            FacadeSystem::Brick => (
                vec![
                    layer(piece_m, 0.115, LayerFunction::Finish),
                    layer(lana, 0.05, LayerFunction::Insulation),
                    layer(lhd, 0.07, LayerFunction::Substrate),
                    layer(yeso, 0.015, LayerFunction::Finish),
                ],
                mortar,
            ),
            FacadeSystem::Block => (
                vec![
                    layer(piece_m, 0.19, LayerFunction::Finish),
                    layer(lana, 0.05, LayerFunction::Insulation),
                    layer(lhd, 0.05, LayerFunction::Substrate),
                    layer(yeso, 0.01, LayerFunction::Finish),
                ],
                mortar,
            ),
            FacadeSystem::Panel => (
                vec![
                    layer(piece_m, 0.12, LayerFunction::Finish),
                    layer(lana, 0.08, LayerFunction::Insulation),
                    layer(lhd, 0.07, LayerFunction::Substrate),
                    layer(yeso, 0.03, LayerFunction::Finish),
                ],
                mortar,
            ),
            FacadeSystem::Ventilated => (
                vec![
                    layer(piece_m, 0.012, LayerFunction::Finish),
                    layer(air, 0.04, LayerFunction::AirGap),
                    layer(lana, 0.06, LayerFunction::Insulation),
                    layer(perforado, 0.193, LayerFunction::Structure),
                    layer(yeso, 0.015, LayerFunction::Finish),
                ],
                open,
            ),
            FacadeSystem::Cladding => (
                vec![
                    layer(piece_m, 0.03, LayerFunction::Finish),
                    layer(mortero, 0.02, LayerFunction::Substrate),
                    layer(perforado, 0.115, LayerFunction::Structure),
                    layer(lana, 0.05, LayerFunction::Insulation),
                    layer(lhd, 0.07, LayerFunction::Substrate),
                    layer(yeso, 0.015, LayerFunction::Finish),
                ],
                mortar,
            ),
        };
        let id = m.add_wall_type(*name, WallFunction::Exterior, layers)?;
        let slab_front = front.map(|slips| SlabFront {
            material: if slips {
                facade_material(m, "Plaqueta cerámica")
            } else {
                None
            },
            ..if slips {
                SlabFront::slips()
            } else {
                SlabFront::soldiers()
            }
        });
        let mut slab_front = slab_front;
        if let Some(f) = slab_front.as_mut() {
            f.coursing.joint_material = joint;
        }
        m.set_wall_type_facade(
            id,
            Some(Facade {
                coursing: archisim_core::Coursing {
                    bond: *bond,
                    pattern: *pattern,
                    accent,
                    joint_material: joint,
                    ..c
                },
                slab_front,
            }),
        )?;
        out.push(id);
    }
    Ok(out)
}
