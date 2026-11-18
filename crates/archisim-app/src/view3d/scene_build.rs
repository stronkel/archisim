//! The scene made a piece at a time and kept (N0, third round,
//! `docs/TERRITORIO.md` §4.3).
//!
//! Each element (a *unit*; also a few pieces made for the whole model at
//! once: the claddings, the fittings, the orthophoto…) is made into its
//! pieces of triangles, and the pieces are put in *cells*: squares of
//! [`CELL`] metres by pass and layer, each made into chunks for the
//! graphics card. Furniture, doors and windows with a detailed model and
//! the buildings of the surroundings are *placed*: a shared model and
//! where it stands.
//!
//! What is kept from the last scene ([`SceneBuilder`]) lets a change make
//! again only the elements it touched and those that depend on them
//! within their reach (a street the junctions, markings, ground and
//! buildings within 30 m; a wall what it joins; a bench the furniture
//! next to it; a tree nothing), and only the cells they are in: the rest
//! of each cell is copied from its chunks as it was, and every other
//! chunk stays the same (and so stays on the graphics card).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use archisim_core::{MassId, Model, ObjectId, WallId};
use archisim_geom::solid::{Only, Solid, SolidOwner, Unit};
use glam::Vec3;

use super::{
    add_solids, cutaway, element, keep_model, kept_model, levels_of, model_key, piece_key,
    srgb_to_linear, Anchor, Chunk, Disciplines, Group, Instance, LinkedPart, Look, Mesh,
    MeshBuilder, Owner, Part, Pass, PlantModel, Scene, Shared, Sink, Vertex, CHUNK, DETAIL_AROUND,
    DETAIL_BUILDINGS, NEAR_PLANTS, NEAR_PLANTS_AROUND,
};
use crate::pick::Element;

/// The side of a cell, metres.
const CELL: f32 = 64.0;
/// Units with at least these triangles in a pass go in cells of their own.
const OWN_CELL: usize = 4096;
/// The layers of those cells (plus the unit's kind).
const OWN_LAYER: u8 = 100;
/// The side of a square of the index of where each unit is, metres.
const GRID: f32 = 32.0;
/// How far a change of a street, the ground or a pool reaches (the bank
/// of a street, the buildings that face it: `STREET_REACH`).
const FAR_REACH: f32 = 30.0;
/// How far a change of furniture reaches (worktops, sofas in L).
const OBJECT_REACH: f32 = 3.0;
/// How far a change of a building element reaches (joins).
const NEAR_REACH: f32 = 2.0;

/// What a part of the scene belongs to: an element (kind [`ELEMENT`]) or
/// a piece of the whole model.
pub(super) type UnitKey = (u8, u64);
const ELEMENT: u8 = 0;
const CLADDING: u8 = 1;
const FITTINGS: u8 = 2;
const FRAMES: u8 = 3;
const ORTHOPHOTO: u8 = 4;
const GROWTH: u8 = 5;
/// A tile of a terrain's ground, by its address (kept alive with it): the
/// same while the tile is the same.
const TILE: u8 = 6;
const PAINT: u8 = 10;
const FACE_ZONES: u8 = 11;
const LINKED: u8 = 12;

fn unit_key(unit: Unit, owner: u64) -> UnitKey {
    match unit {
        Unit::Element => (ELEMENT, owner),
        Unit::Cladding => (CLADDING, 0),
        Unit::Fittings => (FITTINGS, 0),
        Unit::Frames => (FRAMES, 0),
        Unit::Orthophoto => (ORTHOPHOTO, 0),
        Unit::Growth => (GROWTH, owner),
    }
}

/// A cell: pass, layer and square.
pub(super) type CellKey = (u8, u8, i32, i32);

/// What kind of element a unit is, for what it depends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    /// A plant grown by ArchiSim: it depends on nothing else.
    Plant,
    Object,
    /// An object that is a pool (it takes room out of the ground).
    PoolObject,
    /// A building of the surroundings.
    Dressed,
    /// A volume of the concept phase.
    Mass,
    Terrain,
    /// A street or a roundabout.
    Road,
    Pool,
    Marking,
    /// An urban surface (N5): on the ground or the street.
    Surface,
    /// Walls, slabs, roofs, doors, stairs, pipes…
    General,
    /// Not drawn (rooms, plots…).
    Other,
    /// A piece of the whole model.
    Whole,
}

impl Class {
    fn of(model: &Model, id: u64) -> Class {
        use archisim_core::*;
        if model.road(RoadId(id)).is_some() || model.roundabout(RoundaboutId(id)).is_some() {
            Class::Road
        } else if model.terrain(TerrainId(id)).is_some() {
            Class::Terrain
        } else if model.pool(PoolId(id)).is_some() {
            Class::Pool
        } else if let Some(o) = model.object(ObjectId(id)) {
            if model.type_grows(o.object_type) {
                Class::Plant
            } else if model.type_prop_is(o.object_type, "fine_pool") {
                Class::PoolObject
            } else {
                Class::Object
            }
        } else if let Some(m) = model.mass(MassId(id)) {
            if m.dress.is_some() {
                Class::Dressed
            } else {
                Class::Mass
            }
        } else if model.marking(MarkingId(id)).is_some() {
            Class::Marking
        } else if model.surface(SurfaceId(id)).is_some() {
            Class::Surface
        } else if model.room(RoomId(id)).is_some()
            || model.plot(PlotId(id)).is_some()
            || model.planting(PlantingId(id)).is_some()
        {
            Class::Other
        } else {
            Class::General
        }
    }

    /// How far a change of this kind of element reaches.
    fn reach(self) -> f32 {
        match self {
            Class::Road | Class::Terrain | Class::Pool | Class::PoolObject => FAR_REACH,
            Class::Object => OBJECT_REACH,
            _ => NEAR_REACH,
        }
    }

    /// Whether this kind of element is made again when an element of
    /// kind `changed` changes within its reach.
    fn listens(self, changed: Class) -> bool {
        match self {
            Class::Object | Class::PoolObject => {
                matches!(changed, Class::Object | Class::PoolObject)
            }
            // Its party walls and the streets it faces.
            Class::Dressed => matches!(changed, Class::Dressed | Class::Road),
            Class::Road | Class::Marking => matches!(changed, Class::Road),
            Class::Surface => matches!(changed, Class::Road | Class::Terrain),
            Class::General => matches!(changed, Class::General),
            _ => false,
        }
    }

    /// Whether a change of it changes the ground (and what lies on it:
    /// the orthophoto, the grass).
    fn shapes_ground(self) -> bool {
        matches!(
            self,
            Class::Road | Class::Terrain | Class::Pool | Class::PoolObject
        )
    }

    /// The layer of the cells its pieces go in: what changes together.
    fn layer(self) -> u8 {
        match self {
            Class::Plant | Class::Object | Class::PoolObject => 0,
            Class::Terrain => 1,
            Class::Road | Class::Marking | Class::Surface => 2,
            _ => 3,
        }
    }
}

/// Some triangles of one element made for one pass, in their place.
pub(super) struct Piece {
    pass: Pass,
    mesh: Mesh,
    owner: Owner,
    lo: Vec3,
    hi: Vec3,
}

impl Piece {
    fn new(pass: Pass, mesh: Mesh, owner: Owner) -> Option<Piece> {
        if mesh.is_empty() {
            return None;
        }
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        for v in &mesh.vertices {
            lo = lo.min(Vec3::from_array(v.pos));
            hi = hi.max(Vec3::from_array(v.pos));
        }
        Some(Piece {
            pass,
            mesh,
            owner,
            lo,
            hi,
        })
    }
}

