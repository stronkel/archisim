//! What the Exterior brush paints over the ground (C8): earths, sand,
//! gravel, paving and fallen leaves, and lawns and meadows with their
//! grass and flowers (drawn blade by blade in 3D, `archisim_geom::growth`).
//! Each cover is a material of the project, made the first time it is
//! painted, with its colour, its texture and what grows on it.

use archisim_core::{Appearance, CutPattern, Growth, MaterialId, Model, TextureKind};

/// The family a cover belongs to, for the brush's palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverGroup {
    /// Lawns and meadows: grass, with or without flowers.
    Grass,
    /// Earth, sand, gravel, paving, leaves.
    Ground,
}

/// A cover of the ground.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundCover {
    pub name: &'static str,
    pub group: CoverGroup,
    pub category: &'static str,
    pub rgb: [u8; 3],
    pub texture: TextureKind,
    /// The texture's piece (m); none: its own.
    pub scale: Option<f64>,
    /// Grass height (m), tufts per m², flower colours and flowers per m².
    pub grass: Option<(f64, f64)>,
    pub flowers: &'static [[u8; 3]],
    pub bloom: f64,
}

const fn cover(
    name: &'static str,
    group: CoverGroup,
    category: &'static str,
    rgb: [u8; 3],
    texture: TextureKind,
) -> GroundCover {
    GroundCover {
        name,
        group,
        category,
        rgb,
        texture,
        scale: None,
        grass: None,
        flowers: &[],
        bloom: 0.0,
    }
}

const YELLOW: [u8; 3] = [246, 206, 46];
const ORANGE: [u8; 3] = [238, 112, 40];
const VIOLET: [u8; 3] = [132, 118, 222];
const WHITE: [u8; 3] = [246, 244, 236];
const PINK: [u8; 3] = [236, 150, 186];
const RED: [u8; 3] = [214, 48, 44];

/// Every cover, in the order of the brush's palette: grass first.
pub const GROUND_COVERS: [GroundCover; 16] = [
    cover(
        "Césped",
        CoverGroup::Grass,
        "grass",
        [112, 158, 82],
        TextureKind::Grass,
    ),
    GroundCover {
        grass: Some((0.08, 22.0)),
        ..cover(
            "Césped natural",
            CoverGroup::Grass,
            "grass",
            [104, 150, 70],
            TextureKind::Grass,
        )
    },
    GroundCover {
        grass: Some((0.45, 10.0)),
        ..cover(
            "Hierba alta",
            CoverGroup::Grass,
            "grass",
            [126, 158, 66],
            TextureKind::Grass,
        )
    },
    GroundCover {
        grass: Some((0.35, 9.0)),
        ..cover(
            "Pradera seca",
            CoverGroup::Grass,
            "grass",
            [186, 170, 98],
            TextureKind::Grass,
        )
    },
    GroundCover {
        grass: Some((0.25, 12.0)),
        flowers: &[YELLOW],
        bloom: 10.0,
        ..cover(
            "Pradera con flores amarillas",
            CoverGroup::Grass,
            "grass",
            [118, 160, 70],
            TextureKind::Grass,
        )
    },
    GroundCover {
        grass: Some((0.3, 12.0)),
        flowers: &[ORANGE, VIOLET, YELLOW],
        bloom: 12.0,
        ..cover(
            "Flores silvestres",
            CoverGroup::Grass,
            "grass",
            [120, 156, 68],
            TextureKind::Grass,
        )
    },
    GroundCover {
        grass: Some((0.18, 14.0)),
        flowers: &[WHITE, PINK],
        bloom: 12.0,
        ..cover(
            "Margaritas",
            CoverGroup::Grass,
            "grass",
            [110, 156, 76],
            TextureKind::Grass,
        )
    },
    GroundCover {
        grass: Some((0.3, 10.0)),
        flowers: &[RED],
        bloom: 7.0,
        ..cover(
            "Amapolas",
            CoverGroup::Grass,
            "grass",
            [128, 160, 74],
            TextureKind::Grass,
        )
    },
    cover(
        "Tierra",
        CoverGroup::Ground,
        "earth",
        [150, 122, 92],
        TextureKind::Soil,
    ),
    cover(
        "Tierra oscura",
        CoverGroup::Ground,
        "earth",
        [96, 74, 56],
        TextureKind::Soil,
    ),
    cover(
        "Camino de tierra",
        CoverGroup::Ground,
        "earth",
        [178, 148, 106],
        TextureKind::Soil,
    ),
    GroundCover {
        scale: Some(0.25),
        ..cover(
            "Arena",
            CoverGroup::Ground,
            "sand",
            [226, 202, 152],
            TextureKind::Soil,
        )
    },
    cover(
        "Grava",
        CoverGroup::Ground,
        "gravel",
        [196, 190, 176],
        TextureKind::Gravel,
    ),
    cover(
        "Losas de piedra",
        CoverGroup::Ground,
        "stone",
        [182, 172, 156],
        TextureKind::Stone,
    ),
    cover(
        "Hojarasca",
        CoverGroup::Ground,
        "earth",
        [112, 88, 60],
        TextureKind::Leaves,
    ),
    GroundCover {
        scale: Some(0.6),
        ..cover(
            "Corteza de pino",
            CoverGroup::Ground,
            "wood",
            [124, 78, 52],
            TextureKind::Gravel,
        )
    },
];

impl GroundCover {
    /// How it looks in 3D.
    pub fn appearance(&self) -> Appearance {
        Appearance {
            color: None,
            texture: Some(self.texture),
            scale: self.scale,
            growth: self.grass.map(|(h, d)| {
                if self.flowers.is_empty() {
                    Growth::grass(h, d)
                } else {
                    Growth::flowering(h, d, self.flowers, self.bloom)
                }
            }),
        }
    }
}

/// A cover by its name.
pub fn cover_named(name: &str) -> Option<&'static GroundCover> {
    GROUND_COVERS.iter().find(|c| c.name == name)
}

/// The project's material of a cover, made (with how it looks) if it has
/// none of that name.
pub fn cover_material(m: &mut Model, c: &GroundCover) -> MaterialId {
    if let Some(x) = m.materials().find(|x| x.name == c.name) {
        return x.id;
    }
    let id = m.add_material(c.name, c.category, c.rgb, CutPattern::Solid);
    let _ = m.set_appearance(id, c.appearance());
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covers_make_their_materials_once() {
        let mut m = Model::new("t");
        let meadow = cover_named("Flores silvestres").unwrap();
        let a = cover_material(&mut m, meadow);
        let b = cover_material(&mut m, meadow);
        assert_eq!(a, b);
        let g = m.appearance(a).growth.unwrap();
        assert_eq!(g.flower_colours().len(), 3);
        assert!(g.height > 0.2 && g.bloom > 0.0);
        // The plain lawn grows nothing in 3D; earths neither.
        let lawn = cover_material(&mut m, cover_named("Césped").unwrap());
        assert!(m.appearance(lawn).growth.is_none());
        let names: std::collections::HashSet<&str> = GROUND_COVERS.iter().map(|c| c.name).collect();
        assert_eq!(names.len(), GROUND_COVERS.len());
    }
}
