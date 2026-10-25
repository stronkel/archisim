//! Draws a real picture with whatever graphics adapter the machine has (a
//! software one is enough). Skipped, with a note, if there is none.

use std::collections::BTreeSet;

use eframe::wgpu;

use super::camera::Camera;
use super::render::Renderer;
use super::scene::Scene;
use crate::pick::tests::room;

/// In the continuous integration, a failing test is also written as a
/// note of the job, readable without its log (only some runners have a
/// graphics adapter, so a 3D test may fail where no one can run it).
pub(crate) fn ci_notes() {
    if std::env::var_os("GITHUB_ACTIONS").is_none() {
        return;
    }
    static HOOK: std::sync::Once = std::sync::Once::new();
    HOOK.call_once(|| {
        let old = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let msg = info.to_string().replace('\n', " ");
            println!("::error title=Prueba fallida::{msg}");
            old(info);
        }));
    });
}

/// Whether the only graphics adapter is OpenGL.
fn gl_only() -> bool {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        ..Default::default()
    }))
    .is_ok_and(|a| a.get_info().backend == wgpu::Backend::Gl)
}

pub(crate) fn gpu() -> Option<(wgpu::Device, wgpu::Queue, bool)> {
    ci_notes();
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        force_fallback_adapter: false,
        compatible_surface: None,
        ..Default::default()
    }))
    .ok()?;
    let view_formats = adapter
        .get_downlevel_capabilities()
        .flags
        .contains(wgpu::DownlevelFlags::VIEW_FORMATS);
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
    Some((device, queue, view_formats))
}

/// Renders the test room and returns the picture.
pub(crate) fn picture(size: [u32; 2]) -> Option<(u32, u32, Vec<u8>)> {
    let Some((device, queue, view_formats)) = gpu() else {
        eprintln!("No graphics adapter: 3D drawing test skipped.");
        return None;
    };
    let (m, _, _) = room();
    let scene = Scene::build(&m, &BTreeSet::new());
    let mut camera = Camera::default();
    camera.fit(scene.lo, scene.hi, size[0] as f32 / size[1] as f32);
    let mut r = Renderer::new(&device, view_formats, super::render::TextureMode::Full);
    r.set_scene(&device, &scene);
    r.render(&device, &queue, &camera, size, 1);
    r.read_pixels(&device, &queue)
}

#[test]
fn draws_the_room_with_contours() {
    let size = std::env::var("ARCHISIM_3D_SIZE")
        .ok()
        .and_then(|s| {
            s.split_once("x")
                .map(|(a, b)| [a.parse().unwrap(), b.parse().unwrap()])
        })
        .unwrap_or([480, 320]);
    let Some((w, h, px)) = picture(size) else {
        return;
    };
    assert_eq!([w, h], size);
    if let Some(dir) = std::env::var_os("ARCHISIM_TEST_OUT") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        let pixmap = resvg::tiny_skia::Pixmap::from_vec(
            px.clone(),
            resvg::tiny_skia::IntSize::from_wh(w, h).unwrap(),
        )
        .unwrap();
        pixmap.save_png(dir.join("vista3d.png")).unwrap();
    }
    let at = |x: u32, y: u32| {
        let i = ((y * w + x) * 4) as usize;
        [px[i], px[i + 1], px[i + 2], px[i + 3]]
    };
    // The corner is background; the middle shows the building.
    let bg = at(2, 2);
    assert!(bg[0] > 200 && bg[3] == 255, "{bg:?}");
    let building = (0..w * h)
        .filter(|i| {
            let p = at(i % w, i / w);
            p[..3] != bg[..3]
        })
        .count();
    assert!(building > (w * h / 10) as usize, "{building}");
    // Contour lines in ink.
    let ink = (0..w * h)
        .filter(|i| at(i % w, i / w)[..3].iter().all(|c| *c < 90))
        .count();
    assert!(ink > 200, "{ink} ink pixels");
}

