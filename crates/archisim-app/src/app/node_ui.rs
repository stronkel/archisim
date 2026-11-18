//! The nodes of the streets (N1, `docs/TERRITORIO.md` §2): the tool Nudo
//! picks the crossing (or the free end of a street) under a click and its
//! properties change it — what it is (crossing, raised, shared surface,
//! continuous, bend, dead end, no node), its height and main street, each
//! arm's setback, sign and zebra crossing, and each corner's radius,
//! shape, sidewalk and bump-out. The arms' ends and the corners have
//! handles in plan.

use archisim_core::{
    ArmRef, ArmSign, CornerSettings, CornerShape, IslandPaint, LaneLink, Model, NodeKey, NodeKind,
    NodeSettings, PaintedIsland, Point2, Turns,
};
use archisim_geom::crossroads::{self, Arm, Junction};
use eframe::egui::{self, Color32, ComboBox, Id, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};

use super::props::{self, group, row};
use super::widgets::SwitchExt;
use super::*;

/// How far from a node's centre a click still picks it, metres.
const PICK_REACH: f64 = 15.0;
/// How near a handle a press grabs it, screen points.
const HANDLE_REACH: f32 = 10.0;
/// The handles' colour.
const HANDLE: Color32 = Color32::from_rgb(230, 120, 40);

/// What a click of the tool Nudo does (N3).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum NodeMode {
    /// Pick a node (and drag its handles).
    #[default]
    Pick,
    /// Connect lanes by hand: a lane coming in, then one going out (the
    /// lane coming in picked, if it is).
    Link(Option<(ArmRef, usize)>),
    /// Draw a painted island: its corners so far, and how it is painted.
    Island(Vec<Point2>, IslandPaint),
}

/// The colours of the lanes' connections: worked out and by hand.
const LANE_WORKED: Color32 = Color32::from_rgb(40, 120, 220);
const LANE_BY_HAND: Color32 = Color32::from_rgb(230, 120, 40);

/// Where an arm goes, as a point of the compass.
fn heading(a: &Arm) -> &'static str {
    // 0 is east, counter-clockwise; north is +y.
    let t = a.dir.y.atan2(a.dir.x).to_degrees().rem_euclid(360.0);
    const NAMES: [&str; 8] = [
        "este", "noreste", "norte", "noroeste", "oeste", "suroeste", "sur", "sureste",
    ];
    NAMES[(((t + 22.5) / 45.0).floor() as usize) % 8]
}

/// An arm for Antonio: its street and where it goes.
fn arm_name(m: &Model, a: &Arm) -> String {
    let name = m.road(a.road).map_or("calle", |r| r.name.as_str());
    format!("{name}, hacia el {}", heading(a))
}

/// The names of a node's streets.
pub fn node_title(m: &Model, key: &NodeKey) -> String {
    let mut names: Vec<String> = key
        .road_ids()
        .filter_map(|r| m.road(r).map(|x| x.name.clone()))
        .collect();
    names.dedup();
    match names.len() {
        0 => "Nudo".into(),
        1 if key.roads.len() == 1 => format!("Final de {}", names[0]),
        1 => format!("Nudo de {}", names[0]),
        _ => {
            let last = names.pop().unwrap_or_default();
            format!("Nudo de {} y {last}", names.join(", "))
        }
    }
}

/// What explains each kind of node.
fn kind_note(k: NodeKind) -> &'static str {
    match k {
        NodeKind::Crossing => "Las calles se unen con sus esquinas y bordillos.",
        NodeKind::Raised => {
            "La calzada sube a la cota de la acera, con una rampa de 2 m en cada calle."
        }
        NodeKind::Shared => {
            "Plataforma única: calzada a la cota de la acera, con su pavimento, sin bordillos y con bolardos en las esquinas."
        }
        NodeKind::Continuous => {
            "Una calle sigue en la otra sin cruce: bordillos y aceras rectos de una sección a la otra, en 6 m por cada lado."
        }
        NodeKind::Bend => {
            "Una curva de la misma calle: la esquina de dentro con su radio y la de fuera alrededor del mismo centro, con las aceras siguiendo la curva."
        }
        NodeKind::DeadEnd => {
            "Fondo de saco: la calle acaba en un círculo de giro con su bordillo y su acera."
        }
        NodeKind::Open => "Las calles se cruzan sin unirse (como un paso a distinto nivel).",
    }
}

