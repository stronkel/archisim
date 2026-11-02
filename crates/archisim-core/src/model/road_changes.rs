//! Changes of section along a road (U2): from a distance along its axis
//! the road takes other strips across — a sidewalk that widens, parking
//! that ends, a lane that is added —, with a transition of a length over
//! which the strips' widths change gradually (when the two sections have
//! the same strips in the same order; otherwise the change is sharp, in
//! the middle of the transition).
//!
//! Everything else of the section (kerb height, parking mode, build-outs,
//! crossings, materials) is the road's own.
//!
//! Where the section changes, the carriageway stays where it was: the
//! strips are placed so that the middle of its lanes is where it is in the
//! road's own section ([`Road::offsets_at`]); widening one sidewalk does
//! not move the lanes. [`Model::sidewalks_to_plots`] uses that to take the
//! outer sidewalks out to the plots' façade line.

use super::*;

/// From `at` along the axis, the road has the strips `strips`, reached
/// over `transition` metres before `at`.
#[derive(Debug, Clone, PartialEq)]
pub struct SectionChange {
    pub at: f64,
    pub transition: f64,
    pub strips: Vec<RoadStrip>,
}

/// Whether two lists of strips can change gradually one into the other.
fn alike(a: &[RoadStrip], b: &[RoadStrip]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.band == y.band)
}

impl Road {
    fn sorted_changes(&self) -> Vec<&SectionChange> {
        let mut c: Vec<&SectionChange> = self.changes.iter().collect();
        c.sort_by(|a, b| a.at.total_cmp(&b.at));
        c
    }

    /// The strips of the road at `s` along its axis.
    pub fn strips_at(&self, s: f64) -> Vec<RoadStrip> {
        let mut now = self.section.strips.clone();
        for c in self.sorted_changes() {
            let start = c.at - c.transition.max(0.0);
            if s >= c.at {
                now = c.strips.clone();
                continue;
            }
            if s > start && c.transition > EPSILON {
                if alike(&now, &c.strips) {
                    let t = (s - start) / c.transition;
                    now = now
                        .iter()
                        .zip(&c.strips)
                        .map(|(x, y)| strip_of(x.band, x.width + (y.width - x.width) * t))
                        .collect();
                } else if s >= (start + c.at) / 2.0 {
                    now = c.strips.clone();
                }
            }
            break;
        }
        now
    }

    /// The road's section at `s` along its axis: its own, with the strips
    /// there.
    pub fn section_at(&self, s: f64) -> RoadSection {
        let mut sec = self.section.clone();
        sec.strips = self.strips_at(s);
        sec
    }

    /// Each strip at `s` along the axis with its offsets from the axis
    /// (inner, outer edge as in [`RoadSection::band_offsets`]), placed so
    /// that the middle of the carriageway is where it is in the road's
    /// own section.
    pub fn offsets_at(&self, s: f64) -> Vec<(RoadBand, f64, f64)> {
        let here = self.section_at(s).band_offsets();
        if self.changes.is_empty() {
            return here;
        }
        let shift = lanes_middle(&self.section.band_offsets()) - lanes_middle(&here);
        here.into_iter()
            .map(|(b, x, y)| (b, x + shift, y + shift))
            .collect()
    }

    /// How far the road reaches to its left and to its right of the axis
    /// at `s`.
    pub fn extent_at(&self, s: f64) -> (f64, f64) {
        let o = self.offsets_at(s);
        let left = o.iter().map(|x| x.2).fold(0.0, f64::max);
        let right = o.iter().map(|x| -x.1).fold(0.0, f64::max);
        (left, right)
    }

    /// Where its pieces are cut for its changes of section: the start and
    /// end of each transition (and its middle, where it is sharp).
    pub fn change_cuts(&self) -> Vec<f64> {
        let mut out = Vec::new();
        let mut now = self.section.strips.clone();
        for c in self.sorted_changes() {
            let start = c.at - c.transition.max(0.0);
            out.push(start);
            out.push(c.at);
            if !alike(&now, &c.strips) && c.transition > EPSILON {
                out.push((start + c.at) / 2.0);
            }
            now = c.strips.clone();
        }
        out
    }

    /// Its widest section's width.
    pub fn widest(&self) -> f64 {
        self.changes
            .iter()
            .map(|c| c.strips.iter().map(|x| x.width).sum::<f64>())
            .fold(self.section.width(), f64::max)
    }

