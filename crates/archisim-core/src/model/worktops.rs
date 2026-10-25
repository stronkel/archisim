//! Worktop runs: base kitchen modules side by side, in an L or back to
//! back under one worktop (C2).
//!
//! Each base module («Módulo bajo de cocina», `props:base_unit`) is made
//! with its own worktop. When the worktops of several of them touch, at
//! the same height and of the same material, they are one run: the
//! modules are drawn without their own top and the run has a single one
//! over all of them, with the openings of their sinks. It is worked out
//! from where the modules are, never stored: moving a module out of a run
//! gives it back its own top.

use std::collections::BTreeMap;

use crate::family::{ParamScope, Value};
use crate::ids::{LevelId, MaterialId, ObjectId};
use crate::math::Point2;

use super::Model;

/// Gap between two worktops that still counts as touching (m).
const TOUCH: f64 = 0.005;
/// Shortest length two worktops must touch along: a corner meeting a
/// corner is not a run (m).
const SHARED: f64 = 0.05;

/// Several base modules under one worktop.
#[derive(Debug, Clone, PartialEq)]
pub struct WorktopRun {
    pub level: LevelId,
    /// The modules, by id: the first one carries the worktop.
    pub members: Vec<ObjectId>,
    pub material: MaterialId,
    /// Height of the top face above the level's elevation, where the
    /// modules stand (m).
    pub top: f64,
    pub thickness: f64,
    /// The worktop of each module in plan: its four corners.
    pub pieces: Vec<[Point2; 4]>,
    /// The openings of the sinks in plan.
    pub sinks: Vec<[Point2; 4]>,
    /// The upstands (copetes) along the back edges of the modules that
    /// carry one and have no other module behind them: the edge, from
    /// end to end with the worktop on its left, and the height over the
    /// worktop.
    pub upstands: Vec<([Point2; 2], f64)>,
    /// Length measured along the modules' fronts: the sum of their widths
    /// (m), as a worktop is priced.
    pub length: f64,
    /// Area of the worktop less its sinks, before the pieces are joined
    /// (they only touch): the sum of the modules' tops (m²).
    pub area: f64,
}

/// A base module as a worktop sees it.
#[derive(Debug, Clone)]
struct Top {
    id: ObjectId,
    level: LevelId,
    material: MaterialId,
    top: f64,
    width: f64,
    piece: [Point2; 4],
    /// The piece a little larger, to see what it touches.
    reach: [Point2; 4],
    sink: Option<[Point2; 4]>,
    /// Its back edge (worktop on its left) and its upstand's height.
    back: [Point2; 2],
    upstand: f64,
    /// A strip just behind its back edge, to see whether another module
    /// is there.
    behind: [Point2; 4],
}

impl Model {
    /// A base kitchen module's worktop, if `id` is one.
    fn module_top(&self, id: ObjectId) -> Option<Top> {
        let o = self.objects.get(&id)?;
        let ty = self.object_types.get(&o.object_type)?;
        if self.family_prop(ty.family).as_deref() != Some("base_unit") {
            return None;
        }
        let mut values = self.family_defaults(ty.family, ParamScope::Type).ok()?;
        values.extend(ty.values.clone());
        let number = |k: &str, d: f64| match values.get(k) {
            Some(Value::Number(n)) => *n,
            _ => d,
        };
        let (w, d, h) = (
            number("width", 0.6),
            number("depth", 0.6),
            number("height", 0.9),
        );
        // What the worktop overhangs in front and behind, and its upstand.
        let front = number("overhang", 0.0).max(0.0);
        let back = number("overhang_back", 0.0).max(0.0);
        let upstand = number("upstand", 0.0).max(0.0);
        let material = match values.get("top") {
            Some(Value::Material(m)) if self.materials.contains_key(&MaterialId(*m)) => {
                MaterialId(*m)
            }
            _ => return None,
        };
        let sink_unit = values
            .get("unit")
            .and_then(|v| v.as_text())
            .and_then(|t| crate::props::choice_number("unit", t))
            == Some(crate::props::SINK_UNIT);
        let rect = |x0: f64, y0: f64, x1: f64, y1: f64| {
            [
                o.to_plan(x0, y0),
                o.to_plan(x1, y0),
                o.to_plan(x1, y1),
                o.to_plan(x0, y1),
            ]
        };
        let sink = if sink_unit {
            crate::props::sink_hole(w, d).map(|(a, b)| rect(a[0], a[1], b[0], b[1]))
        } else {
            None
        };
        Some(Top {
            id,
            level: o.level,
            material,
            top: o.base_offset + h,
            width: w,
            piece: rect(-w / 2.0, -back, w / 2.0, d + front),
            reach: rect(
                -w / 2.0 - TOUCH,
                -back - TOUCH,
                w / 2.0 + TOUCH,
                d + front + TOUCH,
            ),
            sink,
            back: [o.to_plan(-w / 2.0, -back), o.to_plan(w / 2.0, -back)],
            upstand,
            behind: rect(
                -w / 2.0 + TOUCH,
                -back - 4.0 * TOUCH,
                w / 2.0 - TOUCH,
                -back,
            ),
        })
    }

