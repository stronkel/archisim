//! What a pitched roof carries (bloque K de `docs/MAQUETA.md`): dormers
//! ("buhardillas") and roof windows standing on its planes, and its
//! covering: tiles, ridge tiles, gutters and downpipes.
//!
//! A dormer belongs to its roof, like ArchiCAD's dormers made with the
//! roof: it is placed by the middle of its front on one plane, looks down
//! that plane's slope, and follows the roof when it moves, turns or
//! changes. Its position is kept relative to the side of the footprint the
//! plane rises from (`side`, `along` from the middle of that side, `up`
//! from it), so moving, turning or mirroring the roof carries it along. A
//! dormer that no longer fits on its plane (the roof changed under it) is
//! kept but not built ([`Roof::dormer_shape`] gives `None`).
//!
//! Local frame of a dormer: `x` along the side (across the dormer), `y` up
//! the slope (in plan), `z` up; the front's outer face is `y = 0`.

use super::*;

/// The kinds of dormer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DormerKind {
    /// A gabled dormer: a front with its gable, a level ridge dying into
    /// the plane, valleys either side.
    Gable,
    /// A shed dormer: one plane, less steep than the roof, from the head
    /// of the front back to the roof.
    Shed,
    /// A flat-roofed dormer.
    Flat,
    /// A roof window in the plane of the roof (IFC `SKYLIGHT`).
    Skylight,
}

impl DormerKind {
    pub const ALL: [DormerKind; 4] = [Self::Gable, Self::Shed, Self::Flat, Self::Skylight];

    /// Its name in the interface.
    pub fn name(self) -> &'static str {
        match self {
            Self::Gable => "Buhardilla a dos aguas",
            Self::Shed => "Buhardilla a un agua",
            Self::Flat => "Buhardilla plana",
            Self::Skylight => "Ventana de cubierta",
        }
    }

    /// Its code in files.
    pub fn code(self) -> &'static str {
        match self {
            Self::Gable => "gable",
            Self::Shed => "shed",
            Self::Flat => "flat",
            Self::Skylight => "skylight",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.code() == c)
    }
}

/// A dormer or roof window on a plane of a roof.
#[derive(Debug, Clone, PartialEq)]
pub struct Dormer {
    pub guid: GlobalId,
    pub kind: DormerKind,
    /// The side of the footprint whose plane it stands on.
    pub side: usize,
    /// Where the middle of its front is along that side, from the
    /// side's midpoint (metres, in the side's direction).
    pub along: f64,
    /// How far up the slope its front is, in plan, from the side.
    pub up: f64,
    /// Outside width.
    pub width: f64,
    /// Height of the front from the roof's top face to the dormer's eave;
    /// for a roof window, its length along the slope.
    pub height: f64,
    /// Pitch of its own roof (rise per run): across a gabled dormer, up a
    /// shed dormer; none for flat ones and roof windows.
    pub pitch: f64,
    /// Width and height of the window in its front (a roof window is all
    /// window).
    pub window_width: f64,
    pub window_height: f64,
    /// The front and cheeks.
    pub wall_material: MaterialId,
    /// The window frame.
    pub frame_material: MaterialId,
    pub glass_material: MaterialId,
}

/// Thickness of the front and cheeks of a dormer.
pub const DORMER_WALL: f64 = 0.15;
/// How far a dormer's own roof stands out over its front and cheeks.
pub const DORMER_EAVE: f64 = 0.2;
/// Width of a window frame.
pub const DORMER_FRAME: f64 = 0.07;

/// The plane a dormer stands on, in the dormer's local frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DormerFrame {
    /// The middle of the front's outer face, in plan.
    pub origin: Point2,
    /// Unit vector across the dormer (along the side), in plan.
    pub across: Vec2,
    /// Unit vector up the slope, in plan.
    pub up: Vec2,
    /// The roof's pitch there.
    pub slope: f64,
    /// Height of the roof's top face above the level at the front.
    pub z0: f64,
}

impl DormerFrame {
    /// A local point in plan.
    pub fn plan(&self, x: f64, y: f64) -> Point2 {
        self.origin + self.across * x + self.up * y
    }

    /// The roof's top face above the level `y` up the slope.
    pub fn roof_top(&self, y: f64) -> f64 {
        self.z0 + self.slope * y
    }
}

/// One plane of a dormer's own roof, as an inclined slab: its outline in
/// plan (eaves included) and its top face (`base` above the level at
/// `slope.from`, rising as `slope` says).
#[derive(Debug, Clone, PartialEq)]
pub struct DormerPlane {
    pub outline: Vec<Point2>,
    pub slope: SlabSlope,
    pub base: f64,
}

impl DormerPlane {
    /// Its top face above the level at `p`.
    pub fn top_at(&self, p: Point2) -> f64 {
        self.base + self.slope.rise_at(p)
    }
}