    /// The changes' text, for IFC: `at transition band width,band width;…`.
    pub fn encode_changes(&self) -> String {
        self.sorted_changes()
            .iter()
            .map(|c| {
                let strips: Vec<String> = c
                    .strips
                    .iter()
                    .map(|x| format!("{} {}", x.band.code(), x.width))
                    .collect();
                format!("{} {} {}", c.at, c.transition, strips.join(","))
            })
            .collect::<Vec<_>>()
            .join(";")
    }

    /// The changes of [`Road::encode_changes`]'s text.
    pub fn decode_changes(text: &str) -> Option<Vec<SectionChange>> {
        let mut out = Vec::new();
        for x in text.split(';').filter(|x| !x.trim().is_empty()) {
            let mut it = x.trim().splitn(3, ' ');
            let at = it.next()?.parse().ok()?;
            let transition = it.next()?.parse().ok()?;
            let mut strips = Vec::new();
            for st in it.next()?.split(',') {
                let (b, w) = st.trim().split_once(' ')?;
                strips.push(strip_of(RoadBand::from_code(b)?, w.trim().parse().ok()?));
            }
            out.push(SectionChange {
                at,
                transition,
                strips,
            });
        }
        Some(out)
    }
}

/// The middle of the lanes of a section's offsets (of its traffic strips
/// if it has no lanes; of the whole if neither).
fn lanes_middle(offs: &[(RoadBand, f64, f64)]) -> f64 {
    let span = |f: &dyn Fn(RoadBand) -> bool| {
        let v: Vec<&(RoadBand, f64, f64)> = offs.iter().filter(|o| f(o.0)).collect();
        (!v.is_empty()).then(|| {
            let lo = v.iter().map(|o| o.1).fold(f64::MAX, f64::min);
            let hi = v.iter().map(|o| o.2).fold(f64::MIN, f64::max);
            (lo + hi) / 2.0
        })
    };
    span(&|b| b == RoadBand::Carriageway)
        .or_else(|| span(&|b| b.traffic()))
        .or_else(|| span(&|_| true))
        .unwrap_or(0.0)
}

/// How far the façade line of the plots is to each side of a road
/// (U2): the nearest plot edge crossed by the line square to the axis at
/// `p` (direction `u`), within `limit`; None if none.
fn room_to_plots(p: Point2, u: Vec2, plots: &[&Plot], limit: f64) -> (Option<f64>, Option<f64>) {
    let n = u.perp_left();
    let mut best = (None::<f64>, None::<f64>);
    for plot in plots {
        if point_in_polygon(p, &plot.outline) {
            continue;
        }
        let o = &plot.outline;
        for i in 0..o.len() {
            let (a, b) = (o[i], o[(i + 1) % o.len()]);
            // p + n·t = a + (b − a)·k
            let e = b - a;
            let den = n.x * e.y - n.y * e.x;
            if den.abs() < 1e-12 {
                continue;
            }
            let w = a - p;
            let t = (w.x * e.y - w.y * e.x) / den;
            let k = (w.x * n.y - w.y * n.x) / den;
            if !(-1e-9..=1.0 + 1e-9).contains(&k) || t.abs() > limit {
                continue;
            }
            if t > 0.0 {
                best.0 = Some(best.0.map_or(t, |x: f64| x.min(t)));
            } else if t < 0.0 {
                best.1 = Some(best.1.map_or(-t, |x: f64| x.min(-t)));
            }
        }
    }
    best
}

/// A station along a road where the façade line is measured: where it
/// is, its strips there and the sidewalk width taken on each side.
type Station = (f64, Vec<RoadStrip>, Option<f64>, Option<f64>);

/// How often the façade line is measured along a road, metres.
pub const FACADE_STEP: f64 = 1.0;
/// How much the façade line may wander before the sidewalk changes.
pub const FACADE_TOLERANCE: f64 = 0.25;
/// The narrowest sidewalk taken to the façade line.
pub const MIN_SIDEWALK: f64 = 0.9;