/// A model shared by objects (or a building's own), ready to draw.
pub(super) struct GroupModel {
    /// Its number, for the graphics card to keep it.
    pub id: u64,
    pub opaque: Arc<Mesh>,
    pub glass: Arc<Mesh>,
    pub levels: Vec<(u32, u32)>,
    pub lo: Vec3,
    pub hi: Vec3,
    pub exact_shadow: bool,
    /// The middle of what glows and its reach.
    glow: Option<(Vec3, f32, f32)>,
    /// What it was made from, kept alive while it is used (its key is
    /// the address of it).
    _from: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

static NEXT_KEY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// A new number for a chunk or a model, never given before.
pub(super) fn next_key() -> u64 {
    NEXT_KEY.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

impl GroupModel {
    /// The model of a group's triangles made in its own axes, with its
    /// coarser versions for farther away (or its own plain one).
    fn finish(mut shared: Shared, exact_shadow: bool) -> GroupModel {
        let glow = shared.glow;
        let simple = shared.simple.take();
        let shared = shared.done();
        let mut opaque = shared.opaque.mesh;
        let glass = shared.glass.mesh;
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        for v in opaque.vertices.iter().chain(&glass.vertices) {
            lo = lo.min(Vec3::from_array(v.pos));
            hi = hi.max(Vec3::from_array(v.pos));
        }
        let size = (hi - lo).length().max(0.05);
        let levels = match simple {
            Some(plain) if !plain.is_empty() => {
                // Its plain version after the whole of it.
                let full = opaque.indices.len() as u32;
                let base = opaque.vertices.len() as u32;
                opaque.vertices.extend(plain.vertices);
                opaque
                    .indices
                    .extend(plain.indices.iter().map(|i| i + base));
                vec![(0, full), (full, plain.indices.len() as u32)]
            }
            _ => levels_of(&mut opaque, size),
        };
        GroupModel {
            id: next_key(),
            opaque: Arc::new(opaque),
            glass: Arc::new(glass),
            levels,
            lo,
            hi,
            exact_shadow,
            glow,
            _from: None,
        }
    }
}

/// An object (or a building) drawn from a shared model.
#[derive(Clone)]
pub(super) struct Placed {
    model: Arc<GroupModel>,
    instance: Instance,
    owner: Element,
    bounds: (Vec3, Vec3),
}

/// What a unit made in one scene.
#[derive(Default)]
struct Made {
    pieces: Vec<Piece>,
    placed: Vec<Placed>,
    /// What glows, by element: middle (sum), corners and reach.
    lit: BTreeMap<u64, (Vec3, f32, f32)>,
    lamps: Vec<[f32; 4]>,
    /// For a building of the surroundings: what it was dressed against.
    context: u64,
    /// The element it is drawn for, if not its own (a tile: its terrain).
    owner: Option<u64>,
    /// What it was made from, kept alive while it is in the scene.
    keep: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

/// Where the triangles of one solid go, by pass.
#[derive(Default)]
struct PieceSink {
    passes: [MeshBuilder; 3],
    owner: Option<Owner>,
    lit: BTreeMap<u64, (Vec3, f32, f32)>,
}

impl Sink for PieceSink {
    fn push(&mut self, pass: Pass, tri: [Vertex; 3], owner: Owner) {
        self.owner = Some(owner);
        self.passes[pass as usize].push(tri);
    }

    fn glow(&mut self, id: u64, p: Vec3, reach: f32) {
        let (sum, n, r) = self.lit.entry(id).or_insert((Vec3::ZERO, 0.0, reach));
        *r = reach;
        *sum += p;
        *n += 1.0;
    }
}

impl Made {
    /// A solid made into its pieces, one per pass.
    fn add(&mut self, model: &Model, solid: Solid, look: &Look) {
        let mut sink = PieceSink::default();
        add_solids(model, vec![solid], look, &mut sink);
        self.take(sink);
    }

    fn take(&mut self, sink: PieceSink) {
        let Some(owner) = sink.owner else {
            return;
        };
        for (k, b) in sink.passes.into_iter().enumerate() {
            let pass = [Pass::Opaque, Pass::Glass, Pass::Grass][k];
            if let Some(p) = Piece::new(pass, b.mesh, owner) {
                self.pieces.push(p);
            }
        }
        for (id, (sum, n, reach)) in sink.lit {
            let e = self.lit.entry(id).or_insert((Vec3::ZERO, 0.0, reach));
            e.0 += sum;
            e.1 += n;
            e.2 = reach;
        }
    }

    /// Its lamps: its own and those of what glows in its pieces.
    fn finish_lamps(&mut self) {
        for (sum, n, reach) in std::mem::take(&mut self.lit).into_values() {
            if n > 0.0 {
                let c = sum / n;
                self.lamps.push([c.x, c.y, c.z, reach]);
            }
        }
    }
}

/// What changes everything in the scene when it changes.
#[derive(Debug, Clone, PartialEq)]
struct Setting {
    disciplines: Disciplines,
    frames: bool,
    plant_lod: archisim_core::flora::Lod,
    plant_detail: archisim_core::flora::Lod,
    crowd: bool,
}

impl Setting {
    fn of(model: &Model, disciplines: &Disciplines) -> Setting {
        Setting {
            disciplines: disciplines.clone(),
            frames: archisim_geom::framing::frames_shown(),
            plant_lod: archisim_geom::solid::plant_lod(model),
            plant_detail: model.plant_detail(),
            crowd: archisim_geom::env_buildings::light_dress(model),
        }
    }
}

/// What a unit put in the last scene.
struct Kept {
    class: Class,
    /// The boxes of its pieces and of what it placed.
    boxes: Vec<(Vec3, Vec3)>,
    cells: Vec<CellKey>,
    placed: Vec<Placed>,
    lamps: Vec<[f32; 4]>,
    context: u64,
    /// The element its pieces are drawn for.
    owner: u64,
    _keep: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

/// The scene kept from one model to the next.
struct State {
    model: Arc<Model>,
    setting: Setting,
    units: BTreeMap<UnitKey, Kept>,
    /// Which units are in each square of [`GRID`] metres.
    grid: HashMap<(i32, i32), Vec<UnitKey>>,
    cells: BTreeMap<CellKey, Vec<Chunk>>,
    /// Where each wall is, for the cut away walls.
    anchors: HashMap<WallId, Anchor>,
    /// The shared models of furniture, doors and windows, by their mesh
    /// and materials.
    models: HashMap<(usize, Vec<u64>), Arc<GroupModel>>,
    near_plants: BTreeSet<ObjectId>,
    detailed_masses: HashSet<MassId>,
    selection: BTreeSet<Element>,
}

/// Makes the 3D scene, keeping what it made to make again only what
/// changes.
#[derive(Default)]
pub struct SceneBuilder {
    state: Option<State>,
}

/// How many units were made for the last scene, and how many cells.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BuildStats {
    pub units: usize,
    pub cells: usize,
    /// Whether everything was made (not only what changed).
    pub whole: bool,
}

/// The plants nearest the project's origin that keep every leaf.
fn near_plants(model: &Model, disciplines: &Disciplines) -> BTreeSet<ObjectId> {
    use archisim_core::flora::Lod;
    if model.plant_detail() == Lod::Full {
        return BTreeSet::new();
    }
    let mut near: Vec<(f64, ObjectId)> = model
        .objects()
        .filter(|o| o.position.length() < NEAR_PLANTS_AROUND)
        .filter(|o| model.type_grows(o.object_type))
        .filter(|o| !model.hidden_by_discipline(&disciplines.hidden, o.id.0))
        .map(|o| (o.position.length(), o.id))
        .collect();
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    near.truncate(NEAR_PLANTS);
    near.into_iter().map(|(_, id)| id).collect()
}

/// The buildings of the surroundings with every detail: with a whole
/// neighbourhood, the nearest to the project's origin, no farther than
/// `DETAIL_AROUND` and no more than `DETAIL_BUILDINGS` of them.
fn detailed_masses(model: &Model, crowd: bool) -> HashSet<MassId> {
    if !crowd {
        return HashSet::new();
    }
    let centre = |m: &archisim_core::Mass| match &m.shape {
        archisim_core::MassShape::Prism(p) if !p.is_empty() => Some(
            p.iter()
                .fold(archisim_core::Point2::new(0.0, 0.0), |a, q| a + *q)
                * (1.0 / p.len() as f64),
        ),
        _ => None,
    };
    let mut by: Vec<(f64, MassId)> = model
        .masses()
        .filter(|m| m.dress.is_some())
        .filter_map(|m| Some((centre(m)?.length(), m.id)))
        .filter(|(d, _)| *d < DETAIL_AROUND)
        .collect();
    by.sort_by(|a, b| a.0.total_cmp(&b.0));
    by.into_iter()
        .take(DETAIL_BUILDINGS)
        .map(|(_, id)| id)
        .collect()
}

/// What a building of the surroundings is dressed against: its detail,
/// its party walls and the streets it faces.
fn dressed_context(model: &Model, id: MassId, near: bool) -> u64 {
    let mut h = super::FastHash::new(u64::from(near));
    for b in model.party_sides(id) {
        h.add(u64::from(b));
    }
    h.add(99);
    for b in model.street_sides(id) {
        h.add(u64::from(b));
    }
    h.finish()
}

/// Where each wall is, from its solids and its accessories'.
fn wall_anchor(idx: &archisim_geom::WallIndex, id: WallId) -> Option<Anchor> {
    let mut walls = archisim_geom::solid::wall_solids_in(idx, id);
    for p in archisim_geom::sweeps::wall_sweep_pieces_in(idx, id) {
        walls.push(Solid {
            owner: SolidOwner::Wall(id),
            material: p.material,
            triangles: archisim_geom::solid::mesh_triangles(&p.mesh),
        });
    }
    super::anchors(&walls).remove(&id)
}

/// The ground's tiles made for the 3D, each kept while it is the same
/// tile (the same `Arc`) and shown the same way: its terrain, the tile,
/// its pieces and how it is shown.
type KeptTile = (u64, Arc<Vec<[[f64; 3]; 3]>>, Arc<[Mesh; 3]>, u64);
static KEPT_TILES: std::sync::Mutex<Option<HashMap<usize, KeptTile>>> = std::sync::Mutex::new(None);

/// What everything of one making shares.
struct Making<'a> {
    model: &'a Model,
    disciplines: &'a Disciplines,
    /// Selected (only to see an isolated clash: otherwise the selection is
    /// marked afterwards).
    selection: &'a BTreeSet<Element>,
    isolate: bool,
    anchors: &'a mut HashMap<WallId, Anchor>,
    models: &'a mut HashMap<(usize, Vec<u64>), Arc<GroupModel>>,
    near_plants: &'a BTreeSet<ObjectId>,
    detailed_masses: &'a HashSet<MassId>,
    crowd: bool,
    /// The units already in the scene (the tiles of the ground that are
    /// the same are not made again).
    existing: &'a HashSet<UnitKey>,
    /// Those of them found still the same.
    still: Vec<UnitKey>,
}

impl Making<'_> {
    fn look<'b>(
        &'b self,
        anchors: &'b HashMap<WallId, Anchor>,
        shared: bool,
        between: bool,
    ) -> Look<'b> {
        Look {
            selection: self.selection,
            isolate: self.isolate,
            disciplines: self.disciplines,
            anchors,
            shared,
            between,
        }
    }

    fn flags(&self, e: Element, id: u64) -> [f32; 4] {
        [
            if self.isolate && self.selection.contains(&e) {
                1.0
            } else {
                0.0
            },
            if self.model.dimmed(self.disciplines.focus, id) {
                1.0
            } else {
                0.0
            },
            0.0,
            0.0,
        ]
    }

    /// The units of the elements `want` (all with `None`) and of the
    /// pieces of the whole model `whole`.
    fn make(
        &mut self,
        want: Option<&HashSet<u64>>,
        whole: &BTreeSet<u8>,
        linked: &[LinkedPart],
    ) -> BTreeMap<UnitKey, Made> {
        let model = self.model;
        let disciplines = self.disciplines;
        let wants = |id: u64| want.is_none_or(|w| w.contains(&id));
        let hidden = |id: u64| model.hidden_by_discipline(&disciplines.hidden, id);
        let mut out: BTreeMap<UnitKey, Made> = BTreeMap::new();
        let mut skip: HashSet<ObjectId> = HashSet::new();
        let mut detailed: HashSet<archisim_core::OpeningId> = HashSet::new();
        // Where the walls are, for the cut away walls: before anything is
        // made (their doors and windows go with them).
        {
            let _span = archisim_geom::meter::span("escena: muros para el corte");
            let idx = archisim_geom::WallIndex::new(model);
            match want {
                None => {
                    self.anchors.clear();
                    for w in model.walls() {
                        if let Some(a) = wall_anchor(&idx, w.id) {
                            self.anchors.insert(w.id, a);
                        }
                    }
                }
                Some(ids) => {
                    for id in ids {
                        let w = WallId(*id);
                        self.anchors.remove(&w);
                        if model.wall(w).is_some() {
                            if let Some(a) = wall_anchor(&idx, w) {
                                self.anchors.insert(w, a);
                            }
                        }
                    }
                }
            }
        }
        let zone_ids: BTreeSet<u64> = model
            .face_zones()
            .filter(|z| z.material.is_some())
            .map(|z| z.element)
            .collect();
        if !self.isolate {
            self.make_placed(&mut out, &mut skip, &mut detailed, want, &zone_ids);
        }
        let anchors = std::mem::take(self.anchors);
        let look = self.look(&anchors, false, false);
        // The ground, by tiles.
        let mut still = Vec::new();
        {
            let _span = archisim_geom::meter::span("escena: terreno");
            let mut kept = KEPT_TILES.lock().unwrap_or_else(|e| e.into_inner());
            let tiles = kept.get_or_insert_with(HashMap::new);
            // How the ground is shown (greyed, hidden) goes with its tiles.
            let shown = {
                let mut h = super::FastHash::new(0x7e44);
                h.add_str(&format!("{:?}", self.disciplines));
                h.finish()
            };
            for t in model.terrains().filter(|t| wants(t.id.0)) {
                let parts = if hidden(t.id.0) {
                    None
                } else {
                    archisim_geom::solid::terrain_parts(model, t.id)
                };
                let used: HashSet<usize> = parts
                    .iter()
                    .flat_map(|p| p.tiles.iter().map(|t| Arc::as_ptr(t) as usize))
                    .collect();
                // The tiles of this terrain no longer in it: let go.
                tiles.retain(|k, e| e.0 != t.id.0 || used.contains(k));
                let Some(parts) = parts else {
                    continue;
                };
                let owner = Owner::Model(Element::Terrain(t.id));
                // The tiles not in the scene yet, made on every core.
                let mut todo: Vec<&Arc<Vec<[[f64; 3]; 3]>>> = Vec::new();
                for tile in &parts.tiles {
                    let key = Arc::as_ptr(tile) as usize;
                    let unit = (TILE, key as u64);
                    if self.existing.contains(&unit) {
                        still.push(unit);
                        continue;
                    }
                    match tiles.get(&key) {
                        Some(e) if e.3 == shown && Arc::ptr_eq(&e.1, tile) => {
                            let meshes = e.2.clone();
                            let made = out.entry(unit).or_default();
                            made.owner = Some(t.id.0);
                            made.keep = Some(tile.clone());
                            push_meshes(made, &meshes, owner);
                        }
                        _ => todo.push(tile),
                    }
                }
                let _span = archisim_geom::meter::span("conversión: terreno");
                let material = parts.material;
                let done = archisim_geom::parallel::map(&todo, |tile| {
                    let mut sink = PieceSink::default();
                    add_solids(
                        model,
                        vec![Solid {
                            owner: SolidOwner::Terrain(t.id),
                            material,
                            triangles: tile.as_ref().clone(),
                        }],
                        &look,
                        &mut sink,
                    );
                    Arc::new(sink.passes.map(|b| b.mesh))
                });
                for (tile, meshes) in todo.into_iter().zip(done) {
                    let key = Arc::as_ptr(tile) as usize;
                    tiles.insert(key, (t.id.0, tile.clone(), meshes.clone(), shown));
                    let made = out.entry((TILE, key as u64)).or_default();
                    made.owner = Some(t.id.0);
                    made.keep = Some(tile.clone());
                    push_meshes(made, &meshes, owner);
                }
                // Its walls, bottom, sides and painted areas: the same
                // while they are.
                let unit = (TILE, Arc::as_ptr(&parts.rest) as usize as u64);
                if self.existing.contains(&unit) {
                    still.push(unit);
                } else {
                    let made = out.entry(unit).or_default();
                    made.owner = Some(t.id.0);
                    made.keep = Some(parts.rest.clone());
                    for s in parts.rest.iter() {
                        made.add(model, s.clone(), &look);
                    }
                }
            }
        }
        // Everything else, as it is made, a few at a time.
        {
            let _span = archisim_geom::meter::span("escena: sólidos sueltos");
            let mut units = BTreeSet::new();
            for (kind, unit) in [
                (CLADDING, Unit::Cladding),
                (FITTINGS, Unit::Fittings),
                (FRAMES, Unit::Frames),
                (ORTHOPHOTO, Unit::Orthophoto),
                (GROWTH, Unit::Growth),
            ] {
                if whole.contains(&kind) {
                    units.insert(unit);
                }
            }
            let only = Only {
                ids: want.cloned(),
                units,
                no_terrains: true,
            };
            let mut zone_solids: Vec<Solid> = Vec::new();
            let generation = super::next_piece_generation();
            let mut put = |unit: Unit, solid: Solid| {
                if zone_ids.contains(&solid.owner.id()) {
                    zone_solids.push(solid.clone());
                }
                let key = unit_key(unit, solid.owner.id());
                let made = out.entry(key).or_default();
                // The streets and their markings, kept as they were made:
                // a junction made again as it was is not converted again.
                if unit == Unit::Element {
                    let k = archisim_geom::meter::time("escena: huella de piezas", || {
                        piece_key(model, &solid, &look)
                    });
                    if let Some(k) = k {
                        if hidden(solid.owner.id()) {
                            return;
                        }
                        let owner = Owner::Model(element(solid.owner));
                        let meshes = super::kept_piece(k, generation).unwrap_or_else(|| {
                            let mut c = super::Capture::default();
                            add_solids(model, vec![solid], &look, &mut c);
                            let m = Arc::new(c.passes.map(|b| b.mesh));
                            super::keep_piece(k, m.clone(), generation);
                            m
                        });
                        for (i, pass) in [Pass::Opaque, Pass::Glass, Pass::Grass]
                            .into_iter()
                            .enumerate()
                        {
                            if let Some(p) = Piece::new(pass, meshes[i].clone(), owner) {
                                made.pieces.push(p);
                            }
                        }
                        return;
                    }
                }
                made.add(model, solid, &look);
            };
            let dressed = !self.isolate && model.masses().any(|m| m.dress.is_some());
            let mut each = || {
                archisim_geom::solid::model_solids_units(
                    model,
                    Some(&only),
                    &skip,
                    &detailed,
                    &mut put,
                )
            };
            if dressed {
                archisim_geom::solid::without_dressed(&mut each);
            } else {
                each();
            }
            // The streets' pieces kept only while used (with a whole
            // making: those of this scene and the one before).
            if want.is_none() {
                super::forget_pieces_before(generation);
            }
            // Where the layers of a wall, a slab or a roof touch: seen
            // only through a cut.
            let between = self.look(&anchors, false, true);
            let idx = archisim_geom::WallIndex::new(model);
            for w in model.walls().filter(|w| wants(w.id.0)) {
                let made = out.entry((ELEMENT, w.id.0)).or_default();
                for s in archisim_geom::solid::wall_inner_faces(&idx, w.id) {
                    made.add(model, s, &between);
                }
            }
            for sl in model.slabs().filter(|s| wants(s.id.0)) {
                let made = out.entry((ELEMENT, sl.id.0)).or_default();
                for s in archisim_geom::solid::slab_inner_faces(&idx, sl.id) {
                    made.add(model, s, &between);
                }
            }
            for r in model.roofs().filter(|r| wants(r.id.0)) {
                let made = out.entry((ELEMENT, r.id.0)).or_default();
                for s in archisim_geom::solid::roof_inner_faces(model, r.id) {
                    made.add(model, s, &between);
                }
            }
            // Painted zones of walls' faces, just off them.
            if whole.contains(&PAINT) {
                let made = out.entry((PAINT, 0)).or_default();
                let mut sinks: BTreeMap<Element, PieceSink> = BTreeMap::new();
                for (wall, tri, [r, g, b]) in archisim_geom::solid::paint_zone_triangles(model) {
                    if hidden(wall.0) {
                        continue;
                    }
                    let e = Element::Wall(wall);
                    let rgb = [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b)];
                    let select = 0.0;
                    let cut = anchors
                        .get(&wall)
                        .map_or([cutaway::WALL, 0.0, 0.0, 0.0], |a| a.cut());
                    let v = flat_tri(tri, [rgb[0], rgb[1], rgb[2], 1.0], -1.0, cut, select);
                    sinks
                        .entry(e)
                        .or_default()
                        .push(Pass::Opaque, v, Owner::Model(e));
                }
                for s in sinks.into_values() {
                    made.take(s);
                }
            }
            // Painted zones of the faces of slabs, footings… (G3).
            if whole.contains(&FACE_ZONES) {
                let made = out.entry((FACE_ZONES, 0)).or_default();
                let mut sinks: BTreeMap<Element, PieceSink> = BTreeMap::new();
                for (owner, tri, [r, g, b]) in
                    archisim_geom::solid::face_zone_triangles(model, &zone_solids)
                {
                    if hidden(owner.id()) {
                        continue;
                    }
                    let e = element(owner);
                    let rgb = [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b)];
                    let select = 0.0;
                    let v = flat_tri(
                        tri,
                        [rgb[0], rgb[1], rgb[2], 1.0],
                        -1.0,
                        [cutaway::PLAIN, 0.0, 0.0, 0.0],
                        select,
                    );
                    sinks
                        .entry(e)
                        .or_default()
                        .push(Pass::Opaque, v, Owner::Model(e));
                }
                for s in sinks.into_values() {
                    made.take(s);
                }
            }
        }
        // The elements of linked models.
        if !linked.is_empty() {
            let made = out.entry((LINKED, 0)).or_default();
            for part in linked {
                let [r, g, b] = part.color;
                let color = [
                    srgb_to_linear(r),
                    srgb_to_linear(g),
                    srgb_to_linear(b),
                    part.alpha,
                ];
                let pass = if part.alpha < 1.0 {
                    Pass::Glass
                } else {
                    Pass::Opaque
                };
                let owner = Owner::Linked(part.link, part.index);
                let mut sink = PieceSink::default();
                for tri in part.triangles {
                    let v = flat_tri(*tri, color, 0.0, [cutaway::PLAIN, 0.0, 0.0, 0.0], 0.0);
                    sink.push(pass, v, owner);
                }
                made.take(sink);
            }
        }
        self.still.extend(still);
        *self.anchors = anchors;
        for m in out.values_mut() {
            m.finish_lamps();
        }
        out
    }

    /// The furniture, doors, windows, buildings and plants drawn from a
    /// model made once: their units, and those to leave out of the rest.
    fn make_placed(
        &mut self,
        out: &mut BTreeMap<UnitKey, Made>,
        skip: &mut HashSet<ObjectId>,
        detailed: &mut HashSet<archisim_core::OpeningId>,
        want: Option<&HashSet<u64>>,
        zone_ids: &BTreeSet<u64>,
    ) {
        let model = self.model;
        let disciplines = self.disciplines;
        let hidden = |id: u64| model.hidden_by_discipline(&disciplines.hidden, id);
        let no_anchors = HashMap::new();
        let wants = |id: u64| want.is_none_or(|w| w.contains(&id));
        // The objects to make: those asked for, by id (not every object of
        // the model looked at).
        let objects: Vec<&archisim_core::Object> = match want {
            None => model.objects().collect(),
            Some(ids) => {
                let mut v: Vec<&archisim_core::Object> = ids
                    .iter()
                    .filter_map(|id| model.object(ObjectId(*id)))
                    .collect();
                v.sort_by_key(|o| o.id.0);
                v
            }
        };
        let openings: Vec<&archisim_core::Opening> = match want {
            None => model.openings().collect(),
            Some(ids) => {
                let mut v: Vec<&archisim_core::Opening> = ids
                    .iter()
                    .filter_map(|id| model.opening(archisim_core::OpeningId(*id)))
                    .collect();
                v.sort_by_key(|o| o.id.0);
                v
            }
        };
        // The furniture shared by type.
        {
            let _span = archisim_geom::meter::span("escena: objetos repetidos");
            for o in objects.iter().copied() {
                if hidden(o.id.0) {
                    skip.insert(o.id);
                    continue;
                }
                let Some(inst) = archisim_geom::solid::object_instance(model, o.id) else {
                    continue;
                };
                skip.insert(o.id);
                let key = (
                    Arc::as_ptr(&inst.mesh) as usize,
                    inst.parts.iter().map(|p| p.0 .0).collect::<Vec<_>>(),
                );
                let g = match self.models.get(&key) {
                    Some(g) => g.clone(),
                    None => {
                        let mut shared = Shared::default();
                        let solids = instance_solids(
                            &inst.parts,
                            &inst.mesh.positions,
                            SolidOwner::Object(o.id),
                        );
                        add_solids(
                            model,
                            solids,
                            &self.look(&no_anchors, true, false),
                            &mut shared,
                        );
                        let mut g = GroupModel::finish(shared, false);
                        g._from = Some(inst.mesh.clone());
                        let g = Arc::new(g);
                        self.models.insert(key, g.clone());
                        g
                    }
                };
                let (sn, cs) = (inst.rotation as f32).sin_cos();
                let mut instance = Instance {
                    place: [cs, sn, inst.origin[0] as f32, inst.origin[1] as f32],
                    lift: [inst.origin[2] as f32, 1.0, 1.0, 1.0],
                    cut: [0.0; 4],
                    flags: self.flags(Element::Object(o.id), o.id.0),
                };
                // Its box where it stands, from the corners of the model's.
                let Some((lo, hi)) = placed_box(&instance, g.lo, g.hi) else {
                    continue;
                };
                let c = (lo + hi) / 2.0;
                instance.cut = [cutaway::OBJECT, c.x, c.y, inst.origin[2] as f32];
                let made = out.entry((ELEMENT, o.id.0)).or_default();
                if let Some((sum, n, reach)) = g.glow.filter(|g| g.1 > 0.0) {
                    if instance.flags[1] < 0.5 {
                        let m = instance.place(sum / n);
                        made.lamps.push([m.x, m.y, m.z, reach]);
                    }
                }
                made.placed.push(Placed {
                    model: g,
                    instance,
                    owner: Element::Object(o.id),
                    bounds: (lo, hi),
                });
            }
        }
        // The doors and windows whose family makes a detailed model
        // (paquete 9), cut as their walls.
        {
            let _span = archisim_geom::meter::span("escena: puertas y ventanas");
            for o in openings.iter().copied() {
                if hidden(o.id.0) {
                    continue;
                }
                let Some(inst) = archisim_geom::solid::opening_detail(model, o.id) else {
                    continue;
                };
                detailed.insert(o.id);
                let key = (
                    Arc::as_ptr(&inst.mesh) as usize,
                    inst.parts.iter().map(|p| p.0 .0).collect::<Vec<_>>(),
                );
                let g = match self.models.get(&key) {
                    Some(g) => g.clone(),
                    None => {
                        let look = self.look(&no_anchors, true, false);
                        let mut shared = Shared::default();
                        let solids = instance_solids(
                            &inst.parts,
                            &inst.mesh.positions,
                            SolidOwner::Opening(o.id),
                        );
                        add_solids(model, solids, &look, &mut shared);
                        if let Some(far) =
                            archisim_geom::solid::opening_detail_with(model, o.id, true)
                        {
                            let solids = instance_solids(
                                &far.parts,
                                &far.mesh.positions,
                                SolidOwner::Opening(o.id),
                            );
                            let mut plain = Shared::default();
                            add_solids(model, solids, &look, &mut plain);
                            shared.simple = Some(plain.opaque.mesh);
                        }
                        let mut g = GroupModel::finish(shared, true);
                        g._from = Some(inst.mesh.clone());
                        let g = Arc::new(g);
                        self.models.insert(key, g.clone());
                        g
                    }
                };
                let (sn, cs) = (inst.rotation as f32).sin_cos();
                let mut instance = Instance {
                    place: [cs, sn, inst.origin[0] as f32, inst.origin[1] as f32],
                    lift: [inst.origin[2] as f32, 1.0, 1.0, 1.0],
                    cut: [
                        cutaway::WALL,
                        inst.origin[0] as f32,
                        inst.origin[1] as f32,
                        inst.origin[2] as f32,
                    ],
                    flags: self.flags(Element::Opening(o.id), o.id.0),
                };
                let mut lo = Vec3::splat(f32::INFINITY);
                let mut hi = Vec3::splat(f32::NEG_INFINITY);
                for v in g.opaque.vertices.iter().chain(&g.glass.vertices) {
                    let p = instance.place(Vec3::from_array(v.pos));
                    lo = lo.min(p);
                    hi = hi.max(p);
                }
                if lo.x > hi.x {
                    continue;
                }
                instance.cut[3] = lo.z;
                if let Some(a) = self.anchors.get(&inst.wall) {
                    instance.cut = a.cut();
                }
                out.entry((ELEMENT, o.id.0))
                    .or_default()
                    .placed
                    .push(Placed {
                        model: g,
                        instance,
                        owner: Element::Opening(o.id),
                        bounds: (lo, hi),
                    });
            }
        }
        // The buildings of the surroundings: each its own model with all
        // its detail near and its light version far.
        {
            let _span = archisim_geom::meter::span("escena: edificios vestidos");
            for m in model.masses().filter(|m| wants(m.id.0)) {
                if m.dress.is_none() || hidden(m.id.0) {
                    continue;
                }
                let near = !self.crowd || self.detailed_masses.contains(&m.id);
                let full = archisim_geom::env_buildings::dressed_solids_detail(model, m, !near);
                if full.is_empty() {
                    continue;
                }
                let light = near
                    .then(|| archisim_geom::env_buildings::dressed_solids_detail(model, m, true));
                // Made before as it is now: its model as it was.
                let key = model_key(
                    model,
                    &full,
                    light.as_deref().unwrap_or(&[]),
                    u64::from(near),
                );
                let g = match kept_model(key) {
                    Some(g) => g,
                    None => {
                        let look = self.look(&no_anchors, true, false);
                        let mut shared = Shared::default();
                        add_solids(model, full, &look, &mut shared);
                        if let Some(light) = light {
                            let mut plain = Shared::default();
                            add_solids(model, light, &look, &mut plain);
                            shared.simple = Some(plain.opaque.mesh);
                        }
                        let g = Arc::new(GroupModel::finish(shared, false));
                        keep_model(key, g.clone());
                        g
                    }
                };
                if g.lo.x > g.hi.x {
                    continue;
                }
                let instance = Instance {
                    place: [1.0, 0.0, 0.0, 0.0],
                    lift: [0.0, 1.0, 1.0, 1.0],
                    cut: [cutaway::PLAIN, 0.0, 0.0, g.lo.z],
                    flags: self.flags(Element::Mass(m.id), m.id.0),
                };
                let made = out.entry((ELEMENT, m.id.0)).or_default();
                made.context = dressed_context(model, m.id, near);
                made.placed.push(Placed {
                    bounds: (g.lo, g.hi),
                    model: g,
                    instance,
                    owner: Element::Mass(m.id),
                });
            }
        }
        // The plants nearest the project (its origin) with every leaf,
        // when there are so many that the project grows them all coarser.
        {
            use archisim_core::flora::Lod;
            let _span = archisim_geom::meter::span("escena: plantas");
            let base = model.plant_detail();
            for id in self.near_plants.iter().filter(|id| wants(id.0)) {
                let full = archisim_geom::solid::with_plant_lod(Lod::Full, || {
                    archisim_geom::solid::plant_solids(model, *id)
                });
                let far = archisim_geom::solid::with_plant_lod(base, || {
                    archisim_geom::solid::plant_solids(model, *id)
                });
                let (Some(full), Some(far)) = (full, far) else {
                    continue;
                };
                let key = model_key(model, &full, &far, 2);
                let g = match kept_model(key) {
                    Some(g) => g,
                    None => {
                        let look = self.look(&no_anchors, true, false);
                        let mut shared = Shared::default();
                        add_solids(model, full, &look, &mut shared);
                        let mut plain = Shared::default();
                        add_solids(model, far, &look, &mut plain);
                        shared.simple = Some(plain.opaque.mesh);
                        let g = Arc::new(GroupModel::finish(shared, false));
                        keep_model(key, g.clone());
                        g
                    }
                };
                if g.lo.x > g.hi.x {
                    continue;
                }
                skip.insert(*id);
                let c = (g.lo + g.hi) / 2.0;
                let instance = Instance {
                    place: [1.0, 0.0, 0.0, 0.0],
                    lift: [0.0, 1.0, 1.0, 1.0],
                    cut: [cutaway::OBJECT, c.x, c.y, g.lo.z],
                    flags: self.flags(Element::Object(*id), id.0),
                };
                out.entry((ELEMENT, id.0)).or_default().placed.push(Placed {
                    bounds: (g.lo, g.hi),
                    model: g,
                    instance,
                    owner: Element::Object(*id),
                });
            }
        }
        // The plants (each grows its own): their models made once in their
        // own axes and kept, here only turned and moved to where each
        // stands. Not those greyed or with painted zones.
        {
            let _span = archisim_geom::meter::span("escena: plantas colocadas");
            let lod = archisim_geom::solid::plant_lod(model);
            for o in objects.iter().copied() {
                if skip.contains(&o.id)
                    || !model.type_grows(o.object_type)
                    || model.dimmed(self.disciplines.focus, o.id.0)
                    || zone_ids.contains(&o.id.0)
                {
                    continue;
                }
                let Some((mesh, parts)) = model.plant_mesh(o.id, lod) else {
                    continue;
                };
                let local = super::plant_model(model, o.id, mesh, &parts, lod);
                let made = out.entry((ELEMENT, o.id.0)).or_default();
                made.pieces.extend(place_plant(model, o, &local));
                skip.insert(o.id);
            }
        }
    }
}

