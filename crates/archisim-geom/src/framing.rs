//! Light framing (steel frame and timber frame): the members of the walls,
//! floors and roofs whose types are framed (`Model::type_framing`), laid
//! from the elements as they are, so that they follow every change. Nothing
//! is stored: the members, the prefabricated panels and the cut list are
//! worked out here each time.
//!
//! - Walls (straight ones): cut into panels no longer than [`PANEL_MAX`]
//!   at a stud; each panel with its bottom and top tracks (plates; a double
//!   top plate in timber), studs at the spacing from the start of the wall,
//!   a double stud at each end, and round every door and window a king
//!   stud, a jack stud, a header, a sill track and short studs above and
//!   below; a row of noggins at mid-height. The frame stands in the type's
//!   frame layer (`Model::frame_layer`). Where a wall meets another, it
//!   stops at the other's frame (a T), or one of the two (the older) runs
//!   to the outside of the other's (an L).
//! - Floors: joists square to the longest side of the outline at the
//!   spacing, cut by the outline and its holes, with a rim joist round the
//!   edges and the holes, and a row of blocking at the middle.
//! - Roofs: rafters down each plane, with a rim along its edges (eaves,
//!   verges, ridge and hips).

use crate::solid::{Solid, SolidOwner, P3};
use archisim_core::{FrameMaterial, FrameSection, Framing, Model, Point2, Vec2, WallId};

/// The longest prefabricated wall panel that travels on a lorry, metres.
pub const PANEL_MAX: f64 = 6.0;

/// What a member does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    BottomTrack,
    TopTrack,
    /// The second top plate of a timber wall, over the joints of the panels.
    TopPlate,
    Stud,
    /// The full-height stud beside a door or window.
    KingStud,
    /// The stud under the header, beside the king stud.
    JackStud,
    Header,
    /// The track under a window.
    SillTrack,
    /// A short stud above a header or under a sill.
    Cripple,
    Noggin,
    Joist,
    Rim,
    Blocking,
    Rafter,
    /// The edge members of a roof plane: eaves, verge, ridge, hip.
    RoofRim,
}

impl Role {
    pub const ALL: [Role; 15] = [
        Role::BottomTrack,
        Role::TopTrack,
        Role::TopPlate,
        Role::Stud,
        Role::KingStud,
        Role::JackStud,
        Role::Header,
        Role::SillTrack,
        Role::Cripple,
        Role::Noggin,
        Role::Joist,
        Role::Rim,
        Role::Blocking,
        Role::Rafter,
        Role::RoofRim,
    ];

    /// Its name, as each trade calls it.
    pub fn name(self, m: FrameMaterial) -> &'static str {
        let steel = m == FrameMaterial::Steel;
        match self {
            Role::BottomTrack if steel => "Canal inferior",
            Role::BottomTrack => "Durmiente inferior",
            Role::TopTrack if steel => "Canal superior",
            Role::TopTrack => "Carrera superior",
            Role::TopPlate => "Carrera de atado",
            Role::Stud => "Montante",
            Role::KingStud => "Montante de refuerzo",
            Role::JackStud => "Montante de apoyo del dintel",
            Role::Header => "Dintel",
            Role::SillTrack if steel => "Canal de antepecho",
            Role::SillTrack => "Travesaño de antepecho",
            Role::Cripple => "Montante corto",
            Role::Noggin if steel => "Travesaño (bloqueo)",
            Role::Noggin => "Travesaño (noggin)",
            Role::Joist => "Vigueta",
            Role::Rim if steel => "Canal de borde (zuncho)",
            Role::Rim => "Zuncho de borde",
            Role::Blocking => "Riostra",
            Role::Rafter => "Cabio",
            Role::RoofRim => "Zuncho de faldón",
        }
    }

    /// Whether it is cut from the track section (else from the stud's).
    fn of_track(self) -> bool {
        matches!(
            self,
            Role::BottomTrack
                | Role::TopTrack
                | Role::TopPlate
                | Role::SillTrack
                | Role::Noggin
                | Role::Rim
                | Role::Blocking
                | Role::RoofRim
        )
    }
}

/// One member of a frame: a straight piece of a section between two
/// points, its web (`depth`) along `across`, its flanges (`width`) along
/// `open` (steel Cs open that way).
#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    pub owner: SolidOwner,
    pub role: Role,
    pub material: FrameMaterial,
    pub section: FrameSection,
    pub start: P3,
    pub end: P3,
    pub across: P3,
    pub open: P3,
    /// The prefabricated panel it belongs to (walls), from 0.
    pub panel: Option<usize>,
}

impl Member {
    pub fn length(&self) -> f64 {
        len(sub(self.end, self.start))
    }

    pub fn kg(&self) -> f64 {
        self.length() * self.section.kg_per_m()
    }

    /// Volume of timber, m³ (0 for steel).
    pub fn m3(&self) -> f64 {
        if self.section.is_steel() {
            0.0
        } else {
            self.length() * self.section.area()
        }
    }
}

fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: P3, b: P3) -> P3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn mul(a: P3, k: f64) -> P3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn len(a: P3) -> f64 {
    dot(a, a).sqrt()
}
fn p3(p: Point2, z: f64) -> P3 {
    [p.x, p.y, z]
}
fn v3(v: Vec2) -> P3 {
    [v.x, v.y, 0.0]
}
const UP: P3 = [0.0, 0.0, 1.0];

