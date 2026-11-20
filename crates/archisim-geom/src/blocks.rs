//! City blocks, their plots and filling them with buildings (N7,
//! `docs/TERRITORIO.md` §2.10), as Zoning Adjuster, Ploppable RICO and
//! Historical Districts do in Cities: Skylines: the land the streets
//! close round is a block; a block is cut into plots along its façade
//! line; the empty plots are filled with buildings of the place by zone,
//! and a locked plot keeps what it has.

use archisim_core::{
    grow_polygon, signed_area, BlockZone, LevelId, Model, ModelError, NewPlot, NewWorldBuilding,
    Point2, Vec2, WorldPick, WorldTypology,
};

use crate::junctions::{union, Shape};
use archisim_core::centroid;

/// The smallest block taken as one, m².
pub const MIN_BLOCK: f64 = 80.0;

/// The blocks of a level: the land the streets (with their crossings and
/// roundabouts) close round, counter-clockwise, at their façade lines.
pub fn city_blocks(model: &Model, level: LevelId) -> Vec<Vec<Point2>> {
    let ccw = |mut p: Vec<Point2>| {
        if signed_area(&p) < 0.0 {
            p.reverse();
        }
        p
    };
    let mut pieces: Vec<Shape> = model
        .roads_on_level(level)
        .filter(|r| !r.free)
        .map(|r| (ccw(r.outline()), Vec::new()))
        .filter(|(o, _)| o.len() >= 3)
        .collect();
    for c in model.roundabouts_on_level(level) {
        pieces.push((ccw(c.outline()), Vec::new()));
    }
    for j in crate::crossroads::junctions(model, level) {
        for p in std::iter::once(&j.plate)
            .chain(&j.sidewalks)
            .chain(&j.kerbs)
        {
            if p.len() >= 3 {
                pieces.push((ccw(p.clone()), Vec::new()));
            }
        }
    }
    if pieces.is_empty() {
        return Vec::new();
    }
    let streets = union(&pieces, &[]);
    let mut out: Vec<Vec<Point2>> = streets
        .into_iter()
        .flat_map(|(_, holes)| holes)
        .map(ccw)
        .filter(|h| signed_area(h) >= MIN_BLOCK)
        .collect();
    out.sort_by(|a, b| {
        let c = |p: &Vec<Point2>| centroid(p);
        let (ca, cb) = (c(a), c(b));
        // Row by row from the south, a metre's tolerance.
        if (ca.y - cb.y).abs() > 1.0 {
            ca.y.total_cmp(&cb.y)
        } else {
            ca.x.total_cmp(&cb.x)
        }
    });
    out
}

/// A plot cut from a block: its outline, its front on the façade line (two
/// ends, the block's edge, counter-clockwise) and its depth.
#[derive(Debug, Clone, PartialEq)]
pub struct Lot {
    pub outline: Vec<Point2>,
    pub front: (Point2, Point2),
    pub depth: f64,
}

/// A block cut into lots of about `frontage` along each of its sides and
/// `depth` deep (less in a narrow block: to its middle). The corners are
/// mitred, so the lots meet without overlapping; a block too small for a
/// courtyard is one lot.
pub fn block_lots(block: &[Point2], frontage: f64, depth: f64) -> Vec<Lot> {
    let n = block.len();
    if n < 3 {
        return Vec::new();
    }
    let area = signed_area(block).abs();
    // Deep enough to leave a courtyard; else the whole block, one lot.
    let mut d = depth.max(3.0);
    // Shallower than half the depth asked: not worth a courtyard.
    let least = (d * 0.5).max(3.0);
    let mut inner = Vec::new();
    for _ in 0..12 {
        let g = grow_polygon(block, -d);
        let ok = g.len() == n
            && signed_area(&g) > area * 0.02
            && (0..n).all(|i| {
                // The inner side keeps its direction (no corner flipped).
                let (a, b) = (block[i], block[(i + 1) % n]);
                let (c, e) = (g[i], g[(i + 1) % n]);
                (b - a).dot(e - c) > 0.0
            });
        if ok {
            inner = g;
            break;
        }
        d *= 0.8;
        if d < least {
            break;
        }
    }
    if inner.is_empty() {
        let (a, b) = (0..n)
            .map(|i| (block[i], block[(i + 1) % n]))
            .max_by(|x, y| x.0.distance(x.1).total_cmp(&y.0.distance(y.1)))
            .unwrap_or((block[0], block[1 % n]));
        return vec![Lot {
            outline: block.to_vec(),
            front: (a, b),
            depth: area / a.distance(b).max(1.0),
        }];
    }
    // Each side keeps the corner it starts at: its first plot fronts on
    // both streets (a corner plot) and its last stops where the next
    // side's corner plot begins, so the plots are square to their front.
    let mut out = Vec::new();
    for i in 0..n {
        let j = (i + 1) % n;
        let h = (i + n - 1) % n;
        let (a, b) = (block[i], block[j]);
        let (c, e) = (inner[i], inner[j]);
        let Some(u) = (b - a).normalized() else {
            continue;
        };
        // Where the inner side starts and ends, along the front.
        let (tc, te) = ((c - a).dot(u), (e - a).dot(u));
        if te - tc < 0.5 {
            continue;
        }
        // The point of the previous side across from the inner corner.
        let prev = {
            let (pa, pb) = (block[h], a);
            match (pb - pa).normalized() {
                Some(v) => pa + v * (inner[i] - pa).dot(v),
                None => a,
            }
        };
        // The corner plot: the corner and one frontage; the rest of the
        // side in plots of about the frontage.
        let f = frontage.max(3.0);
        let first = (tc + f).min(te);
        let rest = ((te - first) / f).round() as usize;
        let mut cuts = vec![0.0, first];
        if rest == 0 {
            *cuts.last_mut().unwrap() = te;
        } else {
            for k in 1..=rest {
                cuts.push(first + (te - first) * k as f64 / rest as f64);
            }
        }
        let count = cuts.len() - 1;
        let at = |t: f64| -> (Point2, Point2) {
            let q = if t <= tc {
                c
            } else {
                c + (e - c) * ((t - tc) / (te - tc))
            };
            (a + u * t, q)
        };
        for k in 0..count {
            let (t0, t1) = (cuts[k], cuts[k + 1]);
            let ((p0, q0), (p1, q1)) = (at(t0), at(t1));
            let outline = if k == 0 {
                let mut o = vec![prev, a, p1, q1, c];
                o.dedup_by(|x, y| x.distance(*y) < 1e-6);
                o
            } else {
                vec![p0, p1, q1, q0]
            };
            if signed_area(&outline).abs() < 4.0 {
                continue;
            }
            out.push(Lot {
                outline,
                front: (p0, p1),
                depth: d,
            });
        }
    }
    out
}