/// A plant's model turned and moved to where the plant stands.
fn place_plant(model: &Model, o: &archisim_core::Object, plant: &PlantModel) -> Vec<Piece> {
    let z0 = model.level(o.level).map_or(0.0, |l| l.elevation) + o.base_offset;
    let (sn, cs) = o.rotation.sin_cos();
    let cut = [
        cutaway::OBJECT,
        o.position.x as f32,
        o.position.y as f32,
        z0 as f32,
    ];
    let mut out = Vec::new();
    for (pass, mesh) in [(Pass::Opaque, &plant.opaque), (Pass::Glass, &plant.glass)] {
        if mesh.is_empty() {
            continue;
        }
        let vertices: Vec<Vertex> = mesh
            .vertices
            .iter()
            .map(|v| {
                let mut w = *v;
                let q = o.to_plan(v.pos[0] as f64, v.pos[1] as f64);
                w.pos = [q.x as f32, q.y as f32, (z0 + v.pos[2] as f64) as f32];
                let (nx, ny) = (v.normal[0] as f64, v.normal[1] as f64);
                w.normal[0] = (nx * cs - ny * sn).round().clamp(-32767.0, 32767.0) as i16;
                w.normal[1] = (nx * sn + ny * cs).round().clamp(-32767.0, 32767.0) as i16;
                w.cut = cut;
                w
            })
            .collect();
        let m = Mesh {
            vertices,
            indices: mesh.indices.clone(),
        };
        out.extend(Piece::new(pass, m, Owner::Model(Element::Object(o.id))));
    }
    out
}

