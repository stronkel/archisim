//! Urbanismo (`docs/DISCIPLINAS.md` §3): plots and roads, which belong to
//! the site and not to the building.
//!
//! A [`Plot`] is a parcel by its boundary, with its cadastral reference and
//! the setback the building keeps from its boundary: its area is what the
//! masses' plot ratio and coverage are worked out with. A [`Road`] runs
//! along an axis with a cross-section ([`RoadSection`], H6): its strips
//! across — lanes, parking in line, at 45° or at right angles, bike
//! lanes, kerbs, sidewalks, planting strips and medians —, with the
//! sidewalk widening over the parking where it is asked («orejas») and
//! zebra crossings; a single-level street has no kerbs standing up. IFC: a plot is an
//! `IfcSite` inside the project's site; a road an `IfcRoad` with its
//! `IfcPavement`s and `IfcKerb`s (in IFC4, proxies on the site).

use super::*;

/// A parcel.
#[derive(Debug, Clone, PartialEq)]
pub struct Plot {
    pub id: PlotId,
    pub guid: GlobalId,
    pub name: String,
    pub level: LevelId,
    /// Its boundary, counter-clockwise.
    pub outline: Vec<Point2>,
    /// What the building keeps from the boundary, metres.
    pub setback: f64,
    /// Cadastral reference.
    pub reference: String,
}

/// Data needed to make a plot.
#[derive(Debug, Clone, PartialEq)]
pub struct NewPlot {
    pub name: String,
    pub level: LevelId,
    pub outline: Vec<Point2>,
    pub setback: f64,
    pub reference: String,
}

impl Plot {
    pub fn area(&self) -> f64 {
        signed_area(&self.outline).abs()
    }

    pub fn perimeter(&self) -> f64 {
        let n = self.outline.len();
        (0..n)
            .map(|i| self.outline[i].distance(self.outline[(i + 1) % n]))
            .sum()
    }

    /// Where the building may go: the boundary `setback` inwards (none if
    /// it does not fit).
    pub fn buildable(&self) -> Option<Vec<Point2>> {
        if self.setback <= EPSILON {
            Some(self.outline.clone())
        } else {
            offset_contour(&self.outline, -self.setback)
        }
    }
}

/// How cars park in a parking strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParkingMode {
    /// Along the kerb («en cordón»): bays of 5,00 m.
    #[default]
    Line,
    /// At 45° («en espiga»): bays of 2,50 m across, slanted.
    Angled,
    /// At right angles («en batería»): bays of 2,50 m.
    Battery,
}

impl ParkingMode {
    pub const ALL: [ParkingMode; 3] =
        [ParkingMode::Line, ParkingMode::Angled, ParkingMode::Battery];

    pub fn code(self) -> &'static str {
        match self {
            ParkingMode::Line => "line",
            ParkingMode::Angled => "angled",
            ParkingMode::Battery => "battery",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.code() == c)
    }

    pub fn name(self) -> &'static str {
        match self {
            ParkingMode::Line => "En cordón",
            ParkingMode::Angled => "En espiga (45°)",
            ParkingMode::Battery => "En batería",
        }
    }

    /// The usual width of its strip, metres.
    pub fn width(self) -> f64 {
        match self {
            ParkingMode::Line => 2.2,
            ParkingMode::Angled => 4.5,
            ParkingMode::Battery => 5.0,
        }
    }

    /// How far apart its bay lines are along the road.
    pub fn bay(self) -> f64 {
        match self {
            ParkingMode::Line => 5.0,
            ParkingMode::Angled => 3.5,
            ParkingMode::Battery => 2.5,
        }
    }
}

/// A strip of a road's cross-section.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoadStrip {
    pub band: RoadBand,
    pub width: f64,
}

/// Where the sidewalk widens over a parking strip («orejas», «quiebros en
/// el acerado»): on a side of the road, from a distance along its axis to
/// another.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Buildout {
    /// To the left of the axis (looking along it) or to its right.
    pub left: bool,
    pub from: f64,
    pub to: f64,
}

/// A zebra crossing over the carriageway, centred at a distance along the
/// axis, `width` wide along it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crossing {
    pub at: f64,
    pub width: f64,
}

