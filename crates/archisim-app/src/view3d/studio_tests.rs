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
    // `ARCHISIM_TALLER_SOLO=pilea`: only the pieces whose name has it.
    let solo = std::env::var("ARCHISIM_TALLER_SOLO")
        .ok()
        .map(|s| s.to_lowercase());
    for (i, piece) in archisim_library::workshop::PIECES.iter().enumerate() {
        if solo
            .as_ref()
            .is_some_and(|s| !piece.name.to_lowercase().contains(s.as_str()))
        {
            continue;
        }
        let new = m.object_types().find(|t| t.name == piece.name).unwrap().id;
        let one = only(&m, &[new]);
        let n = i + 1;
        // A tall piece (a lamp post) from a little further, to see it all.
        let (_, tall) = piece_size(&m, new);
        let back = if tall > 1.4 * piece_width(&m, new) {
            0.8
        } else {
            1.0
        };
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
                    let mut c = camera(lo, hi, aspect, yaw, pitch, zoom * back);
                    if zoom > 1.0 {
                        // Close to one end and the upper part: a sofa's
                        // seat and cushion, a plant's leaves, a tap.
                        let (fx, fz) = detail_of(piece.family);
                        c.target = glam::Vec3::new(
                            lo.x + (hi.x - lo.x) * fx,
                            (lo.y + hi.y) / 2.0,
                            lo.z + (hi.z - lo.z) * fz,
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
            let (depth, height) = {
                let (a, b) = (piece_size(&m, new), piece_size(&m, old));
                (a.0.max(b.0), a.1.max(b.1))
            };
            // Each one framed on the same box round it: same size for both.
            let shot = |t: archisim_core::ObjectTypeId| {
                let o = both.objects().find(|o| o.object_type == t).unwrap();
                let (x, y) = (o.position.x as f32, o.position.y as f32);
                (
                    glam::Vec3::new(x - width / 2.0, y - depth, 0.0),
                    glam::Vec3::new(x + width / 2.0, y, height),
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

/// Where the close-up looks, as shares of the piece's width and height.
fn detail_of(family: &str) -> (f32, f32) {
    if family.starts_with("Planta") {
        (0.55, 0.75)
    } else if family.starts_with("Módulo") {
        (0.5, 0.92)
    } else if family.starts_with("Farola") {
        (0.5, 0.88)
    } else {
        (0.25, 0.55)
    }
}

/// Depth and height of a type's model (of its 3D, leaves and all).
fn piece_size(m: &Model, t: archisim_core::ObjectTypeId) -> (f32, f32) {
    let positions = if m.type_grows(t) {
        let Some(o) = m.objects().find(|o| o.object_type == t) else {
            return (1.0, 0.9);
        };
        match m.plant_mesh(o.id, archisim_core::flora::Lod::Low) {
            Some((mesh, _)) => mesh.positions.clone(),
            None => return (1.0, 0.9),
        }
    } else {
        match m.type_prop(t) {
            Some((mesh, _)) => mesh.positions.clone(),
            None => return (1.0, 0.9),
        }
    };
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for p in &positions {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    ((hi[1] - lo[1]).max(0.2), hi[2].max(0.2))
}

fn piece_width(m: &Model, t: archisim_core::ObjectTypeId) -> f32 {
    if m.type_grows(t) {
        // A plant: as wide as it is deep.
        return piece_size(m, t).0;
    }
    m.object_type_box(t)
        .map_or(1.0, |b| (b.1[0] - b.0[0]) as f32)
}

/// The living and dining room of the library (paquete 1) in each of its
/// eight collections, a piece of each type standing in a row per
/// collection; with `copies`, each piece that many times more (as a real
/// project repeats its chairs).
fn collections_project(copies: usize) -> Model {
    use archisim_library::collections::{add_collection_entry, COLLECTIONS};
    let families = [
        "Sofá",
        "Butaca",
        "Puf",
        "Silla",
        "Taburete",
        "Mesa",
        "Mesa de centro",
        "Mesa auxiliar",
        "Aparador",
        "Estantería",
        "Mueble TV",
        "Vitrina",
        "Alfombra",
        "Cortina",
        "Chimenea",
    ];
    let mut m = archisim_library::new_project("Colecciones");
    let level = m.levels()[0].id;
    for (row, c) in COLLECTIONS.iter().enumerate() {
        let mut x = 0.0;
        for (name, family, _) in archisim_library::OBJECT_CATALOGUE.iter() {
            if !families.contains(family) {
                continue;
            }
            let t = add_collection_entry(&mut m, name, c).unwrap();
            for k in 0..=copies {
                m.add_object(archisim_core::NewObject {
                    object_type: t,
                    level,
                    position: archisim_core::Point2::new(x, 4.0 * row as f64 + 40.0 * k as f64),
                    rotation: 0.0,
                    base_offset: 0.0,
                })
                .unwrap();
            }
            x += 3.2;
        }
    }
    m
}

/// The library's living room in its eight collections is a large scene,
/// and it draws (Antonio's laptop closed the app on the workshop's).
#[test]
fn collections_scene() {
    let m = collections_project(0);
    let scene = Scene::build(&m, &BTreeSet::new());
    eprintln!(
        "collections scene: {} triangles, {} groups, {} objects",
        scene.triangle_count(),
        scene.groups.len(),
        m.objects().count()
    );
    assert!(scene.triangle_count() > 1_000_000, "a large scene");
    // Every piece is a group of its own (none drawn loose).
    assert_eq!(
        scene
            .groups
            .iter()
            .map(|g| g.instances.len())
            .sum::<usize>(),
        m.objects().count()
    );
    let Some(gpu) = super::gpu_tests::gpu() else {
        return;
    };
    let mut r = Renderer::new(&gpu.0, gpu.2, TextureMode::Full);
    let pic = picture(
        &mut r,
        &gpu,
        &m,
        |lo, hi| camera(lo, hi, 1.5, -62.0, 30.0, 1.0),
        [1200, 800],
    );
    if let Ok(dir) = std::env::var("ARCHISIM_TALLER_OUT") {
        pic.save_png(Path::new(&dir).join("colecciones.png"))
            .unwrap();
    }
}

/// What the 3D view costs with the library's living room in its eight
/// collections, once and four times over:
/// building the scene, memory on the card, and drawing far and near
/// (`ARCHISIM_MEDIR_3D=1 cargo test --release -p archisim-app --
/// scene_benchmark --nocapture`).
#[test]
fn scene_benchmark() {
    if std::env::var_os("ARCHISIM_MEDIR_3D").is_none() {
        return;
    }
    let Some(gpu) = super::gpu_tests::gpu() else {
        return;
    };
    for (name, m) in [
        ("colecciones", collections_project(0)),
        ("repetido", collections_project(3)),
    ] {
        let t = std::time::Instant::now();
        let scene = Scene::build(&m, &BTreeSet::new());
        let build = t.elapsed();
        let mut r = Renderer::new(&gpu.0, gpu.2, TextureMode::Full);
        let t = std::time::Instant::now();
        r.set_scene(&gpu.0, &scene);
        let upload = t.elapsed();
        r.set_look(
            Quality::Pretty,
            Some(glam::Vec3::new(-0.45, -0.7, 0.75).normalize()),
        );
        let mut times = Vec::new();
        for (view, zoom) in [("todo", 1.0), ("cerca", 8.0)] {
            let mut cam = camera(scene.lo, scene.hi, 1.5, -62.0, 30.0, zoom);
            cam.target = scene.lo + (scene.hi - scene.lo) * glam::Vec3::new(0.3, 0.5, 0.2);
            r.render(&gpu.0, &gpu.1, &cam, [1280, 800], 1);
            let _ = r.read_pixels(&gpu.0, &gpu.1);
            let t = std::time::Instant::now();
            for _ in 0..3 {
                r.render(&gpu.0, &gpu.1, &cam, [1280, 800], 1);
                let _ = r.read_pixels(&gpu.0, &gpu.1);
            }
            if let (Ok(dir), Some((w, h, px))) = (
                std::env::var("ARCHISIM_TALLER_OUT"),
                r.read_pixels(&gpu.0, &gpu.1),
            ) {
                let mut pic = Pixmap::new(w, h).unwrap();
                pic.data_mut().copy_from_slice(&px);
                pic.save_png(Path::new(&dir).join(format!("medida-{name}-{view}.png")))
                    .unwrap();
            }
            times.push(format!(
                "{view} {:.0} ms",
                t.elapsed().as_secs_f64() * 1000.0 / 3.0
            ));
        }
        eprintln!(
            "{name}: {} objetos, escena {:.0} ms, subida {:.0} ms, {} MB en la tarjeta, fotograma: {}",
            m.objects().count(),
            build.as_secs_f64() * 1000.0,
            upload.as_secs_f64() * 1000.0,
            r.scene_bytes() >> 20,
            times.join(", ")
        );
    }
}

/// A picture of the library's living room in each collection
/// (`ARCHISIM_COLECCIONES_OUT=carpeta`): its pieces in a grid, from above
/// and to one side.
#[test]
fn collection_pictures() {
    use archisim_library::collections::{add_collection_entry, COLLECTIONS};
    let Some(dir) = std::env::var_os("ARCHISIM_COLECCIONES_OUT") else {
        return;
    };
    let dir = Path::new(&dir);
    std::fs::create_dir_all(dir).unwrap();
    let Some(gpu) = super::gpu_tests::gpu() else {
        return;
    };
    let mut r = Renderer::new(&gpu.0, gpu.2, TextureMode::Full);
    let families = [
        "Sofá",
        "Butaca",
        "Puf",
        "Silla",
        "Taburete",
        "Mesa",
        "Mesa de centro",
        "Mesa auxiliar",
        "Aparador",
        "Estantería",
        "Mueble TV",
        "Vitrina",
        "Alfombra",
        "Cortina",
        "Chimenea",
    ];
    for (i, c) in COLLECTIONS.iter().enumerate() {
        let mut m = archisim_library::new_project("Colección");
        let level = m.levels()[0].id;
        let mut k = 0;
        for f in families {
            // The first piece of each family.
            let Some((name, _, _)) = archisim_library::OBJECT_CATALOGUE.iter().find(|e| e.1 == f)
            else {
                continue;
            };
            let t = add_collection_entry(&mut m, name, c).unwrap();
            m.add_object(archisim_core::NewObject {
                object_type: t,
                level,
                position: archisim_core::Point2::new(3.4 * (k % 5) as f64, -3.4 * (k / 5) as f64),
                rotation: 0.0,
                base_offset: 0.0,
            })
            .unwrap();
            k += 1;
        }
        let pic = picture(
            &mut r,
            &gpu,
            &m,
            |lo, hi| camera(lo, hi, 1.5, -70.0, 32.0, 1.15),
            [1500, 1000],
        );
        pic.save_png(dir.join(format!("{}-{}.png", i + 1, c.name)))
            .unwrap();
    }
}

/// The workshop's kitchens (paquete 2a), one by one with their wall
/// (`ARCHISIM_COCINAS_OUT=carpeta`): from three quarters, from the front,
/// and close to the sink and to the hob.
#[test]
fn kitchen_pictures() {
    let Some(dir) = std::env::var_os("ARCHISIM_COCINAS_OUT") else {
        return;
    };
    let dir = Path::new(&dir);
    std::fs::create_dir_all(dir).unwrap();
    let Some(gpu) = super::gpu_tests::gpu() else {
        eprintln!("No graphics adapter: kitchen pictures skipped.");
        return;
    };
    let mut r = Renderer::new(&gpu.0, gpu.2, TextureMode::Full);
    let m = archisim_library::workshop::project().unwrap();
    let size = [1200u32, 800u32];
    let aspect = size[0] as f32 / size[1] as f32;
    for (i, k) in archisim_library::workshop::KITCHENS.iter().enumerate() {
        let mine: Vec<_> = m
            .object_types()
            .filter(|t| t.name.contains(k.collection) && t.name.ends_with("(taller)"))
            .map(|t| t.id)
            .collect();
        let mut one = m.clone();
        let others: Vec<_> = one
            .objects()
            .filter(|o| !mine.contains(&o.object_type))
            .map(|o| o.id)
            .collect();
        for o in others {
            one.remove_object(o).unwrap();
        }
        let (x0, x1) = one
            .objects()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |a, o| {
                (a.0.min(o.position.x), a.1.max(o.position.x))
            });
        let walls: Vec<_> = one
            .walls()
            .filter(|w| w.start.x.max(w.end.x) < x0 - 1.0 || w.start.x.min(w.end.x) > x1 + 1.0)
            .map(|w| w.id)
            .collect();
        for w in walls {
            one.remove_wall(w).unwrap();
        }
        // The sink's and the hob's places.
        let at = |unit: &str| {
            one.objects()
                .find(|o| {
                    one.object_type(o.object_type)
                        .is_some_and(|t| t.name.starts_with(unit))
                })
                .map(|o| (o.position.x as f32, o.position.y as f32))
        };
        let sink = at("Fregadero").unwrap();
        let hob = at("Placa").unwrap();
        let n = i + 1;
        for (name, yaw, pitch, zoom, focus) in [
            ("tres-cuartos", -58.0, 18.0, 1.15, None),
            ("frente", -90.0, 8.0, 1.2, None),
            ("fregadero", -72.0, 32.0, 3.6, Some((sink, 0.95))),
            ("placa-campana", -78.0, 14.0, 2.2, Some((hob, 1.35))),
        ] {
            let pic = picture(
                &mut r,
                &gpu,
                &one,
                |lo, hi| {
                    // The kitchen's box, not its whole wall.
                    let lo = glam::Vec3::new(x0 as f32 - 0.4, lo.y.max(-0.2), 0.0);
                    let hi = glam::Vec3::new(x1 as f32 + 0.4, hi.y, 2.3);
                    let mut c = camera(lo, hi, aspect, yaw, pitch, zoom);
                    if let Some(((x, y), z)) = focus {
                        c.target = glam::Vec3::new(x, y - 0.35, z);
                    }
                    c
                },
                size,
            );
            let slug = k.collection.to_lowercase().replace(' ', "-");
            pic.save_png(dir.join(format!("{n}-{slug}-{name}.png")))
                .unwrap();
        }
    }
}