/// A tile's pieces, by pass.
fn push_meshes(made: &mut Made, meshes: &[Mesh; 3], owner: Owner) {
    for (k, pass) in [Pass::Opaque, Pass::Glass, Pass::Grass]
        .into_iter()
        .enumerate()
    {
        if let Some(p) = Piece::new(pass, meshes[k].clone(), owner) {
            made.pieces.push(p);
        }
    }
}

/// The solids of a shared model's parts, for making it once.
fn instance_solids(
    parts: &[(archisim_core::MaterialId, Vec<[u32; 3]>)],
    positions: &[[f32; 3]],
    owner: SolidOwner,
) -> Vec<Solid> {
    parts
        .iter()
        .map(|(material, tris)| Solid {
            owner,
            material: *material,
            triangles: tris
                .iter()
                .map(|t| t.map(|i| positions[i as usize].map(|c| c as f64)))
                .collect(),
        })
        .collect()
}

/// The box of a model `lo`–`hi` where an instance stands.
fn placed_box(instance: &Instance, lo: Vec3, hi: Vec3) -> Option<(Vec3, Vec3)> {
    if lo.x > hi.x {
        return None;
    }
    let mut a = Vec3::splat(f32::INFINITY);
    let mut b = Vec3::splat(f32::NEG_INFINITY);
    for k in 0..8 {
        let corner = Vec3::new(
            if k & 1 == 0 { lo.x } else { hi.x },
            if k & 2 == 0 { lo.y } else { hi.y },
            if k & 4 == 0 { lo.z } else { hi.z },
        );
        let p = instance.place(corner);
        a = a.min(p);
        b = b.max(p);
    }
    Some((a, b))
}

