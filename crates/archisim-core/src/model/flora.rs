//! Plants grown by ArchiSim (crate `archisim-flora`): objects whose family
//! says `generator = "flora"` in its outputs (the system family
//! «Vegetación»). Their type is a **species** (how it grows, its colours
//! and how it is drawn); each object is **one plant** of it, grown from
//! its seed, at its own size if it has one. Their 3D and their plan
//! symbol come from the grown plant, not from the recipe's body, which
//! only gives their bulk (selection, gallery, IFC fallback).

use super::*;
use crate::family::{LineStyle, Pen};
use archisim_flora as flora;
use std::sync::Arc;

/// The system family of generated plants.
pub const VEGETATION_FAMILY: &str = "Vegetación";

/// The materials of a plant, for its 3D and its plan colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlantMaterials {
    pub leaves: MaterialId,
    pub bark: MaterialId,
    pub flowers: MaterialId,
    pub fruit: MaterialId,
}

/// Everything that makes one plant: its species at its size, its seed,
/// how much it may differ, its colours and how it is drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct PlantLook {
    pub species: flora::Species,
    pub seed: u64,
    pub variation: f64,
    pub materials: PlantMaterials,
    pub colors: flora::Colors,
    pub technical: flora::PlanStyle,
    pub presentation: flora::PlanStyle,
    pub look: flora::Look,
    pub opacity: f64,
}

fn num(v: &Values, k: &str, d: f64) -> f64 {
    v.get(k).and_then(|x| x.as_number()).unwrap_or(d)
}

fn text<'a>(v: &'a Values, k: &str) -> &'a str {
    v.get(k).and_then(|x| x.as_text()).unwrap_or("")
}

/// The species of a type's values.
pub fn species_of(v: &Values) -> flora::Species {
    let d = flora::Species::default();
    flora::Species {
        habit: flora::Habit::from_name(text(v, "habit")).unwrap_or(d.habit),
        crown: flora::CrownShape::from_name(text(v, "crown_shape")).unwrap_or(d.crown),
        foliage: flora::Foliage::from_name(text(v, "foliage")).unwrap_or(d.foliage),
        height: num(v, "height", d.height),
        width: num(v, "crown", d.width),
        trunk: num(v, "trunk", d.trunk),
        clear: num(v, "clear", d.clear),
        stems: num(v, "stems", d.stems as f64).round().max(1.0) as u32,
        lean: num(v, "lean", d.lean),
        gnarl: num(v, "gnarl", d.gnarl),
        limbs: num(v, "limbs", d.limbs as f64).round().max(1.0) as u32,
        levels: num(v, "levels", d.levels as f64).round().max(1.0) as u32,
        angle: num(v, "angle", d.angle),
        droop: num(v, "droop", d.droop),
        density: num(v, "density", d.density),
        clump: num(v, "clump", d.clump),
        bloom: num(v, "bloom", d.bloom),
        fruit: num(v, "fruit", d.fruit),
        fronds: num(v, "fronds", d.fronds as f64).round().max(1.0) as u32,
        frond_length: num(v, "frond_length", d.frond_length),
    }
}

/// A number that stays with an object: from its `GlobalId`, so a copy
/// grows a different plant and the same object always the same one.
fn seed_of_guid(guid: &GlobalId) -> u64 {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for b in guid.to_string().bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01B3);
    }
    h
}

impl Model {
    /// Whether a family grows plants (its outputs say `generator =
    /// "flora"`).
    pub fn family_grows(&self, family: FamilyId) -> bool {
        self.family_generator(family).as_deref() == Some("flora")
    }

    /// Whether objects of a type are grown plants.
    pub fn type_grows(&self, t: ObjectTypeId) -> bool {
        self.object_types
            .get(&t)
            .is_some_and(|t| self.family_grows(t.family))
    }

    /// The values of a type, over its family's defaults.
    fn type_values_full(&self, t: ObjectTypeId) -> Option<Values> {
        let ty = self.object_types.get(&t)?;
        let mut v = self.family_defaults(ty.family, ParamScope::Type).ok()?;
        v.extend(ty.values.clone());
        Some(v)
    }

    fn color_of(&self, m: Option<u64>, fallback: [u8; 3]) -> (MaterialId, [u8; 3]) {
        match m.and_then(|m| self.materials.get(&MaterialId(m))) {
            Some(mat) => (mat.id, mat.cut_color),
            None => (MaterialId(m.unwrap_or(0)), fallback),
        }
    }