/// What a zone builds: its world typology and programme.
pub fn zone_pick(model: &Model, zone: BlockZone) -> WorldPick {
    let place = model.world_place();
    let (typology, programme) = match zone {
        BlockZone::OldTown => (WorldTypology::Street, 0),
        BlockZone::Flats => (WorldTypology::Block, 0),
        BlockZone::Houses => (WorldTypology::House, 0),
        BlockZone::Shops => (WorldTypology::Facility, shop_programme()),
        BlockZone::Civic => (WorldTypology::Facility, school_programme()),
    };
    WorldPick {
        place,
        typology,
        programme,
    }
}

fn programme_named(name: &str) -> u8 {
    archisim_core::world_programmes()
        .iter()
        .position(|p| *p == name)
        .unwrap_or(0) as u8
}

fn shop_programme() -> u8 {
    programme_named("Tienda")
}

fn school_programme() -> u8 {
    programme_named("Colegio")
}

/// The usual frontage of the plots of a zone.
pub fn zone_frontage(model: &Model, zone: BlockZone) -> f64 {
    let (w, _, _, _) = zone_pick(model, zone).usual();
    match zone {
        BlockZone::Civic => w.max(20.0),
        _ => w.clamp(5.0, 30.0),
    }
}

/// The usual depth of the plots of a zone.
pub fn zone_depth(model: &Model, zone: BlockZone) -> f64 {
    let (_, d, _, _) = zone_pick(model, zone).usual();
    match zone {
        BlockZone::Houses => (d + 10.0).clamp(12.0, 30.0),
        _ => (d + 4.0).clamp(10.0, 40.0),
    }
}

/// Cuts a block into plots of a zone (its usual frontage and depth, or
/// those given) and adds them to the model, named after the block. Their
/// ids.
pub fn add_block_plots(
    model: &mut Model,
    level: LevelId,
    block: &[Point2],
    number: usize,
    zone: BlockZone,
    frontage: Option<f64>,
    depth: Option<f64>,
) -> Result<Vec<archisim_core::PlotId>, ModelError> {
    let f = frontage.unwrap_or_else(|| zone_frontage(model, zone));
    let d = depth.unwrap_or_else(|| zone_depth(model, zone));
    let mut out = Vec::new();
    for (k, lot) in block_lots(block, f, d).into_iter().enumerate() {
        let id = model.add_plot(NewPlot {
            name: format!("Manzana {number} · {}", k + 1),
            level,
            outline: lot.outline,
            setback: 0.0,
            reference: String::new(),
        })?;
        model.edit_plot(id, |p| p.zone = Some(zone))?;
        out.push(id);
    }
    Ok(out)
}