impl Model {
    /// The road's changes of section that take its outer sidewalks out to the
    /// façade line of the plots of its level on each side (U2): the line
    /// is measured every [`FACADE_STEP`] square to the axis; a new stretch
    /// starts where it moves more than [`FACADE_TOLERANCE`], and each
    /// stretch's sidewalk reaches the nearest point of the line along it,
    /// so it never goes into a plot. They replace the road's changes (its
    /// own section stays, and so the lanes stay where they are). Sides
    /// without plots (or without a sidewalk on the outside) keep their
    /// strips. None if the road has no plots beside it.
    pub fn sidewalks_to_plots(&self, id: RoadId) -> Option<Vec<SectionChange>> {
        let r = self.road(id)?;
        let plots: Vec<&Plot> = self.plots_on_level(r.level).collect();
        if plots.is_empty() {
            return None;
        }
        let len = r.length();
        let n = (len / FACADE_STEP).ceil().max(1.0) as usize;
        let limit = r.widest() / 2.0 + 30.0;
        let mut found = false;
        // At each station: its strips with the outer sidewalks out to the
        // line, and the room each side had (for grouping).
        let mut samples: Vec<Station> = Vec::new();
        for k in 0..=n {
            let s = (len * k as f64 / n as f64).clamp(0.0, len);
            let (p, u) = r.axis_at(s);
            let (left, right) = room_to_plots(p, u, &plots, limit);
            let mut strips = r.strips_at(s);
            let offs = r.offsets_at(s);
            let last = strips.len().saturating_sub(1);
            let mut left_used = None;
            let mut right_used = None;
            if let (Some(room), Some(first)) = (left, strips.first()) {
                if first.band.walk() {
                    let inner = offs[0].1;
                    if room - inner >= MIN_SIDEWALK {
                        left_used = Some(room - inner);
                    }
                }
            }
            if let (Some(room), Some(end)) = (right, strips.last()) {
                if end.band.walk() && last > 0 {
                    let inner = -offs[last].2;
                    if room - inner >= MIN_SIDEWALK {
                        right_used = Some(room - inner);
                    }
                }
            }
            if let Some(w) = left_used {
                strips[0].width = w;
                found = true;
            }
            if let Some(w) = right_used {
                strips[last].width = w;
                found = true;
            }
            samples.push((s, strips, left_used, right_used));
        }
        if !found {
            return None;
        }
        // Past the last plot at each end (a crossing, as a rule) the
        // sidewalk goes on as wide as by the plot.
        for left in [true, false] {
            let used = |x: &Station| {
                if left {
                    x.2
                } else {
                    x.3
                }
            };
            let Some(first) = samples.iter().position(|x| used(x).is_some()) else {
                continue;
            };
            let last = samples
                .iter()
                .rposition(|x| used(x).is_some())
                .unwrap_or(first);
            for (from, range) in [(first, 0..first), (last, last + 1..samples.len())] {
                let (w, bands) = {
                    let x = &samples[from];
                    let k = if left { 0 } else { x.1.len() - 1 };
                    (x.1[k].width, x.1.iter().map(|y| y.band).collect::<Vec<_>>())
                };
                for i in range {
                    let x = &mut samples[i];
                    if x.1.iter().map(|y| y.band).ne(bands.iter().copied()) {
                        continue;
                    }
                    let k = if left { 0 } else { x.1.len() - 1 };
                    x.1[k].width = w;
                    if left {
                        x.2 = Some(w);
                    } else {
                        x.3 = Some(w);
                    }
                }
            }
        }
        // Group the stations into stretches where nothing moves more than
        // the tolerance; each takes the narrowest sidewalk of its own.
        let close = |a: Option<f64>, b: Option<f64>| match (a, b) {
            (Some(x), Some(y)) => (x - y).abs() <= FACADE_TOLERANCE,
            (None, None) => true,
            _ => false,
        };
        let mut stretches: Vec<(f64, Vec<RoadStrip>)> = Vec::new();
        let mut i = 0;
        while i < samples.len() {
            let (s0, ref strips0, l0, r0) = samples[i];
            let mut strips = strips0.clone();
            let mut j = i + 1;
            while j < samples.len() {
                let (_, ref sj, lj, rj) = samples[j];
                let same_bands = sj.len() == strips.len()
                    && sj.iter().zip(&strips).all(|(a, b)| a.band == b.band);
                let inner_same = sj.iter().zip(&strips).enumerate().all(|(k, (a, b))| {
                    k == 0 || k + 1 == sj.len() || (a.width - b.width).abs() < 1e-6
                });
                if !same_bands || !inner_same || !close(l0, lj) || !close(r0, rj) {
                    break;
                }
                if lj.is_some() {
                    strips[0].width = strips[0].width.min(sj[0].width);
                }
                if rj.is_some() {
                    let e = strips.len() - 1;
                    strips[e].width = strips[e].width.min(sj[e].width);
                }
                j += 1;
            }
            for x in &mut strips {
                x.width = (x.width * 100.0).round() / 100.0;
            }
            // The change happens halfway between the last station of one
            // stretch and the first of the next.
            let at = if i == 0 {
                s0
            } else {
                (samples[i - 1].0 + s0) / 2.0
            };
            stretches.push((at, strips));
            i = j;
        }
        // The road's own section stays (the lanes are placed by it); the
        // first stretch is a change at the start if it differs.
        let mut changes = Vec::new();
        let mut now = r.section.strips.clone();
        for (at, strips) in stretches {
            if strips == now {
                continue;
            }
            now = strips.clone();
            changes.push(SectionChange {
                at: (at * 100.0).round() / 100.0,
                transition: 0.0,
                strips,
            });
        }
        Some(changes)
    }
}