/// A small drawing of a kind of node, for its card: streets in grey, the
/// sidewalks' edges in ink.
fn paint_kind(p: &egui::Painter, r: Rect, k: NodeKind, ink: Color32) {
    let road = Color32::from_gray(200);
    let raised = Color32::from_gray(160);
    let paving = Color32::from_rgb(232, 222, 200);
    let c = r.center();
    let w = r.width().min(r.height());
    let half = w * 0.16;
    let reach = w * 0.45;
    let bar = |p: &egui::Painter, a: Pos2, b: Pos2, col: Color32| {
        p.line_segment([a, b], Stroke::new(half * 2.0, col));
    };
    let thin = Stroke::new(1.0, ink);
    match k {
        NodeKind::Crossing | NodeKind::Raised | NodeKind::Shared | NodeKind::Open => {
            bar(
                p,
                c - Vec2::new(reach, 0.0),
                c + Vec2::new(reach, 0.0),
                road,
            );
            if k == NodeKind::Open {
                // One over the other: a gap round the upper one.
                p.line_segment(
                    [c - Vec2::new(0.0, reach), c - Vec2::new(0.0, half * 1.6)],
                    Stroke::new(half * 2.0, road),
                );
                p.line_segment(
                    [c + Vec2::new(0.0, half * 1.6), c + Vec2::new(0.0, reach)],
                    Stroke::new(half * 2.0, road),
                );
                for y in [-1.0, 1.0] {
                    p.line_segment(
                        [
                            c + Vec2::new(-reach, y * half),
                            c + Vec2::new(reach, y * half),
                        ],
                        thin,
                    );
                }
                return;
            }
            bar(
                p,
                c - Vec2::new(0.0, reach),
                c + Vec2::new(0.0, reach),
                road,
            );
            let square = Rect::from_center_size(c, Vec2::splat(half * 2.0));
            match k {
                NodeKind::Raised => {
                    p.rect_filled(square, 0.0, raised);
                }
                NodeKind::Shared => {
                    p.rect_filled(square.expand(half * 0.6), 2.0, paving);
                    for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                        p.circle_filled(c + Vec2::new(dx, dy) * half * 1.4, 1.5, ink);
                    }
                }
                _ => {}
            }
        }
        NodeKind::Continuous => {
            // A wide street going on into a narrow one.
            let pts = vec![
                c + Vec2::new(-reach, -half * 1.3),
                c + Vec2::new(-half, -half * 1.3),
                c + Vec2::new(half, -half * 0.7),
                c + Vec2::new(reach, -half * 0.7),
                c + Vec2::new(reach, half * 0.7),
                c + Vec2::new(half, half * 0.7),
                c + Vec2::new(-half, half * 1.3),
                c + Vec2::new(-reach, half * 1.3),
            ];
            p.add(Shape::convex_polygon(pts, road, thin));
        }
        NodeKind::Bend => {
            let centre = c + Vec2::new(-reach, reach);
            let steps = 12;
            for rad in [reach * 1.4 - half * 2.0, reach * 1.4] {
                let pts: Vec<Pos2> = (0..=steps)
                    .map(|i| {
                        let t = -std::f32::consts::FRAC_PI_2 * i as f32 / steps as f32;
                        centre + Vec2::new(t.cos(), t.sin()) * rad
                    })
                    .collect();
                p.add(Shape::line(pts, thin));
            }
            let mid: Vec<Pos2> = (0..=steps)
                .map(|i| {
                    let t = -std::f32::consts::FRAC_PI_2 * i as f32 / steps as f32;
                    centre + Vec2::new(t.cos(), t.sin()) * (reach * 1.4 - half)
                })
                .collect();
            p.add(Shape::line(mid, Stroke::new(half * 1.8, road)));
        }
        NodeKind::DeadEnd => {
            bar(p, c - Vec2::new(reach, 0.0), c + Vec2::new(half, 0.0), road);
            p.circle(c + Vec2::new(half, 0.0), half * 2.0, road, thin);
        }
    }
}

/// A card for a kind of node: its drawing and its name. Whether it was
/// clicked.
fn kind_card(ui: &mut Ui, k: NodeKind, on: bool) -> bool {
    let size = Vec2::new(78.0, 70.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let visuals = ui.visuals();
    let ink = visuals.text_color();
    let frame = if on {
        visuals.selection.bg_fill
    } else if resp.hovered() {
        visuals.widgets.hovered.bg_fill
    } else {
        visuals.widgets.inactive.bg_fill
    };
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 6.0, frame);
    let picture = Rect::from_min_size(rect.min + Vec2::new(4.0, 4.0), Vec2::new(70.0, 46.0));
    p.rect_filled(picture, 4.0, Color32::WHITE);
    paint_kind(&p, picture.shrink(3.0), k, Color32::from_gray(60));
    p.text(
        Pos2::new(rect.center().x, rect.max.y - 9.0),
        egui::Align2::CENTER_CENTER,
        k.label(),
        egui::FontId::proportional(11.0),
        ink,
    );
    resp.on_hover_text(kind_note(k)).clicked()
}

/// A handle of the picked node being dragged (N1).
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGrab {
    key: NodeKey,
    what: Grab,
}

#[derive(Debug, Clone, PartialEq)]
enum Grab {
    /// An arm's end: dragging along it changes its setback.
    Setback(Arm),
    /// A corner's middle: dragging along its bisector changes its radius.
    Radius {
        from: archisim_core::ArmRef,
        apex: Point2,
        bis: archisim_core::Vec2,
        half: f64,
    },
}

impl Grab {
    /// The value the handle takes at `p`.
    fn value_at(&self, p: Point2) -> f64 {
        match self {
            Grab::Setback(a) => (p - a.origin).dot(a.dir).max(0.5),
            Grab::Radius {
                apex, bis, half, ..
            } => crossroads::corner_radius_at(*apex, *bis, *half, p),
        }
    }
}

/// The usual radius of a corner from `a` to `b`: the smaller of their
/// streets'.
fn usual_radius(m: &Model, a: &Arm, b: &Arm) -> f64 {
    let of = |id| {
        m.road(id)
            .map_or(4.0, |r: &archisim_core::Road| r.section.corner_radius)
            .max(0.0)
    };
    of(a.road).min(of(b.road))
}

