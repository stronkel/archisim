//! The Exterior brushes of the Maqueta (MQ3, C5): dragged over the
//! ground they raise it, lower it, smooth it or flatten it, paint lawn or
//! gravel on it, or scatter plants. No screen here: `app/brush_ui` takes
//! the mouse and calls these once for each dab of a stroke.

use archisim_core::{
    LevelId, MaterialId, Model, ModelError, NewObject, NewTerrain, ObjectTypeId, Point2, Sculpt,
    SurveyPoint, TerrainId, TerrainPatch,
};
use archisim_geom::junctions::{self, Shape};

/// What a brush does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushKind {
    Raise,
    Lower,
    Smooth,
    /// To the height where the stroke began.
    Flatten,
    Lawn,
    Gravel,
    /// A path of stone slabs.
    Path,
    /// Plants of the species chosen, scattered.
    Plants,
}

impl BrushKind {
    pub const ALL: [BrushKind; 8] = [
        BrushKind::Raise,
        BrushKind::Lower,
        BrushKind::Smooth,
        BrushKind::Flatten,
        BrushKind::Lawn,
        BrushKind::Gravel,
        BrushKind::Path,
        BrushKind::Plants,
    ];

    pub fn name(self) -> &'static str {
        match self {
            BrushKind::Raise => "Subir",
            BrushKind::Lower => "Bajar",
            BrushKind::Smooth => "Alisar",
            BrushKind::Flatten => "Aplanar",
            BrushKind::Lawn => "Césped",
            BrushKind::Gravel => "Grava",
            BrushKind::Path => "Camino",
            BrushKind::Plants => "Esparcir plantas",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            BrushKind::Raise => {
                "Arrastra sobre el terreno para levantarlo, más en el centro del pincel"
            }
            BrushKind::Lower => "Arrastra para hundirlo",
            BrushKind::Smooth => "Arrastra para suavizar los bultos y los escalones",
            BrushKind::Flatten => "Arrastra para dejarlo a la altura del punto donde empiezas",
            BrushKind::Lawn => "Arrastra para pintar césped sobre el terreno",
            BrushKind::Gravel => "Arrastra para pintar grava sobre el terreno",
            BrushKind::Path => {
                "Arrastra a lo largo de un camino para pavimentarlo con losas de piedra"
            }
            BrushKind::Plants => {
                "Arrastra para esparcir plantas de la especie elegida, separadas entre sí"
            }
        }
    }

    /// What it is called in Deshacer.
    pub fn undo_label(self) -> &'static str {
        match self {
            BrushKind::Raise => "Subir terreno",
            BrushKind::Lower => "Bajar terreno",
            BrushKind::Smooth => "Alisar terreno",
            BrushKind::Flatten => "Aplanar terreno",
            BrushKind::Lawn => "Pintar césped",
            BrushKind::Gravel => "Pintar grava",
            BrushKind::Path => "Pintar camino",
            BrushKind::Plants => "Esparcir plantas",
        }
    }
}

/// The brush's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushSettings {
    pub kind: BrushKind,
    /// Its radius (m).
    pub radius: f64,
    /// How much each dab does: metres up or down at its middle, or from 0
    /// to 1 for smoothing and flattening; plants per 10 m².
    pub strength: f64,
    /// The species scattered (none: the first in the project, or an olive
    /// tree from the library).
    pub species: Option<ObjectTypeId>,
}

impl Default for BrushSettings {
    fn default() -> Self {
        BrushSettings {
            kind: BrushKind::Raise,
            radius: 2.0,
            strength: 0.15,
            species: None,
        }
    }
}

/// How far a stroke goes between dabs, in radii.
pub const DAB_STEP: f64 = 0.3;

/// The side of the flat ground made where there is none (m).
const NEW_GROUND: f64 = 40.0;

/// The terrain of a level, made flat (a square of [`NEW_GROUND`] round
/// `at`, at the level's height, in earth) if there is none.
pub fn ensure_terrain(m: &mut Model, level: LevelId, at: Point2) -> Result<TerrainId, ModelError> {
    if let Some(t) = m.terrains_on_level(level).next() {
        return Ok(t.id);
    }
    let z = m.level(level).map_or(0.0, |l| l.elevation);
    let earth = material_named(m, "Tierra", "earth", [150, 122, 92]);
    let h = NEW_GROUND / 2.0;
    let c = Point2::new((at.x / 5.0).round() * 5.0, (at.y / 5.0).round() * 5.0);
    let sp = |dx: f64, dy: f64| SurveyPoint {
        at: c + Point2::new(dx, dy),
        z,
    };
    m.add_terrain(NewTerrain {
        level,
        points: vec![sp(-h, -h), sp(h, -h), sp(h, h), sp(-h, h)],
        material: earth,
        depth: 1.0,
        contour: 0.5,
        lines: Vec::new(),
    })
}