/// The cross-section of a road: its strips across, from the left edge
/// (looking along the axis) to the right one, centred on the axis; and what
/// changes along it.
#[derive(Debug, Clone, PartialEq)]
pub struct RoadSection {
    pub strips: Vec<RoadStrip>,
    /// How far the kerbs, sidewalks, planting strips and medians stand
    /// over the carriageway (none in a single-level street).
    pub kerb_height: f64,
    /// Depth of the pavement under its top.
    pub depth: f64,
    /// All at one level («plataforma única»): no kerbs standing up.
    pub single_level: bool,
    pub parking: ParkingMode,
    pub buildouts: Vec<Buildout>,
    pub crossings: Vec<Crossing>,
    /// The planting strips' soil, the bike lanes' and the road markings'
    /// materials; none: the sidewalk's, the pavement's, and markings drawn
    /// in plan only.
    pub planting: Option<MaterialId>,
    pub bike: Option<MaterialId>,
    pub markings: Option<MaterialId>,
}

impl Default for RoadSection {
    /// Two lanes of 3,00 m, parking of 2,20 m, kerbs of 0,15 m and
    /// sidewalks of 2,50 m.
    fn default() -> Self {
        RoadSection::classic(6.0, 2.2, 0.15, 2.5)
    }
}

/// A strip.
pub const fn strip_of(band: RoadBand, width: f64) -> RoadStrip {
    RoadStrip { band, width }
}

impl RoadSection {
    /// The section of before H6: a carriageway of two lanes in the middle
    /// and, on each side, parking (none if 0), kerb and sidewalk.
    pub fn classic(carriageway: f64, parking: f64, kerb: f64, sidewalk: f64) -> Self {
        let side = [
            strip_of(RoadBand::Sidewalk, sidewalk),
            strip_of(RoadBand::Kerb, kerb),
            strip_of(RoadBand::Parking, parking),
        ];
        let mut strips: Vec<RoadStrip> = side.to_vec();
        strips.push(strip_of(RoadBand::Carriageway, carriageway / 2.0));
        strips.push(strip_of(RoadBand::Carriageway, carriageway / 2.0));
        strips.extend(side.iter().rev().copied());
        strips.retain(|s| s.width > EPSILON);
        RoadSection::of(strips)
    }

    /// A section of these strips, the rest as usual.
    pub fn of(strips: Vec<RoadStrip>) -> Self {
        RoadSection {
            strips,
            kerb_height: 0.15,
            depth: 0.4,
            single_level: false,
            parking: ParkingMode::Line,
            buildouts: Vec::new(),
            crossings: Vec::new(),
            planting: None,
            bike: None,
            markings: None,
        }
    }

    pub fn width(&self) -> f64 {
        self.strips.iter().map(|s| s.width).sum()
    }

    /// The width of the strips of a band.
    pub fn width_of(&self, band: RoadBand) -> f64 {
        self.strips
            .iter()
            .filter(|s| s.band == band)
            .map(|s| s.width)
            .sum()
    }

    /// Each strip with how far its two edges are to the left of the axis
    /// (negative: to its right), the nearer first, and whether it is on the
    /// left half; from the left edge to the right one.
    pub fn band_offsets(&self) -> Vec<(RoadBand, f64, f64)> {
        let mut d = self.width() / 2.0;
        let mut out = Vec::new();
        for s in &self.strips {
            out.push((s.band, d - s.width, d));
            d -= s.width;
        }
        out
    }

    /// Its strips, parking mode, build-outs and crossings as a text, for
    /// IFC: `lane 3;parking 2.2|line|0.15 0.4 0|L 10 16;R 30 36|20 4`.
    pub fn encode(&self) -> String {
        let strips: Vec<String> = self
            .strips
            .iter()
            .map(|x| format!("{} {}", x.band.code(), x.width))
            .collect();
        let outs: Vec<String> = self
            .buildouts
            .iter()
            .map(|b| format!("{} {} {}", if b.left { "L" } else { "R" }, b.from, b.to))
            .collect();
        let cross: Vec<String> = self
            .crossings
            .iter()
            .map(|c| format!("{} {}", c.at, c.width))
            .collect();
        format!(
            "{}|{}|{} {} {}|{}|{}",
            strips.join(";"),
            self.parking.code(),
            self.kerb_height,
            self.depth,
            u8::from(self.single_level),
            outs.join(";"),
            cross.join(";")
        )
    }

