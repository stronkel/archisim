//! Moving an underlay with the mouse (0.84): «Mover» by a base point and
//! a destination, «Encajar» by two points of the underlay and the two
//! places they go (moved, turned and, if wanted, scaled), and «Escalar» by
//! two points and the length they really measure. The points snap to the
//! lines of the underlay itself (a DXF in a drafting view, the drawing of
//! another plan) and to the model, with the tracking lines.

use super::*;
use archisim_core::{DraftViewId, Underlay};

/// Where the underlays being edited live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnderlayHost {
    Level(LevelId),
    Drafting(DraftViewId),
}

/// What the clicks do to the underlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnderlayMode {
    /// Base point, destination.
    Move,
    /// Two points of the underlay, then where each goes; `scale`: also
    /// scaled so the second lands on its place.
    Fit { scale: bool },
    /// Two points of the underlay, then how long that really is.
    Measure,
}

/// An underlay being moved, fitted or scaled.
#[derive(Debug, Clone, PartialEq)]
pub struct UnderlayPick {
    pub host: UnderlayHost,
    pub index: usize,
    pub mode: UnderlayMode,
    pub points: Vec<Point2>,
    /// «Escalar»: the length typed, once the two points are in.
    pub typed: String,
}

impl UnderlayPick {
    fn wanted(&self) -> usize {
        match self.mode {
            UnderlayMode::Move => 2,
            UnderlayMode::Fit { .. } => 4,
            UnderlayMode::Measure => 2,
        }
    }

    /// What the next click is for.
    pub fn step(&self) -> &'static str {
        match (self.mode, self.points.len()) {
            (UnderlayMode::Move, 0) => "Mover calco: clic en el punto base (se engancha al calco)",
            (UnderlayMode::Move, _) => {
                "Mover calco: clic en el destino, o escribe la distancia e Intro"
            }
            (UnderlayMode::Fit { .. }, 0) => "Encajar calco: primer punto del calco",
            (UnderlayMode::Fit { .. }, 1) => "Encajar calco: segundo punto del calco",
            (UnderlayMode::Fit { .. }, 2) => "Encajar calco: dónde va el primer punto",
            (UnderlayMode::Fit { .. }, _) => "Encajar calco: hacia dónde va el segundo",
            (UnderlayMode::Measure, 0) => "Escalar calco: un extremo de algo cuya medida sabes",
            (UnderlayMode::Measure, 1) => "Escalar calco: el otro extremo",
            (UnderlayMode::Measure, _) => "Escalar calco: escribe cuánto mide de verdad e Intro",
        }
    }
}

impl ArchiSimApp {
    /// The underlays of a host.
    pub(super) fn underlays_of(&self, host: UnderlayHost) -> Vec<Underlay> {
        let m = self.ed.model();
        match host {
            UnderlayHost::Level(l) => m.level_underlays(l).to_vec(),
            UnderlayHost::Drafting(d) => m
                .draft_view(d)
                .map(|v| v.underlays.clone())
                .unwrap_or_default(),
        }
    }

    /// Puts one underlay of a host, as one step to undo.
    pub(super) fn set_underlay(
        &mut self,
        host: UnderlayHost,
        index: usize,
        u: Underlay,
        label: &str,
    ) {
        let mut list = self.underlays_of(host);
        let Some(slot) = list.get_mut(index) else {
            return;
        };
        *slot = u;
        let r = match host {
            UnderlayHost::Level(l) => self
                .ed
                .transact("Calco", |m| m.set_level_underlays(l, list)),
            UnderlayHost::Drafting(d) => {
                let Some(mut v) = self.ed.model().draft_view(d).cloned() else {
                    return;
                };
                v.underlays = list;
                self.ed.transact("Calco", |m| m.set_drafting_view(v))
            }
        };
        self.report(r, label);
    }

    /// Starts moving, fitting or scaling an underlay with the mouse, in
    /// the view it is under (a level's plan is opened if need be).
    pub(super) fn start_underlay_pick(
        &mut self,
        host: UnderlayHost,
        index: usize,
        mode: UnderlayMode,
    ) {
        if self.underlays_of(host).get(index).is_some_and(|u| u.locked) {
            self.complain("El calco está bloqueado: quita el candado para moverlo.");
            return;
        }
        if let UnderlayHost::Level(l) = host {
            if self.ed.level != l || self.mode != ViewMode::Plan {
                self.go_to_level_plan(l);
            }
        }
        self.tool = Tool::Arrow;
        self.drag = None;
        let pick = UnderlayPick {
            host,
            index,
            mode,
            points: Vec::new(),
            typed: String::new(),
        };
        self.say(pick.step());
        self.underlay_pick = Some(pick);
    }