thread_local! {
    static SHOWN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs `f` with the framed walls, floors and roofs drawn as their bare
/// frames (the 3D view's «Entramado a la vista», the pictures).
pub fn with_frames_shown<T>(f: impl FnOnce() -> T) -> T {
    let old = SHOWN.with(|c| c.replace(true));
    let out = f();
    SHOWN.with(|c| c.set(old));
    out
}

/// Whether [`crate::solid::model_solids`] draws the bare frames.
pub fn frames_shown() -> bool {
    SHOWN.with(|c| c.get())
}

/// Where the frame of a type stands in its layers: the offset of its
/// middle from the first face (the exterior face of a wall, the top of a
/// slab) and the depth of its layer.
fn frame_band(layers: &[archisim_core::Layer]) -> Option<(f64, f64)> {
    let k = Model::frame_layer(layers)?;
    let before: f64 = layers[..k].iter().map(|l| l.thickness).sum();
    Some((before + layers[k].thickness / 2.0, layers[k].thickness))
}

/// A straight wall's frame line: its start, its direction, its length,
/// its normal (to the exterior), the offset of the frame's middle from the
/// drawn line and the frame's depth.
struct WallLine {
    a: Point2,
    u: Vec2,
    n: Vec2,
    length: f64,
    centre: f64,
    depth: f64,
    framed: bool,
}

fn wall_line(model: &Model, id: WallId) -> Option<WallLine> {
    let w = model.wall(id)?;
    if w.bulge.abs() > 1e-9 {
        return None;
    }
    let t = model.wall_type(w.wall_type)?;
    let (ext, _) = w.face_offsets(t.thickness());
    let (mid, layer) = frame_band(&t.layers)?;
    let framed = model.type_framing(w.wall_type.0).is_some();
    let depth = model
        .type_framing(w.wall_type.0)
        .map_or(layer, |f| f.stud.depth.max(f.track.depth));
    let d = w.end - w.start;
    let u = d.normalized()?;
    Some(WallLine {
        a: w.start,
        u,
        n: u.perp_left(),
        length: d.length(),
        centre: ext - mid,
        depth,
        framed,
    })
}

/// How much the frame of `id` is lengthened (positive) or shortened at its
/// start and end where it meets other walls.
fn wall_end_trims(model: &Model, id: WallId, line: &WallLine) -> [f64; 2] {
    let Some(w) = model.wall(id) else {
        return [0.0; 2];
    };
    let mut out = [0.0; 2];
    for (k, end) in [(0usize, w.start), (1, w.end)] {
        // Pointing out of the wall at this end.
        let out_dir = if k == 0 { -line.u } else { line.u };
        let p = end + line.n * line.centre;
        let mut best: Option<f64> = None;
        for o in model.walls() {
            if o.id == id || o.level != w.level || !w.allow_join[k] {
                continue;
            }
            let Some(ol) = wall_line(model, o.id) else {
                continue;
            };
            let thick = model.wall_type(o.wall_type).map_or(0.3, |t| t.thickness());
            // Near the other's drawn line, within its length.
            let along = (p - ol.a).dot(ol.u);
            if along < -thick || along > ol.length + thick {
                continue;
            }
            let off = (p - ol.a).dot(ol.n);
            if (off - ol.centre).abs() > thick + line.depth {
                continue;
            }
            let k_dir = out_dir.dot(ol.n);
            if k_dir.abs() < 0.3 {
                continue;
            }
            // An L: the ends are together. The older wall runs through.
            let at_end = along < thick || along > ol.length - thick;
            let through = at_end && ol.framed && id.0 < o.id.0;
            let target = if (k_dir > 0.0) != through {
                ol.centre - ol.depth / 2.0
            } else {
                ol.centre + ol.depth / 2.0
            };
            let t = (target - off) / k_dir;
            if t.abs() <= thick + line.depth {
                best = Some(best.map_or(t, |b: f64| if t.abs() < b.abs() { t } else { b }));
            }
        }
        out[k] = best.unwrap_or(0.0);
    }
    out
}

/// The members of a framed wall (none if its type is not framed or it is
/// curved), panel by panel.
pub fn wall_members(model: &Model, id: WallId) -> Vec<Member> {
    let Some(w) = model.wall(id) else {
        return Vec::new();
    };
    let Some(f) = model.type_framing(w.wall_type.0).cloned() else {
        return Vec::new();
    };
    let Some(line) = wall_line(model, id) else {
        return Vec::new();
    };
    let trims = wall_end_trims(model, id, &line);
    let level_z = model.level(w.level).map_or(0.0, |l| l.elevation);
    let base = level_z + w.base_offset;
    let top = base + w.height;
    let (u0, u1) = (-trims[0], line.length + trims[1]);
    if u1 - u0 < f.stud.width * 2.0 || top - base < 0.2 {
        return Vec::new();
    }
    // The holes: along the frame line, and their bottom and top.
    let holes: Vec<(f64, f64, f64, f64)> = model
        .openings_in_wall(id)
        .filter_map(|o| {
            let t = model.opening_type(o.opening_type)?;
            let (a, b) = o.span(t.width);
            let zb = level_z + o.sill;
            Some((a, b, zb.max(base), (zb + t.height).min(top)))
        })
        .filter(|h| h.1 > u0 && h.0 < u1 && h.3 > h.2)
        .collect();
    let sw = f.stud.width;
    let steel = f.material == FrameMaterial::Steel;
    // How high a track or plate lies.
    let lay = |s: &FrameSection| if s.is_steel() { s.thickness } else { s.width };
    let plates = if steel { 1 } else { 2 };
    let stud_lo = base + lay(&f.track);
    let stud_hi = top - lay(&f.track) * plates as f64;
    let in_hole = |u: f64| {
        holes
            .iter()
            .any(|h| u > h.0 - sw * 2.5 && u < h.1 + sw * 2.5)
    };
    // The regular studs, from the start at the spacing.
    let mut regular = Vec::new();
    let mut u = u0 + sw / 2.0;
    while u < u1 - sw / 2.0 - 0.05 {
        regular.push(u);
        u = if regular.len() == 1 {
            (u0 / f.spacing).floor() * f.spacing + f.spacing
        } else {
            u + f.spacing
        };
        if regular.len() == 1 && u - regular[0] < sw * 1.5 {
            u += f.spacing;
        }
    }
    // The panels: as many as the lorry needs, as even as the studs let
    // them be, cut at a regular stud clear of the holes.
    let mut cuts = vec![u0];
    loop {
        let from = *cuts.last().unwrap();
        let left = u1 - from;
        if left <= PANEL_MAX {
            break;
        }
        let parts = (left / PANEL_MAX).ceil();
        let aim = from + left / parts;
        let cut = regular
            .iter()
            .copied()
            .filter(|&s| s > from + 0.6 && s <= from + PANEL_MAX && u1 - s > 0.6 && !in_hole(s))
            .min_by(|a, b| (a - aim).abs().total_cmp(&(b - aim).abs()));
        match cut {
            Some(c) => cuts.push(c),
            None => break,
        }
    }
    cuts.push(u1);
    let owner = SolidOwner::Wall(id);
    let at = |u: f64, z: f64| p3(line.a + line.u * u + line.n * line.centre, z);
    let n3 = v3(line.n);
    let u3 = v3(line.u);
    let n3_off = |o: f64| mul(n3, o);
    let mut out = Vec::new();
    let mut push = |role: Role, a: P3, b: P3, across: P3, open: P3, panel: usize| {
        let section = if role.of_track() {
            f.track.clone()
        } else {
            f.stud.clone()
        };
        out.push(Member {
            owner,
            role,
            material: f.material,
            section,
            start: a,
            end: b,
            across,
            open,
            panel: Some(panel),
        });
    };
    let tw = lay(&f.track);
    for (pi, pair) in cuts.windows(2).enumerate() {
        let (pa, pb) = (pair[0], pair[1]);
        let mid = |z0: f64, z1: f64| (z0 + z1) / 2.0;
        // Bottom and top tracks.
        push(
            Role::BottomTrack,
            at(pa, mid(base, base + tw)),
            at(pb, mid(base, base + tw)),
            n3,
            UP,
            pi,
        );
        push(
            Role::TopTrack,
            at(
                pa,
                mid(
                    stud_hi + if steel { 0.0 } else { tw },
                    top - if steel { 0.0 } else { tw },
                )
                .max(stud_hi),
            ),
            at(
                pb,
                mid(
                    stud_hi + if steel { 0.0 } else { tw },
                    top - if steel { 0.0 } else { tw },
                )
                .max(stud_hi),
            ),
            n3,
            mul(UP, -1.0),
            pi,
        );
        // The studs of this panel: both ends (double at the ends of the
        // wall) and the regular ones in between, clear of the holes.
        let mut studs: Vec<(f64, f64, f64, Role)> = vec![
            (pa + sw / 2.0, stud_lo, stud_hi, Role::Stud),
            (pb - sw / 2.0, stud_lo, stud_hi, Role::Stud),
        ];
        if pi == 0 {
            studs.push((pa + sw * 1.5, stud_lo, stud_hi, Role::Stud));
        }
        if pi + 2 == cuts.len() {
            studs.push((pb - sw * 1.5, stud_lo, stud_hi, Role::Stud));
        }
        for &s in &regular {
            if s > pa + sw * 1.4 && s < pb - sw * 1.4 && !in_hole(s) {
                studs.push((s, stud_lo, stud_hi, Role::Stud));
            }
        }
        // Round each hole in this panel.
        let mut header_spans: Vec<(f64, f64, f64, f64)> = Vec::new();
        for &(a, b, zb, zt) in &holes {
            let (a, b) = (a.max(pa + sw * 2.0), b.min(pb - sw * 2.0));
            if b <= a {
                continue;
            }
            for (side, k) in [(a - sw / 2.0, -1.0), (b + sw / 2.0, 1.0)] {
                studs.push((side, stud_lo, zt.min(stud_hi), Role::JackStud));
                studs.push((side + k * sw, stud_lo, stud_hi, Role::KingStud));
            }
            let hd = f.stud.depth;
            header_spans.push((a - sw, b + sw, zb, zt + hd));
            // The header: two members on edge, one at each face of the
            // frame (a box), over the jack studs.
            let side = (line.depth - f.stud.width) / 2.0;
            for k in [-1.0, 1.0] {
                let o = n3_off(k * side);
                push(
                    Role::Header,
                    add(at(a - sw, zt + hd / 2.0), o),
                    add(at(b + sw, zt + hd / 2.0), o),
                    UP,
                    mul(n3, -k),
                    pi,
                );
            }
            let sill = zb > stud_lo + tw * 2.0;
            if sill {
                push(
                    Role::SillTrack,
                    at(a, zb - tw / 2.0),
                    at(b, zb - tw / 2.0),
                    n3,
                    UP,
                    pi,
                );
            }
            // Short studs at the spacing over the header and under the sill.
            for &s in &regular {
                if s > a + sw && s < b - sw {
                    if zt + hd < stud_hi - 0.05 {
                        studs.push((s, zt + hd, stud_hi, Role::Cripple));
                    }
                    if sill && zb - tw > stud_lo + 0.05 {
                        studs.push((s, stud_lo, zb - tw, Role::Cripple));
                    }
                }
            }
        }
        studs.sort_by(|x, y| x.0.total_cmp(&y.0));
        for &(s, z0, z1, role) in &studs {
            if z1 - z0 > 0.02 {
                push(role, at(s, z0), at(s, z1), n3, u3, pi);
            }
        }
        // A row of noggins at mid-height between the full-height studs.
        if f.noggins {
            let zm = (stud_lo + stud_hi) / 2.0;
            let full: Vec<f64> = studs
                .iter()
                .filter(|s| (s.1 - stud_lo).abs() < 1e-6 && (s.2 - stud_hi).abs() < 1e-6)
                .map(|s| s.0)
                .collect();
            for pair in full.windows(2) {
                let (s0, s1) = (pair[0] + sw / 2.0, pair[1] - sw / 2.0);
                if s1 - s0 < 0.05 {
                    continue;
                }
                let blocked = header_spans
                    .iter()
                    .any(|h| s1 > h.0 && s0 < h.1 && zm > h.2 - tw && zm < h.3 + tw);
                if !blocked {
                    push(Role::Noggin, at(s0, zm), at(s1, zm), n3, UP, pi);
                }
            }
        }
    }
    // The second top plate of timber, over the whole wall.
    if !steel {
        push(
            Role::TopPlate,
            at(u0, top - tw / 2.0),
            at(u1, top - tw / 2.0),
            n3,
            mul(UP, -1.0),
            cuts.len() - 2,
        );
    }
    out
}

/// The members of a deck (a floor or a roof plane): members along `dir` at
/// the spacing, cut by the outline and the holes, a rim round every edge,
/// and (`blocking`) a row at the middle. `z` gives the middle of the frame
/// at a point.
#[allow(clippy::too_many_arguments)]
fn deck_members(
    owner: SolidOwner,
    f: &Framing,
    outline: &[Point2],
    holes: &[Vec<Point2>],
    dir: Vec2,
    z: &dyn Fn(Point2) -> f64,
    main: Role,
    rim: Role,
) -> Vec<Member> {
    let mut out = Vec::new();
    if outline.len() < 3 {
        return out;
    }
    let across_dir = dir.perp_left();
    let rim_w = f.track.width;
    let half = f.stud.width / 2.0;
    let mk = |role: Role, a: Point2, b: Point2, open: Vec2| {
        let section = if role.of_track() {
            f.track.clone()
        } else {
            f.stud.clone()
        };
        let (pa, pb) = (p3(a, z(a)), p3(b, z(b)));
        let axis = sub(pb, pa);
        let l = len(axis).max(1e-9);
        let ax = mul(axis, 1.0 / l);
        // The web square to the member, as near to plumb as it goes.
        let o3 = v3(open);
        let mut up = cross(o3, ax);
        if up[2] < 0.0 {
            up = mul(up, -1.0);
        }
        let ul = len(up).max(1e-9);
        Member {
            owner,
            role,
            material: f.material,
            section,
            start: pa,
            end: pb,
            across: mul(up, 1.0 / ul),
            open: o3,
            panel: None,
        }
    };
    // The rims, set in by half their width.
    let mut rings: Vec<&[Point2]> = vec![outline];
    rings.extend(holes.iter().map(|h| h.as_slice()));
    let ccw = archisim_core::signed_area(outline) > 0.0;
    for (ri, ring) in rings.iter().enumerate() {
        let n = ring.len();
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            let Some(u) = (b - a).normalized() else {
                continue;
            };
            // Inward: left of a counter-clockwise outline; holes go the
            // other way round for the same rule, so look outward of them.
            let mut inward = u.perp_left();
            if !ccw {
                inward = -inward;
            }
            if ri > 0 {
                let hole_ccw = archisim_core::signed_area(ring) > 0.0;
                if hole_ccw == ccw {
                    inward = -inward;
                }
            }
            let off = inward * (rim_w / 2.0);
            out.push(mk(rim, a + off, b + off, inward));
        }
    }
    // The main members: lines along `dir` across the outline.
    let all: Vec<Point2> = rings.iter().flat_map(|r| r.iter().copied()).collect();
    let (lo, hi) = all
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
            let t = p.dot(across_dir);
            (lo.min(t), hi.max(t))
        });
    let origin = Point2::new(0.0, 0.0);
    let mut lines: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut stations = Vec::new();
    let mut t = lo + rim_w + half;
    while t < hi - rim_w - half + 1e-9 {
        stations.push(t);
        t += f.spacing;
    }
    if stations
        .last()
        .is_none_or(|&l| hi - rim_w - half - l > f.spacing * 0.3)
    {
        stations.push(hi - rim_w - half);
    }
    for &t in &stations {
        let base = origin + across_dir * t;
        // Where the line crosses the rings.
        let mut cuts: Vec<f64> = Vec::new();
        for ring in &rings {
            let n = ring.len();
            for i in 0..n {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                let (ta, tb) = (a.dot(across_dir) - t, b.dot(across_dir) - t);
                if (ta > 0.0) != (tb > 0.0) {
                    let k = ta / (ta - tb);
                    let p = a + (b - a) * k;
                    cuts.push((p - base).dot(dir));
                }
            }
        }
        cuts.sort_by(|a, b| a.total_cmp(b));
        let mut segs = Vec::new();
        for pair in cuts.chunks(2) {
            if let [s0, s1] = *pair {
                // Between the rims, square to the line where the edge is
                // not: a little short of them.
                let (s0, s1) = (s0 + rim_w * 1.05, s1 - rim_w * 1.05);
                if s1 - s0 > 0.1 {
                    out.push(mk(main, base + dir * s0, base + dir * s1, dir.perp_left()));
                    segs.push((s0, s1));
                }
            }
        }
        lines.push(segs);
    }
    // A row of blocking across the middle.
    if f.noggins && lines.len() > 1 {
        let (mut smin, mut smax) = (f64::INFINITY, f64::NEG_INFINITY);
        for segs in &lines {
            for &(a, b) in segs {
                smin = smin.min(a);
                smax = smax.max(b);
            }
        }
        let sm = (smin + smax) / 2.0;
        let has = |segs: &Vec<(f64, f64)>| segs.iter().any(|&(a, b)| sm > a + 0.2 && sm < b - 0.2);
        for i in 0..lines.len() - 1 {
            if has(&lines[i]) && has(&lines[i + 1]) {
                let a = origin + across_dir * (stations[i] + half) + dir * sm;
                let b = origin + across_dir * (stations[i + 1] - half) + dir * sm;
                if (b - a).length() > 0.05 {
                    out.push(mk(Role::Blocking, a, b, dir));
                }
            }
        }
    }
    out
}

