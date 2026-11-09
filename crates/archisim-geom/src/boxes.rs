//! The chassis of the modular boxes (M3, `docs/MODULAR.md` §5 bis): the
//! welded tubes laid out from a box and its [`Chassis`], which keeps only
//! their sizes. A floor frame round the box at its underside, its joists
//! across (two spans over a middle beam along it), a roof frame of the
//! same tube with its underside at the ceiling's plane and its joists, and
//! posts at the corners from the underside to the top, with another in the
//! middle of each long side that is open or that the box hangs from. Each
//! piece is a straight bar of rectangular hollow section, given as a box
//! in the box's own axes (`s` along it, `t` across it, `z` over its
//! level); in 3D a closed cuboid, in IFC the tube's profile swept along
//! it.

use archisim_core::{BoxId, BoxTube, LiftPoints, Model, ModularBox, Point2, SideState};

use crate::solid::{Solid, SolidOwner, P3};

/// What a piece of the chassis is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChassisRole {
    /// The floor frame's long sides and ends.
    FloorEdge,
    /// The roof frame's.
    RoofEdge,
    /// A corner post.
    Post,
    /// A post in the middle of a long side.
    MidPost,
    /// The beam along the middle of the floor.
    MiddleBeam,
    FloorJoist,
    RoofJoist,
}

impl ChassisRole {
    pub const ALL: [ChassisRole; 7] = [
        ChassisRole::FloorEdge,
        ChassisRole::RoofEdge,
        ChassisRole::Post,
        ChassisRole::MidPost,
        ChassisRole::MiddleBeam,
        ChassisRole::FloorJoist,
        ChassisRole::RoofJoist,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ChassisRole::FloorEdge => "Marco de suelo",
            ChassisRole::RoofEdge => "Marco de techo",
            ChassisRole::Post => "Pilar de esquina",
            ChassisRole::MidPost => "Pilar intermedio",
            ChassisRole::MiddleBeam => "Viga central",
            ChassisRole::FloorJoist => "Vigueta de suelo",
            ChassisRole::RoofJoist => "Vigueta de techo",
        }
    }

    /// The code kept in IFC (`ArchiSim_BoxPart.Role`).
    pub fn code(self) -> &'static str {
        match self {
            ChassisRole::FloorEdge => "floor_edge",
            ChassisRole::RoofEdge => "roof_edge",
            ChassisRole::Post => "post",
            ChassisRole::MidPost => "mid_post",
            ChassisRole::MiddleBeam => "middle_beam",
            ChassisRole::FloorJoist => "floor_joist",
            ChassisRole::RoofJoist => "roof_joist",
        }
    }

    pub fn is_post(self) -> bool {
        matches!(self, ChassisRole::Post | ChassisRole::MidPost)
    }
}

/// One tube of a chassis: the box `lo`–`hi` in the box's axes, running
/// along `axis` (0 along the box, 1 across it, 2 up).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChassisMember {
    pub role: ChassisRole,
    pub tube: BoxTube,
    pub lo: [f64; 3],
    pub hi: [f64; 3],
    pub axis: usize,
}

impl ChassisMember {
    pub fn length(&self) -> f64 {
        self.hi[self.axis] - self.lo[self.axis]
    }

    pub fn weight(&self) -> f64 {
        self.tube.kg_per_m() * self.length()
    }

    /// Its middle, in the box's axes.
    pub fn centre(&self) -> [f64; 3] {
        [
            (self.lo[0] + self.hi[0]) / 2.0,
            (self.lo[1] + self.hi[1]) / 2.0,
            (self.lo[2] + self.hi[2]) / 2.0,
        ]
    }
}

/// A point of a box's axes in the project (`z` over the level's datum).
pub fn box_point(b: &ModularBox, p: [f64; 3]) -> Point2 {
    b.origin + b.along() * p[0] + b.across() * p[1]
}

