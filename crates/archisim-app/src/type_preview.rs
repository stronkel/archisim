//! Previews in the types and materials dialog, drawn by the same plan
//! writer as the plan view and the exported plan: a short piece of wall of
//! the type being edited, cut in plan at 1:20; a slab type's layers as the
//! same band seen in section; a door or window in a piece of wall. What the
//! preview shows is what the plan will show.

use archisim_core::{
    DoorOperation, Layer, LayerFunction, LevelId, MaterialId, Model, NewOpening, NewWall,
    OpeningKind, OpeningType, Point2, ReferenceLine, WallFunction, WallSide, WindowOperation,
};
use archisim_plan::PlanOptions;
use eframe::egui::ColorImage;

use crate::plan_image::rasterize_with;

/// Scale of the previews: a construction detail.
pub const PREVIEW_SCALE: f64 = 20.0;

/// Pictures of objects: the longer side in points, and the drawing density
/// (screen points per metre) the picture asked for is measured against.
const OBJECT_PICTURE: f64 = 150.0;
const OBJECT_POINTS_PER_M: f64 = 460.0;

/// A model with one level and one straight wall from (0, 0) to
/// (`length`, 0), the exterior on top, built from `layers` with the
/// materials of `source`. `None` if the layers are not valid.
pub fn layers_model(source: &Model, layers: &[Layer], length: f64) -> Option<(Model, LevelId)> {
    let mut m = Model::new("preview");
    let level = m.add_level("preview", 0.0);
    let mapped = copy_layers(source, &mut m, layers)?;
    let t = m
        .add_wall_type("preview", WallFunction::Exterior, mapped)
        .ok()?;
    m.add_wall(NewWall {
        wall_type: t,
        level,
        start: Point2::new(0.0, 0.0),
        end: Point2::new(length, 0.0),
        reference: ReferenceLine::Center,
        height: 3.0,
        base_offset: 0.0,
        bulge: 0.0,
        top: archisim_core::WallTop::Unconnected,
    })
    .ok()?;
    Some((m, level))
}

/// The layers with the materials of `source` copied into `m` (each once).
fn copy_layers(source: &Model, m: &mut Model, layers: &[Layer]) -> Option<Vec<Layer>> {
    let mut mapped = Vec::new();
    let mut ids: Vec<(MaterialId, MaterialId)> = Vec::new();
    for l in layers {
        let id = match ids.iter().find(|(from, _)| *from == l.material) {
            Some((_, to)) => *to,
            None => {
                let mat = source.material(l.material)?;
                let to = m.add_material(
                    mat.name.clone(),
                    mat.category.clone(),
                    mat.cut_color,
                    mat.cut_pattern,
                );
                ids.push((l.material, to));
                to
            }
        };
        mapped.push(Layer {
            material: id,
            ..l.clone()
        });
    }
    Some(mapped)
}

/// A piece of wall of the layers, 1.2 m long and high, for the 3D preview.
pub fn wall_piece_model(source: &Model, layers: &[Layer]) -> Option<Model> {
    let (mut m, _) = layers_model(source, layers, 1.2)?;
    let wall = m.walls().next()?.id;
    m.set_wall_height(wall, 1.2, 0.0).ok()?;
    Some(m)
}

/// A square of slab of the layers, 1.2 m wide, for the 3D preview.
pub fn slab_piece_model(source: &Model, layers: &[Layer]) -> Option<Model> {
    let mut m = Model::new("preview");
    let level = m.add_level("preview", 0.0);
    let mapped = copy_layers(source, &mut m, layers)?;
    let t = m
        .add_slab_type("preview", archisim_core::SlabFunction::Floor, mapped)
        .ok()?;
    let s = 1.2;
    m.add_slab(archisim_core::NewSlab {
        slab_type: t,
        level,
        outline: vec![
            Point2::new(0.0, 0.0),
            Point2::new(s, 0.0),
            Point2::new(s, s),
            Point2::new(0.0, s),
        ],
        top_offset: 0.0,
    })
    .ok()?;
    Some(m)
}