/// The members of a framed floor: joists square to its longest side.
pub fn slab_members(model: &Model, id: archisim_core::SlabId) -> Vec<Member> {
    let Some(s) = model.slab(id) else {
        return Vec::new();
    };
    let Some(f) = model.type_framing(s.slab_type.0) else {
        return Vec::new();
    };
    let Some(t) = model.slab_type(s.slab_type) else {
        return Vec::new();
    };
    let Some((mid, _)) = frame_band(&t.layers) else {
        return Vec::new();
    };
    let n = s.outline.len();
    let longest = (0..n)
        .map(|i| s.outline[(i + 1) % n] - s.outline[i])
        .max_by(|a, b| a.length().total_cmp(&b.length()));
    let Some(dir) = longest.and_then(|e| e.normalized()).map(|e| e.perp_left()) else {
        return Vec::new();
    };
    let top = model.level(s.level).map_or(0.0, |l| l.elevation) + s.top_offset;
    let slope = s.slope;
    let z = move |p: Point2| match slope {
        Some(sl) => top + sl.rise_at(p) - sl.vertical(mid),
        None => top - mid,
    };
    deck_members(
        SolidOwner::Slab(id),
        f,
        &s.outline,
        &s.holes,
        dir,
        &z,
        Role::Joist,
        Role::Rim,
    )
}

