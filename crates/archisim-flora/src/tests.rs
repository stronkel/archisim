use super::*;

fn olive() -> Species {
    Species {
        habit: Habit::Tree,
        crown: CrownShape::Irregular,
        foliage: Foliage::Fine,
        height: 5.0,
        width: 5.5,
        trunk: 0.45,
        clear: 1.2,
        stems: 1,
        lean: 0.3,
        gnarl: 0.85,
        limbs: 4,
        levels: 3,
        angle: 50.0,
        droop: 0.1,
        density: 0.6,
        clump: 0.45,
        ..Default::default()
    }
}

fn stone_pine() -> Species {
    Species {
        habit: Habit::Tree,
        crown: CrownShape::Umbrella,
        foliage: Foliage::Needles,
        height: 14.0,
        width: 12.0,
        trunk: 0.6,
        clear: 9.0,
        lean: 0.25,
        gnarl: 0.3,
        limbs: 6,
        levels: 3,
        angle: 55.0,
        droop: -0.2,
        density: 0.8,
        clump: 0.8,
        ..Default::default()
    }
}

fn cypress() -> Species {
    Species {
        habit: Habit::Conifer,
        crown: CrownShape::Columnar,
        foliage: Foliage::Scales,
        height: 12.0,
        width: 2.2,
        trunk: 0.35,
        clear: 0.3,
        limbs: 5,
        levels: 2,
        angle: 20.0,
        droop: -0.3,
        density: 0.95,
        clump: 0.5,
        ..Default::default()
    }
}

fn date_palm() -> Species {
    Species {
        habit: Habit::Palm,
        crown: CrownShape::Round,
        foliage: Foliage::Pinnate,
        height: 10.0,
        width: 7.0,
        trunk: 0.5,
        clear: 8.0,
        lean: 0.2,
        fronds: 26,
        frond_length: 4.0,
        fruit: 0.4,
        ..Default::default()
    }
}

fn fan_palm() -> Species {
    Species {
        habit: Habit::Palm,
        foliage: Foliage::Fan,
        height: 3.0,
        width: 3.0,
        trunk: 0.25,
        stems: 4,
        fronds: 14,
        frond_length: 1.4,
        ..Default::default()
    }
}

fn rosemary() -> Species {
    Species {
        habit: Habit::Shrub,
        crown: CrownShape::Mound,
        foliage: Foliage::Fine,
        height: 1.0,
        width: 1.2,
        trunk: 0.04,
        clear: 0.0,
        stems: 9,
        limbs: 9,
        levels: 2,
        angle: 35.0,
        droop: -0.3,
        density: 0.85,
        clump: 0.14,
        bloom: 0.35,
        ..Default::default()
    }
}

fn broom() -> Species {
    Species {
        habit: Habit::Shrub,
        crown: CrownShape::Mound,
        foliage: Foliage::Broom,
        height: 2.0,
        width: 2.2,
        trunk: 0.08,
        stems: 8,
        levels: 3,
        angle: 30.0,
        droop: 0.2,
        density: 0.6,
        clump: 0.15,
        bloom: 0.7,
        ..Default::default()
    }
}

fn samphire() -> Species {
    Species {
        habit: Habit::Cushion,
        crown: CrownShape::Cushion,
        foliage: Foliage::Succulent,
        height: 0.35,
        width: 0.8,
        trunk: 0.02,
        stems: 14,
        levels: 2,
        angle: 55.0,
        density: 0.9,
        clump: 0.07,
        ..Default::default()
    }
}

fn grass() -> Species {
    Species {
        habit: Habit::Grass,
        foliage: Foliage::Blades,
        height: 0.9,
        width: 1.0,
        angle: 45.0,
        droop: 0.5,
        fronds: 50,
        frond_length: 0.9,
        bloom: 0.5,
        ..Default::default()
    }
}

fn agave() -> Species {
    Species {
        habit: Habit::Rosette,
        foliage: Foliage::Blades,
        height: 1.4,
        width: 2.2,
        clear: 0.0,
        fronds: 28,
        frond_length: 1.2,
        ..Default::default()
    }
}

fn jacaranda() -> Species {
    Species {
        crown: CrownShape::Spreading,
        height: 9.0,
        width: 9.0,
        trunk: 0.4,
        clear: 2.5,
        limbs: 5,
        angle: 50.0,
        density: 0.55,
        clump: 0.55,
        bloom: 0.75,
        ..Default::default()
    }
}