/// With a section box cutting the walls at 1 m, seen from above, the cut
/// walls show their caps in their own colours (not the floor's), and what
/// lies above the box is gone.
#[test]
fn section_box_cuts_and_caps() {
    let Some((device, queue, view_formats)) = gpu() else {
        eprintln!("No graphics adapter: section box test skipped.");
        return;
    };
    let (m, _, _) = room();
    let scene = Scene::build(&m, &BTreeSet::new());
    let size = [400u32, 300u32];
    let mut camera = Camera {
        pitch: 1.45,
        perspective: false,
        ..Camera::default()
    };
    camera.fit(scene.lo, scene.hi, size[0] as f32 / size[1] as f32);
    let mut r = Renderer::new(&device, view_formats, super::render::TextureMode::Full);
    r.set_scene(&device, &scene);
    let mut cut = super::section_box::SectionBox::around(scene.lo, scene.hi);
    cut.hi.z = 1.0;
    r.set_clip(Some((cut.lo, cut.hi)));
    r.render(&device, &queue, &camera, size, 1);
    let (w, h, px) = r.read_pixels(&device, &queue).unwrap();
    if let Some(dir) = std::env::var_os("ARCHISIM_TEST_OUT") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        let pixmap = resvg::tiny_skia::Pixmap::from_vec(
            px.clone(),
            resvg::tiny_skia::IntSize::from_wh(w, h).unwrap(),
        )
        .unwrap();
        pixmap.save_png(dir.join("caja-seccion.png")).unwrap();
    }
    // The façade's brick (reddish) shows in the caps of the cut walls.
    let reddish = (0..(w * h) as usize)
        .filter(|i| {
            let p = &px[i * 4..i * 4 + 3];
            p[0] as i32 - p[2] as i32 > 25
        })
        .count();
    assert!(reddish > 300, "{reddish} brick pixels in the caps");
    // Without the box, from above, the tops of the walls are drawn too:
    // the picture changes.
    r.set_clip(None);
    r.render(&device, &queue, &camera, size, 1);
    let (_, _, full) = r.read_pixels(&device, &queue).unwrap();
    assert_ne!(full, px);
}

/// The textures looked up in pictures look like the full ones: a brick
/// wall seen up close differs little between the two, and much from
/// plain colours.
#[test]
fn pictures_look_like_the_full_textures() {
    let Some((device, queue, view_formats)) = gpu() else {
        eprintln!("No graphics adapter: texture pictures test skipped.");
        return;
    };
    if gl_only() {
        // OpenGL does not see the pictures (the app uses the light
        // textures there).
        eprintln!("OpenGL: texture pictures test skipped.");
        return;
    }
    let (mut m, _, _) = room();
    // Every finish in brick, seen up close.
    let mats: Vec<_> = m.materials().map(|x| x.id).collect();
    for id in mats {
        m.set_appearance(
            id,
            archisim_core::Appearance {
                color: Some([170, 80, 60]),
                texture: Some(archisim_core::TextureKind::Brick),
                scale: Some(0.24),
            },
        )
        .unwrap();
    }
    let scene = Scene::build(&m, &BTreeSet::new());
    let size = [320u32, 240u32];
    let mut camera = Camera::default();
    camera.fit(scene.lo, scene.hi, size[0] as f32 / size[1] as f32);
    camera.distance *= 0.35;
    let draw = |mode| {
        let mut r = Renderer::new(&device, view_formats, mode);
        r.set_scene(&device, &scene);
        r.render(&device, &queue, &camera, size, 1);
        r.read_pixels(&device, &queue).unwrap().2
    };
    let full = draw(super::render::TextureMode::Full);
    let pictures = draw(super::render::TextureMode::Pictures);
    let off = draw(super::render::TextureMode::Off);
    if let Some(dir) = std::env::var_os("ARCHISIM_TEST_OUT") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        for (name, px) in [("completas", &full), ("imagen", &pictures), ("lisas", &off)] {
            let pixmap = resvg::tiny_skia::Pixmap::from_vec(
                px.clone(),
                resvg::tiny_skia::IntSize::from_wh(size[0], size[1]).unwrap(),
            )
            .unwrap();
            pixmap
                .save_png(dir.join(format!("texturas-{name}.png")))
                .unwrap();
        }
    }
    // The same pattern, not in the same place: alike in mean and in
    // contrast, both far from the plain colour's.
    let stats = |px: &[u8]| -> (f64, f64) {
        let v: Vec<f64> = px
            .chunks(4)
            .map(|p| (p[0] as f64 + p[1] as f64 + p[2] as f64) / 3.0)
            .collect();
        let mean = v.iter().sum::<f64>() / v.len() as f64;
        let var = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / v.len() as f64;
        (mean, var.sqrt())
    };
    let (f, p, o) = (stats(&full), stats(&pictures), stats(&off));
    assert!(f.1 > o.1 * 1.5, "full {f:?} plain {o:?}");
    assert!(p.1 > o.1 * 1.5, "pictures {p:?} plain {o:?}");
    assert!((f.0 - p.0).abs() < 12.0, "full {f:?} pictures {p:?}");
    assert!((f.1 - p.1).abs() < f.1 * 0.35, "full {f:?} pictures {p:?}");
}