/// The members of a framed roof: rafters down each plane.
pub fn roof_members(model: &Model, id: archisim_core::RoofId) -> Vec<Member> {
    let Some(r) = model.roof(id) else {
        return Vec::new();
    };
    let Some(f) = model.type_framing(r.roof_type.0) else {
        return Vec::new();
    };
    let Some(t) = model.slab_type(r.roof_type) else {
        return Vec::new();
    };
    let Some((mid, _)) = frame_band(&t.layers) else {
        return Vec::new();
    };
    let base = model.level(r.level).map_or(0.0, |l| l.elevation);
    let mut out = Vec::new();
    for plane in r.planes() {
        let Some(dir) = plane.slope.direction() else {
            continue;
        };
        let sl = plane.slope;
        let z = |p: Point2| base + r.base_offset + sl.rise_at(p) - sl.vertical(mid);
        out.extend(deck_members(
            SolidOwner::Roof(id),
            f,
            &plane.outline,
            &[],
            dir,
            &z,
            Role::Rafter,
            Role::RoofRim,
        ));
    }
    out
}

/// Every member of the project, walls first.
pub fn all_members(model: &Model) -> Vec<Member> {
    if model.framings().next().is_none() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for w in model.walls() {
        out.extend(wall_members(model, w.id));
    }
    for s in model.slabs() {
        out.extend(slab_members(model, s.id));
    }
    for r in model.roofs() {
        out.extend(roof_members(model, r.id));
    }
    out
}