    /// How a plant of a type grows and is drawn, with these instance
    /// values (its own seed, size, drawing…).
    pub fn plant_look_of(
        &self,
        t: ObjectTypeId,
        inst: &Values,
        default_seed: u64,
    ) -> Option<PlantLook> {
        if !self.type_grows(t) {
            return None;
        }
        let tv = self.type_values_full(t)?;
        let mut species = species_of(&tv);
        let h = num(inst, "height_own", 0.0);
        let w = num(inst, "crown_own", 0.0);
        if h > 0.0 || w > 0.0 {
            let h = if h > 0.0 { h } else { species.height };
            let w = if w > 0.0 { w } else { species.width };
            species = species.sized(h, w);
        }
        let seed = match num(inst, "plant_seed", 0.0) {
            s if s >= 1.0 => s as u64,
            _ => default_seed,
        };
        let d = flora::Colors::default();
        let mat = |k: &str| tv.get(k).and_then(|x| x.as_material());
        let own_leaves = inst
            .get("color_own")
            .and_then(|x| x.as_bool())
            .unwrap_or(false)
            .then(|| inst.get("leaves_own").and_then(|x| x.as_material()))
            .flatten();
        let (leaves, leaves_c) = self.color_of(own_leaves.or(mat("leaves")), d.leaves);
        let (bark, bark_c) = self.color_of(mat("bark"), d.bark);
        let (flowers, flowers_c) = self.color_of(mat("flowers"), d.flowers);
        let (fruit, fruit_c) = self.color_of(mat("fruits"), d.fruit);
        let style =
            |k: &str, d: flora::PlanStyle| flora::PlanStyle::from_name(text(&tv, k)).unwrap_or(d);
        let mut presentation = style("style", flora::PlanStyle::Outline);
        if let Some(s) = flora::PlanStyle::from_name(text(inst, "style_own")) {
            presentation = s;
        }
        let mut look = flora::Look::from_name(text(&tv, "look")).unwrap_or(flora::Look::SeeThrough);
        if let Some(l) = flora::Look::from_name(text(inst, "look_own")) {
            look = l;
        }
        let mut opacity = num(&tv, "opacity", 0.55);
        let own_op = num(inst, "opacity_own", 0.0);
        if own_op > 0.0 {
            opacity = own_op;
        }
        Some(PlantLook {
            species,
            seed,
            variation: num(inst, "variation", 0.5),
            materials: PlantMaterials {
                leaves,
                bark,
                flowers,
                fruit,
            },
            colors: flora::Colors {
                leaves: leaves_c,
                bark: bark_c,
                flowers: flowers_c,
                fruit: fruit_c,
            },
            technical: style("tech_style", flora::PlanStyle::Circle),
            presentation,
            look,
            opacity,
        })
    }

    /// How a placed plant grows and is drawn.
    pub fn plant_look(&self, id: ObjectId) -> Option<PlantLook> {
        let o = self.objects.get(&id)?;
        self.plant_look_of(
            o.object_type,
            &self.object_values(id),
            seed_of_guid(&o.guid),
        )
    }

    /// The grown plant of an object (kept: growing it again is free).
    pub fn object_plant(&self, id: ObjectId) -> Option<Arc<flora::Plant>> {
        let l = self.plant_look(id)?;
        Some(flora::plant(&l.species, l.seed, l.variation))
    }

    /// The plan symbol of a plant, in the family's axes: of presentation
    /// or technical, at a drawing scale (1:`scale`).
    pub fn plant_symbol(&self, id: ObjectId, presenting: bool, scale: f64) -> Option<Vec<Mark>> {
        let l = self.plant_look(id)?;
        Some(self.plant_marks(&l, presenting, scale))
    }

    /// The marks of a plant of this look.
    pub fn plant_marks(&self, l: &PlantLook, presenting: bool, scale: f64) -> Vec<Mark> {
        let plant = flora::plant(&l.species, l.seed, l.variation);
        let crown_mm = (plant.radius * 2.0).max(l.species.width * 0.3) * 1000.0 / scale.max(1.0);
        let options = if presenting {
            flora::PlanOptions {
                style: l.presentation,
                look: l.look,
                opacity: l.opacity,
                crown_mm,
            }
        } else {
            flora::PlanOptions {
                style: l.technical,
                look: flora::Look::Lines,
                opacity: l.opacity,
                crown_mm,
            }
        };
        plant_shape_marks(l, plant.plan(&options))
    }