    /// The section of [`RoadSection::encode`]'s text (its materials
    /// none).
    pub fn decode(text: &str) -> Option<RoadSection> {
        let parts: Vec<&str> = text.split('|').collect();
        if parts.len() != 5 {
            return None;
        }
        let num = |t: &str| t.trim().parse::<f64>().ok();
        let mut strips = Vec::new();
        for x in parts[0].split(';').filter(|x| !x.trim().is_empty()) {
            let (b, w) = x.trim().split_once(' ')?;
            strips.push(strip_of(RoadBand::from_code(b)?, num(w)?));
        }
        let mut s = RoadSection::of(strips);
        s.parking = ParkingMode::from_code(parts[1].trim())?;
        let v: Vec<&str> = parts[2].split_whitespace().collect();
        if v.len() != 3 {
            return None;
        }
        s.kerb_height = num(v[0])?;
        s.depth = num(v[1])?;
        s.single_level = v[2] == "1";
        for x in parts[3].split(';').filter(|x| !x.trim().is_empty()) {
            let v: Vec<&str> = x.split_whitespace().collect();
            if v.len() != 3 {
                return None;
            }
            s.buildouts.push(Buildout {
                left: v[0] == "L",
                from: num(v[1])?,
                to: num(v[2])?,
            });
        }
        for x in parts[4].split(';').filter(|x| !x.trim().is_empty()) {
            let (a, w) = x.trim().split_once(' ')?;
            s.crossings.push(Crossing {
                at: num(a)?,
                width: num(w)?,
            });
        }
        Some(s)
    }

    /// Whether a band stands at the kerb's height.
    pub fn raised(&self, band: RoadBand) -> bool {
        !self.single_level && band.raised()
    }
}

/// The usual roads: their name and their section.
pub fn road_presets() -> Vec<(&'static str, RoadSection)> {
    use RoadBand::*;
    let s = |b, w| strip_of(b, w);
    let mut single = RoadSection::of(vec![
        s(Sidewalk, 1.8),
        s(Carriageway, 3.5),
        s(Sidewalk, 1.8),
    ]);
    single.single_level = true;
    let mut battery = RoadSection::of(vec![
        s(Sidewalk, 2.5),
        s(Kerb, 0.15),
        s(Parking, 5.0),
        s(Carriageway, 3.5),
        s(Kerb, 0.15),
        s(Sidewalk, 2.5),
    ]);
    battery.parking = ParkingMode::Battery;
    let mut rural = RoadSection::of(vec![
        s(Planting, 1.0),
        s(Carriageway, 2.75),
        s(Carriageway, 2.75),
        s(Planting, 1.0),
    ]);
    rural.single_level = true;
    vec![
        (
            "Calle de doble sentido con aparcamiento",
            RoadSection::default(),
        ),
        (
            "Calle de un sentido con aparcamiento",
            RoadSection::of(vec![
                s(Sidewalk, 2.0),
                s(Kerb, 0.15),
                s(Parking, 2.2),
                s(Carriageway, 3.5),
                s(Kerb, 0.15),
                s(Sidewalk, 2.0),
            ]),
        ),
        ("Calle de plataforma única", single),
        ("Calle con aparcamiento en batería", battery),
        (
            "Calle con carril bici",
            RoadSection::of(vec![
                s(Sidewalk, 2.5),
                s(Kerb, 0.15),
                s(BikeLane, 2.0),
                s(Kerb, 0.15),
                s(Parking, 2.2),
                s(Carriageway, 3.0),
                s(Carriageway, 3.0),
                s(Kerb, 0.15),
                s(Sidewalk, 2.5),
            ]),
        ),
        (
            "Avenida con mediana arbolada",
            RoadSection::of(vec![
                s(Sidewalk, 2.5),
                s(Planting, 1.2),
                s(Kerb, 0.15),
                s(Parking, 2.2),
                s(Carriageway, 3.25),
                s(Carriageway, 3.25),
                s(Kerb, 0.15),
                s(Median, 2.0),
                s(Kerb, 0.15),
                s(Carriageway, 3.25),
                s(Carriageway, 3.25),
                s(Parking, 2.2),
                s(Kerb, 0.15),
                s(Planting, 1.2),
                s(Sidewalk, 2.5),
            ]),
        ),
        (
            "Bulevar con paseo central",
            RoadSection::of(vec![
                s(Sidewalk, 3.0),
                s(Kerb, 0.15),
                s(Carriageway, 3.5),
                s(Kerb, 0.15),
                s(Planting, 1.5),
                s(Median, 8.0),
                s(Planting, 1.5),
                s(Kerb, 0.15),
                s(Carriageway, 3.5),
                s(Kerb, 0.15),
                s(Sidewalk, 3.0),
            ]),
        ),
        ("Camino rural", rural),
    ]
}