/// A dormer worked out on its plane.
#[derive(Debug, Clone, PartialEq)]
pub struct DormerShape {
    pub frame: DormerFrame,
    /// Its footprint on the roof, counter-clockwise: the hole in the plane
    /// (outer faces of the front and cheeks; the window's outline for a
    /// roof window).
    pub footprint: Vec<Point2>,
    /// How far up the slope its cheeks reach (where its eave meets the
    /// roof).
    pub back: f64,
    /// How far up the slope its ridge reaches (gabled ones; else `back`).
    pub ridge_back: f64,
    /// Height of its eave (the head of the front) above the level.
    pub eave_z: f64,
    /// Height of its ridge above the level (gabled ones; else `eave_z`).
    pub ridge_z: f64,
    /// Its own roof.
    pub planes: Vec<DormerPlane>,
    /// The sides whose planes it cuts: its own, and those in the same
    /// plane as it that it reaches over (past where two valleys meet, the
    /// plane of a roof is two faces side by side).
    pub over: Vec<usize>,
}

impl DormerShape {
    /// The front's outline in its own plane: `(x, z)` with `z` above the
    /// level, from the roof's underside (`below` under the top face) up to
    /// the eave and, on a gabled dormer, the ridge; counter-clockwise seen
    /// from the front.
    pub fn front_profile(&self, below: f64, width: f64) -> Vec<Point2> {
        let hw = width / 2.0;
        let z0 = self.frame.z0 - below;
        let mut p = vec![
            Point2::new(-hw, z0),
            Point2::new(hw, z0),
            Point2::new(hw, self.eave_z),
        ];
        if self.ridge_z > self.eave_z + 1e-9 {
            p.push(Point2::new(0.0, self.ridge_z));
        }
        p.push(Point2::new(-hw, self.eave_z));
        p
    }

    /// A cheek's outline in its own plane: `(y, z)`, from the front to
    /// where the eave meets the roof, between the roof's underside and the
    /// eave (on a shed dormer, its sloping roof).
    pub fn cheek_profile(&self, below: f64, kind: DormerKind, pitch: f64) -> Vec<Point2> {
        let f = &self.frame;
        let top = |y: f64| match kind {
            DormerKind::Shed => self.eave_z + pitch * y,
            _ => self.eave_z,
        };
        let b = self.back;
        vec![
            Point2::new(0.0, f.z0 - below),
            Point2::new(b, f.roof_top(b) - below),
            Point2::new(b, top(b)),
            Point2::new(0.0, top(0.0)),
        ]
    }

    /// The window in the front: `(x0, z0, x1, z1)` with `z` above the
    /// level.
    pub fn window(&self, d: &Dormer) -> Option<[f64; 4]> {
        if d.kind == DormerKind::Skylight {
            return None;
        }
        let room = d.width - 2.0 * DORMER_WALL - 0.1;
        let h = self.eave_z - self.frame.z0;
        let ww = d.window_width.min(room);
        let wh = d.window_height.min(h - 0.15);
        if ww < 0.2 || wh < 0.2 {
            return None;
        }
        let sill = self.frame.z0 + ((h - wh) * 0.4).max(0.08);
        Some([-ww / 2.0, sill, ww / 2.0, sill + wh])
    }
}

impl Dormer {
    /// A dormer of `kind` with the usual sizes, of the given materials.
    pub fn new(
        kind: DormerKind,
        wall_material: MaterialId,
        frame_material: MaterialId,
        glass_material: MaterialId,
    ) -> Self {
        let (width, height, pitch, ww, wh) = match kind {
            DormerKind::Gable => (1.6, 1.2, 0.8, 0.9, 0.9),
            DormerKind::Shed => (2.4, 1.2, 0.12, 1.6, 0.9),
            DormerKind::Flat => (2.0, 1.2, 0.0, 1.2, 0.9),
            DormerKind::Skylight => (0.78, 1.18, 0.0, 0.0, 0.0),
        };
        Dormer {
            guid: GlobalId::new(),
            kind,
            side: 0,
            along: 0.0,
            up: 0.0,
            width,
            height,
            pitch,
            window_width: ww,
            window_height: wh,
            wall_material,
            frame_material,
            glass_material,
        }
    }

    fn validate(&self) -> Result<(), ModelError> {
        for (v, what) in [
            (self.along, "dormer position"),
            (self.up, "dormer position"),
            (self.width, "dormer width"),
            (self.height, "dormer height"),
            (self.pitch, "dormer pitch"),
            (self.window_width, "dormer window"),
            (self.window_height, "dormer window"),
        ] {
            finite(v, what)?;
        }
        let ok = (0.4..=8.0).contains(&self.width)
            && (0.3..=4.0).contains(&self.height)
            && (0.0..=MAX_SLAB_SLOPE).contains(&self.pitch)
            && self.window_width >= 0.0
            && self.window_height >= 0.0
            && (self.kind != DormerKind::Gable || self.pitch > 0.05)
            && (self.kind != DormerKind::Shed || self.pitch > 0.0);
        if ok {
            Ok(())
        } else {
            Err(ModelError::InvalidDormer)
        }
    }
}

/// The kinds of roof tile ("cubrición").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileKind {
    /// Curved clay tiles ("teja árabe"): channels and covers.
    Arabic,
    /// Interlocking clay tiles ("teja mixta").
    Mixed,
    /// Flat clay tiles.
    Flat,
    Slate,
    /// Standing-seam metal sheet.
    Metal,
}