/// Cuts a block that already has plots again (N7): its plots that are not
/// locked go, with their buildings, and the new lots that do not fall on a
/// locked plot take their place. The new plots' ids and how many went.
pub fn replot_block(
    model: &mut Model,
    level: LevelId,
    block: &[Point2],
    number: usize,
    zone: BlockZone,
    frontage: Option<f64>,
    depth: Option<f64>,
) -> Result<(Vec<archisim_core::PlotId>, usize), ModelError> {
    let inside: Vec<archisim_core::Plot> = model
        .plots_on_level(level)
        .filter(|p| archisim_core::point_in_polygon(centroid(&p.outline), block))
        .cloned()
        .collect();
    let (locked, loose): (Vec<_>, Vec<_>) = inside.into_iter().partition(|p| p.locked);
    let gone: Vec<archisim_core::PlotId> = loose.iter().map(|p| p.id).collect();
    clear_plots(model, level, Some(&gone))?;
    for id in &gone {
        model.remove_plot(*id)?;
    }
    let f = frontage.unwrap_or_else(|| zone_frontage(model, zone));
    let d = depth.unwrap_or_else(|| zone_depth(model, zone));
    // The names of the locked plots stay; the new ones go on after them.
    let mut k = locked.len();
    let mut out = Vec::new();
    for lot in block_lots(block, f, d) {
        let c = centroid(&lot.outline);
        let on_locked = locked.iter().any(|p| {
            archisim_core::point_in_polygon(c, &p.outline)
                || archisim_core::point_in_polygon(centroid(&p.outline), &lot.outline)
        });
        if on_locked {
            continue;
        }
        k += 1;
        let id = model.add_plot(NewPlot {
            name: format!("Manzana {number} · {k}"),
            level,
            outline: lot.outline,
            setback: 0.0,
            reference: String::new(),
        })?;
        model.edit_plot(id, |p| p.zone = Some(zone))?;
        out.push(id);
    }
    Ok((out, gone.len()))
}

/// The streets of a model as they were before a change, to see what the
/// change did to its blocks ([`follow_streets`]).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Streets {
    roads: Vec<archisim_core::Road>,
    rings: Vec<archisim_core::Roundabout>,
}

impl Streets {
    pub fn of(model: &Model) -> Self {
        Streets {
            roads: model.roads().cloned().collect(),
            rings: model.roundabouts().cloned().collect(),
        }
    }
}

/// Whether there is anything that follows the streets: plots cut from the
/// blocks (with a zone) or buildings of the surroundings.
pub fn streets_matter(model: &Model) -> bool {
    model.plots().any(|p| p.zone.is_some()) || model.masses().any(|m| m.dress.is_some())
}

/// What following the streets did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Followed {
    /// Blocks cut again, and the plots that are in them now.
    pub blocks: usize,
    pub plots: usize,
    /// Buildings put again on the new plots.
    pub built: usize,
    /// Buildings of the surroundings taken away because a street now runs
    /// where they were.
    pub on_streets: usize,
}

impl Followed {
    pub fn is_empty(&self) -> bool {
        self.blocks == 0 && self.on_streets == 0
    }

    /// What it did, said in a line.
    pub fn note(&self) -> Option<String> {
        let mut parts = Vec::new();
        if self.blocks > 0 {
            let mut t = format!(
                "{} {} vuelta{} a parcelar ({} parcela{}",
                self.blocks,
                if self.blocks == 1 {
                    "manzana"
                } else {
                    "manzanas"
                },
                if self.blocks == 1 { "" } else { "s" },
                self.plots,
                if self.plots == 1 { "" } else { "s" },
            );
            if self.built > 0 {
                t.push_str(&format!(
                    ", {} edificio{}",
                    self.built,
                    if self.built == 1 { "" } else { "s" }
                ));
            }
            t.push(')');
            parts.push(t);
        }
        if self.on_streets > 0 {
            parts.push(format!(
                "{} edificio{} quitado{} de donde ahora pasa la calle",
                self.on_streets,
                if self.on_streets == 1 { "" } else { "s" },
                if self.on_streets == 1 { "" } else { "s" },
            ));
        }
        (!parts.is_empty()).then(|| {
            let mut t = parts.join("; ");
            t.push('.');
            let mut c = t.chars();
            c.next().map_or(t.clone(), |f| {
                f.to_uppercase().collect::<String>() + c.as_str()
            })
        })
    }
}

/// The model with its streets as they were.
fn with_streets(model: &Model, before: &Streets) -> Result<Model, ModelError> {
    let mut old = model.clone();
    let now: Vec<archisim_core::RoadId> = old.roads().map(|r| r.id).collect();
    for id in now {
        match before.roads.iter().find(|r| r.id == id) {
            Some(b) => old.edit_road(id, |r| *r = b.clone())?,
            None => {
                old.remove_road(id)?;
            }
        }
    }
    for b in before.roads.iter().filter(|b| model.road(b.id).is_none()) {
        let id = old.add_road(archisim_core::NewRoad {
            name: b.name.clone(),
            level: b.level,
            axis: b.axis.clone(),
            section: b.section.clone(),
            top_offset: b.top_offset,
            profile: b.profile.clone(),
            changes: b.changes.clone(),
            pavement: b.pavement,
            kerb_material: b.kerb_material,
            sidewalk_material: b.sidewalk_material,
        })?;
        old.edit_road(id, |r| {
            let (id, guid) = (r.id, r.guid.clone());
            *r = b.clone();
            r.id = id;
            r.guid = guid;
        })?;
    }
    let now: Vec<archisim_core::RoundaboutId> = old.roundabouts().map(|r| r.id).collect();
    for id in now {
        match before.rings.iter().find(|r| r.id == id) {
            Some(b) => old.edit_roundabout(id, |r| *r = b.clone())?,
            None => {
                old.remove_roundabout(id)?;
            }
        }
    }
    for b in before
        .rings
        .iter()
        .filter(|b| model.roundabout(b.id).is_none())
    {
        let id = old.add_roundabout(archisim_core::NewRoundabout {
            name: b.name.clone(),
            level: b.level,
            centre: b.centre,
            radius: b.radius,
            island: b.island,
            apron: b.apron,
            top_offset: b.top_offset,
            plaza: b.plaza,
            island_finish: b.island_finish,
            pavement: b.pavement,
            kerb_material: b.kerb_material,
            island_material: b.island_material,
        })?;
        old.edit_roundabout(id, |r| {
            let (id, guid) = (r.id, r.guid.clone());
            *r = b.clone();
            r.id = id;
            r.guid = guid;
        })?;
    }
    Ok(old)
}

