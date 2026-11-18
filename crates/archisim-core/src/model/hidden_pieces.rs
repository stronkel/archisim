//! Pieces of a street hidden one by one, and zebras let go of their street
//! (N6, `docs/TERRITORIO.md` §2.9), as Hide It and Move It do in Cities:
//! Skylines: a dash of a lane line, a bay of the parking, a tree pit, the
//! sign or a bollard of a node are made by the street; each can be left
//! out by hand. They are kept by where they are along the street and
//! across it, so they follow it when it moves.

use super::*;

/// What a hidden piece of a street is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PieceKind {
    /// A piece of its own markings (a dash, a line, a bay, a zebra's bar).
    Marking,
    /// A tree pit of its sidewalk.
    TreePit,
    /// The sign of a node, with its post.
    Sign,
    /// A bollard of a node.
    Bollard,
}

impl PieceKind {
    pub const ALL: [PieceKind; 4] = [
        PieceKind::Marking,
        PieceKind::TreePit,
        PieceKind::Sign,
        PieceKind::Bollard,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PieceKind::Marking => "Marca",
            PieceKind::TreePit => "Alcorque",
            PieceKind::Sign => "Señal",
            PieceKind::Bollard => "Bolardo",
        }
    }

    fn code(self) -> &'static str {
        match self {
            PieceKind::Marking => "m",
            PieceKind::TreePit => "t",
            PieceKind::Sign => "s",
            PieceKind::Bollard => "b",
        }
    }

    fn of_code(c: &str) -> Option<PieceKind> {
        PieceKind::ALL.into_iter().find(|k| k.code() == c)
    }
}

/// A piece hidden: what it is, how far along the street's axis and how far
/// to its left (negative: right).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HiddenPiece {
    pub kind: PieceKind,
    pub at: f64,
    pub off: f64,
}

/// How near a piece's middle must be to a hidden one's place to be it,
/// metres.
pub const HIDE_REACH: f64 = 0.4;

/// The middle of a polygon (of its corners).
pub fn middle_of(poly: &[Point2]) -> Point2 {
    let n = poly.len().max(1) as f64;
    let (x, y) = poly.iter().fold((0.0, 0.0), |(x, y), q| (x + q.x, y + q.y));
    Point2::new(x / n, y / n)
}

impl Road {
    /// Where `p` is on the street: how far along its axis, and how far to
    /// its left.
    pub fn place_of(&self, p: Point2) -> (f64, f64) {
        let s = self.station_of(p);
        let (q, u) = axis_point(&self.axis, s);
        (s, -(p - q).cross(u))
    }

    /// Whether a piece of this kind with its middle at `p` is hidden.
    pub fn hides(&self, kind: PieceKind, p: Point2) -> bool {
        if self.hidden.is_empty() {
            return false;
        }
        let (at, off) = self.place_of(p);
        self.hidden
            .iter()
            .any(|h| h.kind == kind && (h.at - at).hypot(h.off - off) < HIDE_REACH)
    }

    /// Its own markings but those over `gaps` and those hidden.
    pub fn visible_markings(&self, gaps: &[(f64, f64)]) -> Vec<Vec<Point2>> {
        let mut out = self.markings_except(gaps);
        if !self.hidden.is_empty() {
            out.retain(|m| !self.hides(PieceKind::Marking, middle_of(m)));
        }
        out
    }

    /// Its tree pits but those over `gaps` and those hidden.
    pub fn visible_tree_pits(&self, gaps: &[(f64, f64)]) -> Vec<Vec<Point2>> {
        let mut out = self.tree_pits_except(gaps);
        if !self.hidden.is_empty() {
            out.retain(|m| !self.hides(PieceKind::TreePit, middle_of(m)));
        }
        out
    }

    /// Its hidden pieces as text (for the IFC): `kind at off|…`.
    pub fn encode_hidden(&self) -> String {
        self.hidden
            .iter()
            .map(|h| format!("{} {} {}", h.kind.code(), h.at, h.off))
            .collect::<Vec<_>>()
            .join("|")
    }

    pub fn decode_hidden(text: &str) -> Option<Vec<HiddenPiece>> {
        let mut out = Vec::new();
        for t in text.split('|').filter(|t| !t.trim().is_empty()) {
            let f: Vec<&str> = t.split_whitespace().collect();
            if f.len() != 3 {
                return None;
            }
            out.push(HiddenPiece {
                kind: PieceKind::of_code(f[0])?,
                at: f[1].parse().ok()?,
                off: f[2].parse().ok()?,
            });
        }
        Some(out)
    }
}

impl Model {
    /// Hides the piece of a street of this kind whose middle is at `p`.
    pub fn hide_piece(
        &mut self,
        road: RoadId,
        kind: PieceKind,
        p: Point2,
    ) -> Result<(), ModelError> {
        let r = self.road(road).ok_or(ModelError::UnknownElement(road.0))?;
        if r.hides(kind, p) {
            return Ok(());
        }
        let (at, off) = r.place_of(p);
        self.edit_road(road, |r| r.hidden.push(HiddenPiece { kind, at, off }))
    }

    /// Shows again every piece of a street hidden by hand. How many.
    pub fn show_pieces(&mut self, road: RoadId) -> Result<usize, ModelError> {
        let n = self
            .road(road)
            .ok_or(ModelError::UnknownElement(road.0))?
            .hidden
            .len();
        self.edit_road(road, |r| r.hidden.clear())?;
        Ok(n)
    }

