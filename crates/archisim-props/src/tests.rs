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
            assert!(hi[2] < 3.0, "{kind} {s}: {hi:?}");
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
