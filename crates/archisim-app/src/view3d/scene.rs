//! The model as triangles ready for the graphics card, and picking in 3D.
//!
//! Two kinds of triangles: the elements drawn once (walls, slabs, roofs,
//! the terrain…), in chunks of shared corners (`Chunk`: each corner is
//! stored once and the triangles say which three they join), and the
//! furniture ArchiSim makes, whose model is the same for every object of
//! a type: each such model is sent to the card once (`Group`) and drawn
//! wherever an object of it stands (`Instance`), nearer or farther with
//! fewer triangles (`Group::levels`), and not at all when it is out of
//! sight (the renderer decides, each picture).

use std::collections::{BTreeSet, HashMap};

use archisim_core::Model;
use archisim_geom::solid::SolidOwner;
use bytemuck::{Pod, Zeroable};
use glam::Vec3;

use super::camera::Ray;
use super::cutaway::{self, CutRule};
use crate::pick::Element;

/// One corner of a triangle as the shader reads it: 48 bytes (it was 72
/// before the corners were shared and packed).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    /// The normal, each part times 32767; the fourth holds the red and
    /// green of the cut colour (see [`Vertex::with_cap`]).
    pub normal: [i16; 4],
    /// sRGB colour and opacity, 0…255.
    pub color: [u8; 4],
    /// Which face wins where two elements touch on a visible edge, plus
    /// three (faces looking up give way, 4; doors and windows win against
    /// the reveals they sit in, 2; foliage 5, what glows 6, grass 7, many
    /// small leaves 8; a face between two layers, seen only as a cut, 1;
    /// the filled section of a cut object, 0); 1 if its element is
    /// selected (2 drawn stretched over the scene); the texture (0 none);
    /// the blue of the cut colour.
    pub misc: [u8; 4],
    /// The size of the texture's piece, metres.
    pub tex_size: f32,
    /// What the cut away walls do with its element (`cutaway`): its kind,
    /// a point of it in plan and its base.
    pub cut: [f32; 4],
}

/// What the bias of a corner is stored above (it may be as low as -3).
const BIAS_ZERO: f32 = 3.0;

impl Vertex {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pos: Vec3,
        normal: Vec3,
        color: [f32; 4],
        bias: f32,
        tex: [f32; 2],
        cut: [f32; 4],
        select: f32,
    ) -> Self {
        let n = |c: f32| (c.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        let c = |c: f32| (linear_to_srgb(c) * 255.0).round() as u8;
        Vertex {
            pos: pos.to_array(),
            normal: [n(normal.x), n(normal.y), n(normal.z), 0],
            color: [
                c(color[0]),
                c(color[1]),
                c(color[2]),
                (color[3].clamp(0.0, 1.0) * 255.0).round() as u8,
            ],
            misc: [
                (bias + BIAS_ZERO).round().clamp(0.0, 255.0) as u8,
                select.round().clamp(0.0, 255.0) as u8,
                tex[0].round().clamp(0.0, 255.0) as u8,
                0,
            ],
            tex_size: tex[1],
            cut,
        }
        .with_cap([color[0], color[1], color[2]])
    }

    /// With `cap` (linear RGB) as what its cut shows when a section box or
    /// the cut away walls cut it: its material's cut colour, the plan's
    /// poché (each layer of a wall its own). Kept in the corner's spare
    /// bytes, sRGB: red and green in the normal's fourth part, blue in the
    /// last of `misc`.
    pub fn with_cap(mut self, cap: [f32; 3]) -> Self {
        let c = |c: f32| (linear_to_srgb(c) * 255.0).round() as u8;
        self.normal[3] = i16::from_le_bytes([c(cap[0]), c(cap[1])]);
        self.misc[3] = c(cap[2]);
        self
    }

    /// Its cut colour, linear RGB.
    pub fn cap(&self) -> [f32; 3] {
        let [r, g] = self.normal[3].to_le_bytes();
        [
            srgb_to_linear(r),
            srgb_to_linear(g),
            srgb_to_linear(self.misc[3]),
        ]
    }

    /// Which face wins where two elements touch, as given to [`Vertex::new`].
    pub fn bias(&self) -> f32 {
        self.misc[0] as f32 - BIAS_ZERO
    }

    /// 1 if its element is selected, 2 if drawn stretched over the scene.
    #[cfg(test)]
    pub fn select(&self) -> f32 {
        self.misc[1] as f32
    }

    /// 1 if its element is selected, 2 if drawn stretched over the scene.
    fn select_value(&self) -> f32 {
        self.misc[1] as f32
    }

    fn key(&self) -> [u32; 12] {
        bytemuck::cast(*self)
    }
}

/// Where an object stands, as the shader reads it (one per object drawn
/// from a group; the elements drawn once have one that changes nothing).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Instance {
    /// cos and sin of its turn about the vertical, then where it goes in
    /// plan.
    pub place: [f32; 4],
    /// Its height, then its scale along x, y and z.
    pub lift: [f32; 4],
    /// What the cut away walls do with it; x below zero: the corners'
    /// own.
    pub cut: [f32; 4],
    /// 1 if selected (below zero: the corners' own), 1 if greyed by the
    /// disciplines, unused, unused.
    pub flags: [f32; 4],
}

impl Instance {
    /// The one of the elements drawn once: nothing moves, the corners say
    /// the rest.
    pub const SAME: Instance = Instance {
        place: [1.0, 0.0, 0.0, 0.0],
        lift: [0.0, 1.0, 1.0, 1.0],
        cut: [-1.0, 0.0, 0.0, 0.0],
        flags: [-1.0, 0.0, 0.0, 0.0],
    };

    /// A point of the model in the object's own axes, where it is.
    pub fn place(&self, p: Vec3) -> Vec3 {
        let [c, s, x, y] = self.place;
        let q = p * Vec3::new(self.lift[1], self.lift[2], self.lift[3]);
        Vec3::new(
            c * q.x - s * q.y + x,
            s * q.x + c * q.y + y,
            q.z + self.lift[0],
        )
    }

    /// A point of the world in the object's own axes.
    pub fn unplace(&self, p: Vec3) -> Vec3 {
        let [c, s, x, y] = self.place;
        let (dx, dy) = (p.x - x, p.y - y);
        let q = Vec3::new(c * dx + s * dy, -s * dx + c * dy, p.z - self.lift[0]);
        q / Vec3::new(self.lift[1], self.lift[2], self.lift[3])
    }

    /// A direction of the world in the object's own axes (not scaled).
    fn unturn(&self, d: Vec3) -> Vec3 {
        let [c, s, _, _] = self.place;
        let q = Vec3::new(c * d.x + s * d.y, -s * d.x + c * d.y, d.z);
        q / Vec3::new(self.lift[1], self.lift[2], self.lift[3])
    }
}

/// Triangles with shared corners.
#[derive(Debug, Clone, Default)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    #[cfg(test)]
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Its triangles (of the first `count` indices) as points.
    fn triangles(&self) -> impl Iterator<Item = [Vec3; 3]> + '_ {
        self.indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| t.map(|i| Vec3::from_array(self.vertices[i as usize].pos)))
    }
}

/// Builds a mesh, sharing equal corners.
#[derive(Default)]
struct MeshBuilder {
    mesh: Mesh,
    seen: HashMap<[u32; 12], u32, CornerHash>,
}

/// A quick hash for the corners of a mesh (the default one, made to resist
/// chosen keys, took most of the time of making a scene).
#[derive(Default, Clone, Copy)]
struct CornerHash;

impl std::hash::BuildHasher for CornerHash {
    type Hasher = CornerHasher;
    fn build_hasher(&self) -> CornerHasher {
        CornerHasher(0)
    }
}

struct CornerHasher(u64);

impl std::hash::Hasher for CornerHasher {
    fn finish(&self) -> u64 {
        let mut x = self.0;
        x ^= x >> 33;
        x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
        x ^= x >> 33;
        x
    }

    fn write(&mut self, bytes: &[u8]) {
        for c in bytes.chunks(8) {
            let mut b = [0u8; 8];
            b[..c.len()].copy_from_slice(c);
            self.write_u64(u64::from_le_bytes(b));
        }
    }

    fn write_u32(&mut self, v: u32) {
        self.write_u64(u64::from(v));
    }

    fn write_u64(&mut self, v: u64) {
        self.0 = (self.0.rotate_left(5) ^ v).wrapping_mul(0x517c_c1b7_2722_0a95);
    }

    fn write_usize(&mut self, v: usize) {
        self.write_u64(v as u64);
    }
}

impl MeshBuilder {
    fn push(&mut self, tri: [Vertex; 3]) {
        for v in tri {
            let i = *self.seen.entry(v.key()).or_insert_with(|| {
                self.mesh.vertices.push(v);
                (self.mesh.vertices.len() - 1) as u32
            });
            self.mesh.indices.push(i);
        }
    }
}