/// A band of a road's section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RoadBand {
    /// A lane of the carriageway.
    Carriageway,
    Parking,
    Kerb,
    Sidewalk,
    BikeLane,
    /// A strip of soil for trees and shrubs («alcorques»).
    Planting,
    /// A raised median or central walk.
    Median,
}

impl RoadBand {
    pub const ALL: [RoadBand; 7] = [
        RoadBand::Carriageway,
        RoadBand::Parking,
        RoadBand::Kerb,
        RoadBand::Sidewalk,
        RoadBand::BikeLane,
        RoadBand::Planting,
        RoadBand::Median,
    ];

    pub fn name(self) -> &'static str {
        match self {
            RoadBand::Carriageway => "Calzada",
            RoadBand::Parking => "Aparcamiento",
            RoadBand::Kerb => "Bordillo",
            RoadBand::Sidewalk => "Acera",
            RoadBand::BikeLane => "Carril bici",
            RoadBand::Planting => "Alcorques",
            RoadBand::Median => "Mediana",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            RoadBand::Carriageway => "lane",
            RoadBand::Parking => "parking",
            RoadBand::Kerb => "kerb",
            RoadBand::Sidewalk => "sidewalk",
            RoadBand::BikeLane => "bike",
            RoadBand::Planting => "planting",
            RoadBand::Median => "median",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|b| b.code() == c)
    }

    /// Whether it stands at the kerb's height (but in a single-level
    /// street).
    pub fn raised(self) -> bool {
        matches!(
            self,
            RoadBand::Kerb | RoadBand::Sidewalk | RoadBand::Planting | RoadBand::Median
        )
    }

    /// Whether cars, bikes or the parked ones go on it.
    pub fn traffic(self) -> bool {
        matches!(
            self,
            RoadBand::Carriageway | RoadBand::Parking | RoadBand::BikeLane
        )
    }
}

/// A piece of a road: a strip over a stretch of its axis.
#[derive(Debug, Clone, PartialEq)]
pub struct RoadPiece {
    pub band: RoadBand,
    pub outline: Vec<Point2>,
    pub raised: bool,
    pub material: MaterialId,
}

/// The point at `s` along a path and its direction there.
fn along(path: &[Point2], s: f64) -> (Point2, Point2) {
    let mut left = s.max(0.0);
    for w in path.windows(2) {
        let l = w[0].distance(w[1]);
        let u = (w[1] - w[0]) * (1.0 / l.max(EPSILON));
        if left <= l + EPSILON {
            return (w[0] + u * left, u);
        }
        left -= l;
    }
    let n = path.len();
    let u = (path[n - 1] - path[n - 2]) * (1.0 / path[n - 2].distance(path[n - 1]).max(EPSILON));
    (path[n - 1], u)
}

/// The part of a path from `s0` to `s1` along it.
fn sub_path(path: &[Point2], s0: f64, s1: f64) -> Vec<Point2> {
    let mut out = vec![along(path, s0).0];
    let mut s = 0.0;
    for w in path.windows(2) {
        s += w[0].distance(w[1]);
        if s > s0 + EPSILON && s < s1 - EPSILON {
            out.push(w[1]);
        }
    }
    out.push(along(path, s1).0);
    out
}

/// A rectangle along the axis: from `s0` to `s1` along it, from `a` to `b`
/// to its left.
fn quad(path: &[Point2], s0: f64, s1: f64, a: f64, b: f64) -> Vec<Point2> {
    let (p, u) = along(path, s0);
    let (q, v) = along(path, s1);
    let (n, m) = (u.perp_left(), v.perp_left());
    vec![p + n * a, q + m * a, q + m * b, p + n * b]
}

/// A road.
#[derive(Debug, Clone, PartialEq)]
pub struct Road {
    pub id: RoadId,
    pub guid: GlobalId,
    pub name: String,
    pub level: LevelId,
    /// Its axis, two points at least.
    pub axis: Vec<Point2>,
    pub section: RoadSection,
    /// Height of the carriageway over the level.
    pub top_offset: f64,
    /// Of the carriageway and parking, of the kerbs and of the sidewalks.
    pub pavement: MaterialId,
    pub kerb_material: MaterialId,
    pub sidewalk_material: MaterialId,
}

