//! The plan on screen is the very same drawing as the exported 1:50 plan:
//! the SVG of the visible region is rasterised at screen resolution. Line
//! weights are therefore true paper weights at 1:50, as in ArchiCAD's
//! "true line weight" view, and what is seen is exactly what is exported.

use archisim_core::{LevelId, Model, Point2};
use archisim_plan::{render_plan, PlanOptions};
use eframe::egui::ColorImage;
use resvg::{tiny_skia, usvg};

/// Everything that decides how the picture looks.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanKey {
    pub revision: u64,
    pub level: LevelId,
    pub lo: Point2,
    pub hi: Point2,
    pub size: [usize; 2],
    pub show_reference_lines: bool,
}

/// Draws the region `lo`–`hi` of a level into an image of `size` pixels.
/// Outside the drawing the image is transparent.
pub fn rasterize(
    model: &Model,
    level: LevelId,
    lo: Point2,
    hi: Point2,
    size: [usize; 2],
    show_reference_lines: bool,
) -> ColorImage {
    let options = PlanOptions {
        viewport: Some((lo, hi)),
        show_reference_lines,
        ..PlanOptions::default()
    };
    rasterize_with(model, level, size, &options)
}

/// Draws a level with the given plan options (a viewport is required) into
/// an image of `size` pixels. The scale only changes the line weights and
/// hatch spacing, which are paper millimetres at that scale.
pub fn rasterize_with(
    model: &Model,
    level: LevelId,
    size: [usize; 2],
    options: &PlanOptions,
) -> ColorImage {
    let [w, h] = [size[0].max(1), size[1].max(1)];
    let svg = render_plan(model, level, options);
    let mut pixmap = tiny_skia::Pixmap::new(w as u32, h as u32).expect("non-empty image");
    match usvg::Tree::from_str(&svg, &usvg::Options::default()) {
        Ok(tree) => {
            let s = tree.size();
            let t = tiny_skia::Transform::from_scale(w as f32 / s.width(), h as f32 / s.height());
            resvg::render(&tree, t, &mut pixmap.as_mut());
        }
        Err(e) => {
            // The plan writer always produces valid SVG; if not, show
            // nothing rather than crash, and say so where it can be seen.
            eprintln!("ArchiSim: no se pudo dibujar la planta: {e}");
        }
    }
    ColorImage::from_rgba_premultiplied([w, h], pixmap.data())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pick::tests::room;

    #[test]
    fn draws_the_walls_where_the_view_says() {
        let (m, lvl, _) = room();
        // 10 × 8 m around the 6 × 4 m room, 20 px per metre.
        let lo = Point2::new(-2.0, -2.0);
        let hi = Point2::new(8.0, 6.0);
        let img = rasterize(&m, lvl, lo, hi, [200, 160], false);
        assert_eq!(img.size, [200, 160]);
        let px = |x: f64, y: f64| {
            let col = ((x - lo.x) * 20.0) as usize;
            let row = ((hi.y - y) * 20.0) as usize;
            img.pixels[row * 200 + col]
        };
        // Outside the building: transparent.
        assert_eq!(px(-1.5, 3.0).a(), 0);
        // Inside the west wall's brick (x 0.015–0.13): opaque, warm colour.
        let brick = px(0.07, 2.0);
        assert_eq!(brick.a(), 255);
        assert!(brick.r() > brick.b(), "{brick:?}");
        // Inside the room the ground slab is drawn below the cut plane.
        assert!(px(3.0, 2.0).a() > 0);
    }
}

#[cfg(test)]
mod timing {
    /// `cargo test --release -p archisim-app plan_timing -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn plan_timing() {
        let (m, lvl, _) = crate::pick::tests::room();
        for (lo, hi, size) in [
            ((-2.0, -2.0), (8.0, 6.0), [1600, 1000]),
            ((-0.5, 3.0), (1.5, 4.5), [1600, 1000]),
            ((-2.0, -2.0), (8.0, 6.0), [3200, 2000]),
        ] {
            let t = std::time::Instant::now();
            let n = 10;
            for _ in 0..n {
                super::rasterize(
                    &m,
                    lvl,
                    archisim_core::Point2::new(lo.0, lo.1),
                    archisim_core::Point2::new(hi.0, hi.1),
                    size,
                    false,
                );
            }
            println!(
                "{size:?} {lo:?}-{hi:?}: {:.1} ms",
                t.elapsed().as_secs_f64() * 1000.0 / n as f64
            );
        }
    }
}