/// Elements drawn once: up to [`CHUNK`] triangles with shared corners,
/// the element of each and the box round them all (out of sight, the
/// renderer skips the chunk).
#[derive(Debug, Clone, Default)]
pub struct Chunk {
    pub mesh: std::sync::Arc<Mesh>,
    pub owners: std::sync::Arc<Vec<Owner>>,
    pub lo: Vec3,
    pub hi: Vec3,
    /// Its number: the same while it does not change (the graphics card
    /// keeps it meanwhile).
    pub key: u64,
    /// Where each unit's corners and triangles are in it.
    parts: std::sync::Arc<Vec<Part>>,
}

/// The corners and triangles of one unit (and one owner) in a chunk:
/// first and count of each.
#[derive(Debug, Clone, Copy)]
struct Part {
    unit: scene_build::UnitKey,
    vertices: [u32; 2],
    indices: [u32; 2],
}

#[path = "scene_build.rs"]
mod scene_build;
pub use scene_build::SceneBuilder;

impl Chunk {
    /// Its corners, to change them (a copy if shared), and a new number.
    pub fn mesh_mut(&mut self) -> &mut Mesh {
        self.key = scene_build::next_key();
        std::sync::Arc::make_mut(&mut self.mesh)
    }
}

/// With a whole neighbourhood of buildings of the surroundings, those
/// within this distance of the project's origin keep their detail.
const DETAIL_AROUND: f64 = 150.0;
/// And no more than these (a town brought in round the project would put
/// hundreds within that distance: each with all its detail is some
/// thirty thousand corners, and they did not fit in memory).
const DETAIL_BUILDINGS: usize = 40;

/// How many plants, nearest the project's origin and no farther than
/// [`NEAR_PLANTS_AROUND`], keep every leaf when there are too many for all
/// of them to.
const NEAR_PLANTS: usize = 12;
const NEAR_PLANTS_AROUND: f64 = 60.0;

/// Triangles in a chunk: small enough to leave out what is not seen,
/// large enough not to draw too many pieces.
const CHUNK: usize = 32_768;

/// A model shared by several objects, and where each stands.
#[derive(Debug, Clone, Default)]
pub struct Group {
    /// Its model's number: the same while the model is the same (the
    /// graphics card keeps it meanwhile).
    pub key: u64,
    /// Its opaque triangles: all of them (the first `levels[0]` indices),
    /// then each coarser version for farther away.
    pub opaque: std::sync::Arc<Mesh>,
    /// The first index and the count of each version, finest first.
    pub levels: Vec<(u32, u32)>,
    pub glass: std::sync::Arc<Mesh>,
    pub instances: Vec<Instance>,
    /// The element of each object.
    pub owners: Vec<Element>,
    /// The box of each object, where it stands.
    pub bounds: Vec<(Vec3, Vec3)>,
    /// The model's box in its own axes.
    pub lo: Vec3,
    pub hi: Vec3,
    /// Casts its shadow with its finest version, not a coarser one: a door
    /// sits so close to its wall that a coarser version would shade it.
    pub exact_shadow: bool,
}

impl Group {
    /// A group of a model, with no object yet.
    fn of(m: std::sync::Arc<scene_build::GroupModel>) -> Group {
        Group {
            key: m.id,
            opaque: m.opaque.clone(),
            levels: m.levels.clone(),
            glass: m.glass.clone(),
            lo: m.lo,
            hi: m.hi,
            exact_shadow: m.exact_shadow,
            ..Default::default()
        }
    }

    /// The finest version's triangles, then the glass, in its own axes.
    fn local_triangles(&self) -> impl Iterator<Item = [Vec3; 3]> + '_ {
        let count = self.levels.first().map_or(0, |l| l.1) as usize;
        self.opaque.indices[..count]
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| t.map(|i| Vec3::from_array(self.opaque.vertices[i as usize].pos)))
            .chain(self.glass.triangles())
    }

    /// How big it is (half its box's diagonal), for choosing its version.
    pub fn radius(&self) -> f32 {
        (self.hi - self.lo).length() / 2.0
    }
}

/// What the disciplines change in the 3D view: what is greyed and what
/// is left out.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Disciplines {
    pub focus: archisim_core::DisciplineFocus,
    pub hidden: std::collections::BTreeSet<archisim_core::ElementDiscipline>,
}

/// How see-through glass is (as in the IFC export).
const GLASS_ALPHA: f32 = 0.4;
/// Water lets the floor of a pool be seen through it, a little clouded.
const WATER_ALPHA: f32 = 0.55;
/// What is left of the rest of the model around an isolated clash.
pub const GHOST_ALPHA: f32 = 0.12;

/// What a triangle belongs to: an element of the project, or an element
/// of a linked model (link, element index).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Owner {
    Model(Element),
    Linked(usize, usize),
}

/// Triangles of a linked element to add to the scene.
pub struct LinkedPart<'a> {
    pub triangles: &'a [[[f64; 3]; 3]],
    /// sRGB.
    pub color: [u8; 3],
    pub alpha: f32,
    pub link: usize,
    pub index: usize,
}

/// The model ready to draw: the elements drawn once, opaque, see-through
/// and grass, and the groups of furniture.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub opaque: Vec<Chunk>,
    pub glass: Vec<Chunk>,
    /// The grass and flowers on painted ground: drawn after the opaque
    /// model without changing its normals and distances (no contour lines
    /// between the blades), not picked.
    pub grass: Vec<Chunk>,
    pub groups: Vec<Group>,
    pub lo: Vec3,
    pub hi: Vec3,
    /// The lamps: where their light comes from (the middle of what glows)
    /// and how far it reaches, metres.
    pub lamps: Vec<[f32; 4]>,
    /// The orthophoto laid on the ground: its picture drawn pixel by
    /// pixel on the faces marked for it (`ORTHO_TEX`), not only the
    /// colours of its grid's corners.
    pub orthophoto: Option<archisim_core::Orthophoto>,
}

/// The size of the texture's piece that marks the orthophoto's corners:
/// the shader looks the picture up on them.
pub const ORTHO_TEX: f32 = -1.0;

/// Where a triangle goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pass {
    Opaque,
    Glass,
    Grass,
}

/// Where the triangles of the solids go: the chunks of the elements drawn
/// once, or a group's model.
trait Sink {
    fn push(&mut self, pass: Pass, tri: [Vertex; 3], owner: Owner);
    /// What glows: its corners, for the lamp's light.
    fn glow(&mut self, id: u64, p: Vec3, reach: f32);
}

/// The elements drawn once, as they are made.
/// A chunk as it is filled.
#[derive(Default)]
struct RawChunk {
    mesh: Mesh,
    owners: Vec<Owner>,
    lo: Vec3,
    hi: Vec3,
}

#[derive(Default)]
struct Loose {
    passes: [(Vec<RawChunk>, MeshBuilder); 3],
    /// Middle and reach of what glows, per element.
    lit: std::collections::BTreeMap<u64, (Vec3, f32, f32)>,
}

impl Loose {
    fn finish(mut self) -> ([Vec<Chunk>; 3], Vec<[f32; 4]>) {
        let mut out: [Vec<Chunk>; 3] = Default::default();
        for (k, (mut chunks, builder)) in std::mem::take(&mut self.passes).into_iter().enumerate() {
            if let Some(c) = chunks.last_mut() {
                c.mesh = builder.mesh;
            }
            out[k] = chunks
                .into_iter()
                .filter(|c| !c.mesh.is_empty())
                .map(|c| Chunk {
                    mesh: std::sync::Arc::new(c.mesh),
                    owners: std::sync::Arc::new(c.owners),
                    lo: c.lo,
                    hi: c.hi,
                    key: scene_build::next_key(),
                    parts: Default::default(),
                })
                .collect();
        }
        let lamps = self
            .lit
            .into_values()
            .filter(|(_, n, _)| *n > 0.0)
            .map(|(sum, n, reach)| {
                let c = sum / n;
                [c.x, c.y, c.z, reach]
            })
            .collect();
        (out, lamps)
    }
}

impl Sink for Loose {
    fn push(&mut self, pass: Pass, tri: [Vertex; 3], owner: Owner) {
        let (chunks, builder) = &mut self.passes[pass as usize];
        let full = chunks.last().is_none_or(|c| c.owners.len() >= CHUNK);
        if full {
            if let Some(c) = chunks.last_mut() {
                c.mesh = std::mem::take(&mut builder.mesh);
            }
            builder.seen.clear();
            chunks.push(RawChunk {
                lo: Vec3::splat(f32::INFINITY),
                hi: Vec3::splat(f32::NEG_INFINITY),
                ..Default::default()
            });
        }
        let c = chunks.last_mut().unwrap();
        for v in &tri {
            let p = Vec3::from_array(v.pos);
            c.lo = c.lo.min(p);
            c.hi = c.hi.max(p);
        }
        c.owners.push(owner);
        builder.push(tri);
    }

