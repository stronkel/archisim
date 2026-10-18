//! Curved walls: faces along arcs, corners with straight and curved walls,
//! walls stopping at a curved face, solids and rooms.

use super::*;
use crate::solid::wall_solids;
use archisim_core::{CutPattern, Layer, NewWall, ReferenceLine, WallFunction, WallTop, WallTypeId};
use std::f64::consts::PI;

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

struct Fixture {
    m: Model,
    lvl: LevelId,
    facade: WallTypeId,
    partition: WallTypeId,
}

fn fixture() -> Fixture {
    let mut m = Model::new("T");
    let lvl = m.add_level("PB", 0.0);
    let a = m.add_material("A", "brick", [0, 0, 0], CutPattern::Solid);
    let b = m.add_material("B", "plaster", [0, 0, 0], CutPattern::Solid);
    let facade = m
        .add_wall_type(
            "Fachada",
            WallFunction::Exterior,
            vec![
                Layer {
                    material: a,
                    thickness: 0.2,
                    function: LayerFunction::Structure,
                },
                Layer {
                    material: b,
                    thickness: 0.1,
                    function: LayerFunction::Finish,
                },
            ],
        )
        .unwrap();
    let partition = m
        .add_wall_type(
            "Tabique",
            WallFunction::Interior,
            vec![Layer {
                material: a,
                thickness: 0.1,
                function: LayerFunction::Structure,
            }],
        )
        .unwrap();
    Fixture {
        m,
        lvl,
        facade,
        partition,
    }
}

fn wall(f: &mut Fixture, t: WallTypeId, a: (f64, f64), b: (f64, f64), bulge: f64) -> WallId {
    f.m.add_wall(NewWall {
        wall_type: t,
        level: f.lvl,
        start: Point2::new(a.0, a.1),
        end: Point2::new(b.0, b.1),
        bulge,
        reference: if t == f.facade {
            ReferenceLine::ExteriorFace
        } else {
            ReferenceLine::Center
        },
        height: 3.0,
        base_offset: 0.0,
        top: WallTop::Unconnected,
    })
    .unwrap()
}

/// A round room of radius 3 (exterior face), drawn clockwise as two half
/// circles so the exterior is outside.
fn round_room(f: &mut Fixture) -> (WallId, WallId) {
    let t = f.facade;
    let a = wall(f, t, (3.0, 0.0), (-3.0, 0.0), -1.0);
    let b = wall(f, t, (-3.0, 0.0), (3.0, 0.0), -1.0);
    (a, b)
}

#[test]
fn half_circles_close_a_round_room() {
    let mut f = fixture();
    let (a, b) = round_room(&mut f);
    let ring = PI * (3.0_f64.powi(2) - 2.7_f64.powi(2)) / 2.0;
    for id in [a, b] {
        let g = wall_geometry(&f.m, id).unwrap();
        assert!(g.arc.is_some());
        // Running on smoothly: cut straight across at the junctions.
        assert!(g.start_cut.is_mitre() && g.end_cut.is_mitre());
        let area = signed_area(&g.outline);
        assert!(close(area, ring, ring * 1e-3), "{area} vs {ring}");
        // Faces on their circles.
        for p in &g.outline {
            let r = p.distance(Point2::new(0.0, 0.0));
            assert!(close(r, 3.0, 1e-9) || close(r, 2.7, 1e-9), "{r}");
        }
        // The layers fill the outline.
        let layers: f64 = g.layers.iter().map(|l| signed_area(&l.polygon)).sum();
        assert!(close(layers, area, 1e-9));
        // The plaster is the inner band.
        let plaster = signed_area(&g.layers[1].polygon);
        let expected = PI * (2.8_f64.powi(2) - 2.7_f64.powi(2)) / 2.0;
        assert!(close(plaster, expected, expected * 1e-3));
        // Exact arcs for the IFC: two arcs and two straight ends.
        let path = g.outline_path().unwrap();
        assert_eq!(path.len(), 4);
        let exact = path_area(&path);
        assert!(close(exact, ring, 1e-9), "{exact} vs {ring}");
        assert_eq!(
            path.iter()
                .filter(|s| matches!(s, PathSeg::Arc(..)))
                .count(),
            2
        );
        // A closed solid of the same volume, without faces between layers.
        let volume: f64 = wall_solids(&f.m, id).iter().map(|s| s.volume()).sum();
        assert!(close(volume, area * 3.0, 1e-6), "{volume}");
    }
    // The magic wand finds the round room.
    let inside = room_outline(&f.m, f.lvl, Point2::new(0.5, 0.2), Contour::InteriorFace).unwrap();
    let a_int = signed_area(&inside);
    assert!(close(a_int, PI * 2.7 * 2.7, 0.01), "{a_int}");
    let axis = room_outline(&f.m, f.lvl, Point2::new(0.5, 0.2), Contour::WallAxis).unwrap();
    let a_axis = signed_area(&axis);
    assert!(close(a_axis, PI * 2.85 * 2.85, 0.02), "{a_axis}");
    let out = room_outline(&f.m, f.lvl, Point2::new(0.5, 0.2), Contour::ExteriorFace).unwrap();
    assert!(close(signed_area(&out), PI * 9.0, 0.02));
}