    /// The worktop runs of the model: base modules whose worktops touch,
    /// at the same height and of the same material, two or more.
    pub fn worktop_runs(&self) -> Vec<WorktopRun> {
        self.worktops()
            .into_iter()
            .filter(|r| r.members.len() >= 2)
            .collect()
    }

    /// Every worktop of the model: the runs, and the top of each base
    /// module on its own (a run of one), as they are measured.
    pub fn worktops(&self) -> Vec<WorktopRun> {
        let tops: Vec<Top> = self
            .objects
            .keys()
            .filter_map(|id| self.module_top(*id))
            .collect();
        runs_of(&tops, 1)
    }

    /// The run a base module is in, if it is in one.
    pub fn worktop_run_of(&self, id: ObjectId) -> Option<WorktopRun> {
        let me = self.module_top(id)?;
        // Only the modules of its level, at its height, can be in its run.
        let tops: Vec<Top> = self
            .objects
            .values()
            .filter(|o| o.level == me.level)
            .filter_map(|o| self.module_top(o.id))
            .filter(|t| (t.top - me.top).abs() < 0.005 && t.material == me.material)
            .collect();
        runs_of(&tops, 2)
            .into_iter()
            .find(|r| r.members.contains(&id))
    }
}

/// Whether two module tops are part of one worktop.
fn joined(a: &Top, b: &Top) -> bool {
    a.level == b.level
        && a.material == b.material
        && (a.top - b.top).abs() < 0.005
        && overlap(&a.reach, &b.piece) >= TOUCH * SHARED
}

/// Groups the tops that touch into runs of `fewest` or more.
fn runs_of(tops: &[Top], fewest: usize) -> Vec<WorktopRun> {
    let n = tops.len();
    let mut group: Vec<usize> = (0..n).collect();
    fn root(group: &mut [usize], mut i: usize) -> usize {
        while group[i] != i {
            group[i] = group[group[i]];
            i = group[i];
        }
        i
    }
    for i in 0..n {
        for j in i + 1..n {
            if joined(&tops[i], &tops[j]) {
                let (a, b) = (root(&mut group, i), root(&mut group, j));
                if a != b {
                    group[a.max(b)] = a.min(b);
                }
            }
        }
    }
    let mut by_root: BTreeMap<usize, Vec<&Top>> = BTreeMap::new();
    for (i, top) in tops.iter().enumerate() {
        let r = root(&mut group, i);
        by_root.entry(r).or_default().push(top);
    }
    let mut runs: Vec<WorktopRun> = by_root
        .into_values()
        .filter(|g| g.len() >= fewest)
        .map(|mut g| {
            g.sort_by_key(|t| t.id);
            let first = g[0];
            let area = |r: &[Point2; 4]| crate::math::signed_area(r).abs();
            WorktopRun {
                level: first.level,
                members: g.iter().map(|t| t.id).collect(),
                material: first.material,
                top: first.top,
                thickness: crate::props::WORKTOP,
                pieces: g.iter().map(|t| t.piece).collect(),
                sinks: g.iter().filter_map(|t| t.sink).collect(),
                upstands: g
                    .iter()
                    .filter(|t| t.upstand > 1e-3)
                    .filter(|t| {
                        !g.iter()
                            .any(|o| o.id != t.id && overlap(&t.behind, &o.piece) > 1e-6)
                    })
                    .map(|t| (t.back, t.upstand))
                    .collect(),
                length: g.iter().map(|t| t.width).sum(),
                area: g
                    .iter()
                    .map(|t| area(&t.piece) - t.sink.as_ref().map_or(0.0, area))
                    .sum(),
            }
        })
        .collect();
    runs.sort_by_key(|r| r.members[0]);
    runs
}

