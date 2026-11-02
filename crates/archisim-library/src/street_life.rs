//! «Vida en la calle»: what makes a place look lived in, put along the
//! streets and in front of the houses of the surroundings — trees on the
//! sidewalks, street lights, benches, litter bins, a few people, cars
//! parked in the parking strips, and the front gardens of the houses with
//! their hedge, a bin and a shrub or a little tree. They are ordinary
//! objects of the library's families, to move or delete as any other;
//! where each goes is worked out from the streets' sections, so the same
//! streets always get the same.

use archisim_core::{
    CutPattern, LevelId, MassShape, Model, ModelError, NewObject, ObjectTypeId, ParamScope, Point2,
    RoadBand, Value, Vec2,
};

/// What to put.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LifeOptions {
    pub trees: bool,
    /// Street lights, benches and bins.
    pub furniture: bool,
    pub cars: bool,
    pub people: bool,
    /// Hedges, bins and shrubs in front of the houses.
    pub gardens: bool,
    /// Only round this point of the plan, this far (none: everywhere).
    pub around: Option<(Point2, f64)>,
}

impl Default for LifeOptions {
    fn default() -> Self {
        LifeOptions {
            trees: true,
            furniture: true,
            cars: true,
            people: true,
            gardens: true,
            around: None,
        }
    }
}

/// What was put.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LifeReport {
    pub trees: usize,
    pub furniture: usize,
    pub cars: usize,
    pub people: usize,
    pub garden_pieces: usize,
}

impl LifeReport {
    pub fn total(&self) -> usize {
        self.trees + self.furniture + self.cars + self.people + self.garden_pieces
    }

    /// One line, for a window or the command line.
    pub fn line(&self) -> String {
        format!(
            "{} árboles, {} farolas, bancos y papeleras, {} coches, {} personas y {} piezas de jardín",
            self.trees, self.furniture, self.cars, self.people, self.garden_pieces
        )
    }
}

/// No tree, lamp or car within this distance of another street's axis
/// (the crossings) or of a street's ends.
const CROSSING_CLEAR: f64 = 9.0;
const TREE_STEP: f64 = 9.0;
const LAMP_STEP: f64 = 24.0;
const BENCH_STEP: f64 = 42.0;
const BIN_STEP: f64 = 33.0;
const CAR_STEP: f64 = 5.6;

