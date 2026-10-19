//! The model as triangles ready for the graphics card, and picking in 3D.

use std::collections::BTreeSet;

use archisim_core::Model;
use archisim_geom::solid::{model_solids, SolidOwner};
use bytemuck::{Pod, Zeroable};
use glam::Vec3;

use super::camera::Ray;
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
}

/// Selected elements are tinted with this colour (linear RGB).
const HIGHLIGHT: [f32; 3] = [0.05, 0.35, 0.85];
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
    /// Element of each opaque triangle, then of each glass triangle.
    owners: Vec<Owner>,
    pub lo: Vec3,
    pub hi: Vec3,
}

fn srgb_to_linear(c: u8) -> f32 {
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
        Self::build_with(model, selection, &[], false)
    }

    /// The model and, after it, the elements of linked models. With
    /// `isolate`, only the selection keeps its colour: the rest of the
    /// project is a faint ghost, to see a clash through walls and roofs.
    pub fn build_with(
        model: &Model,
        selection: &BTreeSet<Element>,
        linked: &[LinkedPart],
        isolate: bool,
    ) -> Self {
        let mut s = Scene {
            lo: Vec3::splat(f32::INFINITY),
            hi: Vec3::splat(f32::NEG_INFINITY),
            ..Default::default()
        };
        let mut glass_owners = Vec::new();
        for solid in model_solids(model) {
            let owner = Owner::Model(element(solid.owner));
            let mat = model.material(solid.material);
            let [r, g, b] = mat.map_or([200, 200, 200], |m| m.cut_color);
            let mut rgb = [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b)];
            if selection.contains(&element(solid.owner)) {
                for (c, h) in rgb.iter_mut().zip(HIGHLIGHT) {
                    *c = *c * 0.45 + h * 0.55;
                }
            }
            let ghost = isolate && !selection.contains(&element(solid.owner));
            let is_glass = ghost || mat.is_some_and(|m| m.category == "glass");
            // Doors and windows win where they touch the reveals.
            let opening = matches!(solid.owner, SolidOwner::Opening(_));
            let color = [
                rgb[0],
                rgb[1],
                rgb[2],
                if ghost {
                    GHOST_ALPHA
                } else if is_glass {
                    GLASS_ALPHA
                } else {
                    1.0
                },
            ];
            for tri in &solid.triangles {
                let p = tri.map(|v| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32));
                let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
                // Faces looking up give way where another element rests on
                // them: the floor under a wall, the top of a wall under the
                // roof slab.
                let bias = if opening {
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
                let target = if is_glass {
                    &mut s.glass
                } else {
                    &mut s.opaque
                };
                for v in p {
                    target.push(Vertex {
                        pos: v.to_array(),
                        normal: n.to_array(),
                        color,
                        bias,
                    });
                }
                if is_glass {
                    glass_owners.push(owner);
                } else {
                    s.owners.push(owner);
                }
            }
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

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.opaque.is_empty() && self.glass.is_empty()
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
    ) -> Option<Owner> {
        let tris = self
            .opaque
            .as_chunks::<3>()
            .0
            .iter()
            .chain(self.glass.as_chunks::<3>().0);
        let mut best: Option<(f32, Owner)> = None;
        for (i, t) in tris.enumerate() {
            let [a, b, c] = [t[0].pos, t[1].pos, t[2].pos].map(Vec3::from_array);
            if let Some(d) = hit(ray, a, b, c) {
                if clip.is_some_and(|bx| !bx.contains(ray.origin + ray.dir * d)) {
                    continue;
                }
                if best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, self.owners[i]));
                }
            }
        }
        best.map(|(_, e)| e)
    }

    /// The nearest element of the project hit by a ray (none if a linked
    /// element is in front).
    #[cfg(test)]
    pub fn pick(&self, ray: Ray) -> Option<Element> {
        match self.pick_in(ray, None)? {
            Owner::Model(e) => Some(e),
            Owner::Linked(..) => None,
        }
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

        // Selecting tints the selected element and nothing else.
        let before = s.opaque.clone();
        let mut sel = BTreeSet::new();
        sel.insert(Element::Wall(west));
        s = Scene::build(&m, &sel);
        let changed = before
            .iter()
            .zip(&s.opaque)
            .filter(|(a, b)| a.color != b.color)
            .count();
        assert!(changed > 0 && changed < before.len());
    }
}