/// Data needed to make a road.
#[derive(Debug, Clone, PartialEq)]
pub struct NewRoad {
    pub name: String,
    pub level: LevelId,
    pub axis: Vec<Point2>,
    pub section: RoadSection,
    pub top_offset: f64,
    pub pavement: MaterialId,
    pub kerb_material: MaterialId,
    pub sidewalk_material: MaterialId,
}

impl Road {
    /// Its pieces: each strip of the section over each stretch of the axis
    /// between the ends of the build-outs, where a build-out turns the
    /// parking of its side into sidewalk.
    pub fn pieces(&self) -> Vec<RoadPiece> {
        let s = &self.section;
        let len = self.length();
        let mut cuts = vec![0.0, len];
        for b in &s.buildouts {
            cuts.push(b.from.clamp(0.0, len));
            cuts.push(b.to.clamp(0.0, len));
        }
        cuts.sort_by(f64::total_cmp);
        cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        let mut out = Vec::new();
        for w in cuts.windows(2) {
            let (s0, s1) = (w[0], w[1]);
            if s1 - s0 < 1e-6 {
                continue;
            }
            let mid = (s0 + s1) / 2.0;
            let out_left = s
                .buildouts
                .iter()
                .any(|b| b.left && b.from <= mid && b.to >= mid);
            let out_right = s
                .buildouts
                .iter()
                .any(|b| !b.left && b.from <= mid && b.to >= mid);
            let path = sub_path(&self.axis, s0, s1);
            for (band, a, b) in s.band_offsets() {
                let left = a + b > 0.0;
                let band =
                    if band == RoadBand::Parking && ((left && out_left) || (!left && out_right)) {
                        RoadBand::Sidewalk
                    } else {
                        band
                    };
                let outline = strip(&path, a, b);
                if outline.len() < 3 {
                    continue;
                }
                let material = match band {
                    RoadBand::Carriageway | RoadBand::Parking => self.pavement,
                    RoadBand::BikeLane => s.bike.unwrap_or(self.pavement),
                    RoadBand::Kerb => self.kerb_material,
                    RoadBand::Sidewalk | RoadBand::Median => self.sidewalk_material,
                    RoadBand::Planting => s.planting.unwrap_or(self.sidewalk_material),
                };
                out.push(RoadPiece {
                    band,
                    raised: s.raised(band),
                    outline,
                    material,
                });
            }
        }
        out
    }

    /// Each band of the section with its outline (its pieces).
    pub fn bands(&self) -> Vec<(RoadBand, Vec<Point2>)> {
        self.pieces()
            .into_iter()
            .map(|p| (p.band, p.outline))
            .collect()
    }

    /// The road markings, as thin polygons on the carriageway: the lines
    /// between lanes (dashed), the bays of the parking (not where the
    /// sidewalk widens) and the zebra crossings.
    pub fn markings(&self) -> Vec<Vec<Point2>> {
        let s = &self.section;
        let len = self.length();
        let offs = s.band_offsets();
        let mut out = Vec::new();
        let line = 0.1;
        // Between two lanes: dashes of 3 m every 8 m.
        for w in offs.windows(2) {
            if w[0].0 == RoadBand::Carriageway && w[1].0 == RoadBand::Carriageway {
                let d = w[0].1;
                let mut t = 1.0;
                while t + 3.0 <= len - 1.0 {
                    out.push(quad(&self.axis, t, t + 3.0, d - line / 2.0, d + line / 2.0));
                    t += 8.0;
                }
            }
        }
        // The bays.
        let widened = |left: bool, t: f64| {
            s.buildouts
                .iter()
                .any(|b| b.left == left && b.from - 0.5 <= t && b.to + 0.5 >= t)
        };
        let crossing = |t: f64| {
            s.crossings
                .iter()
                .any(|c| (t - c.at).abs() <= c.width / 2.0 + 0.5)
        };
        for &(band, a, b) in &offs {
            if band != RoadBand::Parking {
                continue;
            }
            let left = a + b > 0.0;
            let bay = s.parking.bay();
            let mut t = bay;
            while t < len - 0.5 {
                if !widened(left, t) && !crossing(t) {
                    let (p, u) = along(&self.axis, t);
                    let n = u.perp_left();
                    let slant = match s.parking {
                        ParkingMode::Angled => b - a,
                        _ => 0.0,
                    };
                    let (inner, outer) = if left { (a, b) } else { (b, a) };
                    let p0 = p + n * inner;
                    let p1 = p + n * outer + u * slant;
                    let w = u * (line / 2.0);
                    out.push(vec![p0 - w, p0 + w, p1 + w, p1 - w]);
                }
                t += bay;
            }
        }
        // Zebra crossings over the traffic strips: bars along the road,
        // 0,5 m wide every metre.
        for c in &s.crossings {
            let (s0, s1) = (c.at - c.width / 2.0, c.at + c.width / 2.0);
            if s0 < 0.0 || s1 > len {
                continue;
            }
            for &(band, a, b) in &offs {
                if !band.traffic() || band == RoadBand::Parking {
                    continue;
                }
                let mut d = a + 0.25;
                while d + 0.5 <= b - 0.1 {
                    out.push(quad(&self.axis, s0, s1, d, d + 0.5));
                    d += 1.0;
                }
            }
        }
        out
    }

