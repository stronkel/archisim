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

/// A model with one level and one straight wall from (0, 0) to
/// (`length`, 0), the exterior on top, built from `layers` with the
/// materials of `source`. `None` if the layers are not valid.
pub fn layers_model(source: &Model, layers: &[Layer], length: f64) -> Option<(Model, LevelId)> {
    let mut m = Model::new("preview");
    let level = m.add_level("preview", 0.0);
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
        top: archisim_core::WallTop::Unconnected,
    })
    .ok()?;
    Some((m, level))
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

/// Draws a preview model around its wall, `px_per_m` pixels per metre.
/// Returns the image and the drawn region (lower-left, upper-right).
pub fn draw(model: &Model, level: LevelId, px_per_m: f64) -> (ColorImage, Point2, Point2) {
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
        OpeningKind::Window(_) => 0.05,
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
