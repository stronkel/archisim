//! The plumbing diagrams (esquemas de fontanería), A3 landscape pages (SVG
//! in millimetres) drawn from the calculation of `archisim-mep::water`:
//!
//! - FO-01, the vertical diagram (esquema vertical): the levels one over
//!   another, the source (meter, tank or heater) with its pressure, the
//!   risers with each stretch between floors and its diameter, and on each
//!   floor a box for every branch that leaves the risers (a dwelling, a
//!   wet room): what it feeds, the diameter where it starts and the least
//!   pressure at its taps. Cold water on the left half, hot on the right.
//! - one page per level with taps (distribución): the tree of every
//!   branch of the level, stretch by stretch with its name and diameter,
//!   down to each tap with the pressure left at it.
//!
//! The diameters written are the calculated ones; where the pipe drawn is
//! smaller, both are written («Ø20→26») in red, and so are the taps short
//! of pressure as drawn.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use archisim_core::{Appliance, DeviceId, LevelId, Model, Point2, ServiceSystem};
use archisim_mep::water::{self as wa, Network, Source, Water, TAP_PRESSURE};

use crate::lettering::{paint, text_path, text_width, Align};
use crate::structure_sheets::{
    centred, line, n, page, rect, text, BLOCK_H, BLOCK_W, GREY, H, INK, MARGIN, MARK, RULE, W,
};

const STROKE: f64 = 0.35;
/// The dashes of the hot water.
const HOT_DASH: &str = "2.2 1.1";
/// Where the drawing ends above the title block.
const BOTTOM: f64 = H - MARGIN - BLOCK_H - 6.0;

/// The pages: the vertical diagram and one per level with taps. Empty if
/// there is no water network.
pub fn pages(model: &Model) -> Vec<String> {
    let _font = crate::lettering::use_font(model.graphics.font);
    let w = wa::water(model);
    if w.networks.iter().all(|n| n.stretches.is_empty()) {
        return Vec::new();
    }
    let trees: Vec<Tree> = w.networks.iter().map(Tree::of).collect();
    let mut out = vec![vertical_page(model, &w, &trees)];
    let mut k = 2;
    for l in model.levels() {
        if trees.iter().any(|t| !t.roots_on(l.id).is_empty()) {
            out.push(level_page(model, &w, &trees, l.id, &format!("FO-{k:02}")));
            k += 1;
        }
    }
    out
}

/// A network as a tree of stretches.
struct Tree<'a> {
    net: &'a Network,
    children: Vec<Vec<usize>>,
    /// The taps whose last stretch is each one.
    taps_on: Vec<Vec<usize>>,
    /// The stretches on the way to a riser (and the risers).
    trunk: Vec<bool>,
}

impl<'a> Tree<'a> {
    fn of(net: &'a Network) -> Self {
        let k = net.stretches.len();
        let mut children = vec![Vec::new(); k];
        for (i, s) in net.stretches.iter().enumerate() {
            if let Some(p) = s.parent {
                children[p].push(i);
            }
        }
        let mut taps_on = vec![Vec::new(); k];
        for (t, tap) in net.taps.iter().enumerate() {
            if let Some(last) = tap.path.last() {
                taps_on[*last].push(t);
            }
        }
        // Children come after their parent: from the end, upwards.
        let mut trunk = vec![false; k];
        for i in (0..k).rev() {
            if net.stretches[i].riser.is_some() || children[i].iter().any(|c| trunk[*c]) {
                trunk[i] = true;
            }
        }
        Tree {
            net,
            children,
            taps_on,
            trunk,
        }
    }

    /// The branches that leave the trunk (or the source) on a level, or
    /// come down from another level.
    fn roots_on(&self, level: LevelId) -> Vec<usize> {
        let st = &self.net.stretches;
        (0..st.len())
            .filter(|i| !self.trunk[*i] && st[*i].level == level)
            .filter(|i| match st[*i].parent {
                None => true,
                Some(p) => self.trunk[p] || st[p].level != level,
            })
            .collect()
    }

