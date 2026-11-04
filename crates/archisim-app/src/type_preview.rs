//! Previews in the types and materials dialog, drawn by the same plan
//! writer as the plan view and the exported plan: a short piece of wall of
//! the type being edited, cut in plan at 1:20; a slab type's layers as the
//! same band seen in section; a door or window in a piece of wall. What the
//! preview shows is what the plan will show.

use archisim_core::{DoorOperation, LevelId, Model, OpeningKind, Point2, WindowOperation};
use archisim_plan::PlanOptions;
use eframe::egui::ColorImage;

use crate::plan_image::rasterize_with;

/// Scale of the previews: a construction detail.
pub const PREVIEW_SCALE: f64 = 20.0;

/// Pictures of objects: the longer side in points, and the drawing density
/// (screen points per metre) the picture asked for is measured against.
const OBJECT_PICTURE: f64 = 150.0;
const OBJECT_POINTS_PER_M: f64 = 460.0;

pub use archisim_plan::type_models::*;

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
    use archisim_core::WallFunction;

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