impl TileKind {
    pub const ALL: [TileKind; 5] = [
        Self::Arabic,
        Self::Mixed,
        Self::Flat,
        Self::Slate,
        Self::Metal,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Arabic => "Teja árabe",
            Self::Mixed => "Teja mixta",
            Self::Flat => "Teja plana",
            Self::Slate => "Pizarra",
            Self::Metal => "Chapa de junta alzada",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Arabic => "arabic",
            Self::Mixed => "mixed",
            Self::Flat => "flat",
            Self::Slate => "slate",
            Self::Metal => "metal",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.code() == c)
    }

    /// Length of a course along the slope (metres; for metal sheet, none).
    pub fn course(self) -> Option<f64> {
        match self {
            Self::Arabic => Some(0.33),
            Self::Mixed => Some(0.33),
            Self::Flat => Some(0.30),
            Self::Slate => Some(0.22),
            Self::Metal => None,
        }
    }

    /// Distance between the ribs across the slope (covers, rolls or
    /// seams); none for flat tiles and slate.
    pub fn ribs(self) -> Option<f64> {
        match self {
            Self::Arabic => Some(0.20),
            Self::Mixed => Some(0.30),
            Self::Metal => Some(0.50),
            Self::Flat | Self::Slate => None,
        }
    }

    /// Pieces per m² of roof (on the slope), for the measurements; none for
    /// metal sheet (measured in m²).
    pub fn per_m2(self) -> Option<f64> {
        match self {
            Self::Arabic => Some(28.0),
            Self::Mixed => Some(14.0),
            Self::Flat => Some(12.0),
            Self::Slate => Some(35.0),
            Self::Metal => None,
        }
    }
}

/// The shape of a gutter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GutterShape {
    /// Half-round ("media caña").
    HalfRound,
    /// Square ("cuadrado").
    Box,
}

impl GutterShape {
    pub fn name(self) -> &'static str {
        match self {
            Self::HalfRound => "Media caña",
            Self::Box => "Cuadrado",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::HalfRound => "half_round",
            Self::Box => "box",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        [Self::HalfRound, Self::Box]
            .into_iter()
            .find(|k| k.code() == c)
    }
}

/// Gutters along every eave of a roof, with their downpipes.
#[derive(Debug, Clone, PartialEq)]
pub struct Gutter {
    pub shape: GutterShape,
    /// Width of the gutter (metres).
    pub size: f64,
    pub material: MaterialId,
    /// Diameter of the downpipes.
    pub downpipe: f64,
    /// Greatest length of gutter one downpipe serves (one at each end of
    /// an eave, and more in between when it is longer).
    pub spacing: f64,
}

/// What covers a roof: its tiles and its gutters.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RoofCovering {
    /// None: the top layer of the roof type, as before.
    pub tiles: Option<TileKind>,
    /// The tiles' material; none: the top layer's.
    pub tile_material: Option<MaterialId>,
    pub gutter: Option<Gutter>,
}

impl Gutter {
    fn validate(&self) -> Result<(), ModelError> {
        for v in [self.size, self.downpipe, self.spacing] {
            finite(v, "gutter")?;
        }
        if (0.05..=0.5).contains(&self.size)
            && (0.03..=0.3).contains(&self.downpipe)
            && (2.0..=50.0).contains(&self.spacing)
        {
            Ok(())
        } else {
            Err(ModelError::InvalidDormer)
        }
    }
}

/// One stretch of gutter along an eave, with its downpipes.
#[derive(Debug, Clone, PartialEq)]
pub struct GutterRun {
    /// The side of the footprint.
    pub side: usize,
    /// Along the eave, in plan: the gutter's centre line.
    pub start: Point2,
    pub end: Point2,
    /// Height of the gutter's top above the level.
    pub z: f64,
    /// Outward from the eave, in plan.
    pub out: Vec2,
    /// Downpipes: where they meet the wall face (in plan).
    pub downpipes: Vec<Point2>,
}

impl Roof {
    /// The plane a dormer stands on, in its local frame; `None` if its
    /// side has no slope or no longer exists.
    pub fn dormer_frame(&self, d: &Dormer) -> Option<DormerFrame> {
        let n = self.outline.len();
        if d.side >= n {
            return None;
        }
        let s = self.slopes[d.side]?;
        let (a, b) = (self.outline[d.side], self.outline[(d.side + 1) % n]);
        let across = (b - a).normalized()?;
        let up = across.perp_left();
        let mid = a + (b - a) * 0.5;
        Some(DormerFrame {
            origin: mid + across * d.along + up * d.up,
            across,
            up,
            slope: s,
            z0: self.base_offset + s * d.up,
        })
    }