pub fn all() -> Vec<(&'static str, Species)> {
    vec![
        ("olivo", olive()),
        ("pino", stone_pine()),
        ("cipres", cypress()),
        ("datilera", date_palm()),
        ("palmito", fan_palm()),
        ("romero", rosemary()),
        ("retama", broom()),
        ("salicornia", samphire()),
        ("gramínea", grass()),
        ("pita", agave()),
        ("jacaranda", jacaranda()),
        ("frondoso", Species::default()),
    ]
}

#[test]
fn names_round_trip() {
    for h in Habit::ALL {
        assert_eq!(Habit::from_name(h.name()), Some(*h));
    }
    for c in CrownShape::ALL {
        assert_eq!(CrownShape::from_name(&c.name().to_uppercase()), Some(*c));
    }
    for f in Foliage::ALL {
        assert_eq!(Foliage::from_name(f.name()), Some(*f));
    }
    for s in PlanStyle::ALL {
        assert_eq!(PlanStyle::from_name(s.name()), Some(*s));
    }
    for l in Look::ALL {
        assert_eq!(Look::from_name(l.name()), Some(*l));
    }
}

#[test]
fn same_seed_same_plant_other_seed_other_plant() {
    for (name, s) in all() {
        let a = grow(&s, 7, 0.5);
        let b = grow(&s, 7, 0.5);
        assert_eq!(a, b, "{name}");
        let c = grow(&s, 8, 0.5);
        assert_ne!(a, c, "{name}");
    }
}

#[test]
fn plants_are_their_size() {
    for (name, s) in all() {
        let p = grow(&s, 3, 0.5);
        assert!(
            p.height > s.height * 0.6 && p.height < s.height * 1.35,
            "{name}: alto {} de {}",
            p.height,
            s.height
        );
        assert!(
            p.radius > s.width * 0.25 && p.radius < s.width * 0.9 + s.trunk,
            "{name}: radio {} de ancho {}",
            p.radius,
            s.width
        );
    }
}

#[test]
fn meshes_get_lighter() {
    let colors = Colors::default();
    for (name, s) in all() {
        let p = grow(&s, 11, 0.5);
        let full = p.mesh(&colors, Lod::Full);
        let med = p.mesh(&colors, Lod::Medium);
        let low = p.mesh(&colors, Lod::Low);
        eprintln!(
            "{name}: {} ({} madera, {} hojas) / {} / {}",
            full.triangle_count(),
            full.bark.len(),
            full.leaves.len(),
            med.triangle_count(),
            low.triangle_count()
        );
        assert!(full.triangle_count() > 0, "{name}");
        assert!(med.triangle_count() <= full.triangle_count(), "{name}");
        assert!(low.triangle_count() <= med.triangle_count(), "{name}");
        assert!(
            low.triangle_count() < 3000,
            "{name}: {}",
            low.triangle_count()
        );
        assert!(
            full.triangle_count() < 90_000,
            "{name}: {}",
            full.triangle_count()
        );
        for m in [&full, &med, &low] {
            assert_eq!(m.positions.len(), m.colors.len());
            let n = m.positions.len() as u32;
            assert!(m
                .bark
                .iter()
                .chain(&m.leaves)
                .chain(&m.flowers)
                .flatten()
                .all(|&i| i < n));
            assert!(m.positions.iter().flatten().all(|v| v.is_finite()));
        }
        eprintln!(
            "{name}: {} / {} / {} triángulos",
            full.triangle_count(),
            med.triangle_count(),
            low.triangle_count()
        );
    }
}