    /// The taps after a stretch.
    fn taps_after(&self, s: usize) -> Vec<usize> {
        (0..self.net.taps.len())
            .filter(|t| self.net.taps[*t].path.contains(&s))
            .collect()
    }

    /// The boxes of a branch for the vertical diagram: the branch itself,
    /// or, if it is large and splits, each of the branches it splits into
    /// (with the taps on the split, if any, as one more), split again in
    /// the same way.
    fn boxes(&self, root: usize) -> Vec<Group> {
        let mut cur = root;
        while self.children[cur].len() == 1 && self.taps_on[cur].is_empty() {
            cur = self.children[cur][0];
        }
        let parts = self.children[cur].len() + usize::from(!self.taps_on[cur].is_empty());
        if self.net.stretches[root].appliances <= 6 || parts < 2 {
            return vec![Group {
                head: root,
                taps: self.taps_after(root),
            }];
        }
        // Each part split again while it is large.
        let mut out: Vec<Group> = self.children[cur]
            .iter()
            .flat_map(|c| self.boxes(*c))
            .collect();
        if !self.taps_on[cur].is_empty() {
            out.push(Group {
                head: cur,
                taps: self.taps_on[cur].clone(),
            });
        }
        out
    }

    /// The trunk stretch a branch hangs from, if any.
    fn trunk_of(&self, s: usize) -> Option<usize> {
        let mut p = self.net.stretches[s].parent;
        while let Some(i) = p {
            if self.trunk[i] {
                return Some(i);
            }
            p = self.net.stretches[i].parent;
        }
        None
    }
}

/// A box of the vertical diagram.
struct Group {
    head: usize,
    taps: Vec<usize>,
}

fn hot(net: &Network) -> bool {
    net.system == ServiceSystem::HotWater
}

fn dash_line(s: &mut String, a: (f64, f64), b: (f64, f64), w: f64, colour: &str, dashed: bool) {
    if !dashed {
        line(s, a, b, w, colour);
        return;
    }
    let _ = writeln!(
        s,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{colour}" stroke-width="{}" stroke-dasharray="{HOT_DASH}"/>"#,
        n(a.0),
        n(a.1),
        n(b.0),
        n(b.1),
        n(w)
    );
}

/// A text written upwards, its baseline on `x`, starting at `y`.
fn text_up(s: &mut String, t: &str, x: f64, y: f64, h: f64, colour: &str) {
    if t.trim().is_empty() {
        return;
    }
    let _ = writeln!(
        s,
        r#"<path d="{}" {}/>"#,
        text_path(t, Point2::new(x, -y), Point2::new(0.0, 1.0), h, Align::Left),
        paint(h, colour)
    );
}

/// «Ø26», or «Ø20→26» if the pipe drawn is smaller; and whether it is.
fn diameter(st: &wa::Stretch) -> (String, bool) {
    let d = (st.drawn.0 * 1000.0).round();
    let z = (st.sized.0 * 1000.0).round();
    if d + 0.5 < z {
        (format!("Ø{d:.0}→{z:.0}"), true)
    } else {
        (format!("Ø{z:.0}"), false)
    }
}

fn short(a: Appliance) -> &'static str {
    match a {
        Appliance::Wc => "Inodoro",
        other => other.name(),
    }
}

fn dec(v: f64, d: usize) -> String {
    format!("{v:.d$}").replace('.', ",")
}

fn frame(s: &mut String, model: &Model, code: &str, title: &str) {
    rect(
        s,
        MARGIN,
        MARGIN,
        W - 2.0 * MARGIN,
        H - 2.0 * MARGIN,
        0.5,
        INK,
    );
    let (x, y) = (W - MARGIN - BLOCK_W, H - MARGIN - BLOCK_H);
    rect(s, x, y, BLOCK_W, BLOCK_H, 0.35, INK);
    line(
        s,
        (x + BLOCK_W - 30.0, y),
        (x + BLOCK_W - 30.0, y + BLOCK_H),
        0.25,
        INK,
    );
    text(s, &model.project_name, x + 4.0, y + 7.0, 3.5, INK);
    text(s, title, x + 4.0, y + 15.0, 3.0, INK);
    text(
        s,
        "Fontanería (CTE DB-HS 4) · sin escala",
        x + 4.0,
        y + 22.5,
        2.5,
        GREY,
    );
    centred(s, code, x + BLOCK_W - 15.0, y + 17.0, 6.0, INK);
}