/// A model with a door or window of type `t` in the middle of a wall that
/// can hold its frame: the project's first wall type of the right kind
/// (façade for windows, partition for doors) if it is deep enough, or a
/// plain wall otherwise.
pub fn opening_model(source: &Model, t: &OpeningType) -> Option<(Model, LevelId)> {
    let wanted = if t.is_door() {
        WallFunction::Interior
    } else {
        WallFunction::Exterior
    };
    let host = source
        .wall_types()
        .filter(|w| w.thickness() >= t.frame_depth + 0.01)
        .min_by_key(|w| w.function != wanted);
    let layers = match host {
        Some(w) => w.layers.clone(),
        None => vec![Layer {
            material: t.frame_material,
            thickness: t.frame_depth + 0.1,
            function: LayerFunction::Structure,
        }],
    };
    let length = t.width + 0.6;
    let (mut m, level) = layers_model(source, &layers, length)?;
    // The same family and values, with the materials copied over.
    let mut copied: std::collections::HashMap<u64, MaterialId> = Default::default();
    let mut values = t.values.clone();
    for v in values.values_mut() {
        if let archisim_core::Value::Material(id) = v {
            let new = match copied.get(id) {
                Some(n) => *n,
                None => {
                    let mat = source.material(MaterialId(*id))?.clone();
                    let n = m.add_material(mat.name, mat.category, mat.cut_color, mat.cut_pattern);
                    // Its colour and texture in 3D too.
                    let _ = m.set_appearance(n, source.appearance(MaterialId(*id)));
                    copied.insert(*id, n);
                    n
                }
            };
            *v = archisim_core::Value::Material(new.0);
        }
    }
    let family = m.add_family(&source.family(t.family)?.source).ok()?;
    let ot = m
        .add_family_type(archisim_core::NewFamilyType {
            family,
            name: t.name.clone(),
            mark: Some(t.mark.clone()),
            description: String::new(),
            values,
        })
        .ok()?;
    let wall = m.walls().next()?.id;
    // Cut by the plan's cut plane (1.10 m): doors from the floor, windows
    // at a usual sill unless they are too low to reach it.
    let sill = if t.is_door() {
        0.0
    } else if t.height > 0.25 {
        0.9
    } else {
        1.1 - t.height / 2.0
    };
    let height = (sill + t.height + 0.2).max(3.0);
    m.set_wall_height(wall, height, 0.0).ok()?;
    m.add_opening(NewOpening {
        opening_type: ot,
        wall,
        along: length / 2.0,
        sill,
        opens_to: WallSide::Interior,
        hand: archisim_core::Hand::Left,
        setback: None,
    })
    .ok()?;
    Some((m, level))
}

/// [`opening_model`] for a picture in 3D: the wall only a little higher
/// than the door or window, so that it fills the picture.
pub fn opening_model_3d(source: &Model, t: &OpeningType) -> Option<(Model, LevelId)> {
    let (mut m, level) = opening_model(source, t)?;
    let wall = m.walls().next()?.id;
    let sill = m.openings().next()?.sill;
    m.set_wall_height(wall, sill + t.height + 0.12, 0.0).ok()?;
    Some((m, level))
}

/// A model with one object of type `t` at the origin, unturned: its plan
/// symbol as the plan draws it.
pub fn object_model(source: &Model, t: &archisim_core::ObjectType) -> Option<(Model, LevelId)> {
    let mut m = Model::new("preview");
    let level = m.add_level("preview", 0.0);
    let mut copied: std::collections::HashMap<u64, MaterialId> = Default::default();
    let mut values = t.values.clone();
    for v in values.values_mut() {
        if let archisim_core::Value::Material(id) = v {
            let new = match copied.get(id) {
                Some(n) => *n,
                None => {
                    let mat = source.material(MaterialId(*id))?.clone();
                    let n = m.add_material(mat.name, mat.category, mat.cut_color, mat.cut_pattern);
                    copied.insert(*id, n);
                    n
                }
            };
            *v = archisim_core::Value::Material(new.0);
        }
    }
    let family = m.add_family(&source.family(t.family)?.source).ok()?;
    let ot = m.add_object_type(t.name.clone(), family, values).ok()?;
    // Its own plan drawing (a DXF symbol, a picture), as the plan draws it.
    if t.symbol.is_some() {
        m.set_object_type_symbol(ot, t.symbol.clone()).ok()?;
    }
    if t.plan_image.is_some() {
        m.set_object_type_plan_image(ot, t.plan_image.clone())
            .ok()?;
    }
    let id = m
        .add_object(archisim_core::NewObject {
            level,
            object_type: ot,
            position: Point2::new(0.0, 0.0),
            rotation: 0.0,
            base_offset: 0.0,
        })
        .ok()?;
    // Trees and the like show their drawing of presentation (the form of
    // the canopy is what tells one from another), when they have one.
    let presented = m
        .object_presentation_symbol(id, 20.0)
        .is_some_and(|r| r.is_ok());
    let has_choice = m.family(family).is_some_and(|f| {
        f.def()
            .params_of(archisim_core::family::ParamScope::Instance)
            .any(|p| p.name == "drawing")
    });
    if presented && has_choice {
        let mut v = archisim_core::Values::new();
        v.insert(
            "drawing".into(),
            archisim_core::Value::Text("presentación".into()),
        );
        let _ = m.set_object_values(id, v);
    }
    Some((m, level))
}

