//! Plants made by rules, as EZ-Tree or the tree generators of the
//! modelling programs do, but for architecture: a **species** (how the
//! plant grows: its habit, the shape of its crown, its trunk, branches
//! and leaves) and a **seed** give one plant, always the same for the same
//! seed and different for every other. From the plant come its 3D mesh,
//! in three levels of detail, and its plan symbol in any of the drawing
//! styles of landscape plans (outline, toothed, cloud, branches, leaves,
//! foliage masses, stipple, radial, fronds, watercolour).
//!
//! Axes: origin at the foot of the trunk, `x` east, `y` north, `z` up,
//! in metres. No dependencies: it knows nothing of the model; colours
//! arrive as RGB and the drawing as plain shapes.

macro_rules! names {
    ($t:ty { $($v:ident => $n:literal),* $(,)? }) => {
        impl $t {
            /// Every value, in the order of the choices.
            pub const ALL: &'static [$t] = &[$(<$t>::$v),*];

            /// Its name in Spanish, as the choices of the recipe say it.
            pub fn name(self) -> &'static str {
                match self { $(<$t>::$v => $n),* }
            }

            /// The value of a name (ignoring case).
            pub fn from_name(s: &str) -> Option<Self> {
                let s = s.trim().to_lowercase();
                Self::ALL.iter().copied().find(|v| v.name().to_lowercase() == s)
            }
        }
    };
}

mod envelope;
mod grow;
mod mesh;
mod plan;
mod rng;
mod side;

#[cfg(test)]
mod tests;

pub use grow::{Branch, Clump, ClumpKind, Frond, FrondKind, Plant};
pub use mesh::{FloraMesh, Lod};
pub use plan::{Look, PlanOptions, PlanStyle, Shape, Stroke, Tone};
pub use side::SideOptions;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

pub type Rgb = [u8; 3];

/// How a plant grows: its overall form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Habit {
    /// A trunk that splits into limbs (oak, olive, umbrella pine).
    Tree,
    /// A leader to the top with whorls of branches (fir, cypress).
    Conifer,
    /// A bare trunk with a crown of fronds.
    Palm,
    /// Several stems from the ground (rosemary, mastic, broom).
    Shrub,
    /// A low cushion of short stems (samphire, thyme, saltbush).
    Cushion,
    /// Blades from a tuft (grasses, esparto, pennisetum).
    Grass,
    /// Thick leaves in a rosette (agave, yucca, aloe).
    Rosette,
}

/// The shape of the crown, seen from the side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CrownShape {
    Round,
    Oval,
    /// Flat and wide on a tall trunk (umbrella pine, acacia).
    Umbrella,
    /// Wide and low, a little flattened (carob, jacaranda).
    Spreading,
    /// Narrow and tall, pointed (cypress, poplar).
    Columnar,
    Conical,
    /// Opening upwards (hackberry, elm, zelkova).
    Vase,
    /// Branches hanging (weeping willow, pepper tree).
    Weeping,
    /// Lumpy and asymmetric (cork oak, old olive).
    Irregular,
    /// A dome from the ground (shrubs).
    Mound,
    /// A flat cushion on the ground.
    Cushion,
}

/// What the leaves look like.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Foliage {
    /// Broad leaves in round clumps.
    Broad,
    /// Small leaves, airy and silvery (olive, wild olive, rosemary).
    Fine,
    /// Needles in tufts (pines).
    Needles,
    /// Scales packed tight (cypress, juniper, tamarisk).
    Scales,
    /// Green twigs almost without leaves (broom, retama).
    Broom,
    /// Fleshy jointed stems (samphire, saltwort).
    Succulent,
    /// Feather fronds (date palm, Canary palm).
    Pinnate,
    /// Fan fronds (washingtonia, dwarf fan palm).
    Fan,
    /// Narrow blades (grasses) or thick swords (agave, yucca).
    Blades,
}

names!(Habit {
    Tree => "árbol",
    Conifer => "conífera",
    Palm => "palmera",
    Shrub => "arbusto",
    Cushion => "mata",
    Grass => "gramínea",
    Rosette => "roseta",
});

names!(CrownShape {
    Round => "redonda",
    Oval => "ovalada",
    Umbrella => "parasol",
    Spreading => "extendida",
    Columnar => "columnar",
    Conical => "cónica",
    Vase => "en vaso",
    Weeping => "llorona",
    Irregular => "irregular",
    Mound => "cúpula",
    Cushion => "cojín",
});