/// The legend and the data of the calculation, under the drawing.
fn legend(s: &mut String, w: &Water) {
    let y = H - MARGIN - BLOCK_H + 4.0;
    let x = MARGIN + 6.0;
    line(s, (x, y), (x + 12.0, y), STROKE, INK);
    text(s, "Agua fría", x + 14.0, y + 1.0, 2.4, INK);
    dash_line(s, (x + 40.0, y), (x + 52.0, y), STROKE, INK, true);
    text(s, "Agua caliente sanitaria", x + 54.0, y + 1.0, 2.4, INK);
    text(
        s,
        &format!(
            "Presión en el contador {} kPa · {} · simultaneidad {} · v ≤ {} m/s · +{} % de pérdidas locales",
            dec(w.pressure, 0),
            w.material.name(),
            if w.dwelling {
                "UNE 149201 (viviendas)"
            } else {
                "Qt/√(n−1)"
            },
            dec(wa::DESIGN_SPEED, 1),
            dec(wa::FITTINGS * 100.0, 0)
        ),
        x,
        y + 7.0,
        2.2,
        GREY,
    );
    text(
        s,
        &format!(
            "Diámetros exteriores calculados (mm). En rojo: tubería dibujada menor (dibujado→calculado) y grifos por debajo de {TAP_PRESSURE:.0} kPa con lo dibujado."
        ),
        x,
        y + 11.0,
        2.2,
        GREY,
    );
    text(
        s,
        "Presión mínima en grifos 100 kPa y en calentadores 150 kPa; máxima 500 kPa (HS 4, 2.1.3).",
        x,
        y + 15.0,
        2.2,
        GREY,
    );
}

/// The levels with something of the networks, bottom first, and the
/// height of each on the page.
fn level_heights(model: &Model, trees: &[Tree]) -> Vec<(LevelId, f64)> {
    let mut used: Vec<LevelId> = Vec::new();
    for t in trees {
        for st in &t.net.stretches {
            used.push(st.level);
        }
        if let Source::Device(d, _) = t.net.source {
            if let Some(dev) = model.device(d) {
                used.push(dev.level);
            }
        }
        if let Source::End(l, _) = t.net.source {
            used.push(l);
        }
    }
    let levels: Vec<LevelId> = model
        .levels()
        .iter()
        .map(|l| l.id)
        .filter(|l| used.contains(l))
        .collect();
    let (top, bottom) = (MARGIN + 46.0, BOTTOM - 22.0);
    let k = levels.len();
    if k <= 1 {
        return levels.into_iter().map(|l| (l, bottom)).collect();
    }
    let step = ((bottom - top) / (k - 1) as f64).min(60.0);
    // Few levels: in the middle of the page.
    let lift = ((bottom - top) - step * (k - 1) as f64) / 2.0;
    levels
        .into_iter()
        .enumerate()
        .map(|(i, l)| (l, bottom - lift - step * i as f64))
        .collect()
}