/// Whether an element is drawn by its frame when the frames are shown.
pub fn is_framed(model: &Model, owner: SolidOwner) -> bool {
    match owner {
        SolidOwner::Wall(id) | SolidOwner::Facade(id) => model
            .wall(id)
            .is_some_and(|w| w.bulge.abs() < 1e-9 && model.type_framing(w.wall_type.0).is_some()),
        SolidOwner::Slab(id) => model
            .slab(id)
            .is_some_and(|s| model.type_framing(s.slab_type.0).is_some()),
        SolidOwner::Roof(id) | SolidOwner::Dormer(id, _) => model
            .roof(id)
            .is_some_and(|r| model.type_framing(r.roof_type.0).is_some()),
        _ => false,
    }
}

/// The solid of one member: a C or U of thin plates in steel, a rectangle
/// in timber.
pub fn member_triangles(m: &Member) -> Vec<[P3; 3]> {
    let axis = sub(m.end, m.start);
    let s = &m.section;
    let (d, w) = (s.depth / 2.0, s.width / 2.0);
    let rects: Vec<(f64, f64, f64, f64)> = if s.is_steel() {
        let t = s.thickness.max(0.0005);
        let mut r = vec![(-d, d, -w, -w + t), (-d, -d + t, -w, w), (d - t, d, -w, w)];
        if s.lip > 0.0 {
            r.push((-d + t, -d + s.lip, w - t, w));
            r.push((d - s.lip, d - t, w - t, w));
        }
        r
    } else {
        vec![(-d, d, -w, w)]
    };
    let mut out = Vec::new();
    for (d0, d1, w0, w1) in rects {
        let o = add(add(m.start, mul(m.across, d0)), mul(m.open, w0));
        cuboid(
            &mut out,
            o,
            mul(m.across, d1 - d0),
            mul(m.open, w1 - w0),
            axis,
        );
    }
    out
}

/// The twelve triangles of the box `o + a·ex + b·ey + c·ez`, facing out.
fn cuboid(out: &mut Vec<[P3; 3]>, o: P3, ex: P3, ey: P3, ez: P3) {
    let (ex, ey) = if dot(cross(ex, ey), ez) < 0.0 {
        (ey, ex)
    } else {
        (ex, ey)
    };
    let c = |i: u8| {
        let mut p = o;
        if i & 1 != 0 {
            p = add(p, ex);
        }
        if i & 2 != 0 {
            p = add(p, ey);
        }
        if i & 4 != 0 {
            p = add(p, ez);
        }
        p
    };
    // Each face as a quad, counter-clockwise from outside.
    for q in [
        [0, 2, 3, 1],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 4, 6, 2],
        [1, 3, 7, 5],
    ] {
        out.push([c(q[0]), c(q[1]), c(q[2])]);
        out.push([c(q[0]), c(q[2]), c(q[3])]);
    }
}

