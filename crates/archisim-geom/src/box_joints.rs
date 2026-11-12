//! What closes the joints between boxes and holds them together (M13,
//! `docs/MODULAR.md`): the mineral wool in the gap, the waterproofing band
//! over it on the roof, the membrane over it on the façade, the cover
//! strips on the floor and the ceiling where the joint is open; the plates
//! bolted across the joint at the floor and the roof frames, the lifting
//! lugs, and the base plates with their anchor bolts on the supports.
//! Like the chassis, they are worked out from the boxes, not kept.

use archisim_core::{
    BoxId, Model, Point2, SideState, BOX_PARAPET, COVER_STRIP, FACADE_BAND, JOINT_WOOL, ROOF_BAND,
};

use crate::boxes::lift_points;
use crate::solid::{Solid, SolidOwner};

/// What a piece is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PieceKind {
    /// Mineral wool pressed into the gap between two chassis.
    JointWool,
    /// The waterproofing band over the joint on the roof, welded to the
    /// two boxes' sheets on site.
    RoofBand,
    /// The breather membrane over the joint on the façade, behind the
    /// cladding, with its airtight tape.
    FacadeBand,
    /// The strip over the joint on the floor where it is open.
    FloorStrip,
    /// The strip over the joint on the ceiling where it is open.
    CeilingStrip,
    /// A steel plate bolted across the joint to the two boxes' frames.
    JointPlate,
    /// The lug the box is lifted from.
    LiftLug,
    /// The steel plate on a support, under the boxes.
    BasePlate,
    /// An anchor bolt of a base plate into the pier.
    AnchorBolt,
}

impl PieceKind {
    pub const ALL: [PieceKind; 9] = [
        PieceKind::JointWool,
        PieceKind::RoofBand,
        PieceKind::FacadeBand,
        PieceKind::FloorStrip,
        PieceKind::CeilingStrip,
        PieceKind::JointPlate,
        PieceKind::LiftLug,
        PieceKind::BasePlate,
        PieceKind::AnchorBolt,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PieceKind::JointWool => "Lana mineral de la junta",
            PieceKind::RoofBand => "Banda impermeable de cubierta",
            PieceKind::FacadeBand => "Membrana y cinta de fachada",
            PieceKind::FloorStrip => "Tapajuntas de suelo",
            PieceKind::CeilingStrip => "Tapajuntas de techo",
            PieceKind::JointPlate => "Placa de unión entre cajas",
            PieceKind::LiftLug => "Orejeta de izado",
            PieceKind::BasePlate => "Placa de apoyo",
            PieceKind::AnchorBolt => "Perno de anclaje M16",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            PieceKind::JointWool => "joint-wool",
            PieceKind::RoofBand => "roof-band",
            PieceKind::FacadeBand => "facade-band",
            PieceKind::FloorStrip => "floor-strip",
            PieceKind::CeilingStrip => "ceiling-strip",
            PieceKind::JointPlate => "joint-plate",
            PieceKind::LiftLug => "lift-lug",
            PieceKind::BasePlate => "base-plate",
            PieceKind::AnchorBolt => "anchor-bolt",
        }
    }

    /// Steel pieces (the rest are bands and strips).
    pub fn is_steel(self) -> bool {
        matches!(
            self,
            PieceKind::JointPlate
                | PieceKind::LiftLug
                | PieceKind::BasePlate
                | PieceKind::AnchorBolt
        )
    }

    /// The material the piece is drawn with: its name in the project
    /// ([`archisim_core::JOINT_MATERIALS`]); steel pieces take the frames' steel.
    pub fn material_name(self) -> Option<&'static str> {
        match self {
            PieceKind::JointWool => Some(JOINT_WOOL),
            PieceKind::RoofBand => Some(ROOF_BAND),
            PieceKind::FacadeBand => Some(FACADE_BAND),
            PieceKind::FloorStrip | PieceKind::CeilingStrip => Some(COVER_STRIP),
            _ => None,
        }
    }
}

