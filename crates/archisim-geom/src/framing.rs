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
/// The widest floor or roof cassette that travels without escort, metres.
pub const CASSETTE_MAX: f64 = 2.4;

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
    /// The tie across the foot of a pair of rafters (with them, a truss).
    Tie,
}

impl Role {
    pub const ALL: [Role; 16] = [
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
        Role::Tie,
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
            Role::Tie => "Tirante (cercha)",
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
    /// The prefabricated panel it belongs to (walls, floor and roof
    /// cassettes), from 0.
    pub panel: Option<usize>,
    /// Where it runs along its wall's drawn line, from the start (walls).
    pub along: [f64; 2],
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

/// A wall's frame line: the middle of its frame layer, straight or along
/// the arc of a curved wall, measured `u` along the drawn line from its
/// start; with its frame's depth and the wall's top.
struct WallLine {
    wall: archisim_core::Wall,
    length: f64,
    /// Offset of the frame's middle from the drawn line (left positive).
    centre: f64,
    depth: f64,
    framed: bool,
    curved: bool,
    /// The wall's own top, from project zero, and its base.
    top: f64,
    base: f64,
    /// The undersides of the roof it is trimmed under (`z = a·x + b·y + c`).
    trims: Vec<[f64; 3]>,
}

impl WallLine {
    /// The frame's middle at `u`, `off` more to the left.
    fn point(&self, u: f64, off: f64) -> Point2 {
        if self.curved {
            self.wall
                .point_at((u / self.length).clamp(-0.5, 1.5), self.centre + off)
        } else {
            let d = self.wall.end - self.wall.start;
            let t = d.normalized().unwrap_or(Vec2::new(1.0, 0.0));
            self.wall.start + t * u + t.perp_left() * (self.centre + off)
        }
    }

    /// The direction of the wall at `u`.
    fn tangent(&self, u: f64) -> Vec2 {
        let h = 0.01;
        (self.wall.point_at((u + h) / self.length, 0.0)
            - self.wall.point_at((u - h) / self.length, 0.0))
        .normalized()
        .unwrap_or(Vec2::new(1.0, 0.0))
    }

    /// Its exterior side at `u`.
    fn normal(&self, u: f64) -> Vec2 {
        self.tangent(u).perp_left()
    }

    /// The top of the wall at `u` (from project zero): its own, the
    /// upper edge of its profile, or under the roof it is trimmed under.
    fn top_at(&self, u: f64) -> f64 {
        let mut z = match &self.wall.profile {
            Some(p) => self.base + upper_edge(&p.outline, u).unwrap_or(self.top - self.base),
            None => self.top,
        };
        let p = self.point(u, 0.0);
        for t in &self.trims {
            z = z.min(t[0] * p.x + t[1] * p.y + t[2]);
        }
        z
    }

    /// How steep the top is at `u` (rise per run).
    fn top_slope(&self, u: f64) -> f64 {
        let h = 0.02;
        (self.top_at(u + h) - self.top_at(u - h)) / (2.0 * h)
    }
}

/// The highest point of a polygon over `x` (an outline in elevation).
fn upper_edge(outline: &[Point2], x: f64) -> Option<f64> {
    let n = outline.len();
    let mut best: Option<f64> = None;
    for i in 0..n {
        let (a, b) = (outline[i], outline[(i + 1) % n]);
        let (lo, hi) = (a.x.min(b.x), a.x.max(b.x));
        if x < lo - 1e-9 || x > hi + 1e-9 {
            continue;
        }
        let z = if (b.x - a.x).abs() < 1e-9 {
            a.y.max(b.y)
        } else {
            a.y + (b.y - a.y) * (x - a.x) / (b.x - a.x)
        };
        best = Some(best.map_or(z, |m: f64| m.max(z)));
    }
    best
}

fn wall_line(model: &Model, id: WallId) -> Option<WallLine> {
    let w = model.wall(id)?;
    let t = model.wall_type(w.wall_type)?;
    let (ext, _) = w.face_offsets(t.thickness());
    let (mid, layer) = frame_band(&t.layers)?;
    let framed = model.type_framing(w.wall_type.0).is_some();
    let depth = model
        .type_framing(w.wall_type.0)
        .map_or(layer, |f| f.stud.depth.max(f.track.depth));
    let length = w.length();
    if length < 1e-6 {
        return None;
    }
    let level = model.level(w.level).map_or(0.0, |l| l.elevation);
    let base = level + w.base_offset;
    let trims = model.wall_trim_planes(id);
    let top = if trims.is_empty() {
        base + w.height
    } else {
        model.wall_top_z(w)
    };
    Some(WallLine {
        wall: w.clone(),
        length,
        centre: ext - mid,
        depth,
        framed,
        curved: w.is_curved(),
        top,
        base,
        trims,
    })
}

/// How much the frame of `id` is lengthened (positive) or shortened at its
/// start and end where it meets other walls (straight ones).
fn wall_end_trims(model: &Model, id: WallId, line: &WallLine) -> [f64; 2] {
    let w = &line.wall;
    let mut out = [0.0; 2];
    for (k, u_end) in [(0usize, 0.0), (1, line.length)] {
        // Pointing out of the wall at this end.
        let t = line.tangent(u_end);
        let out_dir = if k == 0 { -t } else { t };
        let p = line.point(u_end, 0.0);
        let mut best: Option<f64> = None;
        for o in model.walls() {
            if o.id == id || o.level != w.level || !w.allow_join[k] || o.is_curved() {
                continue;
            }
            let Some(ol) = wall_line(model, o.id) else {
                continue;
            };
            let (oa, ou) = (o.start, ol.tangent(0.0));
            let on = ou.perp_left();
            let thick = model.wall_type(o.wall_type).map_or(0.3, |t| t.thickness());
            // Near the other's drawn line, within its length.
            let along = (p - oa).dot(ou);
            if along < -thick || along > ol.length + thick {
                continue;
            }
            let off = (p - oa).dot(on);
            if (off - ol.centre).abs() > thick + line.depth {
                continue;
            }
            let k_dir = out_dir.dot(on);
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

/// The section of a header over a hole `span` wide: two of the studs'
/// section on edge, deeper as the span grows (about a tenth of the span
/// in timber and a twelfth in steel), no deeper than `room`.
pub fn header_section(f: &Framing, span: f64, room: f64) -> FrameSection {
    let want = match f.material {
        FrameMaterial::Steel => span / 12.0,
        FrameMaterial::Timber => span / 10.0,
    };
    if want <= f.stud.depth {
        return f.stud.clone();
    }
    let list: Vec<FrameSection> = match f.material {
        FrameMaterial::Steel => archisim_core::steel_studs()
            .into_iter()
            .filter(|s| (s.thickness - f.stud.thickness.max(0.0015)).abs() < 1e-6)
            .collect(),
        FrameMaterial::Timber => archisim_core::timber_sections()
            .into_iter()
            .filter(|s| (s.width - f.stud.width).abs() < 1e-6)
            .collect(),
    };
    let fits = |s: &&FrameSection| s.depth <= room + 1e-6;
    list.iter()
        .filter(fits)
        .filter(|s| s.depth >= want - 1e-6)
        .min_by(|a, b| a.depth.total_cmp(&b.depth))
        .or_else(|| {
            list.iter()
                .filter(fits)
                .max_by(|a, b| a.depth.total_cmp(&b.depth))
        })
        .filter(|s| s.depth > f.stud.depth)
        .cloned()
        .unwrap_or_else(|| f.stud.clone())
}

/// The members of a framed wall (none if its type is not framed), panel
/// by panel. Straight or curved; its top follows its profile or the roof
/// it is trimmed under (gables, lean-tos).
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
    let base = line.base;
    let (u0, u1) = (-trims[0], line.length + trims[1]);
    if u1 - u0 < f.stud.width * 2.0 || line.top_at((u0 + u1) / 2.0) - base < 0.2 {
        return Vec::new();
    }
    let sw = f.stud.width;
    let steel = f.material == FrameMaterial::Steel;
    // How high a track or plate lies.
    let lay = |s: &FrameSection| if s.is_steel() { s.thickness } else { s.width };
    let tw = lay(&f.track);
    let plates = if steel { 1.0 } else { 2.0 };
    let stud_lo = base + tw;
    // Where the studs stop under the top track(s), square to a sloping top.
    let stud_hi = |u: f64| {
        let s = line.top_slope(u);
        line.top_at(u) - tw * plates * (1.0 + s * s).sqrt()
    };
    // The holes: along the frame line, and their bottom and top.
    let holes: Vec<(f64, f64, f64, f64)> = model
        .openings_in_wall(id)
        .filter_map(|o| {
            let t = model.opening_type(o.opening_type)?;
            let (a, b) = o.span(t.width);
            let zb = level_z + o.sill;
            let top = stud_hi(a).min(stud_hi(b));
            Some((a, b, zb.max(base), (zb + t.height).min(top)))
        })
        .filter(|h| h.1 > u0 && h.0 < u1 && h.3 > h.2)
        .collect();
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
    let at = |u: f64, z: f64| p3(line.point(u, 0.0), z);
    let at_off = |u: f64, z: f64, o: f64| p3(line.point(u, o), z);
    let n3 = |u: f64| v3(line.normal(u));
    let mut out = Vec::new();
    let mut push = |role: Role,
                    a: (f64, P3),
                    b: (f64, P3),
                    across: P3,
                    open: P3,
                    panel: usize,
                    section: Option<FrameSection>| {
        let section = section.unwrap_or_else(|| {
            if role.of_track() {
                f.track.clone()
            } else {
                f.stud.clone()
            }
        });
        out.push(Member {
            owner,
            role,
            material: f.material,
            section,
            start: a.1,
            end: b.1,
            across,
            open,
            panel: Some(panel),
            along: [a.0, b.0],
        });
    };
    // A track or plate from `pa` to `pb` at `z(u)`: one piece where it
    // runs straight, broken where the top changes slope and, in a curved
    // wall, at every stud (chords).
    let pieces = |pa: f64, pb: f64, follow_top: bool, stations: &[f64]| -> Vec<(f64, f64)> {
        let mut knots = vec![pa, pb];
        if line.curved {
            knots.extend(
                stations
                    .iter()
                    .copied()
                    .filter(|&s| s > pa + 0.01 && s < pb - 0.01),
            );
        }
        if follow_top {
            // Where the slope of the top changes.
            let n = ((pb - pa) / 0.05).ceil().max(1.0) as usize;
            let step = (pb - pa) / n as f64;
            let mut last = line.top_slope(pa + step / 2.0);
            for i in 1..n {
                let s = line.top_slope(pa + step * (i as f64 + 0.5));
                if (s - last).abs() > 0.01 {
                    knots.push(pa + step * i as f64);
                    last = s;
                }
            }
        }
        knots.sort_by(|a, b| a.total_cmp(b));
        knots.dedup_by(|a, b| (*a - *b).abs() < 0.005);
        // Two knots either side of a corner of the top (sampled): the
        // corner itself, where the two slopes meet.
        if follow_top {
            let mut i = 1;
            while i + 2 < knots.len() {
                let (c, d) = (knots[i], knots[i + 1]);
                if d - c < 0.12 {
                    let (sl, sr) = (line.top_slope(c - 0.05), line.top_slope(d + 0.05));
                    if (sl - sr).abs() > 1e-3 {
                        let u = (line.top_at(d) - line.top_at(c) + sl * c - sr * d) / (sl - sr);
                        knots[i] = u.clamp(c, d);
                        knots.remove(i + 1);
                        continue;
                    }
                }
                i += 1;
            }
        }
        knots.windows(2).map(|w| (w[0], w[1])).collect()
    };
    for (pi, pair) in cuts.windows(2).enumerate() {
        let (pa, pb) = (pair[0], pair[1]);
        // The studs of this panel: both ends (double at the ends of the
        // wall) and the regular ones in between, clear of the holes.
        let mut studs: Vec<(f64, f64, f64, Role)> = vec![
            (pa + sw / 2.0, stud_lo, stud_hi(pa + sw / 2.0), Role::Stud),
            (pb - sw / 2.0, stud_lo, stud_hi(pb - sw / 2.0), Role::Stud),
        ];
        if pi == 0 {
            studs.push((pa + sw * 1.5, stud_lo, stud_hi(pa + sw * 1.5), Role::Stud));
        }
        if pi + 2 == cuts.len() {
            studs.push((pb - sw * 1.5, stud_lo, stud_hi(pb - sw * 1.5), Role::Stud));
        }
        for &s in &regular {
            if s > pa + sw * 1.4 && s < pb - sw * 1.4 && !in_hole(s) {
                studs.push((s, stud_lo, stud_hi(s), Role::Stud));
            }
        }
        let stations: Vec<f64> = studs.iter().map(|s| s.0).collect();
        // Bottom and top tracks.
        for (a, b) in pieces(pa, pb, false, &stations) {
            push(
                Role::BottomTrack,
                (a, at(a, base + tw / 2.0)),
                (b, at(b, base + tw / 2.0)),
                n3((a + b) / 2.0),
                UP,
                pi,
                None,
            );
        }
        let track_z = |u: f64| {
            let s = line.top_slope(u);
            line.top_at(u) - (if steel { tw / 2.0 } else { tw * 1.5 }) * (1.0 + s * s).sqrt()
        };
        for (a, b) in pieces(pa, pb, true, &stations) {
            push(
                Role::TopTrack,
                (a, at(a, track_z(a + 0.001))),
                (b, at(b, track_z(b - 0.001))),
                n3((a + b) / 2.0),
                mul(UP, -1.0),
                pi,
                None,
            );
        }
        // Round each hole in this panel.
        let mut header_spans: Vec<(f64, f64, f64, f64)> = Vec::new();
        for &(a, b, zb, zt) in &holes {
            let (a, b) = (a.max(pa + sw * 2.0), b.min(pb - sw * 2.0));
            if b <= a {
                continue;
            }
            let room = stud_hi(a).min(stud_hi(b)) - zt;
            let hs = header_section(&f, b - a, room.max(0.0));
            let hd = hs.depth.min(room.max(0.0));
            for (side, k) in [(a - sw / 2.0, -1.0), (b + sw / 2.0, 1.0)] {
                studs.push((side, stud_lo, zt.min(stud_hi(side)), Role::JackStud));
                studs.push((
                    side + k * sw,
                    stud_lo,
                    stud_hi(side + k * sw),
                    Role::KingStud,
                ));
            }
            header_spans.push((a - sw, b + sw, zb, zt + hd));
            // The header: two members on edge, one at each face of the
            // frame (a box), over the jack studs.
            if hd > 0.02 {
                let side = (line.depth - hs.width) / 2.0;
                for k in [-1.0, 1.0] {
                    let mid = (a + b) / 2.0;
                    push(
                        Role::Header,
                        (a - sw, at_off(a - sw, zt + hd / 2.0, k * side)),
                        (b + sw, at_off(b + sw, zt + hd / 2.0, k * side)),
                        UP,
                        mul(n3(mid), -k),
                        pi,
                        Some(hs.clone()),
                    );
                }
            }
            let sill = zb > stud_lo + tw * 2.0;
            if sill {
                push(
                    Role::SillTrack,
                    (a, at(a, zb - tw / 2.0)),
                    (b, at(b, zb - tw / 2.0)),
                    n3((a + b) / 2.0),
                    UP,
                    pi,
                    None,
                );
            }
            // Short studs at the spacing over the header and under the sill.
            for &s in &regular {
                if s > a + sw && s < b - sw {
                    if zt + hd < stud_hi(s) - 0.05 {
                        studs.push((s, zt + hd, stud_hi(s), Role::Cripple));
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
                push(
                    role,
                    (s, at(s, z0)),
                    (s, at(s, z1)),
                    n3(s),
                    v3(line.tangent(s)),
                    pi,
                    None,
                );
            }
        }
        // A row of noggins at one height, between the full-height studs.
        if f.noggins {
            let low = (0..=40)
                .map(|i| stud_hi(u0 + (u1 - u0) * i as f64 / 40.0))
                .fold(f64::INFINITY, f64::min);
            let zm = (stud_lo + low.max(stud_lo + 0.4)) / 2.0;
            let full: Vec<f64> = studs
                .iter()
                .filter(|s| (s.1 - stud_lo).abs() < 1e-6 && (s.2 - stud_hi(s.0)).abs() < 1e-6)
                .map(|s| s.0)
                .collect();
            for pair in full.windows(2) {
                let (s0, s1) = (pair[0] + sw / 2.0, pair[1] - sw / 2.0);
                if s1 - s0 < 0.05 || zm > stud_hi(pair[0]).min(stud_hi(pair[1])) - 0.1 {
                    continue;
                }
                let blocked = header_spans
                    .iter()
                    .any(|h| s1 > h.0 && s0 < h.1 && zm > h.2 - tw && zm < h.3 + tw);
                if !blocked {
                    push(
                        Role::Noggin,
                        (s0, at(s0, zm)),
                        (s1, at(s1, zm)),
                        n3((s0 + s1) / 2.0),
                        UP,
                        pi,
                        None,
                    );
                }
            }
        }
    }
    // The second top plate of timber, over the whole wall.
    if !steel {
        let all: Vec<f64> = regular.clone();
        for (a, b) in pieces(u0, u1, true, &all) {
            let z = |u: f64| {
                let s = line.top_slope(u);
                line.top_at(u) - tw / 2.0 * (1.0 + s * s).sqrt()
            };
            let panel = cuts
                .windows(2)
                .position(|c| (a + b) / 2.0 >= c[0] && (a + b) / 2.0 <= c[1])
                .unwrap_or(0);
            push(
                Role::TopPlate,
                (a, at(a, z(a + 0.001))),
                (b, at(b, z(b - 0.001))),
                n3((a + b) / 2.0),
                mul(UP, -1.0),
                panel,
                None,
            );
        }
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
            along: [0.0; 2],
        }
    };
    let mut rings: Vec<&[Point2]> = vec![outline];
    rings.extend(holes.iter().map(|h| h.as_slice()));
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
    let mut stations: Vec<f64> = Vec::new();
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
    // Prefabricated cassettes no wider than CASSETTE_MAX: at each seam
    // the member is doubled, one for each cassette.
    let mut band_of: Vec<usize> = Vec::new();
    let mut seams: Vec<f64> = Vec::new();
    {
        let mut start = stations.first().copied().unwrap_or(0.0) - half - rim_w;
        let mut more = Vec::new();
        let mut prev: Option<f64> = None;
        for &t in &stations {
            if let Some(p) = prev {
                if t + half - start > CASSETTE_MAX && p - start > 0.3 {
                    // The seam just after the last member that fits, and
                    // the next cassette's first member beside it.
                    let c = p + half;
                    seams.push(c);
                    if t - (c + half) > 3.0 * half {
                        more.push((c + half, seams.len()));
                    }
                    start = c;
                }
            }
            band_of.push(seams.len());
            prev = Some(t);
        }
        for (t, b) in more {
            let i = stations.partition_point(|&x| x < t);
            stations.insert(i, t);
            band_of.insert(i, b);
        }
    }
    let band_at = |t: f64| seams.iter().filter(|&&c| t > c).count();
    for (ti, &t) in stations.iter().enumerate() {
        let band = band_of[ti];
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
                    let mut m = mk(main, base + dir * s0, base + dir * s1, dir.perp_left());
                    m.panel = Some(band);
                    out.push(m);
                    segs.push((s0, s1));
                }
            }
        }
        lines.push(segs);
    }
    // The rims, set in by half their width.
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
            let (a, b) = (a + off, b + off);
            // Cut at the seams of the cassettes.
            let (ta, tb) = (a.dot(across_dir), b.dot(across_dir));
            let mut ks = vec![0.0, 1.0];
            for &c in &seams {
                if (ta - c) * (tb - c) < 0.0 {
                    ks.push((c - ta) / (tb - ta));
                }
            }
            ks.sort_by(|x, y| x.total_cmp(y));
            for w in ks.windows(2) {
                let (p, q) = (a + (b - a) * w[0], a + (b - a) * w[1]);
                if p.distance(q) < 0.02 {
                    continue;
                }
                let mut m = mk(rim, p, q, inward);
                m.panel = Some(band_at(((p + q) * 0.5).dot(across_dir)));
                out.push(m);
            }
        }
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
                if (b - a).length() > 0.05 && band_of[i] == band_of[i + 1] {
                    let mut m = mk(Role::Blocking, a, b, dir);
                    m.panel = Some(band_of[i]);
                    out.push(m);
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
    let planes = r.planes();
    for (pi, plane) in planes.iter().enumerate() {
        let Some(dir) = plane.slope.direction() else {
            continue;
        };
        let sl = plane.slope;
        let z = |p: Point2| base + r.base_offset + sl.rise_at(p) - sl.vertical(mid);
        for mut m in deck_members(
            SolidOwner::Roof(id),
            f,
            &plane.outline,
            &[],
            dir,
            &z,
            Role::Rafter,
            Role::RoofRim,
        ) {
            // Each plane's cassettes on their own.
            m.panel = m.panel.map(|b| pi * ROOF_PLANE_PANELS + b);
            out.push(m);
        }
    }
    // A gable roof (its planes all run one way, down either side): a tie
    // at the foot of each pair of rafters, across the footprint, so that
    // each pair with its tie is a truss.
    let dirs: Vec<Vec2> = planes.iter().filter_map(|p| p.slope.direction()).collect();
    if let Some(&d0) = dirs.first() {
        if dirs.len() >= 2 && dirs.iter().all(|d| d.dot(d0).abs() > 0.999) {
            let sl = planes[0].slope;
            let across = d0.perp_left();
            let rafter_bottom = mid + f.stud.depth / 2.0;
            let z = base + r.base_offset - sl.vertical(rafter_bottom) - f.stud.depth / 2.0;
            let rafters: Vec<f64> = out
                .iter()
                .filter(|m| {
                    m.role == Role::Rafter && m.panel.is_some_and(|p| p < ROOF_PLANE_PANELS)
                })
                .map(|m| Point2::new(m.start[0], m.start[1]).dot(across))
                .collect();
            for t in rafters {
                let base_pt = across * t;
                let mut cuts: Vec<f64> = Vec::new();
                let n = r.outline.len();
                for i in 0..n {
                    let (a, b) = (r.outline[i], r.outline[(i + 1) % n]);
                    let (ta, tb) = (a.dot(across) - t, b.dot(across) - t);
                    if (ta > 0.0) != (tb > 0.0) {
                        let k = ta / (ta - tb);
                        cuts.push((a + (b - a) * k - base_pt).dot(d0));
                    }
                }
                cuts.sort_by(|a, b| a.total_cmp(b));
                if let (Some(&s0), Some(&s1)) = (cuts.first(), cuts.last()) {
                    if s1 - s0 > 0.5 {
                        let (a, b) = (base_pt + d0 * s0, base_pt + d0 * s1);
                        out.push(Member {
                            owner: SolidOwner::Roof(id),
                            role: Role::Tie,
                            material: f.material,
                            section: f.stud.clone(),
                            start: p3(a, z),
                            end: p3(b, z),
                            across: UP,
                            open: v3(across),
                            panel: None,
                            along: [0.0; 2],
                        });
                    }
                }
            }
        }
    }
    out
}

/// Cassettes of a roof are numbered plane by plane: plane `i`'s start at
/// `i · ROOF_PLANE_PANELS`.
const ROOF_PLANE_PANELS: usize = 1000;

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
            .is_some_and(|w| model.type_framing(w.wall_type.0).is_some()),
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
        let l = w.length();
        let holes: f64 = model
            .openings_in_wall(w.id)
            .filter_map(|o| model.opening_type(o.opening_type))
            .map(|t| t.width * t.height)
            .sum();
        let anchors = (l / 1.2).ceil() as usize + 1;
        // Its face: up to its top, gable or not.
        let face = match wall_line(model, w.id) {
            Some(line) => {
                let n = ((l / 0.1).ceil() as usize).max(1);
                (0..n)
                    .map(|i| {
                        let u = l * (i as f64 + 0.5) / n as f64;
                        (line.top_at(u) - line.base).max(0.0) * l / n as f64
                    })
                    .sum::<f64>()
            }
            None => l * w.height,
        };
        out.push((
            (face - holes).max(0.0),
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
        let base = model.level(w.level).map_or(0.0, |l| l.elevation) + w.base_offset;
        let count = members.iter().filter_map(|m| m.panel).max().unwrap_or(0) + 1;
        for i in 0..count {
            let mine: Vec<Member> = members
                .iter()
                .filter(|m| m.panel == Some(i) && m.role != Role::TopPlate)
                .cloned()
                .collect();
            let bottoms: Vec<&Member> = mine
                .iter()
                .filter(|m| m.role == Role::BottomTrack)
                .collect();
            if bottoms.is_empty() {
                continue;
            }
            let from = bottoms
                .iter()
                .map(|m| m.along[0].min(m.along[1]))
                .fold(f64::INFINITY, f64::min);
            let to = bottoms
                .iter()
                .map(|m| m.along[0].max(m.along[1]))
                .fold(f64::NEG_INFINITY, f64::max);
            let top = mine
                .iter()
                .map(|m| m.start[2].max(m.end[2]) + m.section.thickness.max(0.0))
                .fold(base, f64::max);
            out.push(Panel {
                mark: format!("M{n}-{}", i + 1),
                wall: w.id,
                index: i,
                from,
                to,
                bottom: base,
                top: top.max(base + 0.1),
                members: mine,
            });
        }
    }
    out
}

/// A member of a panel in the panel's own elevation: from its left end and
/// its bottom, m, as seen from the exterior face.
pub fn panel_elevation(p: &Panel) -> Vec<(Role, [f64; 2], [f64; 2], f64)> {
    // Seen from outside (the left of the wall), its start is on the right.
    let at = |u: f64, q: P3| [p.to - u, q[2] - p.bottom];
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
            (m.role, at(m.along[0], m.start), at(m.along[1], m.end), w)
        })
        .collect()
}