    /// A dormer worked out on its plane; `None` if it does not fit on it
    /// (its footprint is not all on that plane, or its roof is not less
    /// steep than the plane where it must be).
    pub fn dormer_shape(&self, d: &Dormer) -> Option<DormerShape> {
        let f = self.dormer_frame(d)?;
        let s = f.slope;
        let (hw, h, p, o) = (d.width / 2.0, d.height, d.pitch, DORMER_EAVE);
        let eave_z = f.z0 + h;
        let pl = |pts: &[(f64, f64)]| -> Vec<Point2> {
            pts.iter().map(|&(x, y)| f.plan(x, y)).collect()
        };
        let shape = match d.kind {
            DormerKind::Skylight => {
                let len = h / (1.0 + s * s).sqrt();
                DormerShape {
                    frame: f,
                    footprint: pl(&[(-hw, 0.0), (hw, 0.0), (hw, len), (-hw, len)]),
                    back: len,
                    ridge_back: len,
                    eave_z: f.z0,
                    ridge_z: f.z0,
                    planes: Vec::new(),
                    over: vec![d.side],
                }
            }
            DormerKind::Gable => {
                let ye = h / s;
                let yr = (h + p * hw) / s;
                let yo = (h - p * o) / s;
                if yo <= -o + 1e-6 {
                    return None;
                }
                let left = DormerPlane {
                    outline: pl(&[(-hw - o, -o), (0.0, -o), (0.0, yr), (-hw - o, yo)]),
                    slope: SlabSlope {
                        from: f.plan(-hw, 0.0),
                        to: f.plan(-hw + 1.0, 0.0),
                        slope: p,
                    },
                    base: eave_z,
                };
                let right = DormerPlane {
                    outline: pl(&[(0.0, -o), (hw + o, -o), (hw + o, yo), (0.0, yr)]),
                    slope: SlabSlope {
                        from: f.plan(hw, 0.0),
                        to: f.plan(hw - 1.0, 0.0),
                        slope: p,
                    },
                    base: eave_z,
                };
                DormerShape {
                    frame: f,
                    footprint: pl(&[(-hw, 0.0), (hw, 0.0), (hw, ye), (0.0, yr), (-hw, ye)]),
                    back: ye,
                    ridge_back: yr,
                    eave_z,
                    ridge_z: eave_z + p * hw,
                    planes: vec![left, right],
                    over: vec![d.side],
                }
            }
            DormerKind::Shed | DormerKind::Flat => {
                let p = if d.kind == DormerKind::Flat { 0.0 } else { p };
                if p >= s - 1e-3 {
                    return None;
                }
                let yb = h / (s - p);
                let plane = DormerPlane {
                    outline: pl(&[(-hw - o, -o), (hw + o, -o), (hw + o, yb), (-hw - o, yb)]),
                    slope: SlabSlope {
                        from: f.plan(0.0, 0.0),
                        to: f.plan(0.0, 1.0),
                        slope: p,
                    },
                    base: eave_z,
                };
                DormerShape {
                    frame: f,
                    footprint: pl(&[(-hw, 0.0), (hw, 0.0), (hw, yb), (-hw, yb)]),
                    back: yb,
                    ridge_back: yb,
                    eave_z,
                    ridge_z: eave_z,
                    planes: vec![plane],
                    over: vec![d.side],
                }
            }
        };
        // All of it on its plane, a little in from its edges: on the
        // pieces of the roof in that plane (its own side's, and those of
        // other sides in the same plane, as the two faces past where two
        // valleys meet), away from the edges they do not share.
        let planes = self.planes();
        let own = planes.iter().find(|pl| pl.side == d.side)?;
        let same: Vec<&RoofPlane> = planes
            .iter()
            .filter(|pl| pl.side == d.side || coplanar(own, pl, self.base_offset))
            .collect();
        let shared = |piece: usize, a: Point2, b: Point2| {
            let mid = a + (b - a) * 0.5;
            same.iter()
                .enumerate()
                .any(|(j, pl)| j != piece && edge_distance(mid, &pl.outline) < 1e-6)
        };
        let on = |q: Point2| {
            same.iter().enumerate().any(|(i, pl)| {
                if !point_in_polygon(q, &pl.outline) {
                    return false;
                }
                let n = pl.outline.len();
                (0..n).all(|k| {
                    let (a, b) = (pl.outline[k], pl.outline[(k + 1) % n]);
                    shared(i, a, b) || edge_distance(q, &[a, b]) > 0.02
                })
            })
        };
        if !shape.footprint.iter().all(|q| on(*q)) {
            return None;
        }
        // Not over another plane's side either: the middle of each edge.
        let m = shape.footprint.len();
        for i in 0..m {
            let (a, b) = (shape.footprint[i], shape.footprint[(i + 1) % m]);
            if !on(a + (b - a) * 0.5) {
                return None;
            }
        }
        let mut shape = shape;
        shape.over = Vec::new();
        for pl in &same {
            if !shape.over.contains(&pl.side)
                && signed_area(&clip_to_convex(&pl.outline, &shape.footprint)).abs() > 1e-6
            {
                shape.over.push(pl.side);
            }
        }
        if !shape.over.contains(&d.side) {
            shape.over.insert(0, d.side);
        }
        Some(shape)
    }

    /// The holes the dormers make in the plane rising from `side`: their
    /// footprints, whole (one may reach over the edge of a piece of that
    /// plane into another side's piece in the same plane; see
    /// [`Roof::plane_cuts`]).
    pub fn plane_holes(&self, side: usize) -> Vec<Vec<Point2>> {
        self.dormers
            .iter()
            .filter_map(|d| self.dormer_shape(d))
            .filter(|s| s.over.contains(&side))
            .map(|s| s.footprint)
            .collect()
    }

    /// What the dormers take out of one piece of the roof: the holes all
    /// inside it, and the footprints that cross its edge (to be taken
    /// from its outline).
    pub fn plane_cuts(&self, plane: &RoofPlane) -> (Vec<Vec<Point2>>, Vec<Vec<Point2>>) {
        let mut inside = Vec::new();
        let mut crossing = Vec::new();
        for h in self.plane_holes(plane.side) {
            if h.iter().all(|q| point_in_polygon(*q, &plane.outline)) {
                inside.push(h);
            } else if signed_area(&clip_to_convex(&plane.outline, &h)).abs() > 1e-6 {
                crossing.push(h);
            }
        }
        (inside, crossing)
    }