    /// The points of an underlay near `near`, where they are drawn: the
    /// ends and middles of the lines of a drafting view (a DXF brought
    /// in), or of the drawing, walls and slabs of a level.
    fn underlay_points(
        &self,
        host: UnderlayHost,
        index: usize,
        near: Point2,
        reach: f64,
    ) -> Vec<Point2> {
        let Some(u) = self.underlays_of(host).get(index).copied() else {
            return Vec::new();
        };
        let m = self.ed.model();
        let k = crate::pick::paper_k();
        // Near in the underlay's own metres.
        let q = u.source_of(near);
        let r = reach / u.scale.max(1e-6);
        let close = |p: Point2| p.distance(q) <= r * 1.5;
        let mut src: Vec<Point2> = Vec::new();
        let view = match u.source {
            archisim_core::UnderlaySource::Level(l) => {
                for w in m.walls().filter(|w| w.level == l) {
                    src.extend([w.start, w.end, (w.start + w.end) * 0.5]);
                }
                for s in m.slabs().filter(|s| s.level == l) {
                    src.extend(s.outline.iter().copied());
                }
                Some(archisim_core::ViewRef::Level(l))
            }
            archisim_core::UnderlaySource::Drafting(d) => Some(archisim_core::ViewRef::Drafting(d)),
            archisim_core::UnderlaySource::Section(_) => None,
        };
        if let Some(view) = view {
            for d in m.drafts_in(view) {
                let (lo, hi) = archisim_draft::bounds(&d.shape, k);
                if q.x < lo.x - r || q.x > hi.x + r || q.y < lo.y - r || q.y > hi.y + r {
                    continue;
                }
                src.extend(
                    archisim_draft::snap_points(&d.shape)
                        .into_iter()
                        .map(|(p, _)| p),
                );
            }
        }
        src.into_iter()
            .filter(|p| close(*p))
            .map(|p| u.place(p))
            .collect()
    }

    /// The cursor while an underlay is being edited: snapped to the
    /// underlay's points (while picking points of it) and to the model,
    /// with tracking from the last point.
    pub(super) fn underlay_cursor(&mut self, ctx: &egui::Context, raw: Point2, reach: f64) {
        let Some(pick) = self.underlay_pick.clone() else {
            return;
        };
        // The points of the underlay itself: its base, and for «Encajar»
        // and «Escalar» its two points; the destinations snap to the model.
        let of_underlay = match pick.mode {
            UnderlayMode::Move => pick.points.is_empty(),
            UnderlayMode::Fit { .. } => pick.points.len() < 2,
            UnderlayMode::Measure => true,
        };
        let mut extra = if of_underlay {
            self.underlay_points(pick.host, pick.index, raw, reach)
        } else {
            Vec::new()
        };
        extra.extend(pick.points.iter().copied());
        let base = pick.points.last().copied();
        let mut s = match self.mode {
            ViewMode::Drafting(d) => crate::snap::snap_view(
                self.ed.model(),
                archisim_core::ViewRef::Drafting(d),
                raw,
                reach,
                &extra,
                self.snap_filter,
                base,
            ),
            _ => crate::snap::snap_filtered(
                &self.ed,
                self.ed.level,
                raw,
                reach,
                &extra,
                &BTreeSet::new(),
                self.snap_filter,
                &self.other_walls,
            ),
        };
        if let Some(b) = base {
            if ctx.input(|i| i.modifiers.shift) {
                s = crate::snap::constrain(b, s);
            } else if self.snap_filter.tracking {
                s = crate::snap::track(b, s, self.view.to_metres(TRACK_REACH));
            }
        }
        self.cursor = Some(s);
        self.hover = None;
    }

    /// A click while an underlay is being edited: one more point, and the
    /// change when they are all in.
    pub(super) fn underlay_click(&mut self, p: Point2) {
        let Some(mut pick) = self.underlay_pick.take() else {
            return;
        };
        if pick.points.len() < pick.wanted() {
            pick.points.push(p);
        }
        if pick.points.len() < pick.wanted() || pick.mode == UnderlayMode::Measure {
            self.say(pick.step());
            self.underlay_pick = Some(pick);
            return;
        }
        self.finish_underlay(pick);
    }