/// The lines of a street drawn in sketch mode (U2): the kerb and the
/// façade line (the outer edge of the sidewalk) of each side, each from
/// the road's start to its end; empty where the side has no sidewalk.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RoadEdges {
    pub left_kerb: Vec<Point2>,
    pub left_facade: Vec<Point2>,
    pub right_kerb: Vec<Point2>,
    pub right_facade: Vec<Point2>,
}

impl RoadEdges {
    /// All four, those there are.
    pub fn lines(&self) -> Vec<Vec<Point2>> {
        [
            &self.left_kerb,
            &self.left_facade,
            &self.right_kerb,
            &self.right_facade,
        ]
        .into_iter()
        .filter(|l| l.len() >= 2)
        .cloned()
        .collect()
    }
}

/// On one side of a road with a sidewalk at its edge: how far from the
/// axis its kerb is (the kerb's outer face, or the sidewalk's inner edge
/// without a kerb) and its façade line (the sidewalk's outer edge), and
/// the strip just inside the kerb, which a kerb's move widens or
/// narrows.
fn side_lines(
    strips: &[RoadStrip],
    offs: &[(RoadBand, f64, f64)],
    left: bool,
) -> Option<(f64, f64, Option<usize>)> {
    let n = strips.len();
    if n < 2 || offs.len() != n {
        return None;
    }
    // Indices from that side inwards.
    let order: Vec<usize> = if left {
        (0..n).collect()
    } else {
        (0..n).rev().collect()
    };
    if !strips[order[0]].band.walk() {
        return None;
    }
    // Distance from the axis to an edge of strip i, on this side.
    let outer_of = |i: usize| if left { offs[i].2 } else { -offs[i].1 };
    let inner_of = |i: usize| if left { offs[i].1 } else { -offs[i].2 };
    let half = &order[..n.div_ceil(2)];
    let kb = half
        .iter()
        .copied()
        .find(|&i| strips[i].band == RoadBand::Kerb);
    let kerb = match kb {
        Some(i) => outer_of(i),
        None => inner_of(order[0]),
    };
    let from = kb.map_or(1, |i| order.iter().position(|&x| x == i).unwrap_or(0) + 1);
    let inner = order[from.min(n - 1)..]
        .iter()
        .copied()
        .find(|&i| !strips[i].band.walk() && strips[i].band != RoadBand::Kerb);
    Some((kerb, outer_of(order[0]), inner))
}

/// The narrowest a strip is left by moving a kerb or a façade line.
pub const MIN_EDGE_STRIP: f64 = 0.3;

impl Road {
    /// Where its section can change: its ends, its axis's corners and its
    /// changes' cuts.
    fn edge_stations(&self) -> Vec<f64> {
        let len = self.length();
        let mut st = vec![0.0, len];
        let mut s = 0.0;
        for w in self.axis.windows(2) {
            s += w[0].distance(w[1]);
            st.push(s);
        }
        st.extend(self.change_cuts());
        st.retain(|x| (0.0..=len).contains(x));
        st.sort_by(|a, b| a.total_cmp(b));
        st.dedup_by(|a, b| (*a - *b).abs() < 0.05);
        st
    }