fn vertical_page(model: &Model, w: &Water, trees: &[Tree]) -> String {
    let mut s = String::new();
    frame(&mut s, model, "FO-01", "Esquema vertical de fontanería");
    text(
        &mut s,
        "ESQUEMA VERTICAL DE FONTANERÍA",
        MARGIN + 6.0,
        MARGIN + 10.0,
        4.5,
        INK,
    );
    let heights = level_heights(model, trees);
    let y_of = |l: LevelId| {
        heights
            .iter()
            .find(|h| h.0 == l)
            .map_or(BOTTOM - 22.0, |h| h.1)
    };
    // The floor lines across the page, with their names.
    for (l, y) in &heights {
        let Some(level) = model.level(*l) else {
            continue;
        };
        let _ = writeln!(
            s,
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{RULE}" stroke-width="0.25" stroke-dasharray="4 2"/>"#,
            n(MARGIN + 4.0),
            n(*y),
            n(W - MARGIN - 4.0),
            n(*y)
        );
        text(&mut s, &level.name, MARGIN + 5.0, y - 1.5, 2.6, INK);
        text(
            &mut s,
            &format!(
                "{}{} m",
                if level.elevation >= 0.0 { "+" } else { "" },
                dec(level.elevation, 2)
            ),
            MARGIN + 5.0,
            y + 3.5,
            2.2,
            GREY,
        );
    }
    let halves = [
        (MARGIN + 50.0, W / 2.0 + 6.0),
        (W / 2.0 + 18.0, W - MARGIN - 6.0),
    ];
    for (t, tree) in trees.iter().enumerate().take(2) {
        let (x0, x1) = halves[if hot(tree.net) { 1 } else { t.min(1) }];
        network_half(&mut s, model, tree, x0, x1, &y_of);
    }
    legend(&mut s, w);
    page(&s)
}

