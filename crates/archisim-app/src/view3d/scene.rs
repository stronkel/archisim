//! The model as triangles ready for the graphics card, and picking in 3D.

use std::collections::BTreeSet;

use archisim_core::Model;
use archisim_geom::solid::{model_solids, SolidOwner};
use bytemuck::{Pod, Zeroable};
use glam::Vec3;

use super::camera::Ray;
use super::cutaway::{self, CutRule};
use crate::pick::Element;

/// One corner of a triangle as the shader reads it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    /// Linear RGB and opacity.
    pub color: [f32; 4],
    /// Which face wins where two elements touch on a visible edge: faces
    /// looking up give way (+1), doors and windows win against the reveals
    /// they sit in (-1).
    pub bias: f32,
    /// The texture (0: none) and the size of its piece, metres.
    pub tex: [f32; 2],
    /// What the cut away walls do with its element (`cutaway`): its kind,
    /// a point of it in plan and its base.
    pub cut: [f32; 4],
    /// 1 if its element is selected: outlined (the contour pass), its
    /// colours left as they are.
    pub select: f32,
    /// Linear RGB of the material's cut colour (its poché in plan): what
    /// its cut shows when a section box or the cut away walls cut it.
    pub cap: [f32; 3],
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

/// Triangles of the whole model, split into opaque and see-through, with
/// the element each one belongs to.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub opaque: Vec<Vertex>,
    pub glass: Vec<Vertex>,
    /// The grass and flowers on painted ground: drawn after the opaque
    /// model without changing its normals and distances (no contour lines
    /// between the blades), not picked.
    pub grass: Vec<Vertex>,
    /// Element of each opaque triangle, then of each glass triangle.
    owners: Vec<Owner>,
    pub lo: Vec3,
    pub hi: Vec3,
    /// The lamps: where their light comes from (the middle of what glows)
    /// and how far it reaches, metres.
    pub lamps: Vec<[f32; 4]>,
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