/// Sizes of the pieces, m.
pub const ROOF_BAND_WIDTH: f64 = 0.30;
pub const FACADE_BAND_WIDTH: f64 = 0.20;
pub const STRIP_WIDTH: f64 = 0.08;
pub const PLATE_REACH: f64 = 0.09;
pub const PLATE_ALONG: f64 = 0.10;
pub const PLATE_THICKNESS: f64 = 0.010;
/// Bolts of a joint plate (two into each box).
pub const PLATE_BOLTS: usize = 4;
/// Self-drilling screws per stud of the boxes' walls (two at each end).
pub const SCREWS_PER_STUD: usize = 4;

/// A piece, as a block in its box's axes (`s` along, `t` across, `z` over
/// the level), as the chassis' tubes.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxPiece {
    pub kind: PieceKind,
    pub lo: [f64; 3],
    pub hi: [f64; 3],
}

impl BoxPiece {
    pub fn size(&self) -> [f64; 3] {
        [
            self.hi[0] - self.lo[0],
            self.hi[1] - self.lo[1],
            self.hi[2] - self.lo[2],
        ]
    }

    /// Its longest side, m.
    pub fn length(&self) -> f64 {
        let s = self.size();
        s[0].max(s[1]).max(s[2])
    }

    /// Steel weight, kg (none for the bands).
    pub fn weight(&self) -> f64 {
        if !self.kind.is_steel() {
            return 0.0;
        }
        let s = self.size();
        s[0] * s[1] * s[2] * archisim_core::STEEL_DENSITY
    }
}

fn block(kind: PieceKind, a: [f64; 3], b: [f64; 3]) -> BoxPiece {
    BoxPiece {
        kind,
        lo: [a[0].min(b[0]), a[1].min(b[1]), a[2].min(b[2])],
        hi: [a[0].max(b[0]), a[1].max(b[1]), a[2].max(b[2])],
    }
}

/// Where the roof's top is over the level: under the crown of the parapet,
/// or at the top of the box when another stands on it.
fn roof_top(model: &Model, id: BoxId) -> f64 {
    let b = model.modular_box(id).unwrap();
    let covered = !model.boxes_above(id).is_empty();
    b.bottom + b.height - if covered { 0.0 } else { BOX_PARAPET }
}

