//! Services in a light steel frame of modular boxes (M6,
//! `docs/MODULAR.md`): where a pipe, duct or conduit crosses the joint
//! between two boxes (it is cut there in the factory and joined on site:
//! a coupling, a flange, a junction box, with a register to reach it), and
//! where it goes through the studs of a framed wall (a hole in the web,
//! with its grommet, that must be small enough and far enough from the
//! tracks). Worked out from the model, not kept.

use archisim_core::{BoxId, DeviceKind, Model, Point2, RunId, ServiceSystem};

use crate::framing::{wall_members, Role};

/// How a run is joined at a joint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum JoinKind {
    /// Pipes: a coupling (press or push-fit; for drains, a sleeve with a
    /// seal).
    Coupling,
    /// Ducts: a flanged joint with its seal.
    Flange,
    /// Cables: a junction box with a connector.
    Connector,
}

impl JoinKind {
    pub fn of(system: ServiceSystem) -> Self {
        if system.is_duct() {
            JoinKind::Flange
        } else if system.is_cable() {
            JoinKind::Connector
        } else {
            JoinKind::Coupling
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            JoinKind::Coupling => "Manguito de unión en la junta",
            JoinKind::Flange => "Unión con brida en la junta",
            JoinKind::Connector => "Caja de conexión en la junta",
        }
    }
}

/// Where a run crosses a joint between boxes (or a stacked box's floor).
#[derive(Debug, Clone, PartialEq)]
pub struct JointCrossing {
    /// The run, or the vertical device (riser, downpipe, shaft).
    pub run: Option<RunId>,
    pub device: Option<archisim_core::DeviceId>,
    pub system: ServiceSystem,
    pub at: Point2,
    /// Over the level.
    pub z: f64,
    pub a: BoxId,
    pub b: BoxId,
    pub kind: JoinKind,
    /// «Ø 25 mm».
    pub size: String,
}

fn cross(a0: Point2, a1: Point2, b0: Point2, b1: Point2) -> Option<(f64, Point2)> {
    let r = a1 - a0;
    let s = b1 - b0;
    let d = r.x * s.y - r.y * s.x;
    if d.abs() < 1e-12 {
        return None;
    }
    let q = b0 - a0;
    let t = (q.x * s.y - q.y * s.x) / d;
    let u = (q.x * r.y - q.y * r.x) / d;
    ((-1e-9..=1.0 + 1e-9).contains(&t) && (-1e-9..=1.0 + 1e-9).contains(&u))
        .then(|| (t, a0 + r * t))
}

/// Where the runs cross the joints between boxes, level by level, and
/// where the risers and downpipes go from a box to the box on it.
pub fn joint_crossings(model: &Model) -> Vec<JointCrossing> {
    let mut out = Vec::new();
    let levels: std::collections::BTreeSet<_> = model.boxes().map(|b| b.level).collect();
    for level in levels {
        let joints = model.box_joints_on(level);
        for r in model.runs_on_level(level) {
            let mut s = 0.0;
            for w in r.path.windows(2) {
                let len = (w[1] - w[0]).length();
                for j in &joints {
                    if let Some((t, at)) = cross(w[0], w[1], j.start, j.end) {
                        out.push(JointCrossing {
                            run: Some(r.id),
                            device: None,
                            system: r.system,
                            at,
                            z: r.z_at(s + t * len),
                            a: j.a,
                            b: j.b,
                            kind: JoinKind::of(r.system),
                            size: r.size_text(),
                        });
                    }
                }
                s += len;
            }
        }
    }
    // Vertical devices through the floor of a stacked box.
    for d in model.devices() {
        if !matches!(
            d.kind,
            DeviceKind::Riser | DeviceKind::Downpipe | DeviceKind::AirShaft
        ) {
            continue;
        }
        let Some(lv) = model.level(d.level) else {
            continue;
        };
        let top = lv.elevation + d.elevation + d.height;
        for b in model.boxes().filter(|b| b.level == d.level) {
            if !archisim_core::point_in_polygon(d.position, &b.outline()) {
                continue;
            }
            for up in model.boxes_above(b.id) {
                let Some(u) = model.modular_box(up) else {
                    continue;
                };
                let ul = model.level(u.level).map_or(0.0, |l| l.elevation);
                if ul + u.bottom < top && archisim_core::point_in_polygon(d.position, &u.outline())
                {
                    let system = match d.kind {
                        DeviceKind::Downpipe => ServiceSystem::Drainage,
                        DeviceKind::AirShaft => ServiceSystem::ExhaustAir,
                        _ => ServiceSystem::ColdWater,
                    };
                    out.push(JointCrossing {
                        run: None,
                        device: Some(d.id),
                        system,
                        at: d.position,
                        z: ul + u.bottom - lv.elevation,
                        a: b.id,
                        b: up,
                        kind: JoinKind::of(system),
                        size: format!("Ø {:.0} mm", d.width * 1000.0),
                    });
                }
            }
        }
    }
    out
}

/// A run's share of a stud's web it may take (a punched or drilled hole).
pub const STUD_HOLE_SHARE: f64 = 0.40;
/// How far from the stud's ends (the tracks) a hole must be, m.
pub const STUD_HOLE_END: f64 = 0.25;
/// The grommet or sleeve round the run, m (on the diameter).
pub const GROMMET: f64 = 0.010;

/// A hole in a stud for a run.
#[derive(Debug, Clone, PartialEq)]
pub struct StudHole {
    pub wall: archisim_core::WallId,
    pub run: RunId,
    pub at: Point2,
    /// Over the level.
    pub z: f64,
    /// Along the wall, from its start.
    pub along: f64,
    /// The hole with its grommet, m.
    pub diameter: f64,
    /// The stud's depth (its web), m.
    pub web: f64,
    /// Too big for the web.
    pub too_big: bool,
    /// Too near a track.
    pub too_near_end: bool,
    pub stud: String,
}

