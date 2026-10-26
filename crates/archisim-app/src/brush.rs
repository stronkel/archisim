//! The Exterior brush (MQ3 in C5, one brush in C8, as in Paralives): it
//! paints covers over the ground (lawns, meadows with flowers, earths,
//! sand, gravel, paving), rubs them out, raises, lowers, smooths or
//! flattens the ground, or scatters plants and rocks; with a round, square,
//! uneven or splattering head. No screen here: `app/brush_ui` takes the
//! mouse and calls these once for each dab of a stroke.

use archisim_core::{
    LevelId, MaterialId, Model, ModelError, NewObject, NewTerrain, ObjectId, ObjectTypeId, Point2,
    Sculpt, SurveyPoint, TerrainId, TerrainPatch,
};
use archisim_geom::junctions::{self, Shape};

/// What the brush does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushMode {
    /// Paints the cover chosen.
    Paint,
    /// Rubs out what is painted (and, if asked, plants and rocks).
    Erase,
    Raise,
    Lower,
    Smooth,
    /// To the height where the stroke began.
    Flatten,
    /// Plants or rocks of the type chosen, scattered.
    Scatter,
}

impl BrushMode {
    pub const ALL: [BrushMode; 7] = [
        BrushMode::Paint,
        BrushMode::Erase,
        BrushMode::Raise,
        BrushMode::Lower,
        BrushMode::Smooth,
        BrushMode::Flatten,
        BrushMode::Scatter,
    ];

    pub fn name(self) -> &'static str {
        match self {
            BrushMode::Paint => "Pintar",
            BrushMode::Erase => "Borrar",
            BrushMode::Raise => "Subir",
            BrushMode::Lower => "Bajar",
            BrushMode::Smooth => "Alisar",
            BrushMode::Flatten => "Aplanar",
            BrushMode::Scatter => "Esparcir",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            BrushMode::Paint => "Arrastra para pintar sobre el terreno lo elegido en la paleta",
            BrushMode::Erase => "Arrastra para borrar lo pintado (y, si lo pides, plantas y rocas)",
            BrushMode::Raise => {
                "Arrastra sobre el terreno para levantarlo, más en el centro del pincel"
            }
            BrushMode::Lower => "Arrastra para hundirlo",
            BrushMode::Smooth => "Arrastra para suavizar los bultos y los escalones",
            BrushMode::Flatten => "Arrastra para dejarlo a la altura del punto donde empiezas",
            BrushMode::Scatter => {
                "Arrastra para esparcir plantas o rocas del tipo elegido, separadas entre sí"
            }
        }
    }

    /// What it is called in Deshacer.
    pub fn undo_label(self) -> &'static str {
        match self {
            BrushMode::Paint => "Pintar el terreno",
            BrushMode::Erase => "Borrar con el pincel",
            BrushMode::Raise => "Subir terreno",
            BrushMode::Lower => "Bajar terreno",
            BrushMode::Smooth => "Alisar terreno",
            BrushMode::Flatten => "Aplanar terreno",
            BrushMode::Scatter => "Esparcir",
        }
    }

    /// Whether it changes the ground's height.
    pub fn sculpts(self) -> bool {
        matches!(
            self,
            BrushMode::Raise | BrushMode::Lower | BrushMode::Smooth | BrushMode::Flatten
        )
    }

    /// Whether the head's shape counts (the sculpting brushes are round,
    /// softer at their edge).
    pub fn has_head(self) -> bool {
        !self.sculpts()
    }
}

/// The shape of the brush's head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushHead {
    Round,
    Square,
    /// Round, its edge uneven: strokes that look laid by hand.
    Soft,
    /// A splatter of little blots inside the circle.
    Splatter,
}

impl BrushHead {
    pub const ALL: [BrushHead; 4] = [
        BrushHead::Round,
        BrushHead::Square,
        BrushHead::Soft,
        BrushHead::Splatter,
    ];

    pub fn name(self) -> &'static str {
        match self {
            BrushHead::Round => "Redondo",
            BrushHead::Square => "Cuadrado",
            BrushHead::Soft => "Borde irregular",
            BrushHead::Splatter => "Salpicado",
        }
    }
}