/// One network on the vertical diagram, between `x0` and `x1`.
fn network_half(
    s: &mut String,
    model: &Model,
    tree: &Tree,
    x0: f64,
    x1: f64,
    y_of: &dyn Fn(LevelId) -> f64,
) {
    let net = tree.net;
    let dashed = hot(net);
    let title = if dashed {
        "AGUA CALIENTE SANITARIA"
    } else {
        "AGUA FRÍA"
    };
    text(s, title, x0, MARGIN + 18.0, 3.0, INK);
    // A column for each riser.
    let mut risers: Vec<DeviceId> = net.stretches.iter().filter_map(|s| s.riser).collect();
    risers.sort();
    risers.dedup();
    let col_of = |d: Option<DeviceId>| {
        let i = d
            .and_then(|d| risers.iter().position(|r| *r == d))
            .unwrap_or(0);
        x0 + 6.0 + 9.0 * i as f64
    };
    // The source, under its floor.
    let (src_level, src_name) = match &net.source {
        Source::Device(d, _) => (
            model.device(*d).map(|d| d.level),
            wa::source_name(model, net),
        ),
        Source::End(l, _) => (Some(*l), "Extremo de la red".to_string()),
    };
    let y_src = src_level.map_or(BOTTOM - 22.0, y_of);
    let xs = col_of(risers.first().copied());
    let pipe = |y: f64| y - 4.0;
    let sy = y_src + 9.0;
    if dashed {
        rect(s, xs - 3.5, sy - 3.0, 7.0, 6.0, STROKE, INK);
        centred(s, "ACS", xs, sy + 1.0, 2.0, INK);
    } else {
        let _ = writeln!(
            s,
            r#"<circle cx="{}" cy="{}" r="3" fill="none" stroke="{INK}" stroke-width="{STROKE}"/>"#,
            n(xs),
            n(sy)
        );
        centred(s, "C", xs, sy + 1.0, 2.2, INK);
    }
    text(
        s,
        &format!("{src_name} · {} kPa", dec(net.source_pressure, 0)),
        xs + 5.0,
        sy + 1.0,
        2.3,
        INK,
    );
    dash_line(s, (xs, sy - 3.0), (xs, pipe(y_src)), STROKE, INK, dashed);
    // The trunk: risers up the columns, the rest named by the source.
    let mut by_source: Vec<String> = Vec::new();
    for (i, st) in net.stretches.iter().enumerate() {
        if !tree.trunk[i] {
            continue;
        }
        let (d, red) = diameter(st);
        let colour = if red { MARK } else { INK };
        match st.riser {
            Some(r) => {
                let x = col_of(Some(r));
                let below = st.parent.map_or(y_src, |p| y_of(net.stretches[p].level));
                let (ya, yb) = (pipe(below), pipe(y_of(st.level)));
                dash_line(s, (x, ya), (x, yb), 0.5, INK, dashed);
                // Along the riser, on its left.
                let label = format!("{} {d}", st.name);
                let len = text_width(&label, 2.1);
                text_up(s, &label, x - 0.9, (ya + yb + len) / 2.0, 2.1, colour);
            }
            None => by_source.push(format!("{} {d}", st.name)),
        }
    }
    if !by_source.is_empty() {
        text(s, &by_source.join(" · "), xs + 5.0, sy + 5.0, 2.1, GREY);
    }
    // The boxes of each floor.
    let mut by_level: BTreeMap<i64, (LevelId, Vec<Group>)> = BTreeMap::new();
    for l in model.levels() {
        let roots = tree.roots_on(l.id);
        if roots.is_empty() {
            continue;
        }
        let groups: Vec<Group> = roots.iter().flat_map(|r| tree.boxes(*r)).collect();
        by_level.insert((l.elevation * 1000.0) as i64, (l.id, groups));
    }
    let start = col_of(risers.last().copied()) + 16.0;
    for (level, groups) in by_level.values() {
        let y = y_of(*level);
        let k = groups.len().max(1) as f64;
        let gap = 3.0;
        let bw = ((x1 - start - gap * (k - 1.0)) / k).clamp(18.0, 44.0);
        let hpx = (bw / 44.0).clamp(0.7, 1.0);
        for (i, g) in groups.iter().enumerate() {
            let bx = start + i as f64 * (bw + gap);
            if bx + bw > x1 + 1.0 {
                // No room: say how many are left.
                text(
                    s,
                    &format!("+{} ramales más", groups.len() - i),
                    bx.min(x1 - 20.0),
                    pipe(y) - 2.0,
                    2.2,
                    GREY,
                );
                break;
            }
            let from = col_of(
                tree.trunk_of(g.head)
                    .and_then(|t| net.stretches[t].riser)
                    .or(risers.first().copied()),
            );
            let xc = bx + bw / 2.0;
            // The pipe from the riser along the floor, and up to the box.
            dash_line(s, (from, pipe(y)), (xc, pipe(y)), STROKE, INK, dashed);
            // What it feeds.
            let mut kinds: BTreeMap<Appliance, usize> = BTreeMap::new();
            for t in &g.taps {
                *kinds.entry(net.taps[*t].kind).or_default() += 1;
            }
            let rows = kinds.len() as f64;
            let rh = 3.1 * hpx;
            let bh = 5.0 + rh * (rows + 1.0) + 1.0;
            let top = pipe(y) - 3.0 - bh;
            dash_line(s, (xc, pipe(y)), (xc, top + bh), STROKE, INK, dashed);
            rect(s, bx, top, bw, bh, 0.3, INK);
            let st = &net.stretches[g.head];
            let (d, red) = diameter(st);
            text(
                s,
                &format!("{} {d}", st.name),
                bx + 1.2,
                top + 3.6,
                2.3 * hpx,
                if red { MARK } else { INK },
            );
            line(s, (bx, top + 5.0), (bx + bw, top + 5.0), 0.15, RULE);
            for (r, (a, c)) in kinds.iter().enumerate() {
                text(
                    s,
                    &format!("{c} {}", short(*a)),
                    bx + 1.2,
                    top + 5.0 + rh * (r as f64 + 0.85),
                    2.1 * hpx,
                    INK,
                );
            }
            let worst = g
                .taps
                .iter()
                .map(|t| &net.taps[*t])
                .min_by(|a, b| a.sized_pressure.total_cmp(&b.sized_pressure));
            if let Some(t) = worst {
                let short_drawn = g.taps.iter().any(|t| net.taps[*t].pressure < TAP_PRESSURE);
                text(
                    s,
                    &format!("P mín. {} kPa", dec(t.sized_pressure, 0)),
                    bx + 1.2,
                    top + 5.0 + rh * (rows + 0.85),
                    2.1 * hpx,
                    if t.sized_pressure < TAP_PRESSURE || short_drawn {
                        MARK
                    } else {
                        GREY
                    },
                );
            }
        }
    }
    if !net.loose.is_empty() {
        text(
            s,
            &format!(
                "{} aparato{} sin unir a esta red",
                net.loose.len(),
                if net.loose.len() == 1 { "" } else { "s" }
            ),
            x0,
            MARGIN + 23.0,
            2.3,
            MARK,
        );
    }
}