/// The pieces of a box: those of the joints it is the first box of (each
/// joint once), its lugs, and the base plates and anchors of the supports
/// it is the first box on.
pub fn box_pieces(model: &Model, id: BoxId) -> Vec<BoxPiece> {
    let Some(b) = model.modular_box(id) else {
        return Vec::new();
    };
    let (o, u, v) = (b.origin, b.along(), b.across());
    let local = |p: Point2| {
        let d = p - o;
        (d.x * u.x + d.y * u.y, d.x * v.x + d.y * v.y)
    };
    let (l, w) = (b.length, b.width);
    let c = &b.chassis;
    let mut out = Vec::new();
    let level_boxes: Vec<_> = model.boxes_on_level(b.level).collect();
    let in_any_box = |p: Point2| {
        level_boxes
            .iter()
            .any(|x| archisim_core::point_in_polygon(p, &x.outline()))
    };
    for j in model
        .box_joints_on(b.level)
        .into_iter()
        .filter(|j| j.a == id)
    {
        let side = j.side_a;
        let (s0, t0) = local(j.start);
        let (s1, t1) = local(j.end);
        // Along the side, and the gap across it, in the box's axes.
        let (along_lo, along_hi) = if side % 2 == 0 {
            (s0.min(s1), s0.max(s1))
        } else {
            (t0.min(t1), t0.max(t1))
        };
        let gap = j.gap.max(0.005);
        let (g0, g1) = match side {
            0 => (-gap, 0.0),
            1 => (l, l + gap),
            2 => (w, w + gap),
            _ => (-gap, 0.0),
        };
        let mid = (g0 + g1) / 2.0;
        // A block over the joint: `along` along it, `across` across it
        // around the gap's middle (or the gap itself), z from–to.
        let put = |kind: PieceKind, a0: f64, a1: f64, across: Option<f64>, z0: f64, z1: f64| {
            let (c0, c1) = match across {
                Some(width) => (mid - width / 2.0, mid + width / 2.0),
                None => (g0, g1),
            };
            if side % 2 == 0 {
                block(kind, [a0, c0, z0], [a1, c1, z1])
            } else {
                block(kind, [c0, a0, z0], [c1, a1, z1])
            }
        };
        let top = roof_top(model, j.a).min(roof_top(model, j.b));
        out.push(put(
            PieceKind::JointWool,
            along_lo,
            along_hi,
            None,
            b.bottom,
            top,
        ));
        if model.boxes_above(j.a).is_empty() && model.boxes_above(j.b).is_empty() {
            out.push(put(
                PieceKind::RoofBand,
                along_lo,
                along_hi,
                Some(ROOF_BAND_WIDTH),
                top,
                top + 0.004,
            ));
        }
        // The ends of the joint on the façade.
        let dir = (j.end - j.start).normalized().unwrap_or(u);
        for (end, sign, at) in [(j.start, -1.0, along_lo), (j.end, 1.0, along_hi)] {
            let beyond = end + dir * (0.10 * sign);
            if in_any_box(beyond) {
                continue;
            }
            // Its sign in the box's axes along the side.
            let along_dir = if side % 2 == 0 {
                dir.x * u.x + dir.y * u.y
            } else {
                dir.x * v.x + dir.y * v.y
            };
            let out_sign = sign * along_dir.signum();
            let (a0, a1) = if out_sign > 0.0 {
                (at, at + 0.002)
            } else {
                (at - 0.002, at)
            };
            out.push(put(
                PieceKind::FacadeBand,
                a0,
                a1,
                Some(FACADE_BAND_WIDTH),
                b.bottom,
                top,
            ));
        }
        // Open on both sides: strips on the floor and the ceiling.
        let open_a = b.sides[side] == SideState::Open;
        let open_b = model
            .modular_box(j.b)
            .is_some_and(|x| x.sides[j.side_b] == SideState::Open);
        if open_a && open_b {
            out.push(put(
                PieceKind::FloorStrip,
                along_lo,
                along_hi,
                Some(STRIP_WIDTH),
                0.0,
                0.008,
            ));
            out.push(put(
                PieceKind::CeilingStrip,
                along_lo,
                along_hi,
                Some(STRIP_WIDTH),
                c.roof_at - 0.008,
                c.roof_at,
            ));
        }
        // Plates across the joint at its ends (and in its middle when it
        // is long), on the floor frame and on the roof frame.
        let len = along_hi - along_lo;
        let mut stations = vec![along_lo + 0.15, along_hi - 0.15];
        if len > 4.0 {
            stations.push((along_lo + along_hi) / 2.0);
        }
        let floor_z = b.bottom + c.edge.depth;
        let roof_z = c.roof_at + c.roof_edge.depth;
        for s in stations {
            for z in [floor_z, roof_z] {
                out.push(put(
                    PieceKind::JointPlate,
                    s - PLATE_ALONG / 2.0,
                    s + PLATE_ALONG / 2.0,
                    Some(gap + 2.0 * PLATE_REACH),
                    z,
                    z + PLATE_THICKNESS,
                ));
            }
        }
    }
    // The lugs, on the floor frame at the lifting points.
    for p in lift_points(b) {
        let z = b.bottom + c.edge.depth;
        out.push(block(
            PieceKind::LiftLug,
            [p[0] - 0.05, p[1] - 0.0075, z],
            [p[0] + 0.05, p[1] + 0.0075, z + 0.12],
        ));
    }
    // The base plates and anchors of the supports this box is the first on.
    let lv = model.level(b.level).map_or(0.0, |x| x.elevation);
    for s in model.box_supports() {
        if s.boxes.first().map(|(x, _)| *x) != Some(id) {
            continue;
        }
        let (ps, pt) = local(s.at);
        let n = s.boxes.len();
        let (hs, ht) = match n {
            1 => (0.125, 0.125),
            2 => (0.20, 0.125),
            _ => (0.20, 0.20),
        };
        let z = s.z - lv;
        out.push(block(
            PieceKind::BasePlate,
            [ps - hs, pt - ht, z - 0.015],
            [ps + hs, pt + ht, z],
        ));
        // Two bolts per box it carries, along the plate.
        let bolts = 2 * n;
        for k in 0..bolts {
            let f = (k as f64 + 0.5) / bolts as f64;
            let bs = ps - hs + 0.04 + f * (2.0 * hs - 0.08);
            let bt = if k % 2 == 0 {
                pt - ht + 0.04
            } else {
                pt + ht - 0.04
            };
            out.push(block(
                PieceKind::AnchorBolt,
                [bs - 0.008, bt - 0.008, z - 0.315],
                [bs + 0.008, bt + 0.008, z + 0.03],
            ));
        }
    }
    out
}