#[test]
fn a_partition_stops_at_a_curved_face() {
    let mut f = fixture();
    round_room(&mut f);
    // From the centre to inside the bottom half (between 2.7 and 3).
    let t = f.partition;
    let p = wall(&mut f, t, (0.0, 0.0), (0.0, -2.85), 0.0);
    let g = wall_geometry(&f.m, p).unwrap();
    assert!(matches!(g.end_cut, EndCut::Face(Curve::Circle { .. })));
    // Its end lies on the inner face, at radius 2.7.
    let far: Vec<&Point2> = g.outline.iter().filter(|q| q.y < -1.0).collect();
    assert_eq!(far.len(), 2);
    for q in far {
        assert!(close(q.distance(Point2::new(0.0, 0.0)), 2.7, 1e-9));
    }
    // Its 3D body ends there too: the volume of a 0.1 m wall 2.7 m long
    // (less the sliver of the curve).
    let volume: f64 = wall_solids(&f.m, p).iter().map(|s| s.volume()).sum();
    let len = 2.7_f64.powi(2) - 0.05_f64.powi(2);
    assert!(close(volume, 0.1 * len.sqrt() * 3.0, 1e-4), "{volume}");
    // The round wall keeps its whole face.
    let host = wall_geometry(&f.m, f.m.walls().next().unwrap().id).unwrap();
    assert!(host.notches.is_empty());
}

#[test]
fn a_curved_wall_meets_a_straight_one_at_a_corner() {
    let mut f = fixture();
    let t = f.facade;
    // A quarter circle from (0, 0) to (3, 3) bowing out to the bottom
    // right, then a straight wall on to (3, 6) (on tangent) and another
    // one at an angle from (0, 0) back to (-3, 0).
    let s = (0.25 * PI / 2.0).tan();
    let arc = wall(&mut f, t, (0.0, 0.0), (3.0, 3.0), s);
    let on = wall(&mut f, t, (3.0, 3.0), (3.0, 6.0), 0.0);
    let back = wall(&mut f, t, (-3.0, 0.0), (0.0, 0.0), 0.0);
    let ga = wall_geometry(&f.m, arc).unwrap();
    let a = ga.arc.unwrap();
    assert!(a.centre.approx_eq(Point2::new(0.0, 3.0)));
    // On tangent: the faces simply run on.
    let go = wall_geometry(&f.m, on).unwrap();
    assert!(go.start_cut.is_mitre() && ga.end_cut.is_mitre());
    // At an angle: both walls share the two corners of the mitre.
    let gb = wall_geometry(&f.m, back).unwrap();
    assert!(gb.end_cut.is_mitre() && ga.start_cut.is_mitre());
    let shared = |x: &WallGeometry, y: &WallGeometry| {
        x.outline
            .iter()
            .filter(|p| y.outline.iter().any(|q| q.approx_eq(**p)))
            .count()
    };
    assert_eq!(shared(&ga, &gb), 2);
    assert_eq!(shared(&ga, &go), 2);
    // The mitre corners lie on the circles of the curved faces (turning
    // left, the exterior is towards the centre: the interior face is
    // 0.3 m further out).
    for p in ga
        .outline
        .iter()
        .filter(|p| gb.outline.iter().any(|q| q.approx_eq(**p)))
    {
        let r = p.distance(a.centre);
        assert!(close(r, 3.0, 1e-9) || close(r, 3.3, 1e-9), "{r}");
    }
}