#[test]
fn shaders_with_every_texture_mode_are_valid() {
    use super::render::{shader_source, TextureMode};
    for mode in TextureMode::ALL {
        let source = shader_source(mode);
        let module = naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|e| panic!("{mode:?}: {}", e.emit_to_string(&source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{mode:?}: {e:?}"));
    }
    assert!(!shader_source(TextureMode::Off).contains("fn cells("));
    let light = shader_source(TextureMode::Light);
    assert!(!light.contains("fn cells(") && !light.contains("switch kind"));
}

/// Selecting outlines in orange without tinting; cutting the walls
/// leaves the walls in front low and the ones behind whole.
#[test]
fn selection_outline_and_walls_in_front() {
    let Some((device, queue, view_formats)) = gpu() else {
        eprintln!("No graphics adapter: 3D drawing test skipped.");
        return;
    };
    let (m, _, _) = room();
    let size = [400, 300];
    let draw = |selection: &BTreeSet<crate::pick::Element>,
                cut: Option<super::cutaway::CutRule>| {
        let scene = Scene::build(&m, selection);
        let mut camera = Camera::default();
        camera.fit(scene.lo, scene.hi, size[0] as f32 / size[1] as f32);
        let mut r = Renderer::new(
            &device,
            view_formats,
            crate::view3d::render::TextureMode::Full,
        );
        r.set_scene(&device, &scene);
        r.set_cut(cut);
        r.render(&device, &queue, &camera, size, 1);
        (r.read_pixels(&device, &queue).unwrap().2, camera)
    };
    let orange = |px: &[u8]| {
        px.as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] > 230 && p[1] > 150 && p[1] < 215 && p[2] < 90)
            .count()
    };
    let (plain, camera) = draw(&BTreeSet::new(), None);
    let wall = m.walls().next().unwrap().id;
    let sel: BTreeSet<_> = [crate::pick::Element::Wall(wall)].into_iter().collect();
    let (marked, _) = draw(&sel, None);
    assert!(
        orange(&marked) > orange(&plain) + 100,
        "outline: {} orange pixels selected, {} not",
        orange(&marked),
        orange(&plain)
    );
    // Away from the outline the colours are the same as unselected.
    let same = plain
        .as_chunks::<4>()
        .0
        .iter()
        .zip(marked.as_chunks::<4>().0)
        .filter(|(a, b)| a == b)
        .count();
    assert!(
        same > size[0] as usize * size[1] as usize * 9 / 10,
        "{same}"
    );
    // Walls in front low: more of the floor is seen than with all the
    // walls up, less than with all of them cut low.
    let eye = camera.eye();
    let toward = glam::Vec2::new(eye.x - camera.target.x, eye.y - camera.target.y).normalize();
    let rule = |front: bool| super::cutaway::CutRule {
        top: if front { 10.0 } else { 0.3 },
        front: front.then_some(super::cutaway::Front {
            toward,
            at: toward.dot(camera.target.truncate()),
            low: 0.3,
            above: 0.5,
        }),
    };
    let (front, _) = draw(&BTreeSet::new(), Some(rule(true)));
    let (low, _) = draw(&BTreeSet::new(), Some(rule(false)));
    let differ = |a: &[u8], b: &[u8]| {
        a.as_chunks::<4>()
            .0
            .iter()
            .zip(b.as_chunks::<4>().0)
            .filter(|(x, y)| x != y)
            .count()
    };
    assert!(
        differ(&plain, &front) > 500,
        "front walls not cut: {}",
        differ(&plain, &front)
    );
    assert!(
        differ(&front, &low) > 500,
        "back walls cut too: {}",
        differ(&front, &low)
    );
}