/// Area of the overlap of two convex quadrilaterals (Sutherland–Hodgman).
fn overlap(a: &[Point2; 4], b: &[Point2; 4]) -> f64 {
    let ccw = |p: &[Point2; 4]| -> Vec<Point2> {
        if crate::math::signed_area(p) < 0.0 {
            p.iter().rev().copied().collect()
        } else {
            p.to_vec()
        }
    };
    let clip_by = ccw(b);
    let mut poly = ccw(a);
    for i in 0..clip_by.len() {
        let (p, q) = (clip_by[i], clip_by[(i + 1) % clip_by.len()]);
        let side = |x: Point2| (q.x - p.x) * (x.y - p.y) - (q.y - p.y) * (x.x - p.x);
        let input = std::mem::take(&mut poly);
        for j in 0..input.len() {
            let (c, d) = (input[j], input[(j + 1) % input.len()]);
            let (sc, sd) = (side(c), side(d));
            if sc >= 0.0 {
                poly.push(c);
            }
            if (sc >= 0.0) != (sd >= 0.0) {
                let t = sc / (sc - sd);
                poly.push(Point2::new(c.x + (d.x - c.x) * t, c.y + (d.y - c.y) * t));
            }
        }
        if poly.is_empty() {
            return 0.0;
        }
    }
    crate::math::signed_area(&poly).abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::family::Value;
    use crate::ids::ObjectTypeId;
    use crate::NewObject;

    /// A model with the base module family and a type of it.
    pub(crate) fn kitchen() -> (Model, LevelId, ObjectTypeId, ObjectTypeId) {
        let mut m = Model::new("cocina");
        let l = m.add_level("PB", 0.0);
        let top = m.add_material(
            "Encimera",
            "stone",
            [230, 230, 225],
            crate::CutPattern::Solid,
        );
        let wood = m.add_material(
            "Madera de pino",
            "wood",
            [200, 170, 120],
            crate::CutPattern::Solid,
        );
        let fam = m.system_family("Módulo bajo de cocina").expect("family");
        let mut values = m.family_defaults(fam, ParamScope::Type).expect("defaults");
        values.insert("top".into(), Value::Material(top.0));
        values.insert("mat".into(), Value::Material(wood.0));
        let drawers = m
            .add_object_type("Cajonera 60", fam, values.clone())
            .expect("type");
        values.insert("unit".into(), Value::Text("Fregadero".into()));
        values.insert("width".into(), Value::Number(0.8));
        let sink = m
            .add_object_type("Fregadero 80", fam, values)
            .expect("type");
        (m, l, drawers, sink)
    }

    fn place(m: &mut Model, l: LevelId, t: ObjectTypeId, x: f64, y: f64, rot: f64) -> ObjectId {
        m.add_object(NewObject {
            object_type: t,
            level: l,
            position: Point2::new(x, y),
            rotation: rot,
            base_offset: 0.0,
        })
        .expect("object")
    }

    #[test]
    fn modules_side_by_side_make_one_run() {
        let (mut m, l, drawers, sink) = kitchen();
        let a = place(&mut m, l, drawers, 0.3, 0.0, 0.0);
        let b = place(&mut m, l, sink, 1.0, 0.0, 0.0);
        let c = place(&mut m, l, drawers, 1.7, 0.0, 0.0);
        // Away from them: on its own, with its own top.
        let d = place(&mut m, l, drawers, 5.0, 0.0, 0.0);
        let runs = m.worktop_runs();
        assert_eq!(runs.len(), 1, "{runs:?}");
        let r = &runs[0];
        assert_eq!(r.members, vec![a, b, c]);
        assert!((r.length - 2.0).abs() < 1e-9, "{}", r.length);
        assert_eq!(r.sinks.len(), 1);
        assert!((r.top - 0.9).abs() < 1e-9);
        // 2 × 0.6 less the sink's opening.
        let hole = (0.8 - 0.12) * (0.6 - 0.22);
        assert!((r.area - (2.0 * 0.6 - hole)).abs() < 1e-6, "{}", r.area);
        assert_eq!(m.worktop_run_of(b).map(|r| r.members), Some(vec![a, b, c]));
        assert!(m.worktop_run_of(d).is_none());
        // Measured: the run and the module on its own.
        let all = m.worktops();
        assert_eq!(all.len(), 2);
        assert!((all.iter().map(|r| r.length).sum::<f64>() - 2.6).abs() < 1e-9);
    }

    #[test]
    fn an_l_and_back_to_back_are_runs_too() {
        let (mut m, l, drawers, _) = kitchen();
        // Along the x axis, backs on y = 0; then up the wall x = 0, turned
        // a quarter, its end against the first one's front.
        let a = place(&mut m, l, drawers, 0.3, 0.0, 0.0);
        let b = place(&mut m, l, drawers, 0.0, 0.9, -std::f64::consts::FRAC_PI_2);
        assert_eq!(m.worktop_run_of(a).map(|r| r.members), Some(vec![a, b]));
        // Back to back: an island of two.
        let c = place(&mut m, l, drawers, 5.0, 0.0, 0.0);
        let d = place(&mut m, l, drawers, 5.0, 0.0, std::f64::consts::PI);
        assert_eq!(m.worktop_run_of(c).map(|r| r.members), Some(vec![c, d]));
        // Only corner to corner: not a run.
        let e = place(&mut m, l, drawers, 9.0, 0.0, 0.0);
        let f = place(&mut m, l, drawers, 9.6, 0.6, 0.0);
        assert!(m.worktop_run_of(e).is_none() && m.worktop_run_of(f).is_none());
    }

    #[test]
    fn different_heights_or_tops_do_not_join() {
        let (mut m, l, drawers, _) = kitchen();
        let a = place(&mut m, l, drawers, 0.3, 0.0, 0.0);
        let b = place(&mut m, l, drawers, 0.9, 0.0, 0.0);
        m.edit_object(b, |o| o.base_offset = 0.1).unwrap();
        assert!(m.worktop_run_of(a).is_none());
    }

    #[test]
    fn overhang_and_upstand() {
        let (mut m, l, drawers, _) = kitchen();
        let fam = m.object_type(drawers).unwrap().family;
        let mut values = m.object_type(drawers).unwrap().values.clone();
        values.insert("overhang".into(), Value::Number(0.3));
        values.insert("upstand".into(), Value::Number(0.1));
        let bar = m.add_object_type("Isla con vuelo", fam, values).unwrap();
        // An island of two back to back: one with its bar overhang, the
        // other with an upstand that no module stands behind... except
        // the first one: none.
        let a = place(&mut m, l, bar, 0.0, 0.0, 0.0);
        let b = place(&mut m, l, drawers, 0.0, 0.0, std::f64::consts::PI);
        let r = m.worktop_run_of(a).expect("a run");
        assert_eq!(r.members, vec![a, b]);
        assert!(r.upstands.is_empty(), "{:?}", r.upstands);
        // 0.6 × 0.9 for the bar module, 0.6 × 0.6 for the other.
        assert!((r.area - (0.6 * 0.9 + 0.36)).abs() < 1e-9, "{}", r.area);
        // Against a wall, side by side: the upstand all along.
        let c = place(&mut m, l, bar, 5.0, 0.0, 0.0);
        let d = place(&mut m, l, bar, 5.6, 0.0, 0.0);
        let r = m.worktop_run_of(c).expect("a run");
        assert_eq!(r.members, vec![c, d]);
        assert_eq!(r.upstands.len(), 2);
        assert!((r.upstands[0].1 - 0.1).abs() < 1e-9);
        let along: f64 = r.upstands.iter().map(|(e, _)| e[0].distance(e[1])).sum();
        assert!((along - 1.2).abs() < 1e-9);
    }
}