/// A material of the project by its name, made if there is none.
pub fn material_named(m: &mut Model, name: &str, category: &str, rgb: [u8; 3]) -> MaterialId {
    if let Some(x) = m.materials().find(|x| x.name == name) {
        return x.id;
    }
    m.add_material(name, category, rgb, archisim_core::CutPattern::Solid)
}

/// What a painting brush paints: its material (made with its texture if
/// the project has none).
pub fn paint_material(m: &mut Model, kind: BrushKind) -> Option<MaterialId> {
    use archisim_core::TextureKind as T;
    let (name, category, rgb, texture) = match kind {
        BrushKind::Lawn => ("Césped", "grass", [112, 158, 82], T::Grass),
        BrushKind::Gravel => ("Grava", "gravel", [196, 190, 176], T::Gravel),
        BrushKind::Path => ("Losas de piedra", "stone", [182, 172, 156], T::Stone),
        _ => return None,
    };
    let fresh = !m.materials().any(|x| x.name == name);
    let id = material_named(m, name, category, rgb);
    if fresh {
        let _ = m.set_appearance(
            id,
            archisim_core::Appearance {
                color: None,
                texture: Some(texture),
                scale: None,
            },
        );
    }
    Some(id)
}

/// A circle as a polygon of 32 sides, counter-clockwise.
pub fn circle(c: Point2, r: f64) -> Vec<Point2> {
    (0..32)
        .map(|i| {
            let a = i as f64 / 32.0 * std::f64::consts::TAU;
            c + Point2::new(r * a.cos(), r * a.sin())
        })
        .collect()
}

/// Shapes of the core's regions.
fn shapes(regions: &[(Vec<Point2>, Vec<Vec<Point2>>)]) -> Vec<Shape> {
    regions.to_vec()
}

/// One dab of a sculpting brush.
pub fn sculpt(
    m: &mut Model,
    terrain: TerrainId,
    kind: BrushKind,
    at: Point2,
    radius: f64,
    strength: f64,
    flat: f64,
) -> Result<(), ModelError> {
    let how = match kind {
        BrushKind::Raise => Sculpt::Raise,
        BrushKind::Lower => Sculpt::Lower,
        BrushKind::Smooth => Sculpt::Smooth,
        BrushKind::Flatten => Sculpt::Flatten(flat),
        _ => return Ok(()),
    };
    m.edit_terrain(terrain, |t| t.sculpt(at, radius, how, strength))
}

/// One dab of paint: `material` over a circle (inside the terrain); the
/// other patches give way under it.
pub fn paint(
    m: &mut Model,
    terrain: TerrainId,
    material: MaterialId,
    at: Point2,
    radius: f64,
) -> Result<(), ModelError> {
    let t = m
        .terrain(terrain)
        .ok_or(ModelError::UnknownTerrain(terrain))?
        .clone();
    let edge: Vec<Point2> = t.boundary().iter().map(|i| t.points[*i].at).collect();
    let dab: Vec<Shape> = vec![(circle(at, radius), Vec::new())];
    let dab = if edge.len() >= 3 {
        junctions::intersection(&dab, &[(archisim_geom::ensure_ccw(edge), Vec::new())])
    } else {
        dab
    };
    if dab.is_empty() {
        return Ok(());
    }
    let mut patches: Vec<TerrainPatch> = Vec::new();
    let mut painted = false;
    for p in &t.patches {
        let regions = if p.material == material {
            painted = true;
            junctions::union(&shapes(&p.regions), &dab)
        } else {
            junctions::difference(&shapes(&p.regions), &dab)
        };
        patches.push(TerrainPatch {
            material: p.material,
            regions,
        });
    }
    if !painted {
        patches.push(TerrainPatch {
            material,
            regions: dab,
        });
    }
    m.edit_terrain(terrain, |t| t.set_patches(patches))
}

/// A small, repeatable random number generator.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Whether an object type is a plant (a tree, a shrub, a hedge, a grown
/// species).
pub fn is_plant(m: &Model, t: ObjectTypeId) -> bool {
    m.type_grows(t)
        || m.object_type(t)
            .and_then(|x| m.family(x.family))
            .is_some_and(|f| f.def().category == "site")
}