/// A prefabricated floor or roof cassette: a strip of joists or rafters no
/// wider than [`CASSETTE_MAX`] with its rims.
#[derive(Debug, Clone, PartialEq)]
pub struct Cassette {
    /// «F2-3»: the second framed floor, its third cassette; «C1-2» for
    /// roofs.
    pub mark: String,
    pub owner: SolidOwner,
    pub index: usize,
    /// Along the joists and across them, m.
    pub length: f64,
    pub width: f64,
    pub members: Vec<Member>,
}

impl Cassette {
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
        let ty = match self.owner {
            SolidOwner::Slab(s) => model.slab(s).map(|s| s.slab_type.0),
            SolidOwner::Roof(r) => model.roof(r).map(|r| r.roof_type.0),
            _ => None,
        };
        let board = ty
            .and_then(|t| model.type_framing(t))
            .and_then(|f| f.sheathing.as_ref())
            .map_or(0.0, |s| s.thickness * 600.0 * self.length * self.width);
        frame + board
    }

    /// Along its joists and across them, m.
    pub fn size(&self) -> (f64, f64) {
        let mut size = (0.0f64, 0.0f64);
        for (_, a, b, w) in self.plan() {
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let l = (dx * dx + dy * dy).sqrt().max(1e-9);
            let (nx, ny) = (-dy / l * w / 2.0, dx / l * w / 2.0);
            for q in [a, b] {
                for sgn in [-1.0, 1.0] {
                    size.0 = size.0.max(q[0] + nx * sgn);
                    size.1 = size.1.max(q[1] + ny * sgn);
                }
            }
        }
        size
    }

    /// Its members seen from above in its own axes: along the joists and
    /// across them, from its corner, m; with how wide each is drawn.
    pub fn plan(&self) -> Vec<(Role, [f64; 2], [f64; 2], f64)> {
        let Some(main) = self
            .members
            .iter()
            .find(|m| matches!(m.role, Role::Joist | Role::Rafter))
        else {
            return Vec::new();
        };
        let ex = {
            let a = sub(main.end, main.start);
            mul(a, 1.0 / len(a).max(1e-9))
        };
        let ey = {
            let c = cross(UP, ex);
            mul(c, 1.0 / len(c).max(1e-9))
        };
        // Each member as its two ends and its width square to it, in the
        // cassette's axes.
        let flat: Vec<(Role, [f64; 2], [f64; 2], f64)> = self
            .members
            .iter()
            .map(|m| {
                let at = |p: P3| [dot(p, ex), dot(p, ey)];
                (m.role, at(m.start), at(m.end), m.section.width)
            })
            .collect();
        let mut lo = [f64::INFINITY; 2];
        for (_, a, b, w) in &flat {
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let l = (dx * dx + dy * dy).sqrt().max(1e-9);
            let (nx, ny) = (-dy / l * w / 2.0, dx / l * w / 2.0);
            for q in [a, b] {
                for sgn in [-1.0, 1.0] {
                    let c = [q[0] + nx * sgn, q[1] + ny * sgn];
                    for k in 0..2 {
                        lo[k] = lo[k].min(c[k]);
                    }
                }
            }
        }
        flat.into_iter()
            .map(|(r, a, b, w)| {
                (
                    r,
                    [a[0] - lo[0], a[1] - lo[1]],
                    [b[0] - lo[0], b[1] - lo[1]],
                    w,
                )
            })
            .collect()
    }
}

