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
    /// Linen and cotton: a plain weave with slubs.
    Linen,
    /// Bouclé: little loops of yarn.
    Boucle,
    /// Knitted wool: rows of stitches.
    Knit,
    /// Velvet: a sheen where the surface turns away.
    Velvet,
    /// Rattan, rush, bamboo, straw: a basket weave.
    Wicker,
    /// Terrazzo: chips of stone in a ground.
    Terrazzo,
    /// Marble: veins.
    Marble,
    /// Earth: a mottle with clods and little stones (soil, sand, a dirt
    /// path).
    Soil,
    /// Fallen leaves on the ground ("hojarasca").
    Leaves,
    // The stylized range (2026-11-02): drawn with a relief that the light
    // catches and a few tones a piece, as a painter would.
    /// Ashlar ("sillería"): big squared blocks in courses, chamfered.
    Ashlar,
    /// Clapboard ("tablas solapadas"): boards along the wall, each leaning
    /// out over the one below.
    Clapboard,
    /// Shingles ("tejuela", wood or slate): staggered rows.
    Shingles,
    /// Fish scales ("escamas"): shingles with rounded lower edges.
    Scales,
    /// Barrel tiles ("teja curva"): channels and caps down the slope.
    Barrel,
    /// Standing seam metal ("chapa de junta alzada"): copper, zinc.
    Seam,
    /// Setts ("adoquín"): rounded stones in courses.
    Pavers,
    /// Asphalt: a blue-grey ground with worn patches.
    Asphalt,
    /// Rock in strata ("roca en estratos"): stacked slabs of sandstone.
    Strata,
    /// The sea: deep, with strokes of light that drift (water of a pool
    /// is [`TextureKind::Water`]).
    Sea,
    /// Setts laid diagonally ("adoquín en diagonal").
    SettsDiagonal,
    /// Setts in fans ("adoquín en abanico").
    SettsFan,
    /// Cobbles ("empedrado"): rounded stones packed close.
    Cobbles,
    /// Old cobbles ("empedrado antiguo"): uneven, worn, earth and moss in
    /// the joints.
    OldCobbles,
    /// Opus romanum: squares and oblongs of four sizes in a module.
    Opus,
    /// Flagstone ("laja"): irregular flat stones, wide joints.
    Flagstone,
    /// Flagstones in grass ("laja en césped"): stepping stones.
    FlagsInGrass,
    /// Marble slabs ("losas de mármol"): veined slabs with thin joints.
    MarbleSlabs,
    /// Pebbles ("guijarros"): small oval stones of three colours.
    Pebbles,
    /// Metro tiles ("azulejo metro"): glazed oblongs, staggered.
    MetroTiles,
    /// Patterned tiles ("azulejo andaluz", "baldosa hidráulica"): a motif in
    /// three colours on each tile.
    PatternTiles,
    /// An earth path ("tierra de camino"): trodden earth, little stones.
    EarthPath,
    /// Albero: the yellow sand of Andalusian paths.
    Albero,
    /// Furrows ("tierra en surcos"): a field ploughed or sown.
    Furrows,
    /// A kitchen garden ("huerta"): rows of plants on ridges.
    KitchenGarden,
    /// Cracked earth ("tierra cuarteada"): dry plates.
    CrackedEarth,
}