/// The pieces of a box's chassis.
pub fn chassis_members(b: &ModularBox) -> Vec<ChassisMember> {
    let c = &b.chassis;
    let (l, w) = (b.length, b.width);
    let (ed, ew) = (c.edge.depth, c.edge.width);
    // Posts: their depth along the box, their width across it.
    let (pd, pw) = (c.post.depth, c.post.width);
    let (z0, top) = (b.bottom, b.bottom + b.height);
    let floor_top = z0 + ed;
    let roof = c.roof_at;
    let mut out = Vec::new();
    let mut push = |role, tube, lo: [f64; 3], hi: [f64; 3], axis| {
        if hi[axis] - lo[axis] > 1e-3 {
            out.push(ChassisMember {
                role,
                tube,
                lo,
                hi,
                axis,
            });
        }
    };
    // Corner posts, the whole height.
    for (s, t) in [(0.0, 0.0), (l - pd, 0.0), (l - pd, w - pw), (0.0, w - pw)] {
        push(
            ChassisRole::Post,
            c.post,
            [s, t, z0],
            [s + pd, t + pw, top],
            2,
        );
    }
    // In the middle of a long side that is open, or that it hangs from.
    for (side, t) in [(0usize, 0.0), (2usize, w - pw)] {
        if b.sides[side] == SideState::Open || c.lift == LiftPoints::Six {
            let s = (l - pd) / 2.0;
            push(
                ChassisRole::MidPost,
                c.post,
                [s, t, z0],
                [s + pd, t + pw, top],
                2,
            );
        }
    }
    // The two frames: long sides between the posts, ends between them.
    for (role, za) in [(ChassisRole::FloorEdge, z0), (ChassisRole::RoofEdge, roof)] {
        for t in [0.0, w - ew] {
            push(role, c.edge, [pd, t, za], [l - pd, t + ew, za + ed], 0);
        }
        for s in [0.0, l - ew] {
            push(role, c.edge, [s, pw, za], [s + ew, w - pw, za + ed], 1);
        }
    }
    // The middle beam along the floor, its top flush with the frame's.
    let j = c.joist;
    let mid = (w - j.width) / 2.0;
    if c.middle_beam {
        push(
            ChassisRole::MiddleBeam,
            j,
            [ew, mid, floor_top - j.depth],
            [l - ew, mid + j.width, floor_top],
            0,
        );
    }
    // Joists across, evenly between the ends of the frame.
    let stations = |spacing: f64| -> Vec<f64> {
        let inner = l - 2.0 * ew;
        let n = (inner / spacing - 1e-6).ceil().max(1.0) as usize;
        (1..n).map(|k| ew + inner * k as f64 / n as f64).collect()
    };
    for s in stations(c.spacing) {
        let (a, z) = (s - j.width / 2.0, s + j.width / 2.0);
        let (lo, hi) = (floor_top - j.depth, floor_top);
        if c.middle_beam {
            push(ChassisRole::FloorJoist, j, [a, ew, lo], [z, mid, hi], 1);
            push(
                ChassisRole::FloorJoist,
                j,
                [a, mid + j.width, lo],
                [z, w - ew, hi],
                1,
            );
        } else {
            push(ChassisRole::FloorJoist, j, [a, ew, lo], [z, w - ew, hi], 1);
        }
    }
    let r = c.roof_joist;
    for s in stations(c.spacing) {
        let (a, z) = (s - r.width / 2.0, s + r.width / 2.0);
        push(
            ChassisRole::RoofJoist,
            r,
            [a, ew, roof],
            [z, w - ew, roof + r.depth],
            1,
        );
    }
    out
}

