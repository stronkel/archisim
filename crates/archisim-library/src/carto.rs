//! The surroundings of a project from free cartography (H7): the
//! cadastre's buildings and parcels, OpenStreetMap's streets, buildings
//! and trees, and a terrain grid (the CNIG's MDT), read by
//! `archisim-carto` and turned into masses by floors, plots, roads by
//! type, trees and a terrain around a centre.
//!
//! Everything is placed by the project's georeference; a project without
//! one takes it from the files (the cadastre's system, or the UTM zone of
//! OpenStreetMap's longitudes) with its origin at the middle of what was
//! read.

use archisim_carto::{asc, catastro, osm, FileKind};
use archisim_core::{
    signed_area, Georeference, LevelId, MassShape, MaterialId, Model, ModelError, NewMass,
    NewObject, NewPlot, NewRoad, NewTerrain, Point2, RoadSection, SurveyPoint,
};

/// What to bring and how far.
#[derive(Debug, Clone, PartialEq)]
pub struct CartoOptions {
    /// Centre of what is brought, in the project's plan (usually 0,0).
    pub centre: Point2,
    /// How far from the centre, metres.
    pub radius: f64,
    pub buildings: bool,
    pub parcels: bool,
    pub streets: bool,
    pub trees: bool,
    pub terrain: bool,
    /// Height of a floor of the buildings brought, metres.
    pub floor_height: f64,
    /// Leave out the buildings over the project's own slabs.
    pub skip_own: bool,
    /// Put the project's origin at the middle of what the files bring,
    /// even if the project is already georeferenced (it is moved there).
    pub recentre: bool,
}

impl Default for CartoOptions {
    fn default() -> Self {
        CartoOptions {
            centre: Point2::new(0.0, 0.0),
            radius: 150.0,
            buildings: true,
            parcels: true,
            streets: true,
            trees: true,
            terrain: true,
            floor_height: 3.0,
            skip_own: true,
            recentre: false,
        }
    }
}

/// What was brought.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CartoReport {
    pub buildings: usize,
    pub plots: usize,
    pub roads: usize,
    pub trees: usize,
    pub terrain_points: usize,
    /// The georeference was taken from the files.
    pub georeferenced: bool,
    /// Things worth saying (files not understood, what was left out).
    pub notes: Vec<String>,
}

impl CartoReport {
    /// One line per kind, for a dialog or the command line.
    pub fn lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.georeferenced {
            out.push("Georreferencia del proyecto tomada de los ficheros".into());
        }
        for (n, what) in [
            (self.buildings, "edificios (volúmenes por plantas)"),
            (self.plots, "parcelas"),
            (self.roads, "calles"),
            (self.trees, "árboles"),
            (self.terrain_points, "puntos de terreno"),
        ] {
            if n > 0 {
                out.push(format!("{n} {what}"));
            }
        }
        out.extend(self.notes.iter().cloned());
        out
    }
}

/// The files read, by kind.
#[derive(Default)]
struct Read {
    cadastre: Vec<catastro::Cadastre>,
    osm: Vec<osm::Osm>,
    grids: Vec<asc::Grid>,
}

/// Map ↔ plan by the georeference and the true north (as the IFC's
/// `IfcMapConversion` writes it).
#[derive(Debug, Clone, Copy)]
struct Frame {
    e: f64,
    n: f64,
    h: f64,
    cos: f64,
    sin: f64,
}

impl Frame {
    fn of(m: &Model, g: &Georeference) -> Frame {
        let a = m.true_north() + g.convergence();
        Frame {
            e: g.eastings,
            n: g.northings,
            h: g.height,
            cos: a.cos(),
            sin: a.sin(),
        }
    }

    fn to_plan(self, p: [f64; 2]) -> Point2 {
        let (dx, dy) = (p[0] - self.e, p[1] - self.n);
        Point2::new(
            dx * self.cos + dy * self.sin,
            -dx * self.sin + dy * self.cos,
        )
    }
}

