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
    seen: HashMap<[u32; 12], u32>,
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
    pub mesh: Mesh,
    pub owners: Vec<Owner>,
    pub lo: Vec3,
    pub hi: Vec3,
}

/// Triangles in a chunk: small enough to leave out what is not seen,
/// large enough not to draw too many pieces.
const CHUNK: usize = 32_768;

/// A model shared by several objects, and where each stands.
#[derive(Debug, Clone, Default)]
pub struct Group {
    /// Its opaque triangles: all of them (the first `levels[0]` indices),
    /// then each coarser version for farther away.
    pub opaque: Mesh,
    /// The first index and the count of each version, finest first.
    pub levels: Vec<(u32, u32)>,
    pub glass: Mesh,
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
/// What is left of the rest of the model around an isolated clash.
pub const GHOST_ALPHA: f32 = 0.12;

/// What a triangle belongs to: an element of the project, or an element
/// of a linked model (link, element index).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

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
#[derive(Default)]
struct Loose {
    passes: [(Vec<Chunk>, MeshBuilder); 3],
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
            chunks.retain(|c| !c.mesh.is_empty());
            out[k] = chunks;
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
            chunks.push(Chunk {
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
        let mut s = Scene {
            lo: Vec3::splat(f32::INFINITY),
            hi: Vec3::splat(f32::NEG_INFINITY),
            ..Default::default()
        };
        // The furniture shared by type (never while isolating a clash,
        // whose ghosts are drawn see-through one by one).
        let mut skip = std::collections::HashSet::new();
        let mut keys: HashMap<(usize, Vec<archisim_core::MaterialId>), usize> = HashMap::new();
        let mut groups: Vec<(Shared, Group)> = Vec::new();
        if !isolate {
            for o in model.objects() {
                if model.hidden_by_discipline(&disciplines.hidden, o.id.0) {
                    continue;
                }
                let Some(inst) = archisim_geom::solid::object_instance(model, o.id) else {
                    continue;
                };
                skip.insert(o.id);
                let key = (
                    std::sync::Arc::as_ptr(&inst.mesh) as usize,
                    inst.parts.iter().map(|p| p.0).collect::<Vec<_>>(),
                );
                let g = *keys.entry(key).or_insert_with(|| {
                    let mut shared = Shared::default();
                    let solids: Vec<archisim_geom::solid::Solid> = inst
                        .parts
                        .iter()
                        .map(|(material, tris)| archisim_geom::solid::Solid {
                            owner: SolidOwner::Object(o.id),
                            material: *material,
                            triangles: tris
                                .iter()
                                .map(|t| {
                                    t.map(|i| inst.mesh.positions[i as usize].map(|c| c as f64))
                                })
                                .collect(),
                        })
                        .collect();
                    let look = Look {
                        selection,
                        isolate: false,
                        disciplines,
                        anchors: &HashMap::new(),
                        shared: true,
                        between: false,
                    };
                    add_solids(model, solids, &look, &mut shared);
                    groups.push((shared, Group::default()));
                    groups.len() - 1
                });
                let (shared, group) = &mut groups[g];
                let (sn, cs) = (inst.rotation as f32).sin_cos();
                let mut instance = Instance {
                    place: [cs, sn, inst.origin[0] as f32, inst.origin[1] as f32],
                    lift: [inst.origin[2] as f32, 1.0, 1.0, 1.0],
                    cut: [0.0; 4],
                    flags: [
                        if selection.contains(&Element::Object(o.id)) {
                            1.0
                        } else {
                            0.0
                        },
                        if model.dimmed(disciplines.focus, o.id.0) {
                            1.0
                        } else {
                            0.0
                        },
                        0.0,
                        0.0,
                    ],
                };
                // Its box where it stands, from the model's corners.
                let mut lo = Vec3::splat(f32::INFINITY);
                let mut hi = Vec3::splat(f32::NEG_INFINITY);
                for v in shared
                    .opaque
                    .mesh
                    .vertices
                    .iter()
                    .chain(&shared.glass.mesh.vertices)
                {
                    let p = instance.place(Vec3::from_array(v.pos));
                    lo = lo.min(p);
                    hi = hi.max(p);
                }
                if lo.x > hi.x {
                    continue;
                }
                let c = (lo + hi) / 2.0;
                instance.cut = [cutaway::OBJECT, c.x, c.y, inst.origin[2] as f32];
                if let Some((sum, n, reach)) = shared.glow.filter(|g| g.1 > 0.0) {
                    if instance.flags[1] < 0.5 {
                        let m = instance.place(sum / n);
                        s.lamps.push([m.x, m.y, m.z, reach]);
                    }
                }
                s.lo = s.lo.min(lo);
                s.hi = s.hi.max(hi);
                group.instances.push(instance);
                group.owners.push(Element::Object(o.id));
                group.bounds.push((lo, hi));
            }
        }
        // The doors and windows whose family makes a detailed model
        // (paquete 9): the model by type and hand, drawn in each, nearer
        // or farther with fewer triangles; the rest of their body (frame,
        // transom, sidelights) as before. Their cut is their wall's, set
        // once the walls are known.
        let mut detailed = std::collections::HashSet::new();
        let mut opening_walls: HashMap<archisim_core::OpeningId, archisim_core::WallId> =
            HashMap::new();
        if !isolate {
            for o in model.openings() {
                if model.hidden_by_discipline(&disciplines.hidden, o.id.0) {
                    continue;
                }
                let Some(inst) = archisim_geom::solid::opening_detail(model, o.id) else {
                    continue;
                };
                detailed.insert(o.id);
                opening_walls.insert(o.id, inst.wall);
                let key = (
                    std::sync::Arc::as_ptr(&inst.mesh) as usize,
                    inst.parts.iter().map(|p| p.0).collect::<Vec<_>>(),
                );
                let g = *keys.entry(key).or_insert_with(|| {
                    let mut shared = Shared::default();
                    let solids: Vec<archisim_geom::solid::Solid> = inst
                        .parts
                        .iter()
                        .map(|(material, tris)| archisim_geom::solid::Solid {
                            owner: SolidOwner::Opening(o.id),
                            material: *material,
                            triangles: tris
                                .iter()
                                .map(|t| {
                                    t.map(|i| inst.mesh.positions[i as usize].map(|c| c as f64))
                                })
                                .collect(),
                        })
                        .collect();
                    let look = Look {
                        selection,
                        isolate: false,
                        disciplines,
                        anchors: &HashMap::new(),
                        shared: true,
                        between: false,
                    };
                    add_solids(model, solids, &look, &mut shared);
                    if let Some(far) = archisim_geom::solid::opening_detail_with(model, o.id, true)
                    {
                        let solids = far
                            .parts
                            .iter()
                            .map(|(material, tris)| archisim_geom::solid::Solid {
                                owner: SolidOwner::Opening(o.id),
                                material: *material,
                                triangles: tris
                                    .iter()
                                    .map(|t| {
                                        t.map(|i| far.mesh.positions[i as usize].map(|c| c as f64))
                                    })
                                    .collect(),
                            })
                            .collect();
                        let mut plain = Shared::default();
                        add_solids(model, solids, &look, &mut plain);
                        shared.simple = Some(plain.opaque.mesh);
                    }
                    groups.push((shared, Group::default()));
                    groups.len() - 1
                });
                let (shared, group) = &mut groups[g];
                group.exact_shadow = true;
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
                    flags: [
                        if selection.contains(&Element::Opening(o.id)) {
                            1.0
                        } else {
                            0.0
                        },
                        if model.dimmed(disciplines.focus, o.id.0) {
                            1.0
                        } else {
                            0.0
                        },
                        0.0,
                        0.0,
                    ],
                };
                let mut lo = Vec3::splat(f32::INFINITY);
                let mut hi = Vec3::splat(f32::NEG_INFINITY);
                for v in shared
                    .opaque
                    .mesh
                    .vertices
                    .iter()
                    .chain(&shared.glass.mesh.vertices)
                {
                    let p = instance.place(Vec3::from_array(v.pos));
                    lo = lo.min(p);
                    hi = hi.max(p);
                }
                if lo.x > hi.x {
                    continue;
                }
                instance.cut[3] = lo.z;
                s.lo = s.lo.min(lo);
                s.hi = s.hi.max(hi);
                group.instances.push(instance);
                group.owners.push(Element::Opening(o.id));
                group.bounds.push((lo, hi));
            }
        }
        for (shared, mut group) in groups {
            group.opaque = shared.opaque.mesh;
            group.glass = shared.glass.mesh;
            let mut lo = Vec3::splat(f32::INFINITY);
            let mut hi = Vec3::splat(f32::NEG_INFINITY);
            for v in group.opaque.vertices.iter().chain(&group.glass.vertices) {
                lo = lo.min(Vec3::from_array(v.pos));
                hi = hi.max(Vec3::from_array(v.pos));
            }
            (group.lo, group.hi) = (lo, hi);
            let size = (hi - lo).length().max(0.05);
            group.levels = match shared.simple {
                Some(plain) if !plain.is_empty() => {
                    // Its plain version after the whole of it.
                    let full = group.opaque.indices.len() as u32;
                    let base = group.opaque.vertices.len() as u32;
                    group.opaque.vertices.extend(plain.vertices);
                    group
                        .opaque
                        .indices
                        .extend(plain.indices.iter().map(|i| i + base));
                    vec![(0, full), (full, plain.indices.len() as u32)]
                }
                _ => levels_of(&mut group.opaque, size),
            };
            if !group.instances.is_empty() {
                s.groups.push(group);
            }
        }
        let mut loose = Loose::default();
        let solids = archisim_geom::solid::model_solids_except_detailed(model, &skip, &detailed);
        let anchors = anchors(&solids);
        // The faces with painted zones (G3): their elements' solids.
        let zone_solids: Vec<archisim_geom::solid::Solid> = {
            let ids: std::collections::BTreeSet<u64> = model
                .face_zones()
                .filter(|z| z.material.is_some())
                .map(|z| z.element)
                .collect();
            solids
                .iter()
                .filter(|s| ids.contains(&s.owner.id()))
                .cloned()
                .collect()
        };
        // The detailed doors and windows are cut as their walls.
        for g in &mut s.groups {
            for (inst, owner) in g.instances.iter_mut().zip(&g.owners) {
                if let Element::Opening(id) = owner {
                    if let Some(a) = opening_walls.get(id).and_then(|w| anchors.get(w)) {
                        inst.cut = a.cut();
                    }
                }
            }
        }
        let look = Look {
            selection,
            isolate,
            disciplines,
            anchors: &anchors,
            shared: false,
            between: false,
        };
        add_solids(model, solids, &look, &mut loose);
        // Where the layers of a wall, a slab or a roof touch: seen only
        // through a cut.
        let idx = archisim_geom::WallIndex::new(model);
        let between: Vec<_> = model
            .walls()
            .flat_map(|w| archisim_geom::solid::wall_inner_faces(&idx, w.id))
            .chain(
                model
                    .slabs()
                    .flat_map(|sl| archisim_geom::solid::slab_inner_faces(&idx, sl.id)),
            )
            .chain(
                model
                    .roofs()
                    .flat_map(|r| archisim_geom::solid::roof_inner_faces(model, r.id)),
            )
            .collect();
        add_solids(
            model,
            between,
            &Look {
                between: true,
                ..look
            },
            &mut loose,
        );
        // Painted zones of walls' faces, just off them.
        for (wall, tri, [r, g, b]) in archisim_geom::solid::paint_zone_triangles(model) {
            if model.hidden_by_discipline(&disciplines.hidden, wall.0) {
                continue;
            }
            let owner = Owner::Model(Element::Wall(wall));
            let rgb = [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b)];
            let select = if selection.contains(&Element::Wall(wall)) {
                1.0
            } else {
                0.0
            };
            let cut = anchors
                .get(&wall)
                .map_or([cutaway::WALL, 0.0, 0.0, 0.0], |a| a.cut());
            let p = tri.map(|v| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32));
            let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
            let v = p.map(|v| {
                Vertex::new(
                    v,
                    n,
                    [rgb[0], rgb[1], rgb[2], 1.0],
                    -1.0,
                    [0.0, 0.0],
                    cut,
                    select,
                )
            });
            loose.push(Pass::Opaque, v, owner);
        }
        // Painted zones of the faces of slabs, footings… (G3), just off them.
        for (owner, tri, [r, g, b]) in
            archisim_geom::solid::face_zone_triangles(model, &zone_solids)
        {
            if model.hidden_by_discipline(&disciplines.hidden, owner.id()) {
                continue;
            }
            let element = crate::pick::of_solid(owner);
            let rgb = [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b)];
            let select = if selection.contains(&element) {
                1.0
            } else {
                0.0
            };
            let p = tri.map(|v| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32));
            let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
            let v = p.map(|v| {
                Vertex::new(
                    v,
                    n,
                    [rgb[0], rgb[1], rgb[2], 1.0],
                    -1.0,
                    [0.0, 0.0],
                    [cutaway::PLAIN, 0.0, 0.0, 0.0],
                    select,
                )
            });
            loose.push(Pass::Opaque, v, Owner::Model(element));
        }
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
            for tri in part.triangles {
                let p = tri.map(|v| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32));
                let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
                let v = p.map(|v| {
                    Vertex::new(
                        v,
                        n,
                        color,
                        0.0,
                        [0.0, 0.0],
                        [cutaway::PLAIN, 0.0, 0.0, 0.0],
                        0.0,
                    )
                });
                loose.push(pass, v, owner);
            }
        }
        let ([opaque, glass, grass], lamps) = loose.finish();
        s.lamps.extend(lamps);
        for c in opaque.iter().chain(&glass).chain(&grass) {
            s.lo = s.lo.min(c.lo);
            s.hi = s.hi.max(c.hi);
        }
        (s.opaque, s.glass, s.grass) = (opaque, glass, grass);
        if s.lo.x > s.hi.x {
            s.lo = Vec3::new(0.0, 0.0, 0.0);
            s.hi = Vec3::new(10.0, 10.0, 3.0);
        }
        s
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
            for v in &mut c.mesh.vertices {
                v.misc[1] = 2;
            }
            s.lo = s.lo.min(c.lo);
            s.hi = s.hi.max(c.hi);
        }
        (s.opaque, s.glass) = (opaque, glass);
        s
    }

    pub fn is_empty(&self) -> bool {
        self.opaque.is_empty() && self.glass.is_empty() && self.groups.is_empty()
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
            for (t, o) in c.mesh.indices.as_chunks::<3>().0.iter().zip(&c.owners) {
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
            for (t, o) in c.mesh.indices.as_chunks::<3>().0.iter().zip(&c.owners) {
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
            for (t, o) in c.mesh.indices.as_chunks::<3>().0.iter().zip(&c.owners) {
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
        let mut consider = |d: f32, tri: [Vec3; 3], owner: Owner, code: [f32; 4]| {
            let at = ray.origin + ray.dir * d;
            if clip.is_some_and(|bx| !bx.contains(at)) {
                return;
            }
            if cut.is_some_and(|r| at.z > r.top_of(code) + 1e-4) {
                return;
            }
            if best.as_ref().is_none_or(|b| d < b.distance) {
                best = Some(Hit {
                    distance: d,
                    triangle: tri,
                    owner,
                });
            }
        };
        for c in self.opaque.iter().chain(&self.glass) {
            if !ray_meets_box(ray, c.lo, c.hi) {
                continue;
            }
            for (t, o) in c.mesh.indices.as_chunks::<3>().0.iter().zip(&c.owners) {
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
        best
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
        let owner = Owner::Model(element(solid.owner));
        let mat = model.material(solid.material);
        let dimmed = !look.shared && model.dimmed(look.disciplines.focus, id);
        let selected = !look.shared && look.selection.contains(&element(solid.owner));
        let ghost = look.isolate && !selected;
        let is_glass = ghost || mat.is_some_and(|m| m.category == "glass");
        // Doors and windows win where they touch the reveals.
        let opening = matches!(solid.owner, SolidOwner::Opening(_));
        let alpha = if ghost {
            GHOST_ALPHA
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
        let (glows, reach) = match solid.owner {
            SolidOwner::Object(o) if !dimmed && !ghost => {
                let (mats, reach) = archisim_geom::solid::glow_materials(model, o);
                (mats.contains(&solid.material), reach as f32)
            }
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
                        .zip(&c.owners)
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