    fn glow(&mut self, id: u64, p: Vec3, reach: f32) {
        let (sum, n, r) = self.lit.entry(id).or_insert((Vec3::ZERO, 0.0, reach));
        *r = reach;
        *sum += p;
        *n += 1.0;
    }
}

/// A group's model, as it is made in its own axes.
#[derive(Default)]
struct Shared {
    opaque: MeshBuilder,
    glass: MeshBuilder,
    /// Its own plain version for far away, instead of a coarser one
    /// worked out from it (doors and windows: clustering would close up
    /// their glass).
    simple: Option<Mesh>,
    /// Middle of what glows and its reach.
    glow: Option<(Vec3, f32, f32)>,
}

impl Shared {
    /// Its model is made: the corners found are no longer needed (with a
    /// whole neighbourhood of buildings their table weighs more than the
    /// model itself).
    fn done(mut self) -> Self {
        for b in [&mut self.opaque, &mut self.glass] {
            b.seen = HashMap::default();
            b.mesh.vertices.shrink_to_fit();
            b.mesh.indices.shrink_to_fit();
        }
        self
    }
}

impl Sink for Shared {
    fn push(&mut self, pass: Pass, tri: [Vertex; 3], _owner: Owner) {
        match pass {
            Pass::Glass => self.glass.push(tri),
            _ => self.opaque.push(tri),
        }
    }

    fn glow(&mut self, _id: u64, p: Vec3, reach: f32) {
        let g = self.glow.get_or_insert((Vec3::ZERO, 0.0, reach));
        g.0 += p;
        g.1 += 1.0;
    }
}

/// A corner as a key, to the tenth of a millimetre.
fn vertex_key(v: &[f64; 3]) -> [i64; 3] {
    v.map(|c| (c * 10_000.0).round() as i64)
}

pub(crate) fn srgb_to_linear(c: u8) -> f32 {
    let c = c as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

fn element(owner: SolidOwner) -> Element {
    crate::pick::of_solid(owner)
}

/// The coarser versions of a model, for farther away: corners closer than
/// `cell` of the same colour and texture become one, and the triangles
/// that shrink to nothing go (clustering, as `archisim_mesh::simplify`).
fn coarser(mesh: &Mesh, count: usize, cell: f32) -> Vec<u32> {
    let mut rep: HashMap<([i32; 3], [u8; 4], u8), u32> = HashMap::new();
    let map: Vec<u32> = mesh
        .vertices
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let p = Vec3::from_array(v.pos) / cell;
            let k = (
                [p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32],
                v.color,
                v.misc[2],
            );
            *rep.entry(k).or_insert(i as u32)
        })
        .collect();
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for t in mesh.indices[..count].as_chunks::<3>().0 {
        let t = t.map(|i| map[i as usize]);
        if t[0] == t[1] || t[1] == t[2] || t[0] == t[2] {
            continue;
        }
        let mut k = t;
        k.sort_unstable();
        if seen.insert(k) {
            out.extend(t);
        }
    }
    out
}

/// The versions of a group's model: the whole of it and, if it has many
/// triangles, two coarser ones that are worth it.
fn levels_of(mesh: &mut Mesh, size: f32) -> Vec<(u32, u32)> {
    let full = mesh.indices.len();
    let mut levels = vec![(0, full as u32)];
    if full / 3 < 1_500 {
        return levels;
    }
    let mut last = full;
    for k in [60.0, 18.0] {
        let idx = coarser(mesh, full, size / k);
        // Only if it saves a good part of what the previous one has.
        if idx.len() * 10 < last * 7 && !idx.is_empty() {
            let first = mesh.indices.len() as u32;
            last = idx.len();
            levels.push((first, idx.len() as u32));
            mesh.indices.extend(idx);
        }
    }
    levels
}

/// What each solid needs to know of how it is drawn.
struct Look<'a> {
    selection: &'a BTreeSet<Element>,
    isolate: bool,
    disciplines: &'a Disciplines,
    anchors: &'a HashMap<archisim_core::WallId, Anchor>,
    /// For a group's model: no selection, no greying, no cut of its own
    /// (the instance has them).
    shared: bool,
    /// Faces between two layers of a wall, a slab or a roof: drawn only
    /// as a cut, so that a cut wall shows each layer solid in its cut
    /// colour.
    between: bool,
}

impl Scene {
    /// Whether some water is seen through (a pool's, the sea): it moves,
    /// so the picture is drawn again while it is in view.
    pub fn has_water(&self) -> bool {
        let water = |v: &Vertex| matches!(v.misc[2], 10 | 30);
        self.glass.iter().any(|c| c.mesh.vertices.iter().any(water))
            || self
                .groups
                .iter()
                .any(|g| g.glass.vertices.iter().any(water))
    }

    pub fn build(model: &Model, selection: &BTreeSet<Element>) -> Self {
        Self::build_with(model, selection, &[], false, &Default::default())
    }

    /// The model and, after it, the elements of linked models. With
    /// `isolate`, only the selection keeps its colour: the rest of the
    /// project is a faint ghost, to see a clash through walls and roofs.
    pub fn build_with(
        model: &Model,
        selection: &BTreeSet<Element>,
        linked: &[LinkedPart],
        isolate: bool,
        disciplines: &Disciplines,
    ) -> Self {
        // In the browser, short of memory (`docs/IPAD.md` §12), the plants
        // in their medium version, all of them.
        use archisim_core::flora::Lod;
        if crate::platform::WEB && model.plant_detail() == Lod::Full {
            return archisim_geom::solid::with_plant_lod(Lod::Medium, || {
                scene_build::build_once(model, selection, linked, isolate, disciplines)
            });
        }
        scene_build::build_once(model, selection, linked, isolate, disciplines)
    }

    /// A scene of a few solids only: what is being stretched, drawn over
    /// the scene while the handle is dragged (its element is marked
    /// selected, select 2, so the scene's own copy is hidden).
    pub fn of_solids(model: &Model, solids: Vec<archisim_geom::solid::Solid>) -> Self {
        let selection: BTreeSet<Element> = solids.iter().map(|x| element(x.owner)).collect();
        let look = Look {
            selection: &selection,
            isolate: false,
            disciplines: &Default::default(),
            anchors: &HashMap::new(),
            shared: false,
            between: false,
        };
        let mut loose = Loose::default();
        add_solids(model, solids, &look, &mut loose);
        let ([mut opaque, mut glass, grass], lamps) = loose.finish();
        let mut s = Scene {
            lo: Vec3::splat(f32::INFINITY),
            hi: Vec3::splat(f32::NEG_INFINITY),
            lamps,
            grass,
            ..Default::default()
        };
        for c in opaque.iter_mut().chain(&mut glass) {
            for v in &mut std::sync::Arc::make_mut(&mut c.mesh).vertices {
                v.misc[1] = 2;
            }
            s.lo = s.lo.min(c.lo);
            s.hi = s.hi.max(c.hi);
        }
        (s.opaque, s.glass) = (opaque, glass);
        s
    }

    /// Solids drawn over the scene as they are, not marked: what changes
    /// round what is being drawn (the streets a new one meets).
    pub fn of_solids_unmarked(model: &Model, solids: Vec<archisim_geom::solid::Solid>) -> Self {
        let selection = BTreeSet::new();
        let look = Look {
            selection: &selection,
            isolate: false,
            disciplines: &Default::default(),
            anchors: &HashMap::new(),
            shared: false,
            between: false,
        };
        let mut loose = Loose::default();
        add_solids(model, solids, &look, &mut loose);
        let ([opaque, glass, grass], lamps) = loose.finish();
        let mut s = Scene {
            lo: Vec3::splat(f32::INFINITY),
            hi: Vec3::splat(f32::NEG_INFINITY),
            lamps,
            grass,
            ..Default::default()
        };
        for c in opaque.iter().chain(&glass) {
            s.lo = s.lo.min(c.lo);
            s.hi = s.hi.max(c.hi);
        }
        (s.opaque, s.glass) = (opaque, glass);
        s
    }

    /// This scene with another's pieces added.
    pub fn joined(mut self, other: Scene) -> Self {
        self.lo = self.lo.min(other.lo);
        self.hi = self.hi.max(other.hi);
        self.opaque.extend(other.opaque);
        self.glass.extend(other.glass);
        self.grass.extend(other.grass);
        self.lamps.extend(other.lamps);
        self
    }