    /// Where a dormer at `at` (in plan) would stand: the side whose plane
    /// covers it, and its `along` and `up`.
    pub fn locate_on_plane(&self, at: Point2) -> Option<(usize, f64, f64)> {
        let plane = self
            .planes()
            .into_iter()
            .find(|pl| point_in_polygon(at, &pl.outline))?;
        let n = self.outline.len();
        let (a, b) = (self.outline[plane.side], self.outline[(plane.side + 1) % n]);
        let across = (b - a).normalized()?;
        let mid = a + (b - a) * 0.5;
        Some((
            plane.side,
            (at - mid).dot(across),
            (at - a).dot(across.perp_left()),
        ))
    }

    /// The ridges and hips (where two planes meet over the roof, the
    /// water running away from them), for ridge tiles: pairs of points
    /// in plan.
    pub fn ridge_lines(&self) -> Vec<(Point2, Point2)> {
        self.plane_meetings(true)
    }

    /// The valleys ("limahoyas"), where the water runs together.
    pub fn valley_lines(&self) -> Vec<(Point2, Point2)> {
        self.plane_meetings(false)
    }

    fn plane_meetings(&self, ridges: bool) -> Vec<(Point2, Point2)> {
        let planes = self.planes();
        let key = |p: Point2| ((p.x * 1e6).round() as i64, (p.y * 1e6).round() as i64);
        let mut out = Vec::new();
        for (i, a) in planes.iter().enumerate() {
            let m = a.outline.len();
            for k in 0..m {
                let (p, q) = (a.outline[k], a.outline[(k + 1) % m]);
                // Shared with a later plane (the other way round).
                let shared = planes.iter().skip(i + 1).any(|b| {
                    let l = b.outline.len();
                    (0..l).any(|j| {
                        key(b.outline[j]) == key(q) && key(b.outline[(j + 1) % l]) == key(p)
                    })
                });
                if !shared || p.distance(q) < 1e-6 {
                    continue;
                }
                // A ridge or hip is higher than either side of it.
                let mid = p + (q - p) * 0.5;
                let n = (q - p)
                    .normalized()
                    .map_or(Vec2::new(0.0, 0.0), |u| u.perp_left());
                let z = self.top_at(mid);
                let (za, zb) = (self.top_at(mid + n * 0.05), self.top_at(mid - n * 0.05));
                let convex = za < z - 1e-6 && zb < z - 1e-6;
                let concave = za > z + 1e-6 && zb > z + 1e-6;
                // Two faces in one plane (past where two valleys meet) do
                // not make either.
                if (ridges && convex) || (!ridges && concave) {
                    out.push((p, q));
                }
            }
        }
        out
    }

    /// The gutters along the eaves (sloping sides with a horizontal eave),
    /// with their downpipes; none without a gutter.
    pub fn gutter_runs(&self) -> Vec<GutterRun> {
        let Some(g) = &self.covering.gutter else {
            return Vec::new();
        };
        let n = self.outline.len();
        let eaves = self.eaves();
        let mut out = Vec::new();
        for i in 0..n {
            let Some(s) = self.slopes[i] else { continue };
            let (a, b) = (self.outline[i], self.outline[(i + 1) % n]);
            let Some(u) = (b - a).normalized() else {
                continue;
            };
            let outward = -u.perp_left();
            // The eave line of this side: its edge of the eaves.
            let (ea, eb) = (eaves[i], eaves[(i + 1) % n]);
            let (ea, eb) = if eaves.len() == n {
                (ea, eb)
            } else {
                (a + outward * self.overhang, b + outward * self.overhang)
            };
            let len = ea.distance(eb);
            if len < 0.3 {
                continue;
            }
            let z = self.base_offset - s * self.overhang;
            let centre = outward * (g.size / 2.0);
            // Downpipes: in from each end, more where the eave is long,
            // against the wall face (the footprint).
            let count = ((len / g.spacing).ceil() as usize).max(1) + 1;
            let wall_len = a.distance(b);
            let inset = 0.25_f64.min(wall_len / 4.0);
            let downpipes = (0..count)
                .map(|k| {
                    let t = if count == 1 {
                        0.5
                    } else {
                        k as f64 / (count - 1) as f64
                    };
                    a + u * (inset + (wall_len - 2.0 * inset) * t)
                        + outward * (g.downpipe / 2.0 + 0.02)
                })
                .collect();
            out.push(GutterRun {
                side: i,
                start: ea + centre,
                end: eb + centre,
                z,
                out: outward,
                downpipes,
            });
        }
        out
    }
}

/// Two pieces of a roof in the same plane: the same height at three
/// points.
fn coplanar(a: &RoofPlane, b: &RoofPlane, base: f64) -> bool {
    let z = |pl: &RoofPlane, p: Point2| base + pl.slope.rise_at(p);
    let o = a.outline[0];
    [o, o + Vec2::new(1.0, 0.0), o + Vec2::new(0.0, 1.0)]
        .iter()
        .all(|p| (z(a, *p) - z(b, *p)).abs() < 1e-6)
}