impl ArchiSimApp {
    /// A click with the tool Nudo: picks the node under it (or the free
    /// end of a street).
    pub(super) fn node_click(&mut self, p: Point2) {
        match self.node_mode.clone() {
            NodeMode::Pick => {}
            NodeMode::Link(start) => return self.node_link_click(p, start),
            NodeMode::Island(mut pts, paint) => {
                let reach = self.view.to_metres(crate::touch::reach(HANDLE_REACH));
                if pts.len() >= 3 && pts[0].distance(p) < reach {
                    self.node_mode = NodeMode::Island(pts, paint);
                    self.node_finish();
                    return;
                }
                pts.push(p);
                self.node_mode = NodeMode::Island(pts, paint);
                return;
            }
        }
        let level = self.ed.level;
        match crossroads::node_key_at(self.ed.model(), level, p, PICK_REACH) {
            Some(k) => {
                let title = node_title(self.ed.model(), &k);
                self.say(format!(
                    "{title}: cámbialo en Propiedades o con sus tiradores."
                ));
                self.node_pick = Some(k);
                self.node_mode = NodeMode::Pick;
            }
            None => {
                self.node_pick = None;
                self.say("Aquí no se encuentran calles ni acaba ninguna.");
            }
        }
    }

    /// A click while connecting lanes: the end of a lane coming in picks it;
    /// then the start of a lane going out connects them, or takes their
    /// connection away if they had one. The first connection by hand of an
    /// arm keeps the ones it had worked out.
    fn node_link_click(&mut self, p: Point2, start: Option<(ArmRef, usize)>) {
        let Some((key, Some(j), _)) = self.picked_node() else {
            self.node_mode = NodeMode::Pick;
            return;
        };
        let reach = self
            .view
            .to_metres(crate::touch::reach(HANDLE_REACH))
            .max(0.8);
        let mut best: Option<(f64, ArmRef, usize, bool)> = None;
        for a in &j.arms {
            let r = Junction::arm_ref(a);
            for (lanes, coming) in [(&a.lanes_in, true), (&a.lanes_out, false)] {
                for (i, &off) in lanes.iter().enumerate() {
                    let d = crossroads::lane_end(a, off).distance(p);
                    // Coming in first where a lane on the axis is both.
                    let d = if coming { d - 1e-6 } else { d };
                    if d < reach && best.is_none_or(|b| d < b.0) {
                        best = Some((d, r, i, coming));
                    }
                }
            }
        }
        let Some((_, arm, lane, coming)) = best else {
            self.say("Pincha el final de un carril (los puntos de cada calle).");
            return;
        };
        match start {
            Some((from, from_lane)) if !coming || from == arm => {
                if coming {
                    // The same arm: another lane of it.
                    self.node_mode = NodeMode::Link(Some((arm, lane)));
                    return;
                }
                let mut n = self
                    .ed
                    .model()
                    .node(&key)
                    .cloned()
                    .unwrap_or_else(|| NodeSettings::new(key.clone()));
                if !n.links.iter().any(|l| l.from == from) {
                    for c in j.lanes.iter().filter(|c| c.from == from) {
                        n.links.push(LaneLink {
                            from,
                            lane: c.lane as u8,
                            to: c.to,
                            to_lane: c.to_lane as u8,
                        });
                    }
                }
                let link = LaneLink {
                    from,
                    lane: from_lane as u8,
                    to: arm,
                    to_lane: lane as u8,
                };
                let had = n.links.contains(&link);
                n.links.retain(|l| *l != link);
                if !had {
                    n.links.push(link);
                    n.links.sort();
                }
                let res = self.ed.transact("Conexión de carriles", |m| m.set_node(n));
                self.report(
                    res,
                    if had {
                        "Conexión quitada."
                    } else {
                        "Carriles conectados."
                    },
                );
                self.node_mode = NodeMode::Link(None);
            }
            _ if coming => {
                self.node_mode = NodeMode::Link(Some((arm, lane)));
                self.say("Ahora el principio del carril por el que sale.");
            }
            _ => self.say("Empieza por el final de un carril que entra al nudo."),
        }
    }

    /// Intro, doble clic o clic derecho: closes the island being drawn, or
    /// stops connecting lanes. Whether it did something.
    pub(super) fn node_finish(&mut self) -> bool {
        match std::mem::take(&mut self.node_mode) {
            NodeMode::Pick => false,
            NodeMode::Link(_) => {
                self.say("Conexiones de carril terminadas.");
                true
            }
            NodeMode::Island(pts, paint) => {
                let Some((key, j, arms)) = self.picked_node() else {
                    return true;
                };
                if pts.len() < 3 {
                    self.say("Una isleta necesita al menos tres esquinas.");
                    return true;
                }
                let at = j.as_ref().map_or_else(|| arms[0].origin, |j| j.at);
                let (o, u) = crossroads::node_frame(&arms, at);
                let v = u.perp_left();
                let points = pts
                    .iter()
                    .map(|q| Point2::new((*q - o).dot(u), (*q - o).dot(v)))
                    .collect();
                let mut n = self
                    .ed
                    .model()
                    .node(&key)
                    .cloned()
                    .unwrap_or_else(|| NodeSettings::new(key.clone()));
                n.islands.push(PaintedIsland { points, paint });
                let res = self.ed.transact("Isleta pintada", |m| m.set_node(n));
                self.report(res, "Isleta pintada.");
                true
            }
        }
    }

