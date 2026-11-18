//! Hiding pieces of the streets one by one and letting zebras go (N6,
//! `docs/TERRITORIO.md` §2.9), as Hide It and Move It in Cities: Skylines:
//! a click on a dash, a bay line, a tree pit, a node's sign or a bollard
//! hides it; a click on a zebra of a street makes it a marking of its own,
//! selected to be moved.

use archisim_core::{middle_of, PieceKind, Point2, RoadId};
use archisim_geom::crossroads;

use super::*;

/// What a click on the plan does with the pieces of the streets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiecePick {
    /// Hide the piece under it.
    Hide,
    /// Let the zebra under it go.
    Detach,
}

/// How far from a piece's middle a click still finds it, metres.
const REACH: f64 = 1.0;

/// The piece of a street nearest `p` on a level: its street, what it is
/// and its middle.
pub fn piece_at(
    model: &archisim_core::Model,
    level: archisim_core::LevelId,
    p: Point2,
) -> Option<(RoadId, PieceKind, Point2)> {
    let junctions = crossroads::junctions(model, level);
    let mut best: Option<(f64, RoadId, PieceKind, Point2)> = None;
    let mut consider = |d: f64, road: RoadId, kind: PieceKind, at: Point2| {
        if d < REACH && best.is_none_or(|b| d < b.0) {
            best = Some((d, road, kind, at));
        }
    };
    for r in model.roads_on_level(level) {
        let outline = r.outline();
        if outline.iter().all(|q| q.distance(p) > 60.0)
            && !archisim_core::point_in_polygon(p, &outline)
        {
            continue;
        }
        let gaps = crossroads::marking_gaps(&junctions, r.id);
        for m in r.visible_markings(&gaps) {
            let c = middle_of(&m);
            let d = if archisim_core::point_in_polygon(p, &m) {
                0.0
            } else {
                c.distance(p)
            };
            consider(d, r.id, PieceKind::Marking, c);
        }
        let gaps = crossroads::road_gaps(&junctions, r.id);
        for t in r.visible_tree_pits(&gaps) {
            let c = middle_of(&t);
            consider(c.distance(p), r.id, PieceKind::TreePit, c);
        }
    }
    // The nearest street of a node to one of its pieces.
    let nearest = |j: &crossroads::Junction, q: Point2| {
        j.arms
            .iter()
            .filter_map(|a| model.road(a.road))
            .min_by(|a, b| {
                let d = |r: &archisim_core::Road| r.axis_at(r.station_of(q)).0.distance(q);
                d(a).total_cmp(&d(b))
            })
            .map(|r| r.id)
    };
    for j in &junctions {
        for (post, _, _) in &j.signs {
            if let Some(r) = nearest(j, *post) {
                consider(post.distance(p), r, PieceKind::Sign, *post);
            }
        }
        for part in &j.parts {
            let c = middle_of(&part.outline);
            let small = part.outline.iter().all(|q| q.distance(c) < 0.25);
            if small && part.z1 - part.z0 > 0.5 && part.z1 - part.z0 < 1.5 {
                if let Some(r) = nearest(j, c) {
                    consider(c.distance(p), r, PieceKind::Bollard, c);
                }
            }
        }
    }
    best.map(|(_, r, k, at)| (r, k, at))
}

impl ArchiSimApp {
    /// A click while hiding pieces or letting zebras go.
    pub(super) fn piece_click(&mut self, p: Point2) {
        let level = self.ed.level;
        match self.piece_pick {
            Some(PiecePick::Hide) => match piece_at(self.ed.model(), level, p) {
                Some((road, kind, at)) => {
                    let res = self
                        .ed
                        .transact("Ocultar pieza", |m| m.hide_piece(road, kind, at));
                    self.report(
                        res,
                        &format!(
                            "{} oculto. En la calle, «Mostrar las piezas ocultas» lo devuelve.",
                            kind.label()
                        ),
                    );
                }
                None => self.say("Aquí no hay ninguna pieza de calle que ocultar."),
            },
            Some(PiecePick::Detach) => {
                let found = self.ed.model().roads_on_level(level).find_map(|r| {
                    let (at, off) = r.place_of(p);
                    let (l, rr) = r.extent_at(at);
                    let on = off <= l && -off <= rr;
                    r.section
                        .crossings
                        .iter()
                        .find(|c| on && (c.at - at).abs() <= c.width / 2.0 + 0.5)
                        .map(|c| (r.id, c.at))
                });
                match found {
                    Some((road, at)) => {
                        let mut made = None;
                        let res = self.ed.transact("Soltar paso de peatones", |m| {
                            made = Some(m.detach_crossing(road, at)?);
                            Ok::<(), archisim_core::ModelError>(())
                        });
                        if let (Ok(()), Some(k)) = (&res, made) {
                            self.ed.selection = [Element::Marking(k)].into_iter().collect();
                            self.piece_pick = None;
                        }
                        self.report(
                            res,
                            "Paso de peatones suelto: ya es una marca vial, elegida para moverla (arrastrar, flechas o Mover).",
                        );
                    }
                    None => self.say("Aquí no hay un paso de peatones de una calle."),
                }
            }
            None => {}
        }
    }
}