/// A model with one column of the section and material of a column or
/// beam type, cut in plan: the section as the plan draws it.
pub fn member_model(source: &Model, t: &archisim_core::MemberType) -> Option<(Model, LevelId)> {
    let mut m = Model::new("preview");
    let level = m.add_level("preview", 0.0);
    let mat = source.material(t.material)?;
    let mid = m.add_material(
        mat.name.clone(),
        mat.category.clone(),
        mat.cut_color,
        mat.cut_pattern,
    );
    let ct = m
        .add_member_type(
            "preview",
            archisim_core::MemberUsage::Column,
            mid,
            t.profile,
        )
        .ok()?;
    m.add_column(archisim_core::NewColumn {
        member_type: ct,
        level,
        position: Point2::new(0.0, 0.0),
        rotation: 0.0,
        base_offset: 0.0,
        top: archisim_core::WallTop::Unconnected,
        height: 3.0,
    })
    .ok()?;
    Some((m, level))
}

/// A model with one curtain wall of design `design` (materials copied
/// from `source`), from (0, 0) to (`length`, 0), `height` high: what the
/// curtain wall types window draws and shows in 3D.
pub fn curtain_model(
    source: &Model,
    design: &archisim_core::CurtainDesign,
    length: f64,
    height: f64,
) -> Option<Model> {
    let mut m = Model::new("preview");
    let level = m.add_level("preview", 0.0);
    let mut copied: std::collections::HashMap<MaterialId, MaterialId> = Default::default();
    let mut copy = |m: &mut Model, id: MaterialId| -> Option<MaterialId> {
        if let Some(n) = copied.get(&id) {
            return Some(*n);
        }
        let mat = source.material(id)?;
        let n = m.add_material(
            mat.name.clone(),
            mat.category.clone(),
            mat.cut_color,
            mat.cut_pattern,
        );
        copied.insert(id, n);
        Some(n)
    };
    let mut d = design.clone();
    for p in [&mut d.border, &mut d.mullion, &mut d.transom] {
        p.material = copy(&mut m, p.material)?;
    }
    for p in &mut d.panels {
        p.material = copy(&mut m, p.material)?;
    }
    if let Some(c) = &mut d.caps {
        c.material = copy(&mut m, c.material)?;
    }
    if let Some(l) = &mut d.louvres {
        l.material = copy(&mut m, l.material)?;
    }
    let t = m.add_curtain_type("preview", d).ok()?;
    m.add_curtain_wall(archisim_core::NewCurtainWall {
        curtain_type: t,
        level,
        start: Point2::new(0.0, 0.0),
        end: Point2::new(length, 0.0),
        base_offset: 0.0,
        top: archisim_core::WallTop::Unconnected,
        height,
    })
    .ok()?;
    Some(m)
}

/// Draws a preview model around its wall (or its column), `px_per_m`
/// pixels per metre. Returns the image and the drawn region (lower-left,
/// upper-right).
pub fn draw(model: &Model, level: LevelId, px_per_m: f64) -> (ColorImage, Point2, Point2) {
    if model.walls().next().is_none() && model.columns().next().is_none() {
        if let Some((lo, hi)) = model
            .objects()
            .next()
            .and_then(|o| model.object_type_box(o.object_type))
        {
            // Its box with a little room around, drawn about the size it is
            // shown (screen textures are not filtered when made smaller,
            // and thin lines would vanish), at the scale that keeps its
            // lines a couple of pixels wide whatever its size.
            let pad = 0.06 + 0.02 * (hi[0] - lo[0]).max(hi[1] - lo[1]);
            let lo = Point2::new(lo[0] - pad, lo[1] - pad);
            let hi = Point2::new(hi[0] + pad, hi[1] + pad);
            let span = (hi.x - lo.x).max(hi.y - lo.y);
            let density = px_per_m / OBJECT_POINTS_PER_M;
            let px = OBJECT_PICTURE * density / span;
            let size = [
                ((hi.x - lo.x) * px).round().max(1.0) as usize,
                ((hi.y - lo.y) * px).round().max(1.0) as usize,
            ];
            // A drawing of its own from a DXF (a tree of a library) in
            // thin lines: its pens, made smaller with it, would fill it in.
            let own = model
                .objects()
                .next()
                .and_then(|o| model.object_type(o.object_type))
                .is_some_and(|t| t.symbol.is_some());
            let options = PlanOptions {
                scale: (PREVIEW_SCALE * 5.0 * span).max(PREVIEW_SCALE),
                viewport: Some((lo, hi)),
                thin_lines: own,
                ..PlanOptions::default()
            };
            return (rasterize_with(model, level, size, &options), lo, hi);
        }
    }
    if model.walls().next().is_none() {
        if let Some(c) = model.columns().next() {
            let (w, d) = model
                .member_type(c.member_type)
                .map_or((0.3, 0.3), |t| t.profile.size());
            // Square, with room around the section.
            let half = w.max(d) / 2.0 + 0.06;
            let lo = Point2::new(-half, -half);
            let hi = Point2::new(half, half);
            let n = ((2.0 * half) * px_per_m).round().max(1.0) as usize;
            let options = PlanOptions {
                scale: PREVIEW_SCALE,
                viewport: Some((lo, hi)),
                ..PlanOptions::default()
            };
            return (rasterize_with(model, level, [n, n], &options), lo, hi);
        }
    }
    let wall = model.walls().next();
    let (length, thickness) = wall.map_or((1.0, 0.3), |w| {
        let t = model.wall_type(w.wall_type).map_or(0.3, |t| t.thickness());
        (w.length(), t)
    });
    // Room for door leaves and their swing on the interior side (below:
    // the exterior face is on top), and for the window sill outside.
    let swing = model
        .openings()
        .filter_map(|o| model.opening_type(o.opening_type))
        .map(|t| swing_depth(t.kind, t.width))
        .fold(0.0_f64, f64::max);
    let lo = Point2::new(-0.02, -(thickness / 2.0 + swing + 0.08));
    let hi = Point2::new(length + 0.02, thickness / 2.0 + 0.1);
    let size = [
        ((hi.x - lo.x) * px_per_m).round().max(1.0) as usize,
        ((hi.y - lo.y) * px_per_m).round().max(1.0) as usize,
    ];
    let options = PlanOptions {
        scale: PREVIEW_SCALE,
        viewport: Some((lo, hi)),
        ..PlanOptions::default()
    };
    (rasterize_with(model, level, size, &options), lo, hi)
}