/// A triangle of one colour, flat.
fn flat_tri(
    tri: [[f64; 3]; 3],
    color: [f32; 4],
    bias: f32,
    cut: [f32; 4],
    select: f32,
) -> [Vertex; 3] {
    let p = tri.map(|v| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32));
    let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
    p.map(|v| Vertex::new(v, n, color, bias, [0.0, 0.0], cut, select))
}

/// The cell of a piece: its pass, its layer and the square of its middle.
fn cell_of(pass: Pass, layer: u8, lo: Vec3, hi: Vec3) -> CellKey {
    let c = (lo + hi) / 2.0;
    (
        pass as u8,
        layer,
        (c.x / CELL).floor() as i32,
        (c.y / CELL).floor() as i32,
    )
}

/// The squares of the index a box is in.
fn squares(lo: Vec3, hi: Vec3) -> impl Iterator<Item = (i32, i32)> {
    let (x0, x1) = ((lo.x / GRID).floor() as i32, (hi.x / GRID).floor() as i32);
    let (y0, y1) = ((lo.y / GRID).floor() as i32, (hi.y / GRID).floor() as i32);
    // A box of no size or too big (a mistake): its middle only.
    let huge = (x1 - x0) as i64 * (y1 - y0) as i64 > 1_000_000 || x0 > x1 || y0 > y1;
    let (x0, x1, y0, y1) = if huge {
        let c = (lo + hi) / 2.0;
        let (x, y) = ((c.x / GRID).floor() as i32, (c.y / GRID).floor() as i32);
        (x, x, y, y)
    } else {
        (x0, x1, y0, y1)
    };
    (x0..=x1).flat_map(move |x| (y0..=y1).map(move |y| (x, y)))
}

/// Where a street changed, if only some of it did: its nodes changed, the
/// paving of some stretches or some pieces hidden, but not its shape. The
/// boxes round those places (on plan; any height). None: it changed
/// whole (or it is not a street in both).
fn local_road_change(old: &Model, new: &Model, id: u64) -> Option<Vec<(Vec3, Vec3)>> {
    use archisim_core::{Point2, RoadId};
    let road = RoadId(id);
    let (a, b) = (old.road(road)?, new.road(road)?);
    // The same street but for what changes only where it is.
    let mut same = a.clone();
    same.skins = b.skins.clone();
    same.hidden = b.hidden.clone();
    if &same != b {
        return None;
    }
    let mut out: Vec<(Vec3, Vec3)> = Vec::new();
    let mut around = |pts: &[Point2]| {
        if pts.is_empty() {
            return;
        }
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for p in pts {
            lo = lo.min(Vec3::new(p.x as f32, p.y as f32, -1.0e4));
            hi = hi.max(Vec3::new(p.x as f32, p.y as f32, 1.0e4));
        }
        // The street's width about them.
        let w = b.section.width() as f32;
        out.push((lo - Vec3::new(w, w, 0.0), hi + Vec3::new(w, w, 0.0)));
    };
    // The stretches whose paving changed: their ends and the axis between.
    let stretch = |r: &archisim_core::Road, from: f64, to: f64| -> Vec<Point2> {
        let (from, to) = (from.min(to), from.max(to));
        let mut pts = vec![r.axis_at(from).0, r.axis_at(to).0];
        let mut s = 0.0;
        for w in r.axis.windows(2) {
            if (from..=to).contains(&s) {
                pts.push(w[0]);
            }
            s += w[0].distance(w[1]);
        }
        pts
    };
    for k in a.skins.iter().filter(|k| !b.skins.contains(k)) {
        around(&stretch(a, k.from, k.to));
    }
    for k in b.skins.iter().filter(|k| !a.skins.contains(k)) {
        around(&stretch(b, k.from, k.to));
    }
    // The pieces hidden or shown again.
    for h in a
        .hidden
        .iter()
        .filter(|h| !b.hidden.contains(h))
        .chain(b.hidden.iter().filter(|h| !a.hidden.contains(h)))
    {
        let (q, u) = b.axis_at(h.at);
        around(&[q + u.perp_left() * h.off]);
    }
    // The nodes of the street that changed: their junctions, as they were
    // and as they are.
    let nodes = |m: &Model| -> BTreeMap<archisim_core::NodeKey, archisim_core::NodeSettings> {
        m.nodes()
            .filter(|n| n.key.road_ids().any(|r| r == road))
            .map(|n| (n.key.clone(), n.clone()))
            .collect()
    };
    let (was, now) = (nodes(old), nodes(new));
    for key in was.keys().chain(now.keys()) {
        if was.get(key) == now.get(key) {
            continue;
        }
        for m in [old, new] {
            if let Some(j) = archisim_geom::crossroads::node_junction(m, key) {
                let pts: Vec<Point2> = std::iter::once(&j.plate)
                    .chain(&j.sidewalks)
                    .chain(&j.kerbs)
                    .flatten()
                    .copied()
                    .chain(j.arms.iter().map(|x| x.origin))
                    .collect();
                around(&pts);
            }
        }
    }
    Some(out)
}