    /// A placed plant seen from the side, looking along `toward` (level,
    /// in the project's axes), for elevations and sections: of
    /// presentation or technical, at a drawing scale (1:`scale`). Each
    /// point is `[across, up]` in metres from its foot, `across` to the
    /// viewer's right.
    pub fn plant_side_marks(
        &self,
        id: ObjectId,
        toward: [f64; 2],
        presenting: bool,
        scale: f64,
    ) -> Option<Vec<Mark>> {
        let l = self.plant_look(id)?;
        let o = self.objects.get(&id)?;
        // The view in the plant's own axes (it is turned with its object).
        let (sn, cs) = (-o.rotation).sin_cos();
        let local = [
            toward[0] * cs - toward[1] * sn,
            toward[0] * sn + toward[1] * cs,
        ];
        let options = flora::SideOptions {
            style: if presenting {
                l.presentation
            } else {
                l.technical
            },
            look: if presenting {
                l.look
            } else {
                flora::Look::Lines
            },
            opacity: l.opacity,
            m_per_mm: scale.max(1.0) / 1000.0,
        };
        let side = flora::side_of(&l.species, l.seed, l.variation, local, &options);
        Some(plant_shape_marks(&l, side.as_ref().clone()))
    }

    /// The level of detail the views take for the plants: the most
    /// detailed with which all the plants of the project stay within the
    /// triangles of [`MESH_BUDGET`].
    pub fn plant_detail(&self) -> flora::Lod {
        let n = self
            .objects
            .values()
            .filter(|o| self.type_grows(o.object_type))
            .count();
        // Triangles of a plant, roughly, at each level.
        if n * 20_000 <= MESH_BUDGET {
            flora::Lod::Full
        } else if n * 3_000 <= MESH_BUDGET {
            flora::Lod::Medium
        } else {
            flora::Lod::Low
        }
    }

    /// The triangles of a plant at a level of detail: its mesh (corners in
    /// the family's axes, metres from its foot) and its triangles by
    /// material, each material once.
    #[allow(clippy::type_complexity)]
    pub fn plant_mesh(
        &self,
        id: ObjectId,
        lod: flora::Lod,
    ) -> Option<(Arc<flora::FloraMesh>, Vec<(MaterialId, Vec<[u32; 3]>)>)> {
        let l = self.plant_look(id)?;
        Some(self.plant_mesh_of(&l, lod))
    }

    #[allow(clippy::type_complexity)]
    pub fn plant_mesh_of(
        &self,
        l: &PlantLook,
        lod: flora::Lod,
    ) -> (Arc<flora::FloraMesh>, Vec<(MaterialId, Vec<[u32; 3]>)>) {
        let mesh = flora::mesh_of(&l.species, l.seed, l.variation, &l.colors, lod);
        let mut parts: Vec<(MaterialId, Vec<[u32; 3]>)> = Vec::new();
        for (m, tris) in [
            (l.materials.bark, &mesh.bark),
            (l.materials.leaves, &mesh.leaves),
            (l.materials.flowers, &mesh.flowers),
        ] {
            if tris.is_empty() {
                continue;
            }
            match parts.iter_mut().find(|(x, _)| *x == m) {
                Some((_, v)) => v.extend(tris.iter().copied()),
                None => parts.push((m, tris.clone())),
            }
        }
        (mesh, parts)
    }
}

/// Plant shapes as marks: fills tinted with the plant's materials (the
/// paper's white untinted), lines with the view pen (outline) or thin.
fn plant_shape_marks(l: &PlantLook, shapes: Vec<flora::Shape>) -> Vec<Mark> {
    let tint = |tone: flora::Tone| match tone {
        flora::Tone::Leaves => Some(l.materials.leaves.0),
        flora::Tone::Bark => Some(l.materials.bark.0),
        flora::Tone::Flowers => Some(l.materials.flowers.0),
        flora::Tone::Fruit => Some(l.materials.fruit.0),
        flora::Tone::Paper => None,
        flora::Tone::Ink => Some(crate::family::INK_TINT),
    };
    let mut marks = Vec::new();
    for s in shapes {
        match s {
            flora::Shape::Fill {
                points,
                tone,
                opacity,
            } => marks.push(Mark::Polygon {
                points,
                pen: Pen::None,
                style: LineStyle::Continuous,
                filled: true,
                tint: tint(tone),
                opacity,
            }),

            flora::Shape::Line {
                points,
                closed,
                stroke,
            } => {
                let pen = match stroke {
                    flora::Stroke::Outline => Pen::View,
                    flora::Stroke::Thin => Pen::Thin,
                };
                if closed && points.len() >= 3 {
                    marks.push(Mark::Polygon {
                        points,
                        pen,
                        style: LineStyle::Continuous,
                        filled: false,
                        tint: None,
                        opacity: 1.0,
                    });
                } else {
                    for w in points.windows(2) {
                        marks.push(Mark::Line {
                            from: w[0],
                            to: w[1],
                            pen,
                            style: LineStyle::Continuous,
                        });
                    }
                }
            }
        }
    }
    marks
}
