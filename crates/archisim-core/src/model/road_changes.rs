//! Changes of section along a road (U2): from a distance along its axis
//! the road takes other strips across — a sidewalk that widens, parking
//! that ends, a lane that is added —, with a transition of a length over
//! which the strips' widths change gradually (when the two sections have
//! the same strips in the same order; otherwise the change is sharp, in
//! the middle of the transition).
//!
//! Everything else of the section (kerb height, parking mode, build-outs,
//! crossings, materials) is the road's own.

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
}