names!(Foliage {
    Broad => "hoja ancha",
    Fine => "hoja menuda",
    Needles => "acículas",
    Scales => "escamas",
    Broom => "retama",
    Succulent => "suculenta",
    Pinnate => "pinnada",
    Fan => "abanico",
    Blades => "hojas largas",
});

/// How a species grows. Sizes in metres, angles in degrees, the rest
/// from 0 to 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Species {
    pub habit: Habit,
    pub crown: CrownShape,
    pub foliage: Foliage,
    /// Total height.
    pub height: f64,
    /// Width of the crown.
    pub width: f64,
    /// Diameter of the trunk at the foot.
    pub trunk: f64,
    /// Height of the clear trunk, up to the first branches.
    pub clear: f64,
    /// Trunks or stems from the ground.
    pub stems: u32,
    /// How much the trunk leans.
    pub lean: f64,
    /// How twisted trunk and branches are (old olives, cork oaks).
    pub gnarl: f64,
    /// Main branches (limbs), or branches per whorl in conifers.
    pub limbs: u32,
    /// Orders of branching (1 to 4).
    pub levels: u32,
    /// Angle of the main branches from the vertical.
    pub angle: f64,
    /// How the branches bend: -1 upwards, 0 straight, 1 hanging.
    pub droop: f64,
    /// How full the crown is.
    pub density: f64,
    /// Size of the clumps of leaves.
    pub clump: f64,
    /// Share of the crown in flower.
    pub bloom: f64,
    /// Fruit on the crown (oranges, carobs…).
    pub fruit: f64,
    /// Fronds of a palm, blades of a grass, leaves of a rosette.
    pub fronds: u32,
    /// Length of the fronds or blades.
    pub frond_length: f64,
}

impl Default for Species {
    fn default() -> Self {
        Species {
            habit: Habit::Tree,
            crown: CrownShape::Round,
            foliage: Foliage::Broad,
            height: 6.0,
            width: 4.5,
            trunk: 0.3,
            clear: 1.8,
            stems: 1,
            lean: 0.1,
            gnarl: 0.15,
            limbs: 5,
            levels: 3,
            angle: 40.0,
            droop: 0.0,
            density: 0.7,
            clump: 0.55,
            bloom: 0.0,
            fruit: 0.0,
            fronds: 18,
            frond_length: 2.5,
        }
    }
}

impl Species {
    /// The species at another size: the clear trunk, the trunk, the
    /// clumps and the fronds keep their proportions.
    pub fn sized(mut self, height: f64, width: f64) -> Self {
        let (h0, w0) = (self.height.max(0.05), self.width.max(0.05));
        let (kh, kw) = (height / h0, width / w0);
        let k = (kh * kw).sqrt();
        self.height = height;
        self.width = width;
        self.clear *= kh;
        self.trunk *= k;
        self.clump *= k.sqrt().min(k * 1.5);
        self.frond_length *= k;
        self
    }

    /// Clamps every value into its range, so that any input grows a plant.
    pub fn sane(mut self) -> Self {
        let fin = |v: f64, d: f64| if v.is_finite() { v } else { d };
        self.height = fin(self.height, 6.0).clamp(0.1, 60.0);
        self.width = fin(self.width, 4.0).clamp(0.1, 50.0);
        self.trunk = fin(self.trunk, 0.3).clamp(0.005, 4.0).min(self.width * 0.6);
        self.clear = fin(self.clear, 1.0).clamp(0.0, self.height * 0.9);
        self.stems = self.stems.clamp(1, 40);
        self.lean = fin(self.lean, 0.0).clamp(0.0, 1.0);
        self.gnarl = fin(self.gnarl, 0.0).clamp(0.0, 1.0);
        self.limbs = self.limbs.clamp(1, 24);
        self.levels = self.levels.clamp(1, 4);
        self.angle = fin(self.angle, 40.0).clamp(0.0, 110.0);
        self.droop = fin(self.droop, 0.0).clamp(-1.0, 1.0);
        self.density = fin(self.density, 0.7).clamp(0.05, 1.0);
        self.clump = fin(self.clump, 0.5).clamp(0.02, 5.0);
        self.bloom = fin(self.bloom, 0.0).clamp(0.0, 1.0);
        self.fruit = fin(self.fruit, 0.0).clamp(0.0, 1.0);
        self.fronds = self.fronds.clamp(1, 120);
        self.frond_length = fin(self.frond_length, 1.0).clamp(0.05, 15.0);
        self
    }