impl StudHole {
    pub fn fails(&self) -> bool {
        self.too_big || self.too_near_end
    }
}

/// The holes the runs need in the studs of the framed walls.
pub fn stud_holes(model: &Model) -> Vec<StudHole> {
    let mut out = Vec::new();
    for w in model.walls() {
        if model.type_framing(w.wall_type.0).is_none() {
            continue;
        }
        let runs: Vec<_> = model
            .runs_on_level(w.level)
            .filter(|r| !r.system.is_urban())
            .collect();
        if runs.is_empty() {
            continue;
        }
        let base = model.level(w.level).map_or(0.0, |l| l.elevation);
        for m in wall_members(model, w.id) {
            if !matches!(
                m.role,
                Role::Stud | Role::KingStud | Role::JackStud | Role::Cripple
            ) {
                continue;
            }
            // Its web in plan, across the wall.
            let c = Point2::new(m.start[0], m.start[1]);
            let across = Point2::new(m.across[0], m.across[1]);
            let half = m.section.depth / 2.0;
            let (w0, w1) = (c - across * half, c + across * half);
            let (z0, z1) = (
                m.start[2].min(m.end[2]) - base,
                m.start[2].max(m.end[2]) - base,
            );
            for r in &runs {
                let mut s = 0.0;
                for seg in r.path.windows(2) {
                    let len = (seg[1] - seg[0]).length();
                    if let Some((t, at)) = cross(seg[0], seg[1], w0, w1) {
                        let z = r.z_at(s + t * len);
                        let size = if r.is_duct() {
                            r.width.max(r.height)
                        } else {
                            r.width
                        };
                        if z > z0 - 1e-6 && z < z1 + 1e-6 {
                            let diameter = size + GROMMET;
                            out.push(StudHole {
                                wall: w.id,
                                run: r.id,
                                at,
                                z,
                                along: m.along[0],
                                diameter,
                                web: m.section.depth,
                                too_big: diameter > STUD_HOLE_SHARE * m.section.depth + 1e-9,
                                too_near_end: z - z0 < STUD_HOLE_END || z1 - z < STUD_HOLE_END,
                                stud: m.section.name.clone(),
                            });
                        }
                    }
                    s += len;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{NewBox, NewRun, BOX_JOINT, BOX_WIDTH};

    #[test]
    fn a_pipe_across_two_boxes_is_joined_at_the_joint() {
        let mut m = Model::new("T");
        let lv = m.add_level("PB", 0.0);
        let a = m
            .add_box(NewBox::usual(lv, Point2::new(0.0, 0.0), 0.0))
            .unwrap();
        let b = m
            .add_box(NewBox::usual(
                lv,
                Point2::new(0.0, BOX_WIDTH + BOX_JOINT),
                0.0,
            ))
            .unwrap();
        m.add_run(NewRun {
            level: lv,
            system: ServiceSystem::ColdWater,
            path: vec![Point2::new(2.0, 1.0), Point2::new(2.0, BOX_WIDTH + 1.0)],
            elevation: 0.4,
            width: 0.025,
            height: 0.025,
            slope: 0.0,
        })
        .unwrap();
        let c = joint_crossings(&m);
        assert_eq!(c.len(), 1, "{c:?}");
        assert_eq!(c[0].kind, JoinKind::Coupling);
        assert!((c[0].at.x - 2.0).abs() < 1e-9);
        assert!([a, b].contains(&c[0].a) && [a, b].contains(&c[0].b));
        assert!((c[0].z - 0.4).abs() < 1e-9);
    }

    #[test]
    fn a_pipe_through_a_stud_needs_a_small_enough_hole() {
        use archisim_core::{
            CutPattern, FrameMaterial, Framing, Layer, LayerFunction, NewWall, ReferenceLine,
            WallFunction, WallTop,
        };
        let mut m = Model::new("T");
        let lv = m.add_level("PB", 0.0);
        let wool = m.add_material(
            "Lana mineral",
            "insulation",
            [235, 225, 150],
            CutPattern::Solid,
        );
        let t = m
            .add_wall_type(
                "Tabique",
                WallFunction::Interior,
                vec![Layer {
                    material: wool,
                    thickness: 0.10,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        m.set_type_framing(t.0, Some(Framing::for_depth(FrameMaterial::Steel, 0.10)))
            .unwrap();
        let w = m
            .add_wall(NewWall {
                wall_type: t,
                level: lv,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(3.0, 0.0),
                bulge: 0.0,
                reference: ReferenceLine::Center,
                height: 2.6,
                base_offset: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        assert!(wall_members(&m, w).iter().any(|x| x.role == Role::Stud));
        // Along the wall, inside it: through every stud.
        let run = |m: &mut Model, d: f64, z: f64| {
            m.add_run(NewRun {
                level: lv,
                system: ServiceSystem::ColdWater,
                path: vec![Point2::new(0.3, 0.0), Point2::new(2.7, 0.0)],
                elevation: z,
                width: d,
                height: d,
                slope: 0.0,
            })
            .unwrap()
        };
        let small = run(&mut m, 0.020, 0.6);
        let big = run(&mut m, 0.050, 1.0);
        let low = run(&mut m, 0.020, 0.1);
        let holes = stud_holes(&m);
        let of = |r: RunId| holes.iter().find(|h| h.run == r).unwrap();
        assert!(holes.iter().filter(|h| h.run == small).count() >= 3);
        assert!(!of(small).fails(), "{:?}", of(small));
        assert!(of(big).too_big);
        assert!(of(low).too_near_end);
    }
}
