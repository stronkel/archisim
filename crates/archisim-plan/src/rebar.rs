//! The reinforcement in plan (E11, G1), as it is drawn on a structure
//! plan at 1:50 or more: one bar of each mesh of footings and slabs, laid
//! across the element with its bent ends turned down into the plane
//! (abatidas) and its label («Ø12 c/15», with «sup.» for the top meshes);
//! the label of each beam along it (bottom, top and stirrups); and the
//! label of each column cut beside it (its bars are drawn by
//! `members::draw_column_rebar`).

use std::fmt::Write as _;

use archisim_core::{LevelId, Model, Point2, RebarKind};
use archisim_geom::rebar::{element_bars, BarRole};

use crate::{lettering, num};

/// The coarsest scale at which a plan marks the reinforcement (metres per
/// millimetre of paper: 1:50).
pub const MARK_K: f64 = 0.05;

/// Height of the labels, paper millimetres.
const LABEL: f64 = 2.0;

/// Colour of the bars and their labels: the ink of a structure plan.
const BAR_INK: &str = "#1d4f91";

/// Draws the marks of the reinforcement of a level's plan.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_rebar_marks(
    s: &mut String,
    model: &Model,
    level: LevelId,
    cut_plane: f64,
    k: f64,
    visible: &dyn Fn(&[Point2]) -> bool,
    footings: bool,
    structure: bool,
    slabs: bool,
) {
    if k > MARK_K || model.reinforcements().next().is_none() {
        return;
    }
    s.push_str("<g id=\"rebar-marks\">\n");
    if footings {
        for f in model.footings_on_level(level) {
            let o = f.outline();
            if visible(&o) {
                mesh_marks(s, model, f.id.0, k);
            }
        }
    }
    if slabs {
        for sl in model.slabs_on_level(level) {
            if visible(&sl.outline) {
                mesh_marks(s, model, sl.id.0, k);
            }
        }
    }
    if structure {
        // Each beam on the plan of its own level (the floor it carries).
        for (b, _) in crate::members::plan_beams(model, level, cut_plane) {
            if b.level != level {
                continue;
            }
            let poly = crate::members::beam_outline(model, b);
            if !poly.is_empty() && visible(&poly) {
                beam_label(s, model, b.id.0, b.start, b.end, k);
            }
        }
        for (c, v) in crate::members::plan_columns(model, level, cut_plane) {
            if v == crate::members::MemberView::Cut {
                column_label(s, model, c.id.0, c.position, k);
            }
        }
    }
    s.push_str("</g>\n");
}

fn label(s: &mut String, text: &str, at: Point2, dir: Point2, k: f64, align: lettering::Align) {
    let h = LABEL * k;
    let _ = writeln!(
        s,
        r#"<path class="rebar-label" d="{}" {}/>"#,
        lettering::text_path(text, at, dir, h, align),
        lettering::paint(h, BAR_INK)
    );
}

/// Reads left to right or bottom to top.
fn readable(d: Point2) -> Point2 {
    if d.x < -1e-9 || (d.x.abs() < 1e-9 && d.y < 0.0) {
        Point2::new(-d.x, -d.y)
    } else {
        d
    }
}