/// The solids of the frames of the project: one per element and material.
pub fn frame_solids(model: &Model) -> Vec<Solid> {
    let mut by: std::collections::BTreeMap<(SolidOwner, archisim_core::MaterialId), Vec<[P3; 3]>> =
        Default::default();
    for m in all_members(model) {
        let Some(mat) = model
            .frame_member_material(m.material)
            .or_else(|| model.materials().next().map(|x| x.id))
        else {
            continue;
        };
        by.entry((m.owner, mat))
            .or_default()
            .extend(member_triangles(&m));
    }
    by.into_iter()
        .map(|((owner, material), triangles)| Solid {
            owner,
            material,
            triangles,
        })
        .collect()
}

/// A line of the cut list: so many pieces of one section and length.
#[derive(Debug, Clone, PartialEq)]
pub struct CutLine {
    pub material: FrameMaterial,
    pub section: String,
    pub role: Role,
    /// Millimetres.
    pub length_mm: i64,
    pub count: usize,
    pub kg_per_m: f64,
    pub area: f64,
}

impl CutLine {
    pub fn metres(&self) -> f64 {
        self.length_mm as f64 / 1000.0 * self.count as f64
    }
    pub fn kg(&self) -> f64 {
        self.metres() * self.kg_per_m
    }
    pub fn m3(&self) -> f64 {
        if self.material == FrameMaterial::Timber {
            self.metres() * self.area
        } else {
            0.0
        }
    }
}

/// The total of one section.
#[derive(Debug, Clone, PartialEq)]
pub struct SectionTotal {
    pub material: FrameMaterial,
    pub section: String,
    pub pieces: usize,
    pub metres: f64,
    pub kg: f64,
    pub m3: f64,
    /// Bars of the commercial length ([`BAR_STEEL`], [`BAR_TIMBER`]) to buy.
    pub bars: usize,
}

/// The commercial length of a steel profile, m.
pub const BAR_STEEL: f64 = 6.0;
/// The commercial length of a timber piece, m.
pub const BAR_TIMBER: f64 = 5.4;
/// What is lost cutting boards.
pub const BOARD_WASTE: f64 = 0.10;

/// The boards of one kind.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardTotal {
    pub name: String,
    pub area: f64,
    pub boards: usize,
    pub board: (f64, f64),
}

/// The breakdown of the frames of the project.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CutList {
    pub lines: Vec<CutLine>,
    pub totals: Vec<SectionTotal>,
    pub boards: Vec<BoardTotal>,
    /// Screws (steel) and nails (timber), estimated.
    pub screws: usize,
    pub nails: usize,
    /// Hold-downs and anchors of the walls to what carries them.
    pub anchors: usize,
}

/// The cut list of `members`, with the boards of the framed elements.
pub fn cut_list(model: &Model, members: &[Member]) -> CutList {
    let mut lines: std::collections::BTreeMap<(String, Role, i64), CutLine> = Default::default();
    let mut screws = 0usize;
    let mut nails = 0usize;
    for m in members {
        let mm = (m.length() * 1000.0).round() as i64;
        if mm <= 0 {
            continue;
        }
        let e = lines
            .entry((m.section.name.clone(), m.role, mm))
            .or_insert_with(|| CutLine {
                material: m.material,
                section: m.section.name.clone(),
                role: m.role,
                length_mm: mm,
                count: 0,
                kg_per_m: m.section.kg_per_m(),
                area: m.section.area(),
            });
        e.count += 1;
        match m.material {
            FrameMaterial::Steel => screws += 4,
            FrameMaterial::Timber => nails += 4,
        }
    }
    let mut lines: Vec<CutLine> = lines.into_values().collect();
    lines.sort_by(|a, b| {
        (a.section.as_str(), a.role, -a.length_mm).cmp(&(b.section.as_str(), b.role, -b.length_mm))
    });
    let mut totals: Vec<SectionTotal> = Vec::new();
    for l in &lines {
        let t = match totals.iter_mut().find(|t| t.section == l.section) {
            Some(t) => t,
            None => {
                totals.push(SectionTotal {
                    material: l.material,
                    section: l.section.clone(),
                    pieces: 0,
                    metres: 0.0,
                    kg: 0.0,
                    m3: 0.0,
                    bars: 0,
                });
                totals.last_mut().unwrap()
            }
        };
        t.pieces += l.count;
        t.metres += l.metres();
        t.kg += l.kg();
        t.m3 += l.m3();
    }
    // Bars to buy: the pieces packed into commercial lengths, longest first.
    for t in &mut totals {
        let bar = if t.material == FrameMaterial::Steel {
            BAR_STEEL
        } else {
            BAR_TIMBER
        };
        let mut pieces: Vec<f64> = lines
            .iter()
            .filter(|l| l.section == t.section)
            .flat_map(|l| std::iter::repeat_n(l.length_mm as f64 / 1000.0, l.count))
            .collect();
        pieces.sort_by(|a, b| b.total_cmp(a));
        let mut open: Vec<f64> = Vec::new();
        for p in pieces {
            if p > bar {
                // Longer than a bar: joined from whole bars.
                t.bars += (p / bar).ceil() as usize;
                continue;
            }
            match open.iter_mut().find(|r| **r >= p + 0.003) {
                Some(r) => *r -= p + 0.003,
                None => open.push(bar - p - 0.003),
            }
        }
        t.bars += open.len();
    }
    // Boards and their fixings, and the anchors of the walls.
    let mut boards: Vec<BoardTotal> = Vec::new();
    let mut anchors = 0usize;
    for (area, sheathing, material, anchor) in sheathed_areas(model) {
        if let Some(s) = sheathing {
            match boards.iter_mut().find(|b| b.name == s.name) {
                Some(b) => b.area += area,
                None => boards.push(BoardTotal {
                    name: s.name.clone(),
                    area,
                    boards: 0,
                    board: s.board,
                }),
            }
            match material {
                FrameMaterial::Steel => screws += (area * 20.0).ceil() as usize,
                FrameMaterial::Timber => nails += (area * 30.0).ceil() as usize,
            }
        }
        anchors += anchor;
    }
    for b in &mut boards {
        let one = b.board.0 * b.board.1;
        b.boards = (b.area * (1.0 + BOARD_WASTE) / one).ceil() as usize;
    }
    CutList {
        lines,
        totals,
        boards,
        screws,
        nails,
        anchors,
    }
}