/// Whether two blocks are the same (the same corners, a centimetre's
/// tolerance).
fn same_block(a: &[Point2], b: &[Point2]) -> bool {
    a.len() == b.len() && a.iter().all(|p| b.iter().any(|q| p.distance(*q) < 0.01))
}

/// Whether two blocks overlap: a corner or the middle of one inside the
/// other.
fn overlap(a: &[Point2], b: &[Point2]) -> bool {
    let inside = |p: Point2, o: &[Point2]| archisim_core::point_in_polygon(p, o);
    inside(centroid(a), b)
        || inside(centroid(b), a)
        || a.iter().any(|p| inside(*p, b))
        || b.iter().any(|p| inside(*p, a))
}

/// After a change of the streets (`before`, how they were): the blocks
/// that changed shape are cut again into plots of the zone they had,
/// keeping the locked plots, and built again if they were built; and the
/// buildings of the surroundings a street now runs over go. Plots drawn by
/// hand (with no zone) are left as they are.
pub fn follow_streets(model: &mut Model, before: &Streets) -> Result<Followed, ModelError> {
    let now = Streets::of(model);
    if now == *before {
        return Ok(Followed::default());
    }
    let mut levels: Vec<LevelId> = Vec::new();
    {
        let mut note = |l: LevelId| {
            if !levels.contains(&l) {
                levels.push(l);
            }
        };
        for r in &now.roads {
            if before.roads.iter().find(|b| b.id == r.id) != Some(r) {
                note(r.level);
            }
        }
        for b in &before.roads {
            if now.roads.iter().find(|r| r.id == b.id) != Some(b) {
                note(b.level);
            }
        }
        for r in &now.rings {
            if before.rings.iter().find(|b| b.id == r.id) != Some(r) {
                note(r.level);
            }
        }
        for b in &before.rings {
            if now.rings.iter().find(|r| r.id == b.id) != Some(b) {
                note(b.level);
            }
        }
    }
    let old = with_streets(model, before)?;
    let mut done = Followed::default();
    for level in levels {
        let old_blocks = city_blocks(&old, level);
        let new_blocks = city_blocks(model, level);
        let gone: Vec<&Vec<Point2>> = old_blocks
            .iter()
            .filter(|o| !new_blocks.iter().any(|n| same_block(o, n)))
            .collect();
        let mut redo: Vec<(usize, BlockZone, bool)> = Vec::new();
        for ob in gone {
            let loose: Vec<archisim_core::Plot> = model
                .plots_on_level(level)
                .filter(|p| {
                    p.zone.is_some()
                        && !p.locked
                        && archisim_core::point_in_polygon(centroid(&p.outline), ob)
                })
                .cloned()
                .collect();
            if loose.is_empty() {
                continue;
            }
            // The zone most of them had.
            let mut zones: Vec<(BlockZone, usize)> = Vec::new();
            for p in &loose {
                let z = p.zone.unwrap_or(BlockZone::Houses);
                match zones.iter_mut().find(|(x, _)| *x == z) {
                    Some(e) => e.1 += 1,
                    None => zones.push((z, 1)),
                }
            }
            let zone = zones
                .iter()
                .max_by_key(|(_, n)| *n)
                .map(|(z, _)| *z)
                .unwrap();
            let built = loose.iter().any(|p| plot_built(model, p));
            let ids: Vec<archisim_core::PlotId> = loose.iter().map(|p| p.id).collect();
            clear_plots(model, level, Some(&ids))?;
            for id in ids {
                model.remove_plot(id)?;
            }
            for (k, nb) in new_blocks.iter().enumerate() {
                let new = !old_blocks.iter().any(|o| same_block(o, nb));
                if new && overlap(ob, nb) && !redo.iter().any(|(i, _, _)| *i == k) {
                    redo.push((k, zone, built));
                }
            }
        }
        for (k, zone, built) in redo {
            let (ids, _) = replot_block(model, level, &new_blocks[k], k + 1, zone, None, None)?;
            done.blocks += 1;
            done.plots += ids.len();
            if built {
                done.built += fill_plots(model, level, Some(&ids), None)?;
            }
        }
        // The buildings a street now runs over.
        let streets: Vec<Vec<Point2>> = now
            .roads
            .iter()
            .filter(|r| r.level == level && !r.free)
            .filter(|r| before.roads.iter().find(|b| b.id == r.id) != Some(*r))
            .map(|r| r.outline())
            .chain(
                now.rings
                    .iter()
                    .filter(|r| r.level == level)
                    .filter(|r| before.rings.iter().find(|b| b.id == r.id) != Some(*r))
                    .map(|r| r.outline()),
            )
            .collect();
        let under: Vec<archisim_core::MassId> = model
            .masses_on_level(level)
            .filter(|m| m.dress.is_some())
            .filter(|m| match &m.shape {
                archisim_core::MassShape::Prism(o) => {
                    let c = centroid(o);
                    streets
                        .iter()
                        .any(|s| archisim_core::point_in_polygon(c, s))
                }
                _ => false,
            })
            .map(|m| m.id)
            .collect();
        for id in under {
            model.remove_mass(id)?;
            done.on_streets += 1;
        }
    }
    Ok(done)
}