    /// Esc with the tool Nudo: the last corner of an island, the lane picked
    /// or the mode, before leaving the tool. Whether it did something.
    pub(super) fn node_escape(&mut self) -> bool {
        match &mut self.node_mode {
            NodeMode::Pick => false,
            NodeMode::Link(start) => {
                if start.take().is_none() {
                    self.node_mode = NodeMode::Pick;
                }
                true
            }
            NodeMode::Island(pts, _) => {
                if pts.pop().is_none() {
                    self.node_mode = NodeMode::Pick;
                }
                true
            }
        }
    }

    /// The node picked, if it is still there: its key, its junction (none
    /// for a plain dead end) and its arms.
    fn picked_node(&self) -> Option<(NodeKey, Option<Junction>, Vec<Arm>)> {
        let key = self.node_pick.clone()?;
        let model = self.ed.model();
        let j = crossroads::node_junction(model, &key);
        let arms = match &j {
            Some(j) => j.arms.clone(),
            None => crossroads::node_arms(model, &key),
        };
        if arms.is_empty() {
            return None;
        }
        Some((key, j, arms))
    }

    /// The handles of the picked node: where each is and what it changes.
    fn node_handles(&self) -> Vec<(Point2, Grab)> {
        let Some((key, _, arms)) = self.picked_node() else {
            return Vec::new();
        };
        let model = self.ed.model();
        let n = model.node(&key);
        let mut out: Vec<(Point2, Grab)> = arms
            .iter()
            .map(|a| (a.origin + a.dir * a.cut, Grab::Setback(a.clone())))
            .collect();
        let count = arms.len();
        if count >= 2 && n.is_none_or(|n| n.kind() != NodeKind::Continuous) {
            for i in 0..count {
                let (a, b) = (&arms[i], &arms[(i + 1) % count]);
                let from = Junction::arm_ref(a);
                let c = n.map(|n| n.corner(from)).unwrap_or_default();
                if c.shape == Some(CornerShape::Sharp) {
                    continue;
                }
                let Some((apex, bis, half)) = crossroads::corner_frame(a, b) else {
                    continue;
                };
                let r = c.radius.unwrap_or_else(|| usual_radius(model, a, b));
                out.push((
                    crossroads::corner_middle(apex, bis, half, r),
                    Grab::Radius {
                        from,
                        apex,
                        bis,
                        half,
                    },
                ));
            }
        }
        out
    }

    /// Dragging a handle of the picked node: its arm's setback or its
    /// corner's radius.
    pub(super) fn node_drag(&mut self, ctx: &egui::Context, rect: Rect, response: &egui::Response) {
        if self.node_mode != NodeMode::Pick {
            return;
        }
        if response.drag_started_by(egui::PointerButton::Primary) {
            let origin = ctx.input(|i| i.pointer.press_origin());
            if let (Some(o), Some(key)) = (origin, self.node_pick.clone()) {
                let at = self.view.to_model(rect, o);
                let reach = self.view.to_metres(crate::touch::reach(HANDLE_REACH));
                self.node_grab = self
                    .node_handles()
                    .into_iter()
                    .filter(|(p, _)| p.distance(at) < reach)
                    .min_by(|a, b| a.0.distance(at).total_cmp(&b.0.distance(at)))
                    .map(|(_, what)| NodeGrab { key, what });
            }
        }
        if response.drag_stopped() {
            let at = ctx
                .input(|i| i.pointer.interact_pos())
                .map(|p| self.view.to_model(rect, p));
            let (Some(g), Some(c)) = (self.node_grab.take(), at) else {
                return;
            };
            let v = (g.what.value_at(c) * 100.0).round() / 100.0;
            let mut n = self
                .ed
                .model()
                .node(&g.key)
                .cloned()
                .unwrap_or_else(|| NodeSettings::new(g.key.clone()));
            let done = match &g.what {
                Grab::Setback(a) => {
                    let r = Junction::arm_ref(a);
                    let mut s = n.arm(r);
                    s.setback = Some(v);
                    n.set_arm(r, s);
                    format!("Retranqueo: {} m.", fmt_m(v))
                }
                Grab::Radius { from, .. } => {
                    let mut s = n.corner(*from);
                    s.radius = Some(v);
                    n.set_corner(*from, s);
                    format!("Radio de la esquina: {} m.", fmt_m(v))
                }
            };
            let res = self.ed.transact("Nudo", |m| m.set_node(n.clone()));
            self.report(res, &done);
        }
    }