/// The sheathed area of each framed element (one face), its boards, its
/// material and the anchors it takes (walls: one every 1,2 m and at each
/// end).
fn sheathed_areas(
    model: &Model,
) -> Vec<(f64, Option<archisim_core::Sheathing>, FrameMaterial, usize)> {
    let mut out = Vec::new();
    for w in model.walls() {
        let Some(f) = model.type_framing(w.wall_type.0) else {
            continue;
        };
        if w.bulge.abs() > 1e-9 {
            continue;
        }
        let l = w.length();
        let holes: f64 = model
            .openings_in_wall(w.id)
            .filter_map(|o| model.opening_type(o.opening_type))
            .map(|t| t.width * t.height)
            .sum();
        let anchors = (l / 1.2).ceil() as usize + 1;
        out.push((
            (l * w.height - holes).max(0.0),
            f.sheathing.clone(),
            f.material,
            anchors,
        ));
    }
    for s in model.slabs() {
        let Some(f) = model.type_framing(s.slab_type.0) else {
            continue;
        };
        let k = s.slope.map_or(1.0, |sl| (1.0 + sl.slope * sl.slope).sqrt());
        let area = archisim_core::signed_area(&s.outline).abs()
            - s.holes
                .iter()
                .map(|h| archisim_core::signed_area(h).abs())
                .sum::<f64>();
        out.push((area.max(0.0) * k, f.sheathing.clone(), f.material, 0));
    }
    for r in model.roofs() {
        let Some(f) = model.type_framing(r.roof_type.0) else {
            continue;
        };
        let area: f64 = r
            .planes()
            .iter()
            .map(|p| {
                archisim_core::signed_area(&p.outline).abs()
                    * (1.0 + p.slope.slope * p.slope.slope).sqrt()
            })
            .sum();
        out.push((area, f.sheathing.clone(), f.material, 0));
    }
    out
}

/// A prefabricated wall panel.
#[derive(Debug, Clone, PartialEq)]
pub struct Panel {
    /// «M3-2»: the third framed wall, its second panel.
    pub mark: String,
    pub wall: WallId,
    pub index: usize,
    /// From and to along the wall's frame line, from its start, m.
    pub from: f64,
    pub to: f64,
    pub bottom: f64,
    pub top: f64,
    pub members: Vec<Member>,
}

impl Panel {
    pub fn length(&self) -> f64 {
        self.to - self.from
    }
    pub fn height(&self) -> f64 {
        self.top - self.bottom
    }
    /// The frame's weight with its boards (on one face), kg.
    pub fn weight(&self, model: &Model) -> f64 {
        let frame: f64 = self
            .members
            .iter()
            .map(|m| {
                if m.section.is_steel() {
                    m.kg()
                } else {
                    m.m3() * FrameMaterial::Timber.density()
                }
            })
            .sum();
        let board = model
            .wall(self.wall)
            .and_then(|w| model.type_framing(w.wall_type.0))
            .and_then(|f| f.sheathing.as_ref())
            .map_or(0.0, |s| s.thickness * 600.0 * self.length() * self.height());
        frame + board
    }
}

/// The prefabricated panels of the framed walls, in the order of the walls.
pub fn wall_panels(model: &Model) -> Vec<Panel> {
    let mut out = Vec::new();
    let mut n = 0;
    for w in model.walls() {
        let members = wall_members(model, w.id);
        if members.is_empty() {
            continue;
        }
        n += 1;
        let Some(line) = wall_line(model, w.id) else {
            continue;
        };
        let along = |p: P3| (Point2::new(p[0], p[1]) - line.a).dot(line.u);
        let count = members.iter().filter_map(|m| m.panel).max().unwrap_or(0) + 1;
        for i in 0..count {
            let mine: Vec<Member> = members
                .iter()
                .filter(|m| m.panel == Some(i) && m.role != Role::TopPlate)
                .cloned()
                .collect();
            let Some(bottom) = mine.iter().find(|m| m.role == Role::BottomTrack) else {
                continue;
            };
            let (from, to) = (along(bottom.start), along(bottom.end));
            let base = model.level(w.level).map_or(0.0, |l| l.elevation) + w.base_offset;
            out.push(Panel {
                mark: format!("M{n}-{}", i + 1),
                wall: w.id,
                index: i,
                from: from.min(to),
                to: from.max(to),
                bottom: base,
                top: base + w.height,
                members: mine,
            });
        }
    }
    out
}