fn overlaps(a: (Vec3, Vec3), b: (Vec3, Vec3)) -> bool {
    a.0.x <= b.1.x
        && b.0.x <= a.1.x
        && a.0.y <= b.1.y
        && b.0.y <= a.1.y
        && a.0.z <= b.1.z
        && b.0.z <= a.1.z
}

/// Some pieces and some parts of old chunks put in new chunks of up to
/// [`CHUNK`] triangles, each part marked selected or not.
struct Packer<'a> {
    selection: &'a BTreeSet<Element>,
    out: Vec<Chunk>,
    mesh: Mesh,
    owners: Vec<Owner>,
    parts: Vec<Part>,
    lo: Vec3,
    hi: Vec3,
}

impl<'a> Packer<'a> {
    fn new(selection: &'a BTreeSet<Element>) -> Self {
        Packer {
            selection,
            out: Vec::new(),
            mesh: Mesh::default(),
            owners: Vec::new(),
            parts: Vec::new(),
            lo: Vec3::splat(f32::INFINITY),
            hi: Vec3::splat(f32::NEG_INFINITY),
        }
    }

    fn room_for(&mut self, triangles: usize) {
        if !self.owners.is_empty() && self.owners.len() + triangles > CHUNK {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if self.owners.is_empty() {
            return;
        }
        self.out.push(Chunk {
            mesh: Arc::new(std::mem::take(&mut self.mesh)),
            owners: Arc::new(std::mem::take(&mut self.owners)),
            lo: self.lo,
            hi: self.hi,
            key: next_key(),
            parts: Arc::new(std::mem::take(&mut self.parts)),
        });
        self.lo = Vec3::splat(f32::INFINITY);
        self.hi = Vec3::splat(f32::NEG_INFINITY);
    }

    fn selected(&self, owner: Owner) -> bool {
        matches!(owner, Owner::Model(e) if self.selection.contains(&e))
    }

    /// Some corners and triangles of one unit and one owner.
    fn add(
        &mut self,
        unit: UnitKey,
        owner: Owner,
        vertices: &[Vertex],
        indices: &[u32],
        base: u32,
    ) {
        let n = indices.len() / 3;
        if n == 0 {
            return;
        }
        self.room_for(n);
        let on = u8::from(self.selected(owner));
        let v0 = self.mesh.vertices.len() as u32;
        let i0 = self.mesh.indices.len() as u32;
        for v in vertices {
            let mut w = *v;
            if w.misc[1] < 2 {
                w.misc[1] = on;
            }
            let p = Vec3::from_array(w.pos);
            self.lo = self.lo.min(p);
            self.hi = self.hi.max(p);
            self.mesh.vertices.push(w);
        }
        self.mesh
            .indices
            .extend(indices.iter().map(|i| i - base + v0));
        self.owners.extend(std::iter::repeat_n(owner, n));
        self.parts.push(Part {
            unit,
            vertices: [v0, vertices.len() as u32],
            indices: [i0, indices.len() as u32],
        });
    }

    fn finish(mut self) -> Vec<Chunk> {
        self.flush();
        self.out
    }
}

impl SceneBuilder {
    /// [`SceneBuilder::build`], with the plants in their medium version in
    /// the browser (short of memory, `docs/IPAD.md` §12).
    pub fn build_scene(
        &mut self,
        model: Arc<Model>,
        selection: &BTreeSet<Element>,
        disciplines: &Disciplines,
    ) -> Scene {
        use archisim_core::flora::Lod;
        if crate::platform::WEB && model.plant_detail() == Lod::Full {
            return archisim_geom::solid::with_plant_lod(Lod::Medium, || {
                self.build(model, selection, disciplines).0
            });
        }
        self.build(model, selection, disciplines).0
    }

    /// The scene of `model`, with `selection` marked: everything made the
    /// first time, and after only what changed since the last one.
    pub fn build(
        &mut self,
        model: Arc<Model>,
        selection: &BTreeSet<Element>,
        disciplines: &Disciplines,
    ) -> (Scene, BuildStats) {
        let setting = Setting::of(&model, disciplines);
        let state = self.state.take().filter(|s| s.setting == setting);
        let (state, stats) = match state {
            Some(s) => {
                let changes = archisim_geom::meter::time("escena: qué cambió", || {
                    model.changes_since(&s.model)
                });
                if changes.global {
                    whole(model, setting, selection)
                } else {
                    s.update(model, &changes.ids, selection)
                }
            }
            None => whole(model, setting, selection),
        };
        let scene = state.scene();
        self.state = Some(state);
        (scene, stats)
    }
}

/// Everything made: a new state.
fn whole(
    model: Arc<Model>,
    setting: Setting,
    selection: &BTreeSet<Element>,
) -> (State, BuildStats) {
    let mut st = State {
        near_plants: near_plants(&model, &setting.disciplines),
        detailed_masses: detailed_masses(&model, setting.crowd),
        model,
        setting,
        units: BTreeMap::new(),
        grid: HashMap::new(),
        cells: BTreeMap::new(),
        anchors: HashMap::new(),
        models: HashMap::new(),
        selection: selection.clone(),
    };
    let model = st.model.clone();
    let made = {
        let mut making = Making {
            model: &model,
            disciplines: &st.setting.disciplines,
            selection: &BTreeSet::new(),
            isolate: false,
            anchors: &mut st.anchors,
            models: &mut st.models,
            near_plants: &st.near_plants,
            detailed_masses: &st.detailed_masses,
            crowd: st.setting.crowd,
            existing: &HashSet::new(),
            still: Vec::new(),
        };
        let whole: BTreeSet<u8> = [
            CLADDING, FITTINGS, FRAMES, ORTHOPHOTO, GROWTH, PAINT, FACE_ZONES,
        ]
        .into_iter()
        .collect();
        making.make(None, &whole, &[])
    };
    let units = made.len();
    let replaced = BTreeSet::new();
    st.install(&model, made, &replaced, &BTreeSet::new());
    let stats = BuildStats {
        units,
        cells: st.cells.len(),
        whole: true,
    };
    (st, stats)
}

impl State {
    /// What changed made again, with what depends on it.
    fn update(
        mut self,
        model: Arc<Model>,
        changed: &BTreeSet<u64>,
        selection: &BTreeSet<Element>,
    ) -> (State, BuildStats) {
        let _span = archisim_geom::meter::span("escena: rehacer lo cambiado");
        let class = |id: u64| -> Class {
            let c = Class::of(&model, id);
            if c != Class::General {
                return c;
            }
            // Gone: what it was.
            self.units
                .get(&(ELEMENT, id))
                .map_or_else(|| Class::of(&self.model, id), |k| k.class)
        };
        let classes: Vec<(u64, Class)> = changed.iter().map(|id| (*id, class(*id))).collect();
        let mut want: HashSet<u64> = changed.iter().copied().collect();
        // The plants that gain or lose every leaf, the buildings their
        // detail.
        let near = near_plants(&model, &self.setting.disciplines);
        want.extend(near.symmetric_difference(&self.near_plants).map(|o| o.0));
        self.near_plants = near;
        let detailed = detailed_masses(&model, self.setting.crowd);
        want.extend(
            detailed
                .symmetric_difference(&self.detailed_masses)
                .map(|m| m.0),
        );
        self.detailed_masses = detailed;
        // The pieces of the whole model that depend on what changed.
        let mut whole: BTreeSet<u8> = BTreeSet::new();
        let any = |f: &dyn Fn(Class) -> bool| classes.iter().any(|(_, c)| f(*c));
        if any(&|c| c == Class::General) {
            whole.extend([CLADDING, FITTINGS, FRAMES, PAINT]);
        }
        if any(&|c| c.shapes_ground()) {
            whole.extend([ORTHOPHOTO, GROWTH]);
            want.extend(model.terrains().map(|t| t.id.0));
        }
        // A terrain changed: the streets on it.
        if any(&|c| c == Class::Terrain) {
            for (k, kept) in &self.units {
                if k.0 == ELEMENT && matches!(kept.class, Class::Road | Class::Marking) {
                    want.insert(k.1);
                }
            }
        }
        let zone_ids: BTreeSet<u64> = model
            .face_zones()
            .filter(|z| z.material.is_some())
            .map(|z| z.element)
            .collect();
        let zones = |want: &HashSet<u64>| zone_ids.iter().any(|id| want.contains(id));
        if zones(&want) {
            whole.insert(FACE_ZONES);
            want.extend(zone_ids.iter().copied());
        }
        // First what changed, to know where it is now.
        let (mut made, mut still) = self.make(&model, &want, &whole);
        // Then what depends on it within its reach.
        let mut area: Vec<((Vec3, Vec3), Class)> = Vec::new();
        for (id, c) in &classes {
            let r = Vec3::new(c.reach(), c.reach(), c.reach());
            // A street that changed only near some points (a node, a
            // stretch's paving, a hidden piece): what is round those.
            if *c == Class::Road {
                if let Some(local) = local_road_change(&self.model, &model, *id) {
                    for b in local {
                        area.push(((b.0 - r, b.1 + r), *c));
                    }
                    continue;
                }
            }
            let old = self.units.get(&(ELEMENT, *id)).map(|k| k.boxes.clone());
            let new = made.get(&(ELEMENT, *id)).map(boxes_of);
            for b in old.into_iter().chain(new).flatten() {
                area.push(((b.0 - r, b.1 + r), *c));
            }
        }
        let mut more: HashSet<u64> = HashSet::new();
        {
            let _span = archisim_geom::meter::span("escena: alrededor");
            let mut seen: HashSet<UnitKey> = HashSet::new();
            for (b, c) in &area {
                for sq in squares(b.0, b.1) {
                    let Some(units) = self.grid.get(&sq) else {
                        continue;
                    };
                    for u in units {
                        if u.0 != ELEMENT || want.contains(&u.1) || !seen.insert(*u) {
                            continue;
                        }
                        let Some(k) = self.units.get(u) else {
                            continue;
                        };
                        if k.class.listens(*c) && k.boxes.iter().any(|x| overlaps(*x, *b)) {
                            more.insert(u.1);
                        } else {
                            // Another change may reach it.
                            seen.remove(u);
                        }
                    }
                }
            }
        }
        // The buildings of the surroundings dressed as before: kept.
        more.retain(|id| {
            let Some(k) = self.units.get(&(ELEMENT, *id)) else {
                return true;
            };
            if k.class != Class::Dressed {
                return true;
            }
            let near = !self.setting.crowd || self.detailed_masses.contains(&MassId(*id));
            dressed_context(&model, MassId(*id), near) != k.context
        });
        let mut whole_b: BTreeSet<u8> = BTreeSet::new();
        if zones(&more) && !whole.contains(&FACE_ZONES) {
            // Their painted zones need every zoned element's solids.
            whole_b.insert(FACE_ZONES);
            more.extend(zone_ids.iter().copied());
        }
        if !more.is_empty() || !whole_b.is_empty() {
            let (also, same) = self.make(&model, &more, &whole_b);
            made.extend(also);
            still.extend(same);
        }
        want.extend(more);
        whole.extend(whole_b);
        // The units made again: the elements, and every piece of the
        // whole model of the kinds made (some may be gone).
        let mut replaced: BTreeSet<UnitKey> = want.iter().map(|id| (ELEMENT, *id)).collect();
        let still: HashSet<UnitKey> = still.into_iter().collect();
        for (k, kept) in &self.units {
            if k.0 != ELEMENT && whole.contains(&k.0) {
                replaced.insert(*k);
            }
            // The tiles of a terrain made again that are no longer in it.
            if k.0 == TILE && want.contains(&kept.owner) && !still.contains(k) {
                replaced.insert(*k);
            }
        }
        let units = made.len();
        let old_selection = std::mem::replace(&mut self.selection, selection.clone());
        let toggled: BTreeSet<Element> = old_selection
            .symmetric_difference(selection)
            .copied()
            .collect();
        self.model = model.clone();
        self.install(&model, made, &replaced, &toggled);
        let stats = BuildStats {
            units,
            cells: self.cells.len(),
            whole: false,
        };
        (self, stats)
    }