    /// The picked node's outline and handles, over the plan; while a
    /// handle is dragged, where it would go.
    pub(super) fn draw_node_pick(&self, painter: &egui::Painter, rect: Rect) {
        if self.tool != Tool::Node {
            return;
        }
        let Some((_, j, arms)) = self.picked_node() else {
            return;
        };
        if let Some(j) = &j {
            let mut outline = j.plate.clone();
            if let Some(f) = outline.first().copied() {
                outline.push(f);
            }
            painter.add(Shape::line(
                self.polygon(rect, &outline),
                Stroke::new(2.5, ACCENT),
            ));
        }
        // Where each lane goes (N3), and the lanes' ends while connecting.
        if let Some(j) = &j {
            let linking = matches!(self.node_mode, NodeMode::Link(_));
            for c in &j.lanes {
                let col = if c.by_hand { LANE_BY_HAND } else { LANE_WORKED };
                let line = self.polygon(rect, &c.path);
                painter.add(Shape::line(
                    line.clone(),
                    Stroke::new(if linking { 2.0 } else { 1.2 }, col.gamma_multiply(0.85)),
                ));
                if let (Some(&tip), Some(&before)) =
                    (line.last(), line.get(line.len().saturating_sub(2)))
                {
                    let d = (tip - before).normalized() * 7.0;
                    let n = Vec2::new(-d.y, d.x) * 0.5;
                    painter.add(Shape::convex_polygon(
                        vec![tip, tip - d + n, tip - d - n],
                        col,
                        Stroke::NONE,
                    ));
                }
            }
            if let NodeMode::Link(start) = &self.node_mode {
                for a in &j.arms {
                    let r = Junction::arm_ref(a);
                    for (i, &off) in a.lanes_in.iter().enumerate() {
                        let at = self.view.to_screen(rect, crossroads::lane_end(a, off));
                        let on = *start == Some((r, i));
                        painter.circle(
                            at,
                            if on { 7.0 } else { 5.0 },
                            if on {
                                LANE_BY_HAND
                            } else {
                                Color32::from_rgb(60, 170, 90)
                            },
                            Stroke::new(1.5, Color32::WHITE),
                        );
                    }
                    for &off in &a.lanes_out {
                        let at = self.view.to_screen(rect, crossroads::lane_end(a, off));
                        painter.circle(at, 5.0, Color32::WHITE, Stroke::new(2.0, LANE_WORKED));
                    }
                }
            }
        }
        // The island being drawn.
        if let NodeMode::Island(pts, _) = &self.node_mode {
            let mut line = pts.clone();
            if let Some(c) = self.cursor {
                line.push(c.point);
            }
            if let Some(&f) = pts.first() {
                line.push(f);
            }
            painter.add(Shape::line(
                self.polygon(rect, &line),
                Stroke::new(1.5, LANE_BY_HAND),
            ));
            for q in pts {
                painter.circle_filled(self.view.to_screen(rect, *q), 3.5, LANE_BY_HAND);
            }
            return;
        }
        if matches!(self.node_mode, NodeMode::Link(_)) {
            return;
        }
        // The arms' ends.
        for a in &arms {
            let n = a.dir.perp_left();
            let end = a.origin + a.dir * a.cut;
            let line = [end - n * a.edge_right, end + n * a.edge_left];
            painter.add(Shape::line(
                self.polygon(rect, &line),
                Stroke::new(1.5, HANDLE),
            ));
        }
        for (p, _) in self.node_handles() {
            let at = self.view.to_screen(rect, p);
            painter.circle(at, 5.0, Color32::WHITE, Stroke::new(2.0, HANDLE));
        }
        // Where the dragged handle would go, and its value.
        let at = painter
            .ctx()
            .input(|i| i.pointer.interact_pos())
            .map(|p| self.view.to_model(rect, p));
        if let (Some(g), Some(c)) = (&self.node_grab, at) {
            let v = g.what.value_at(c);
            let (p, text) = match &g.what {
                Grab::Setback(a) => {
                    let n = a.dir.perp_left();
                    let end = a.origin + a.dir * v;
                    let line = [end - n * a.edge_right, end + n * a.edge_left];
                    painter.add(Shape::dashed_line(
                        &self.polygon(rect, &line),
                        Stroke::new(1.5, ACCENT),
                        6.0,
                        4.0,
                    ));
                    (end, format!("Retranqueo {} m", fmt_m(v)))
                }
                Grab::Radius {
                    apex, bis, half, ..
                } => (
                    crossroads::corner_middle(*apex, *bis, *half, v),
                    format!("Radio {} m", fmt_m(v)),
                ),
            };
            let at = self.view.to_screen(rect, p);
            painter.circle_filled(at, 5.0, ACCENT);
            painter.text(
                at + Vec2::new(10.0, -10.0),
                egui::Align2::LEFT_BOTTOM,
                text,
                egui::FontId::proportional(13.0),
                ACCENT,
            );
        }
    }