/// Where a box hangs from when it is craned, in the box's axes: on the
/// floor frame, at the corners (and in the middle of the long sides).
pub fn lift_points(b: &ModularBox) -> Vec<[f64; 3]> {
    let c = &b.chassis;
    let (l, w) = (b.length, b.width);
    let (s0, s1) = (c.post.depth / 2.0, l - c.post.depth / 2.0);
    let (t0, t1) = (c.edge.width / 2.0, w - c.edge.width / 2.0);
    let z = b.bottom + c.edge.depth / 2.0;
    let mut v = vec![[s0, t0, z], [s1, t0, z], [s1, t1, z], [s0, t1, z]];
    if c.lift == LiftPoints::Six {
        v.insert(1, [l / 2.0, t0, z]);
        v.insert(4, [l / 2.0, t1, z]);
    }
    v
}

/// The steel of the chassis, kg.
pub fn chassis_weight(b: &ModularBox) -> f64 {
    chassis_members(b).iter().map(|m| m.weight()).sum()
}

/// The closed cuboid of a piece, in the project.
pub fn member_triangles(model: &Model, b: &ModularBox, m: &ChassisMember) -> Vec<[P3; 3]> {
    let lv = model.level(b.level).map_or(0.0, |l| l.elevation);
    let (u, v) = (b.along(), b.across());
    let o = box_point(b, m.lo);
    let o = [o.x, o.y, lv + m.lo[2]];
    let ds = m.hi[0] - m.lo[0];
    let dt = m.hi[1] - m.lo[1];
    let dz = m.hi[2] - m.lo[2];
    let mut out = Vec::with_capacity(12);
    crate::framing::cuboid(
        &mut out,
        o,
        [u.x * ds, u.y * ds, 0.0],
        [v.x * dt, v.y * dt, 0.0],
        [0.0, 0.0, dz],
    );
    out
}