    /// The units of `want` and of the pieces `whole`, and the tiles of
    /// the ground found still the same.
    fn make(
        &mut self,
        model: &Model,
        want: &HashSet<u64>,
        whole: &BTreeSet<u8>,
    ) -> (BTreeMap<UnitKey, Made>, Vec<UnitKey>) {
        let existing: HashSet<UnitKey> = if model.terrains().any(|t| want.contains(&t.id.0)) {
            self.units.keys().filter(|k| k.0 == TILE).copied().collect()
        } else {
            HashSet::new()
        };
        let mut making = Making {
            model,
            disciplines: &self.setting.disciplines,
            selection: &BTreeSet::new(),
            isolate: false,
            anchors: &mut self.anchors,
            models: &mut self.models,
            near_plants: &self.near_plants,
            detailed_masses: &self.detailed_masses,
            crowd: self.setting.crowd,
            existing: &existing,
            still: Vec::new(),
        };
        let made = making.make(Some(want), whole, &[]);
        (made, making.still)
    }

    /// The units made put in place of those `replaced`, and the cells
    /// they are in made again (and those with an element whose selection
    /// was `toggled`).
    fn install(
        &mut self,
        model: &Model,
        made: BTreeMap<UnitKey, Made>,
        replaced: &BTreeSet<UnitKey>,
        toggled: &BTreeSet<Element>,
    ) {
        let _span = archisim_geom::meter::span("escena: celdas");
        let mut dirty: BTreeSet<CellKey> = BTreeSet::new();
        let mut gone: HashSet<UnitKey> = HashSet::new();
        for k in replaced.iter().chain(made.keys()) {
            if let Some(old) = self.units.remove(k) {
                dirty.extend(old.cells.iter().copied());
                for b in &old.boxes {
                    for sq in squares(b.0, b.1) {
                        if let Some(v) = self.grid.get_mut(&sq) {
                            v.retain(|u| u != k);
                        }
                    }
                }
            }
            gone.insert(*k);
        }
        // The cells of what was or is now selected (or not).
        for e in toggled {
            let Some(id) = crate::app::discipline_ui::element_id(*e) else {
                continue;
            };
            if let Some(k) = self.units.get(&(ELEMENT, id)) {
                dirty.extend(k.cells.iter().copied());
            }
        }
        if !toggled.is_empty() {
            let ids: HashSet<u64> = toggled
                .iter()
                .filter_map(|e| crate::app::discipline_ui::element_id(*e))
                .collect();
            for (k, kept) in &self.units {
                if (k.0 != ELEMENT && k.0 != TILE) || (k.0 == TILE && ids.contains(&kept.owner)) {
                    dirty.extend(kept.cells.iter().copied());
                }
            }
        }
        let mut incoming: BTreeMap<CellKey, Vec<(UnitKey, Piece)>> = BTreeMap::new();
        for (k, m) in made {
            let class = match k.0 {
                ELEMENT => Class::of(model, k.1),
                TILE => Class::Terrain,
                _ => Class::Whole,
            };
            let layer = match k.0 {
                ORTHOPHOTO | GROWTH => 1,
                _ => class.layer(),
            };
            let mut boxes = boxes_of(&m);
            boxes.dedup_by(|a, b| a == b);
            let mut cells: Vec<CellKey> = Vec::new();
            // A big unit (a long street, a tile of the ground) in cells of
            // its own: made again, it does not copy its neighbours.
            let mut size = [0usize; 3];
            for p in &m.pieces {
                size[p.pass as usize] += p.mesh.indices.len() / 3;
            }
            for p in m.pieces {
                let c = if size[p.pass as usize] >= OWN_CELL {
                    (
                        p.pass as u8,
                        OWN_LAYER + k.0,
                        (k.1 >> 32) as i32,
                        k.1 as u32 as i32,
                    )
                } else {
                    cell_of(p.pass, layer, p.lo, p.hi)
                };
                if !cells.contains(&c) {
                    cells.push(c);
                }
                dirty.insert(c);
                incoming.entry(c).or_default().push((k, p));
            }
            if k.0 == ELEMENT {
                for b in &boxes {
                    for sq in squares(b.0, b.1) {
                        let v = self.grid.entry(sq).or_default();
                        if !v.contains(&k) {
                            v.push(k);
                        }
                    }
                }
            }
            self.units.insert(
                k,
                Kept {
                    class,
                    boxes,
                    cells,
                    placed: m.placed,
                    lamps: m.lamps,
                    context: m.context,
                    owner: m.owner.unwrap_or(if k.0 == ELEMENT { k.1 } else { 0 }),
                    _keep: m.keep,
                },
            );
        }
        // Each cell touched: what stays copied from its chunks as it was,
        // then what was made.
        for c in dirty {
            let old = self.cells.remove(&c).unwrap_or_default();
            let mut packer = Packer::new(&self.selection);
            for chunk in &old {
                for part in chunk.parts.iter() {
                    if gone.contains(&part.unit) {
                        continue;
                    }
                    let [v0, nv] = part.vertices;
                    let [i0, ni] = part.indices;
                    let owner = chunk.owners[(i0 / 3) as usize];
                    packer.add(
                        part.unit,
                        owner,
                        &chunk.mesh.vertices[v0 as usize..(v0 + nv) as usize],
                        &chunk.mesh.indices[i0 as usize..(i0 + ni) as usize],
                        v0,
                    );
                }
            }
            for (unit, piece) in incoming.remove(&c).unwrap_or_default() {
                packer.add(
                    unit,
                    piece.owner,
                    &piece.mesh.vertices,
                    &piece.mesh.indices,
                    0,
                );
            }
            let chunks = packer.finish();
            if !chunks.is_empty() {
                self.cells.insert(c, chunks);
            }
        }
    }