/// A tree laid out: each stretch with its x, its depth and where its
/// children's bar is; each tap with its x.
struct Layout {
    x: BTreeMap<usize, f64>,
    depth: BTreeMap<usize, usize>,
    taps: Vec<(usize, f64, usize)>,
    deepest: usize,
}

/// Lays out the branches from `roots`, a leaf every `step` from `x`.
fn lay_out(tree: &Tree, roots: &[usize], x: &mut f64, step: f64) -> Layout {
    fn walk(tree: &Tree, s: usize, d: usize, x: &mut f64, step: f64, out: &mut Layout) -> f64 {
        let mut xs = Vec::new();
        for t in &tree.taps_on[s] {
            out.taps.push((*t, *x, d + 1));
            xs.push(*x);
            *x += step;
        }
        for c in &tree.children[s] {
            if tree.trunk[*c] {
                continue;
            }
            xs.push(walk(tree, *c, d + 1, x, step, out));
        }
        let me = if xs.is_empty() {
            let v = *x;
            *x += step;
            v
        } else {
            (xs[0] + xs[xs.len() - 1]) / 2.0
        };
        out.x.insert(s, me);
        out.depth.insert(s, d);
        out.deepest = out.deepest.max(d + 1);
        me
    }
    let mut out = Layout {
        x: BTreeMap::new(),
        depth: BTreeMap::new(),
        taps: Vec::new(),
        deepest: 0,
    };
    for r in roots {
        walk(tree, *r, 0, x, step, &mut out);
        *x += step * 0.8;
    }
    out
}

/// How many leaves the branches from `roots` have.
fn leaves(tree: &Tree, roots: &[usize]) -> usize {
    fn count(tree: &Tree, s: usize) -> usize {
        let kids: usize = tree.children[s]
            .iter()
            .filter(|c| !tree.trunk[**c])
            .map(|c| count(tree, *c))
            .sum();
        let own = tree.taps_on[s].len();
        (kids + own).max(1)
    }
    roots.iter().map(|r| count(tree, *r)).sum()
}

