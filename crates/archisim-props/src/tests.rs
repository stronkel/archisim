use super::*;

fn bounds(m: &PropMesh) -> ([f32; 3], [f32; 3]) {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for p in &m.positions {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    (lo, hi)
}

/// Every model of every kind is made, within its measurements, with a
/// sensible number of triangles, all of them pointing outwards (the solid
/// encloses a positive volume).
#[test]
fn every_model_is_made() {
    for (kind, styles) in KINDS {
        for s in 1..=styles.len() {
            let mut p = Params::default();
            p.set("style", s as f64);
            let m = make(kind, &p).unwrap_or_else(|| panic!("{kind}"));
            let n = m.triangle_count();
            assert!(n > 20 && n < 60_000, "{kind} {s}: {n} triangles");
            let mut volume = 0.0f64;
            for (_, tris) in &m.parts {
                for t in tris {
                    let [a, b, c] = t.map(|i| m.positions[i as usize].map(f64::from));
                    volume += (a[0] * (b[1] * c[2] - b[2] * c[1])
                        - a[1] * (b[0] * c[2] - b[2] * c[0])
                        + a[2] * (b[0] * c[1] - b[1] * c[0]))
                        / 6.0;
                }
            }
            assert!(volume > 0.0, "{kind} {s}: volume {volume}");
            let (lo, hi) = bounds(&m);
            assert!(lo[2] >= -1e-3, "{kind} {s}: below the floor {lo:?}");
            // Streetlights stand well over a storey.
            let tallest = match *kind {
                "streetlight" => 17.0,
                "fireplace" | "ceiling_fan" => 4.1,
                _ => 3.0,
            };
            assert!(hi[2] < tallest, "{kind} {s}: {hi:?}");
        }
    }
}

#[test]
fn models_follow_their_measurements() {
    let mut p = Params::default();
    p.set("width", 3.0);
    p.set("places", 4.0);
    let m = make("sofa", &p).unwrap();
    let (lo, hi) = bounds(&m);
    assert!((hi[0] - lo[0] - 3.0).abs() < 0.02, "{lo:?} {hi:?}");
    // A table with six chairs: the chairs stand out in front and behind.
    let mut t = Params::default();
    t.set("chairs", 6.0);
    let m = make("table", &t).unwrap();
    let (lo, hi) = bounds(&m);
    assert!(lo[1] < -0.2 && hi[1] > 1.0, "{lo:?} {hi:?}");
    // A taller person is taller.
    let mut a = Params::default();
    a.set("height", 1.2);
    let (_, hi) = bounds(&make("person", &a).unwrap());
    assert!((hi[2] - 1.2).abs() < 0.08, "{hi:?}");
    // A car as long as asked, its front towards +y.
    let mut c = Params::default();
    c.set("length", 4.0);
    let (lo, hi) = bounds(&make("car", &c).unwrap());
    assert!((hi[1] - lo[1] - 4.0).abs() < 0.1, "{lo:?} {hi:?}");
    // Kept: asked twice, made once.
    let x = mesh_of("bed", &Params::default()).unwrap();
    let y = mesh_of("bed", &Params::default()).unwrap();
    assert!(Arc::ptr_eq(&x, &y));
    assert!(make("spaceship", &Params::default()).is_none());
}

/// Bloque 5: drawers and handles as asked.
#[test]
fn fronts_with_drawers_and_handles() {
    let count = |kind: &str, set: &[(&str, f64)]| {
        let mut p = Params::default();
        for (k, v) in set {
            p.set(k, *v);
        }
        make(kind, &p).unwrap().triangle_count()
    };
    // More drawers, more triangles; no handle, fewer.
    assert!(count("chest", &[("drawers", 6.0)]) > count("chest", &[("drawers", 3.0)]));
    assert!(count("nightstand", &[("handle", 5.0)]) < count("nightstand", &[("handle", 1.0)]));
    assert!(count("wardrobe", &[("drawers", 2.0)]) > count("wardrobe", &[]));
    assert!(count("counter", &[("drawers", 3.0)]) != count("counter", &[]));
    assert!(count("desk", &[("pedestal", 1.0)]) < count("desk", &[("pedestal", 3.0)]));
    for h in 1..=5 {
        assert!(count("wall_cabinet", &[("handle", h as f64)]) > 20);
    }
    assert_eq!(choice_number("handle", "Pomo"), Some(2.0));
    assert_eq!(choice_number("pedestal", "A la izquierda"), Some(2.0));
}

/// The cosy pieces follow their measurements; the sofa takes its loose
/// cushions and its throw.
#[test]
fn cosy_pieces() {
    let make_with = |kind: &str, set: &[(&str, f64)]| {
        let mut p = Params::default();
        for (k, v) in set {
            p.set(k, *v);
        }
        make(kind, &p).unwrap()
    };
    let (lo, hi) = bounds(&make_with("rug", &[("width", 3.0), ("depth", 2.0)]));
    assert!((hi[0] - lo[0] - 3.0).abs() < 0.02 && (hi[1] - lo[1] - 2.0).abs() < 0.02);
    assert!(hi[2] < 0.05);
    let (_, hi) = bounds(&make_with("lamp", &[("height", 1.8)]));
    assert!((hi[2] - 1.8).abs() < 0.1, "{hi:?}");
    let (_, hi) = bounds(&make_with("potted_plant", &[("height", 1.4)]));
    assert!(hi[2] > 1.0 && hi[2] < 1.6, "{hi:?}");
    let plain = make_with("sofa", &[]).triangle_count();
    let dressed = make_with("sofa", &[("pillows", 2.0), ("throw", 1.0)]);
    assert!(dressed.triangle_count() > plain);
    assert!(dressed.parts.iter().any(|(r, _)| *r == "pillow"));
    assert!(dressed.parts.iter().any(|(r, _)| *r == "throw"));
    // Curtains drawn open gather to the sides: less in the middle.
    let shut = make_with("curtain", &[("open", 0.0)]).triangle_count();
    let open = make_with("curtain", &[("open", 1.0)]).triangle_count();
    assert!(open < shut);
}

/// The modular kitchen: each base unit is made as it says, and the fronts'
/// styles change the model.
#[test]
fn kitchen_modules_and_front_styles() {
    let mesh = |unit: f64, style: f64| {
        let mut p = Params::default();
        p.set("unit", unit);
        p.set("front_style", style);
        make("base_unit", &p)
            .unwrap()
            .parts
            .iter()
            .map(|(_, t)| t.len())
            .sum::<usize>()
    };
    let units: Vec<usize> = (1..=8).map(|u| mesh(u as f64, 1.0)).collect();
    let distinct: std::collections::BTreeSet<_> = units.iter().collect();
    assert!(distinct.len() >= 6, "{units:?}");
    let styles: Vec<usize> = (1..=4).map(|s| mesh(2.0, s as f64)).collect();
    let distinct: std::collections::BTreeSet<_> = styles.iter().collect();
    assert_eq!(distinct.len(), 4, "{styles:?}");
    for tall in 1..=3 {
        let mut p = Params::default();
        p.set("tall", tall as f64);
        assert!(make("tall_unit", &p).is_some());
    }
}

/// Nothing in the air: every piece of every model, with its default
/// measurements, is joined through the pieces it touches to the floor (or
/// to the wall or ceiling it hangs from). `cargo test -p archisim-props --
/// nothing_floats --nocapture` lists what is loose.
#[test]
fn nothing_floats() {
    let mut bad = Vec::new();
    for (kind, styles) in KINDS {
        for s in 1..=styles.len() {
            let mut p = Params::default();
            p.set("style", s as f64);
            let m = make(kind, &p).unwrap();
            let loose = check::loose_pieces(&m, check::anchors_for(kind), 0.004);
            if loose.is_empty() {
                continue;
            }
            let worst = loose.iter().map(|l| l.gap).fold(0.0, f64::max);
            let roles: std::collections::BTreeSet<_> = loose.iter().map(|l| l.role).collect();
            println!(
                "{kind} {s} «{}»: {} loose, up to {worst:.3} m off, {roles:?}",
                styles[s - 1],
                loose.len()
            );
            for l in loose.iter().take(3) {
                let (role, lo, hi) = l.near;
                println!(
                    "    {} {:.3} off: x {:.3}..{:.3} y {:.3}..{:.3} z {:.3}..{:.3}  near {role} x {:.3}..{:.3} y {:.3}..{:.3} z {:.3}..{:.3}",
                    l.role, l.gap, l.lo[0], l.hi[0], l.lo[1], l.hi[1], l.lo[2], l.hi[2],
                    lo[0], hi[0], lo[1], hi[1], lo[2], hi[2]
                );
            }
            bad.push(format!("{kind} {s}"));
        }
    }
    println!("{} models with pieces in the air", bad.len());
    assert!(bad.is_empty(), "{bad:?}");
}

/// A collection's kitchen taken to one's taste: each choice over the
/// collection changes the model, and «as the collection» leaves it as it
/// was.
#[test]
fn a_kitchen_to_ones_taste() {
    let size = |kind: &str, unit: f64, set: &[(&str, f64)]| {
        let mut p = Params::default();
        // The nordic collection (after the family's own first model).
        p.set("style", 2.0);
        p.set("unit", unit);
        for (k, v) in set {
            p.set(k, *v);
        }
        let m = make(kind, &p).unwrap();
        // Its shape, roughly: where its points are, part by part.
        let shape: f64 = m
            .parts
            .iter()
            .enumerate()
            .flat_map(|(i, (_, tris))| tris.iter().map(move |t| (i, t)))
            .flat_map(|(i, t)| t.iter().map(move |v| (i, *v)))
            .map(|(i, v)| {
                let q = m.positions[v as usize];
                (i + 1) as f64
                    * ((q[0] as f64).powi(2) * 1.3 + q[1] as f64 * 1.7 + q[2] as f64 * 1.9)
            })
            .sum();
        (m.triangle_count(), (shape * 1000.0).round() as i64)
    };
    let sink = 3.0;
    let hob = 6.0;
    for (kind, key, n, unit) in [
        ("base_unit", "fronts", fine::kitchen::FRONTS.len(), 2.0),
        ("base_unit", "pulls", fine::kitchen::PULLS.len(), 1.0),
        ("base_unit", "stand", fine::kitchen::STANDS.len(), 1.0),
        ("base_unit", "basin", fine::kitchen::BASINS.len(), sink),
        ("base_unit", "tap", fine::kitchen::TAPS.len(), sink),
        ("base_unit", "hob", fine::kitchen::HOBS.len(), hob),
        (
            "wall_cabinet",
            "shelving",
            fine::kitchen::SHELVINGS.len(),
            1.0,
        ),
        ("hood", "canopy", fine::kitchen::CANOPIES.len(), 1.0),
    ] {
        let style = if kind == "hood" { 3.0 } else { 2.0 };
        let mut seen = std::collections::BTreeSet::new();
        let mut all = Vec::new();
        for i in 1..=n {
            let s = size(kind, unit, &[("style", style), (key, i as f64)]);
            seen.insert(s);
            all.push(s);
        }
        // The collection's own is one of the others (the nordic's), so
        // n − 1 different models at least.
        assert!(
            seen.len() >= n - 1,
            "{kind} {key}: {} of {n} {all:?}",
            seen.len()
        );
    }
    // «As the collection» is the collection.
    assert_eq!(
        size("base_unit", 1.0, &[("pulls", 1.0)]),
        size("base_unit", 1.0, &[])
    );
}

/// Whatever is chosen over a collection, nothing of the kitchen is left
/// in the air.
#[test]
fn a_kitchen_to_ones_taste_stands() {
    use fine::kitchen::*;
    let mut bad = Vec::new();
    // A kind, the units tried and its choices with how many options.
    type Case<'a> = (&'a str, &'a [f64], &'a [(&'a str, usize)]);
    let cases: &[Case] = &[
        (
            "base_unit",
            &[1.0, 2.0, 3.0, 6.0],
            &[
                ("fronts", FRONTS.len()),
                ("pulls", PULLS.len()),
                ("stand", STANDS.len()),
                ("basin", BASINS.len()),
                ("tap", TAPS.len()),
                ("hob", HOBS.len()),
            ],
        ),
        (
            "tall_unit",
            &[1.0, 2.0],
            &[
                ("fronts", FRONTS.len()),
                ("pulls", PULLS.len()),
                ("stand", STANDS.len()),
            ],
        ),
        (
            "wall_cabinet",
            &[1.0],
            &[
                ("fronts", FRONTS.len()),
                ("pulls", PULLS.len()),
                ("shelving", SHELVINGS.len()),
            ],
        ),
        (
            "hood",
            &[1.0],
            &[("canopy", CANOPIES.len()), ("fronts", FRONTS.len())],
        ),
    ];
    for (kind, units, keys) in cases {
        let first = if *kind == "hood" { 2.0 } else { 1.0 };
        for c in 1..=8 {
            for unit in *units {
                for (key, n) in *keys {
                    for i in 2..=*n {
                        let mut p = Params::default();
                        p.set("style", first + c as f64);
                        p.set("unit", *unit);
                        p.set("tall", *unit);
                        p.set(key, i as f64);
                        let m = make(kind, &p).unwrap();
                        let loose = check::loose_pieces(&m, check::anchors_for(kind), 0.004);
                        if let Some(l) = loose.first() {
                            bad.push(format!(
                                "{kind} colección {c} unidad {unit} {key}={i}: {} a {:.3}",
                                l.role, l.gap
                            ));
                        }
                    }
                }
            }
        }
    }
    for b in &bad {
        println!("{b}");
    }
    assert!(bad.is_empty(), "{} loose", bad.len());
}

/// Every door of every collection, and every leaf, casing, handle and
/// hinge over them, stands whole: nothing in the air.
#[test]
fn doors_stand_whole() {
    use fine::door::*;
    let mut bad = Vec::new();
    let mut most = 0;
    for style in 1..=8 {
        for (key, n) in [
            ("leaf_design", DESIGNS.len()),
            ("casing", CASINGS.len()),
            ("handle_style", HANDLES.len()),
            ("hinges", HINGES.len()),
        ] {
            for i in 1..=n {
                for leaves in [1.0, 2.0] {
                    let mut p = Params::default();
                    p.set("style", style as f64);
                    p.set(key, i as f64);
                    p.set("leaves", leaves);
                    p.set("leaf_x0", -0.4);
                    p.set("leaf_x1", 0.4);
                    p.set("leaf_split", 0.0);
                    p.set("lining_w", 0.9);
                    p.set("lining_h", 2.08);
                    p.set("leaf_top", 2.03);
                    p.set("face_pos", 0.06);
                    p.set("face_neg", -0.06);
                    p.set("peephole", 1.0);
                    // A closer and kick plates on every third.
                    if i % 3 == 0 {
                        p.set("closer", 1.0);
                        p.set("kick_plate", 1.0);
                    }
                    // Every other one as seen from far away.
                    if i % 2 == 0 {
                        p.set("lod", 1.0);
                    }
                    let m = make("door", &p).unwrap();
                    most = most.max(m.triangle_count());
                    let loose = check::loose_pieces(&m, check::anchors_for("door"), 0.004);
                    if let Some(l) = loose.first() {
                        bad.push(format!(
                            "colección {style} {key}={i} hojas {leaves}: {} a {:.3} ({:?}..{:?})",
                            l.role, l.gap, l.lo, l.hi
                        ));
                    }
                }
            }
        }
    }
    for b in &bad {
        println!("{b}");
    }
    println!("most triangles: {most}");
    assert!(bad.is_empty(), "{} loose", bad.len());
}

#[test]
#[ignore]
fn door_weights() {
    for style in 1..=8 {
        for d in 1..=fine::door::DESIGNS.len() {
            let mut p = Params::default();
            p.set("style", style as f64);
            p.set("leaf_design", d as f64);
            let m = make("door", &p).unwrap();
            println!("style {style} design {d}: {}", m.triangle_count());
        }
    }
}

/// Every window of every collection, with every profile, glazing bars,
/// handle and kind of sash, stands whole; and what goes outside it (the
/// roller blind, the shutters, the grilles flat or boxed) holds together.
#[test]
fn windows_stand_whole() {
    use fine::window::*;
    let base = |style: usize, leaves: f64| {
        let mut p = Params::default();
        p.set("style", style as f64);
        p.set("leaves", leaves);
        p.set("frame_depth", 0.07);
        p.set("w_fw", 0.06);
        p.set("w_gc0", -0.54);
        p.set("w_gc1", 0.54);
        p.set("w_w", 1.2);
        p.set("w_z0", 0.0);
        p.set("w_z1", 1.2);
        p.set("w_zl", 1.2);
        p.set("w_outside", -1.0);
        p.set("w_width", 1.2);
        p.set("w_height", 1.2);
        p.set("w_top", 1.2);
        p.set("face_ext", -0.14);
        p
    };
    let mut bad = Vec::new();
    let mut most = 0;
    let mut check = |p: &Params, what: String, only: Option<&str>| {
        let m = make("window", p).unwrap();
        if m.triangle_count() > most {
            println!("{what}: {} triangles", m.triangle_count());
        }
        most = most.max(m.triangle_count());
        let loose = check::loose_pieces(&m, check::anchors_for("window"), 0.004);
        if let Some(l) = loose.iter().find(|l| only.is_none_or(|r| l.role == r)) {
            bad.push(format!(
                "{what}: {} a {:.3} ({:?}..{:?})",
                l.role, l.gap, l.lo, l.hi
            ));
        }
    };
    for style in 1..=8 {
        for (key, n) in [
            ("window_profile", PROFILES.len()),
            ("glazing_bars", BARS.len()),
            ("window_handle", HANDLES.len()),
        ] {
            for i in 1..=n {
                for leaves in [1.0, 2.0] {
                    let mut p = base(style, leaves);
                    p.set(key, i as f64);
                    check(
                        &p,
                        format!("colección {style} {key}={i} hojas {leaves}"),
                        None,
                    );
                }
            }
        }
        for op in 1..=OPERATIONS.len() {
            let mut p = base(style, 2.0);
            p.set("leaf_1", op as f64);
            p.set("leaf_2", op as f64);
            check(&p, format!("colección {style} hoja {op}"), None);
        }
    }
    // What goes outside, each on its own (the wall that holds them is not
    // in the model).
    for down in [0.0, 50.0, 100.0] {
        let mut p = base(5, 2.0);
        p.set("shutter", 1.0);
        p.set("shutter_down", down);
        p.set("w_z1", 0.95);
        p.set("w_zl", 0.95);
        p.set("w_top", 0.95);
        check(&p, format!("persiana {down}%"), None);
    }
    for kind in 2..=SHUTTERS_KINDS.len() {
        for open in [0.0, 1.0] {
            let mut p = base(2, 2.0);
            p.set("outside_shutters", kind as f64);
            p.set("shutters_open", open);
            check(
                &p,
                format!("contraventanas {kind} abiertas {open}"),
                Some("shutters_material"),
            );
        }
    }
    for kind in 2..=GRILLES.len() {
        for boxed in [0.0, 1.0] {
            let mut p = base(6, 2.0);
            p.set("grille", kind as f64);
            p.set("grille_box", boxed);
            check(
                &p,
                format!("reja {kind} de cajón {boxed}"),
                Some("grille_material"),
            );
        }
    }
    // And their plain versions for far away.
    for style in 1..=8 {
        let mut p = base(style, 2.0);
        p.set("lod", 1.0);
        check(&p, format!("colección {style} de lejos"), None);
    }
    for (key, n) in [
        ("outside_shutters", SHUTTERS_KINDS.len()),
        ("grille", GRILLES.len()),
    ] {
        for kind in 2..=n {
            let mut p = base(6, 2.0);
            p.set(key, kind as f64);
            p.set("grille_box", 1.0);
            p.set("lod", 1.0);
            let role = if key == "grille" {
                "grille_material"
            } else {
                "shutters_material"
            };
            check(&p, format!("{key} {kind} de lejos"), Some(role));
        }
    }
    for b in &bad {
        println!("{b}");
    }
    println!("most triangles: {most}");
    assert!(bad.is_empty(), "{} loose", bad.len());
}