/// How far the leaves of a door or window reach into the room when open.
fn swing_depth(kind: OpeningKind, width: f64) -> f64 {
    match kind {
        OpeningKind::Door(DoorOperation::SingleSwing)
        | OpeningKind::Window(WindowOperation::SingleCasement) => width,
        OpeningKind::Door(DoorOperation::DoubleSwing)
        | OpeningKind::Window(WindowOperation::DoubleCasement) => width / 2.0,
        OpeningKind::Door(DoorOperation::Sliding) => 0.1,
        OpeningKind::Door(DoorOperation::Folding) => width / 3.0,
        OpeningKind::Door(DoorOperation::Pivot) => width * 0.8,
        OpeningKind::Door(DoorOperation::Overhead) => width.min(2.4),
        OpeningKind::Door(DoorOperation::Rolling) => 0.35,
        OpeningKind::Window(_) => 0.05,
        OpeningKind::Void => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previews_use_the_project_materials() {
        let m = archisim_library::new_project("p");
        let facade = m
            .wall_types()
            .find(|t| t.function == WallFunction::Exterior)
            .unwrap();
        let (pm, level) = layers_model(&m, &facade.layers, 1.0).unwrap();
        let wt = pm.wall_types().next().unwrap();
        assert_eq!(wt.layers.len(), facade.layers.len());
        // Plaster is used twice but copied once.
        assert_eq!(
            pm.materials().count(),
            facade
                .layers
                .iter()
                .map(|l| l.material)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
        );
        let (img, lo, hi) = draw(&pm, level, 200.0);
        assert_eq!(img.size[0], ((hi.x - lo.x) * 200.0).round() as usize);
        // The middle of the wall is drawn, opaque.
        let row = img.size[1] / 2;
        let col = img.size[0] / 2;
        assert_eq!(img.pixels[row * img.size[0] + col].a(), 255);
        // An invalid layer list gives no preview.
        assert!(layers_model(&m, &[], 1.0).is_none());
        // The 3D pieces.
        assert_eq!(
            wall_piece_model(&m, &facade.layers)
                .unwrap()
                .walls()
                .count(),
            1
        );
        let st = m.slab_types().next().unwrap();
        assert_eq!(slab_piece_model(&m, &st.layers).unwrap().slabs().count(), 1);
    }

    #[test]
    fn every_library_door_and_window_has_a_preview() {
        let m = archisim_library::new_project("p");
        for t in m.opening_types() {
            let (pm, level) = opening_model(&m, t).unwrap_or_else(|| panic!("{}", t.name));
            assert_eq!(pm.openings().count(), 1, "{}", t.name);
            let o = pm.openings().next().unwrap();
            let pt = pm.opening_type(o.opening_type).unwrap();
            // Cut by the plan's cut plane at 1.10 m.
            assert!(o.sill < 1.1 && o.sill + pt.height > 1.1, "{}", t.name);
            let (img, _, _) = draw(&pm, level, 150.0);
            assert!(img.pixels.iter().any(|p| p.a() > 0));
        }
    }
}