/// The brush's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushSettings {
    pub mode: BrushMode,
    pub head: BrushHead,
    /// Its radius (half the side of the square head), m.
    pub radius: f64,
    /// How much each dab does: metres up or down at its middle, or from 0
    /// to 1 for smoothing and flattening; plants per 10 m².
    pub strength: f64,
    /// The cover painted: its index in
    /// `archisim_library::ground::GROUND_COVERS`.
    pub cover: usize,
    /// The type scattered (none: the first plant in the project, or an
    /// olive tree from the library).
    pub species: Option<ObjectTypeId>,
    /// Scatter the other types of the same family too, in turn (rocks of
    /// every variant).
    pub vary: bool,
    /// Rubbing out takes the plants and rocks under the brush too.
    pub erase_plants: bool,
}

impl Default for BrushSettings {
    fn default() -> Self {
        BrushSettings {
            mode: BrushMode::Paint,
            head: BrushHead::Round,
            radius: 2.0,
            strength: 0.15,
            cover: 0,
            species: None,
            vary: true,
            erase_plants: false,
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

/// The cover painted: its material, made with how it looks (and what
/// grows on it) if the project has none.
pub fn cover_material(m: &mut Model, cover: usize) -> MaterialId {
    let covers = &archisim_library::ground::GROUND_COVERS;
    let c = &covers[cover.min(covers.len() - 1)];
    archisim_library::ground::cover_material(m, c)
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

/// The shape one dab of the head leaves at `at`: a circle, a square, a
/// circle with an uneven edge or a splatter of blots (both changing from
/// dab to dab with `seed`).
pub fn dab_shape(head: BrushHead, at: Point2, radius: f64, seed: u64) -> Vec<Shape> {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    match head {
        BrushHead::Round => vec![(circle(at, radius), Vec::new())],
        BrushHead::Square => {
            let r = radius;
            vec![(
                vec![
                    at + Point2::new(-r, -r),
                    at + Point2::new(r, -r),
                    at + Point2::new(r, r),
                    at + Point2::new(-r, r),
                ],
                Vec::new(),
            )]
        }
        BrushHead::Soft => {
            let phase = [rng.next(), rng.next(), rng.next()].map(|x| x * std::f64::consts::TAU);
            let ring = (0..40)
                .map(|i| {
                    let a = i as f64 / 40.0 * std::f64::consts::TAU;
                    let f = 0.82
                        + 0.1 * (3.0 * a + phase[0]).sin()
                        + 0.06 * (5.0 * a + phase[1]).sin()
                        + 0.04 * (9.0 * a + phase[2]).sin();
                    at + Point2::new(a.cos(), a.sin()) * (radius * f)
                })
                .collect();
            vec![(ring, Vec::new())]
        }
        BrushHead::Splatter => {
            let blots: Vec<Shape> = (0..7)
                .map(|_| {
                    let (u, v) = (rng.next(), rng.next());
                    let r = radius * 0.72 * u.sqrt();
                    let a = v * std::f64::consts::TAU;
                    let size = radius * (0.12 + 0.2 * rng.next());
                    (
                        circle(at + Point2::new(r * a.cos(), r * a.sin()), size),
                        Vec::new(),
                    )
                })
                .collect();
            junctions::union(&blots, &[])
        }
    }
}

/// Whether `p` is inside any of `shapes`.
pub fn inside(shapes: &[Shape], p: Point2) -> bool {
    shapes.iter().any(|(o, holes)| {
        archisim_core::point_in_polygon(p, o)
            && !holes.iter().any(|h| archisim_core::point_in_polygon(p, h))
    })
}

/// One dab of a sculpting brush.
pub fn sculpt(
    m: &mut Model,
    terrain: TerrainId,
    mode: BrushMode,
    at: Point2,
    radius: f64,
    strength: f64,
    flat: f64,
) -> Result<(), ModelError> {
    let how = match mode {
        BrushMode::Raise => Sculpt::Raise,
        BrushMode::Lower => Sculpt::Lower,
        BrushMode::Smooth => Sculpt::Smooth,
        BrushMode::Flatten => Sculpt::Flatten(flat),
        _ => return Ok(()),
    };
    m.edit_terrain(terrain, |t| t.sculpt(at, radius, how, strength))
}

/// The part of `dab` over the terrain.
fn on_terrain(t: &archisim_core::Terrain, dab: Vec<Shape>) -> Vec<Shape> {
    let edge: Vec<Point2> = t.boundary().iter().map(|i| t.points[*i].at).collect();
    if edge.len() >= 3 {
        junctions::intersection(&dab, &[(archisim_geom::ensure_ccw(edge), Vec::new())])
    } else {
        dab
    }
}

/// One dab of paint: `material` over `dab` (inside the terrain); the other
/// patches give way under it.
pub fn paint(
    m: &mut Model,
    terrain: TerrainId,
    material: MaterialId,
    dab: Vec<Shape>,
) -> Result<(), ModelError> {
    let t = m
        .terrain(terrain)
        .ok_or(ModelError::UnknownTerrain(terrain))?
        .clone();
    let dab = on_terrain(&t, dab);
    if dab.is_empty() {
        return Ok(());
    }
    let mut patches: Vec<TerrainPatch> = Vec::new();
    let mut painted = false;
    for p in &t.patches {
        let regions = if p.material == material {
            painted = true;
            junctions::union(&p.regions, &dab)
        } else {
            junctions::difference(&p.regions, &dab)
        };
        if !regions.is_empty() {
            patches.push(TerrainPatch {
                material: p.material,
                regions,
            });
        }
    }
    if !painted {
        patches.push(TerrainPatch {
            material,
            regions: dab,
        });
    }
    m.edit_terrain(terrain, |t| t.set_patches(patches))
}

/// One dab of the rubber: what is painted under `dab` goes (the bare
/// ground shows again), and, with `plants`, the plants and rocks standing
/// in it. Returns how many plants and rocks went.
pub fn erase(
    m: &mut Model,
    level: LevelId,
    dab: Vec<Shape>,
    plants: bool,
) -> Result<usize, ModelError> {
    let terrains: Vec<TerrainId> = m.terrains_on_level(level).map(|t| t.id).collect();
    for id in terrains {
        let Some(t) = m.terrain(id) else { continue };
        if t.patches.is_empty() {
            continue;
        }
        let patches: Vec<TerrainPatch> = t
            .patches
            .iter()
            .filter_map(|p| {
                let regions = junctions::difference(&p.regions, &dab);
                (!regions.is_empty()).then_some(TerrainPatch {
                    material: p.material,
                    regions,
                })
            })
            .collect();
        if patches != t.patches {
            m.edit_terrain(id, |t| t.set_patches(patches))?;
        }
    }
    if !plants {
        return Ok(0);
    }
    let gone: Vec<ObjectId> = m
        .objects()
        .filter(|o| o.level == level && is_scattered(m, o.object_type) && inside(&dab, o.position))
        .map(|o| o.id)
        .collect();
    for id in &gone {
        m.remove_object(*id)?;
    }
    Ok(gone.len())
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

/// Whether an object type is something the brush scatters: a plant, or a
/// rock or other natural thing.
pub fn is_scattered(m: &Model, t: ObjectTypeId) -> bool {
    is_plant(m, t)
        || m.object_type(t)
            .and_then(|x| m.family(x.family))
            .is_some_and(|f| f.def().category == "nature")
}

/// The types scattered in turn with `t`: itself, and with `vary` the
/// other types of its family and model (not a grown species: those vary
/// already).
pub fn scatter_types(m: &Model, t: ObjectTypeId, vary: bool) -> Vec<ObjectTypeId> {
    let Some(ty) = m.object_type(t) else {
        return Vec::new();
    };
    if !vary || m.type_grows(t) {
        return vec![t];
    }
    // Of the same family and model (rounded pebbles with rounded pebbles).
    let model3d = |x: &archisim_core::ObjectType| x.values.get("model3d").cloned();
    let mut v: Vec<ObjectTypeId> = m
        .object_types()
        .filter(|x| x.family == ty.family && x.own.is_none() && model3d(x) == model3d(ty))
        .map(|x| x.id)
        .collect();
    if !v.contains(&t) {
        v.insert(0, t);
    }
    v
}

/// One dab of the scattering brush: plants or rocks of `types` (in turn)
/// over `dab` (inside the circle round `at` of `radius`), about `density`
/// per 10 m², none nearer another plant or rock than its own width. They
/// stand on the terrain. Returns those made.
#[allow(clippy::too_many_arguments)]
pub fn scatter(
    m: &mut Model,
    level: LevelId,
    types: &[ObjectTypeId],
    dab: &[Shape],
    at: Point2,
    radius: f64,
    density: f64,
    seed: u64,
) -> Result<Vec<ObjectId>, ModelError> {
    let Some(&first) = types.first() else {
        return Ok(Vec::new());
    };
    let (lo, hi) = m
        .object_type_box_3d(first)
        .unwrap_or(([-0.5, -0.5, 0.0], [0.5, 0.5, 1.0]));
    let spacing = ((hi[0] - lo[0]).max(hi[1] - lo[1]) * 0.8).max(0.3);
    let area = std::f64::consts::PI * radius * radius;
    let wanted = ((area / 10.0) * density.max(0.1)).round().max(1.0) as usize;
    let elevation = m.level(level).map_or(0.0, |l| l.elevation);
    let standing: Vec<Point2> = m
        .objects()
        .filter(|o| o.level == level && is_scattered(m, o.object_type))
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
        if !inside(dab, p) {
            continue;
        }
        if standing
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
        let object_type = types[(rng.next() * types.len() as f64) as usize % types.len()];
        let id = m.add_object(NewObject {
            level,
            object_type,
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
                BrushMode::Raise,
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
                BrushMode::Flatten,
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
        let round = |x: f64, r: f64| dab_shape(BrushHead::Round, Point2::new(x, 0.0), r, 1);
        paint(&mut m, t, lawn, round(0.0, 2.0)).unwrap();
        paint(&mut m, t, lawn, round(1.5, 2.0)).unwrap();
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
        paint(&mut m, t, gravel, round(0.0, 1.0)).unwrap();
        let after = area(&m, lawn);
        assert!((a - after - area(&m, gravel)).abs() < 0.05, "{a} {after}");
        // Out of the terrain: nothing.
        paint(
            &mut m,
            t,
            gravel,
            dab_shape(BrushHead::Round, Point2::new(100.0, 100.0), 1.0, 1),
        )
        .unwrap();
        assert!(area(&m, gravel) < std::f64::consts::PI + 0.01);
        // Rubbed out in the middle: a ring of gravel is left.
        let before = area(&m, gravel);
        let l = m.levels()[0].id;
        erase(&mut m, l, round(0.0, 0.5), false).unwrap();
        let after = area(&m, gravel);
        assert!(
            (before - after - std::f64::consts::PI * 0.25).abs() < 0.05,
            "{before} {after}"
        );
    }

    #[test]
    fn heads_and_scattering_rocks() {
        let at = Point2::new(0.0, 0.0);
        let area = |v: &[Shape]| junctions::area(v);
        let round = area(&dab_shape(BrushHead::Round, at, 1.0, 3));
        let square = area(&dab_shape(BrushHead::Square, at, 1.0, 3));
        let soft = area(&dab_shape(BrushHead::Soft, at, 1.0, 3));
        let splat = dab_shape(BrushHead::Splatter, at, 1.0, 3);
        assert!((square - 4.0).abs() < 1e-9 && round > 3.0 && round < 3.15);
        assert!(soft > 1.5 && soft < round, "{soft}");
        assert!(!splat.is_empty() && area(&splat) < round * 0.6);
        // Two variants of a rock scattered in turn, standing on the ground,
        // and rubbed out again.
        let (mut m, l, _) = site();
        let a = archisim_library::add_object_entry(&mut m, "Roca angulosa 80").unwrap();
        let b = archisim_library::add_object_entry(&mut m, "Roca angulosa 80 (2)").unwrap();
        let types = scatter_types(&m, a, true);
        assert!(types.contains(&a) && types.contains(&b));
        let dab = dab_shape(BrushHead::Round, at, 4.0, 3);
        let made = scatter(&mut m, l, &types, &dab, at, 4.0, 3.0, 7).unwrap();
        assert!(made.len() >= 3, "{}", made.len());
        let kinds: std::collections::HashSet<ObjectTypeId> = made
            .iter()
            .map(|o| m.object(*o).unwrap().object_type)
            .collect();
        assert_eq!(kinds.len(), 2);
        let gone = erase(&mut m, l, dab, true).unwrap();
        assert_eq!(gone, made.len());
    }
}