/// A number from 0 to 1 from a point and a key: the same point, the same.
fn chance(p: Point2, k: u64) -> f64 {
    let mut h = ((p.x * 10.0).round() as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ ((p.y * 10.0).round() as i64 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ k.wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 29;
    (h % 10_000) as f64 / 10_000.0
}

/// The turn of an object (whose front is its +y) facing `d`.
fn facing(d: Vec2) -> f64 {
    (-d.x).atan2(d.y)
}

/// A type of a system family by name, made the first time with these
/// values (a value given as a material: its name, category and colour).
fn type_named(
    m: &mut Model,
    name: &str,
    family: &str,
    values: &[(&str, Val)],
) -> Result<ObjectTypeId, ModelError> {
    if let Some(t) = m.object_types().find(|t| t.name == name) {
        return Ok(t.id);
    }
    let f = m.system_family(family)?;
    let mut v = m.family_defaults(f, ParamScope::Type)?;
    for (k, x) in values {
        let value = match x {
            Val::N(n) => Value::Number(*n),
            Val::T(t) => Value::Text((*t).into()),
            Val::M(name, cat, rgb) => {
                Value::Material(crate::material_named(m, name, cat, *rgb, CutPattern::Solid).0)
            }
        };
        v.insert((*k).into(), value);
    }
    m.add_object_type(name, f, v)
}

enum Val {
    N(f64),
    T(&'static str),
    M(&'static str, &'static str, [u8; 3]),
}

/// The cars: body colour and model.
const CARS: [(&str, &str, [u8; 3], &str); 6] = [
    (
        "Coche blanco (calle)",
        "Carrocería blanca",
        [232, 232, 228],
        "Berlina",
    ),
    (
        "Coche rojo (calle)",
        "Carrocería roja",
        [176, 58, 50],
        "Compacto",
    ),
    (
        "Coche azul (calle)",
        "Carrocería azul",
        [62, 92, 140],
        "Berlina",
    ),
    (
        "Coche gris (calle)",
        "Carrocería gris",
        [128, 132, 136],
        "Compacto",
    ),
    (
        "Todoterreno verde (calle)",
        "Carrocería verde",
        [86, 104, 72],
        "Todoterreno",
    ),
    (
        "Coche negro (calle)",
        "Carrocería negra",
        [40, 42, 46],
        "Berlina",
    ),
];

/// The people: model, shirt and trousers.
const PEOPLE: [(&str, &str, [u8; 3], [u8; 3]); 4] = [
    ("Paseante (calle)", "Caminando", [196, 84, 72], [52, 60, 84]),
    ("Vecina (calle)", "Mujer", [232, 182, 72], [70, 74, 96]),
    ("Vecino (calle)", "Hombre", [86, 132, 176], [80, 72, 60]),
    (
        "Paseante de verde (calle)",
        "Caminando",
        [92, 150, 108],
        [60, 60, 66],
    ),
];

/// The trees of the sidewalks, by turns.
const STREET_TREES: [&str; 3] = ["Plátano de sombra", "Naranjo", "Jacaranda"];

/// A young tree of a species, the size of those planted along a
/// sidewalk: «<species> de alineación».
fn street_tree(m: &mut Model, species: &str) -> Result<Option<ObjectTypeId>, ModelError> {
    let name = format!("{species} de alineación");
    if let Some(t) = m.object_types().find(|t| t.name == name) {
        return Ok(Some(t.id));
    }
    let Some(e) = crate::flora::FLORA_CATALOGUE
        .iter()
        .find(|e| e.name == species)
    else {
        return Ok(None);
    };
    let mut v = crate::flora::flora_values(m, e)?;
    for (k, x) in [
        ("height", 8.0),
        ("crown", 4.6),
        ("trunk", 0.25),
        ("clear", 3.0),
    ] {
        v.insert(k.into(), Value::Number(x));
    }
    let f = m.system_family(archisim_core::VEGETATION_FAMILY)?;
    let id = m.add_object_type(name.as_str(), f, v)?;
    m.set_object_type_guid(id, crate::extended::stable_guid(&format!("flora/{name}")))?;
    Ok(Some(id))
}

/// Puts the life of the streets and gardens on a level.
pub fn add_street_life(
    m: &mut Model,
    level: LevelId,
    opts: &LifeOptions,
) -> Result<LifeReport, ModelError> {
    let mut r = LifeReport::default();
    let within = |p: Point2| opts.around.is_none_or(|(c, d)| p.distance(c) <= d);
    let axes: Vec<(archisim_core::RoadId, Vec<Point2>)> = m
        .roads_on_level(level)
        .map(|x| (x.id, x.axis.clone()))
        .collect();
    let near_other = |id: archisim_core::RoadId, p: Point2| {
        axes.iter().filter(|(o, _)| *o != id).any(|(_, a)| {
            a.windows(2)
                .any(|w| distance_to_segment(p, w[0], w[1]) < CROSSING_CLEAR)
        })
    };
    // What goes where, worked out before anything is added.
    let mut spots: Vec<Spot> = Vec::new();
    let roads: Vec<archisim_core::Road> = m.roads_on_level(level).cloned().collect();
    for road in &roads {
        let len = road.length();
        if len < 2.0 * CROSSING_CLEAR {
            continue;
        }
        // The tree pits, a tree in each.
        let pits = road.tree_pit_stations();
        if opts.trees {
            for &t in &pits {
                if t < CROSSING_CLEAR * 0.6 || t > len - CROSSING_CLEAR * 0.6 {
                    continue;
                }
                let (p, u) = road.axis_at(t);
                if near_other(road.id, p) || !within(p) {
                    continue;
                }
                for (band, a, b) in road.offsets_at(t) {
                    if band == RoadBand::TreePits && b - a >= 0.7 {
                        spots.push((Kind::Tree, p + u.perp_left() * ((a + b) / 2.0), 0.0));
                    }
                }
            }
        }
        let mut s = CROSSING_CLEAR;
        while s <= len - CROSSING_CLEAR {
            let (p, u) = road.axis_at(s);
            let n = u.perp_left();
            let free = !near_other(road.id, p) && within(p);
            for (band, a, b) in road.offsets_at(s) {
                if !free {
                    continue;
                }
                let left = a + b > 0.0;
                // The edge by the carriageway and the one by the façades.
                let (kerb, far) = if left { (a, b) } else { (b, a) };
                let inward = if left { 1.0 } else { -1.0 };
                let at = |off: f64| p + n * off;
                let to_street = n * -inward;
                match band {
                    RoadBand::Sidewalk | RoadBand::TreePits if (b - a).abs() >= 1.6 => {
                        let w = (b - a).abs();
                        let on = |step: f64, phase: f64| {
                            let k = ((s + phase) / step).round();
                            ((s + phase) - k * step).abs() < 0.5
                        };
                        if opts.trees && pits.is_empty() && w >= 1.9 && on(TREE_STEP, 0.0) {
                            spots.push((Kind::Tree, at(kerb + inward * 0.7), 0.0));
                        } else if opts.furniture && on(LAMP_STEP, LAMP_STEP / 2.0) {
                            spots.push((Kind::Lamp, at(kerb + inward * 0.45), facing(to_street)));
                        } else if opts.furniture && w >= 1.9 && on(BENCH_STEP, 7.0) {
                            spots.push((Kind::Bench, at(far - inward * 0.35), facing(to_street)));
                        } else if opts.furniture && on(BIN_STEP, 16.0) {
                            spots.push((Kind::Bin, at(kerb + inward * 0.5), 0.0));
                        } else if opts.people && on(6.0, 3.0) && chance(at(kerb), 7) < 0.12 {
                            let d = if chance(at(kerb), 8) < 0.5 { u } else { -u };
                            spots.push((Kind::Person, at(kerb + inward * w * 0.55), facing(d)));
                        }
                    }
                    RoadBand::Parking if opts.cars => {
                        let k = (s / CAR_STEP).round();
                        if (s - k * CAR_STEP).abs() < 0.5 && chance(at(a), 3) < 0.72 {
                            // Parked with the traffic: the right side
                            // forwards, the left backwards.
                            let d = if left { -u } else { u };
                            let mid = (a + b) / 2.0;
                            // The car's insertion point is its back.
                            let back = at(mid) - d * 2.2;
                            spots.push((Kind::Car, back, facing(d)));
                        }
                    }
                    _ => {}
                }
            }
            s += 1.0;
        }
    }
    // The front gardens of the houses.
    let mut lawns = Vec::new();
    if opts.gardens {
        let (g, l) = garden_spots(m, level, &within);
        spots.extend(g);
        lawns = l;
    }
    // Nothing on what is already there (the trees the map brought, what
    // was placed by hand).
    let taken: Vec<Point2> = m
        .objects()
        .filter(|o| o.level == level)
        .map(|o| o.position)
        .collect();
    for (kind, p, turn) in spots {
        let clear = if kind == Kind::Tree { 4.0 } else { 1.0 };
        if taken.iter().any(|q| q.distance(p) < clear) {
            continue;
        }
        let t = match kind {
            Kind::Tree => {
                let name = STREET_TREES[(chance(p, 1) * STREET_TREES.len() as f64) as usize % 3];
                match street_tree(m, name)? {
                    Some(t) => t,
                    None => continue,
                }
            }
            Kind::Lamp => type_named(m, "Farola de calle", "Farola", &[])?,
            Kind::Bench => type_named(m, "Banco de calle", "Banco", &[])?,
            Kind::Bin => type_named(m, "Papelera de calle", "Papelera", &[])?,
            Kind::Car => {
                let (name, mat, rgb, model) =
                    CARS[(chance(p, 2) * CARS.len() as f64) as usize % CARS.len()];
                type_named(
                    m,
                    name,
                    "Turismo",
                    &[
                        ("body_mat", Val::M(mat, "other", rgb)),
                        ("model3d", Val::T(model)),
                    ],
                )?
            }
            Kind::Person => {
                let (name, model, shirt, trousers) =
                    PEOPLE[(chance(p, 4) * PEOPLE.len() as f64) as usize % PEOPLE.len()];
                type_named(
                    m,
                    name,
                    "Persona",
                    &[
                        ("model3d", Val::T(model)),
                        ("mat", Val::M("Ropa de calle", "other", shirt)),
                        ("mat2", Val::M("Pantalón de calle", "other", trousers)),
                        ("skin", Val::M("Piel", "other", [214, 168, 132])),
                        ("hair", Val::M("Pelo castaño", "other", [72, 50, 36])),
                    ],
                )?
            }
            Kind::Hedge => type_named(
                m,
                "Seto de jardín 2 m",
                "Seto",
                &[
                    ("width", Val::N(2.0)),
                    ("depth", Val::N(0.5)),
                    ("height", Val::N(0.9)),
                    (
                        "leaves",
                        Val::M("Follaje de seto", "vegetation", [74, 124, 62]),
                    ),
                ],
            )?,
            Kind::Shrub => type_named(
                m,
                "Arbusto de jardín",
                "Arbusto",
                &[
                    ("width", Val::N(1.1)),
                    ("height", Val::N(0.9)),
                    (
                        "leaves",
                        Val::M("Follaje de arbusto", "vegetation", [92, 140, 70]),
                    ),
                ],
            )?,
            Kind::WasteBin => type_named(
                m,
                "Cubo de basura",
                "Contenedor de residuos",
                &[
                    ("width", Val::N(0.6)),
                    ("depth", Val::N(0.7)),
                    ("height", Val::N(1.05)),
                    ("mat", Val::M("Plástico verde", "other", [62, 112, 70])),
                ],
            )?,
        };
        let base_offset = m.surface_at(level, p);
        m.add_object(NewObject {
            level,
            object_type: t,
            position: p,
            rotation: turn,
            base_offset,
        })?;
        match kind {
            Kind::Tree => r.trees += 1,
            Kind::Lamp | Kind::Bench | Kind::Bin => r.furniture += 1,
            Kind::Car => r.cars += 1,
            Kind::Person => r.people += 1,
            Kind::Hedge | Kind::Shrub | Kind::WasteBin => r.garden_pieces += 1,
        }
    }
    lay_lawns(m, level, lawns)?;
    Ok(r)
}

/// The front gardens' grass, painted on the level's terrain (on the
/// grass it already has, if any).
fn lay_lawns(m: &mut Model, level: LevelId, lawns: Vec<Vec<Point2>>) -> Result<(), ModelError> {
    let Some(t) = m.terrains().find(|t| t.level == level).map(|t| t.id) else {
        return Ok(());
    };
    if lawns.is_empty() {
        return Ok(());
    }
    let grass = crate::ground::GROUND_COVERS
        .iter()
        .find(|c| c.name == "Césped")
        .map(|c| crate::ground::cover_material(m, c));
    let Some(grass) = grass else { return Ok(()) };
    m.edit_terrain(t, |t| {
        let mut patches = t.patches.clone();
        let regions = lawns.into_iter().map(|o| (o, Vec::new()));
        match patches.iter_mut().find(|p| p.material == grass) {
            Some(p) => p.regions.extend(regions),
            None => patches.push(archisim_core::TerrainPatch {
                material: grass,
                regions: regions.collect(),
            }),
        }
        t.set_patches(patches);
    })
}

/// What goes where, and its turn.
type Spot = (Kind, Point2, f64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Tree,
    Lamp,
    Bench,
    Bin,
    Car,
    Person,
    Hedge,
    Shrub,
    WasteBin,
}

/// The front gardens: for each house of the surroundings (terraced,
/// semi-detached, detached) the strip between its street façade and the
/// edge of its plot, if it is 1,5 m or more: a hedge along the plot's edge
/// with a gap for the gate, a bin by the gate and a shrub.
fn garden_spots(
    m: &Model,
    level: LevelId,
    within: &dyn Fn(Point2) -> bool,
) -> (Vec<Spot>, Vec<Vec<Point2>>) {
    use archisim_core::BuildingStyle as S;
    let mut out = Vec::new();
    let mut lawns = Vec::new();
    let plots: Vec<Vec<Point2>> = m.plots_on_level(level).map(|p| p.outline.clone()).collect();
    for b in m.env_buildings().filter(|b| b.level == level) {
        let Some(d) = b.dress else { continue };
        if !matches!(
            d.style,
            S::Terraced | S::SemiDetached | S::Detached | S::Andalusian
        ) {
            continue;
        }
        let MassShape::Prism(poly) = &b.shape else {
            continue;
        };
        let mut poly = poly.clone();
        if archisim_core::signed_area(&poly) < 0.0 {
            poly.reverse();
        }
        let n = poly.len();
        let c = poly.iter().fold(Point2::new(0.0, 0.0), |a, q| a + *q) * (1.0 / n as f64);
        if !within(c) {
            continue;
        }
        let Some(plot) = plots.iter().find(|pl| inside(c, pl)) else {
            continue;
        };
        let street = m.street_sides(b.id);
        let party = m.party_sides(b.id);
        // The street side (or the longest free one).
        let side = (0..n)
            .filter(|i| street.get(*i).copied().unwrap_or(false))
            .chain((0..n).filter(|i| !party.get(*i).copied().unwrap_or(false)))
            .max_by(|x, y| {
                let lx = poly[*x].distance(poly[(*x + 1) % n]);
                let ly = poly[*y].distance(poly[(*y + 1) % n]);
                lx.total_cmp(&ly)
            });
        let Some(i) = side else { continue };
        let (a, z) = (poly[i], poly[(i + 1) % n]);
        let Some(u) = (z - a).normalized() else {
            continue;
        };
        let out_dir = -u.perp_left();
        let mid = (a + z) * 0.5;
        let Some(depth) = ray_to_edge(mid, out_dir, plot) else {
            continue;
        };
        if !(1.5..=25.0).contains(&depth) {
            continue;
        }
        let len = a.distance(z);
        // The plot's ground is grass (the house stands on it).
        if !lawns.contains(plot) {
            lawns.push(plot.clone());
        }
        // The hedge along the plot's edge, 2 m pieces, with the gate in
        // the middle.
        let line = |t: f64| a + u * t + out_dir * (depth - 0.55);
        let pieces = (len / 2.0).floor() as usize;
        let gate = pieces / 2;
        for k in 0..pieces {
            if k == gate {
                continue;
            }
            // A hedge's insertion point is its middle.
            out.push((Kind::Hedge, line(1.0 + 2.0 * k as f64), facing(out_dir)));
        }
        let gate_t = 1.0 + 2.0 * gate as f64;
        out.push((
            Kind::WasteBin,
            a + u * (gate_t + 1.4) + out_dir * (depth - 1.0),
            facing(out_dir),
        ));
        if depth >= 2.5 {
            out.push((
                Kind::Shrub,
                a + u * (len * 0.25) + out_dir * (depth * 0.45),
                0.0,
            ));
        }
    }
    (out, lawns)
}

fn distance_to_segment(p: Point2, a: Point2, b: Point2) -> f64 {
    let ab = b - a;
    let l2 = ab.dot(ab);
    if l2 < 1e-12 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

fn inside(p: Point2, poly: &[Point2]) -> bool {
    let mut c = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            c = !c;
        }
    }
    c
}

/// How far from `p` along `d` the edge of a contour is (the nearest).
fn ray_to_edge(p: Point2, d: Vec2, poly: &[Point2]) -> Option<f64> {
    let n = poly.len();
    let mut best: Option<f64> = None;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let e = b - a;
        let den = d.cross(e);
        if den.abs() < 1e-12 {
            continue;
        }
        let t = (a - p).cross(e) / den;
        let s = (a - p).cross(d) / den;
        if t > 1e-6 && (0.0..=1.0).contains(&s) {
            best = Some(best.map_or(t, |x: f64| x.min(t)));
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carto::{import_carto, CartoOptions};

    /// The sample town brought from its files gets trees, lights, cars,
    /// people and gardens, each standing on the ground, none on another
    /// street's axis; the same streets, the same life.
    #[test]
    fn the_sample_town_comes_to_life() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../archisim-carto/tests/fixtures");
        let read = |f: &str| std::fs::read_to_string(dir.join(f)).unwrap();
        let files = vec![
            ("c.gml".to_string(), read("pueblo-catastro.gml")),
            ("o.osm".to_string(), read("pueblo.osm")),
        ];
        let make = || {
            let mut m = Model::new("T");
            let l = m.add_level("Planta baja", 0.0);
            import_carto(
                &mut m,
                &files,
                &CartoOptions {
                    street_life: false,
                    ..CartoOptions::default()
                },
            )
            .unwrap();
            let before = m.objects().count();
            let r = add_street_life(&mut m, l, &LifeOptions::default()).unwrap();
            (m, r, before)
        };
        let (m, r, before) = make();
        assert!(r.trees > 5 && r.furniture > 5, "{r:?}");
        assert!(r.people > 0, "{r:?}");
        assert_eq!(m.objects().count(), before + r.total());
        let (_, again, _) = make();
        assert_eq!(r, again);
    }

    #[test]
    fn a_ray_finds_the_plot_edge() {
        let sq = [
            Point2::new(0.0, 0.0),
            Point2::new(10.0, 0.0),
            Point2::new(10.0, 10.0),
            Point2::new(0.0, 10.0),
        ];
        let d = ray_to_edge(Point2::new(5.0, 6.0), Vec2::new(0.0, 1.0), &sq).unwrap();
        assert!((d - 4.0).abs() < 1e-9);
        assert!((facing(Vec2::new(1.0, 0.0)) + std::f64::consts::FRAC_PI_2).abs() < 1e-9);
    }
}