/// What a box's joints and connections add up to, for the measurements and
/// the workshop sheets.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct JointTally {
    /// m² of wool, m of roof band, of façade membrane and of strips.
    pub wool: f64,
    pub roof_band: f64,
    pub facade_band: f64,
    pub strips: f64,
    /// Units.
    pub plates: usize,
    pub plate_bolts: usize,
    pub lugs: usize,
    pub base_plates: usize,
    pub anchors: usize,
    /// Self-drilling screws of the walls' studs, and the welds of the
    /// chassis, m.
    pub screws: usize,
    pub welds: f64,
    /// Steel of the plates, lugs and bolts, kg.
    pub steel: f64,
}

/// The tally of a box's pieces, with its walls' screws and its chassis'
/// welds.
pub fn joint_tally(model: &Model, id: BoxId, studs: usize) -> JointTally {
    let mut t = JointTally::default();
    for p in box_pieces(model, id) {
        let s = p.size();
        match p.kind {
            // Its face in the joint: along × height.
            PieceKind::JointWool => t.wool += s[0].max(s[1]) * s[2],
            PieceKind::RoofBand => t.roof_band += s[0].max(s[1]),
            PieceKind::FacadeBand => t.facade_band += s[2],
            PieceKind::FloorStrip | PieceKind::CeilingStrip => t.strips += s[0].max(s[1]),
            PieceKind::JointPlate => {
                t.plates += 1;
                t.plate_bolts += PLATE_BOLTS;
            }
            PieceKind::LiftLug => t.lugs += 1,
            PieceKind::BasePlate => t.base_plates += 1,
            PieceKind::AnchorBolt => t.anchors += 1,
        }
        t.steel += p.weight();
    }
    t.screws = studs * SCREWS_PER_STUD;
    if let Some(b) = model.modular_box(id) {
        // Each tube welded all round at its two ends.
        t.welds = crate::boxes::chassis_members(b)
            .iter()
            .map(|m| 2.0 * 2.0 * (m.tube.depth + m.tube.width))
            .sum();
    }
    t
}