fn element(owner: SolidOwner) -> Element {
    crate::pick::of_solid(owner)
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
        let mut glass_owners = Vec::new();
        let solids = model_solids(model);
        let anchors = anchors(&solids);
        s.add_solids(
            model,
            solids,
            &anchors,
            selection,
            isolate,
            disciplines,
            &mut glass_owners,
            false,
        );
        // Where the layers of a wall touch: seen only through a cut, so
        // that a cut wall shows each layer solid in its cut colour.
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
        s.add_solids(
            model,
            between,
            &anchors,
            selection,
            isolate,
            disciplines,
            &mut glass_owners,
            true,
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
            for v in p {
                s.opaque.push(Vertex {
                    pos: v.to_array(),
                    normal: n.to_array(),
                    color: [rgb[0], rgb[1], rgb[2], 1.0],
                    bias: -1.0,
                    tex: [0.0, 0.0],
                    cut,
                    select,
                    cap: rgb,
                });
            }
            s.owners.push(owner);
        }
        for part in linked {
            let [r, g, b] = part.color;
            let color = [
                srgb_to_linear(r),
                srgb_to_linear(g),
                srgb_to_linear(b),
                part.alpha,
            ];
            let see_through = part.alpha < 1.0;
            let owner = Owner::Linked(part.link, part.index);
            for tri in part.triangles {
                let p = tri.map(|v| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32));
                let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
                for v in p {
                    s.lo = s.lo.min(v);
                    s.hi = s.hi.max(v);
                }
                let target = if see_through {
                    &mut s.glass
                } else {
                    &mut s.opaque
                };
                for v in p {
                    target.push(Vertex {
                        pos: v.to_array(),
                        normal: n.to_array(),
                        color,
                        bias: 0.0,
                        tex: [0.0, 0.0],
                        cut: [cutaway::PLAIN, 0.0, 0.0, 0.0],
                        select: 0.0,
                        cap: [color[0], color[1], color[2]],
                    });
                }
                if see_through {
                    glass_owners.push(owner);
                } else {
                    s.owners.push(owner);
                }
            }
        }
        s.owners.extend(glass_owners);
        if s.opaque.is_empty() && s.glass.is_empty() {
            s.lo = Vec3::new(0.0, 0.0, 0.0);
            s.hi = Vec3::new(10.0, 10.0, 3.0);
        }
        s
    }

    /// Adds solids of the model, each triangle with its owner (glass
    /// ones in `glass_owners`, to go after the opaque ones).
    #[allow(clippy::too_many_arguments)]
    fn add_solids(
        &mut self,
        model: &Model,
        solids: Vec<archisim_geom::solid::Solid>,
        anchors: &std::collections::HashMap<archisim_core::WallId, Anchor>,
        selection: &BTreeSet<Element>,
        isolate: bool,
        disciplines: &Disciplines,
        glass_owners: &mut Vec<Owner>,
        between: bool,
    ) {
        let s = self;
        // What glows of each object, for its light.
        let mut lit: std::collections::BTreeMap<u64, (Vec3, f32, f32)> = Default::default();
        // The stuffed fabric of each object.
        let mut fabric_of: std::collections::HashMap<
            archisim_core::ObjectId,
            Vec<archisim_core::MaterialId>,
        > = Default::default();
        for solid in solids {
            let id = solid.owner.id();
            if model.hidden_by_discipline(&disciplines.hidden, id) {
                continue;
            }
            let owner = Owner::Model(element(solid.owner));
            let mat = model.material(solid.material);
            let dimmed = model.dimmed(disciplines.focus, id);
            let selected = selection.contains(&element(solid.owner));
            let ghost = isolate && !selected;
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
            // rounded (normals smoothed over each clump) and without
            // contour lines between their facets.
            let soft = match solid.owner {
                SolidOwner::Object(o) => model
                    .plant_look(o)
                    .is_some_and(|l| solid.material != l.materials.bark),
                // Tiles too: hundreds of covers would otherwise be drawn
                // as as many contour lines and darken the roof from afar.
                SolidOwner::Roof(r) => model.roof(r).is_some_and(|r| {
                    r.covering.tiles.is_some() && model.tile_material(r) == Some(solid.material)
                }),
                _ => false,
            };
            // Stuffed fabric (the workshop's upholstery): it has no sharp
            // edge, so it is shaded smooth all over, and outlined only
            // where it ends against what is behind, never at its own seams
            // and folds (drawn as foliage is, bias 2).
            let fabric = match solid.owner {
                SolidOwner::Object(o) if !soft => fabric_of
                    .entry(o)
                    .or_insert_with(|| model.object_soft_materials(o))
                    .contains(&solid.material),
                _ => false,
            };
            let smooth: Option<std::collections::HashMap<[i64; 3], Vec3>> =
                (soft || fabric).then(|| {
                    let mut acc: std::collections::HashMap<[i64; 3], Vec3> = Default::default();
                    for tri in &solid.triangles {
                        let p = tri.map(|v| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32));
                        let n = (p[1] - p[0]).cross(p[2] - p[0]);
                        for v in tri {
                            *acc.entry(vertex_key(v)).or_default() += n;
                        }
                    }
                    acc
                });
            // Furniture: rounded edges and cushions shaded smooth (the
            // faces of each corner that turn less than ~40° share their
            // normal), so they are drawn soft, without a contour line at
            // every facet of a rounding; real edges stay sharp.
            let creased: Option<std::collections::HashMap<[i64; 3], Vec<Vec3>>> =
                (!soft && !fabric && matches!(solid.owner, SolidOwner::Object(_))).then(|| {
                    let mut acc: std::collections::HashMap<[i64; 3], Vec<Vec3>> =
                        Default::default();
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
            // A lampshade, a bulb or a flame: it glows at night, and its
            // middle is where the lamp's light comes from.
            let (glows, reach) = match solid.owner {
                SolidOwner::Object(o) if !dimmed && !ghost => {
                    let (mats, reach) = archisim_geom::solid::glow_materials(model, o);
                    (mats.contains(&solid.material), reach as f32)
                }
                _ => (false, 0.0),
            };
            if glows {
                let (sum, n, r) = lit.entry(id).or_insert((Vec3::ZERO, 0.0, reach));
                *r = reach;
                for tri in &solid.triangles {
                    for v in tri {
                        *sum += Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
                        *n += 1.0;
                    }
                }
            }
            let base = tone(archisim_geom::solid::solid_color(model, &solid), dimmed);
            let cut = cut_of(model, anchors, &solid);
            // The cut of a solid shows its material's cut colour, as the
            // plan's poché (each layer of a wall its own).
            let cap = {
                let c = model
                    .material(solid.material)
                    .map_or([200, 200, 200], |m| m.cut_color);
                let t = tone(c, dimmed);
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
            let own_tex = if dimmed || ghost || grass {
                [0.0, 0.0]
            } else {
                texture_of(solid.material)
            };
            // A model brought from another program: the colours of its
            // vertices (its textures).
            let corners = archisim_geom::solid::solid_vertex_colors(model, &solid);
            for (i, tri) in solid.triangles.iter().enumerate() {
                // A painted face: its paint, never greyed (it is what is
                // being worked on).
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
                // -2: a face between two layers, drawn only as the cut.
                let bias = if between {
                    -2.0
                } else if grass {
                    4.0
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
                for v in p {
                    s.lo = s.lo.min(v);
                    s.hi = s.hi.max(v);
                }
                let target = if grass {
                    &mut s.grass
                } else if is_glass {
                    &mut s.glass
                } else {
                    &mut s.opaque
                };
                for (k, (v, color)) in p.into_iter().zip(colors).enumerate() {
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
                    target.push(Vertex {
                        pos: v.to_array(),
                        normal: normal.to_array(),
                        color,
                        bias,
                        tex,
                        cut,
                        select,
                        cap,
                    });
                }
                if grass {
                    // Not picked: the terrain under it is.
                } else if is_glass {
                    glass_owners.push(owner);
                } else {
                    s.owners.push(owner);
                }
            }
        }
        for (sum, n, reach) in lit.into_values() {
            if n > 0.0 {
                let c = sum / n;
                s.lamps.push([c.x, c.y, c.z, reach]);
            }
        }
    }

    /// A scene of a few solids only: what is being stretched, drawn over
    /// the scene while the handle is dragged (its element is marked
    /// selected, `Vertex::select` 2, so the scene's own copy is hidden).
    pub fn of_solids(model: &Model, solids: Vec<archisim_geom::solid::Solid>) -> Self {
        let mut s = Scene {
            lo: Vec3::splat(f32::INFINITY),
            hi: Vec3::splat(f32::NEG_INFINITY),
            ..Default::default()
        };
        let mut glass_owners = Vec::new();
        let selection: BTreeSet<Element> = solids.iter().map(|x| element(x.owner)).collect();
        s.add_solids(
            model,
            solids,
            &Default::default(),
            &selection,
            false,
            &Default::default(),
            &mut glass_owners,
            false,
        );
        s.owners.extend(glass_owners);
        for v in s.opaque.iter_mut().chain(&mut s.glass) {
            v.select = 2.0;
        }
        s
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.opaque.is_empty() && self.glass.is_empty()
    }

    /// The box around the triangles of each element of the project.
    pub fn element_bounds(&self) -> std::collections::BTreeMap<Element, (Vec3, Vec3)> {
        let tris = self
            .opaque
            .as_chunks::<3>()
            .0
            .iter()
            .chain(self.glass.as_chunks::<3>().0);
        let mut out = std::collections::BTreeMap::new();
        for (i, t) in tris.enumerate() {
            let Owner::Model(e) = self.owners[i] else {
                continue;
            };
            for v in t {
                let p = Vec3::from_array(v.pos);
                out.entry(e)
                    .and_modify(|b: &mut (Vec3, Vec3)| *b = (b.0.min(p), b.1.max(p)))
                    .or_insert((p, p));
            }
        }
        out
    }

    /// The box around the triangles of some elements.
    pub fn bounds_of(&self, elements: &BTreeSet<Element>) -> Option<(Vec3, Vec3)> {
        let tris = self
            .opaque
            .as_chunks::<3>()
            .0
            .iter()
            .chain(self.glass.as_chunks::<3>().0);
        let mut b: Option<(Vec3, Vec3)> = None;
        for (i, t) in tris.enumerate() {
            if !matches!(self.owners[i], Owner::Model(e) if elements.contains(&e)) {
                continue;
            }
            for v in t {
                let p = Vec3::from_array(v.pos);
                b = Some(b.map_or((p, p), |(lo, hi)| (lo.min(p), hi.max(p))));
            }
        }
        b
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
        self.hit_in(ray, clip, cut).map(|(_, i)| self.owners[i])
    }

    /// The nearest triangle hit by a ray (inside the section box): its
    /// distance along the ray and its index.
    pub fn hit_in(
        &self,
        ray: Ray,
        clip: Option<&super::section_box::SectionBox>,
        cut: Option<&CutRule>,
    ) -> Option<(f32, usize)> {
        let mut best: Option<(f32, usize)> = None;
        let firsts = self
            .opaque
            .as_chunks::<3>()
            .0
            .iter()
            .chain(self.glass.as_chunks::<3>().0)
            .map(|t| t[0].cut);
        for (i, ([a, b, c], code)) in self.triangles().zip(firsts).enumerate() {
            if let Some(d) = hit(ray, a, b, c) {
                let at = ray.origin + ray.dir * d;
                if clip.is_some_and(|bx| !bx.contains(at)) {
                    continue;
                }
                if cut.is_some_and(|r| at.z > r.top_of(code) + 1e-4) {
                    continue;
                }
                if best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, i));
                }
            }
        }
        best
    }

    /// The cut faces of the objects a section box from `lo` to `hi` cuts
    /// (furniture, appliances, people, cars): on each face of the box that
    /// goes through one, its section filled whole, in the cut colour of
    /// the parts it was cut from, so that a kitchen unit reads as a solid
    /// block and not as the empty box of its panels. Leaves are left as
    /// they are (their back faces show the cut).
    pub fn object_cut_fill(&self, lo: Vec3, hi: Vec3) -> Vec<Vertex> {
        use archisim_geom::slice::{filled_section, Plane};
        struct Part {
            tris: Vec<([[f64; 3]; 3], [u32; 3])>,
            lo: Vec3,
            hi: Vec3,
            cut: [f32; 4],
            select: f32,
        }
        let mut parts: Vec<(Element, Part)> = Vec::new();
        for (t, owner) in self.opaque.as_chunks::<3>().0.iter().zip(&self.owners) {
            let Owner::Model(e @ Element::Object(_)) = *owner else {
                continue;
            };
            // Leaves, what glows and the grass keep their own look.
            if t[0].bias > 0.5 {
                continue;
            }
            let i = match parts.iter().position(|(o, _)| *o == e) {
                Some(i) => i,
                None => {
                    parts.push((
                        e,
                        Part {
                            tris: Vec::new(),
                            lo: Vec3::splat(f32::MAX),
                            hi: Vec3::splat(f32::MIN),
                            cut: t[0].cut,
                            select: t[0].select,
                        },
                    ));
                    parts.len() - 1
                }
            };
            let part = &mut parts[i].1;
            for v in t {
                part.lo = part.lo.min(Vec3::from_array(v.pos));
                part.hi = part.hi.max(Vec3::from_array(v.pos));
            }
            part.tris
                .push((t.map(|v| v.pos.map(f64::from)), t[0].cap.map(f32::to_bits)));
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
        let mut out = Vec::new();
        for (_, part) in &parts {
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
                        for k in [a, b, c] {
                            out.push(Vertex {
                                pos: at3(region.outline[k]).to_array(),
                                normal: normal.to_array(),
                                color: [cap[0], cap[1], cap[2], 1.0],
                                // A cut face: shaded as the caps are.
                                bias: -3.0,
                                tex: [0.0, 0.0],
                                cut: part.cut,
                                select: part.select,
                                cap,
                            });
                        }
                    }
                }
            }
        }
        out
    }

    /// Every triangle, opaque then glass, as points.
    pub fn triangles(&self) -> impl Iterator<Item = [Vec3; 3]> + '_ {
        self.opaque
            .as_chunks::<3>()
            .0
            .iter()
            .chain(self.glass.as_chunks::<3>().0)
            .map(|t| [t[0].pos, t[1].pos, t[2].pos].map(Vec3::from_array))
    }

    /// What a triangle belongs to.
    pub fn owner(&self, i: usize) -> Option<Owner> {
        self.owners.get(i).copied()
    }

    /// The triangles of one element.
    pub fn triangles_of(&self, owner: Owner) -> impl Iterator<Item = [Vec3; 3]> + '_ {
        self.triangles()
            .zip(&self.owners)
            .filter(move |(_, o)| **o == owner)
            .map(|(t, _)| t)
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
        SolidOwner::Wall(w) => walls.get(&w).map_or(own(cutaway::WALL), |a| a.cut()),
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
        assert_eq!(s.opaque.len() % 3, 0);
        assert_eq!(s.owners.len(), (s.opaque.len() + s.glass.len()) / 3);
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
        let before = s.opaque.clone();
        let mut sel = BTreeSet::new();
        sel.insert(Element::Wall(west));
        s = Scene::build(&m, &sel);
        let marked = s.opaque.iter().filter(|v| v.select > 0.5).count();
        assert!(marked > 0 && marked < before.len());
        assert!(before
            .iter()
            .zip(&s.opaque)
            .all(|(a, b)| a.color == b.color));

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
                .chunks(3)
                .zip(&s.owners)
                .find(|(_, o)| **o == Owner::Model(e))
                .map(|(t, _)| t[0].cut)
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