impl TextureKind {
    pub const ALL: [TextureKind; 46] = [
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
        TextureKind::Linen,
        TextureKind::Boucle,
        TextureKind::Knit,
        TextureKind::Velvet,
        TextureKind::Wicker,
        TextureKind::Terrazzo,
        TextureKind::Marble,
        TextureKind::Soil,
        TextureKind::Leaves,
        TextureKind::Ashlar,
        TextureKind::Clapboard,
        TextureKind::Shingles,
        TextureKind::Scales,
        TextureKind::Barrel,
        TextureKind::Seam,
        TextureKind::Pavers,
        TextureKind::Asphalt,
        TextureKind::Strata,
        TextureKind::Sea,
        TextureKind::SettsDiagonal,
        TextureKind::SettsFan,
        TextureKind::Cobbles,
        TextureKind::OldCobbles,
        TextureKind::Opus,
        TextureKind::Flagstone,
        TextureKind::FlagsInGrass,
        TextureKind::MarbleSlabs,
        TextureKind::Pebbles,
        TextureKind::MetroTiles,
        TextureKind::PatternTiles,
        TextureKind::EarthPath,
        TextureKind::Albero,
        TextureKind::Furrows,
        TextureKind::KitchenGarden,
        TextureKind::CrackedEarth,
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
            TextureKind::Linen => "linen",
            TextureKind::Boucle => "boucle",
            TextureKind::Knit => "knit",
            TextureKind::Velvet => "velvet",
            TextureKind::Wicker => "wicker",
            TextureKind::Terrazzo => "terrazzo",
            TextureKind::Marble => "marble",
            TextureKind::Soil => "soil",
            TextureKind::Leaves => "leaves",
            TextureKind::Ashlar => "ashlar",
            TextureKind::Clapboard => "clapboard",
            TextureKind::Shingles => "shingles",
            TextureKind::Scales => "scales",
            TextureKind::Barrel => "barrel_tiles",
            TextureKind::Seam => "standing_seam",
            TextureKind::Pavers => "setts",
            TextureKind::Asphalt => "asphalt",
            TextureKind::Strata => "strata",
            TextureKind::Sea => "sea",
            TextureKind::SettsDiagonal => "setts_diagonal",
            TextureKind::SettsFan => "setts_fan",
            TextureKind::Cobbles => "cobbles",
            TextureKind::OldCobbles => "old_cobbles",
            TextureKind::Opus => "opus_romanum",
            TextureKind::Flagstone => "flagstone",
            TextureKind::FlagsInGrass => "flagstones_in_grass",
            TextureKind::MarbleSlabs => "marble_slabs",
            TextureKind::Pebbles => "pebbles",
            TextureKind::MetroTiles => "metro_tiles",
            TextureKind::PatternTiles => "pattern_tiles",
            TextureKind::EarthPath => "earth_path",
            TextureKind::Albero => "albero",
            TextureKind::Furrows => "furrows",
            TextureKind::KitchenGarden => "kitchen_garden",
            TextureKind::CrackedEarth => "cracked_earth",
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
            TextureKind::Linen => "Lino",
            TextureKind::Boucle => "Bouclé",
            TextureKind::Knit => "Punto",
            TextureKind::Velvet => "Terciopelo",
            TextureKind::Wicker => "Mimbre",
            TextureKind::Terrazzo => "Terrazo",
            TextureKind::Marble => "Mármol",
            TextureKind::Soil => "Tierra",
            TextureKind::Leaves => "Hojarasca",
            TextureKind::Ashlar => "Sillería",
            TextureKind::Clapboard => "Tablas solapadas",
            TextureKind::Shingles => "Tejuela",
            TextureKind::Scales => "Escamas",
            TextureKind::Barrel => "Teja curva",
            TextureKind::Seam => "Chapa de junta alzada",
            TextureKind::Pavers => "Adoquín",
            TextureKind::Asphalt => "Asfalto",
            TextureKind::Strata => "Roca en estratos",
            TextureKind::Sea => "Agua de mar",
            TextureKind::SettsDiagonal => "Adoquín en diagonal",
            TextureKind::SettsFan => "Adoquín en abanico",
            TextureKind::Cobbles => "Empedrado",
            TextureKind::OldCobbles => "Empedrado antiguo",
            TextureKind::Opus => "Opus romano",
            TextureKind::Flagstone => "Laja",
            TextureKind::FlagsInGrass => "Laja en césped",
            TextureKind::MarbleSlabs => "Losas de mármol",
            TextureKind::Pebbles => "Guijarros",
            TextureKind::MetroTiles => "Azulejo metro",
            TextureKind::PatternTiles => "Azulejo de dibujo",
            TextureKind::EarthPath => "Tierra de camino",
            TextureKind::Albero => "Albero",
            TextureKind::Furrows => "Tierra en surcos",
            TextureKind::KitchenGarden => "Huerta",
            TextureKind::CrackedEarth => "Tierra cuarteada",
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
            TextureKind::Linen => 0.2,
            TextureKind::Boucle => 0.1,
            TextureKind::Knit => 0.12,
            TextureKind::Velvet => 0.3,
            TextureKind::Wicker => 0.1,
            TextureKind::Terrazzo => 0.25,
            TextureKind::Marble => 0.8,
            TextureKind::Soil => 0.5,
            TextureKind::Leaves => 0.4,
            // A block's width; a board's; a course of shingles or scales;
            // a tile's channel; a panel; a sett; a slab's height.
            TextureKind::Ashlar => 0.6,
            TextureKind::Clapboard => 0.16,
            TextureKind::Shingles => 0.18,
            TextureKind::Scales => 0.16,
            TextureKind::Barrel => 0.22,
            TextureKind::Seam => 0.45,
            TextureKind::Pavers => 0.2,
            TextureKind::Asphalt => 1.0,
            TextureKind::Strata => 0.55,
            TextureKind::Sea => 1.0,
            // A sett times √2 (so the diagonal repeats); a sett of the
            // fans; a cobble; an old cobble; the opus's unit; a flag; a
            // stepping stone; a slab; a pebble.
            TextureKind::SettsDiagonal => 0.28,
            TextureKind::SettsFan => 0.12,
            TextureKind::Cobbles => 0.16,
            TextureKind::OldCobbles => 0.24,
            TextureKind::Opus => 0.15,
            TextureKind::Flagstone => 0.5,
            TextureKind::FlagsInGrass => 0.7,
            TextureKind::MarbleSlabs => 0.6,
            TextureKind::Pebbles => 0.09,
            // A tile's length; a tile's side; earth (a metre); a ridge and
            // its trough; a row of plants; a dry plate.
            TextureKind::MetroTiles => 0.2,
            TextureKind::PatternTiles => 0.2,
            TextureKind::EarthPath => 1.0,
            TextureKind::Albero => 1.0,
            TextureKind::Furrows => 0.7,
            TextureKind::KitchenGarden => 0.8,
            TextureKind::CrackedEarth => 0.35,
        }
    }

    /// The texture a material of this name and category looks best with,
    /// if any: what the library gives its own, and what furniture without
    /// an appearance of its own is drawn with.
    pub fn suggested(name: &str, category: &str) -> Option<TextureKind> {
        let has = |w: &str| name.starts_with(w);
        Some(if has("Bouclé") || has("Toalla") {
            TextureKind::Boucle
        } else if has("Lana") {
            TextureKind::Knit
        } else if has("Terciopelo") || has("Pana") {
            TextureKind::Velvet
        } else if ["Ratán", "Enea", "Bambú", "Paja", "Cañizo"]
            .iter()
            .any(|w| has(w))
        {
            TextureKind::Wicker
        } else if has("Terrazo") {
            TextureKind::Terrazzo
        } else if has("Mármol") {
            TextureKind::Marble
        } else if has("Microcemento") || has("Cal ") {
            TextureKind::Plaster
        } else {
            match category {
                "fabric" => TextureKind::Linen,
                "wood" if !has("Lacado") && !has("Madera encalada") => TextureKind::Wood,
                _ => return None,
            }
        })
    }

    /// Whether it covers a roof: its courses run along the eaves and up the
    /// slope on any pitch (the others lie in plan on faces looking up).
    pub fn on_slopes(self) -> bool {
        matches!(
            self,
            TextureKind::Shingles | TextureKind::Scales | TextureKind::Barrel | TextureKind::Seam
        )
    }

    /// Water, seen through and moving.
    pub fn is_water(self) -> bool {
        matches!(self, TextureKind::Water | TextureKind::Sea)
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
    /// What grows on it, drawn blade by blade in 3D where it covers the
    /// ground (a meadow, flowers); none for the rest.
    pub growth: Option<Growth>,
}