/// With the sun, the walls cast their shadows on the floor (darker than
/// without them), and the finest level draws too.
#[test]
fn shadows_of_the_sun() {
    use super::look::Quality;
    let Some((device, queue, view_formats)) = gpu() else {
        eprintln!("No graphics adapter: 3D drawing test skipped.");
        return;
    };
    let (m, _, _) = room();
    let scene = Scene::build(&m, &BTreeSet::new());
    let size = [400, 300];
    let mut camera = Camera::default();
    camera.fit(scene.lo, scene.hi, size[0] as f32 / size[1] as f32);
    let sun = glam::Vec3::new(-0.6, -0.5, 0.45);
    let draw = |q: Quality| {
        let mut r = Renderer::new(
            &device,
            view_formats,
            crate::view3d::render::TextureMode::Full,
        );
        r.set_scene(&device, &scene);
        r.set_look(q, Some(sun));
        r.render(&device, &queue, &camera, size, 1);
        r.read_pixels(&device, &queue).unwrap().2
    };
    let light = draw(Quality::Light);
    let normal = draw(Quality::Normal);
    let pretty = draw(Quality::Pretty);
    if let Some(dir) = std::env::var_os("ARCHISIM_TEST_OUT") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        for (name, px) in [("ligera", &light), ("normal", &normal), ("bonita", &pretty)] {
            resvg::tiny_skia::Pixmap::from_vec(
                px.clone(),
                resvg::tiny_skia::IntSize::from_wh(size[0], size[1]).unwrap(),
            )
            .unwrap()
            .save_png(dir.join(format!("sombras-{name}.png")))
            .unwrap();
        }
    }
    let luma = |p: &[u8; 4]| u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2]);
    let darker = |a: &[u8], b: &[u8]| {
        a.as_chunks::<4>()
            .0
            .iter()
            .zip(b.as_chunks::<4>().0)
            .filter(|(x, y)| luma(y) + 60 < luma(x))
            .count()
    };
    let shaded = darker(&light, &normal);
    assert!(shaded > 1500, "shadow pixels: {shaded}");
    assert!(darker(&light, &pretty) > 1500);
}

/// The Maqueta's sky, and the night: the sky goes dark and a lamp's shade
/// glows and lights what is around it.
#[test]
fn sky_and_night_with_a_lamp() {
    use super::look::{Atmosphere, Light, Quality};
    let Some((device, queue, view_formats)) = gpu() else {
        eprintln!("No graphics adapter: 3D drawing test skipped.");
        return;
    };
    let (mut m, _, _) = room();
    let level = m.levels()[0].id;
    let lamp = archisim_library::add_object_entry(&mut m, "Lámpara de pie").unwrap();
    let (lo, hi) = {
        let s = Scene::build(&m, &BTreeSet::new());
        (s.lo, s.hi)
    };
    m.add_object(archisim_core::NewObject {
        object_type: lamp,
        level,
        position: archisim_core::Point2::new(
            f64::from((lo.x + hi.x) / 2.0),
            f64::from((lo.y + hi.y) / 2.0),
        ),
        rotation: 0.0,
        base_offset: 0.0,
    })
    .unwrap();
    let scene = Scene::build(&m, &BTreeSet::new());
    assert_eq!(scene.lamps.len(), 1, "one light for the lamp");
    let size = [400, 300];
    let mut camera = Camera::default();
    camera.fit(scene.lo, scene.hi, size[0] as f32 / size[1] as f32);
    camera.pitch = 0.08;
    let draw = |light: Light, camera: &Camera| {
        let mut r = Renderer::new(
            &device,
            view_formats,
            crate::view3d::render::TextureMode::Full,
        );
        r.set_scene(&device, &scene);
        r.set_look(Quality::Normal, Some(glam::Vec3::new(-0.6, -0.5, 0.45)));
        r.set_atmosphere(Some(Atmosphere {
            light,
            ground_z: 0.0,
            grid: Some((glam::Vec2::ZERO, 10.0, 0.5)),
        }));
        r.render(&device, &queue, camera, size, 1);
        r.read_pixels(&device, &queue).unwrap().2
    };
    let day = draw(Light::Day, &camera);
    let night = draw(Light::Night, &camera);
    // From above, into the room, to see the lamp.
    let mut above = camera;
    above.pitch = 1.1;
    let inside = draw(Light::Night, &above);
    if let Some(dir) = std::env::var_os("ARCHISIM_TEST_OUT") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        for (name, px) in [("dia", &day), ("noche", &night), ("noche-dentro", &inside)] {
            resvg::tiny_skia::Pixmap::from_vec(
                px.clone(),
                resvg::tiny_skia::IntSize::from_wh(size[0], size[1]).unwrap(),
            )
            .unwrap()
            .save_png(dir.join(format!("cielo-{name}.png")))
            .unwrap();
        }
    }
    let at = |px: &[u8], x: u32, y: u32| {
        let i = ((y * size[0] + x) * 4) as usize;
        [px[i], px[i + 1], px[i + 2]]
    };
    // The top corner is sky: blue by day, dark at night.
    let sky = at(&day, 2, 2);
    assert!(sky[2] > sky[0] && sky[2] > 150, "day sky {sky:?}");
    let dark = at(&night, 2, 2);
    assert!(
        u32::from(dark[0]) + u32::from(dark[1]) + u32::from(dark[2]) < 250,
        "night sky {dark:?}"
    );
    // At night the picture is darker, but the lamp glows.
    let luma = |p: &[u8; 4]| u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2]);
    let mean =
        |px: &[u8]| px.as_chunks::<4>().0.iter().map(luma).sum::<u32>() / (size[0] * size[1]);
    assert!(
        mean(&night) * 2 < mean(&day),
        "{} {}",
        mean(&night),
        mean(&day)
    );
    let bright = inside
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[0] > 200 && p[1] > 150)
        .count();
    assert!(bright > 30, "glowing pixels: {bright}");
}