/// Brings the cartographic files (their names and texts) into the model
/// around `opts.centre`.
pub fn import_carto(
    m: &mut Model,
    files: &[(String, String)],
    opts: &CartoOptions,
) -> Result<CartoReport, ModelError> {
    let mut report = CartoReport::default();
    let mut read = Read::default();
    for (name, text) in files {
        match archisim_carto::kind_of(text) {
            Some(FileKind::Cadastre) => read.cadastre.push(catastro::read(text)),
            Some(FileKind::Osm) => read.osm.push(osm::read(text)),
            Some(FileKind::Grid) => match asc::read(text) {
                Ok(g) => read.grids.push(g),
                Err(e) => report.notes.push(format!("{name}: {e}")),
            },
            None => report.notes.push(format!(
                "{name}: no es GML del Catastro, OSM ni una malla ASC"
            )),
        }
    }
    let had_height = m.georeference().is_some();
    if m.georeference().is_none() || opts.recentre {
        let Some(g) = guess_georeference(&read) else {
            return Err(ModelError::InvalidShape(
                "the project has no georeference and the files give none",
            ));
        };
        let height = m.georeference().map_or(g.height, |old| old.height);
        m.set_georeference(Some(Georeference { height, ..g }))?;
        report.georeferenced = true;
    }
    let g = m.georeference().cloned().expect("set above");
    if let Some(code) = read.cadastre.iter().find_map(|c| c.epsg) {
        if format!("EPSG:{code}") != g.crs {
            report.notes.push(format!(
                "El Catastro viene en EPSG:{code} y el proyecto en {}: se leen como si fueran el mismo",
                g.crs
            ));
        }
    }
    let frame = Frame::of(m, &g);
    let level = ground_level(m);
    let level_z = m.level(level).map_or(0.0, |l| l.elevation);

    // The terrain first: the rest sits on it.
    let mut ground: Vec<SurveyPoint> = Vec::new();
    if opts.terrain {
        for grid in &read.grids {
            ground.extend(grid_points(grid, &frame, opts));
        }
        // A project placed by the files has its ±0,00 on the ground at
        // its origin.
        if !had_height {
            if let Some(z0) = nearest_z(&ground, opts.centre) {
                let z0 = (z0 * 100.0).round() / 100.0;
                ground.iter_mut().for_each(|s| s.z -= z0);
                m.set_georeference(Some(Georeference {
                    height: z0,
                    ..g.clone()
                }))?;
            }
        }
        if ground.len() >= 3 {
            let earth = material(m, "Terreno natural", "earth", [176, 156, 120]);
            let points: Vec<SurveyPoint> = ground
                .iter()
                .map(|s| SurveyPoint {
                    at: s.at,
                    z: s.z - level_z,
                })
                .collect();
            report.terrain_points = points.len();
            m.add_terrain(NewTerrain {
                level,
                points,
                material: earth,
                depth: 1.0,
                contour: 1.0,
                lines: Vec::new(),
            })?;
        }
    }
    let on_ground = |p: Point2| nearest_z(&ground, p).map_or(0.0, |z| z - level_z);

    let own: Vec<Vec<Point2>> = if opts.skip_own {
        m.slabs().map(|s| s.outline.clone()).collect()
    } else {
        Vec::new()
    };
    let inside = |p: Point2| p.distance(opts.centre) <= opts.radius;

    // Buildings: the cadastre's, or OpenStreetMap's when there is none.
    let concept = material(m, "Volumen de concepto", "concept", [222, 222, 214]);
    let mut buildings: Vec<(String, Vec<Point2>, u32)> = Vec::new();
    // The cadastre gives the floors per part of a building (its
    // «buildingpart» file); the buildings alone, without floors, only
    // when there are no parts.
    let parts = read.cadastre.iter().any(|c| {
        c.features
            .iter()
            .any(|f| f.kind == catastro::FeatureKind::BuildingPart)
    });
    let wanted = if parts {
        catastro::FeatureKind::BuildingPart
    } else {
        catastro::FeatureKind::Building
    };
    for c in &read.cadastre {
        for f in &c.features {
            if f.kind != wanted || f.floors_above == Some(0) {
                continue;
            }
            for poly in &f.polygons {
                let outline = plan_ring(&poly.outer, &frame);
                let floors = f.floors_above.unwrap_or(2).max(1);
                buildings.push((name_of("Edificio", &f.reference), outline, floors));
            }
        }
    }
    if buildings.is_empty() {
        for o in &read.osm {
            for w in o
                .ways
                .iter()
                .filter(|w| w.closed && w.tag("building").is_some())
            {
                let outline: Vec<Point2> = w
                    .points
                    .iter()
                    .map(|p| frame.to_plan(utm_of(&g, p[0], p[1])))
                    .collect();
                let floors = w
                    .tag("building:levels")
                    .and_then(|v| v.parse::<f64>().ok())
                    .map_or(2, |v| v.round().max(1.0) as u32);
                buildings.push((
                    name_of("Edificio", w.tag("name").unwrap_or("")),
                    outline,
                    floors,
                ));
            }
        }
    }
    if opts.buildings {
        for (name, mut outline, floors) in buildings {
            dedup_ring(&mut outline);
            if outline.len() < 3 || signed_area(&outline).abs() < 4.0 {
                continue;
            }
            let c = centre_of(&outline);
            if !inside(c) {
                continue;
            }
            if own.iter().any(|s| {
                in_polygon(centre_of(s), &outline) || s.iter().any(|p| in_polygon(*p, &outline))
            }) {
                report
                    .notes
                    .push(format!("{name}: en el sitio del proyecto, no se trae"));
                continue;
            }
            let fh = opts.floor_height.max(2.0);
            m.add_mass(NewMass {
                name,
                level,
                shape: MassShape::Prism(outline),
                base_offset: on_ground(c),
                height: fh * floors as f64,
                floor_height: Some(fh),
                material: concept,
            })?;
            report.buildings += 1;
        }
    }

    // Parcels.
    if opts.parcels {
        for c in &read.cadastre {
            for f in c
                .features
                .iter()
                .filter(|f| f.kind == catastro::FeatureKind::Parcel)
            {
                for poly in &f.polygons {
                    let mut outline = plan_ring(&poly.outer, &frame);
                    dedup_ring(&mut outline);
                    if outline.len() < 3 || !inside(centre_of(&outline)) {
                        continue;
                    }
                    m.add_plot(NewPlot {
                        name: name_of("Parcela", &f.reference),
                        level,
                        outline,
                        setback: 0.0,
                        reference: f.reference.clone(),
                    })?;
                    report.plots += 1;
                }
            }
        }
    }

    // Streets, by their class in OpenStreetMap, narrowed to the room
    // between the parcels (or the buildings) on either side.
    let fronts: Vec<Vec<Point2>> = {
        let parcels: Vec<Vec<Point2>> = read
            .cadastre
            .iter()
            .flat_map(|c| c.features.iter())
            .filter(|f| f.kind == catastro::FeatureKind::Parcel)
            .flat_map(|f| f.polygons.iter().map(|p| plan_ring(&p.outer, &frame)))
            .collect();
        if parcels.is_empty() {
            m.masses()
                .filter_map(|x| match &x.shape {
                    MassShape::Prism(p) => Some(p.clone()),
                    _ => None,
                })
                .collect()
        } else {
            parcels
        }
    };
    if opts.streets {
        let by = |m: &Model, cats: &[&str]| {
            cats.iter()
                .find_map(|c| m.materials().find(|x| x.category == *c))
                .or_else(|| m.materials().next())
                .map_or(MaterialId(0), |x| x.id)
        };
        let pavement = by(m, &["asphalt", "concrete"]);
        let kerb = by(m, &["stone", "concrete"]);
        let sidewalk = by(m, &["ceramic", "stone", "concrete"]);
        for o in &read.osm {
            for w in &o.ways {
                let Some(class) = w.tag("highway") else {
                    continue;
                };
                let Some(section) = street_section(class, w.tag("oneway") == Some("yes")) else {
                    continue;
                };
                let line: Vec<Point2> = w
                    .points
                    .iter()
                    .map(|p| frame.to_plan(utm_of(&g, p[0], p[1])))
                    .collect();
                let pieces = clip_to_circle(&line, opts.centre, opts.radius)
                    .into_iter()
                    .map(|p| simplify(&p, 0.5))
                    .filter(|p| p.len() >= 2 && length(p) >= 5.0);
                for whole in pieces {
                    let section = match room_across(&whole, &fronts) {
                        Some(room) => fit_section(&section, room),
                        None => section.clone(),
                    };
                    // At the ground's mean height along it, and, with a
                    // terrain, following it (its «rasante», U1).
                    {
                        let piece = whole;
                        let spots = along(&piece, 4.0);
                        let top = spots.iter().map(|p| on_ground(*p)).sum::<f64>()
                            / spots.len().max(1) as f64;
                        let id = m.add_road(NewRoad {
                            name: w.tag("name").unwrap_or("Calle").to_string(),
                            level,
                            axis: piece,
                            section,
                            top_offset: top,
                            profile: Vec::new(),
                            pavement,
                            kerb_material: kerb,
                            sidewalk_material: sidewalk,
                        })?;
                        if !ground.is_empty() {
                            if let Some(profile) = m.road_profile_from_terrain(id, 20.0) {
                                m.edit_road(id, |r| r.profile = profile)?;
                            }
                        }
                        report.roads += 1;
                    }
                }
            }
        }
    }

    // Trees: OpenStreetMap's single trees and rows of trees.
    if opts.trees {
        let mut spots: Vec<(Point2, &'static str)> = Vec::new();
        for o in &read.osm {
            for n in o.nodes.iter().filter(|n| n.tag("natural") == Some("tree")) {
                let p = frame.to_plan(utm_of(&g, n.lat, n.lon));
                spots.push((p, species_of(n.tag("leaf_type"), n.tag("genus"))));
            }
            for w in o
                .ways
                .iter()
                .filter(|w| w.tag("natural") == Some("tree_row"))
            {
                let line: Vec<Point2> = w
                    .points
                    .iter()
                    .map(|p| frame.to_plan(utm_of(&g, p[0], p[1])))
                    .collect();
                let sp = species_of(w.tag("leaf_type"), w.tag("genus"));
                for p in along(&line, 8.0) {
                    spots.push((p, sp));
                }
            }
        }
        for (p, name) in spots.into_iter().filter(|(p, _)| inside(*p)) {
            let Some(t) = crate::flora::add_flora_entry(m, name)? else {
                continue;
            };
            m.add_object(NewObject {
                level,
                object_type: t,
                position: p,
                rotation: 0.0,
                base_offset: on_ground(p),
            })?;
            report.trees += 1;
        }
    }
    Ok(report)
}

/// The georeference the files give: the cadastre's system with its origin
/// at the middle of the cadastre's features, or OpenStreetMap's UTM zone
/// (ETRS89) at the middle of its nodes.
fn guess_georeference(read: &Read) -> Option<Georeference> {
    for c in &read.cadastre {
        let Some(code) = c.epsg else { continue };
        let pts: Vec<[f64; 2]> = c
            .features
            .iter()
            .flat_map(|f| f.polygons.iter())
            .flat_map(|p| p.outer.iter().copied())
            .collect();
        if let Some(mid) = middle(&pts) {
            return Some(Georeference {
                crs: format!("EPSG:{code}"),
                eastings: mid[0].round(),
                northings: mid[1].round(),
                height: 0.0,
            });
        }
    }
    for o in &read.osm {
        let pts: Vec<[f64; 2]> = o
            .nodes
            .iter()
            .map(|n| [n.lat, n.lon])
            .chain(o.ways.iter().flat_map(|w| w.points.iter().copied()))
            .collect();
        // Latitude and longitude.
        let Some([lat, lon]) = middle(&pts) else {
            continue;
        };
        let zone = (((lon + 180.0) / 6.0).floor() as u32 + 1).clamp(1, 60);
        let (e, n) = archisim_carto::lat_lon_to_utm(lat, lon, zone, lat >= 0.0);
        let crs = if (28..=38).contains(&zone) && lat >= 0.0 {
            format!("EPSG:{}", 25800 + zone)
        } else if lat >= 0.0 {
            format!("EPSG:{}", 32600 + zone)
        } else {
            format!("EPSG:{}", 32700 + zone)
        };
        return Some(Georeference {
            crs,
            eastings: e.round(),
            northings: n.round(),
            height: 0.0,
        });
    }
    None
}

/// UTM of a latitude and longitude in the georeference's zone.
fn utm_of(g: &Georeference, lat: f64, lon: f64) -> [f64; 2] {
    let (zone, north) = g.utm_zone().unwrap_or((30, true));
    let (e, n) = archisim_carto::lat_lon_to_utm(lat, lon, zone, north);
    [e, n]
}

fn middle(pts: &[[f64; 2]]) -> Option<[f64; 2]> {
    let first = pts.first()?;
    let (mut lo, mut hi) = (*first, *first);
    for p in pts {
        lo = [lo[0].min(p[0]), lo[1].min(p[1])];
        hi = [hi[0].max(p[0]), hi[1].max(p[1])];
    }
    Some([(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0])
}

/// The level at ground: the one nearest to ±0,00.
fn ground_level(m: &Model) -> LevelId {
    m.levels()
        .into_iter()
        .min_by(|a, b| a.elevation.abs().total_cmp(&b.elevation.abs()))
        .map(|l| l.id)
        .expect("a model has a level")
}

fn material(m: &mut Model, name: &str, category: &str, rgb: [u8; 3]) -> MaterialId {
    let found = m.materials().find(|x| x.category == category).map(|x| x.id);
    found.unwrap_or_else(|| m.add_material(name, category, rgb, archisim_core::CutPattern::Solid))
}

fn name_of(what: &str, reference: &str) -> String {
    if reference.trim().is_empty() {
        what.to_string()
    } else {
        format!("{what} {}", reference.trim())
    }
}

fn plan_ring(ring: &[[f64; 2]], frame: &Frame) -> Vec<Point2> {
    ring.iter().map(|p| frame.to_plan(*p)).collect()
}

/// Without the closing point and points repeated.
fn dedup_ring(r: &mut Vec<Point2>) {
    r.dedup_by(|a, b| a.distance(*b) < 0.01);
    while r.len() > 1 && r[0].distance(r[r.len() - 1]) < 0.01 {
        r.pop();
    }
}

fn centre_of(poly: &[Point2]) -> Point2 {
    let n = poly.len().max(1) as f64;
    let (x, y) = poly.iter().fold((0.0, 0.0), |a, p| (a.0 + p.x, a.1 + p.y));
    Point2::new(x / n, y / n)
}

fn in_polygon(p: Point2, poly: &[Point2]) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn length(line: &[Point2]) -> f64 {
    line.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// Points every `step` along a line, from its start.
fn along(line: &[Point2], step: f64) -> Vec<Point2> {
    let mut out = Vec::new();
    let mut next = 0.0;
    let mut done = 0.0;
    for w in line.windows(2) {
        let l = w[0].distance(w[1]);
        while next <= done + l && l > 0.0 {
            let t = (next - done) / l;
            out.push(Point2::new(
                w[0].x + (w[1].x - w[0].x) * t,
                w[0].y + (w[1].y - w[0].y) * t,
            ));
            next += step;
        }
        done += l;
    }
    out
}

/// The pieces of a polyline inside a circle.
fn clip_to_circle(line: &[Point2], c: Point2, r: f64) -> Vec<Vec<Point2>> {
    let mut out = Vec::new();
    let mut cur: Vec<Point2> = Vec::new();
    let inside = |p: Point2| p.distance(c) <= r;
    for w in line.windows(2) {
        let (a, b) = (w[0], w[1]);
        // Where the segment crosses the circle: a + t (b − a).
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let (fx, fy) = (a.x - c.x, a.y - c.y);
        let qa = dx * dx + dy * dy;
        let qb = 2.0 * (fx * dx + fy * dy);
        let qc = fx * fx + fy * fy - r * r;
        let disc = qb * qb - 4.0 * qa * qc;
        let mut ts: Vec<f64> = Vec::new();
        if qa > 0.0 && disc > 0.0 {
            let s = disc.sqrt();
            for t in [(-qb - s) / (2.0 * qa), (-qb + s) / (2.0 * qa)] {
                if t > 0.0 && t < 1.0 {
                    ts.push(t);
                }
            }
        }
        let at = |t: f64| Point2::new(a.x + dx * t, a.y + dy * t);
        if inside(a) && cur.is_empty() {
            cur.push(a);
        }
        for t in ts {
            let p = at(t);
            if cur.is_empty() {
                cur.push(p);
            } else {
                cur.push(p);
                out.push(std::mem::take(&mut cur));
            }
        }
        if inside(b) {
            cur.push(b);
        }
    }
    if cur.len() >= 2 {
        out.push(cur);
    }
    out.retain(|p| p.len() >= 2);
    for p in &mut out {
        p.dedup_by(|a, b| a.distance(*b) < 0.01);
    }
    out
}

/// Fewer points: those that turn the line more than `tol` (metres off
/// the straight line).
fn simplify(line: &[Point2], tol: f64) -> Vec<Point2> {
    if line.len() < 3 {
        return line.to_vec();
    }
    let mut keep = vec![false; line.len()];
    keep[0] = true;
    keep[line.len() - 1] = true;
    let mut stack = vec![(0, line.len() - 1)];
    while let Some((i, j)) = stack.pop() {
        let (a, b) = (line[i], line[j]);
        let len = a.distance(b).max(1e-9);
        let mut far = (0.0, i);
        for (k, p) in line.iter().enumerate().take(j).skip(i + 1) {
            let d = ((b.x - a.x) * (a.y - p.y) - (a.x - p.x) * (b.y - a.y)).abs() / len;
            if d > far.0 {
                far = (d, k);
            }
        }
        if far.0 > tol {
            keep[far.1] = true;
            stack.push((i, far.1));
            stack.push((far.1, j));
        }
    }
    line.iter()
        .zip(keep)
        .filter(|(_, k)| *k)
        .map(|(p, _)| *p)
        .collect()
}

/// The section of a street by its OpenStreetMap class; none for the ones
/// that are not streets of a town (paths, tracks, motorways' links…).
pub fn street_section(class: &str, one_way: bool) -> Option<RoadSection> {
    let preset = |name: &str| {
        archisim_core::road_presets()
            .into_iter()
            .find(|(n, _)| *n == name)
            .map(|(_, s)| s)
    };
    match class {
        "primary" | "secondary" | "trunk" => preset("Avenida con mediana arbolada"),
        "tertiary" | "residential" | "unclassified" => {
            if one_way {
                preset("Calle de un sentido con aparcamiento")
            } else {
                preset("Calle de doble sentido con aparcamiento")
            }
        }
        "living_street" | "pedestrian" => preset("Calle de plataforma única"),
        _ => None,
    }
}

/// The room for a street: twice the usual distance (the median along
/// it) from its axis to the nearest parcel or building that the axis is
/// not inside; none when there is nothing near.
fn room_across(axis: &[Point2], fronts: &[Vec<Point2>]) -> Option<f64> {
    let mut d: Vec<f64> = along(axis, 4.0)
        .into_iter()
        .filter(|p| !fronts.iter().any(|f| in_polygon(*p, f)))
        .filter_map(|p| {
            fronts
                .iter()
                .flat_map(|f| (0..f.len()).map(move |i| (f[i], f[(i + 1) % f.len()])))
                .map(|(a, b)| segment_distance(p, a, b))
                .min_by(f64::total_cmp)
        })
        .filter(|d| *d < 40.0)
        .collect();
    if d.len() < 2 {
        return None;
    }
    d.sort_by(f64::total_cmp);
    Some(2.0 * d[d.len() / 2])
}

fn segment_distance(p: Point2, a: Point2, b: Point2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(Point2::new(a.x + dx * t, a.y + dy * t))
}

/// A section that fits in `room` metres: the one given if it does; if
/// not, the first narrower street type that does (without the median,
/// then without parking on one side, then without parking); the last
/// with its strips made narrower in proportion.
pub fn fit_section(section: &RoadSection, room: f64) -> RoadSection {
    if section.width() <= room + 0.01 {
        return section.clone();
    }
    let presets = archisim_core::road_presets();
    let by = |name: &str| {
        presets
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, s)| s.clone())
    };
    let mut tries: Vec<RoadSection> = [
        "Calle de doble sentido con aparcamiento",
        "Calle de un sentido con aparcamiento",
    ]
    .into_iter()
    .filter_map(by)
    .collect();
    // Without any parking: the strips of a one-way street but its lanes.
    if let Some(mut s) = by("Calle de un sentido con aparcamiento") {
        s.strips
            .retain(|x| x.band != archisim_core::RoadBand::Parking);
        tries.push(s);
    }
    if section.single_level {
        tries.retain(|t| t.single_level);
    }
    let mut last = section.clone();
    for t in tries {
        let mut t = t;
        t.buildouts = section.buildouts.clone();
        t.crossings = section.crossings.clone();
        if t.width() <= room + 0.01 {
            return t;
        }
        last = t;
    }
    // Everything narrower, keeping its proportions (never under 3 m).
    let k = (room.max(3.0) / last.width()).min(1.0);
    for st in &mut last.strips {
        st.width *= k;
    }
    last
}

/// A species of the library for a tree of OpenStreetMap.
fn species_of(leaf: Option<&str>, genus: Option<&str>) -> &'static str {
    match genus.map(|g| g.to_ascii_lowercase()) {
        Some(g) if g == "citrus" => return "Naranjo",
        Some(g) if g == "platanus" => return "Plátano de sombra",
        Some(g) if g == "jacaranda" => return "Jacaranda",
        Some(g) if g == "olea" => return "Olivo",
        Some(g) if g == "pinus" => return "Pino piñonero",
        Some(g) if g == "phoenix" => return "Palmera datilera",
        Some(g) if g == "washingtonia" => return "Washingtonia",
        Some(g) if g == "tipuana" => return "Tipuana",
        Some(g) if g == "populus" => return "Chopo",
        Some(g) if g == "quercus" => return "Encina",
        _ => {}
    }
    match leaf {
        Some("needleleaved") => "Pino piñonero",
        _ => "Plátano de sombra",
    }
}