    /// Its kerbs and façade lines as they are, to edit them.
    pub fn edges(&self) -> RoadEdges {
        let mut e = RoadEdges::default();
        for s in self.edge_stations() {
            let (p, u) = self.axis_at(s);
            let n = u.perp_left();
            let strips = self.strips_at(s);
            let offs = self.offsets_at(s);
            for left in [true, false] {
                if let Some((k, f, _)) = side_lines(&strips, &offs, left) {
                    let sign = if left { 1.0 } else { -1.0 };
                    let (kerb, facade) = if left {
                        (&mut e.left_kerb, &mut e.left_facade)
                    } else {
                        (&mut e.right_kerb, &mut e.right_facade)
                    };
                    kerb.push(p + n * (sign * k));
                    facade.push(p + n * (sign * f));
                }
            }
        }
        e
    }

    /// Which line each of some lines drawn beside the road is: by its
    /// side of the axis, and on each side the nearer the kerb and the
    /// farther the façade line (one alone: the one it is nearer to now).
    pub fn edges_of(&self, lines: Vec<Vec<Point2>>) -> RoadEdges {
        let now = self.edges();
        let side_of = |l: &[Point2]| -> f64 {
            let v: Vec<f64> = l
                .iter()
                .map(|q| {
                    let (p, u) = self.axis_at(self.station_of(*q));
                    (*q - p).dot(u.perp_left())
                })
                .collect();
            v.iter().sum::<f64>() / v.len().max(1) as f64
        };
        let mean_of = |l: &[Point2]| if l.is_empty() { None } else { Some(side_of(l)) };
        let mut e = RoadEdges::default();
        for left in [true, false] {
            let mut mine: Vec<(f64, Vec<Point2>)> = lines
                .iter()
                .map(|l| (side_of(l), l.clone()))
                .filter(|(d, _)| (*d > 0.0) == left)
                .collect();
            mine.sort_by(|a, b| a.0.abs().total_cmp(&b.0.abs()));
            let (kerb, facade) = if left {
                (&mut e.left_kerb, &mut e.left_facade)
            } else {
                (&mut e.right_kerb, &mut e.right_facade)
            };
            match mine.len() {
                0 => {}
                1 => {
                    let (d, l) = mine.remove(0);
                    let (k, f) = if left {
                        (mean_of(&now.left_kerb), mean_of(&now.left_facade))
                    } else {
                        (mean_of(&now.right_kerb), mean_of(&now.right_facade))
                    };
                    let to_kerb = k.map_or(f64::INFINITY, |k| (k - d).abs());
                    let to_facade = f.map_or(f64::INFINITY, |f| (f - d).abs());
                    if to_kerb <= to_facade {
                        *kerb = l;
                    } else {
                        *facade = l;
                    }
                }
                _ => {
                    *kerb = mine[0].1.clone();
                    *facade = mine[mine.len() - 1].1.clone();
                }
            }
        }
        e
    }