    /// Lets a zebra crossing of a street go: its bars are hidden on the
    /// street, which keeps its lowered kerbs where they were, and it
    /// becomes a road marking of its own, where it was, that can be moved,
    /// turned or changed.
    pub fn detach_crossing(&mut self, road: RoadId, at: f64) -> Result<MarkingId, ModelError> {
        let r = self
            .road(road)
            .cloned()
            .ok_or(ModelError::UnknownElement(road.0))?;
        let c = r
            .section
            .crossings
            .iter()
            .copied()
            .min_by(|a, b| (a.at - at).abs().total_cmp(&(b.at - at).abs()))
            .filter(|c| (c.at - at).abs() < c.width.max(2.0))
            .ok_or(ModelError::InvalidShape("no hay paso de peatones ahí"))?;
        // Its bars on the street: those of the street's markings over it.
        let bars: Vec<Point2> = r
            .markings_except(&[])
            .iter()
            .map(|m| middle_of(m))
            .filter(|p| (r.place_of(*p).0 - c.at).abs() <= c.width / 2.0 + 0.05)
            .filter(|p| !r.hides(PieceKind::Marking, *p))
            .collect();
        if bars.is_empty() {
            return Err(ModelError::InvalidShape(
                "ese paso de peatones ya está suelto",
            ));
        }
        let (q, u) = axis_point(&r.axis, c.at);
        let n = u.perp_left();
        let (left, right) = r.extent_at(c.at);
        // Across the carriageway (and parking): to the kerbs.
        let offs = r.offsets_at(c.at);
        let kerb_l = offs
            .iter()
            .filter(|o| o.0 == RoadBand::Kerb && o.1 > 0.0)
            .map(|o| o.1)
            .fold(left, f64::min);
        let kerb_r = offs
            .iter()
            .filter(|o| o.0 == RoadBand::Kerb && o.1 < 0.0)
            .map(|o| -o.1)
            .fold(right, f64::min);
        let material = r
            .section
            .markings
            .unwrap_or_else(|| self.marking_material([245, 245, 240]));
        let (w, dash, gap) = MarkingKind::Zebra.defaults();
        let id = self.add_marking(NewMarking {
            level: r.level,
            kind: MarkingKind::Zebra,
            path: vec![q - n * kerb_r, q + n * kerb_l],
            width: w,
            dash,
            gap,
            depth: c.width,
            material,
        })?;
        let hidden: Vec<HiddenPiece> = bars
            .iter()
            .map(|p| {
                let (at, off) = r.place_of(*p);
                HiddenPiece {
                    kind: PieceKind::Marking,
                    at,
                    off,
                }
            })
            .collect();
        self.edit_road(road, |x| x.hidden.extend(hidden))?;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn street() -> (Model, RoadId) {
        let mut m = Model::new("Piezas");
        let lv = m.add_level("PB", 0.0);
        let mat = m.add_material("Asfalto", "asphalt", [60, 60, 60], CutPattern::Solid);
        let mut section = RoadSection {
            markings: Some(mat),
            ..Default::default()
        };
        section.crossings.push(Crossing {
            at: 30.0,
            width: 4.0,
        });
        let id = m
            .add_road(NewRoad {
                name: "Calle".into(),
                level: lv,
                axis: vec![Point2::new(0.0, 0.0), Point2::new(100.0, 0.0)],
                section,
                top_offset: 0.0,
                profile: Vec::new(),
                changes: Vec::new(),
                pavement: mat,
                kerb_material: mat,
                sidewalk_material: mat,
            })
            .unwrap();
        (m, id)
    }

    /// A dash hidden goes, the rest stay; moving the street, it stays
    /// hidden; the zebra let go is a marking of its own.
    #[test]
    fn hide_a_dash_and_let_a_zebra_go() {
        let (mut m, id) = street();
        let all = m.road(id).unwrap().visible_markings(&[]);
        let one = middle_of(&all[3]);
        m.hide_piece(id, PieceKind::Marking, one).unwrap();
        let r = m.road(id).unwrap();
        assert_eq!(r.visible_markings(&[]).len(), all.len() - 1);
        let text = r.encode_hidden();
        assert_eq!(crate::Road::decode_hidden(&text).unwrap(), r.hidden);
        m.edit_road(id, |r| {
            for q in &mut r.axis {
                q.y += 10.0;
            }
        })
        .unwrap();
        assert_eq!(
            m.road(id).unwrap().visible_markings(&[]).len(),
            all.len() - 1
        );
        assert_eq!(m.show_pieces(id).unwrap(), 1);
        // The zebra.
        let bars = |m: &Model| {
            m.road(id)
                .unwrap()
                .visible_markings(&[])
                .iter()
                .filter(|x| (middle_of(x).x - 30.0).abs() < 2.1)
                .count()
        };
        assert!(bars(&m) > 0);
        let k = m.detach_crossing(id, 30.5).unwrap();
        // The street keeps the crossing for its lowered kerbs; its bars go.
        assert_eq!(m.road(id).unwrap().section.crossings.len(), 1);
        assert_eq!(bars(&m), 0);
        let z = m.marking(k).unwrap();
        assert_eq!(z.kind, MarkingKind::Zebra);
        assert!((z.depth - 4.0).abs() < 1e-9);
        assert!((z.path[0].x - 30.0).abs() < 1e-9);
        assert!(m.detach_crossing(id, 30.0).is_err());
    }
}