    /// The cars and people of the traffic for illustration (N9), drawn
    /// as they are, over the scene.
    pub fn of_traffic(model: &Model, solids: Vec<archisim_geom::solid::Solid>) -> Self {
        let selection = BTreeSet::new();
        let look = Look {
            selection: &selection,
            isolate: false,
            disciplines: &Default::default(),
            anchors: &HashMap::new(),
            shared: false,
            between: false,
        };
        let mut loose = Loose::default();
        add_solids(model, solids, &look, &mut loose);
        let ([opaque, glass, grass], lamps) = loose.finish();
        let mut s = Scene {
            lo: Vec3::splat(f32::INFINITY),
            hi: Vec3::splat(f32::NEG_INFINITY),
            lamps,
            grass,
            ..Default::default()
        };
        for c in opaque.iter().chain(&glass) {
            s.lo = s.lo.min(c.lo);
            s.hi = s.hi.max(c.hi);
        }
        (s.opaque, s.glass) = (opaque, glass);
        s
    }

    pub fn is_empty(&self) -> bool {
        self.opaque.is_empty() && self.glass.is_empty() && self.groups.is_empty()
    }

    /// Marks another selection without building the scene again: the
    /// corners of what was or is now selected, and the objects drawn from
    /// a group. (Building a big project's scene again took seconds, and
    /// it was done at every click.)
    pub fn reselect(&mut self, from: &BTreeSet<Element>, to: &BTreeSet<Element>) {
        let changed: BTreeSet<Element> = from.symmetric_difference(to).copied().collect();
        if changed.is_empty() {
            return;
        }
        for chunk in self.opaque.iter_mut().chain(self.glass.iter_mut()) {
            let touched = chunk
                .owners
                .iter()
                .any(|o| matches!(o, Owner::Model(e) if changed.contains(e)));
            if !touched {
                continue;
            }
            let owners = chunk.owners.clone();
            let mesh = chunk.mesh_mut();
            for (t, owner) in owners.iter().enumerate() {
                let Owner::Model(e) = owner else {
                    continue;
                };
                if !changed.contains(e) {
                    continue;
                }
                let on = u8::from(to.contains(e));
                for k in 0..3 {
                    if let Some(&i) = mesh.indices.get(3 * t + k) {
                        if let Some(v) = mesh.vertices.get_mut(i as usize) {
                            if v.misc[1] < 2 {
                                v.misc[1] = on;
                            }
                        }
                    }
                }
            }
        }
        for g in &mut self.groups {
            for (inst, e) in g.instances.iter_mut().zip(&g.owners) {
                if changed.contains(e) && inst.flags[0] >= 0.0 {
                    inst.flags[0] = if to.contains(e) { 1.0 } else { 0.0 };
                }
            }
        }
    }

    /// Whether anything in it is selected (to outline it).
    pub fn has_selection(&self) -> bool {
        self.opaque
            .iter()
            .any(|c| c.mesh.vertices.iter().any(|v| v.misc[1] > 0))
            || self
                .groups
                .iter()
                .any(|g| g.instances.iter().any(|i| i.flags[0] > 0.5))
    }

    /// Every triangle drawn, where it is (the furniture once per object).
    #[cfg(test)]
    pub fn triangle_count(&self) -> usize {
        self.opaque
            .iter()
            .chain(&self.glass)
            .map(|c| c.mesh.triangle_count())
            .sum::<usize>()
            + self
                .groups
                .iter()
                .map(|g| {
                    (g.levels.first().map_or(0, |l| l.1 as usize) / 3 + g.glass.triangle_count())
                        * g.instances.len()
                })
                .sum::<usize>()
    }

    /// Each triangle with its owner, where it is: the chunks, then each
    /// object of each group.
    fn each_triangle(&self, mut f: impl FnMut([Vec3; 3], Owner, [f32; 4]) -> bool) {
        for c in self.opaque.iter().chain(&self.glass) {
            for (t, o) in c
                .mesh
                .indices
                .as_chunks::<3>()
                .0
                .iter()
                .zip(c.owners.iter())
            {
                let v = t.map(|i| c.mesh.vertices[i as usize]);
                if !f(v.map(|v| Vec3::from_array(v.pos)), *o, v[0].cut) {
                    return;
                }
            }
        }
        for g in &self.groups {
            for (inst, e) in g.instances.iter().zip(&g.owners) {
                for t in g.local_triangles() {
                    if !f(t.map(|p| inst.place(p)), Owner::Model(*e), inst.cut) {
                        return;
                    }
                }
            }
        }
    }

    /// The cut faces of the objects a section box from `lo` to `hi` cuts
    /// (furniture, appliances, people, cars): on each face of the box that
    /// goes through one, its section filled whole, in the cut colour of
    /// the parts it was cut from, so that a kitchen unit reads as a solid
    /// block and not as the empty box of its panels. Leaves are left as
    /// they are (their back faces show the cut).
    pub fn object_cut_fill(&self, lo: Vec3, hi: Vec3) -> Vec<Chunk> {
        use archisim_geom::slice::{filled_section, Plane};
        struct Part {
            tris: Vec<([[f64; 3]; 3], [u32; 3])>,
            lo: Vec3,
            hi: Vec3,
            cut: [f32; 4],
            select: f32,
        }
        let mut parts: Vec<(Element, Part)> = Vec::new();
        let mut add = |e: Element, t: [Vec3; 3], cap: [f32; 3], cut: [f32; 4], select: f32| {
            let i = match parts.iter().position(|(o, _)| *o == e) {
                Some(i) => i,
                None => {
                    parts.push((
                        e,
                        Part {
                            tris: Vec::new(),
                            lo: Vec3::splat(f32::MAX),
                            hi: Vec3::splat(f32::MIN),
                            cut,
                            select,
                        },
                    ));
                    parts.len() - 1
                }
            };
            let part = &mut parts[i].1;
            for v in t {
                part.lo = part.lo.min(v);
                part.hi = part.hi.max(v);
            }
            part.tris.push((
                t.map(|v| v.to_array().map(f64::from)),
                cap.map(f32::to_bits),
            ));
        };
        // Leaves, what glows and the grass keep their own look.
        let plain = |v: &Vertex| v.bias() <= 0.5;
        for c in &self.opaque {
            for (t, o) in c
                .mesh
                .indices
                .as_chunks::<3>()
                .0
                .iter()
                .zip(c.owners.iter())
            {
                let Owner::Model(e @ Element::Object(_)) = *o else {
                    continue;
                };
                let v = t.map(|i| c.mesh.vertices[i as usize]);
                if plain(&v[0]) {
                    let p = v.map(|v| Vec3::from_array(v.pos));
                    add(e, p, v[0].cap(), v[0].cut, v[0].select_value());
                }
            }
        }
        for g in &self.groups {
            let count = g.levels.first().map_or(0, |l| l.1) as usize;
            for (inst, e) in g.instances.iter().zip(&g.owners) {
                if !matches!(e, Element::Object(_)) {
                    continue;
                }
                for t in g.opaque.indices[..count].as_chunks::<3>().0 {
                    let v = t.map(|i| g.opaque.vertices[i as usize]);
                    if plain(&v[0]) {
                        let p = v.map(|v| inst.place(Vec3::from_array(v.pos)));
                        add(*e, p, v[0].cap(), inst.cut, inst.flags[0].max(0.0));
                    }
                }
            }
        }
        // The box's faces, each with its directions along it so that
        // u × v points out of the box, towards what was cut away.
        let faces: [(usize, f32, Vec3, Vec3); 6] = [
            (0, lo.x, Vec3::Z, Vec3::Y),
            (0, hi.x, Vec3::Y, Vec3::Z),
            (1, lo.y, Vec3::X, Vec3::Z),
            (1, hi.y, Vec3::Z, Vec3::X),
            (2, lo.z, Vec3::Y, Vec3::X),
            (2, hi.z, Vec3::X, Vec3::Y),
        ];
        let mut out = Loose::default();
        for (e, part) in &parts {
            // Only what is partly inside the box.
            if (0..3).any(|a| part.hi[a] < lo[a] || part.lo[a] > hi[a]) {
                continue;
            }
            for (axis, at, u, v) in faces {
                if !(part.lo[axis] < at && at < part.hi[axis]) {
                    continue;
                }
                let mut origin = Vec3::ZERO;
                origin[axis] = at;
                let plane = Plane {
                    origin: origin.as_dvec3().to_array(),
                    u: u.as_dvec3().to_array(),
                    v: v.as_dvec3().to_array(),
                };
                let normal = u.cross(v);
                for region in filled_section(&part.tris, plane) {
                    let cap = region.tag.map(f32::from_bits);
                    let at3 = |p: archisim_core::Point2| origin + u * p.x as f32 + v * p.y as f32;
                    for [a, b, c] in archisim_geom::solid::triangulate(&region.outline) {
                        let corner = |k: usize| {
                            Vertex::new(
                                at3(region.outline[k]),
                                normal,
                                [cap[0], cap[1], cap[2], 1.0],
                                // A cut face: shaded as the caps are.
                                -3.0,
                                [0.0, 0.0],
                                part.cut,
                                part.select,
                            )
                            .with_cap(cap)
                        };
                        out.push(
                            Pass::Opaque,
                            [corner(a), corner(b), corner(c)],
                            Owner::Model(*e),
                        );
                    }
                }
            }
        }
        let ([opaque, _, _], _) = out.finish();
        opaque
    }