    /// The changes of section that make the road's kerbs and façade lines
    /// follow the lines given (those missing stay as they are): at each
    /// corner of the axis or of a line, the sidewalk as wide as from the
    /// kerb to the façade line, and the strip inside it (parking, a
    /// planting strip, the carriageway) out to the kerb; between corners,
    /// a straight transition. The road's own section stays, so its lanes
    /// stay where they are; a carriageway widened on one side only widens
    /// half on each.
    pub fn changes_from_edges(&self, e: &RoadEdges) -> Vec<SectionChange> {
        let len = self.length();
        let mut st = self.edge_stations();
        for l in e.lines() {
            st.extend(l.iter().map(|q| self.station_of(*q)));
        }
        st.retain(|x| (0.0..=len).contains(x));
        st.sort_by(|a, b| a.total_cmp(b));
        st.dedup_by(|a, b| (*a - *b).abs() < 0.05);
        // The distance from the axis to a line along the normal there.
        let reach = |p: Point2, n: Vec2, line: &[Point2]| -> Option<f64> {
            let mut best: Option<f64> = None;
            for w in line.windows(2) {
                let (a, b) = (w[0], w[1]);
                let d = b - a;
                let den = n.x * d.y - n.y * d.x;
                if den.abs() < 1e-12 {
                    continue;
                }
                let q = a - p;
                let t = (q.x * d.y - q.y * d.x) / den;
                let k = (q.x * n.y - q.y * n.x) / den;
                if (-1e-9..=1.0 + 1e-9).contains(&k) && t > 0.0 {
                    best = Some(best.map_or(t, |x| x.min(t)));
                }
            }
            best
        };
        let round = |v: f64| (v * 100.0).round() / 100.0;
        let mut stations: Vec<(f64, Vec<RoadStrip>)> = Vec::new();
        for s in st {
            let (p, u) = self.axis_at(s);
            let n = u.perp_left();
            let mut strips = self.strips_at(s);
            let offs = self.offsets_at(s);
            for left in [true, false] {
                let Some((k0, f0, inner)) = side_lines(&strips, &offs, left) else {
                    continue;
                };
                let dir = if left { n } else { -n };
                let (kerb, facade) = if left {
                    (&e.left_kerb, &e.left_facade)
                } else {
                    (&e.right_kerb, &e.right_facade)
                };
                let k = reach(p, dir, kerb).unwrap_or(k0);
                let f = reach(p, dir, facade).unwrap_or(f0);
                // The kerb moved: the strip inside it takes it, and what is
                // outside it goes with it; the sidewalk to the façade line.
                let dk = k - k0;
                if let Some(j) = inner {
                    strips[j].width = (strips[j].width + dk).max(MIN_EDGE_STRIP);
                }
                let outer = if left { 0 } else { strips.len() - 1 };
                strips[outer].width = (strips[outer].width + (f - f0) - dk).max(MIN_EDGE_STRIP);
            }
            for x in &mut strips {
                x.width = round(x.width);
            }
            stations.push((s, strips));
        }
        let same = |a: &[RoadStrip], b: &[RoadStrip]| {
            alike(a, b)
                && a.iter()
                    .zip(b)
                    .all(|(x, y)| (x.width - y.width).abs() < 0.005)
        };
        let mut changes = Vec::new();
        let mut prev = self.section.strips.clone();
        let mut prev_s = 0.0;
        for (i, (s, strips)) in stations.into_iter().enumerate() {
            if !same(&prev, &strips) {
                let gradual = i > 0 && alike(&prev, &strips);
                changes.push(SectionChange {
                    at: round(s),
                    transition: if gradual { round(s - prev_s) } else { 0.0 },
                    strips: strips.clone(),
                });
            }
            prev = strips;
            prev_s = s;
        }
        changes
    }
}

/// A path moved to its left by a distance of its own at each point
/// (negative: to its right), mitred at its corners.
pub(super) fn offset_path_by(path: &[Point2], d: &[f64]) -> Option<Vec<Point2>> {
    let n = path.len();
    if n < 2 || d.len() != n {
        return None;
    }
    let dir = |i: usize| (path[i + 1] - path[i]).normalized();
    let mut out = Vec::with_capacity(n);
    for (i, &p) in path.iter().enumerate() {
        let q = match (
            i.checked_sub(1).and_then(dir),
            (i + 1 < n).then(|| dir(i)).flatten(),
        ) {
            (None, Some(u)) | (Some(u), None) => p + u.perp_left() * d[i],
            (Some(a), Some(b)) => {
                let m = (a.perp_left() + b.perp_left()).normalized()?;
                let cos = m.x * a.perp_left().x + m.y * a.perp_left().y;
                p + m * (d[i] / cos.max(0.2))
            }
            (None, None) => return None,
        };
        out.push(q);
    }
    Some(out)
}