    /// Its outline: the whole width along the axis.
    pub fn outline(&self) -> Vec<Point2> {
        let w = self.section.width() / 2.0;
        strip(&self.axis, -w, w)
    }

    pub fn length(&self) -> f64 {
        self.axis.windows(2).map(|w| w[0].distance(w[1])).sum()
    }

    /// The area of each kind of band, m².
    pub fn area_of(&self, band: RoadBand) -> f64 {
        self.bands()
            .iter()
            .filter(|(b, _)| *b == band)
            .map(|(_, o)| signed_area(o).abs())
            .sum()
    }

    /// The line along the middle of each strip of a band, for planting
    /// trees in it.
    pub fn strip_lines(&self, band: RoadBand) -> Vec<Vec<Point2>> {
        self.section
            .band_offsets()
            .into_iter()
            .filter(|(b, _, _)| *b == band)
            .filter_map(|(_, a, b)| offset_path(&self.axis, (a + b) / 2.0))
            .collect()
    }
}

impl Model {
    // ---- Plots --------------------------------------------------------------

    fn validate_plot(&self, p: &NewPlot) -> Result<(), ModelError> {
        if !self.levels.contains_key(&p.level) {
            return Err(ModelError::UnknownLevel(p.level));
        }
        for q in &p.outline {
            finite_point(*q, "plot boundary")?;
        }
        if p.outline.len() < 3 || signed_area(&p.outline).abs() <= EPSILON {
            return Err(ModelError::InvalidShape(
                "a plot needs a boundary of three points or more",
            ));
        }
        finite(p.setback, "plot setback")?;
        if p.setback < 0.0 {
            return Err(ModelError::NonPositive("plot setback"));
        }
        Ok(())
    }

    pub fn add_plot(&mut self, spec: NewPlot) -> Result<PlotId, ModelError> {
        let mut spec = spec;
        if signed_area(&spec.outline) < 0.0 {
            spec.outline.reverse();
        }
        self.validate_plot(&spec)?;
        let id = PlotId(self.take_id());
        self.plots.insert(
            id,
            Plot {
                id,
                guid: GlobalId::new(),
                name: spec.name,
                level: spec.level,
                outline: spec.outline,
                setback: spec.setback,
                reference: spec.reference,
            },
        );
        Ok(id)
    }

    pub fn edit_plot(
        &mut self,
        id: PlotId,
        edit: impl FnOnce(&mut Plot),
    ) -> Result<(), ModelError> {
        let mut p = self
            .plots
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        edit(&mut p);
        if signed_area(&p.outline) < 0.0 {
            p.outline.reverse();
        }
        self.validate_plot(&NewPlot {
            name: p.name.clone(),
            level: p.level,
            outline: p.outline.clone(),
            setback: p.setback,
            reference: p.reference.clone(),
        })?;
        self.plots.insert(id, p);
        Ok(())
    }

    pub fn remove_plot(&mut self, id: PlotId) -> Result<Plot, ModelError> {
        self.plots
            .remove(&id)
            .ok_or(ModelError::UnknownElement(id.0))
    }

    pub fn plot(&self, id: PlotId) -> Option<&Plot> {
        self.plots.get(&id)
    }

    pub fn plots(&self) -> impl Iterator<Item = &Plot> {
        self.plots.values()
    }

    pub fn plots_on_level(&self, level: LevelId) -> impl Iterator<Item = &Plot> {
        self.plots.values().filter(move |p| p.level == level)
    }