    /// The box around the triangles of each element of the project.
    pub fn element_bounds(&self) -> std::collections::BTreeMap<Element, (Vec3, Vec3)> {
        let mut out = std::collections::BTreeMap::new();
        for c in self.opaque.iter().chain(&self.glass) {
            for (t, o) in c
                .mesh
                .indices
                .as_chunks::<3>()
                .0
                .iter()
                .zip(c.owners.iter())
            {
                let Owner::Model(e) = o else {
                    continue;
                };
                for i in t {
                    let p = Vec3::from_array(c.mesh.vertices[*i as usize].pos);
                    out.entry(*e)
                        .and_modify(|b: &mut (Vec3, Vec3)| *b = (b.0.min(p), b.1.max(p)))
                        .or_insert((p, p));
                }
            }
        }
        for g in &self.groups {
            for (e, b) in g.owners.iter().zip(&g.bounds) {
                out.entry(*e)
                    .and_modify(|x: &mut (Vec3, Vec3)| *x = (x.0.min(b.0), x.1.max(b.1)))
                    .or_insert(*b);
            }
        }
        out
    }

    /// The box around the triangles of some elements.
    pub fn bounds_of(&self, elements: &BTreeSet<Element>) -> Option<(Vec3, Vec3)> {
        let all = self.element_bounds();
        elements
            .iter()
            .filter_map(|e| all.get(e))
            .fold(None, |b, (lo, hi)| {
                Some(b.map_or((*lo, *hi), |(a, z): (Vec3, Vec3)| (a.min(*lo), z.max(*hi))))
            })
    }

    /// The nearest element (of the project or linked) hit by a ray,
    /// inside the section box if there is one (what is cut away cannot be
    /// picked).
    pub fn pick_in(
        &self,
        ray: Ray,
        clip: Option<&super::section_box::SectionBox>,
        cut: Option<&CutRule>,
    ) -> Option<Owner> {
        self.hit_in(ray, clip, cut).map(|h| h.owner)
    }

    /// The nearest triangle hit by a ray (inside the section box): how far
    /// along the ray, the triangle and its owner.
    pub fn hit_in(
        &self,
        ray: Ray,
        clip: Option<&super::section_box::SectionBox>,
        cut: Option<&CutRule>,
    ) -> Option<Hit> {
        let mut best: Option<Hit> = None;
        self.each_hit(ray, clip, cut, |h| {
            if best.as_ref().is_none_or(|b| h.distance < b.distance) {
                best = Some(h);
            }
        });
        best
    }

    /// Every element a ray passes through (inside the section box), the
    /// nearest first, each once: for Tab over things one behind another.
    pub fn owners_in(
        &self,
        ray: Ray,
        clip: Option<&super::section_box::SectionBox>,
        cut: Option<&CutRule>,
    ) -> Vec<Owner> {
        let mut near: HashMap<Owner, f32> = HashMap::new();
        self.each_hit(ray, clip, cut, |h| {
            let d = near.entry(h.owner).or_insert(h.distance);
            if h.distance < *d {
                *d = h.distance;
            }
        });
        let mut v: Vec<(Owner, f32)> = near.into_iter().collect();
        v.sort_by(|a, b| a.1.total_cmp(&b.1));
        v.into_iter().map(|(o, _)| o).collect()
    }

    fn each_hit(
        &self,
        ray: Ray,
        clip: Option<&super::section_box::SectionBox>,
        cut: Option<&CutRule>,
        mut found: impl FnMut(Hit),
    ) {
        let mut consider = |d: f32, tri: [Vec3; 3], owner: Owner, code: [f32; 4]| {
            let at = ray.origin + ray.dir * d;
            if clip.is_some_and(|bx| !bx.contains(at)) {
                return;
            }
            if cut.is_some_and(|r| at.z > r.top_of(code) + 1e-4) {
                return;
            }
            found(Hit {
                distance: d,
                triangle: tri,
                owner,
            });
        };
        for c in self.opaque.iter().chain(&self.glass) {
            if !ray_meets_box(ray, c.lo, c.hi) {
                continue;
            }
            for (t, o) in c
                .mesh
                .indices
                .as_chunks::<3>()
                .0
                .iter()
                .zip(c.owners.iter())
            {
                let v = t.map(|i| c.mesh.vertices[i as usize]);
                let p = v.map(|v| Vec3::from_array(v.pos));
                if let Some(d) = hit(ray, p[0], p[1], p[2]) {
                    consider(d, p, *o, v[0].cut);
                }
            }
        }
        for g in &self.groups {
            for ((inst, e), (lo, hi)) in g.instances.iter().zip(&g.owners).zip(&g.bounds) {
                if !ray_meets_box(ray, *lo, *hi) {
                    continue;
                }
                // The ray in the object's axes: the distances along it are
                // the same (its scale is one for the furniture).
                let local = Ray {
                    origin: inst.unplace(ray.origin),
                    dir: inst.unturn(ray.dir),
                };
                for t in g.local_triangles() {
                    if let Some(d) = hit(local, t[0], t[1], t[2]) {
                        consider(d, t.map(|p| inst.place(p)), Owner::Model(*e), inst.cut);
                    }
                }
            }
        }
    }

    /// The triangles of one element, where they are.
    pub fn triangles_of(&self, owner: Owner) -> Vec<[Vec3; 3]> {
        let mut out = Vec::new();
        self.each_triangle(|t, o, _| {
            if o == owner {
                out.push(t);
            }
            true
        });
        out
    }

    /// The nearest element of the project hit by a ray (none if a linked
    /// element is in front).
    #[cfg(test)]
    pub fn pick(&self, ray: Ray) -> Option<Element> {
        match self.pick_in(ray, None, None)? {
            Owner::Model(e) => Some(e),
            Owner::Linked(..) => None,
        }
    }
}

/// A triangle hit by a ray.
#[derive(Debug, Clone, Copy)]
pub struct Hit {
    pub distance: f32,
    pub triangle: [Vec3; 3],
    pub owner: Owner,
}

/// Whether a ray passes through a box (slabs).
fn ray_meets_box(ray: Ray, lo: Vec3, hi: Vec3) -> bool {
    let inv = ray.dir.recip();
    let a = (lo - Vec3::splat(1e-3) - ray.origin) * inv;
    let b = (hi + Vec3::splat(1e-3) - ray.origin) * inv;
    let near = a.min(b).max_element();
    let far = a.max(b).min_element();
    far >= near.max(0.0)
}

/// Adds solids of the model to where they go, each triangle with its
/// owner.
/// Where [`add_solids`] puts the triangles of a piece to keep: by pass,
/// corners shared.
#[derive(Default)]
struct Capture {
    passes: [MeshBuilder; 3],
}

impl Sink for Capture {
    fn push(&mut self, pass: Pass, tri: [Vertex; 3], _owner: Owner) {
        self.passes[pass as usize].push(tri);
    }

    fn glow(&mut self, _id: u64, _p: Vec3, _reach: f32) {}
}

/// A piece of the ground, a street or a marking made for the 3D, by pass,
/// with the scene it was last used in.
type Piece = std::sync::Arc<[Mesh; 3]>;
/// The number of the last scene, and the pieces kept with the last scene
/// that used each.
type KeptPieces = (u64, HashMap<u64, (Piece, u64)>);
static KEPT_PIECES: std::sync::Mutex<Option<KeptPieces>> = std::sync::Mutex::new(None);

/// A new scene's number, for the pieces it uses.
fn next_piece_generation() -> u64 {
    let mut k = KEPT_PIECES.lock().unwrap_or_else(|e| e.into_inner());
    let k = k.get_or_insert_with(|| (0, HashMap::new()));
    k.0 += 1;
    k.0
}

fn kept_piece(key: u64, generation: u64) -> Option<Piece> {
    let mut k = KEPT_PIECES.lock().ok()?;
    let e = k.as_mut()?.1.get_mut(&key)?;
    e.1 = generation;
    Some(e.0.clone())
}

