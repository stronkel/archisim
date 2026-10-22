//! Draws a real picture with whatever graphics adapter the machine has (a
//! software one is enough). Skipped, with a note, if there is none.

use std::collections::BTreeSet;

use eframe::wgpu;

use super::camera::Camera;
use super::render::Renderer;
use super::scene::Scene;
use crate::pick::tests::room;

fn gpu() -> Option<(wgpu::Device, wgpu::Queue, bool)> {
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
    let mut r = Renderer::new(&device, view_formats, true);
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
    let mut r = Renderer::new(&device, view_formats, true);
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

#[test]
fn shaders_without_textures_are_valid() {
    let source = super::render::shader_source(false);
    assert!(!source.contains("fn cells("));
    let module = naga::front::wgsl::parse_str(&source).expect("parses");
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .expect("valid");
    let full = super::render::shader_source(true);
    let module = naga::front::wgsl::parse_str(&full).expect("parses");
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .expect("valid");
}