    /// A name for a new plot: «Parcela 1».
    pub fn next_plot_name(&self) -> String {
        (1..)
            .map(|n: usize| format!("Parcela {n}"))
            .find(|n| !self.plots.values().any(|p| p.name == *n))
            .unwrap_or_default()
    }

    /// The plot an outline is on: the one holding its centre.
    pub fn plot_under(&self, outline: &[Point2]) -> Option<&Plot> {
        if outline.is_empty() {
            return None;
        }
        let c = outline.iter().fold(Point2::new(0.0, 0.0), |a, p| a + *p)
            * (1.0 / outline.len() as f64);
        self.plots.values().find(|p| point_in(&p.outline, c))
    }

    // ---- Roads --------------------------------------------------------------

    fn validate_road(&self, r: &NewRoad) -> Result<(), ModelError> {
        if !self.levels.contains_key(&r.level) {
            return Err(ModelError::UnknownLevel(r.level));
        }
        for m in [r.pavement, r.kerb_material, r.sidewalk_material] {
            if !self.materials.contains_key(&m) {
                return Err(ModelError::UnknownMaterial(m));
            }
        }
        for q in &r.axis {
            finite_point(*q, "road axis")?;
        }
        if r.axis.len() < 2 || r.axis.windows(2).any(|w| w[0].approx_eq(w[1])) {
            return Err(ModelError::InvalidShape(
                "a road needs an axis of two different points or more",
            ));
        }
        let s = &r.section;
        if s.strips.is_empty()
            || s.width_of(RoadBand::Carriageway) + s.width_of(RoadBand::BikeLane) <= EPSILON
        {
            return Err(ModelError::InvalidShape(
                "a road needs a lane or a bike lane in its section",
            ));
        }
        for st in &s.strips {
            finite(st.width, "road strip width")?;
            if st.width <= EPSILON {
                return Err(ModelError::NonPositive("road strip width"));
            }
        }
        finite(s.depth, "pavement depth")?;
        if s.depth <= EPSILON {
            return Err(ModelError::NonPositive("pavement depth"));
        }
        finite(s.kerb_height, "kerb height")?;
        if s.kerb_height < 0.0 {
            return Err(ModelError::NonPositive("kerb height"));
        }
        for b in &s.buildouts {
            finite(b.from, "build-out start")?;
            finite(b.to, "build-out end")?;
            if b.to <= b.from {
                return Err(ModelError::InvalidShape("a build-out ends after it starts"));
            }
        }
        for c in &s.crossings {
            finite(c.at, "crossing")?;
            finite(c.width, "crossing width")?;
            if c.width <= EPSILON {
                return Err(ModelError::NonPositive("crossing width"));
            }
        }
        for m in [s.planting, s.bike, s.markings].into_iter().flatten() {
            if !self.materials.contains_key(&m) {
                return Err(ModelError::UnknownMaterial(m));
            }
        }
        finite(r.top_offset, "road top offset")?;
        Ok(())
    }

    pub fn add_road(&mut self, spec: NewRoad) -> Result<RoadId, ModelError> {
        self.validate_road(&spec)?;
        let id = RoadId(self.take_id());
        self.roads.insert(
            id,
            Road {
                id,
                guid: GlobalId::new(),
                name: spec.name,
                level: spec.level,
                axis: spec.axis,
                section: spec.section,
                top_offset: spec.top_offset,
                pavement: spec.pavement,
                kerb_material: spec.kerb_material,
                sidewalk_material: spec.sidewalk_material,
            },
        );
        Ok(id)
    }

    pub fn edit_road(
        &mut self,
        id: RoadId,
        edit: impl FnOnce(&mut Road),
    ) -> Result<(), ModelError> {
        let mut r = self
            .roads
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        edit(&mut r);
        self.validate_road(&NewRoad {
            name: r.name.clone(),
            level: r.level,
            axis: r.axis.clone(),
            section: r.section.clone(),
            top_offset: r.top_offset,
            pavement: r.pavement,
            kerb_material: r.kerb_material,
            sidewalk_material: r.sidewalk_material,
        })?;
        self.roads.insert(id, r);
        Ok(())
    }

    pub fn remove_road(&mut self, id: RoadId) -> Result<Road, ModelError> {
        self.roads
            .remove(&id)
            .ok_or(ModelError::UnknownElement(id.0))
    }

    pub fn road(&self, id: RoadId) -> Option<&Road> {
        self.roads.get(&id)
    }

    pub fn roads(&self) -> impl Iterator<Item = &Road> {
        self.roads.values()
    }