/// The solids of a box's pieces; those whose material is not in the
/// project are left out.
pub fn piece_solids(model: &Model, id: BoxId) -> Vec<Solid> {
    let Some(b) = model.modular_box(id) else {
        return Vec::new();
    };
    let steel = model.frame_member_material(archisim_core::FrameMaterial::Steel);
    let named = |name: &str| model.materials().find(|m| m.name == name).map(|m| m.id);
    let lv = model.level(b.level).map_or(0.0, |l| l.elevation);
    let (u, v) = (b.along(), b.across());
    box_pieces(model, id)
        .into_iter()
        .filter_map(|p| {
            let material = match p.kind.material_name() {
                Some(n) => named(n)?,
                None => steel?,
            };
            let o = b.origin + u * p.lo[0] + v * p.lo[1];
            let s = p.size();
            let mut triangles = Vec::with_capacity(12);
            crate::framing::cuboid(
                &mut triangles,
                [o.x, o.y, lv + p.lo[2]],
                [u.x * s[0], u.y * s[0], 0.0],
                [v.x * s[1], v.y * s[1], 0.0],
                [0.0, 0.0, s[2]],
            );
            Some(Solid {
                owner: SolidOwner::Box(id),
                material,
                triangles,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{NewBox, BOX_JOINT, BOX_WIDTH};

    /// Two usual boxes side by side, joined along their long sides; the
    /// first open to the second.
    fn two_boxes() -> (Model, BoxId, BoxId) {
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
        (m, a, b)
    }

    fn of(v: &[BoxPiece], k: PieceKind) -> Vec<&BoxPiece> {
        v.iter().filter(|p| p.kind == k).collect()
    }

    #[test]
    fn a_joint_is_filled_banded_and_bolted() {
        let (m, a, b) = two_boxes();
        let joint = &m.box_joints_on(m.modular_box(a).unwrap().level)[0];
        let first = joint.a;
        let other = if first == a { b } else { a };
        let pieces = box_pieces(&m, first);
        let length = m.modular_box(first).unwrap().length;
        // The wool fills the gap the whole length, up to the roof.
        let wool = of(&pieces, PieceKind::JointWool);
        assert_eq!(wool.len(), 1);
        let s = wool[0].size();
        assert!((s[0] - length).abs() < 1e-6, "{s:?}");
        assert!((s[1] - BOX_JOINT).abs() < 1e-6, "{s:?}");
        // A band over it on the roof, and the membrane at both ends.
        assert_eq!(of(&pieces, PieceKind::RoofBand).len(), 1);
        assert_eq!(of(&pieces, PieceKind::FacadeBand).len(), 2);
        // Walled: no strips.
        assert!(of(&pieces, PieceKind::FloorStrip).is_empty());
        // Long joint: plates at the ends and the middle, floor and roof.
        let plates = of(&pieces, PieceKind::JointPlate);
        assert_eq!(plates.len(), 6);
        // A plate reaches over both boxes' frames.
        assert!(plates[0].size()[1] > BOX_JOINT + 0.15);
        // The other box carries only its own lugs.
        let rest = box_pieces(&m, other);
        assert!(of(&rest, PieceKind::JointWool).is_empty());
        assert_eq!(of(&rest, PieceKind::LiftLug).len(), 4);
        // Steel weighs its volume.
        let w = plates[0].weight();
        assert!((w - 0.20_f64.min(plates[0].size()[1]) * 0.10 * 0.010 * 7850.0).abs() < 0.2);
    }

    #[test]
    fn open_joints_get_strips_and_stacked_boxes_no_roof_band() {
        let (mut m, a, b) = two_boxes();
        let (sa, sb) = {
            let j = &m.box_joints_on(m.modular_box(a).unwrap().level)[0];
            ((j.a, j.side_a), (j.b, j.side_b))
        };
        m.edit_box(sa.0, |x| x.sides[sa.1] = SideState::Open)
            .unwrap();
        m.edit_box(sb.0, |x| x.sides[sb.1] = SideState::Open)
            .unwrap();
        let p = box_pieces(&m, sa.0);
        assert_eq!(of(&p, PieceKind::FloorStrip).len(), 1);
        assert_eq!(of(&p, PieceKind::CeilingStrip).len(), 1);
        // A box over the first one: no band on that roof.
        let up = m.add_level("P1", 3.2);
        let o = m.modular_box(a).unwrap().origin;
        m.add_box(NewBox::usual(up, o, 0.0)).unwrap();
        let p = box_pieces(&m, sa.0);
        assert!(of(&p, PieceKind::RoofBand).is_empty());
        let _ = b;
    }

    #[test]
    fn supports_get_plates_and_anchor_bolts() {
        let (m, a, b) = two_boxes();
        let supports = m.box_supports();
        assert!(!supports.is_empty());
        let all: Vec<BoxPiece> = [a, b].iter().flat_map(|x| box_pieces(&m, *x)).collect();
        assert_eq!(of(&all, PieceKind::BasePlate).len(), supports.len());
        let bolts: usize = supports.iter().map(|s| 2 * s.boxes.len()).sum();
        assert_eq!(of(&all, PieceKind::AnchorBolt).len(), bolts);
        // The plate's top is the boxes' base.
        let plate = of(&all, PieceKind::BasePlate)[0];
        assert!((plate.hi[2] - m.modular_box(a).unwrap().bottom).abs() < 1e-6);
    }

    #[test]
    fn the_tally_counts_screws_and_welds() {
        let (m, a, _) = two_boxes();
        let t = joint_tally(&m, a, 10);
        assert_eq!(t.screws, 40);
        assert!(t.welds > 10.0);
        assert!(t.steel > 0.0);
    }
}