/// One dab of the plants brush: plants of `species` in the circle, about
/// `density` per 10 m², none nearer another plant than its own width. They
/// stand on the terrain. Returns the plants made.
pub fn scatter(
    m: &mut Model,
    level: LevelId,
    species: ObjectTypeId,
    at: Point2,
    radius: f64,
    density: f64,
    seed: u64,
) -> Result<Vec<archisim_core::ObjectId>, ModelError> {
    let (lo, hi) = m
        .object_type_box_3d(species)
        .unwrap_or(([-0.5, -0.5, 0.0], [0.5, 0.5, 1.0]));
    let spacing = ((hi[0] - lo[0]).max(hi[1] - lo[1]) * 0.8).max(0.3);
    let area = std::f64::consts::PI * radius * radius;
    let wanted = ((area / 10.0) * density.max(0.1)).round().max(1.0) as usize;
    let elevation = m.level(level).map_or(0.0, |l| l.elevation);
    let plants: Vec<Point2> = m
        .objects()
        .filter(|o| o.level == level && is_plant(m, o.object_type))
        .map(|o| o.position)
        .collect();
    let mut rng = Rng(seed | 1);
    let mut placed: Vec<Point2> = Vec::new();
    let mut made = Vec::new();
    for _ in 0..wanted * 8 {
        if placed.len() >= wanted {
            break;
        }
        let (u, v) = (rng.next(), rng.next());
        let r = radius * u.sqrt();
        let a = v * std::f64::consts::TAU;
        let p = at + Point2::new(r * a.cos(), r * a.sin());
        if plants
            .iter()
            .chain(placed.iter())
            .any(|q| q.distance(p) < spacing)
        {
            continue;
        }
        let ground = m
            .terrains_on_level(level)
            .find_map(|t| t.height_at(p))
            .unwrap_or(elevation);
        let id = m.add_object(NewObject {
            level,
            object_type: species,
            position: p,
            rotation: rng.next() * std::f64::consts::TAU,
            base_offset: ground - elevation,
        })?;
        placed.push(p);
        made.push(id);
    }
    Ok(made)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site() -> (Model, LevelId, TerrainId) {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let t = ensure_terrain(&mut m, l, Point2::new(0.0, 0.0)).unwrap();
        (m, l, t)
    }

    #[test]
    fn raising_and_flattening_the_ground() {
        let (mut m, _, t) = site();
        // Flat at first, four corners only.
        assert_eq!(m.terrain(t).unwrap().points.len(), 4);
        for _ in 0..4 {
            sculpt(
                &mut m,
                t,
                BrushKind::Raise,
                Point2::new(1.5, 1.5),
                3.0,
                0.25,
                0.0,
            )
            .unwrap();
        }
        let ter = m.terrain(t).unwrap().clone();
        assert!(ter.points.len() > 20, "points added under the brush");
        let top = ter.height_at(Point2::new(1.5, 1.5)).unwrap();
        assert!((top - 1.0).abs() < 0.05, "{top}");
        // Further out, less; beyond the brush, nothing.
        let side = ter.height_at(Point2::new(3.0, 1.5)).unwrap();
        assert!(side > 0.0 && side < top, "{side}");
        assert!(ter.height_at(Point2::new(10.0, 10.0)).unwrap().abs() < 1e-9);
        // Flattened back to 0 where it is brushed long enough.
        for _ in 0..30 {
            sculpt(
                &mut m,
                t,
                BrushKind::Flatten,
                Point2::new(1.5, 1.5),
                3.0,
                0.5,
                0.0,
            )
            .unwrap();
        }
        let z = m
            .terrain(t)
            .unwrap()
            .height_at(Point2::new(1.5, 1.5))
            .unwrap();
        assert!(z.abs() < 0.02, "{z}");
    }

    #[test]
    fn painting_lawn_and_gravel() {
        let (mut m, _, t) = site();
        let lawn = material_named(&mut m, "Césped", "grass", [110, 160, 80]);
        let gravel = material_named(&mut m, "Grava", "gravel", [190, 185, 170]);
        paint(&mut m, t, lawn, Point2::new(0.0, 0.0), 2.0).unwrap();
        paint(&mut m, t, lawn, Point2::new(1.5, 0.0), 2.0).unwrap();
        let area = |m: &Model, mat| {
            m.terrain(t)
                .unwrap()
                .patches
                .iter()
                .filter(|p| p.material == mat)
                .map(|p| p.area())
                .sum::<f64>()
        };
        let one = std::f64::consts::PI * 4.0;
        let a = area(&m, lawn);
        assert!(a > one * 1.2 && a < one * 2.0, "two dabs joined: {a}");
        // Gravel over part of the lawn: the lawn gives way.
        paint(&mut m, t, gravel, Point2::new(0.0, 0.0), 1.0).unwrap();
        let after = area(&m, lawn);
        assert!((a - after - area(&m, gravel)).abs() < 0.05, "{a} {after}");
        // Out of the terrain: nothing.
        paint(&mut m, t, gravel, Point2::new(100.0, 100.0), 1.0).unwrap();
        assert!(area(&m, gravel) < std::f64::consts::PI + 0.01);
    }
}