    pub fn roads_on_level(&self, level: LevelId) -> impl Iterator<Item = &Road> {
        self.roads.values().filter(move |r| r.level == level)
    }

    /// A name for a new road: «Vial 1».
    pub fn next_road_name(&self) -> String {
        (1..)
            .map(|n: usize| format!("Vial {n}"))
            .find(|n| !self.roads.values().any(|r| r.name == *n))
            .unwrap_or_default()
    }

    pub(super) fn restore_urban(
        &mut self,
        plots: Vec<Plot>,
        roads: Vec<Road>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for p in plots {
            let spec = NewPlot {
                name: p.name.clone(),
                level: p.level,
                outline: p.outline.clone(),
                setback: p.setback,
                reference: p.reference.clone(),
            };
            claim(p.id.0, Some(&p.guid))
                .and_then(|_| self.validate_plot(&spec))
                .map_err(|error| RestoreError {
                    element: format!("plot {} «{}»", p.id.0, p.name),
                    error,
                })?;
            self.plots.insert(p.id, p);
        }
        for r in roads {
            let spec = NewRoad {
                name: r.name.clone(),
                level: r.level,
                axis: r.axis.clone(),
                section: r.section.clone(),
                top_offset: r.top_offset,
                pavement: r.pavement,
                kerb_material: r.kerb_material,
                sidewalk_material: r.sidewalk_material,
            };
            claim(r.id.0, Some(&r.guid))
                .and_then(|_| self.validate_road(&spec))
                .map_err(|error| RestoreError {
                    element: format!("road {} «{}»", r.id.0, r.name),
                    error,
                })?;
            self.roads.insert(r.id, r);
        }
        Ok(())
    }
}

/// Whether a closed polygon contains a point (ray casting).
fn point_in(poly: &[Point2], p: Point2) -> bool {
    let n = poly.len();
    let mut inside = false;
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plots_give_their_area_to_the_masses() {
        let mut m = Model::new("Urbanismo");
        let l = m.add_level("Rasante", 0.0);
        let id = m
            .add_plot(NewPlot {
                name: m.next_plot_name(),
                level: l,
                outline: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(0.0, 30.0),
                    Point2::new(40.0, 30.0),
                    Point2::new(40.0, 0.0),
                ],
                setback: 3.0,
                reference: "1234567VK4713S0001AB".into(),
            })
            .unwrap();
        let p = m.plot(id).unwrap();
        assert_eq!(p.name, "Parcela 1");
        assert!(signed_area(&p.outline) > 0.0);
        assert!((p.area() - 1200.0).abs() < 1e-9);
        assert!((p.perimeter() - 140.0).abs() < 1e-9);
        assert!((signed_area(&p.buildable().unwrap()) - 34.0 * 24.0).abs() < 1e-9);
        // Without an area written by hand, the masses take the plots'.
        assert_eq!(m.plot_area(), Some(1200.0));
        m.set_plot_area(Some(1000.0)).unwrap();
        assert_eq!(m.plot_area(), Some(1000.0));
        assert!(m.plot_under(&[Point2::new(10.0, 10.0)]).is_some());
    }

    #[test]
    fn roads_have_their_bands() {
        let mut m = Model::new("Urbanismo");
        let l = m.add_level("Rasante", 0.0);
        let mat = m.add_material("Asfalto", "asphalt", [90, 90, 90], CutPattern::Solid);
        let id = m
            .add_road(NewRoad {
                name: m.next_road_name(),
                level: l,
                axis: vec![Point2::new(0.0, 0.0), Point2::new(100.0, 0.0)],
                section: RoadSection::default(),
                top_offset: 0.0,
                pavement: mat,
                kerb_material: mat,
                sidewalk_material: mat,
            })
            .unwrap();
        let r = m.road(id).unwrap();
        assert!((r.section.width() - 15.7).abs() < 1e-9);
        // Two lanes, and parking, kerb and sidewalk on each side.
        assert_eq!(r.bands().len(), 8);
        assert!((r.area_of(RoadBand::Carriageway) - 600.0).abs() < 1e-9);
        assert!((r.area_of(RoadBand::Sidewalk) - 500.0).abs() < 1e-9);
        assert!((signed_area(&r.outline()) - 1570.0).abs() < 1e-9);
        assert!(m
            .edit_road(id, |r| r.axis = vec![Point2::new(0.0, 0.0)])
            .is_err());
    }
}