/// The cassettes of the framed floors and roofs, floors first.
pub fn deck_panels(model: &Model) -> Vec<Cassette> {
    let mut out = Vec::new();
    let mut groups: Vec<(char, SolidOwner, Vec<Member>)> = Vec::new();
    for s in model.slabs() {
        let ms = slab_members(model, s.id);
        if !ms.is_empty() {
            groups.push(('F', SolidOwner::Slab(s.id), ms));
        }
    }
    for r in model.roofs() {
        let ms = roof_members(model, r.id);
        if !ms.is_empty() {
            groups.push(('C', SolidOwner::Roof(r.id), ms));
        }
    }
    let mut count = std::collections::BTreeMap::<char, usize>::new();
    for (letter, owner, ms) in groups {
        let n = {
            let c = count.entry(letter).or_insert(0);
            *c += 1;
            *c
        };
        let mut keys: Vec<usize> = ms.iter().filter_map(|m| m.panel).collect();
        keys.sort_unstable();
        keys.dedup();
        for (i, key) in keys.into_iter().enumerate() {
            let mine: Vec<Member> = ms
                .iter()
                .filter(|m| m.panel == Some(key))
                .cloned()
                .collect();
            let mut c = Cassette {
                mark: format!("{letter}{n}-{}", i + 1),
                owner,
                index: i,
                length: 0.0,
                width: 0.0,
                members: mine,
            };
            let (l, w) = c.size();
            c.length = l;
            c.width = w;
            out.push(c);
        }
    }
    out
}