/// The front of a plot on its block's façade line: of its sides, the one
/// that has a street just outside it (else its longest).
/// The sides of a plot (by the index of their first corner) that have a
/// street just outside them.
pub fn street_sides(model: &Model, plot: &archisim_core::Plot) -> Vec<usize> {
    let o = &plot.outline;
    let n = o.len();
    if n < 3 {
        return Vec::new();
    }
    let streets: Vec<Vec<Point2>> = model
        .roads_on_level(plot.level)
        .map(|r| r.outline())
        .collect();
    (0..n)
        .filter(|&i| {
            let (a, b) = (o[i], o[(i + 1) % n]);
            let m = (a + b) * 0.5;
            let Some(u) = (b - a).normalized() else {
                return false;
            };
            a.distance(b) > 1.0
                && streets
                    .iter()
                    .any(|s| archisim_core::point_in_polygon(m + Vec2::new(u.y, -u.x) * 1.5, s))
        })
        .collect()
}

pub fn plot_front(model: &Model, plot: &archisim_core::Plot) -> Option<(Point2, Point2)> {
    let o = &plot.outline;
    let n = o.len();
    if n < 3 {
        return None;
    }
    let streets: Vec<Vec<Point2>> = model
        .roads_on_level(plot.level)
        .map(|r| r.outline())
        .collect();
    let on_street = |a: Point2, b: Point2| {
        let m = (a + b) * 0.5;
        let Some(u) = (b - a).normalized() else {
            return false;
        };
        // Outside the plot: to the right of a counter-clockwise side.
        let out = m + Vec2::new(u.y, -u.x) * 1.5;
        streets
            .iter()
            .any(|s| archisim_core::point_in_polygon(out, s))
    };
    let sides: Vec<(Point2, Point2)> = (0..n).map(|i| (o[i], o[(i + 1) % n])).collect();
    sides
        .iter()
        .copied()
        .filter(|(a, b)| on_street(*a, *b))
        .max_by(|x, y| x.0.distance(x.1).total_cmp(&y.0.distance(y.1)))
        .or_else(|| {
            sides
                .iter()
                .copied()
                .max_by(|x, y| x.0.distance(x.1).total_cmp(&y.0.distance(y.1)))
        })
}

/// Whether a building of the surroundings stands on a plot (its middle
/// inside it).
pub fn plot_built(model: &Model, plot: &archisim_core::Plot) -> bool {
    model.masses().any(|m| {
        m.level == plot.level
            && m.dress.is_some()
            && match &m.shape {
                archisim_core::MassShape::Prism(o) => {
                    archisim_core::point_in_polygon(centroid(o), &plot.outline)
                }
                _ => false,
            }
    })
}