/// The part of `poly` inside the convex counter-clockwise polygon `clip`.
pub fn clip_to_convex(poly: &[Point2], clip: &[Point2]) -> Vec<Point2> {
    let mut clip = clip.to_vec();
    if signed_area(&clip) < 0.0 {
        clip.reverse();
    }
    let n = clip.len();
    let mut out = poly.to_vec();
    for i in 0..n {
        if out.len() < 3 {
            return Vec::new();
        }
        let (a, b) = (clip[i], clip[(i + 1) % n]);
        let Some(inward) = (b - a).normalized().map(|u| u.perp_left()) else {
            continue;
        };
        out = clip_half_plane(&out, -inward, -inward.dot(a));
    }
    out
}

/// Shortest distance from `p` to the edges of a polygon.
fn edge_distance(p: Point2, poly: &[Point2]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            let d = b - a;
            let len2 = d.dot(d);
            if len2 <= 0.0 {
                return p.distance(a);
            }
            let t = ((p - a).dot(d) / len2).clamp(0.0, 1.0);
            (a + d * t).distance(p)
        })
        .fold(f64::INFINITY, f64::min)
}

impl Model {
    /// The materials a new dormer is made of: a render for its front and
    /// cheeks, a frame and glass (by category, else the roof's own top
    /// layer).
    pub fn dormer_materials(
        &self,
        roof: RoofId,
    ) -> Result<(MaterialId, MaterialId, MaterialId), ModelError> {
        let r = self.roofs.get(&roof).ok_or(ModelError::UnknownRoof(roof))?;
        let top = self
            .slab_types
            .get(&r.roof_type)
            .and_then(|t| t.layers.first())
            .map(|l| l.material);
        let by = |cats: &[&str]| {
            cats.iter().find_map(|c| {
                self.materials
                    .values()
                    .find(|m| m.category == *c)
                    .map(|m| m.id)
            })
        };
        let fallback = top
            .or_else(|| self.materials.keys().next().copied())
            .ok_or(ModelError::InvalidDormer)?;
        Ok((
            by(&["plaster", "mortar"]).unwrap_or(fallback),
            by(&["aluminium", "wood", "metal"]).unwrap_or(fallback),
            by(&["glass"]).unwrap_or(fallback),
        ))
    }

    /// Puts a dormer of `kind` on the roof with the middle of its front at
    /// `at` (in plan), with the usual sizes; its index in the roof.
    pub fn add_dormer(
        &mut self,
        roof: RoofId,
        kind: DormerKind,
        at: Point2,
    ) -> Result<usize, ModelError> {
        let (w, fr, gl) = self.dormer_materials(roof)?;
        let mut d = Dormer::new(kind, w, fr, gl);
        self.place_dormer(roof, &mut d, at)?;
        let r = self
            .roofs
            .get_mut(&roof)
            .ok_or(ModelError::UnknownRoof(roof))?;
        r.dormers.push(d);
        Ok(r.dormers.len() - 1)
    }

    /// Puts a dormer as it is (its sizes and materials) at `at`; its index.
    pub fn insert_dormer(
        &mut self,
        roof: RoofId,
        mut d: Dormer,
        at: Point2,
    ) -> Result<usize, ModelError> {
        self.place_dormer(roof, &mut d, at)?;
        let r = self
            .roofs
            .get_mut(&roof)
            .ok_or(ModelError::UnknownRoof(roof))?;
        r.dormers.push(d);
        Ok(r.dormers.len() - 1)
    }

    fn place_dormer(&self, roof: RoofId, d: &mut Dormer, at: Point2) -> Result<(), ModelError> {
        let r = self.roofs.get(&roof).ok_or(ModelError::UnknownRoof(roof))?;
        finite_point(at, "dormer position")?;
        let (side, along, up) = r.locate_on_plane(at).ok_or(ModelError::DormerDoesNotFit)?;
        d.side = side;
        d.along = along;
        d.up = up;
        self.check_dormer(r, d)
    }

    /// Its sizes and materials (not whether it fits).
    pub(super) fn check_dormer_parts(&self, d: &Dormer) -> Result<(), ModelError> {
        d.validate()?;
        for m in [d.wall_material, d.frame_material, d.glass_material] {
            if !self.materials.contains_key(&m) {
                return Err(ModelError::UnknownMaterial(m));
            }
        }
        Ok(())
    }

    pub(super) fn check_covering(&self, covering: &RoofCovering) -> Result<(), ModelError> {
        for m in covering
            .tile_material
            .iter()
            .chain(covering.gutter.as_ref().map(|g| &g.material))
        {
            if !self.materials.contains_key(m) {
                return Err(ModelError::UnknownMaterial(*m));
            }
        }
        if let Some(g) = &covering.gutter {
            g.validate()?;
        }
        Ok(())
    }

    fn check_dormer(&self, r: &Roof, d: &Dormer) -> Result<(), ModelError> {
        self.check_dormer_parts(d)?;
        if r.dormer_shape(d).is_none() {
            return Err(ModelError::DormerDoesNotFit);
        }
        Ok(())
    }

    /// Changes a dormer; nothing changes if it would not fit.
    pub fn edit_dormer(
        &mut self,
        roof: RoofId,
        index: usize,
        edit: impl FnOnce(&mut Dormer),
    ) -> Result<(), ModelError> {
        let r = self.roofs.get(&roof).ok_or(ModelError::UnknownRoof(roof))?;
        let mut d = r
            .dormers
            .get(index)
            .cloned()
            .ok_or(ModelError::UnknownDormer(index))?;
        let guid = d.guid.clone();
        edit(&mut d);
        d.guid = guid;
        self.check_dormer(r, &d)?;
        self.roofs.get_mut(&roof).expect("checked").dormers[index] = d;
        Ok(())
    }

