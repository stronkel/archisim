use std::path::Path;

use super::*;

fn fixture(name: &str) -> Mesh {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    read_file(&p, Options::default()).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn size(m: &Mesh) -> [f32; 3] {
    let (lo, hi) = m.bounds();
    [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]]
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn obj_chair_with_its_texture() {
    let m = fixture("silla.obj");
    let s = size(&m);
    // y up in the file: 0.90 high, 0.45 wide and deep.
    assert!(
        close(s[0], 0.45) && close(s[1], 0.45) && close(s[2], 0.90),
        "{s:?}"
    );
    assert_eq!(m.materials.len(), 1);
    assert_eq!(m.materials[0].texture.as_deref(), Some("madera.png"));
    // The wood's browns, not the grey of Kd.
    let c = m.materials[0].color;
    assert!(c[0] > c[2] + 40, "{c:?}");
    assert!(
        m.colors.iter().any(|v| v != &m.colors[0]),
        "sampled per vertex"
    );
    // 6 boxes of 12 triangles.
    assert_eq!(m.triangle_count(), 72);
}

#[test]
fn collada_tree_in_inches_with_a_component() {
    let m = fixture("arbol.dae");
    let s = size(&m);
    // Inches to metres; z up: 4 m tall, 2.5 m across.
    assert!(close(s[2], 4.0) && close(s[0], 2.5), "{s:?}");
    let names: Vec<&str> = m.materials.iter().map(|x| x.name.as_str()).collect();
    assert!(
        names.contains(&"Corteza") && names.contains(&"Hojas"),
        "{names:?}"
    );
    let leaf = m.materials.iter().find(|x| x.name == "Hojas").unwrap();
    assert!(leaf.color[1] > leaf.color[0] && leaf.color[1] > leaf.color[2]);
}

#[test]
fn gltf_and_glb_car() {
    for name in ["coche.glb", "coche.gltf"] {
        let m = fixture(name);
        let s = size(&m);
        // y up: 4.2 long, 1.8 wide, 1.5 high; moved 10 m along x.
        assert!(
            close(s[0], 4.2) && close(s[1], 1.8) && close(s[2], 1.5),
            "{name}: {s:?}"
        );
        let (lo, _) = m.bounds();
        assert!(close(lo[0], 10.0 - 2.1), "{lo:?}");
        let glass = m.materials.iter().find(|x| x.name == "Vidrio").unwrap();
        assert!((glass.alpha - 0.3).abs() < 1e-3);
        let body = m.materials.iter().find(|x| x.name == "Carrocería").unwrap();
        // Linear 0.6 is a brighter sRGB red.
        assert!(
            body.color[0] > 190 && body.color[1] < 40,
            "{:?}",
            body.color
        );
    }
}

#[test]
fn faces_keep_facing_out() {
    // A closed mesh read from each format has a positive volume (its
    // faces turn outwards) after the change of axes.
    for name in ["silla.obj", "arbol.dae", "coche.glb"] {
        let m = fixture(name);
        let v: f64 = m
            .groups
            .iter()
            .flat_map(|g| &g.triangles)
            .map(|t| {
                let [a, b, c] = t.map(|i| m.positions[i as usize].map(|x| x as f64));
                (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.0
            })
            .sum();
        assert!(v > 0.0, "{name}: {v}");
    }
}

#[test]
fn lighter_levels_and_the_outline_in_plan() {
    let mut m = fixture("arbol.dae");
    m.centre_on_base();
    let (lo, hi) = m.bounds();
    assert!(close(lo[2], 0.0) && close(lo[0] + hi[0], 0.0));
    let n = m.triangle_count();
    let light = simplify(&m, n / 4);
    assert!(light.triangle_count() <= n / 4 && light.triangle_count() > 8);
    // About the same size.
    let (a, b) = (size(&m), size(&light));
    assert!((a[2] - b[2]).abs() < 0.4, "{a:?} {b:?}");
    // The canopy seen from above: about a circle of 1.25 m.
    let o = outline(&m, 0.02);
    assert_eq!(o.len(), 1);
    let ring = &o[0].0;
    let area: f64 = (0..ring.len())
        .map(|i| {
            let (p, q) = (ring[i], ring[(i + 1) % ring.len()]);
            p[0] * q[1] - q[0] * p[1]
        })
        .sum::<f64>()
        / 2.0;
    let circle = std::f64::consts::PI * 1.25 * 1.25;
    assert!(
        area > 0.0 && (area - circle).abs() / circle < 0.12,
        "{area} vs {circle}"
    );
    // A big model gets two lighter levels, each lighter.
    let mut b = Builder::default();
    let mat = b.material("x", || Material {
        name: String::new(),
        color: [1, 2, 3],
        alpha: 1.0,
        texture: None,
    });
    for i in 0..120 {
        for j in 0..120 {
            let p = |x: i32, y: i32| {
                [
                    x as f32 * 0.01,
                    y as f32 * 0.01,
                    ((x * y) % 7) as f32 * 0.001,
                ]
            };
            let a = b.vertex(p(i, j), [0, 0, 0]);
            let c = b.vertex(p(i + 1, j), [0, 0, 0]);
            let d = b.vertex(p(i, j + 1), [0, 0, 0]);
            b.triangle(mat, [a, c, d]);
        }
    }
    let levels = levels_of_detail(&b.mesh);
    assert_eq!(levels.len(), 3);
    assert!(levels[1].triangle_count() <= simplify::LEVELS[0]);
    assert!(levels[2].triangle_count() <= simplify::LEVELS[1]);
    assert!(levels[2].triangle_count() < levels[1].triangle_count());
}

#[test]
fn bad_files_say_why() {
    assert_eq!(
        read(b"v 0 0 0\n", "obj", &NoResources, Options::default()),
        Err(Error::Empty)
    );
    assert!(matches!(
        read(b"x", "skp", &NoResources, Options::default()),
        Err(Error::Format(_))
    ));
    assert!(matches!(
        read(b"{", "gltf", &NoResources, Options::default()),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        read(b"<a", "dae", &NoResources, Options::default()),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn written_glb_reads_back_the_same() {
    for name in ["arbol.dae", "silla.obj"] {
        let m = fixture(name);
        let glb = write_glb(&m, "prueba");
        let back = read(&glb, "glb", &NoResources, Options::default()).unwrap();
        assert_eq!(back.triangle_count(), m.triangle_count());
        assert_eq!(back.materials.len(), m.materials.len());
        let (a, b) = (m.bounds(), back.bounds());
        for k in 0..3 {
            assert!((a.0[k] - b.0[k]).abs() < 1e-5 && (a.1[k] - b.1[k]).abs() < 1e-5);
        }
        // The colours of the corners, material by material, in order.
        let corners = |m: &Mesh| -> Vec<Rgb> {
            m.groups
                .iter()
                .flat_map(|g| g.triangles.iter().flatten().map(|&i| m.colors[i as usize]))
                .collect()
        };
        for (x, y) in corners(&m).iter().zip(&corners(&back)) {
            for k in 0..3 {
                assert!(
                    (x[k] as i32 - y[k] as i32).abs() <= 1,
                    "{name}: {x:?} {y:?}"
                );
            }
        }
        for x in &m.materials {
            let y = back.materials.iter().find(|y| y.name == x.name).unwrap();
            for k in 0..3 {
                assert!(
                    (x.color[k] as i32 - y.color[k] as i32).abs() <= 3,
                    "{:?} {:?}",
                    x.color,
                    y.color
                );
            }
        }
    }
}

/// What Blender says of the chair of `butaca.*` (scripts/make_design_fixtures.py).
fn blender_chair() -> ([f32; 3], [f32; 3], Vec<(String, usize)>) {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/butaca.json");
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
    let three = |k: &str| {
        let a = v[k].as_array().unwrap();
        [0, 1, 2].map(|i| a[i].as_f64().unwrap() as f32)
    };
    let mut faces: Vec<(String, usize)> = v["faces"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, n)| (k.clone(), n.as_u64().unwrap() as usize))
        .collect();
    faces.sort();
    (three("lo"), three("hi"), faces)
}

fn faces_by_material(m: &Mesh) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = m
        .groups
        .iter()
        .map(|g| (m.materials[g.material].name.clone(), g.triangles.len()))
        .collect();
    out.sort();
    out
}

fn same_box(m: &Mesh, lo: [f32; 3], hi: [f32; 3], tol: f32) {
    let (a, b) = m.bounds();
    for k in 0..3 {
        assert!(
            (a[k] - lo[k]).abs() < tol && (b[k] - hi[k]).abs() < tol,
            "box {a:?} {b:?}, Blender {lo:?} {hi:?}"
        );
    }
}

#[test]
fn binary_fbx_in_place_with_its_materials_and_texture() {
    let m = fixture("butaca.fbx");
    let (lo, hi, faces) = blender_chair();
    // Centimetres, y up, parts turned −90° and scaled 100, a hierarchy
    // turned 30°: where Blender has it, in metres and z up.
    same_box(&m, lo, hi, 1e-4);
    assert_eq!(faces_by_material(&m), faces);
    let mat = |n: &str| m.materials.iter().find(|x| x.name == n).unwrap();
    let leather = mat("Cuero coñac").color;
    assert!(
        leather[0] > leather[1] && leather[1] > leather[2],
        "{leather:?}"
    );
    assert!(mat("Vidrio").alpha < 0.5);
    // The walnut's texture, from inside the file: its two browns.
    let walnut = m
        .groups
        .iter()
        .find(|g| m.materials[g.material].name == "Nogal")
        .unwrap();
    let mut seen: Vec<Rgb> = walnut
        .triangles
        .iter()
        .flatten()
        .map(|&i| m.colors[i as usize])
        .collect();
    seen.sort();
    seen.dedup();
    assert!(seen.len() >= 2, "{seen:?}");
    assert!(seen.iter().all(|c| c[0] > c[2]), "{seen:?}");
    // Faces outward: the closed parts enclose a positive volume, as
    // Blender's (seat 0.8 × 0.8 × 0.16 and back 0.8 × 0.12 × 0.6).
    let volume: f32 = m
        .groups
        .iter()
        .filter(|g| m.materials[g.material].name == "Cuero coñac")
        .flat_map(|g| &g.triangles)
        .map(|t| {
            let [a, b, c] = t.map(|i| m.positions[i as usize]);
            (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.0
        })
        .sum();
    assert!(
        (volume - (0.8 * 0.8 * 0.16 + 0.8 * 0.12 * 0.6)).abs() < 1e-3,
        "{volume}"
    );
}

#[test]
fn text_fbx_as_the_binary_one() {
    let a = fixture("butaca.fbx");
    let b = fixture("butaca-ascii.fbx");
    let (lo, hi, _) = blender_chair();
    same_box(&b, lo, hi, 1e-3);
    // z up, as 3ds Max writes them.
    let z = fixture("butaca-z.fbx");
    same_box(&z, lo, hi, 1e-4);
    assert_eq!(a.triangle_count(), z.triangle_count());
    assert_eq!(a.triangle_count(), b.triangle_count());
    assert_eq!(a.materials.len(), b.materials.len());
}

#[test]
fn three_ds_and_stl_in_millimetres_are_guessed() {
    let (lo, hi, faces) = blender_chair();
    for f in ["butaca.3ds", "butaca.stl"] {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(f);
        let l = load(&p, None, Options::default()).unwrap();
        assert_eq!(l.guessed_unit, Some(0.001), "{f}");
        same_box(&l.mesh, lo, hi, 2e-3);
        assert_eq!(
            l.mesh.triangle_count(),
            faces.iter().map(|f| f.1).sum::<usize>(),
            "{f}"
        );
    }
    let m = fixture("butaca.3ds");
    assert_eq!(m.materials.len(), 3);
    assert!(m.materials.iter().any(|x| x.alpha < 0.5));
    // Given in metres: as it is, 1000 times too big.
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/butaca.stl");
    let big = read_file(
        &p,
        Options {
            unit: Some(1.0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(big.bounds().1[2] > 1000.0);
}

#[test]
fn a_manufacturers_zip_without_unpacking() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/butaca.zip");
    let l = load(&p, None, Options::default()).unwrap();
    // The FBX first; its texture as a file next to it, inside the zip.
    assert_eq!(l.entry.as_deref(), Some("Butaca Ejemplo/FBX/butaca.fbx"));
    assert_eq!(l.models.len(), 2);
    assert!(l.others.is_empty());
    let walnut = l.mesh.materials.iter().find(|m| m.name == "Nogal").unwrap();
    assert!(walnut.texture.is_some());
    let (lo, hi, _) = blender_chair();
    same_box(&l.mesh, lo, hi, 1e-4);
    // The 3DS inside, chosen.
    let l = load(
        &p,
        Some("Butaca Ejemplo/3DS/butaca.3ds"),
        Options::default(),
    )
    .unwrap();
    assert_eq!(l.guessed_unit, Some(0.001));
    same_box(&l.mesh, lo, hi, 2e-3);
}

#[test]
fn units_by_size() {
    assert_eq!(plausible_unit([0.9, 0.8, 0.75]), 1.0);
    assert_eq!(plausible_unit([90.0, 80.0, 75.0]), 0.01);
    assert_eq!(plausible_unit([900.0, 800.0, 750.0]), 0.01);
    assert_eq!(plausible_unit([2000.0, 900.0, 750.0]), 0.001);
    assert_eq!(plausible_unit([8.0, 2.0, 12.0]), 1.0);
}