    /// A number standing for every value, to keep grown plants.
    fn key(&self, seed: u64, variation: f64) -> u64 {
        let mut h = rng::Hash::new();
        h.add(self.habit as u64);
        h.add(self.crown as u64);
        h.add(self.foliage as u64);
        for v in [
            self.height,
            self.width,
            self.trunk,
            self.clear,
            self.lean,
            self.gnarl,
            self.angle,
            self.droop,
            self.density,
            self.clump,
            self.bloom,
            self.fruit,
            self.frond_length,
            variation,
        ] {
            h.add(v.to_bits());
        }
        h.add(self.stems as u64);
        h.add(self.limbs as u64);
        h.add(self.levels as u64);
        h.add(self.fronds as u64);
        h.add(seed);
        h.finish()
    }
}

/// The colours of a plant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Colors {
    pub leaves: Rgb,
    pub bark: Rgb,
    pub flowers: Rgb,
    pub fruit: Rgb,
}

impl Default for Colors {
    fn default() -> Self {
        Colors {
            leaves: [104, 146, 84],
            bark: [112, 86, 62],
            flowers: [236, 236, 230],
            fruit: [232, 140, 40],
        }
    }
}

/// Grows a plant: the same species, seed and variation give always the
/// same plant. `variation` (0 to 1) says how much this one may differ
/// from the species: lean, lopsided crown, branch angles.
pub fn grow(species: &Species, seed: u64, variation: f64) -> Plant {
    grow::grow(&species.sane(), seed, variation.clamp(0.0, 1.0))
}

/// Plants already grown, kept so that plans and 3D views redrawn many
/// times do not grow them again.
type Nursery = Mutex<HashMap<u64, Arc<Plant>>>;

fn nursery() -> &'static Nursery {
    static N: OnceLock<Nursery> = OnceLock::new();
    N.get_or_init(Default::default)
}

/// As [`grow`], kept: a plant grown before is given again at once.
pub fn plant(species: &Species, seed: u64, variation: f64) -> Arc<Plant> {
    let s = species.sane();
    let variation = variation.clamp(0.0, 1.0);
    let key = s.key(seed, variation);
    if let Some(p) = nursery().lock().ok().and_then(|n| n.get(&key).cloned()) {
        return p;
    }
    let p = Arc::new(grow::grow(&s, seed, variation));
    if let Ok(mut n) = nursery().lock() {
        // Thousands of different plants: start again rather than grow
        // without end.
        if n.len() > 4000 {
            n.clear();
        }
        n.insert(key, p.clone());
    }
    p
}

/// A number from a seed and an index, for the plants of a planting.
pub fn seed_of(seed: u64, index: u64) -> u64 {
    let mut h = rng::Hash::new();
    h.add(seed);
    h.add(index);
    h.finish()
}

type MeshCache = Mutex<HashMap<(u64, [u8; 12], Lod), Arc<FloraMesh>>>;

fn mesh_cache() -> &'static MeshCache {
    static M: OnceLock<MeshCache> = OnceLock::new();
    M.get_or_init(Default::default)
}

/// The mesh of a plant ([`plant`]) at a level of detail, kept as the
/// plants are.
pub fn mesh_of(
    species: &Species,
    seed: u64,
    variation: f64,
    colors: &Colors,
    lod: Lod,
) -> Arc<FloraMesh> {
    let s = species.sane();
    let variation = variation.clamp(0.0, 1.0);
    let mut c = [0u8; 12];
    c[..3].copy_from_slice(&colors.leaves);
    c[3..6].copy_from_slice(&colors.bark);
    c[6..9].copy_from_slice(&colors.flowers);
    c[9..].copy_from_slice(&colors.fruit);
    let key = (s.key(seed, variation), c, lod);
    if let Some(m) = mesh_cache().lock().ok().and_then(|n| n.get(&key).cloned()) {
        return m;
    }
    let m = Arc::new(plant(&s, seed, variation).mesh(colors, lod));
    if let Ok(mut n) = mesh_cache().lock() {
        if n.len() > 3000 {
            n.clear();
        }
        n.insert(key, m.clone());
    }
    m
}