/// The solids of a box's chassis, of the project's steel for frames (none
/// if it has no steel).
pub fn box_solids(model: &Model, id: BoxId) -> Vec<Solid> {
    let Some(b) = model.modular_box(id) else {
        return Vec::new();
    };
    let Some(material) = model.frame_member_material(archisim_core::FrameMaterial::Steel) else {
        return Vec::new();
    };
    chassis_members(b)
        .iter()
        .map(|m| Solid {
            owner: SolidOwner::Box(id),
            material,
            triangles: member_triangles(model, b, m),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{NewBox, BOX_HEIGHT, BOX_LENGTH, BOX_WIDTH};

    fn a_box() -> (Model, BoxId) {
        let mut m = Model::new("T");
        let lv = m.add_level("PB", 0.0);
        let id = m
            .add_box(NewBox::usual(lv, Point2::new(0.0, 0.0), 0.0))
            .unwrap();
        (m, id)
    }

    #[test]
    fn the_usual_chassis() {
        let (m, id) = a_box();
        let b = m.modular_box(id).unwrap();
        let v = chassis_members(b);
        let count = |r| v.iter().filter(|x| x.role == r).count();
        assert_eq!(count(ChassisRole::Post), 4);
        assert_eq!(count(ChassisRole::MidPost), 0);
        assert_eq!(count(ChassisRole::FloorEdge), 4);
        assert_eq!(count(ChassisRole::RoofEdge), 4);
        assert_eq!(count(ChassisRole::MiddleBeam), 1);
        // Joists at no more than 60 cm: 10 bays in 6,00 m, 9 joists, in
        // two spans on the floor.
        assert_eq!(count(ChassisRole::RoofJoist), 9);
        assert_eq!(count(ChassisRole::FloorJoist), 18);
        // Everything inside the box.
        for x in &v {
            assert!(x.lo[0] >= -1e-9 && x.hi[0] <= BOX_LENGTH + 1e-9, "{x:?}");
            assert!(x.lo[1] >= -1e-9 && x.hi[1] <= BOX_WIDTH + 1e-9, "{x:?}");
            assert!(x.lo[2] >= b.bottom - 1e-9 && x.hi[2] <= b.bottom + BOX_HEIGHT + 1e-9);
        }
        // Posts the whole height.
        let post = v.iter().find(|x| x.role == ChassisRole::Post).unwrap();
        assert!((post.length() - BOX_HEIGHT).abs() < 1e-9);
        // Close to a tonne of steel: 4 posts and 37 m of frame of
        // 13,3 kg/m, and some 100 m of joists of 4,4 kg/m.
        let kg = chassis_weight(b);
        assert!((900.0..1100.0).contains(&kg), "{kg}");
        assert_eq!(lift_points(b).len(), 4);
    }

    #[test]
    fn open_sides_and_six_points_add_middle_posts() {
        let (mut m, id) = a_box();
        m.edit_box(id, |b| {
            b.sides[0] = SideState::Open;
            b.chassis.middle_beam = false;
        })
        .unwrap();
        let b = m.modular_box(id).unwrap();
        let v = chassis_members(b);
        assert_eq!(
            v.iter().filter(|x| x.role == ChassisRole::MidPost).count(),
            1
        );
        assert_eq!(
            v.iter()
                .filter(|x| x.role == ChassisRole::FloorJoist)
                .count(),
            9
        );
        m.edit_box(id, |b| b.chassis.lift = LiftPoints::Six)
            .unwrap();
        let b = m.modular_box(id).unwrap();
        assert_eq!(
            chassis_members(b)
                .iter()
                .filter(|x| x.role == ChassisRole::MidPost)
                .count(),
            2
        );
        assert_eq!(lift_points(b).len(), 6);
    }

    #[test]
    fn walls_of_two_boxes_do_not_join() {
        use archisim_core::{
            CutPattern, Layer, LayerFunction, NewWall, ReferenceLine, WallFunction, WallTop,
        };
        let mut m = Model::new("T");
        let lv = m.add_level("PB", 0.0);
        let mat = m.add_material("Placa", "board", [200; 3], CutPattern::Solid);
        let wt = m
            .add_wall_type(
                "Muro",
                WallFunction::Exterior,
                vec![Layer {
                    material: mat,
                    thickness: 0.15,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        // Two boxes end to end with no joint, and a wall on each that
        // meet at the same point over the line between them.
        let a = m
            .add_box(NewBox::usual(lv, Point2::new(0.0, 0.0), 0.0))
            .unwrap();
        let b = m
            .add_box(NewBox::usual(lv, Point2::new(BOX_LENGTH, 0.0), 0.0))
            .unwrap();
        let wall = |m: &mut Model, s: Point2, e: Point2| {
            m.add_wall(NewWall {
                wall_type: wt,
                level: lv,
                start: s,
                end: e,
                bulge: 0.0,
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap()
        };
        let joint = Point2::new(BOX_LENGTH, 1.0);
        let wa = wall(&mut m, Point2::new(1.0, 1.0), joint);
        let wb = wall(&mut m, joint, Point2::new(BOX_LENGTH + 1.5, 3.0));
        assert_eq!(m.box_of(wa.0), Some(a));
        assert_ne!(m.box_of(wb.0), Some(a));
        let g = crate::wall_geometry(&m, wa).unwrap();
        assert!(
            matches!(g.end_cut, crate::EndCut::Square),
            "{:?}",
            g.end_cut
        );
        // In one box they would make a corner.
        m.remove_box(b).unwrap();
        let g = crate::wall_geometry(&m, wa).unwrap();
        assert!(
            !matches!(g.end_cut, crate::EndCut::Square),
            "{:?}",
            g.end_cut
        );
    }

    #[test]
    fn solids_need_steel() {
        let (mut m, id) = a_box();
        assert!(box_solids(&m, id).is_empty());
        m.add_material(
            archisim_core::FRAME_STEEL,
            "steel",
            [180, 180, 180],
            archisim_core::CutPattern::Solid,
        );
        let s = box_solids(&m, id);
        assert!(!s.is_empty());
        assert!(s.iter().all(|x| x.triangles.len() == 12));
    }
}