    pub(super) fn node_tool_properties(&mut self, ui: &mut Ui) {
        let Some((key, j, arms)) = self.picked_node() else {
            props::title(ui, "Herramienta Nudo");
            group(ui, "tool-node", "Nudo", |ui| {
                props::note(
                    ui,
                    "Haz clic en un cruce, una T, una L o el final de una calle para cambiarlo: tipo de nudo, cota, calle principal, retranqueo, señales y paso de peatones de cada calle, y radio, forma, acera y oreja de cada esquina. Los tiradores naranjas se arrastran.",
                );
            });
            self.node_level_group(ui);
            return;
        };
        let model = self.ed.model();
        props::title(ui, &node_title(model, &key));
        let before = model
            .node(&key)
            .cloned()
            .unwrap_or_else(|| NodeSettings::new(key.clone()));
        let mut n = before.clone();
        // Zebra crossings to lay or take away (edits of the streets).
        let mut crossings: Vec<(Arm, bool)> = Vec::new();
        // A roundabout to put at the node (N2).
        let mut roundabout: Option<(Point2, Vec<Arm>)> = None;
        let count = arms.len();
        group(ui, "node-kind", "Tipo de nudo", |ui| {
            ui.horizontal_wrapped(|ui| {
                for k in NodeKind::ALL {
                    if !k.fits(count) && !(k == NodeKind::Open && count >= 2) {
                        continue;
                    }
                    if kind_card(ui, k, n.kind() == k) {
                        n.kind = (k != NodeKind::Crossing).then_some(k);
                    }
                }
            });
            props::note(ui, kind_note(n.kind()));
            if count == 1 && n.kind() != NodeKind::DeadEnd {
                props::note(
                    ui,
                    "La calle acaba recta. Elige Fin para darle un círculo de giro.",
                );
            }
        });
        if n.kind() == NodeKind::DeadEnd {
            group(ui, "node-end", "Círculo de giro", |ui| {
                row(ui, "Radio (mm)", "El del bordillo del círculo", |ui, w| {
                    if let Some(v) = length_field(
                        ui,
                        Id::unique(("node-end-radius", &key)),
                        n.end_radius.unwrap_or(crossroads::TURNING_RADIUS),
                        w - 8.0,
                    ) {
                        n.end_radius = Some(v.max(1.0));
                    }
                });
            });
        }
        let open = n.kind() == NodeKind::Open;
        if !open && count >= 2 {
            group(ui, "node-whole", "El nudo entero", |ui| {
                let mut flat = n.flat.unwrap_or(false);
                ui.switch(&mut flat, "Plano a la cota de su centro");
                n.flat = flat.then_some(true);
                row(
                    ui,
                    "Calle principal",
                    "La que manda en la cota y el pavimento",
                    |ui, w| {
                        let names: Vec<(archisim_core::RoadId, String)> = {
                            let mut ids: Vec<archisim_core::RoadId> = key.road_ids().collect();
                            ids.dedup();
                            ids.into_iter()
                                .map(|r| {
                                    (r, model.road(r).map_or("calle".into(), |x| x.name.clone()))
                                })
                                .collect()
                        };
                        let shown = n
                            .main
                            .and_then(|m| names.iter().find(|x| x.0 == m))
                            .map_or("Ninguna (la más cercana)".to_string(), |x| x.1.clone());
                        ComboBox::from_id_salt(("node-main", &key))
                            .width(w - 8.0)
                            .selected_text(shown)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut n.main, None, "Ninguna (la más cercana)");
                                for (id, name) in &names {
                                    ui.selectable_value(&mut n.main, Some(*id), name);
                                }
                            });
                    },
                );
                if let Some(main) = n.main {
                    if ui
                        .button("Ceda el paso en las demás calles")
                        .on_hover_text("Pone ceda el paso a las calles que llegan a la principal")
                        .clicked()
                    {
                        for a in arms.iter().filter(|a| a.road != main) {
                            let r = Junction::arm_ref(a);
                            let mut s = n.arm(r);
                            s.sign = Some(ArmSign::Yield);
                            n.set_arm(r, s);
                        }
                    }
                }
                let mut quiet = n.hide_marks.unwrap_or(false);
                ui.switch(&mut quiet, "Sin marcas viales cerca del nudo");
                n.hide_marks = quiet.then_some(true);
                if count >= 3
                    && ui
                        .button("Hacer glorieta aquí")
                        .on_hover_text("Una glorieta en el centro del nudo, de radio suficiente para sus calles; sus ajustes se pierden")
                        .clicked()
                {
                    roundabout = j.as_ref().map(|j| (j.at, arms.clone()));
                }
            });
        }
        if !open {
            group(ui, "node-arms", "Calles que llegan", |ui| {
                for a in &arms {
                    let r = Junction::arm_ref(a);
                    let mut s = n.arm(r);
                    ui.label(egui::RichText::new(arm_name(model, a)).strong());
                    let hint = "Hasta dónde llega el nudo por esta calle, desde su centro";
                    row(ui, "Retranqueo (mm)", hint, |ui, w| {
                        let shown = s.setback.unwrap_or(a.cut);
                        if let Some(v) =
                            length_field(ui, Id::unique(("node-arm", &key, r)), shown, w - 8.0)
                        {
                            s.setback = Some(v.max(0.0));
                        }
                    });
                    if count >= 2 {
                        row(
                            ui,
                            "Señal",
                            "Para el tráfico que entra al nudo por esta calle",
                            |ui, w| {
                                let mut sign = s.sign.unwrap_or_default();
                                ComboBox::from_id_salt(("node-sign", &key, r))
                                    .width(w - 8.0)
                                    .selected_text(sign.label())
                                    .show_ui(ui, |ui| {
                                        for k in ArmSign::ALL {
                                            ui.selectable_value(&mut sign, k, k.label());
                                        }
                                    });
                                s.sign = (sign != ArmSign::None).then_some(sign);
                            },
                        );
                        let had = crossroads::arm_crossing(model, a).is_some();
                        let mut zebra = had;
                        ui.switch(&mut zebra, "Paso de peatones (con su bordillo rebajado)");
                        if zebra != had {
                            crossings.push((a.clone(), zebra));
                        }
                    }
                    n.set_arm(r, s);
                }
            });
        }
        if !open && count >= 2 && n.kind() != NodeKind::Continuous {
            let chamfer = crossroads::standard_chamfer(model);
            group(ui, "node-corners", "Esquinas", |ui| {
                if ui
                    .button(format!("Chaflán estándar ({} m) en todas", fmt_m(chamfer)))
                    .on_hover_text(
                        "El habitual del lugar del proyecto; luego cambia el que quieras",
                    )
                    .clicked()
                {
                    for a in &arms {
                        let r = Junction::arm_ref(a);
                        let mut c = n.corner(r);
                        c.shape = Some(CornerShape::Chamfer);
                        c.radius = Some(chamfer);
                        n.set_corner(r, c);
                    }
                }
                for i in 0..count {
                    let (a, b) = (&arms[i], &arms[(i + 1) % count]);
                    let r = Junction::arm_ref(a);
                    let mut c = n.corner(r);
                    ui.label(
                        egui::RichText::new(format!("Entre el {} y el {}", heading(a), heading(b)))
                            .strong(),
                    );
                    let usual = usual_radius(model, a, b);
                    row(
                        ui,
                        "Radio (mm)",
                        "El del bordillo de la esquina",
                        |ui, w| {
                            if let Some(v) = length_field(
                                ui,
                                Id::unique(("node-radius", &key, r)),
                                c.radius.unwrap_or(usual),
                                w - 8.0,
                            ) {
                                c.radius = Some(v.max(0.0));
                            }
                        },
                    );
                    row(ui, "Forma", "", |ui, w| {
                        let mut shape = c.shape.unwrap_or_default();
                        ComboBox::from_id_salt(("node-shape", &key, r))
                            .width(w - 8.0)
                            .selected_text(shape.label())
                            .show_ui(ui, |ui| {
                                for s in CornerShape::ALL {
                                    ui.selectable_value(&mut shape, s, s.label());
                                }
                            });
                        c.shape = (shape != CornerShape::Arc).then_some(shape);
                    });
                    let mut walk = c.sidewalk.unwrap_or(true);
                    ui.switch(&mut walk, "Acera en la esquina");
                    c.sidewalk = (!walk).then_some(false);
                    row(
                        ui,
                        "Oreja (mm)",
                        "Cuánto sale la acera de la esquina sobre la calzada (el aparcamiento)",
                        |ui, w| {
                            if let Some(v) = length_field(
                                ui,
                                Id::unique(("node-bump", &key, r)),
                                c.bump.unwrap_or(0.0),
                                w - 8.0,
                            ) {
                                c.bump = (v > 0.0).then_some(v);
                            }
                        },
                    );
                    n.set_corner(r, c);
                }
            });
        }
        // The lanes (N3): where each goes, its stop line, and connecting
        // them by hand.
        let mut mode = None;
        if let (false, true, Some(j)) = (open, count >= 2, &j) {
            group(ui, "node-lanes", "Carriles", |ui| {
                let hand = n.links.len();
                props::note(
                    ui,
                    &format!(
                        "{} conexiones de carril{}. Las líneas azules son las calculadas y las naranjas las hechas a mano; las flechas se pintan en las calles con más de un carril de entrada o cambiadas a mano.",
                        j.lanes.len(),
                        if hand > 0 { format!(", {hand} a mano") } else { String::new() }
                    ),
                );
                for a in &j.arms {
                    if a.lanes_in.is_empty() {
                        continue;
                    }
                    let r = Junction::arm_ref(a);
                    let mut s = n.arm(r);
                    ui.label(egui::RichText::new(arm_name(model, a)).strong());
                    if n.links.iter().any(|l| l.from == r) {
                        props::note(ui, "Va por sus conexiones hechas a mano.");
                    } else {
                        let mut turns = crossroads::arm_turns(a, Some(&n));
                        let mut changed = false;
                        for (i, t) in turns.iter_mut().enumerate() {
                            let name = if a.lanes_in.len() == 1 {
                                "Carril".to_string()
                            } else if i == 0 {
                                "Carril 1 (bordillo)".to_string()
                            } else {
                                format!("Carril {}", i + 1)
                            };
                            row(ui, &name, "Hacia dónde puede ir", |ui, _| {
                                ui.horizontal(|ui| {
                                    for (k, label, arrow) in Turns::ALL {
                                        let on = t.has(k);
                                        if ui
                                            .selectable_label(on, arrow)
                                            .on_hover_text(label)
                                            .clicked()
                                        {
                                            *t = t.with(k, !on);
                                            changed = true;
                                        }
                                    }
                                });
                            });
                        }
                        if changed {
                            s.turns = Some(turns);
                        }
                        if s.turns.is_some() && ui.small_button("Giros calculados").clicked() {
                            s.turns = None;
                        }
                    }
                    row(
                        ui,
                        "Detención (mm)",
                        "Cuánto se retrasa la línea de detención desde el final del brazo",
                        |ui, w| {
                            if let Some(v) = length_field(
                                ui,
                                Id::unique(("node-stop", &key, r)),
                                s.stop_back.unwrap_or(0.0),
                                w - 8.0,
                            ) {
                                s.stop_back = (v > 0.0).then_some(v);
                            }
                        },
                    );
                    n.set_arm(r, s);
                }
                ui.horizontal_wrapped(|ui| {
                    let linking = matches!(self.node_mode, NodeMode::Link(_));
                    if ui
                        .selectable_label(linking, "Conectar carriles a mano")
                        .on_hover_text("En planta: el final de un carril que entra y luego el principio del carril por el que sale (otra vez, para quitarla)")
                        .clicked()
                    {
                        mode = Some(if linking { NodeMode::Pick } else { NodeMode::Link(None) });
                    }
                    if ui
                        .add_enabled(!n.links.is_empty(), egui::Button::new("Quitar las conexiones a mano"))
                        .clicked()
                    {
                        n.links.clear();
                    }
                });
            });
            group(ui, "node-islands", "Isletas pintadas", |ui| {
                let mut gone = None;
                for (i, isl) in n.islands.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("Isleta {}", i + 1));
                        ComboBox::from_id_salt(("node-island", &key, i))
                            .selected_text(isl.paint.label())
                            .show_ui(ui, |ui| {
                                for k in IslandPaint::ALL {
                                    ui.selectable_value(&mut isl.paint, k, k.label());
                                }
                            });
                        if ui.small_button("Quitar").clicked() {
                            gone = Some(i);
                        }
                    });
                }
                if let Some(i) = gone {
                    n.islands.remove(i);
                }
                let drawing = matches!(self.node_mode, NodeMode::Island(..));
                if ui
                    .selectable_label(drawing, "Dibujar isleta pintada")
                    .on_hover_text("Clic en cada esquina; Intro, doble clic o clic en la primera para cerrarla; Esc quita la última")
                    .clicked()
                {
                    mode = Some(if drawing {
                        NodeMode::Pick
                    } else {
                        NodeMode::Island(Vec::new(), IslandPaint::Chevrons)
                    });
                }
                props::note(
                    ui,
                    "Las isletas van con el nudo: si mueves sus calles, se mueven con él.",
                );
            });
        }
        if let Some(m) = mode {
            self.node_mode = m;
        }
        let mut reset = false;
        group(ui, "node-reset", "Nudo calculado", |ui| {
            props::note(
                ui,
                "Lo que no cambies se sigue calculando con las calles. Si quitas una calle del nudo, sus ajustes se pierden.",
            );
            reset = ui
                .add_enabled(!before.is_plain(), egui::Button::new("Restablecer"))
                .clicked();
        });
        if reset {
            n = NodeSettings::new(key.clone());
        }
        if let Some((at, arms)) = roundabout {
            self.roundabout_at_node(at, &arms);
            self.node_pick = None;
            return;
        }
        if n != before {
            let res = self.ed.transact("Nudo", |m| m.set_node(n.clone()));
            self.report(res, "Nudo cambiado.");
        }
        for (a, on) in crossings {
            let res = self.ed.transact("Paso de peatones", |m| {
                crossroads::set_arm_crossing(m, &a, on)
            });
            self.report(
                res,
                if on {
                    "Paso de peatones puesto."
                } else {
                    "Paso de peatones quitado."
                },
            );
        }
        self.node_level_group(ui);
    }

    /// A roundabout at a node's centre (N2): the usual one, larger if its
    /// streets are wide (a ring of at least 6,5 m round their carriageways).
    pub(super) fn roundabout_at_node(&mut self, at: Point2, arms: &[Arm]) {
        let keep = self.roundabout_settings.clone();
        let wide = arms
            .iter()
            .map(|a| a.kerb_left.max(a.kerb_right))
            .fold(0.0, f64::max);
        let s = &mut self.roundabout_settings;
        let need = wide + 6.5 + s.apron + 2.0;
        if need > s.radius {
            s.island = (need - 6.5 - s.apron).max(2.0);
            s.radius = need;
        }
        self.roundabout_click(at);
        self.roundabout_settings = keep;
    }

    /// What the tool does to every node of the level: the standard chamfer
    /// on every corner.
    fn node_level_group(&mut self, ui: &mut Ui) {
        let model = self.ed.model();
        let level = self.ed.level;
        let chamfer = crossroads::standard_chamfer(model);
        let mut all = false;
        group(ui, "node-level", "Todos los nudos de la planta", |ui| {
            props::note(
                ui,
                "El chaflán habitual del lugar del proyecto en todas las esquinas de los cruces de esta planta; después puedes cambiar cada uno.",
            );
            all = ui
                .button(format!(
                    "Chaflán estándar ({} m) en toda la planta",
                    fmt_m(chamfer)
                ))
                .clicked();
        });
        if all {
            let res = self.ed.transact("Chaflanes", |m| {
                for j in crossroads::junctions(m, level) {
                    let Some(key) = j.key.clone() else { continue };
                    if j.arms.len() < 2 || matches!(j.kind, NodeKind::Continuous | NodeKind::Bend) {
                        continue;
                    }
                    let mut n = m
                        .node(&key)
                        .cloned()
                        .unwrap_or_else(|| NodeSettings::new(key.clone()));
                    for a in &j.arms {
                        let r = Junction::arm_ref(a);
                        let c = CornerSettings {
                            shape: Some(CornerShape::Chamfer),
                            radius: Some(chamfer),
                            ..n.corner(r)
                        };
                        n.set_corner(r, c);
                    }
                    m.set_node(n)?;
                }
                Ok::<(), archisim_core::ModelError>(())
            });
            self.report(res, "Chaflanes puestos en todos los cruces de la planta.");
        }
    }
}

/// Metres with a decimal comma.
fn fmt_m(v: f64) -> String {
    format!("{v:.2}").replace('.', ",")
}

#[cfg(test)]
pub(super) fn first_node(m: &Model, level: archisim_core::LevelId) -> Option<NodeKey> {
    crossroads::node_keys(m, level).into_iter().next()
}