    /// The scene: every cell's chunks and every placed model by model.
    fn scene(&self) -> Scene {
        let _span = archisim_geom::meter::span("escena: juntar");
        let mut s = Scene {
            lo: Vec3::splat(f32::INFINITY),
            hi: Vec3::splat(f32::NEG_INFINITY),
            orthophoto: self.model.orthophoto().cloned(),
            ..Default::default()
        };
        for (c, chunks) in &self.cells {
            let list = match c.0 {
                0 => &mut s.opaque,
                1 => &mut s.glass,
                _ => &mut s.grass,
            };
            for ch in chunks {
                s.lo = s.lo.min(ch.lo);
                s.hi = s.hi.max(ch.hi);
                list.push(ch.clone());
            }
        }
        let mut by: HashMap<u64, usize> = HashMap::new();
        for kept in self.units.values() {
            s.lamps.extend(&kept.lamps);
            for p in &kept.placed {
                let g = *by.entry(p.model.id).or_insert_with(|| {
                    s.groups.push(Group::of(p.model.clone()));
                    s.groups.len() - 1
                });
                let g = &mut s.groups[g];
                let mut inst = p.instance;
                if inst.flags[0] >= 0.0 {
                    inst.flags[0] = if self.selection.contains(&p.owner) {
                        1.0
                    } else {
                        0.0
                    };
                }
                g.instances.push(inst);
                g.owners.push(p.owner);
                g.bounds.push(p.bounds);
                s.lo = s.lo.min(p.bounds.0);
                s.hi = s.hi.max(p.bounds.1);
            }
        }
        if s.lo.x > s.hi.x {
            s.lo = Vec3::new(0.0, 0.0, 0.0);
            s.hi = Vec3::new(10.0, 10.0, 3.0);
        }
        s
    }
}

/// The boxes of what a unit made.
fn boxes_of(m: &Made) -> Vec<(Vec3, Vec3)> {
    m.pieces
        .iter()
        .map(|p| (p.lo, p.hi))
        .chain(m.placed.iter().map(|p| p.bounds))
        .collect()
}

/// The scene made whole, not kept: for an isolated clash (the selection
/// is drawn and the rest is a ghost) or with linked models.
pub(super) fn build_once(
    model: &Model,
    selection: &BTreeSet<Element>,
    linked: &[LinkedPart],
    isolate: bool,
    disciplines: &Disciplines,
) -> Scene {
    let crowd = archisim_geom::env_buildings::light_dress(model);
    let mut st = State {
        model: Arc::new(Model::new("")),
        setting: Setting::of(model, disciplines),
        units: BTreeMap::new(),
        grid: HashMap::new(),
        cells: BTreeMap::new(),
        anchors: HashMap::new(),
        models: HashMap::new(),
        near_plants: near_plants(model, disciplines),
        detailed_masses: detailed_masses(model, crowd),
        selection: selection.clone(),
    };
    let made = {
        let mut making = Making {
            model,
            disciplines,
            selection: if isolate { selection } else { &BTreeSet::new() },
            isolate,
            anchors: &mut st.anchors,
            models: &mut st.models,
            near_plants: &st.near_plants,
            detailed_masses: &st.detailed_masses,
            crowd,
            existing: &HashSet::new(),
            still: Vec::new(),
        };
        let whole: BTreeSet<u8> = [
            CLADDING, FITTINGS, FRAMES, ORTHOPHOTO, GROWTH, PAINT, FACE_ZONES,
        ]
        .into_iter()
        .collect();
        making.make(None, &whole, linked)
    };
    st.install(model, made, &BTreeSet::new(), &BTreeSet::new());
    st.scene()
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{
        NewTerrain, NewWorldBuilding, Point2, RoadStretch, SurveyPoint, Vec2, WorldPick,
        WorldTypology,
    };

    /// A small town: rolling ground, a crossing of two streets and a side
    /// street, a few buildings of the world, the street's life, and a
    /// house of four walls with a door and a floor.
    pub(crate) fn town() -> Model {
        let (mut m, pb, _) = crate::pick::tests::room();
        let mat = |m: &Model, c: &str| m.materials().find(|x| x.category == c).map(|x| x.id);
        let earth = mat(&m, "earth").unwrap_or_else(|| {
            m.add_material(
                "Terreno",
                "earth",
                [150, 160, 110],
                archisim_core::CutPattern::Dots,
            )
        });
        let asphalt = mat(&m, "asphalt").or_else(|| mat(&m, "concrete")).unwrap();
        let mut points = Vec::new();
        for i in -8..=8 {
            for j in -8..=8 {
                let at = Point2::new(i as f64 * 20.0, j as f64 * 20.0);
                let z = 0.01 * at.y + 0.5 * (at.x / 30.0).sin();
                points.push(SurveyPoint { at, z });
            }
        }
        m.add_terrain(NewTerrain {
            level: pb,
            points,
            material: earth,
            depth: 1.0,
            contour: 1.0,
            lines: Vec::new(),
        })
        .unwrap();
        let section = archisim_core::road_presets()
            .into_iter()
            .find(|(n, _)| *n == "Calle de doble sentido con aparcamiento")
            .map(|(_, s)| s)
            .unwrap();
        let lay = |m: &mut Model, a: Point2, b: Point2| {
            m.add_road_stretch(RoadStretch {
                level: pb,
                points: vec![a, b],
                continues: None,
                section: section.clone(),
                top_offset: 0.0,
                pavement: asphalt,
                kerb_material: asphalt,
                sidewalk_material: asphalt,
                follow_ground: Some(0.08),
            })
            .unwrap();
        };
        lay(
            &mut m,
            Point2::new(-150.0, -40.0),
            Point2::new(150.0, -40.0),
        );
        lay(
            &mut m,
            Point2::new(-60.0, -150.0),
            Point2::new(-60.0, 150.0),
        );
        lay(&mut m, Point2::new(60.0, -40.0), Point2::new(60.0, 60.0));
        for (k, x) in [-110.0, -20.0, 20.0, 100.0].into_iter().enumerate() {
            m.add_world_building(NewWorldBuilding {
                level: pb,
                front: Point2::new(x, -52.0),
                facing: Vec2::new(0.0, 1.0),
                width: 18.0,
                depth: 12.0,
                floors: None,
                pick: WorldPick {
                    place: 0,
                    typology: [WorldTypology::Street, WorldTypology::Block][k % 2],
                    programme: 0,
                },
            })
            .unwrap();
        }
        archisim_library::street_life::add_street_life(
            &mut m,
            pb,
            &archisim_library::street_life::LifeOptions::default(),
        )
        .unwrap();
        m
    }

    /// Everything a scene draws, in an order that does not depend on how
    /// it was made: each triangle with its pass and owner, each placed
    /// model by its triangles with where it stands, and the lamps.
    fn contents(s: &Scene) -> (Vec<String>, Vec<String>, Vec<String>) {
        let mut tris = Vec::new();
        for (pass, chunks) in [(0, &s.opaque), (1, &s.glass), (2, &s.grass)] {
            for c in chunks.iter() {
                for (t, o) in c
                    .mesh
                    .indices
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .zip(c.owners.iter())
                {
                    let v: Vec<[u32; 12]> = t
                        .iter()
                        .map(|i| c.mesh.vertices[*i as usize].key())
                        .collect();
                    tris.push(format!("{pass} {o:?} {v:?}"));
                }
            }
        }
        tris.sort();
        let mut placed = Vec::new();
        for g in &s.groups {
            let mut h = super::super::FastHash::new(1);
            for v in g.opaque.vertices.iter().chain(&g.glass.vertices) {
                for x in v.key() {
                    h.add(u64::from(x));
                }
            }
            for i in g.opaque.indices.iter().chain(&g.glass.indices) {
                h.add(u64::from(*i));
            }
            let model = format!("{:x} {:?} {}", h.finish(), g.levels, g.exact_shadow);
            for (inst, o) in g.instances.iter().zip(&g.owners) {
                placed.push(format!("{model} {o:?} {:?}", bytemuck::bytes_of(inst)));
            }
        }
        placed.sort();
        let mut lamps: Vec<String> = s.lamps.iter().map(|l| format!("{l:?}")).collect();
        lamps.sort();
        (tris, placed, lamps)
    }

    /// The scene kept and made again a piece at a time is the scene made
    /// whole, change after change.
    #[test]
    fn a_kept_scene_is_the_whole_one() {
        let mut m = town();
        let none = Disciplines::default();
        let mut builder = SceneBuilder::default();
        let check = |b: &mut SceneBuilder, m: &Model, selection: &BTreeSet<Element>, what: &str| {
            let (kept, stats) = b.build(Arc::new(m.clone()), selection, &none);
            let fresh = build_once(m, selection, &[], false, &none);
            let (a, b2) = (contents(&kept), contents(&fresh));
            assert_eq!(a.0.len(), b2.0.len(), "{what}: triángulos");
            assert!(a.0 == b2.0, "{what}: triángulos distintos");
            assert!(a.1 == b2.1, "{what}: modelos colocados distintos");
            assert!(a.2 == b2.2, "{what}: lámparas distintas");
            stats
        };
        let none_selected = BTreeSet::new();
        let first = check(&mut builder, &m, &none_selected, "primera vez");
        assert!(first.whole);
        let again = check(&mut builder, &m, &none_selected, "sin cambios");
        assert!(!again.whole);
        assert_eq!(again.units, 0, "sin cambios no se rehace nada");
        // A street moved: it, its junctions, the ground and what faces it.
        let side = m.roads().last().unwrap().id;
        m.edit_road(side, |r| {
            for p in &mut r.axis {
                p.x += 1.5;
            }
        })
        .unwrap();
        let moved = check(&mut builder, &m, &none_selected, "calle movida");
        assert!(!moved.whole && moved.units < m.objects().count() / 2);
        // A tree and a bench moved, a car taken away.
        let tree = m
            .objects()
            .find(|o| m.type_grows(o.object_type))
            .unwrap()
            .id;
        m.edit_object(tree, |o| o.position.x += 2.0).unwrap();
        check(&mut builder, &m, &none_selected, "árbol movido");
        let bench = m
            .objects()
            .find(|o| !m.type_grows(o.object_type))
            .unwrap()
            .id;
        m.edit_object(bench, |o| o.rotation += 0.5).unwrap();
        let last = m.objects().last().unwrap().id;
        m.remove_object(last).unwrap();
        check(&mut builder, &m, &none_selected, "objetos");
        // A wall of the house moved: its neighbours, floor and door.
        let w = m.walls().next().unwrap();
        let (a, b) = (w.start, w.end);
        let id = w.id;
        m.move_wall(id, a, Point2::new(b.x - 0.5, b.y)).unwrap();
        check(&mut builder, &m, &none_selected, "muro movido");
        // A building of the surroundings moved.
        let mass = m.masses().find(|x| x.dress.is_some()).unwrap().id;
        m.edit_mass(mass, |x| {
            if let archisim_core::MassShape::Prism(p) = &mut x.shape {
                for q in p.iter_mut() {
                    q.y -= 1.0;
                }
            }
        })
        .unwrap();
        check(&mut builder, &m, &none_selected, "edificio movido");
        // A node raised, a stretch paved apart and a piece hidden: only
        // round them.
        if let Some(key) = archisim_geom::crossroads::all_node_keys(&m)
            .into_iter()
            .next()
        {
            let mut node = archisim_core::NodeSettings::new(key);
            node.kind = Some(archisim_core::NodeKind::Raised);
            m.set_node(node).unwrap();
            check(&mut builder, &m, &none_selected, "nudo elevado");
        }
        let (road, len) = m
            .roads()
            .map(|r| (r.id, r.length()))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        let pav = m.materials().last().unwrap().id;
        m.set_road_skins(
            road,
            vec![archisim_core::RoadSkin {
                from: len * 0.2,
                to: len * 0.4,
                pavement: Some(pav),
                kerb: None,
                sidewalk: Some(pav),
                planting: None,
            }],
        )
        .unwrap();
        check(&mut builder, &m, &none_selected, "tramo pavimentado");
        let mark = m
            .road(road)
            .and_then(|r| r.visible_markings(&[]).into_iter().nth(2))
            .map(|x| archisim_core::centroid(&x));
        if let Some(p) = mark {
            m.hide_piece(road, archisim_core::PieceKind::Marking, p)
                .unwrap();
            check(&mut builder, &m, &none_selected, "marca oculta");
        }
        // Selecting.
        let selected = BTreeSet::from([Element::Wall(id), Element::Object(bench)]);
        check(&mut builder, &m, &selected, "selección");
        // A street taken away.
        m.remove_road(side).unwrap();
        check(&mut builder, &m, &selected, "calle quitada");
        check(&mut builder, &m, &none_selected, "sin selección");
    }
}