/// The strip between two offsets of a path that change along it, each
/// point of the path with its own pair, counter-clockwise.
pub(super) fn strip_by(path: &[Point2], a: &[f64], b: &[f64]) -> Vec<Point2> {
    let (Some(near), Some(mut far)) = (offset_path_by(path, a), offset_path_by(path, b)) else {
        return Vec::new();
    };
    far.reverse();
    let mut out = near;
    out.extend(far);
    out.dedup_by(|p, q| p.distance(*q) < 1e-9);
    if out.len() >= 2 && out[0].distance(out[out.len() - 1]) < 1e-9 {
        out.pop();
    }
    if signed_area(&out) < 0.0 {
        out.reverse();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn road(changes: Vec<SectionChange>) -> Road {
        Road {
            id: RoadId(1),
            guid: GlobalId::new(),
            name: "R".into(),
            level: LevelId(0),
            axis: vec![Point2::new(0.0, 0.0), Point2::new(100.0, 0.0)],
            section: RoadSection::default(),
            top_offset: 0.0,
            profile: Vec::new(),
            changes,
            pavement: MaterialId(0),
            kerb_material: MaterialId(0),
            sidewalk_material: MaterialId(0),
        }
    }

    #[test]
    fn a_sidewalk_widens_over_a_transition() {
        // The parking of both sides becomes sidewalk from 50 m, over 10 m.
        let mut wide = RoadSection::default().strips;
        for s in &mut wide {
            if s.band == RoadBand::Parking {
                s.width = 0.5;
            }
            if s.band == RoadBand::Sidewalk {
                s.width = 4.2;
            }
        }
        let r = road(vec![SectionChange {
            at: 50.0,
            transition: 10.0,
            strips: wide,
        }]);
        let w0 = r.section.width();
        assert!((r.section_at(20.0).width() - w0).abs() < 1e-9);
        assert!((r.section_at(70.0).width() - w0).abs() < 1e-9);
        // Halfway, the parking is between 2,2 and 0,5.
        let mid = r.section_at(45.0);
        let p = mid
            .strips
            .iter()
            .find(|s| s.band == RoadBand::Parking)
            .unwrap();
        assert!((p.width - 1.35).abs() < 1e-9, "{}", p.width);
        assert!(r.change_cuts().contains(&40.0) && r.change_cuts().contains(&50.0));
        // The pieces follow it: the parking ends narrower than it starts.
        let parking: Vec<RoadPiece> = r
            .pieces()
            .into_iter()
            .filter(|x| x.band == RoadBand::Parking && x.span.0 >= 39.9 && x.span.1 <= 50.1)
            .collect();
        assert_eq!(parking.len(), 2);
        let area = signed_area(&parking[0].outline).abs();
        assert!((area - 10.0 * (2.2 + 0.5) / 2.0).abs() < 1e-6, "{area}");
        // Its text, back.
        let back = Road::decode_changes(&r.encode_changes()).unwrap();
        assert_eq!(back, r.changes);
    }

    #[test]
    fn a_change_of_strips_is_sharp() {
        // From 30 m, a single-level street of 8 m.
        let r = road(vec![SectionChange {
            at: 30.0,
            transition: 4.0,
            strips: vec![strip_of(RoadBand::Sidewalk, 8.0)],
        }]);
        assert_eq!(r.strips_at(27.9).len(), r.section.strips.len());
        assert_eq!(r.strips_at(28.1).len(), 1);
        assert!(r.change_cuts().contains(&28.0));
        assert!((r.widest() - r.section.width()).abs() < 1e-9);
    }

    #[test]
    fn sidewalks_reach_the_facade_line() {
        let mut m = Model::new("P");
        let lv = m.add_level("PB", 0.0);
        let mat = m.add_material("Asfalto", "asphalt", [70, 70, 70], CutPattern::Solid);
        let id = m
            .add_road(NewRoad {
                name: "R".into(),
                level: lv,
                axis: vec![Point2::new(0.0, 0.0), Point2::new(100.0, 0.0)],
                section: RoadSection::default(),
                top_offset: 0.0,
                profile: Vec::new(),
                changes: Vec::new(),
                pavement: mat,
                kerb_material: mat,
                sidewalk_material: mat,
            })
            .unwrap();
        let p = Point2::new;
        let plot = |m: &mut Model, outline: Vec<Point2>| {
            m.add_plot(NewPlot {
                name: "P".into(),
                level: lv,
                outline,
                setback: 0.0,
                reference: String::new(),
            })
            .unwrap();
        };
        // To the left, the façade line at 8 m up to 40 m, then at 10 m
        // (with a few centimetres of wandering); to the right, at 9 m.
        plot(
            &mut m,
            vec![p(-5.0, 8.0), p(40.0, 8.0), p(40.0, 30.0), p(-5.0, 30.0)],
        );
        plot(
            &mut m,
            vec![
                p(40.0, 10.0),
                p(70.0, 10.1),
                p(105.0, 10.0),
                p(105.0, 30.0),
                p(40.0, 30.0),
            ],
        );
        plot(
            &mut m,
            vec![
                p(-5.0, -30.0),
                p(105.0, -30.0),
                p(105.0, -9.0),
                p(-5.0, -9.0),
            ],
        );
        let changes = m.sidewalks_to_plots(id).unwrap();
        // Lanes, parking and kerbs as they were; the sidewalks out to the
        // line (the inner edge of a sidewalk is at 5,35 m), from the start.
        assert_eq!(changes.len(), 2, "{changes:?}");
        let first = &changes[0].strips;
        assert!(changes[0].at.abs() < 1e-9);
        assert!((first[0].width - 2.65).abs() < 1e-6, "{first:?}");
        assert!((first[first.len() - 1].width - 3.65).abs() < 1e-6);
        assert!((changes[1].at - 40.0).abs() < 1.0);
        assert!((changes[1].strips[0].width - 4.65).abs() < 1e-6);
        m.edit_road(id, |r| r.changes = changes).unwrap();
        // The lanes stay on the axis; the outline reaches the line.
        let r = m.road(id).unwrap();
        let lanes = |s: f64| {
            r.offsets_at(s)
                .into_iter()
                .filter(|o| o.0 == RoadBand::Carriageway)
                .map(|o| (o.1, o.2))
                .collect::<Vec<_>>()
        };
        assert_eq!(lanes(20.0), lanes(80.0));
        assert!(lanes(80.0)[0].0.abs() < 1e-9 && (lanes(80.0)[1].0 + 3.0).abs() < 1e-9);
        let (l, rr) = r.extent_at(80.0);
        assert!(
            (l - 10.0).abs() < 1e-6 && (rr - 9.0).abs() < 1e-6,
            "{l} {rr}"
        );
    }

    #[test]
    fn kerbs_and_facades_drawn_make_the_changes() {
        let (m, id) = {
            let mut m = Model::new("T");
            let lv = m.add_level("PB", 0.0);
            let mat = m.add_material("A", "asphalt", [60, 60, 60], CutPattern::Solid);
            let id = m
                .add_road(NewRoad {
                    name: "R".into(),
                    level: lv,
                    axis: vec![Point2::new(0.0, 0.0), Point2::new(100.0, 0.0)],
                    section: RoadSection::default(),
                    top_offset: 0.0,
                    profile: Vec::new(),
                    changes: Vec::new(),
                    pavement: mat,
                    kerb_material: mat,
                    sidewalk_material: mat,
                })
                .unwrap();
            (m, id)
        };
        let r = m.road(id).unwrap();
        let e = r.edges();
        assert_eq!(e.lines().len(), 4);
        // As they are: no changes.
        assert!(r.changes_from_edges(&e).is_empty());
        // The left façade line steps back 2 m from 40 to 60 m, with 10 m
        // ramps.
        let f = e.left_facade[0].y;
        let mut moved = e.clone();
        moved.left_facade = vec![
            Point2::new(0.0, f),
            Point2::new(30.0, f),
            Point2::new(40.0, f + 2.0),
            Point2::new(60.0, f + 2.0),
            Point2::new(70.0, f),
            Point2::new(100.0, f),
        ];
        let changes = r.changes_from_edges(&moved);
        let mut r2 = r.clone();
        r2.changes = changes;
        let w0 = r.strips_at(50.0)[0].width;
        assert!((r2.strips_at(50.0)[0].width - (w0 + 2.0)).abs() < 0.011);
        assert!((r2.strips_at(35.0)[0].width - (w0 + 1.0)).abs() < 0.011);
        assert!((r2.strips_at(90.0)[0].width - w0).abs() < 0.011);
        // The kerb did not move: the carriageway is as it was.
        let lanes = |r: &Road, s: f64| r.strips_at(s)[1..r.strips_at(s).len() - 1].to_vec();
        assert_eq!(lanes(&r2, 50.0), lanes(r, 50.0));
        // The right kerb comes 1 m in from 40 to 60 m (a bulb-out): the
        // parking narrows and the sidewalk widens, the façade stays.
        let k = e.right_kerb[0].y;
        let mut bulb = e.clone();
        bulb.right_kerb = vec![
            Point2::new(0.0, k),
            Point2::new(35.0, k),
            Point2::new(40.0, k + 1.0),
            Point2::new(60.0, k + 1.0),
            Point2::new(65.0, k),
            Point2::new(100.0, k),
        ];
        let mut r3 = r.clone();
        r3.changes = r.changes_from_edges(&bulb);
        let mid = r3.strips_at(50.0);
        let own = r.strips_at(50.0);
        let last = mid.len() - 1;
        assert!(
            (mid[last].width - (own[last].width + 1.0)).abs() < 0.011,
            "{mid:?}"
        );
        assert!(
            (mid[last - 2].width - (own[last - 2].width - 1.0)).abs() < 0.011,
            "{mid:?}"
        );
        assert!((r3.edges().right_facade[0].y - e.right_facade[0].y).abs() < 1e-9);
        // The lines told apart by side and distance.
        let back = r.edges_of(vec![
            moved.right_facade.clone(),
            moved.left_kerb.clone(),
            moved.left_facade.clone(),
            moved.right_kerb.clone(),
        ]);
        assert_eq!(back, moved);
    }
}