/// A member of a panel in the panel's own elevation: from its left end and
/// its bottom, m, as seen from the exterior face.
pub fn panel_elevation(model: &Model, p: &Panel) -> Vec<(Role, [f64; 2], [f64; 2], f64)> {
    let Some(line) = wall_line(model, p.wall) else {
        return Vec::new();
    };
    let at = |q: P3| {
        // Seen from outside (the left of the wall), its start is on the
        // right.
        let u = p.to - (Point2::new(q[0], q[1]) - line.a).dot(line.u);
        [u, q[2] - p.bottom]
    };
    p.members
        .iter()
        .map(|m| {
            // How wide it is drawn: the flange of the studs, how high the
            // tracks lie.
            let w = if m.role == Role::Header {
                m.section.depth
            } else if m.role.of_track() {
                if m.section.is_steel() {
                    m.section.width.min(0.04)
                } else {
                    m.section.width
                }
            } else {
                m.section.width
            };
            (m.role, at(m.start), at(m.end), w)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::*;

    fn framed_house(material: FrameMaterial) -> Model {
        let mut m = Model::new("Entramado");
        let level = m.add_level("Planta baja", 0.0);
        let mat = m.add_material(
            "Lana mineral",
            "insulation",
            [230, 220, 120],
            CutPattern::Solid,
        );
        let board = m.add_material(
            "Placa de yeso",
            "gypsum",
            [240, 240, 240],
            CutPattern::Solid,
        );
        let wt = m
            .add_wall_type(
                "Entramado 150",
                WallFunction::Exterior,
                vec![
                    Layer {
                        material: board,
                        thickness: 0.015,
                        function: LayerFunction::Finish,
                    },
                    Layer {
                        material: mat,
                        thickness: 0.15,
                        function: LayerFunction::Structure,
                    },
                    Layer {
                        material: board,
                        thickness: 0.015,
                        function: LayerFunction::Finish,
                    },
                ],
            )
            .unwrap();
        m.set_type_framing(wt.0, Some(Framing::for_depth(material, 0.15)))
            .unwrap();
        let pts = [
            Point2::new(0.0, 0.0),
            Point2::new(9.0, 0.0),
            Point2::new(9.0, 6.0),
            Point2::new(0.0, 6.0),
        ];
        let mut first = None;
        for i in 0..4 {
            let id = m
                .add_wall(NewWall {
                    wall_type: wt,
                    level,
                    start: pts[i],
                    end: pts[(i + 1) % 4],
                    bulge: 0.0,
                    reference: ReferenceLine::Center,
                    height: 2.7,
                    base_offset: 0.0,
                    top: WallTop::Unconnected,
                })
                .unwrap();
            first.get_or_insert(id);
        }
        let ot = m
            .add_opening_type(NewOpeningType {
                name: "Puerta".into(),
                kind: OpeningKind::Door(DoorOperation::SingleSwing),
                width: 0.9,
                height: 2.1,
                frame_width: 0.05,
                frame_depth: 0.07,
                frame_material: board,
                panel_material: board,
            })
            .unwrap();
        m.add_opening(NewOpening {
            opening_type: ot,
            wall: first.unwrap(),
            along: 3.0,
            sill: 0.0,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        })
        .unwrap();
        let st = m
            .add_slab_type(
                "Forjado de entramado",
                SlabFunction::Floor,
                vec![Layer {
                    material: mat,
                    thickness: 0.22,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        m.set_type_framing(st.0, Some(Framing::for_depth(material, 0.22)))
            .unwrap();
        m.add_slab(NewSlab {
            slab_type: st,
            level,
            outline: pts.to_vec(),
            top_offset: 2.92,
        })
        .unwrap();
        m
    }

    #[test]
    fn walls_and_floors_are_framed_and_counted() {
        for material in FrameMaterial::ALL {
            let m = framed_house(material);
            let members = all_members(&m);
            let studs = members.iter().filter(|x| x.role == Role::Stud).count();
            // 9 m and 6 m walls at 600: some sixty studs.
            assert!(studs > 40 && studs < 90, "{material:?}: {studs} studs");
            assert!(members.iter().any(|x| x.role == Role::Joist));
            assert!(members.iter().any(|x| x.role == Role::Rim));
            // The 9 m walls go in two panels.
            let panels = wall_panels(&m);
            assert!(panels.len() >= 6, "{} panels", panels.len());
            assert!(panels.iter().all(|p| p.length() <= PANEL_MAX + 1e-6));
            // Every stud stands between the tracks.
            for s in members.iter().filter(|x| x.role == Role::Stud) {
                assert!(s.start[2] > 0.0 && s.end[2] < 2.7, "{s:?}");
            }
            let list = cut_list(&m, &members);
            assert!(!list.lines.is_empty() && !list.totals.is_empty());
            let total: usize = list.totals.iter().map(|t| t.pieces).sum();
            assert_eq!(total, members.len());
            assert!(list.boards.iter().any(|b| b.boards > 10));
            // Solids close: each member a positive volume.
            for x in members.iter().take(30) {
                let s = Solid {
                    owner: x.owner,
                    material: MaterialId(0),
                    triangles: member_triangles(x),
                };
                assert!(s.volume() > 0.0, "{x:?}");
            }
        }
    }

    #[test]
    fn doors_get_kings_jacks_and_a_header() {
        let m = framed_house(FrameMaterial::Steel);
        let w = m.walls().next().unwrap().id;
        let ms = wall_members(&m, w);
        assert_eq!(ms.iter().filter(|x| x.role == Role::KingStud).count(), 2);
        assert_eq!(ms.iter().filter(|x| x.role == Role::JackStud).count(), 2);
        assert_eq!(ms.iter().filter(|x| x.role == Role::Header).count(), 2);
        // No regular stud inside the door.
        let o = m.openings_in_wall(w).next().unwrap();
        let t = m.opening_type(o.opening_type).unwrap();
        let (a, b) = o.span(t.width);
        for s in ms.iter().filter(|x| x.role == Role::Stud) {
            assert!(s.start[0] < a || s.start[0] > b, "{s:?}");
        }
    }
}