/// Grid points within the radius, thinned so there are no more than about
/// 4 000, in plan with their height above the georeference's ±0,00.
fn grid_points(grid: &asc::Grid, frame: &Frame, opts: &CartoOptions) -> Vec<SurveyPoint> {
    // The cells inside the circle's box.
    let cells_across = (2.0 * opts.radius / grid.cell).ceil().max(1.0);
    let step = ((cells_across * cells_across / 4000.0).sqrt().ceil() as usize).max(1);
    let mut out = Vec::new();
    for row in (0..grid.rows).step_by(step) {
        for col in (0..grid.cols).step_by(step) {
            let Some(z) = grid.at(col, row) else { continue };
            let p = frame.to_plan(grid.point(col, row));
            if p.distance(opts.centre) <= opts.radius {
                out.push(SurveyPoint {
                    at: p,
                    z: z - frame.h,
                });
            }
        }
    }
    out
}

fn nearest_z(points: &[SurveyPoint], p: Point2) -> Option<f64> {
    // The four nearest, weighted by the inverse of their distance squared.
    let mut near: Vec<(f64, f64)> = Vec::with_capacity(5);
    for s in points {
        let d = s.at.distance(p);
        if near.len() < 4 || d < near[near.len() - 1].0 {
            let at = near.partition_point(|x| x.0 < d);
            near.insert(at, (d, s.z));
            near.truncate(4);
        }
    }
    let first = near.first()?;
    if first.0 < 1e-6 {
        return Some(first.1);
    }
    let (w, z) = near.iter().fold((0.0, 0.0), |a, (d, z)| {
        (a.0 + 1.0 / (d * d), a.1 + z / (d * d))
    });
    Some(z / w)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<gml:FeatureCollection xmlns:gml="http://www.opengis.net/gml/3.2" xmlns:bu-ext2d="http://inspire.jrc.ec.europa.eu/schemas/bu-ext2d/2.0" xmlns:bu-core2d="http://inspire.jrc.ec.europa.eu/schemas/bu-core2d/2.0">
<gml:featureMember>
<bu-ext2d:Building gml:id="ES.SDGC.BU.0001">
<bu-core2d:numberOfFloorsAboveGround>3</bu-core2d:numberOfFloorsAboveGround>
<bu-ext2d:geometry><gml:Surface srsName="http://www.opengis.net/def/crs/EPSG/0/25829"><gml:patches><gml:PolygonPatch><gml:exterior><gml:LinearRing>
<gml:posList srsDimension="2">680030 4119000 680050 4119000 680050 4119015 680030 4119015 680030 4119000</gml:posList>
</gml:LinearRing></gml:exterior></gml:PolygonPatch></gml:patches></gml:Surface></bu-ext2d:geometry>
</bu-ext2d:Building>
</gml:featureMember>
<gml:featureMember>
<bu-ext2d:Building gml:id="ES.SDGC.BU.0002">
<bu-core2d:numberOfFloorsAboveGround>2</bu-core2d:numberOfFloorsAboveGround>
<bu-ext2d:geometry><gml:Surface srsName="http://www.opengis.net/def/crs/EPSG/0/25829"><gml:patches><gml:PolygonPatch><gml:exterior><gml:LinearRing>
<gml:posList srsDimension="2">679995 4118995 680005 4118995 680005 4119005 679995 4119005 679995 4118995</gml:posList>
</gml:LinearRing></gml:exterior></gml:PolygonPatch></gml:patches></gml:Surface></bu-ext2d:geometry>
</bu-ext2d:Building>
</gml:featureMember>
</gml:FeatureCollection>"#;

    fn project() -> Model {
        let mut m = crate::new_project("Prueba");
        m.set_georeference(Some(Georeference {
            crs: "EPSG:25829".into(),
            eastings: 680_000.0,
            northings: 4_119_000.0,
            height: 3.0,
        }))
        .unwrap();
        m
    }

    #[test]
    fn cadastre_buildings_become_masses_by_floors() {
        let mut m = project();
        let r = import_carto(
            &mut m,
            &[("a.gml".into(), GML.into())],
            &CartoOptions::default(),
        )
        .unwrap();
        assert_eq!(r.buildings, 2, "{r:?}");
        let tall = m
            .masses()
            .find(|x| (x.height - 9.0).abs() < 1e-9)
            .expect("three floors");
        let MassShape::Prism(p) = &tall.shape else {
            panic!()
        };
        // 30 m east of the origin, turned by the grid convergence (a
        // degree here).
        let c = centre_of(p);
        assert!((c.x - 40.0).abs() < 1.0 && (c.y - 7.5).abs() < 1.0, "{p:?}");
        assert!((signed_area(p).abs() - 300.0).abs() < 1e-6);
    }

    #[test]
    fn osm_streets_and_trees_and_a_grid() {
        let mut m = project();
        let (lat, lon) = m.georeference().unwrap().lat_lon().unwrap();
        let d = 0.0005;
        let osm = format!(
            r#"<?xml version="1.0"?><osm version="0.6">
<node id="1" lat="{a}" lon="{b}"/><node id="2" lat="{a}" lon="{c}"/>
<node id="3" lat="{e}" lon="{b}"><tag k="natural" v="tree"/><tag k="genus" v="Citrus"/></node>
<way id="10"><nd ref="1"/><nd ref="2"/><tag k="highway" v="residential"/><tag k="name" v="Calle Real"/></way>
</osm>"#,
            a = lat,
            b = lon - d,
            c = lon + d,
            e = lat + 0.0001
        );
        let asc = "ncols 3\nnrows 3\nxllcorner 679990\nyllcorner 4118990\ncellsize 10\nNODATA_value -9999\n5 5 5\n4 4 4\n3 3 3\n";
        let r = import_carto(
            &mut m,
            &[("a.osm".into(), osm), ("mdt.asc".into(), asc.into())],
            &CartoOptions::default(),
        )
        .unwrap();
        // On the slope of the grid, in level pieces.
        assert!(r.roads >= 1, "{r:?}");
        assert_eq!(r.trees, 1, "{r:?}");
        assert_eq!(r.terrain_points, 9, "{r:?}");
        assert!(m.roads().all(|x| x.name == "Calle Real"));
        let total: f64 = m.roads().map(|x| x.length()).sum();
        assert!(total > 60.0 && total < 100.0, "{total}");
        assert!(m.object_types().any(|t| t.name == "Naranjo"));
    }

    #[test]
    fn a_project_without_georeference_takes_the_cadastres() {
        let mut m = crate::new_project("Prueba");
        let r = import_carto(
            &mut m,
            &[("a.gml".into(), GML.into())],
            &CartoOptions::default(),
        )
        .unwrap();
        assert!(r.georeferenced);
        assert_eq!(m.georeference().unwrap().crs, "EPSG:25829");
        assert_eq!(r.buildings, 2);
    }

    #[test]
    fn openstreetmap_alone_places_the_project_in_its_utm_zone() {
        // Two streets crossing in Seville (zone 30, west of its meridian).
        let osm = r#"<osm version="0.6">
<node id="1" lat="37.3880" lon="-5.9960"/><node id="2" lat="37.3880" lon="-5.9940"/>
<node id="3" lat="37.3870" lon="-5.9950"/><node id="4" lat="37.3890" lon="-5.9950"/>
<way id="10"><nd ref="1"/><nd ref="2"/><tag k="highway" v="residential"/><tag k="oneway" v="yes"/></way>
<way id="11"><nd ref="3"/><nd ref="4"/><tag k="highway" v="pedestrian"/></way>
<way id="12"><nd ref="3"/><nd ref="4"/><tag k="highway" v="footway"/></way>
</osm>"#;
        let mut m = crate::new_project("Prueba");
        let r = import_carto(
            &mut m,
            &[("a.osm".into(), osm.into())],
            &CartoOptions::default(),
        )
        .unwrap();
        assert!(r.georeferenced);
        let g = m.georeference().unwrap();
        assert_eq!(g.crs, "EPSG:25830");
        let (lat, lon) = g.lat_lon().unwrap();
        assert!(
            (lat - 37.388).abs() < 0.001 && (lon + 5.995).abs() < 0.001,
            "{lat} {lon}"
        );
        // The footway is not a street.
        assert_eq!(r.roads, 2, "{r:?}");
        let single = m
            .roads()
            .find(|x| x.section.single_level)
            .expect("the pedestrian one");
        assert!(single.length() > 150.0, "{}", single.length());
    }

    #[test]
    fn avenues_narrow_to_the_room_between_parcels() {
        let avenue = street_section("secondary", false).unwrap();
        assert!(avenue.width() > 20.0);
        let s = fit_section(&avenue, 12.0);
        assert!(s.width() <= 12.01, "{}", s.width());
        let s = fit_section(&avenue, 8.0);
        assert!(s.width() <= 8.01, "{}", s.width());
        let axis = [Point2::new(0.0, 0.0), Point2::new(40.0, 0.0)];
        let fronts = vec![
            vec![
                Point2::new(-5.0, 6.0),
                Point2::new(45.0, 6.0),
                Point2::new(45.0, 30.0),
                Point2::new(-5.0, 30.0),
            ],
            vec![
                Point2::new(-5.0, -30.0),
                Point2::new(45.0, -30.0),
                Point2::new(45.0, -6.0),
                Point2::new(-5.0, -6.0),
            ],
        ];
        let room = room_across(&axis, &fronts).unwrap();
        assert!((room - 12.0).abs() < 1e-6, "{room}");
    }

    #[test]
    fn clipping_keeps_what_is_inside() {
        let line = [Point2::new(-200.0, 0.0), Point2::new(200.0, 0.0)];
        let parts = clip_to_circle(&line, Point2::new(0.0, 0.0), 100.0);
        assert_eq!(parts.len(), 1);
        assert!((length(&parts[0]) - 200.0).abs() < 1e-6);
    }
}
