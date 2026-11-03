//! The stylized range of materials (2026-11-02): between real and drawn,
//! with colour, as in Paralives or CityRT. Each is a colour and one of the
//! 3D view's textures, with the size of its piece; the 3D view draws them
//! with a relief the light catches (bricks' bevels, boards' lips, tiles'
//! channels). They are offered with the other materials and added to a
//! project when they are first picked (`crate::palette_material`).

use archisim_core::{Appearance, CutPattern, MaterialId, Model, TextureKind};

/// A material of the range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stylized {
    pub name: &'static str,
    /// Its group for people (Fachadas, Cubiertas, Alicatados, Suelos,
    /// Pavimentos, Tierras, Pinturas).
    pub group: &'static str,
    pub category: &'static str,
    /// Its colour (sRGB).
    pub color: [u8; 3],
    pub texture: Option<TextureKind>,
    /// The size of its piece, metres.
    pub scale: f64,
}

const fn st(
    name: &'static str,
    group: &'static str,
    category: &'static str,
    color: [u8; 3],
    texture: TextureKind,
    scale: f64,
) -> Stylized {
    Stylized {
        name,
        group,
        category,
        color,
        texture: Some(texture),
        scale,
    }
}

const fn paint(name: &'static str, color: [u8; 3]) -> Stylized {
    Stylized {
        name,
        group: "Pinturas",
        category: "paint",
        color,
        texture: None,
        scale: 1.0,
    }
}