fn keep_piece(key: u64, piece: Piece, generation: u64) {
    if let Ok(mut k) = KEPT_PIECES.lock() {
        k.get_or_insert_with(|| (0, HashMap::new()))
            .1
            .insert(key, (piece, generation));
    }
}

/// Lets go the pieces not used by this scene nor the one before.
fn forget_pieces_before(generation: u64) {
    if let Ok(mut k) = KEPT_PIECES.lock() {
        if let Some(k) = k.as_mut() {
            k.1.retain(|_, e| e.1 + 1 >= generation);
        }
    }
}

/// The key a piece is kept by, if it is kept: the ground, streets and
/// markings, by what they are made of and how they are shown (selected,
/// greyed); never while a clash is isolated.
fn piece_key(model: &Model, solid: &archisim_geom::solid::Solid, look: &Look) -> Option<u64> {
    if look.isolate || look.shared || look.between {
        return None;
    }
    if !matches!(
        solid.owner,
        SolidOwner::Terrain(_) | SolidOwner::Road(_) | SolidOwner::Marking(_)
    ) {
        return None;
    }
    let id = solid.owner.id();
    let selected = look.selection.contains(&element(solid.owner));
    let dimmed = model.dimmed(look.disciplines.focus, id);
    let salt = 0x7e44_0000 | (u64::from(selected) << 1) | u64::from(dimmed);
    Some(model_key(model, std::slice::from_ref(solid), &[], salt))
}

/// The models of buildings and plants finished (their versions for
/// farther away included), kept to be given again while what they are
/// made of stays the same.
static KEPT_MODELS: std::sync::Mutex<
    Option<HashMap<u64, std::sync::Arc<scene_build::GroupModel>>>,
> = std::sync::Mutex::new(None);
/// How many corners of finished models are kept at most (48 bytes each).
const KEPT_MODEL_CORNERS: usize = if cfg!(target_arch = "wasm32") {
    2_000_000
} else {
    40_000_000
};

fn kept_model(key: u64) -> Option<std::sync::Arc<scene_build::GroupModel>> {
    KEPT_MODELS
        .lock()
        .ok()
        .and_then(|k| k.as_ref().and_then(|k| k.get(&key).cloned()))
}

fn keep_model(key: u64, model: std::sync::Arc<scene_build::GroupModel>) {
    if let Ok(mut k) = KEPT_MODELS.lock() {
        let k = k.get_or_insert_with(HashMap::new);
        let corners: usize = k
            .values()
            .map(|m| m.opaque.vertices.len() + m.glass.vertices.len())
            .sum();
        if corners > KEPT_MODEL_CORNERS {
            k.clear();
        }
        k.insert(key, model);
    }
}

/// A key for what a model is made of: its solids (owners, materials and
/// corners), how their materials look and what a building of the
/// surroundings is dressed with, and `salt`.
fn model_key(
    model: &Model,
    solids: &[archisim_geom::solid::Solid],
    more: &[archisim_geom::solid::Solid],
    salt: u64,
) -> u64 {
    let mut h = FastHash::new(salt);
    let mut seen = std::collections::BTreeSet::new();
    for (k, list) in [solids, more].into_iter().enumerate() {
        h.add(k as u64);
        for s in list {
            h.add(s.owner.id());
            h.add(s.material.0);
            h.add(s.triangles.len() as u64);
            for t in &s.triangles {
                for p in t {
                    for c in p {
                        h.add(c.to_bits());
                    }
                }
            }
            seen.insert(s.material);
            if let SolidOwner::Mass(m) = s.owner {
                if let Some(m) = model.mass(m) {
                    h.add_str(&format!("{:?}", m.dress));
                }
            }
        }
    }
    for m in seen {
        h.add(material_hash(model, m));
    }
    h.finish()
}

/// What a material is and how it looks, hashed: kept while it is the same
/// (writing it out for each piece of a street took longer than the rest).
fn material_hash(model: &Model, id: archisim_core::MaterialId) -> u64 {
    type Kept = HashMap<
        archisim_core::MaterialId,
        (
            Option<archisim_core::Material>,
            archisim_core::Appearance,
            u64,
        ),
    >;
    thread_local! {
        static KEPT: std::cell::RefCell<Kept> = Default::default();
    }
    let mat = model.material(id);
    let look = model.appearance(id);
    KEPT.with(|k| {
        let mut k = k.borrow_mut();
        if let Some((m, a, h)) = k.get(&id) {
            if m.as_ref() == mat && *a == look {
                return *h;
            }
        }
        let mut h = FastHash::new(id.0);
        h.add_str(&format!("{mat:?}"));
        h.add_str(&format!("{look:?}"));
        let h = h.finish();
        k.insert(id, (mat.cloned(), look, h));
        h
    })
}

/// A quick hash of numbers (not for anything an adversary chooses).
struct FastHash(u64);

impl FastHash {
    const K: u64 = 0x517c_c1b7_2722_0a95;

    fn new(seed: u64) -> Self {
        FastHash(seed ^ Self::K)
    }

    fn add(&mut self, v: u64) {
        self.0 = (self.0.rotate_left(5) ^ v).wrapping_mul(Self::K);
    }

    fn add_str(&mut self, s: &str) {
        for c in s.as_bytes().chunks(8) {
            let mut b = [0u8; 8];
            b[..c.len()].copy_from_slice(c);
            self.add(u64::from_le_bytes(b));
        }
        self.add(s.len() as u64);
    }

    fn finish(&self) -> u64 {
        let mut x = self.0;
        x ^= x >> 33;
        x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
        x ^= x >> 33;
        x
    }
}

/// A plant's model in its own axes, made as any solid is, opaque and see
/// through (with its mesh, so that the mesh it was made from stays the
/// same one while it is kept).
struct PlantModel {
    _from: std::sync::Arc<archisim_core::flora::FloraMesh>,
    opaque: Mesh,
    glass: Mesh,
}

/// The plant models kept, by mesh, its parts' materials and detail.
type PlantKey = (usize, Vec<u64>, archisim_core::flora::Lod);
static PLANT_MODELS: std::sync::Mutex<Option<HashMap<PlantKey, std::sync::Arc<PlantModel>>>> =
    std::sync::Mutex::new(None);
/// How many plant models are kept at most.
const PLANT_MODELS_MAX: usize = 20_000;

/// The model of plant `id` from its mesh: kept, the same for every plant
/// that grows the same.
fn plant_model(
    model: &Model,
    id: archisim_core::ObjectId,
    mesh: std::sync::Arc<archisim_core::flora::FloraMesh>,
    parts: &[(archisim_core::MaterialId, Vec<[u32; 3]>)],
    lod: archisim_core::flora::Lod,
) -> std::sync::Arc<PlantModel> {
    let key: PlantKey = (
        std::sync::Arc::as_ptr(&mesh) as usize,
        parts.iter().map(|p| p.0 .0).collect(),
        lod,
    );
    if let Some(m) = PLANT_MODELS
        .lock()
        .ok()
        .and_then(|k| k.as_ref().and_then(|k| k.get(&key).cloned()))
    {
        return m;
    }
    let solids: Vec<archisim_geom::solid::Solid> = parts
        .iter()
        .map(|(material, tris)| archisim_geom::solid::Solid {
            owner: SolidOwner::Object(id),
            material: *material,
            triangles: tris
                .iter()
                .map(|t| t.map(|i| mesh.positions[i as usize].map(|c| c as f64)))
                .collect(),
        })
        .collect();
    let mut shared = Shared::default();
    add_solids(
        model,
        solids,
        &Look {
            selection: &BTreeSet::new(),
            isolate: false,
            disciplines: &Disciplines::default(),
            anchors: &HashMap::new(),
            shared: true,
            between: false,
        },
        &mut shared,
    );
    let shared = shared.done();
    let made = std::sync::Arc::new(PlantModel {
        _from: mesh,
        opaque: shared.opaque.mesh,
        glass: shared.glass.mesh,
    });
    if let Ok(mut k) = PLANT_MODELS.lock() {
        let k = k.get_or_insert_with(HashMap::new);
        if k.len() >= PLANT_MODELS_MAX {
            k.clear();
        }
        k.insert(key, made.clone());
    }
    made
}