fn level_page(model: &Model, w: &Water, trees: &[Tree], level: LevelId, code: &str) -> String {
    let mut s = String::new();
    let name = model.level(level).map_or(String::new(), |l| l.name.clone());
    frame(
        &mut s,
        model,
        code,
        &format!("Distribución de fontanería · {name}"),
    );
    text(
        &mut s,
        &format!("DISTRIBUCIÓN DE FONTANERÍA · {}", name.to_uppercase()),
        MARGIN + 6.0,
        MARGIN + 10.0,
        4.5,
        INK,
    );
    let parts: Vec<(&Tree, Vec<usize>)> = trees
        .iter()
        .map(|t| (t, t.roots_on(level)))
        .filter(|(_, r)| !r.is_empty())
        .collect();
    let total: f64 = parts
        .iter()
        .map(|(t, r)| leaves(t, r) as f64 + 0.8 * r.len() as f64 + 2.0)
        .sum();
    let (left, right) = (MARGIN + 10.0, W - MARGIN - 8.0);
    let step = ((right - left) / total.max(1.0)).clamp(2.5, 20.0);
    let hp = (step / 8.0).clamp(0.65, 1.0);
    let top = MARGIN + 30.0;
    // Few taps: in the middle of the page.
    let mut x = left + ((right - left) - step * total).max(0.0) / 2.0 + step;
    for (tree, roots) in &parts {
        let net = tree.net;
        let dashed = hot(net);
        let x_start = x;
        let lay = lay_out(tree, roots, &mut x, step);
        text(
            &mut s,
            if dashed {
                "AGUA CALIENTE SANITARIA"
            } else {
                "AGUA FRÍA"
            },
            x_start - step * 0.5,
            MARGIN + 18.0,
            3.0,
            INK,
        );
        let dy = ((BOTTOM - 34.0 - top) / (lay.deepest.max(1) as f64)).min(24.0);
        let leaf_y = top + dy * lay.deepest as f64 + 10.0;
        let y_at = |d: usize| top + dy * d as f64;
        for (st_i, sx) in &lay.x {
            let st = &net.stretches[*st_i];
            let d = lay.depth[st_i];
            let (ya, yb) = (y_at(d), y_at(d + 1));
            dash_line(&mut s, (*sx, ya), (*sx, yb), STROKE, INK, dashed);
            // The bar to its children.
            let mut kids: Vec<f64> = tree.children[*st_i]
                .iter()
                .filter_map(|c| lay.x.get(c).copied())
                .collect();
            kids.extend(
                lay.taps
                    .iter()
                    .filter(|t| tree.taps_on[*st_i].contains(&t.0))
                    .map(|t| t.1),
            );
            if kids.len() > 1 {
                let (a, b) = kids
                    .iter()
                    .fold((f64::MAX, f64::MIN), |m, v| (m.0.min(*v), m.1.max(*v)));
                dash_line(&mut s, (a, yb), (b, yb), STROKE, INK, dashed);
            } else if let Some(k) = kids.first() {
                dash_line(&mut s, (*sx, yb), (*k, yb), STROKE, INK, dashed);
            }
            let (dt, red) = diameter(st);
            let label = format!("{} {dt}", st.name);
            let h = 1.8 * hp;
            let len = text_width(&label, h);
            text_up(
                &mut s,
                &label,
                sx + 0.9 + h,
                yb - ((yb - ya - len) / 2.0).max(0.6),
                h,
                if red { MARK } else { INK },
            );
            if d == 0 {
                // Where it comes from.
                let from = match st.parent {
                    Some(p) => format!("de {}", net.stretches[p].name),
                    None => format!("de {}", wa::source_name(model, net)),
                };
                centred(&mut s, &from, *sx, ya - 1.5, 2.0 * hp, GREY);
            }
        }
        for (t, tx, d) in &lay.taps {
            let tap = &net.taps[*t];
            dash_line(&mut s, (*tx, y_at(*d)), (*tx, leaf_y), STROKE, INK, dashed);
            // The tap: a small triangle.
            let _ = writeln!(
                s,
                r#"<path d="M {} {} L {} {} L {} {} Z" fill="{INK}"/>"#,
                n(tx - 1.0),
                n(leaf_y),
                n(tx + 1.0),
                n(leaf_y),
                n(*tx),
                n(leaf_y + 1.8)
            );
            let label = format!("{} · {} kPa", short(tap.kind), dec(tap.sized_pressure, 0));
            let h = 1.9 * hp;
            let len = text_width(&label, h);
            let red = tap.sized_pressure < TAP_PRESSURE || tap.pressure < TAP_PRESSURE;
            text_up(
                &mut s,
                &label,
                tx + h * 0.4,
                leaf_y + 3.0 + len,
                h,
                if red { MARK } else { INK },
            );
        }
        x += step * 2.0;
    }
    legend(&mut s, w);
    page(&s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{DeviceKind, NewDevice, NewObject, NewRun};

    /// A house of two floors: meter and heater downstairs, a cold and a
    /// hot riser, a bathroom on each floor and a kitchen sink downstairs.
    fn house() -> Model {
        let mut m = archisim_library::new_project("Casa");
        let l0 = m.levels()[0].id;
        let l1 = m.add_level("Planta 1", 3.0);
        let wc = archisim_library::add_object_entry(&mut m, "Inodoro").unwrap();
        let basin = archisim_library::add_object_entry(&mut m, "Lavabo 60").unwrap();
        let shower = archisim_library::add_object_entry(&mut m, "Plato de ducha 90×90").unwrap();
        for l in [l0, l1] {
            for (t, x) in [(wc, 2.0), (basin, 3.5), (shower, 5.0)] {
                m.add_object(NewObject {
                    level: l,
                    object_type: t,
                    position: Point2::new(x, 0.8),
                    rotation: 0.0,
                    base_offset: 0.0,
                })
                .unwrap();
            }
        }
        let device = |m: &mut Model, kind, name: &str, x: f64, y: f64, height: f64| {
            m.add_device(NewDevice {
                level: l0,
                kind,
                name: name.into(),
                position: Point2::new(x, y),
                rotation: 0.0,
                elevation: if kind == DeviceKind::Riser { 0.0 } else { 0.3 },
                width: 0.3,
                depth: 0.2,
                height,
            })
            .unwrap();
        };
        device(&mut m, DeviceKind::WaterMeter, "Contador", 0.0, 0.0, 0.4);
        device(&mut m, DeviceKind::WaterHeater, "Termo", 0.0, -1.0, 0.8);
        device(&mut m, DeviceKind::Riser, "Montante AF", 1.0, 0.0, 3.0);
        device(&mut m, DeviceKind::Riser, "Montante ACS", 1.0, -0.2, 3.0);
        let run = |m: &mut Model, level, system, path: Vec<Point2>| {
            m.add_run(NewRun {
                level,
                system,
                path,
                elevation: 0.3,
                width: 0.016,
                height: 0.016,
                slope: 0.0,
            })
            .unwrap();
        };
        let cold = ServiceSystem::ColdWater;
        let hot = ServiceSystem::HotWater;
        run(
            &mut m,
            l0,
            cold,
            vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)],
        );
        run(
            &mut m,
            l0,
            cold,
            vec![Point2::new(0.0, 0.0), Point2::new(0.0, -1.0)],
        );
        run(
            &mut m,
            l0,
            hot,
            vec![
                Point2::new(0.0, -1.0),
                Point2::new(1.0, -1.0),
                Point2::new(1.0, -0.2),
            ],
        );
        for l in [l0, l1] {
            let ps: Vec<(Point2, bool)> = m
                .appliances_on(l)
                .iter()
                .map(|a| (a.point, a.kind.hot()))
                .collect();
            run(
                &mut m,
                l,
                cold,
                vec![Point2::new(1.0, 0.0), Point2::new(6.0, 0.0)],
            );
            run(
                &mut m,
                l,
                hot,
                vec![Point2::new(1.0, -0.2), Point2::new(6.0, -0.2)],
            );
            for (p, takes_hot) in ps {
                run(&mut m, l, cold, vec![Point2::new(p.x, 0.0), p]);
                if takes_hot {
                    let q = Point2::new(p.x + 0.05, p.y);
                    run(&mut m, l, hot, vec![Point2::new(q.x, -0.2), q]);
                }
            }
        }
        m
    }

    #[test]
    fn a_house_draws_its_plumbing() {
        let m = house();
        let w = wa::water(&m);
        assert!(!w.networks.is_empty());
        for net in &w.networks {
            for (i, st) in net.stretches.iter().enumerate() {
                if let Some(p) = st.parent {
                    assert!(p < i, "a stretch hangs from an earlier one");
                }
            }
        }
        let pages = pages(&m);
        assert!(pages.len() >= 2, "the vertical diagram and a level");
        assert!(pages.iter().all(|p| p.starts_with("<svg")));
        assert!(
            pages[0].contains("stroke-dasharray=\"2.2 1.1\""),
            "hot water"
        );
        if let Ok(dir) = std::env::var("ARCHISIM_FONTANERIA_OUT") {
            std::fs::create_dir_all(&dir).unwrap();
            for (i, p) in pages.iter().enumerate() {
                std::fs::write(format!("{dir}/fontaneria-{:02}.svg", i + 1), p).unwrap();
            }
        }
    }

    #[test]
    fn no_water_no_pages() {
        let m = Model::new("Vacío");
        assert!(pages(&m).is_empty());
    }
}