    fn finish_underlay(&mut self, pick: UnderlayPick) {
        let Some(u) = self.underlays_of(pick.host).get(pick.index).copied() else {
            return;
        };
        let pts = &pick.points;
        let (new, label) = match pick.mode {
            UnderlayMode::Move => {
                let to = match (
                    crate::units::parse_length(&pick.typed),
                    (pts[1] - pts[0]).normalized(),
                ) {
                    (Some(len), Some(dir)) if !pick.typed.is_empty() => pts[0] + dir * len,
                    _ => pts[1],
                };
                let d = to - pts[0];
                (
                    Underlay {
                        offset: u.offset + d,
                        ..u
                    },
                    format!("Calco movido {}.", crate::units::fmt_len(d.length())),
                )
            }
            UnderlayMode::Fit { scale } => (
                u.fitted(pts[0], pts[1], pts[2], pts[3], scale),
                if scale {
                    "Calco encajado: movido, girado y escalado.".to_string()
                } else {
                    "Calco encajado: movido y girado.".to_string()
                },
            ),
            UnderlayMode::Measure => {
                let drawn = pts[0].distance(pts[1]);
                let Some(real) = crate::units::parse_length(&pick.typed).filter(|v| *v > 0.0)
                else {
                    self.complain(format!("«{}» no es una medida válida.", pick.typed));
                    return;
                };
                if drawn < 1e-6 {
                    return;
                }
                let k = real / drawn;
                (
                    u.scaled_about(pts[0], k),
                    format!(
                        "Calco escalado ×{:.4}: esa medida es {}.",
                        k,
                        crate::units::fmt_len(real)
                    ),
                )
            }
        };
        self.set_underlay(pick.host, pick.index, new, &label);
    }

    /// Keys while an underlay is being edited: digits for a distance or a
    /// length, Enter, Backspace and Esc. Whether they were taken.
    pub(super) fn underlay_keys(&mut self, ctx: &egui::Context) -> bool {
        if self.underlay_pick.is_none() {
            return false;
        }
        let (events, enter, escape, backspace) = ctx.input(|i| {
            (
                i.events.clone(),
                i.key_pressed(Key::Enter),
                i.key_pressed(Key::Escape),
                i.key_pressed(Key::Backspace),
            )
        });
        if escape {
            self.underlay_pick = None;
            self.say("Cancelado.");
            return true;
        }
        let Some(pick) = self.underlay_pick.as_mut() else {
            return false;
        };
        let typing = match pick.mode {
            UnderlayMode::Move => pick.points.len() == 1,
            UnderlayMode::Measure => pick.points.len() == 2,
            UnderlayMode::Fit { .. } => false,
        };
        if typing {
            for e in &events {
                if let egui::Event::Text(t) = e {
                    for c in t.chars() {
                        if c.is_ascii_digit() || matches!(c, ',' | '.' | 'm' | 'c' | 'M' | 'C') {
                            pick.typed.push(c);
                        }
                    }
                }
            }
            if backspace {
                pick.typed.pop();
            }
        }
        if enter && typing && !pick.typed.is_empty() {
            let mut pick = self.underlay_pick.take().unwrap();
            if pick.mode == UnderlayMode::Move {
                // Towards the cursor.
                let c = self.cursor.map_or(pick.points[0], |c| c.point);
                pick.points.push(c);
            }
            self.finish_underlay(pick);
        }
        true
    }

    /// The points picked, the line to the cursor and what the next click
    /// is for.
    pub(super) fn draw_underlay_pick(&self, painter: &egui::Painter, rect: Rect) {
        let Some(pick) = &self.underlay_pick else {
            return;
        };
        let s = Stroke::new(1.5, ACCENT);
        for (i, p) in pick.points.iter().enumerate() {
            let q = self.view.to_screen(rect, *p);
            painter.circle_stroke(q, 5.0, s);
            painter.text(
                q + Vec2::new(7.0, -7.0),
                Align2::LEFT_BOTTOM,
                format!("{}", i + 1),
                FontId::proportional(11.0),
                ACCENT,
            );
        }
        // «Encajar»: the underlay's pair, and the pair it goes to.
        if pick.points.len() >= 2 && matches!(pick.mode, UnderlayMode::Fit { .. }) {
            let (a, b) = (
                self.view.to_screen(rect, pick.points[0]),
                self.view.to_screen(rect, pick.points[1]),
            );
            painter.line_segment([a, b], Stroke::new(1.0, ACCENT));
        }
        if let (Some(last), Some(c)) = (pick.points.last(), self.cursor) {
            let (a, b) = (
                self.view.to_screen(rect, *last),
                self.view.to_screen(rect, c.point),
            );
            painter.extend(Shape::dashed_line(&[a, b], s, 6.0, 4.0));
            let d = c.point - *last;
            let text = if pick.typed.is_empty() {
                format!(
                    "{}   {}",
                    crate::units::fmt_len(d.length()),
                    fmt_angle(d.y.atan2(d.x))
                )
            } else {
                format!("{}▏", pick.typed)
            };
            if pick.mode != UnderlayMode::Measure || pick.points.len() < 2 {
                self.draw_label(painter, rect, c.point, text);
            }
        }
        if pick.mode == UnderlayMode::Measure && pick.points.len() == 2 {
            let drawn = pick.points[0].distance(pick.points[1]);
            let at = (pick.points[0] + pick.points[1]) * 0.5;
            self.draw_label(
                painter,
                rect,
                at,
                format!(
                    "Dibujado: {}   De verdad: {}▏",
                    crate::units::fmt_len(drawn),
                    pick.typed
                ),
            );
        }
    }
}