fn add_solids(
    model: &Model,
    solids: Vec<archisim_geom::solid::Solid>,
    look: &Look,
    out: &mut dyn Sink,
) {
    // The stuffed fabric of each object.
    let mut fabric_of: HashMap<archisim_core::ObjectId, Vec<archisim_core::MaterialId>> =
        Default::default();
    // The many small leaves of each object (drawn without contours).
    let mut fine_of: HashMap<archisim_core::ObjectId, Vec<archisim_core::MaterialId>> =
        Default::default();
    for solid in solids {
        let id = solid.owner.id();
        if !look.shared && model.hidden_by_discipline(&look.disciplines.hidden, id) {
            continue;
        }
        let _span = archisim_geom::meter::span(match solid.owner {
            SolidOwner::Terrain(_) => "conversión: terreno",
            SolidOwner::Road(_) => "conversión: calles",
            SolidOwner::Object(_) => "conversión: objetos",
            SolidOwner::Mass(_) => "conversión: volúmenes",
            _ => "conversión: lo demás",
        });
        // The orthophoto is picked as the terrain under it.
        let owner = Owner::Model(match solid.owner {
            SolidOwner::Orthophoto => model
                .terrains()
                .next()
                .map_or(element(solid.owner), |t| Element::Terrain(t.id)),
            o => element(o),
        });
        let mat = model.material(solid.material);
        let dimmed = !look.shared && model.dimmed(look.disciplines.focus, id);
        let selected = !look.shared && look.selection.contains(&element(solid.owner));
        let ghost = look.isolate && !selected;
        let is_water = mat.is_some_and(|m| m.category == "water");
        let is_glass = ghost || is_water || mat.is_some_and(|m| m.category == "glass");
        // Doors and windows win where they touch the reveals.
        let opening = matches!(solid.owner, SolidOwner::Opening(_));
        let alpha = if ghost {
            GHOST_ALPHA
        } else if is_water {
            WATER_ALPHA
        } else if is_glass {
            GLASS_ALPHA
        } else {
            1.0
        };
        let tone = |[r, g, b]: [u8; 3], dim: bool| {
            let mut rgb = [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b)];
            if dim {
                // In light grey, as a reference.
                let lum = 0.3 * rgb[0] + 0.59 * rgb[1] + 0.11 * rgb[2];
                let v = 1.0 - (1.0 - lum) * 0.35;
                rgb = [v, v, v];
            }
            [rgb[0], rgb[1], rgb[2], alpha]
        };
        // The leaves, flowers and fruit of a plant grown by ArchiSim:
        // rounded (normals smoothed over each clump) and without contour
        // lines between their facets.
        let soft = match solid.owner {
            SolidOwner::Object(o) => model
                .plant_look(o)
                .is_some_and(|l| solid.material != l.materials.bark),
            // Tiles too: hundreds of covers would otherwise be drawn as as
            // many contour lines and darken the roof from afar.
            SolidOwner::Roof(r) => model.roof(r).is_some_and(|r| {
                r.covering.tiles.is_some() && model.tile_material(r) == Some(solid.material)
            }),
            _ => false,
        };
        // Stuffed fabric (the workshop's upholstery): it has no sharp edge,
        // so it is shaded smooth all over, and outlined only where it ends
        // against what is behind, never at its own seams and folds (drawn
        // as foliage is, bias 2).
        let fabric = match solid.owner {
            SolidOwner::Object(o) if !soft => fabric_of
                .entry(o)
                .or_insert_with(|| model.object_soft_materials(o))
                .contains(&solid.material),
            _ => false,
        };
        // Many small leaves (the terrace's plants): no contour lines at
        // all, or every leaf would be drawn as a dark stroke (bias 5).
        let fine = match solid.owner {
            SolidOwner::Object(o) if fabric => fine_of
                .entry(o)
                .or_insert_with(|| model.object_fine_materials(o))
                .contains(&solid.material),
            // The leaves of a green wall (H5).
            SolidOwner::Facade(_) => model.appearance(solid.material).growth.is_some(),
            // The courses of tiles of the buildings of the surroundings:
            // shaded by the light, not outlined.
            SolidOwner::Mass(m) => model.mass(m).is_some_and(|m| {
                m.dress.is_some_and(|d| {
                    d.materials.roof == solid.material && d.roof != archisim_core::EnvRoof::Flat
                })
            }),
            _ => false,
        };
        let smooth: Option<HashMap<[i64; 3], Vec3>> = (soft || fabric).then(|| {
            let mut acc: HashMap<[i64; 3], Vec3> = Default::default();
            for tri in &solid.triangles {
                let p = tri.map(|v| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32));
                let n = (p[1] - p[0]).cross(p[2] - p[0]);
                for v in tri {
                    *acc.entry(vertex_key(v)).or_default() += n;
                }
            }
            acc
        });
        // Furniture: rounded edges and cushions shaded smooth (the faces of
        // each corner that turn less than ~40° share their normal), so they
        // are drawn soft, without a contour line at every facet of a
        // rounding; real edges stay sharp.
        let creased: Option<HashMap<[i64; 3], Vec<Vec3>>> =
            (!soft && !fabric && matches!(solid.owner, SolidOwner::Object(_))).then(|| {
                let mut acc: HashMap<[i64; 3], Vec<Vec3>> = Default::default();
                for tri in &solid.triangles {
                    let p = tri.map(|v| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32));
                    let n = (p[1] - p[0]).cross(p[2] - p[0]);
                    if n.length_squared() < 1e-14 {
                        continue;
                    }
                    for v in tri {
                        acc.entry(vertex_key(v)).or_default().push(n);
                    }
                }
                acc
            });
        // A lampshade, a bulb or a flame: it glows at night, and its middle
        // is where the lamp's light comes from.
        let (glows, reach) = match archisim_geom::solid::solid_glow(model, &solid) {
            Some(reach) if !dimmed && !ghost => (true, reach as f32),
            _ => (false, 0.0),
        };
        if glows {
            for tri in &solid.triangles {
                for v in tri {
                    out.glow(id, Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32), reach);
                }
            }
        }
        let base = tone(archisim_geom::solid::solid_color(model, &solid), dimmed);
        let cut = if look.shared {
            [0.0; 4]
        } else {
            cut_of(model, look.anchors, &solid)
        };
        // The cut of a solid shows its material's cut colour, as the
        // plan's poché (each layer of a wall its own).
        let cap = {
            let t = tone(mat.map_or([200, 200, 200], |m| m.cut_color), dimmed);
            [t[0], t[1], t[2]]
        };
        let select = if selected { 1.0 } else { 0.0 };
        let is_object = matches!(solid.owner, SolidOwner::Object(_));
        let texture_of = |m: archisim_core::MaterialId| {
            archisim_geom::solid::texture_of(model, m, is_object)
                .map_or([0.0, 0.0], |(t, scale)| [t.number() as f32, scale as f32])
        };
        // Grass and flowers: their own colours, no contour lines at all
        // (thousands of blades would be drawn as as many).
        let grass = matches!(solid.owner, SolidOwner::Growth(_));
        // Each piece of a façade is a brick or a panel: no texture on it.
        let piece = matches!(solid.owner, SolidOwner::Facade(_));
        let own_tex = if dimmed || ghost || grass || piece {
            [0.0, 0.0]
        } else if matches!(solid.owner, SolidOwner::Orthophoto) {
            [0.0, ORTHO_TEX]
        } else {
            texture_of(solid.material)
        };
        // A model brought from another program: the colours of its
        // vertices (its textures).
        let corners = archisim_geom::solid::solid_vertex_colors(model, &solid);
        let pass = if grass {
            Pass::Grass
        } else if is_glass {
            Pass::Glass
        } else {
            Pass::Opaque
        };
        for (i, tri) in solid.triangles.iter().enumerate() {
            // A painted face: its paint, never greyed (it is what is being
            // worked on).
            let painted = archisim_geom::solid::painted_material(model, &solid, tri);
            let color = painted.map_or(base, |m| tone(model.surface_color(m), false));
            let tex = painted.map_or(own_tex, texture_of);
            let colors = match corners.as_ref().and_then(|c| c.get(i)) {
                Some(c) => c.map(|c| tone(c, dimmed)),
                None => [color; 3],
            };
            let p = tri.map(|v| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32));
            let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
            // Faces looking up give way where another element rests on
            // them: the floor under a wall, the top of a wall under the
            // roof slab.
            let bias = if look.between {
                // A face between two layers, drawn only as the cut.
                -2.0
            } else if grass {
                4.0
            } else if fine {
                5.0
            } else if glows {
                3.0
            } else if soft || fabric {
                2.0
            } else if opening {
                -1.0
            } else if n.z > 0.5 {
                1.0
            } else {
                0.0
            };
            let corner = |k: usize| {
                let normal = smooth
                    .as_ref()
                    .and_then(|m| m.get(&vertex_key(&tri[k])))
                    .map(|s| s.normalize_or_zero())
                    // Both faces of a leaf cancel out: its own.
                    .filter(|s| s.length_squared() > 0.5 && s.dot(n) > 0.0)
                    .or_else(|| {
                        let around = creased.as_ref()?.get(&vertex_key(&tri[k]))?;
                        let sum: Vec3 = around
                            .iter()
                            .filter(|f| f.normalize_or_zero().dot(n) > 0.77)
                            .sum();
                        Some(sum.normalize_or_zero()).filter(|s| s.length_squared() > 0.5)
                    })
                    .unwrap_or(n);
                Vertex::new(p[k], normal, colors[k], bias, tex, cut, select).with_cap(cap)
            };
            let v = [corner(0), corner(1), corner(2)];
            out.push(pass, v, owner);
        }
    }
}