/// The range, by group.
pub const STYLIZED: &[Stylized] = &[
    // Façades.
    st(
        "Ladrillo rojo",
        "Fachadas",
        "brick",
        [192, 106, 72],
        TextureKind::Brick,
        0.25,
    ),
    st(
        "Ladrillo amarillo",
        "Fachadas",
        "brick",
        [224, 160, 102],
        TextureKind::Brick,
        0.25,
    ),
    st(
        "Sillería arenisca",
        "Fachadas",
        "stone",
        [205, 191, 163],
        TextureKind::Ashlar,
        0.6,
    ),
    st(
        "Mampostería",
        "Fachadas",
        "stone",
        [180, 176, 166],
        TextureKind::Stone,
        0.4,
    ),
    st(
        "Estuco amarillo",
        "Fachadas",
        "plaster",
        [239, 207, 106],
        TextureKind::Plaster,
        0.5,
    ),
    st(
        "Estuco almagra",
        "Fachadas",
        "plaster",
        [226, 164, 140],
        TextureKind::Plaster,
        0.5,
    ),
    st(
        "Encalado",
        "Fachadas",
        "plaster",
        [244, 239, 228],
        TextureKind::Plaster,
        0.5,
    ),
    st(
        "Tablas solapadas azul",
        "Fachadas",
        "wood",
        [125, 159, 208],
        TextureKind::Clapboard,
        0.16,
    ),
    st(
        "Tablas solapadas verde",
        "Fachadas",
        "wood",
        [169, 211, 181],
        TextureKind::Clapboard,
        0.16,
    ),
    st(
        "Hormigón suave",
        "Fachadas",
        "concrete",
        [201, 196, 187],
        TextureKind::Concrete,
        1.2,
    ),
    // Roofs.
    st(
        "Tejuela de pizarra",
        "Cubiertas",
        "stone",
        [94, 100, 110],
        TextureKind::Shingles,
        0.17,
    ),
    st(
        "Tejuela verde",
        "Cubiertas",
        "wood",
        [79, 122, 88],
        TextureKind::Shingles,
        0.2,
    ),
    st(
        "Escamas de pizarra",
        "Cubiertas",
        "stone",
        [109, 117, 144],
        TextureKind::Scales,
        0.16,
    ),
    st(
        "Teja curva",
        "Cubiertas",
        "ceramic",
        [188, 108, 71],
        TextureKind::Barrel,
        0.22,
    ),
    st(
        "Cobre con pátina",
        "Cubiertas",
        "metal",
        [94, 154, 128],
        TextureKind::Seam,
        0.45,
    ),
    st(
        "Zinc",
        "Cubiertas",
        "metal",
        [141, 151, 163],
        TextureKind::Seam,
        0.45,
    ),
    // Ground.
    st(
        "Adoquín",
        "Suelos",
        "stone",
        [185, 174, 156],
        TextureKind::Pavers,
        0.2,
    ),
    st(
        "Losa de acera",
        "Suelos",
        "stone",
        [214, 208, 196],
        TextureKind::Tile,
        0.5,
    ),
    st(
        "Asfalto",
        "Suelos",
        "asphalt",
        [95, 100, 112],
        TextureKind::Asphalt,
        1.0,
    ),
    st(
        "Tarima",
        "Suelos",
        "wood",
        [192, 138, 88],
        TextureKind::Planks,
        0.14,
    ),
    st(
        "Roca en estratos",
        "Suelos",
        "stone",
        [232, 210, 160],
        TextureKind::Strata,
        0.6,
    ),
    st(
        "Agua de mar",
        "Suelos",
        "water",
        [29, 111, 158],
        TextureKind::Sea,
        1.0,
    ),
    st(
        "Agua de piscina",
        "Suelos",
        "water",
        [71, 196, 216],
        TextureKind::Water,
        1.0,
    ),
    st(
        "Gresite",
        "Suelos",
        "ceramic",
        [108, 194, 214],
        TextureKind::Tile,
        0.05,
    ),
    // Pavings.
    st(
        "Adoquín en diagonal",
        "Pavimentos",
        "stone",
        [179, 167, 150],
        TextureKind::SettsDiagonal,
        0.28,
    ),
    st(
        "Adoquín en abanico",
        "Pavimentos",
        "stone",
        [169, 164, 156],
        TextureKind::SettsFan,
        0.12,
    ),
    st(
        "Empedrado",
        "Pavimentos",
        "stone",
        [164, 154, 140],
        TextureKind::Cobbles,
        0.16,
    ),
    st(
        "Empedrado antiguo",
        "Pavimentos",
        "stone",
        [156, 144, 129],
        TextureKind::OldCobbles,
        0.24,
    ),
    st(
        "Opus romano",
        "Pavimentos",
        "stone",
        [216, 198, 164],
        TextureKind::Opus,
        0.15,
    ),
    st(
        "Laja",
        "Pavimentos",
        "stone",
        [169, 161, 150],
        TextureKind::Flagstone,
        0.5,
    ),
    st(
        "Laja en césped",
        "Pavimentos",
        "stone",
        [181, 171, 156],
        TextureKind::FlagsInGrass,
        0.7,
    ),
    st(
        "Losas de mármol",
        "Pavimentos",
        "stone",
        [241, 238, 232],
        TextureKind::MarbleSlabs,
        0.6,
    ),
    st(
        "Guijarros",
        "Pavimentos",
        "stone",
        [155, 150, 142],
        TextureKind::Pebbles,
        0.09,
    ),
    st(
        "Grava",
        "Pavimentos",
        "gravel",
        [201, 193, 178],
        TextureKind::Gravel,
        0.08,
    ),
    // Wall tiles.
    st(
        "Azulejo metro blanco",
        "Alicatados",
        "ceramic",
        [242, 241, 236],
        TextureKind::MetroTiles,
        0.2,
    ),
    st(
        "Azulejo metro verde",
        "Alicatados",
        "ceramic",
        [95, 156, 134],
        TextureKind::MetroTiles,
        0.15,
    ),
    st(
        "Azulejo liso 15 × 15",
        "Alicatados",
        "ceramic",
        [233, 229, 220],
        TextureKind::Tile,
        0.15,
    ),
    st(
        "Azulejo andaluz",
        "Alicatados",
        "ceramic",
        [47, 106, 163],
        TextureKind::PatternTiles,
        0.15,
    ),
    st(
        "Baldosa hidráulica",
        "Alicatados",
        "ceramic",
        [154, 74, 60],
        TextureKind::PatternTiles,
        0.2,
    ),
    // Earth.
    st(
        "Tierra de camino",
        "Tierras",
        "earth",
        [176, 138, 98],
        TextureKind::EarthPath,
        1.0,
    ),
    st(
        "Albero",
        "Tierras",
        "earth",
        [225, 178, 90],
        TextureKind::Albero,
        1.0,
    ),
    st(
        "Tierra en surcos",
        "Tierras",
        "earth",
        [140, 100, 68],
        TextureKind::Furrows,
        0.7,
    ),
    st(
        "Huerta",
        "Tierras",
        "earth",
        [125, 90, 62],
        TextureKind::KitchenGarden,
        0.8,
    ),
    st(
        "Tierra cuarteada",
        "Tierras",
        "earth",
        [196, 154, 108],
        TextureKind::CrackedEarth,
        0.35,
    ),
    // Paints, for trims, frames and shutters.
    paint("Pintura crema", [243, 236, 216]),
    paint("Pintura verde carruaje", [63, 122, 82]),
    paint("Pintura chocolate", [90, 58, 44]),
    paint("Pintura granate", [168, 67, 60]),
];

/// The material of the range of this name, if any.
pub fn stylized(name: &str) -> Option<&'static Stylized> {
    STYLIZED.iter().find(|s| s.name == name)
}

impl Stylized {
    /// How it looks in 3D.
    pub fn appearance(&self) -> Appearance {
        Appearance {
            color: None,
            texture: self.texture,
            scale: self.texture.map(|_| self.scale),
            growth: None,
        }
    }
}

/// A material of the range, added to the project if it has none of that
/// name (one it has keeps its own look).
pub fn stylized_material(m: &mut Model, name: &str) -> Option<MaterialId> {
    let s = stylized(name)?;
    if let Some(x) = m.materials().find(|x| x.name == s.name) {
        return Some(x.id);
    }
    let id = m.add_material(s.name, s.category, s.color, CutPattern::Solid);
    let _ = m.set_appearance(id, s.appearance());
    Some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_material_of_the_range_can_be_added() {
        let mut m = Model::new("Gama");
        for s in STYLIZED {
            let id = stylized_material(&mut m, s.name).unwrap();
            assert_eq!(m.appearance(id).texture, s.texture, "{}", s.name);
            // Twice: the same material.
            assert_eq!(stylized_material(&mut m, s.name), Some(id));
        }
        let mut names: Vec<&str> = STYLIZED.iter().map(|s| s.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), STYLIZED.len());
    }
}