/// Where a member crosses a plane (`origin`, `normal`): the outline of
/// each plate of its section there (corners on the plane). Empty if it
/// does not cross it.
pub fn member_cut(m: &Member, origin: P3, normal: P3) -> Vec<[P3; 4]> {
    let axis = sub(m.end, m.start);
    let (da, db) = (
        dot(sub(m.start, origin), normal),
        dot(sub(m.end, origin), normal),
    );
    if da * db > 0.0 || (da - db).abs() < 1e-12 {
        return Vec::new();
    }
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
    let an = dot(axis, normal);
    let on_plane = |c: P3| {
        // Slide the corner along the member to the plane.
        let k = -dot(sub(c, origin), normal) / an;
        add(c, mul(axis, k))
    };
    rects
        .into_iter()
        .map(|(d0, d1, w0, w1)| {
            let c =
                |dd: f64, ww: f64| on_plane(add(add(m.start, mul(m.across, dd)), mul(m.open, ww)));
            [c(d0, w0), c(d1, w0), c(d1, w1), c(d0, w1)]
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

    #[test]
    fn gables_follow_the_roof_and_floors_go_in_cassettes() {
        let mut m = framed_house(FrameMaterial::Timber);
        let level = m.levels()[0].id;
        let st = m.slabs().next().unwrap().slab_type;
        let roof = m
            .add_roof(NewRoof {
                roof_type: st,
                level,
                outline: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(9.0, 0.0),
                    Point2::new(9.0, 6.0),
                    Point2::new(0.0, 6.0),
                ],
                // Gables at x = 0 and x = 9: the planes rise from the long
                // sides.
                slopes: vec![Some(0.5), None, Some(0.5), None],
                base_offset: 2.7,
                overhang: 0.3,
            })
            .unwrap();
        let walls: Vec<WallId> = m.walls().map(|w| w.id).collect();
        for &w in &walls {
            m.set_wall_roof(w, Some(roof)).unwrap();
        }
        // The gable at x = 9 (from (9,0) to (9,6)): its middle studs rise
        // to under the ridge, 1,5 m over the eaves.
        let gable = walls[1];
        let studs: Vec<Member> = wall_members(&m, gable)
            .into_iter()
            .filter(|x| x.role == Role::Stud)
            .collect();
        let highest = studs.iter().map(|x| x.end[2]).fold(0.0, f64::max);
        assert!(highest > 3.6 && highest < 4.3, "{highest}");
        let lowest = studs.iter().map(|x| x.end[2]).fold(9.0, f64::min);
        assert!(lowest < 2.9, "{lowest}");
        // Its top track breaks at the ridge: at least two pieces sloping.
        let tops: Vec<Member> = wall_members(&m, gable)
            .into_iter()
            .filter(|x| x.role == Role::TopTrack)
            .collect();
        let rises: Vec<(f64, f64, f64)> = tops
            .iter()
            .map(|t| (t.along[0], t.along[1], t.end[2] - t.start[2]))
            .collect();
        assert!(
            tops.len() >= 2 && tops.iter().all(|t| (t.end[2] - t.start[2]).abs() > 0.3),
            "{rises:?}"
        );
        // The floor in cassettes no wider than 2,4 m, with their joists.
        let cassettes = deck_panels(&m);
        let floors: Vec<&Cassette> = cassettes
            .iter()
            .filter(|c| c.mark.starts_with('F'))
            .collect();
        assert!(floors.len() >= 3, "{}", floors.len());
        assert!(
            floors.iter().all(|c| c.width <= CASSETTE_MAX + 0.05),
            "{floors:?}"
        );
        // The roof's rafter pairs are tied: trusses.
        let ties = roof_members(&m, roof)
            .into_iter()
            .filter(|x| x.role == Role::Tie)
            .count();
        assert!(ties >= 10, "{ties}");
    }

    #[test]
    fn curved_walls_are_framed_along_their_arc() {
        let mut m = framed_house(FrameMaterial::Steel);
        let level = m.levels()[0].id;
        let wt = m.walls().next().unwrap().wall_type;
        let w = m
            .add_wall(NewWall {
                wall_type: wt,
                level,
                start: Point2::new(0.0, 20.0),
                end: Point2::new(8.0, 20.0),
                bulge: 0.4,
                reference: ReferenceLine::Center,
                height: 2.7,
                base_offset: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        let ms = wall_members(&m, w);
        let wall = m.wall(w).unwrap().clone();
        // Studs at the spacing along the arc, every one on it.
        let studs: Vec<&Member> = ms.iter().filter(|x| x.role == Role::Stud).collect();
        assert!(studs.len() as f64 > wall.length() / 0.6, "{}", studs.len());
        let arc = wall.arc().unwrap();
        for s in &studs {
            let r = Point2::new(s.start[0], s.start[1]).distance(arc.centre);
            assert!((r - arc.radius).abs() < 0.1, "{r} {}", arc.radius);
        }
        // The tracks are chords, stud to stud.
        let bottoms = ms.iter().filter(|x| x.role == Role::BottomTrack).count();
        assert!(bottoms > 10, "{bottoms}");
    }

    #[test]
    fn wide_holes_get_deeper_headers_and_cuts_are_plates() {
        let f = Framing::for_depth(FrameMaterial::Timber, 0.15);
        assert!(header_section(&f, 0.9, 1.0).depth <= f.stud.depth + 1e-9);
        let wide = header_section(&f, 2.4, 1.0);
        assert!(wide.depth >= 0.24 - 1e-9, "{}", wide.name);
        let steel = Framing::for_depth(FrameMaterial::Steel, 0.15);
        assert!(header_section(&steel, 3.0, 1.0).depth >= 0.25 - 1e-9);
        // A stud cut by a level plane: a C of three plates and two lips.
        let m = framed_house(FrameMaterial::Steel);
        let w = m.walls().next().unwrap().id;
        let stud = wall_members(&m, w)
            .into_iter()
            .find(|x| x.role == Role::Stud)
            .unwrap();
        let cut = member_cut(&stud, [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]);
        assert_eq!(cut.len(), 5);
        assert!(cut.iter().flatten().all(|p| (p[2] - 1.0).abs() < 1e-9));
        assert!(member_cut(&stud, [0.0, 0.0, 9.0], [0.0, 0.0, 1.0]).is_empty());
    }
}