/// Fills a plot with a building of its zone (or `zone`): its front on the
/// plot's front, as wide as the front and as deep as the plot lets it.
/// None if the plot is locked or already built.
pub fn fill_plot(
    model: &mut Model,
    id: archisim_core::PlotId,
    zone: Option<BlockZone>,
) -> Result<Option<archisim_core::MassId>, ModelError> {
    let Some(p) = model.plot(id).cloned() else {
        return Err(ModelError::UnknownElement(id.0));
    };
    if p.locked || plot_built(model, &p) {
        return Ok(None);
    }
    let Some(zone) = zone.or(p.zone) else {
        return Ok(None);
    };
    // A corner plot, away from the houses with their front garden: the
    // building takes the whole plot, with a façade to each street.
    let fronts = street_sides(model, &p);
    if fronts.len() >= 2 && zone != BlockZone::Houses {
        let outline = grow_polygon(&p.outline, -0.05);
        let n = outline.len();
        if n == p.outline.len() && signed_area(&outline) > 20.0 {
            let main = fronts
                .iter()
                .copied()
                .max_by(|x, y| {
                    let l = |i: usize| outline[i].distance(outline[(i + 1) % n]);
                    l(*x).total_cmp(&l(*y))
                })
                .unwrap_or(0);
            let (a, b) = (outline[main], outline[(main + 1) % n]);
            let facing = (b - a)
                .normalized()
                .map_or(Vec2::new(0.0, -1.0), |u| Vec2::new(u.y, -u.x));
            let id = model.add_world_building_on(
                NewWorldBuilding {
                    level: p.level,
                    front: (a + b) * 0.5,
                    facing,
                    width: a.distance(b).max(1.0),
                    depth: 1.0,
                    floors: None,
                    pick: zone_pick(model, zone),
                },
                outline,
                main,
            )?;
            return Ok(Some(id));
        }
    }
    let Some((a, b)) = plot_front(model, &p) else {
        return Ok(None);
    };
    let Some(u) = (b - a).normalized() else {
        return Ok(None);
    };
    // Into the plot: to the left of a counter-clockwise side.
    let inward = u.perp_left();
    let deep = p
        .outline
        .iter()
        .map(|q| (*q - a).dot(inward))
        .fold(0.0, f64::max);
    let pick = zone_pick(model, zone);
    let (_, usual_depth, _, _) = pick.usual();
    // Houses keep a front garden; the others go to the façade line.
    let set_in = if zone == BlockZone::Houses { 3.0 } else { 0.0 };
    let mut width = a.distance(b) * 0.96;
    let mut depth = (deep - set_in - 0.5)
        .min(usual_depth.max(8.0))
        .min(deep * 0.9);
    let front = (a + b) * 0.5 + inward * (set_in + 0.05);
    if !archisim_core::point_in_polygon(front + inward * 0.5, &p.outline) {
        return Ok(None);
    }
    // Inside the plot: the lots at the corners narrow inwards, so the
    // building gets shallower (and then narrower) until it fits.
    let fits = |w: f64, d: f64| {
        archisim_core::world_footprint(front, inward * -1.0, w * 0.999, d * 0.999)
            .iter()
            .all(|q| archisim_core::point_in_polygon(*q, &p.outline))
    };
    for k in 0..16 {
        if fits(width, depth) {
            break;
        }
        if k % 2 == 0 {
            width *= 0.9;
        } else {
            depth *= 0.85;
        }
    }
    if width < 3.0 || depth < 3.0 || !fits(width, depth) {
        return Ok(None);
    }
    let id = model.add_world_building(NewWorldBuilding {
        level: p.level,
        front,
        facing: inward * -1.0,
        width,
        depth,
        floors: None,
        pick,
    })?;
    Ok(Some(id))
}

/// Fills the empty plots that are not locked (those of `only`, or all of
/// the level). How many buildings it put.
pub fn fill_plots(
    model: &mut Model,
    level: LevelId,
    only: Option<&[archisim_core::PlotId]>,
    zone: Option<BlockZone>,
) -> Result<usize, ModelError> {
    let ids: Vec<archisim_core::PlotId> = match only {
        Some(o) => o.to_vec(),
        None => model.plots_on_level(level).map(|p| p.id).collect(),
    };
    let mut n = 0;
    for id in ids {
        if fill_plot(model, id, zone)?.is_some() {
            n += 1;
        }
    }
    Ok(n)
}

