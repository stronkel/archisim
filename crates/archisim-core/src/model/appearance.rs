//! How a material looks in 3D (its "aspecto" for visualisation): a colour
//! of its surface, if it is not the colour it is drawn with when cut, and
//! a texture drawn on its faces (bricks, boards, stone, grass…). The
//! textures are drawn by the 3D view itself from each face's position, so
//! nothing has to be unwrapped and no image is kept: it changes how a
//! face looks, not what it is made of.

use super::*;

/// A texture the 3D view draws on a face.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextureKind {
    /// Bricks in stretcher bond, with mortar joints.
    Brick,
    /// Irregular stones (masonry or paving).
    Stone,
    /// Square tiles with joints.
    Tile,
    /// Wood grain.
    Wood,
    /// Boards or planks (decking, cladding, parquet).
    Planks,
    /// Board-marked concrete, with the formwork holes.
    Concrete,
    /// Render or plaster: a fine, even grain.
    Plaster,
    /// Grass.
    Grass,
    /// Gravel.
    Gravel,
    /// Water: ripples.
    Water,
    /// Corrugated or ribbed metal sheet.
    Metal,
}

impl TextureKind {
    pub const ALL: [TextureKind; 11] = [
        TextureKind::Brick,
        TextureKind::Stone,
        TextureKind::Tile,
        TextureKind::Wood,
        TextureKind::Planks,
        TextureKind::Concrete,
        TextureKind::Plaster,
        TextureKind::Grass,
        TextureKind::Gravel,
        TextureKind::Water,
        TextureKind::Metal,
    ];

    pub fn code(self) -> &'static str {
        match self {
            TextureKind::Brick => "brick",
            TextureKind::Stone => "stone",
            TextureKind::Tile => "tile",
            TextureKind::Wood => "wood",
            TextureKind::Planks => "planks",
            TextureKind::Concrete => "concrete",
            TextureKind::Plaster => "plaster",
            TextureKind::Grass => "grass",
            TextureKind::Gravel => "gravel",
            TextureKind::Water => "water",
            TextureKind::Metal => "metal",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.code() == c)
    }

    /// Its name for people.
    pub fn label(self) -> &'static str {
        match self {
            TextureKind::Brick => "Ladrillo",
            TextureKind::Stone => "Piedra",
            TextureKind::Tile => "Baldosa",
            TextureKind::Wood => "Madera",
            TextureKind::Planks => "Tablas",
            TextureKind::Concrete => "Hormigón visto",
            TextureKind::Plaster => "Enfoscado",
            TextureKind::Grass => "Césped",
            TextureKind::Gravel => "Grava",
            TextureKind::Water => "Agua",
            TextureKind::Metal => "Chapa",
        }
    }

    /// The size of its piece, metres: a brick's length, a tile's side, a
    /// board's width…
    pub fn default_scale(self) -> f64 {
        match self {
            TextureKind::Brick => 0.25,
            TextureKind::Stone => 0.4,
            TextureKind::Tile => 0.6,
            TextureKind::Wood => 0.5,
            TextureKind::Planks => 0.15,
            TextureKind::Concrete => 1.2,
            TextureKind::Plaster => 0.5,
            TextureKind::Grass => 0.3,
            TextureKind::Gravel => 0.08,
            TextureKind::Water => 1.0,
            TextureKind::Metal => 0.08,
        }
    }

    /// The number the 3D view knows it by (0 is none).
    pub fn number(self) -> u32 {
        Self::ALL
            .iter()
            .position(|k| *k == self)
            .map_or(0, |i| i as u32 + 1)
    }
}

/// How a material looks in 3D.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Appearance {
    /// The colour of its surface (none: its cut colour).
    pub color: Option<[u8; 3]>,
    pub texture: Option<TextureKind>,
    /// The size of the texture's piece, metres (none: the texture's own).
    pub scale: Option<f64>,
}

impl Appearance {
    pub fn is_plain(&self) -> bool {
        self.color.is_none() && self.texture.is_none()
    }

    /// The texture's piece size.
    pub fn texture_scale(&self) -> f64 {
        self.scale
            .or_else(|| self.texture.map(TextureKind::default_scale))
            .unwrap_or(1.0)
    }
}

impl Model {
    /// How a material looks in 3D (plain if nothing was set).
    pub fn appearance(&self, material: MaterialId) -> Appearance {
        self.appearances.get(&material).copied().unwrap_or_default()
    }

    /// The colour a material is seen with in 3D.
    pub fn surface_color(&self, material: MaterialId) -> [u8; 3] {
        self.appearance(material)
            .color
            .or_else(|| self.material(material).map(|m| m.cut_color))
            .unwrap_or([200, 200, 200])
    }

    pub fn set_appearance(
        &mut self,
        material: MaterialId,
        a: Appearance,
    ) -> Result<(), ModelError> {
        if !self.materials.contains_key(&material) {
            return Err(ModelError::UnknownMaterial(material));
        }
        if let Some(s) = a.scale {
            finite(s, "texture size")?;
            if s <= EPSILON {
                return Err(ModelError::NonPositive("texture size"));
            }
        }
        if a.is_plain() {
            self.appearances.remove(&material);
        } else {
            self.appearances.insert(material, a);
        }
        Ok(())
    }

    /// Materials with an appearance of their own.
    pub fn appearances(&self) -> impl Iterator<Item = (MaterialId, Appearance)> + '_ {
        self.appearances
            .iter()
            .filter(|(m, _)| self.materials.contains_key(m))
            .map(|(m, a)| (*m, *a))
    }

    pub(super) fn restore_appearances(
        &mut self,
        list: Vec<(MaterialId, Appearance)>,
    ) -> Result<(), RestoreError> {
        for (m, a) in list {
            self.set_appearance(m, a).map_err(|error| RestoreError {
                element: format!("appearance of material {}", m.0),
                error,
            })?;
        }
        Ok(())
    }
}