/// One bar of each mesh of a footing or slab, with its label.
fn mesh_marks(s: &mut String, model: &Model, id: u64, k: f64) {
    let groups: Vec<_> = element_bars(model, id)
        .into_iter()
        .filter(|g| g.role == BarRole::Mesh && !g.bars.is_empty())
        .collect();
    let slab = model.rebar_kind(id) == Some(RebarKind::Slab);
    for g in &groups {
        // The middle bar, or for the second mesh of each direction one a
        // little further on so the two do not overlap.
        let top = g.place.starts_with("superior");
        // Across the mesh, a quarter of the way in from one side (the top
        // mesh from the other), clear of what stands in the middle.
        let mids: Vec<Point2> = g
            .bars
            .iter()
            .map(|b| {
                let (p, q) = (b[0], b[b.len() - 1]);
                Point2::new((p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0)
            })
            .collect();
        let (a0, b0) = (g.bars[0][0], g.bars[0][g.bars[0].len() - 1]);
        let along = (Point2::new(b0[0], b0[1]) - Point2::new(a0[0], a0[1]))
            .normalized()
            .unwrap_or(Point2::new(1.0, 0.0));
        let across = |p: Point2| p.x * -along.y + p.y * along.x;
        let (lo, hi) = mids
            .iter()
            .map(|p| across(*p))
            .fold((f64::MAX, f64::MIN), |(l, h), v| (l.min(v), h.max(v)));
        let want = if top {
            hi - (hi - lo) / 4.0
        } else {
            lo + (hi - lo) / 4.0
        };
        let at = (0..mids.len())
            .min_by(|i, j| {
                (across(mids[*i]) - want)
                    .abs()
                    .total_cmp(&(across(mids[*j]) - want).abs())
            })
            .unwrap_or(0);
        let bar = &g.bars[at];
        let line = abatida(bar);
        if line.len() < 2 {
            continue;
        }
        let width = (g.diameter / k).max(0.35) * k;
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="none" stroke="{BAR_INK}" stroke-width="{}" stroke-linejoin="round"{}/>"#,
            open_path(&line),
            num(width),
            if top {
                format!(r#" stroke-dasharray="{} {}""#, num(4.0 * k), num(1.5 * k))
            } else {
                String::new()
            }
        );
        // The label over the middle of its longest stretch.
        let (i, _) = line
            .windows(2)
            .enumerate()
            .map(|(i, w)| (i, w[0].distance(w[1])))
            .fold((0, 0.0), |a, b| if b.1 > a.1 { b } else { a });
        let (a, b) = (line[i], line[i + 1]);
        let Some(d) = (b - a).normalized() else {
            continue;
        };
        let d = readable(d);
        let normal = Point2::new(-d.y, d.x);
        // A quarter of the way along, clear of what stands in the middle.
        let t = match (slab, top) {
            (false, _) => 0.25,
            (true, false) => 0.62,
            (true, true) => 0.38,
        };
        let mid = a + (b - a) * t + normal * (0.8 * k);
        let mut text = g.label.clone();
        if top {
            text = format!("sup. {text}");
        } else if slab {
            text = format!("inf. {text}");
        }
        label(s, &text, mid, d, k, lettering::Align::Centre);
    }
}

/// A bar's axis in plan with its bent ends turned into the plane
/// (abatidas): each stretch that goes up or down is laid out along the
/// perpendicular to the bar, at its length.
fn abatida(bar: &[[f64; 3]]) -> Vec<Point2> {
    let flat: Vec<(Point2, f64)> = bar
        .iter()
        .map(|p| (Point2::new(p[0], p[1]), p[2]))
        .collect();
    // The main direction: the longest stretch in plan.
    let mut dir = None;
    let mut best = 0.0;
    for w in flat.windows(2) {
        let l = w[0].0.distance(w[1].0);
        if l > best {
            best = l;
            dir = (w[1].0 - w[0].0).normalized();
        }
    }
    let Some(dir) = dir else {
        return Vec::new();
    };
    let side = Point2::new(-dir.y, dir.x);
    let mut out = vec![flat[0].0];
    let mut cur = flat[0].0;
    // The first point that is truly where the bar is: the start of its
    // first stretch in plan.
    let mut anchor: Option<(usize, Point2)> = None;
    for (i, w) in flat.windows(2).enumerate() {
        let dp = w[1].0 - w[0].0;
        let dz = w[1].1 - w[0].1;
        if dp.length() < 1e-6 && dz.abs() > 1e-6 {
            // A leg: turned onto the plane, to the side it goes up to.
            cur = cur + side * dz.abs();
        } else {
            if anchor.is_none() {
                anchor = Some((i, w[0].0));
            }
            cur = cur + dp;
        }
        out.push(cur);
    }
    // Moved so the bar's own stretches stay where they are.
    if let Some((i, real)) = anchor {
        let shift = real - out[i];
        for p in &mut out {
            *p = *p + shift;
        }
    }
    out
}

fn beam_label(s: &mut String, model: &Model, id: u64, a: Point2, b: Point2, k: f64) {
    let groups = element_bars(model, id);
    if groups.is_empty() {
        return;
    }
    let get = |place: &str| {
        groups
            .iter()
            .find(|g| g.place == place)
            .map(|g| g.label.clone())
    };
    let mut parts = Vec::new();
    if let Some(t) = get("superior") {
        parts.push(format!("sup. {t}"));
    }
    if let Some(t) = get("inferior") {
        parts.push(format!("inf. {t}"));
    }
    if let Some(t) = get("estribos") {
        parts.push(t);
    }
    let Some(d) = (b - a).normalized() else {
        return;
    };
    let d = readable(d);
    let normal = Point2::new(-d.y, d.x);
    let width = model
        .beam(archisim_core::BeamId(id))
        .and_then(|b| model.member_type(b.member_type))
        .map_or(0.3, |t| t.profile.size().0);
    let mid = a + (b - a) * 0.5 + normal * (width / 2.0 + 1.2 * k);
    label(s, &parts.join(" · "), mid, d, k, lettering::Align::Centre);
}

fn column_label(s: &mut String, model: &Model, id: u64, at: Point2, k: f64) {
    let Some(r) = model.reinforcement(id) else {
        return;
    };
    let size = model
        .column(archisim_core::ColumnId(id))
        .and_then(|c| model.member_type(c.member_type))
        .map_or(0.3, |t| {
            let (w, d) = t.profile.size();
            w.max(d)
        });
    let p = at + Point2::new(size / 2.0 + 1.5 * k, size / 2.0 + 1.0 * k);
    label(
        s,
        &r.summary(),
        p,
        Point2::new(1.0, 0.0),
        k,
        lettering::Align::Left,
    );
}

fn open_path(poly: &[Point2]) -> String {
    let mut d = String::new();
    for (i, p) in poly.iter().enumerate() {
        let _ = write!(
            d,
            "{}{},{} ",
            if i == 0 { "M" } else { "L" },
            num(p.x),
            num(-p.y)
        );
    }
    d
}