/// Where a wall is for the cut away walls: the middle of its plan and
/// its base.
#[derive(Debug, Clone, Copy)]
struct Anchor {
    lo: Vec3,
    hi: Vec3,
}

impl Anchor {
    fn cut(&self) -> [f32; 4] {
        let c = (self.lo + self.hi) / 2.0;
        [cutaway::WALL, c.x, c.y, self.lo.z]
    }
}

/// The box of each wall's solids.
fn anchors(
    solids: &[archisim_geom::solid::Solid],
) -> std::collections::HashMap<archisim_core::WallId, Anchor> {
    let mut out: std::collections::HashMap<archisim_core::WallId, Anchor> = Default::default();
    for s in solids {
        let SolidOwner::Wall(w) = s.owner else {
            continue;
        };
        for v in s.triangles.iter().flatten() {
            let p = Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
            let a = out.entry(w).or_insert(Anchor { lo: p, hi: p });
            a.lo = a.lo.min(p);
            a.hi = a.hi.max(p);
        }
    }
    out
}

/// The box of a solid.
fn bounds(solid: &archisim_geom::solid::Solid) -> Option<(Vec3, Vec3)> {
    solid.triangles.iter().flatten().fold(None, |b, v| {
        let p = Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
        Some(b.map_or((p, p), |(lo, hi): (Vec3, Vec3)| (lo.min(p), hi.max(p))))
    })
}

/// What the cut away walls do with a solid (`Vertex::cut`).
fn cut_of(
    model: &Model,
    walls: &std::collections::HashMap<archisim_core::WallId, Anchor>,
    solid: &archisim_geom::solid::Solid,
) -> [f32; 4] {
    let own = |kind: f32| {
        bounds(solid).map_or([kind, 0.0, 0.0, 0.0], |(lo, hi)| {
            let c = (lo + hi) / 2.0;
            [kind, c.x, c.y, lo.z]
        })
    };
    match solid.owner {
        SolidOwner::Wall(w) | SolidOwner::Facade(w) => {
            walls.get(&w).map_or(own(cutaway::WALL), |a| a.cut())
        }
        // Doors and windows go with their wall.
        SolidOwner::Opening(o) => model
            .opening(o)
            .and_then(|o| walls.get(&o.wall))
            .map_or(own(cutaway::WALL), |a| a.cut()),
        SolidOwner::Column(_) | SolidOwner::CurtainWall(_) => own(cutaway::WALL),
        // Furniture by where it stands, not by its lowest piece: a lamp's
        // shade (a solid of its own material) goes with its foot.
        SolidOwner::Object(id) => {
            let mut c = own(cutaway::OBJECT);
            if let Some(o) = model.object(id) {
                let floor = model.level(o.level).map_or(0.0, |l| l.elevation);
                c[3] = (floor + o.base_offset) as f32;
            }
            c
        }
        SolidOwner::Device(_) => own(cutaway::OBJECT),
        // A stair stays whole up to the floor it reaches, as the
        // furniture: by where its foot is.
        SolidOwner::Stair(id) => {
            let mut c = own(cutaway::OBJECT);
            if let Some(s) = model.stair(id) {
                let floor = model.level(s.level).map_or(0.0, |l| l.elevation);
                c[3] = (floor + s.base_offset) as f32;
            }
            c
        }
        _ => own(cutaway::PLAIN),
    }
}

/// Möller–Trumbore ray/triangle intersection: distance along the ray.
fn hit(ray: Ray, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let e1 = b - a;
    let e2 = c - a;
    let p = ray.dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-9 {
        return None;
    }
    let inv = 1.0 / det;
    let s = ray.origin - a;
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = ray.dir.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t > 1e-4).then_some(t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pick::tests::room;

    #[test]
    fn builds_and_picks_the_room() {
        let (m, _, _) = room();
        let mut s = Scene::build(&m, &BTreeSet::new());
        assert!(!s.is_empty());
        for c in &s.opaque {
            assert_eq!(c.mesh.indices.len() % 3, 0);
            assert_eq!(c.owners.len(), c.mesh.indices.len() / 3);
            // Corners are shared: fewer than three per triangle.
            assert!(c.mesh.vertices.len() < c.mesh.indices.len());
        }
        // The 6 × 4 room, 3 m high, and the slab below.
        assert!((s.hi.z - 3.0).abs() < 1e-5 && s.lo.z < 0.0);
        assert!((s.hi.x - 6.0).abs() < 1e-5 && (s.hi.y - 4.0).abs() < 1e-5);

        // Looking straight down at the middle of the west wall: the wall.
        let west = m.walls().next().unwrap().id;
        let down = Ray {
            origin: Vec3::new(0.1, 2.0, 10.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
        };
        assert_eq!(s.pick(down), Some(Element::Wall(west)));
        // Down into the room: the slab.
        let slab = m.slabs().next().unwrap().id;
        let into_room = Ray {
            origin: Vec3::new(3.0, 2.0, 10.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
        };
        assert_eq!(s.pick(into_room), Some(Element::Slab(slab)));
        // From the south, at door height, through the door in the south wall.
        let door = m.openings().next().unwrap().id;
        let at_door = Ray {
            origin: Vec3::new(3.0, -5.0, 1.0),
            dir: Vec3::new(0.0, 1.0, 0.0),
        };
        assert_eq!(s.pick(at_door), Some(Element::Opening(door)));
        // Up into the sky: nothing.
        let up = Ray {
            origin: Vec3::new(3.0, 2.0, 10.0),
            dir: Vec3::Z,
        };
        assert_eq!(s.pick(up), None);

        // Selecting marks the selected element (to outline it) and
        // leaves every colour as it was.
        let colours = |s: &Scene| -> Vec<[u8; 4]> {
            s.opaque
                .iter()
                .flat_map(|c| c.mesh.vertices.iter().map(|v| v.color))
                .collect()
        };
        let before = colours(&s);
        let mut sel = BTreeSet::new();
        sel.insert(Element::Wall(west));
        s = Scene::build(&m, &sel);
        let marked: usize = s
            .opaque
            .iter()
            .map(|c| c.mesh.vertices.iter().filter(|v| v.select() > 0.5).count())
            .sum();
        assert!(marked > 0 && marked < before.len());
        // Marking the selection on the scene there is gives the same
        // triangles marked as building it again.
        let marked_triangles = |s: &Scene| -> usize {
            s.opaque
                .iter()
                .map(|c| {
                    c.mesh
                        .indices
                        .chunks(3)
                        .filter(|t| {
                            t.iter()
                                .all(|&i| c.mesh.vertices[i as usize].select() > 0.5)
                        })
                        .count()
                })
                .sum()
        };
        let mut again = Scene::build(&m, &BTreeSet::new());
        again.reselect(&BTreeSet::new(), &sel);
        assert_eq!(marked_triangles(&again), marked_triangles(&s));
        again.reselect(&sel, &BTreeSet::new());
        assert_eq!(marked_triangles(&again), 0);
        let mut a = before.clone();
        let mut b = colours(&s);
        a.sort_unstable();
        a.dedup();
        b.sort_unstable();
        b.dedup();
        assert_eq!(a, b);

        // Cut away at 1.20 m: through the wall above the cut, the slab;
        // the door and the wall share their wall's point.
        let rule = CutRule {
            top: 1.2,
            front: None,
        };
        let low_down = Ray {
            origin: Vec3::new(0.1, 2.0, 10.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
        };
        assert_eq!(
            s.pick_in(low_down, None, Some(&rule)),
            Some(Owner::Model(Element::Wall(west)))
        );
        let first_of = |e: Element| {
            s.opaque
                .iter()
                .find_map(|c| {
                    c.mesh
                        .indices
                        .chunks(3)
                        .zip(c.owners.iter())
                        .find(|(_, o)| **o == Owner::Model(e))
                        .map(|(t, _)| c.mesh.vertices[t[0] as usize].cut)
                })
                .unwrap()
        };
        let south = m.opening(door).unwrap().wall;
        assert_eq!(
            first_of(Element::Opening(door)),
            first_of(Element::Wall(south))
        );
        assert_eq!(first_of(Element::Wall(south))[0], cutaway::WALL);
    }
}