    /// Moves a dormer so that the middle of its front is at `to`, on
    /// whatever plane of its roof is there.
    pub fn move_dormer(
        &mut self,
        roof: RoofId,
        index: usize,
        to: Point2,
    ) -> Result<(), ModelError> {
        let r = self.roofs.get(&roof).ok_or(ModelError::UnknownRoof(roof))?;
        let mut d = r
            .dormers
            .get(index)
            .cloned()
            .ok_or(ModelError::UnknownDormer(index))?;
        self.place_dormer(roof, &mut d, to)?;
        self.roofs.get_mut(&roof).expect("checked").dormers[index] = d;
        Ok(())
    }

    pub fn remove_dormer(&mut self, roof: RoofId, index: usize) -> Result<Dormer, ModelError> {
        let r = self
            .roofs
            .get_mut(&roof)
            .ok_or(ModelError::UnknownRoof(roof))?;
        if index >= r.dormers.len() {
            return Err(ModelError::UnknownDormer(index));
        }
        Ok(r.dormers.remove(index))
    }

    /// Changes what covers a roof.
    pub fn set_roof_covering(
        &mut self,
        roof: RoofId,
        covering: RoofCovering,
    ) -> Result<(), ModelError> {
        if !self.roofs.contains_key(&roof) {
            return Err(ModelError::UnknownRoof(roof));
        }
        self.check_covering(&covering)?;
        self.roofs.get_mut(&roof).expect("checked").covering = covering;
        Ok(())
    }

    /// A gutter with the usual sizes, of a metal of the project (zinc,
    /// aluminium… by category; else the first material).
    pub fn default_gutter(&self) -> Option<Gutter> {
        let material = ["metal", "zinc", "aluminium"]
            .iter()
            .find_map(|c| self.materials.values().find(|m| m.category == *c))
            .or_else(|| self.materials.values().next())?
            .id;
        Some(Gutter {
            shape: GutterShape::HalfRound,
            size: 0.125,
            material,
            downpipe: 0.08,
            spacing: 10.0,
        })
    }