/// A streetlight's lantern and a fireplace's fire are lamps at night too,
/// the streetlight's light reaching further.
#[test]
fn streetlights_and_fires_are_lamps() {
    let (mut m, _, _) = room();
    let level = m.levels()[0].id;
    for (name, x) in [("Farola fernandina 3,5 m", -3.0), ("Chimenea de obra", 1.0)] {
        let t = archisim_library::add_object_entry(&mut m, name).unwrap();
        m.add_object(archisim_core::NewObject {
            object_type: t,
            level,
            position: archisim_core::Point2::new(x, -2.0),
            rotation: 0.0,
            base_offset: 0.0,
        })
        .unwrap();
    }
    let scene = Scene::build(&m, &BTreeSet::new());
    let mut reach: Vec<f32> = scene.lamps.iter().map(|l| l[3]).collect();
    reach.sort_by(f32::total_cmp);
    assert_eq!(reach.len(), 2, "{:?}", scene.lamps);
    assert!(reach[1] > reach[0] * 2.0, "{reach:?}");
    // The streetlight's light is up in its lantern.
    let high = scene.lamps.iter().map(|l| l[2]).fold(0.0f32, f32::max);
    assert!(high > 2.5, "{high}");
}

/// Thin lines: the contours of the room are one pixel wide instead of
/// two, so the picture has about half the dark ink.
#[test]
fn thin_lines_draw_less_ink() {
    let Some((device, queue, view_formats)) = gpu() else {
        eprintln!("No graphics adapter: 3D drawing test skipped.");
        return;
    };
    let size = [480, 320];
    let (m, _, _) = room();
    let scene = Scene::build(&m, &BTreeSet::new());
    let mut camera = Camera::default();
    camera.fit(scene.lo, scene.hi, size[0] as f32 / size[1] as f32);
    let mut r = Renderer::new(&device, view_formats, super::render::TextureMode::Full);
    r.set_scene(&device, &scene);
    let mut ink = |thin: bool| {
        r.thin_lines = thin;
        r.render(&device, &queue, &camera, size, 1);
        let (_, _, px) = r.read_pixels(&device, &queue).expect("picture");
        let dark = px
            .chunks(4)
            .filter(|p| p[..3].iter().all(|c| *c < 90))
            .count();
        if let Some(dir) = std::env::var_os("ARCHISIM_TEST_OUT") {
            let pixmap = resvg::tiny_skia::Pixmap::from_vec(
                px.clone(),
                resvg::tiny_skia::IntSize::from_wh(size[0], size[1]).unwrap(),
            )
            .unwrap();
            let name = if thin {
                "vista3d-finas.png"
            } else {
                "vista3d-gruesas.png"
            };
            pixmap
                .save_png(std::path::Path::new(&dir).join(name))
                .unwrap();
        }
        dark
    };
    let thick = ink(false);
    let thin = ink(true);
    assert!(thick > 100, "{thick}");
    assert!(
        (thin as f64) > thick as f64 * 0.3 && (thin as f64) < thick as f64 * 0.8,
        "{thin} / {thick}"
    );
}