#[test]
fn every_style_draws_something() {
    for (name, s) in all() {
        let p = grow(&s, 5, 0.5);
        for style in PlanStyle::ALL {
            for look in Look::ALL {
                let shapes = p.plan(&PlanOptions {
                    style: *style,
                    look: *look,
                    opacity: 0.6,
                    crown_mm: 80.0,
                });
                assert!(!shapes.is_empty(), "{name} {style:?} {look:?}");
                for sh in &shapes {
                    let pts = match sh {
                        Shape::Line { points, .. } | Shape::Fill { points, .. } => points,
                    };
                    assert!(
                        pts.iter().flatten().all(|v| v.is_finite()),
                        "{name} {style:?}"
                    );
                    // Within the plant, more or less.
                    assert!(
                        pts.iter().all(|q| q[0].hypot(q[1]) < s.width * 1.2 + 0.5),
                        "{name} {style:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn small_on_paper_is_simple() {
    let p = grow(&olive(), 1, 0.5);
    let big = p.plan(&PlanOptions {
        style: PlanStyle::Leaves,
        crown_mm: 100.0,
        ..Default::default()
    });
    let small = p.plan(&PlanOptions {
        style: PlanStyle::Leaves,
        crown_mm: 4.0,
        ..Default::default()
    });
    assert!(
        small.len() < 5 && big.len() > 30,
        "{} {}",
        small.len(),
        big.len()
    );
}

#[test]
fn foliage_masses_have_outer_loops() {
    let p = grow(&Species::default(), 2, 0.5);
    let loops = p.foliage_loops(0.05);
    assert!(!loops.is_empty());
    let outer: f64 = loops
        .iter()
        .map(|l| plan_area(l))
        .filter(|a| *a > 0.0)
        .sum();
    // Most of the crown is inside an outer loop.
    let crown = std::f64::consts::PI * (p.radius * 0.7).powi(2);
    assert!(outer > crown * 0.5, "{outer} {crown}");
}

fn plan_area(poly: &[[f64; 2]]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| poly[i][0] * poly[(i + 1) % n][1] - poly[(i + 1) % n][0] * poly[i][1])
        .sum::<f64>()
        / 2.0
}

#[test]
fn sized_species_keep_proportions() {
    let s = stone_pine().sized(7.0, 6.0);
    assert!((s.clear - 4.5).abs() < 1e-9);
    let p = grow(&s, 1, 0.3);
    assert!(p.height < 9.0);
}

#[test]
fn nonsense_still_grows() {
    let s = Species {
        height: f64::NAN,
        width: -3.0,
        levels: 99,
        limbs: 0,
        fronds: 0,
        ..Default::default()
    };
    let p = grow(&s, 0, 2.0);
    assert!(!p.mesh(&Colors::default(), Lod::Full).positions.is_empty());
}

/// With FLORA_OUT=<carpeta>, draws every test species in plan in every
/// style (SVG) and in 3D at each level of detail (PPM), to look at them.
#[test]
fn pictures() {
    let Ok(dir) = std::env::var("FLORA_OUT") else {
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    let colors = Colors {
        leaves: [96, 132, 80],
        bark: [110, 88, 66],
        flowers: [150, 130, 205],
        fruit: [235, 140, 40],
    };
    for (name, s) in all() {
        let p = grow(&s, 4, 0.5);
        // Plan: a row per look, a column per style.
        let cell = 140.0;
        let mut svg = String::new();
        let styles = PlanStyle::ALL;
        let looks = [Look::Lines, Look::SeeThrough, Look::Colour];
        svg.push_str(&format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='{}' height='{}' style='background:white'>\n",
            cell * styles.len() as f64,
            cell * looks.len() as f64
        ));
        let k = cell * 0.9 / (p.radius * 2.0).max(0.1);
        for (j, look) in looks.iter().enumerate() {
            for (i, style) in styles.iter().enumerate() {
                let shapes = p.plan(&PlanOptions {
                    style: *style,
                    look: *look,
                    opacity: 0.6,
                    crown_mm: 70.0,
                });
                let ox = cell * (i as f64 + 0.5);
                let oy = cell * (j as f64 + 0.5);
                svg.push_str(&svg_paths(&shapes, ox, oy, k, &colors));
            }
        }
        svg.push_str("</svg>\n");
        std::fs::write(format!("{dir}/{name}-planta.svg"), svg).unwrap();
        // Seen from the side: a row per look, a column per style.
        let span = p.height.max(p.radius * 2.0).max(0.1);
        let k = cell * 0.9 / span;
        let mut side = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='{}' height='{}' style='background:white'>\n",
            cell * styles.len() as f64,
            cell * looks.len() as f64
        );
        for (j, look) in looks.iter().enumerate() {
            for (i, style) in styles.iter().enumerate() {
                let shapes = p.side(
                    [0.0, 1.0],
                    &SideOptions {
                        style: *style,
                        look: *look,
                        opacity: 0.6,
                        m_per_mm: span / 70.0,
                    },
                );
                let ox = cell * (i as f64 + 0.5);
                let oy = cell * (j as f64 + 0.95);
                side.push_str(&svg_paths(&shapes, ox, oy, k, &colors));
            }
        }
        side.push_str("</svg>\n");
        std::fs::write(format!("{dir}/{name}-alzado.svg"), side).unwrap();
        for lod in Lod::ALL {
            let m = p.mesh(&colors, lod);
            let img = render(&m, 360);
            std::fs::write(format!("{dir}/{name}-3d-{lod:?}.ppm"), img).unwrap();
        }
    }
}

/// SVG paths of some shapes, `k` pixels a metre from `(ox, oy)`, `y` up.
fn svg_paths(shapes: &[Shape], ox: f64, oy: f64, k: f64, colors: &Colors) -> String {
    let mut out = String::new();
    for sh in shapes {
        let (pts, closed, stroke, fill, op) = match &sh {
            Shape::Line {
                points,
                closed,
                stroke,
            } => (points, *closed, Some(*stroke), None, 1.0),
            Shape::Fill {
                points,
                tone,
                opacity,
            } => (points, true, None, Some(*tone), *opacity),
        };
        let mut d = String::new();
        for (n, q) in pts.iter().enumerate() {
            d.push_str(&format!(
                "{}{:.2},{:.2} ",
                if n == 0 { "M" } else { "L" },
                ox + q[0] * k,
                oy - q[1] * k
            ));
        }
        if closed {
            d.push('Z');
        }
        let fill = match fill {
            Some(Tone::Leaves) => format!(
                "rgb({},{},{})",
                colors.leaves[0], colors.leaves[1], colors.leaves[2]
            ),
            Some(Tone::Bark) => format!(
                "rgb({},{},{})",
                colors.bark[0], colors.bark[1], colors.bark[2]
            ),
            Some(Tone::Flowers) => format!(
                "rgb({},{},{})",
                colors.flowers[0], colors.flowers[1], colors.flowers[2]
            ),
            Some(Tone::Fruit) => format!(
                "rgb({},{},{})",
                colors.fruit[0], colors.fruit[1], colors.fruit[2]
            ),
            Some(Tone::Paper) => "white".into(),
            Some(Tone::Ink) => "#2b2b2b".into(),
            None => "none".into(),
        };
        let w = match stroke {
            Some(Stroke::Outline) => 0.9,
            Some(Stroke::Thin) => 0.45,
            None => 0.0,
        };
        let stroke = if w > 0.0 { "black" } else { "none" };
        out.push_str(&format!(
            "<path d='{d}' fill='{fill}' fill-opacity='{op}' stroke='{stroke}' stroke-width='{w}'/>\n"
        ));
    }
    out
}

/// A small z-buffer drawing of a mesh, seen from the south a little from
/// above, lit from the south-west.
fn render(m: &FloraMesh, size: usize) -> Vec<u8> {
    let (lo, hi) = m.bounds();
    let span = (hi[0] - lo[0])
        .max(hi[1] - lo[1])
        .max(hi[2] - lo[2])
        .max(0.01) as f64
        * 1.15;
    let cx = (lo[0] + hi[0]) as f64 / 2.0;
    let cy = (lo[1] + hi[1]) as f64 / 2.0;
    let cz = (lo[2] + hi[2]) as f64 / 2.0;
    let (el, az): (f64, f64) = (0.3, -0.5);
    let fwd = [az.sin() * el.cos(), az.cos() * el.cos(), -el.sin()];
    let fwd = [-fwd[0], -fwd[1], fwd[2]];
    let right = grow::norm(grow::cross(fwd, [0.0, 0.0, 1.0]));
    let up = grow::cross(right, fwd);
    let light = grow::norm([-0.5, -0.6, 0.8]);
    let mut z = vec![f64::INFINITY; size * size];
    let mut px = vec![255u8; size * size * 3];
    let project = |p: [f32; 3]| {
        let q = [p[0] as f64 - cx, p[1] as f64 - cy, p[2] as f64 - cz];
        let x = grow::dot(q, right) / span + 0.5;
        let y = 0.5 - grow::dot(q, up) / span;
        let d = grow::dot(q, fwd);
        (x * size as f64, y * size as f64, d)
    };
    for t in m.bark.iter().chain(&m.leaves).chain(&m.flowers) {
        let p: Vec<_> = t.iter().map(|&i| m.positions[i as usize]).collect();
        let a = [p[0][0] as f64, p[0][1] as f64, p[0][2] as f64];
        let b = [p[1][0] as f64, p[1][1] as f64, p[1][2] as f64];
        let c = [p[2][0] as f64, p[2][1] as f64, p[2][2] as f64];
        let n = grow::norm(grow::cross(grow::sub(b, a), grow::sub(c, a)));
        let lit = 0.45 + 0.55 * grow::dot(n, light).abs();
        let col: [f64; 3] = [0, 1, 2].map(|k| {
            t.iter()
                .map(|&i| m.colors[i as usize][k] as f64)
                .sum::<f64>()
                / 3.0
                * lit
        });
        let s: Vec<_> = p.iter().map(|q| project(*q)).collect();
        let minx = s
            .iter()
            .map(|q| q.0)
            .fold(f64::INFINITY, f64::min)
            .floor()
            .max(0.0) as usize;
        let maxx = (s
            .iter()
            .map(|q| q.0)
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil() as usize)
            .min(size - 1);
        let miny = s
            .iter()
            .map(|q| q.1)
            .fold(f64::INFINITY, f64::min)
            .floor()
            .max(0.0) as usize;
        let maxy = (s
            .iter()
            .map(|q| q.1)
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil() as usize)
            .min(size - 1);
        let area = (s[1].0 - s[0].0) * (s[2].1 - s[0].1) - (s[2].0 - s[0].0) * (s[1].1 - s[0].1);
        if area.abs() < 1e-9 {
            continue;
        }
        for y in miny..=maxy {
            for x in minx..=maxx {
                let (fx, fy) = (x as f64 + 0.5, y as f64 + 0.5);
                let w0 = ((s[1].0 - fx) * (s[2].1 - fy) - (s[2].0 - fx) * (s[1].1 - fy)) / area;
                let w1 = ((s[2].0 - fx) * (s[0].1 - fy) - (s[0].0 - fx) * (s[2].1 - fy)) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let d = w0 * s[0].2 + w1 * s[1].2 + w2 * s[2].2;
                let k = y * size + x;
                if d < z[k] {
                    z[k] = d;
                    for c in 0..3 {
                        px[k * 3 + c] = col[c].clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }
    }
    let mut out = format!("P6\n{size} {size}\n255\n").into_bytes();
    out.extend(px);
    out
}

/// The styles of line drawings of trees (25/10/2026): the silhouette is
/// solid in the lines' colour with its branches in white, from above and
/// from the side; the crown leaves its middle open; the twigs carry
/// leaflets.
#[test]
fn graphic_styles() {
    let p = grow(&olive(), 5, 0.5);
    let plan = |style| {
        p.plan(&PlanOptions {
            style,
            look: Look::Lines,
            opacity: 0.6,
            crown_mm: 80.0,
        })
    };
    let side = |style| {
        p.side(
            [0.0, 1.0],
            &crate::SideOptions {
                style,
                look: Look::Lines,
                opacity: 0.6,
                m_per_mm: 0.05,
            },
        )
    };
    for shapes in [plan(PlanStyle::Silhouette), side(PlanStyle::Silhouette)] {
        let tones: Vec<Tone> = shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Fill { tone, .. } => Some(*tone),
                _ => None,
            })
            .collect();
        let ink = tones.iter().position(|t| *t == Tone::Ink).expect("solid");
        // The branches come after the solid crown, white over it.
        assert!(tones[ink..].iter().filter(|t| **t == Tone::Paper).count() > 3);
    }
    // The crown's middle is open: its marks keep away from the centre.
    let c = p.center;
    let r = p.radius;
    let near = plan(PlanStyle::Crown)
        .iter()
        .filter_map(|s| match s {
            // (Not the trunk's ring, of 16 points.)
            Shape::Line { points, .. } if points.len() > 16 => Some(points.clone()),
            _ => None,
        })
        .flatten()
        .filter(|q| (q[0] - c[0]).hypot(q[1] - c[1]) < r * 0.25)
        .count();
    assert_eq!(near, 0);
    // Leaflets: many more strokes than the branches alone.
    let count = |style| plan(style).len();
    assert!(count(PlanStyle::Sprigs) > count(PlanStyle::Branches) * 3);
    for s in [
        PlanStyle::Crown,
        PlanStyle::Silhouette,
        PlanStyle::Sprigs,
        PlanStyle::BranchesEdge,
    ] {
        assert_eq!(PlanStyle::from_name(s.name()), Some(s));
    }
}