    /// The material of a roof's tiles: its own, or the roof type's top
    /// layer.
    pub fn tile_material(&self, roof: &Roof) -> Option<MaterialId> {
        roof.covering.tile_material.or_else(|| {
            self.slab_types
                .get(&roof.roof_type)
                .and_then(|t| t.layers.first())
                .map(|l| l.material)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Model, RoofId) {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let tile = m.add_material("Teja", "ceramic", [190, 100, 70], crate::CutPattern::Solid);
        m.add_material(
            "Monocapa",
            "plaster",
            [240, 240, 230],
            crate::CutPattern::Solid,
        );
        m.add_material(
            "Aluminio",
            "aluminium",
            [120, 120, 120],
            crate::CutPattern::Solid,
        );
        m.add_material("Vidrio", "glass", [200, 220, 240], crate::CutPattern::Solid);
        let t = m
            .add_slab_type(
                "Cubierta",
                SlabFunction::Roof,
                vec![Layer {
                    material: tile,
                    thickness: 0.25,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        // A gabled roof 10 × 12 at 50 %, ridge along x.
        let r = m
            .add_roof(NewRoof {
                roof_type: t,
                level: l,
                outline: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(10.0, 0.0),
                    Point2::new(10.0, 12.0),
                    Point2::new(0.0, 12.0),
                ],
                slopes: vec![Some(0.5), None, Some(0.5), None],
                base_offset: 3.0,
                overhang: 0.5,
            })
            .unwrap();
        (m, r)
    }

    #[test]
    fn a_gabled_dormer_fits_its_plane() {
        let (mut m, r) = setup();
        let i = m
            .add_dormer(r, DormerKind::Gable, Point2::new(5.0, 1.0))
            .unwrap();
        let roof = m.roof(r).unwrap();
        let d = &roof.dormers[i];
        assert_eq!(d.side, 0);
        assert!((d.along).abs() < 1e-9 && (d.up - 1.0).abs() < 1e-9);
        let s = roof.dormer_shape(d).unwrap();
        // Front on the roof at 3.5 m, eave 1.2 m higher, the ridge in the
        // middle at 0.8 × 0.8 m above that.
        assert!((s.frame.z0 - 3.5).abs() < 1e-9);
        assert!((s.eave_z - 4.7).abs() < 1e-9);
        assert!((s.ridge_z - (4.7 + 0.64)).abs() < 1e-9);
        // Its cheeks die into the roof 1.2 / 0.5 = 2.4 m up.
        assert!((s.back - 2.4).abs() < 1e-9);
        // The ridge meets the plane where it is as high.
        let tip = s.frame.plan(0.0, s.ridge_back);
        assert!((roof.top_at(tip) - s.ridge_z).abs() < 1e-9);
        assert_eq!(roof.plane_holes(0).len(), 1);
        assert!(roof.plane_holes(2).is_empty());
    }

    #[test]
    fn too_big_or_off_the_roof() {
        let (mut m, r) = setup();
        // Over the ridge.
        assert_eq!(
            m.add_dormer(r, DormerKind::Gable, Point2::new(5.0, 5.0)),
            Err(ModelError::DormerDoesNotFit)
        );
        // Off the roof.
        assert_eq!(
            m.add_dormer(r, DormerKind::Gable, Point2::new(20.0, 1.0)),
            Err(ModelError::DormerDoesNotFit)
        );
        let i = m
            .add_dormer(r, DormerKind::Shed, Point2::new(2.0, 0.5))
            .unwrap();
        // A shed roof as steep as the roof cannot meet it.
        assert_eq!(
            m.edit_dormer(r, i, |d| d.pitch = 0.6),
            Err(ModelError::DormerDoesNotFit)
        );
        // Wider than the roof.
        assert!(m.edit_dormer(r, i, |d| d.width = 6.0).is_err());
        assert!(m.edit_dormer(r, i, |d| d.width = 3.0).is_ok());
    }

    #[test]
    fn dormers_follow_their_roof() {
        let (mut m, r) = setup();
        let i = m
            .add_dormer(r, DormerKind::Skylight, Point2::new(3.0, 10.5))
            .unwrap();
        assert_eq!(m.roof(r).unwrap().dormers[i].side, 2);
        let before = {
            let roof = m.roof(r).unwrap();
            roof.dormer_frame(&roof.dormers[i]).unwrap().origin
        };
        m.edit_roof(r, |roof| {
            for p in &mut roof.outline {
                *p = *p + Vec2::new(4.0, 1.0);
            }
        })
        .unwrap();
        let roof = m.roof(r).unwrap();
        let after = roof.dormer_frame(&roof.dormers[i]).unwrap().origin;
        assert!(after.distance(before + Vec2::new(4.0, 1.0)) < 1e-9);
        assert!(roof.dormer_shape(&roof.dormers[i]).is_some());
    }

    #[test]
    fn ridges_valleys_and_gutters() {
        let (mut m, r) = setup();
        let roof = m.roof(r).unwrap();
        let ridges = roof.ridge_lines();
        assert_eq!(ridges.len(), 1);
        assert!(roof.valley_lines().is_empty());
        let g = m.default_gutter().unwrap();
        m.set_roof_covering(
            r,
            RoofCovering {
                tiles: Some(TileKind::Arabic),
                tile_material: None,
                gutter: Some(g),
            },
        )
        .unwrap();
        let runs = m.roof(r).unwrap().gutter_runs();
        // The two eaves, each 11 m: two downpipes and one in between.
        assert_eq!(runs.len(), 2);
        assert!(runs.iter().all(|g| g.downpipes.len() == 3));
        // At the eave, 0.5 m out at 50 %: 25 cm under the base.
        assert!((runs[0].z - 2.75).abs() < 1e-9);
    }

    /// A T: past where its two valleys meet, the main plane is two faces
    /// side by side (one per piece of its eave). A dormer over the line
    /// between them stands, cuts both and is tiled round.
    #[test]
    fn over_where_two_valleys_meet() {
        let (mut m, _) = setup();
        let t = m.roofs().next().unwrap().roof_type;
        let l = m.levels()[0].id;
        let p = |x, y| Point2::new(x, y);
        let r = m
            .add_roof(NewRoof {
                roof_type: t,
                level: l,
                outline: vec![
                    p(20.0, 4.0),
                    p(24.0, 4.0),
                    p(24.0, 0.0),
                    p(28.0, 0.0),
                    p(28.0, 4.0),
                    p(32.0, 4.0),
                    p(32.0, 16.0),
                    p(20.0, 16.0),
                ],
                slopes: vec![Some(0.5)],
                base_offset: 3.0,
                overhang: 0.5,
            })
            .unwrap();
        let roof = m.roof(r).unwrap();
        // The valleys meet at (26, 6); the faces of the two pieces of the
        // eave meet on x = 26 above it, which is neither ridge nor valley.
        assert_eq!(roof.valley_lines().len(), 2);
        let i = m
            .add_dormer(r, DormerKind::Gable, Point2::new(26.0, 6.2))
            .unwrap();
        let roof = m.roof(r).unwrap();
        let s = roof.dormer_shape(&roof.dormers[i]).unwrap();
        assert_eq!(s.over.len(), 2, "{:?}", s.over);
        let pieces: Vec<RoofPlane> = roof
            .planes()
            .into_iter()
            .filter(|pl| s.over.contains(&pl.side))
            .collect();
        let taken: f64 = pieces
            .iter()
            .map(|pl| {
                let (inside, crossing) = roof.plane_cuts(pl);
                assert!(inside.is_empty());
                crossing
                    .iter()
                    .map(|h| signed_area(&clip_to_convex(&pl.outline, h)).abs())
                    .sum::<f64>()
            })
            .sum();
        assert!((taken - signed_area(&s.footprint).abs()).abs() < 1e-9);
        // The roof's surface is less the footprint, on the slope.
        let whole = {
            let mut bare = roof.clone();
            bare.dormers.clear();
            bare.surface_area()
        };
        let k = (1.0f64 + 0.25).sqrt();
        assert!((whole - roof.surface_area() - taken * k).abs() < 1e-9);
        // Still not over a hip or the ridge.
        assert!(m
            .add_dormer(r, DormerKind::Gable, Point2::new(26.0, 8.5))
            .is_err());
    }
}