/// Takes away the buildings of the surroundings on the plots that are not
/// locked (those of `only`, or all of the level), to fill them again. How
/// many it took away.
pub fn clear_plots(
    model: &mut Model,
    level: LevelId,
    only: Option<&[archisim_core::PlotId]>,
) -> Result<usize, ModelError> {
    let plots: Vec<archisim_core::Plot> = match only {
        Some(o) => o.iter().filter_map(|id| model.plot(*id).cloned()).collect(),
        None => model.plots_on_level(level).cloned().collect(),
    };
    let gone: Vec<archisim_core::MassId> = model
        .masses()
        .filter(|m| m.level == level && m.dress.is_some())
        .filter(|m| match &m.shape {
            archisim_core::MassShape::Prism(o) => {
                let c = centroid(o);
                plots
                    .iter()
                    .any(|p| !p.locked && archisim_core::point_in_polygon(c, &p.outline))
            }
            _ => false,
        })
        .map(|m| m.id)
        .collect();
    for id in &gone {
        model.remove_mass(*id)?;
    }
    Ok(gone.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{NewRoad, RoadSection};

    /// A town of three streets each way, 120 m apart: four blocks.
    fn town() -> (Model, LevelId) {
        let mut m = Model::new("Manzanas");
        let level = m.add_level("Planta baja", 0.0);
        let mat = m.add_material(
            "Asfalto",
            "asphalt",
            [60, 60, 60],
            archisim_core::CutPattern::Solid,
        );
        let road = |m: &mut Model, a: Point2, b: Point2| {
            m.add_road(NewRoad {
                name: "Calle".into(),
                level,
                axis: vec![a, b],
                section: RoadSection::default(),
                top_offset: 0.0,
                profile: Vec::new(),
                changes: Vec::new(),
                pavement: mat,
                kerb_material: mat,
                sidewalk_material: mat,
            })
            .unwrap();
        };
        for k in 0..3 {
            let t = k as f64 * 120.0;
            road(&mut m, Point2::new(-20.0, t), Point2::new(260.0, t));
            road(&mut m, Point2::new(t, -20.0), Point2::new(t, 260.0));
        }
        (m, level)
    }

    #[test]
    fn streets_close_blocks_and_blocks_take_plots() {
        let (mut m, level) = town();
        let blocks = city_blocks(&m, level);
        assert_eq!(
            blocks.len(),
            4,
            "four blocks between three streets each way"
        );
        let w = RoadSection::default().width();
        for b in &blocks {
            // Each one the square between the façade lines.
            let side = 120.0 - w;
            assert!(
                (signed_area(b) - side * side).abs() < side * 2.0,
                "{} against {}",
                signed_area(b),
                side * side
            );
        }
        // Cut into houses between party walls: plots along every side,
        // meeting without overlapping, and a courtyard left.
        let lots = block_lots(&blocks[0], 8.0, 18.0);
        let sum: f64 = lots.iter().map(|l| signed_area(&l.outline)).sum();
        assert!(lots.len() >= 32, "{} lots", lots.len());
        assert!(sum < signed_area(&blocks[0]) * 0.8, "a courtyard is left");
        assert!(lots.iter().all(|l| signed_area(&l.outline) > 0.0));
        // Into the model, zoned.
        let ids =
            add_block_plots(&mut m, level, &blocks[0], 1, BlockZone::OldTown, None, None).unwrap();
        assert!(!ids.is_empty());
        assert!(ids
            .iter()
            .all(|id| m.plot(*id).unwrap().zone == Some(BlockZone::OldTown)));
        // Filled: a building of the place on each plot, its front on the
        // street.
        let n = fill_plots(&mut m, level, Some(&ids), None).unwrap();
        assert!(n * 10 >= ids.len() * 7, "{n} of {}", ids.len());
        let p = m.plot(ids[3]).unwrap().clone();
        assert!(plot_built(&m, &p));
        // Filling again puts nothing more.
        assert_eq!(fill_plots(&mut m, level, Some(&ids), None).unwrap(), 0);
        // A locked plot keeps its building when the rest are cleared.
        m.edit_plot(ids[3], |p| p.locked = true).unwrap();
        let gone = clear_plots(&mut m, level, Some(&ids)).unwrap();
        assert_eq!(gone + 1, n);
        assert!(plot_built(&m, &p));
        assert!(!plot_built(&m, m.plot(ids[4]).unwrap()));
        // Another zone: blocks of flats on the free plots.
        let again = fill_plots(&mut m, level, Some(&ids), Some(BlockZone::Flats)).unwrap();
        assert!(again > 0);
    }

    #[test]
    fn plots_and_buildings_follow_a_moved_street() {
        let (mut m, level) = town();
        let blocks = city_blocks(&m, level);
        let mut ids = Vec::new();
        for (k, b) in blocks.iter().take(2).enumerate() {
            ids.extend(
                add_block_plots(&mut m, level, b, k + 1, BlockZone::Houses, None, None).unwrap(),
            );
        }
        fill_plots(&mut m, level, None, None).unwrap();
        // A plot drawn by hand (no zone) and a locked one stay as they are.
        let locked = ids[2];
        m.edit_plot(locked, |p| p.locked = true).unwrap();
        let kept = m.plot(locked).unwrap().outline.clone();
        let before = Streets::of(&m);
        // The middle north–south street, 20 m east.
        let mid = m
            .roads()
            .find(|r| r.axis[0] == Point2::new(120.0, -20.0))
            .unwrap()
            .id;
        m.edit_road(mid, |r| {
            r.axis = vec![Point2::new(140.0, -20.0), Point2::new(140.0, 260.0)]
        })
        .unwrap();
        let done = follow_streets(&mut m, &before).unwrap();
        assert_eq!(done.blocks, 2, "{done:?}");
        assert!(done.plots > 0 && done.built > 0, "{done:?}");
        // No plot and no building on the street.
        let street = m.road(mid).unwrap().outline();
        for p in m.plots() {
            assert!(!archisim_core::point_in_polygon(
                centroid(&p.outline),
                &street
            ));
        }
        for x in m.masses() {
            if let archisim_core::MassShape::Prism(o) = &x.shape {
                assert!(!archisim_core::point_in_polygon(centroid(o), &street));
            }
        }
        assert_eq!(m.plot(locked).unwrap().outline, kept);
        // Every zoned plot is inside one of the new blocks.
        let now = city_blocks(&m, level);
        for p in m.plots().filter(|p| p.zone.is_some()) {
            assert!(now
                .iter()
                .any(|b| archisim_core::point_in_polygon(centroid(&p.outline), b)));
        }
        assert!(done.note().unwrap().starts_with("2 manzanas"));
        // Nothing changed: nothing to do.
        let again = Streets::of(&m);
        assert!(follow_streets(&mut m, &again).unwrap().is_empty());
    }

    #[test]
    fn a_street_laid_over_buildings_takes_them_away() {
        let (mut m, level) = town();
        let block = city_blocks(&m, level).remove(0);
        let ids = add_block_plots(&mut m, level, &block, 1, BlockZone::Houses, None, None).unwrap();
        fill_plots(&mut m, level, None, None).unwrap();
        // Plots drawn by hand (no zone): they are not cut again, but the
        // buildings under the new street go.
        for id in ids {
            m.edit_plot(id, |p| p.zone = None).unwrap();
        }
        let built = m.masses().count();
        let before = Streets::of(&m);
        let mat = m.materials().next().unwrap().id;
        let r = m
            .add_road(NewRoad {
                name: "Nueva".into(),
                level,
                axis: vec![Point2::new(60.0, -20.0), Point2::new(60.0, 140.0)],
                section: RoadSection::default(),
                top_offset: 0.0,
                profile: Vec::new(),
                changes: Vec::new(),
                pavement: mat,
                kerb_material: mat,
                sidewalk_material: mat,
            })
            .unwrap();
        let done = follow_streets(&mut m, &before).unwrap();
        assert!(done.on_streets > 0, "{done:?}");
        assert_eq!(done.blocks, 0, "{done:?}");
        assert_eq!(m.masses().count(), built - done.on_streets);
        assert!(done.note().unwrap().contains("quitado"));
        let street = m.road(r).unwrap().outline();
        for x in m.masses() {
            if let archisim_core::MassShape::Prism(o) = &x.shape {
                assert!(!archisim_core::point_in_polygon(centroid(o), &street));
            }
        }
    }

    #[test]
    fn corner_plots_get_a_building_with_two_facades() {
        let (mut m, level) = town();
        let block = city_blocks(&m, level).remove(0);
        let ids = add_block_plots(&mut m, level, &block, 1, BlockZone::Flats, None, None).unwrap();
        let corners: Vec<_> = ids
            .iter()
            .filter(|id| street_sides(&m, m.plot(**id).unwrap()).len() >= 2)
            .copied()
            .collect();
        assert_eq!(corners.len(), 4, "a corner plot at each corner");
        fill_plots(&mut m, level, Some(&corners), None).unwrap();
        let built: Vec<_> = m.masses().map(|x| x.id).collect();
        assert_eq!(built.len(), 4);
        for id in built {
            let fronts = m.street_sides(id).iter().filter(|s| **s).count();
            assert!(fronts >= 2, "a façade to each street ({fronts})");
            // It fills its plot.
            let x = m.mass(id).unwrap();
            let archisim_core::MassShape::Prism(o) = &x.shape else {
                panic!()
            };
            let plot = corners
                .iter()
                .map(|p| m.plot(*p).unwrap())
                .find(|p| archisim_core::point_in_polygon(centroid(o), &p.outline))
                .unwrap();
            assert!(signed_area(o) > signed_area(&plot.outline) * 0.97);
        }
    }

    #[test]
    fn a_block_cut_again_keeps_its_locked_plots() {
        let (mut m, level) = town();
        let block = city_blocks(&m, level).remove(0);
        let ids = add_block_plots(&mut m, level, &block, 1, BlockZone::Flats, None, None).unwrap();
        let first = ids.len();
        let keep = ids[1];
        fill_plots(&mut m, level, None, None).unwrap();
        m.edit_plot(keep, |p| p.locked = true).unwrap();
        let kept = m.plot(keep).unwrap().outline.clone();
        assert!(plot_built(&m, m.plot(keep).unwrap()));
        let buildings = m.masses().count();
        // Cut again with wider plots: fewer, the locked one as it was, with
        // its building, and none over it.
        let (new, gone) = replot_block(
            &mut m,
            level,
            &block,
            1,
            BlockZone::Houses,
            Some(30.0),
            None,
        )
        .unwrap();
        assert_eq!(gone, first - 1);
        assert!(new.len() < first - 1, "{} plots", new.len());
        assert_eq!(m.plot(keep).unwrap().outline, kept);
        assert_eq!(m.masses().count(), 1, "of {buildings}");
        for id in &new {
            let p = m.plot(*id).unwrap();
            assert!(!archisim_core::point_in_polygon(
                centroid(&p.outline),
                &kept
            ));
            assert_eq!(p.zone, Some(BlockZone::Houses));
        }
    }

    #[test]
    fn a_small_block_is_one_plot() {
        let square = vec![
            Point2::new(0.0, 0.0),
            Point2::new(14.0, 0.0),
            Point2::new(14.0, 12.0),
            Point2::new(0.0, 12.0),
        ];
        let lots = block_lots(&square, 8.0, 18.0);
        assert_eq!(lots.len(), 1);
        assert_eq!(lots[0].outline, square);
    }
}
