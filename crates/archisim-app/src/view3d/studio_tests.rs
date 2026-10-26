//! Pictures of the workshop's models (M1), drawn by the same 3D view as
//! the Maqueta, to show Antonio each new piece next to the one it is to
//! replace. Only with `ARCHISIM_TALLER_OUT=<folder>`: they are pictures to
//! look at, not a check.

use std::collections::BTreeSet;
use std::path::Path;

use archisim_core::Model;
use resvg::tiny_skia::{IntSize, Pixmap, PixmapPaint, Transform};

use super::camera::{Camera, FOV_Y};
use super::look::{Atmosphere, Light, Quality};
use super::render::{Renderer, TextureMode};
use super::scene::Scene;

/// The project with only the objects of these types (and nothing else:
/// a studio picture).
fn only(m: &Model, types: &[archisim_core::ObjectTypeId]) -> Model {
    let mut m = m.clone();
    let walls: Vec<_> = m.walls().map(|w| w.id).collect();
    for w in walls {
        m.remove_wall(w).unwrap();
    }
    let others: Vec<_> = m
        .objects()
        .filter(|o| !types.contains(&o.object_type))
        .map(|o| o.id)
        .collect();
    for o in others {
        m.remove_object(o).unwrap();
    }
    m
}

/// A camera looking at the box from `yaw`, `pitch` (degrees), filling
/// the picture (`zoom` > 1: closer).
fn camera(lo: glam::Vec3, hi: glam::Vec3, aspect: f32, yaw: f32, pitch: f32, zoom: f32) -> Camera {
    let mut c = Camera {
        target: (lo + hi) / 2.0,
        yaw: yaw.to_radians(),
        pitch: pitch.to_radians(),
        distance: 1.0,
        perspective: true,
    };
    let radius = ((hi - lo).length() / 2.0).max(0.05);
    let half = (FOV_Y / 2.0).tan() * aspect.min(1.0);
    c.distance = radius / half.atan().sin() * 0.9 / zoom;
    c
}

fn picture(
    r: &mut Renderer,
    gpu: &(eframe::wgpu::Device, eframe::wgpu::Queue, bool),
    m: &Model,
    cam: impl Fn(glam::Vec3, glam::Vec3) -> Camera,
    size: [u32; 2],
) -> Pixmap {
    let scene = Scene::build(m, &BTreeSet::new());
    r.set_scene(&gpu.0, &scene);
    r.set_clip(None);
    r.set_cut(None);
    r.set_transparent(false);
    r.set_atmosphere(Some(Atmosphere {
        light: Light::Day,
        ground_z: 0.0,
        grid: None,
    }));
    r.set_look(
        Quality::Pretty,
        Some(glam::Vec3::new(-0.45, -0.7, 0.75).normalize()),
    );
    r.set_shadow_focus(None);
    let camera = cam(scene.lo, scene.hi);
    r.render(&gpu.0, &gpu.1, &camera, size, 1);
    let (w, h, px) = r.read_pixels(&gpu.0, &gpu.1).unwrap();
    Pixmap::from_vec(px, IntSize::from_wh(w, h).unwrap()).unwrap()
}

/// Two pictures side by side, the first on the left.
fn side_by_side(a: &Pixmap, b: &Pixmap) -> Pixmap {
    let gap = 12;
    let mut out = Pixmap::new(a.width() + b.width() + gap, a.height().max(b.height())).unwrap();
    out.fill(resvg::tiny_skia::Color::WHITE);
    out.draw_pixmap(
        0,
        0,
        a.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    out.draw_pixmap(
        (a.width() + gap) as i32,
        0,
        b.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    out
}

#[test]
fn workshop_pictures() {
    let Some(dir) = std::env::var_os("ARCHISIM_TALLER_OUT") else {
        return;
    };
    let dir = Path::new(&dir);
    std::fs::create_dir_all(dir).unwrap();
    let Some(gpu) = super::gpu_tests::gpu() else {
        eprintln!("No graphics adapter: workshop pictures skipped.");
        return;
    };
    let mut r = Renderer::new(&gpu.0, gpu.2, TextureMode::Full);
    let m = archisim_library::workshop::project().unwrap();
    let size = [1100u32, 760u32];
    let aspect = size[0] as f32 / size[1] as f32;
    for (i, piece) in archisim_library::workshop::PIECES.iter().enumerate() {
        let new = m.object_types().find(|t| t.name == piece.name).unwrap().id;
        let one = only(&m, &[new]);
        let n = i + 1;
        // The new piece from three sides and close up.
        for (name, yaw, pitch, zoom) in [
            ("tres-cuartos", -62.0, 22.0, 1.0),
            ("frente", -90.0, 12.0, 1.0),
            ("otro-lado", -128.0, 30.0, 1.0),
            ("detalle", -70.0, 16.0, 2.6),
        ] {
            let pic = picture(
                &mut r,
                &gpu,
                &one,
                |lo, hi| {
                    let mut c = camera(lo, hi, aspect, yaw, pitch, zoom);
                    if zoom > 1.0 {
                        // Close to the end of the seat and its cushion.
                        c.target = glam::Vec3::new(
                            lo.x + (hi.x - lo.x) * 0.25,
                            (lo.y + hi.y) / 2.0,
                            lo.z + (hi.z - lo.z) * 0.55,
                        );
                    }
                    c
                },
                size,
            );
            pic.save_png(dir.join(format!("{n}-{name}.png"))).unwrap();
        }
        // Before and after, with the same camera and scale.
        if let Some(old) = piece.replaces {
            let old = m.object_types().find(|t| t.name == old).unwrap().id;
            let both = only(&m, &[new, old]);
            let half = [size[0] / 2, size[1]];
            let a = half[0] as f32 / half[1] as f32;
            let width = piece_width(&m, new).max(piece_width(&m, old));
            // Each one framed on the same box round it: same size for both.
            let shot = |t: archisim_core::ObjectTypeId| {
                let o = both.objects().find(|o| o.object_type == t).unwrap();
                let (x, y) = (o.position.x as f32, o.position.y as f32);
                (
                    glam::Vec3::new(x - width / 2.0, y - 1.0, 0.0),
                    glam::Vec3::new(x + width / 2.0, y, 0.9),
                )
            };
            let before = {
                let (l, h) = shot(old);
                picture(
                    &mut r,
                    &gpu,
                    &only(&m, &[old]),
                    |_, _| camera(l, h, a, -62.0, 20.0, 1.0),
                    half,
                )
            };
            let after = {
                let (l, h) = shot(new);
                picture(
                    &mut r,
                    &gpu,
                    &one,
                    |_, _| camera(l, h, a, -62.0, 20.0, 1.0),
                    half,
                )
            };
            side_by_side(&before, &after)
                .save_png(dir.join(format!("{n}-antes-y-despues.png")))
                .unwrap();
        }
    }
}

fn piece_width(m: &Model, t: archisim_core::ObjectTypeId) -> f32 {
    m.object_type_box(t)
        .map_or(1.0, |b| (b.1[0] - b.0[0]) as f32)
}