/// Grass and flowers growing on a surface painted over the ground: tufts
/// of blades and flowers among them, scattered at random (the same each
/// time) where it is painted.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Growth {
    /// How tall the grass grows (m); 0: no blades.
    pub height: f64,
    /// Tufts per m².
    pub density: f64,
    /// The colours of its flowers (the first `colours` of them).
    pub flowers: [[u8; 3]; 3],
    pub colours: u8,
    /// Flowers per m².
    pub bloom: f64,
}

impl Growth {
    /// Grass alone.
    pub fn grass(height: f64, density: f64) -> Growth {
        Growth {
            height,
            density,
            flowers: [[0; 3]; 3],
            colours: 0,
            bloom: 0.0,
        }
    }

    /// Grass with flowers of up to three colours.
    pub fn flowering(height: f64, density: f64, flowers: &[[u8; 3]], bloom: f64) -> Growth {
        let mut g = Growth::grass(height, density);
        for (i, c) in flowers.iter().take(3).enumerate() {
            g.flowers[i] = *c;
        }
        g.colours = flowers.len().min(3) as u8;
        g.bloom = bloom;
        g
    }

    /// The colours of its flowers.
    pub fn flower_colours(&self) -> &[[u8; 3]] {
        &self.flowers[..(self.colours as usize).min(3)]
    }
}

impl Appearance {
    pub fn is_plain(&self) -> bool {
        self.color.is_none() && self.texture.is_none() && self.growth.is_none()
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
        if let Some(g) = a.growth {
            for (v, what) in [
                (g.height, "grass height"),
                (g.density, "grass density"),
                (g.bloom, "flowers"),
            ] {
                finite(v, what)?;
                if v < 0.0 {
                    return Err(ModelError::NonPositive(what));
                }
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
