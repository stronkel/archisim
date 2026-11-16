//! The fingers and the Pencil (`docs/IPAD.md` §2, T3).
//!
//! The browser (and Windows on a touch screen) hands every touch to egui
//! as a touch and, for the first one, as the mouse's left button. Over the
//! drawing (plan, section, sheet, drafting view or 3D) that is not what a
//! touch means here: **the Pencil draws, the fingers move the view**. This
//! module reads the touches before egui does (`eframe::App::raw_input_hook`)
//! and puts in their place what the window already understands:
//!
//! - the Pencil, as it comes: the mouse's left button;
//! - one finger dragged: the button that moves the view (the wheel in
//!   plan, which pans; the right button in 3D, which turns);
//! - two fingers: pinching zooms and dragging pans;
//! - a finger tapped: a click; held still: the right button (the menu);
//! - two fingers tapped: undo; three: redo.
//!
//! With «El dedo dibuja» (no Pencil, or chosen), one finger does what the
//! Pencil does and two move the view. Touches that start on the panels,
//! menus or dialogs are left alone: there a finger is a mouse.
//!
//! It knows nothing of the window but the drawing's rectangle; it is
//! tested with made-up touches.

use std::collections::BTreeMap;

use eframe::egui;
use egui::{Event, Modifiers, PointerButton, Pos2, Rect, TouchId, TouchPhase, Vec2};

/// How far a finger moves (points) before it is a drag and not a tap.
pub const SLOP: f32 = 10.0;
/// How long a finger held still is a long press (seconds).
pub const LONG_PRESS: f64 = 0.5;
/// The longest a tap of two or three fingers lasts (seconds).
const MULTI_TAP: f64 = 0.35;

/// What touched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Pen,
    Finger,
}

/// What the window must do because of a gesture (not an event of egui).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Undo,
    Redo,
}

/// Where the drawing is, from the last frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Area {
    pub rect: Rect,
    /// The 3D view (one finger turns) or a flat one (one finger pans).
    pub three_d: bool,
}

#[derive(Clone, Copy, Debug)]
struct Track {
    kind: Kind,
    start: Pos2,
    pos: Pos2,
    started: f64,
}

/// What the touches are doing now.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    /// No touch.
    Idle,
    /// Left as they come (the Pencil, or touches on the panels).
    Through,
    /// One finger down, not yet a tap, a drag or a long press.
    Waiting,
    /// One finger dragging, holding `button`.
    Drag(PointerButton),
    /// Two or more fingers: pan (the wheel button held at their middle)
    /// and zoom; `moved` once they have gone beyond a tap.
    Pinch {
        centre: Pos2,
        spread: f32,
        moved: bool,
        most: usize,
    },
    /// Done with (a long press, or what is left of a pinch): the touches
    /// are swallowed until all are up. `tap`: the fingers of a pinch that
    /// has not moved (a tap of several fingers, lifted one by one).
    Spent { tap: Option<usize> },
}

/// The state of the touches across frames.
#[derive(Debug)]
pub struct Touches {
    tracks: BTreeMap<TouchId, Track>,
    mode: Mode,
    /// A touch has been seen (shows «El dedo dibuja» in the bar).
    pub seen: bool,
    /// The Pencil has been seen on this device.
    pub pen_seen: bool,
    /// When the gesture started (seconds of the input's clock).
    began: f64,
}

impl Default for Touches {
    fn default() -> Self {
        Touches {
            tracks: BTreeMap::new(),
            mode: Mode::Idle,
            seen: false,
            pen_seen: false,
            began: 0.0,
        }
    }
}

impl Touches {
    /// Whether a gesture is going on (the window should keep looking at
    /// the clock, for the long press).
    pub fn busy(&self) -> bool {
        self.mode == Mode::Waiting
    }

    /// Rewrites the frame's input. `area` is where the drawing is (None:
    /// nowhere), `over_drawing` says whether a point is not covered by a
    /// panel, menu or dialog, `is_pen` whether a touch is the Pencil, and
    /// `finger_draws` whether one finger draws.
    pub fn translate(
        &mut self,
        raw: &mut egui::RawInput,
        area: Option<Area>,
        over_drawing: impl Fn(Pos2) -> bool,
        is_pen: impl Fn(TouchId) -> bool,
        finger_draws: bool,
    ) -> Vec<Action> {
        let now = raw.time.unwrap_or(0.0);
        let mut actions = Vec::new();
        let touches: Vec<(TouchId, TouchPhase, Pos2)> = raw
            .events
            .iter()
            .filter_map(|e| match e {
                Event::Touch { id, phase, pos, .. } => Some((*id, *phase, *pos)),
                _ => None,
            })
            .collect();
        if touches.is_empty() && self.mode != Mode::Waiting {
            return actions;
        }
        let gesture = |m: Mode| !matches!(m, Mode::Idle | Mode::Through);
        let was_gesture = gesture(self.mode);
        let mut out: Vec<Event> = Vec::new();
        let mut ended: Vec<TouchId> = Vec::new();
        for (id, phase, pos) in touches {
            match phase {
                TouchPhase::Start => {
                    self.seen = true;
                    let kind = if is_pen(id) { Kind::Pen } else { Kind::Finger };
                    if kind == Kind::Pen {
                        self.pen_seen = true;
                    }
                    self.tracks.insert(
                        id,
                        Track {
                            kind,
                            start: pos,
                            pos,
                            started: now,
                        },
                    );
                    self.started(id, area, &over_drawing, &mut out, now);
                }
                TouchPhase::Move => {
                    if let Some(t) = self.tracks.get_mut(&id) {
                        t.pos = pos;
                    }
                }
                TouchPhase::End | TouchPhase::Cancel => {
                    if let Some(t) = self.tracks.get_mut(&id) {
                        t.pos = pos;
                    }
                    ended.push(id);
                }
            }
        }
        self.moved(area, finger_draws, &mut out);
        for id in ended {
            self.lifted(id, area, finger_draws, &mut out, &mut actions, now);
        }
        self.held(area, &mut out, now);
        if was_gesture || gesture(self.mode) || !out.is_empty() {
            // What the browser made of the touches as a mouse goes; ours
            // takes its place.
            raw.events.retain(|e| {
                !matches!(
                    e,
                    Event::PointerButton {
                        button: PointerButton::Primary,
                        ..
                    } | Event::PointerMoved(_)
                        | Event::PointerGone
                )
            });
        }
        if self.tracks.is_empty() && self.mode == Mode::Through {
            self.mode = Mode::Idle;
        }
        raw.events.extend(out);
        actions
    }

    /// A touch began.
    fn started(
        &mut self,
        id: TouchId,
        area: Option<Area>,
        over_drawing: &impl Fn(Pos2) -> bool,
        out: &mut Vec<Event>,
        now: f64,
    ) {
        let Some(t) = self.tracks.get(&id).copied() else {
            return;
        };
        let in_drawing = area.is_some_and(|a| a.rect.contains(t.start)) && over_drawing(t.start);
        match self.mode {
            Mode::Idle => {
                self.began = now;
                self.mode = if t.kind == Kind::Pen || !in_drawing {
                    Mode::Through
                } else {
                    Mode::Waiting
                };
            }
            // A second finger: the view moves with both.
            Mode::Waiting | Mode::Drag(_) if t.kind == Kind::Finger => {
                if let Mode::Drag(button) = self.mode {
                    out.push(release(button, self.last_finger()));
                }
                self.began = now;
                self.pinch(out);
            }
            Mode::Pinch { moved, .. } if t.kind == Kind::Finger => {
                // A third finger: release and grip again, without jumping.
                out.push(release(PointerButton::Middle, self.centre()));
                self.pinch(out);
                if let Mode::Pinch { moved: m, .. } = &mut self.mode {
                    *m = moved;
                }
            }
            _ => {}
        }
    }

    /// Fingers that moved this frame.
    fn moved(&mut self, area: Option<Area>, finger_draws: bool, out: &mut Vec<Event>) {
        match self.mode {
            Mode::Waiting => {
                let Some(t) = self.only_finger() else {
                    return;
                };
                if (t.pos - t.start).length() > SLOP {
                    let button = if finger_draws {
                        PointerButton::Primary
                    } else if area.is_some_and(|a| a.three_d) {
                        PointerButton::Secondary
                    } else {
                        PointerButton::Middle
                    };
                    out.push(Event::PointerMoved(t.start));
                    out.push(press(button, t.start));
                    out.push(Event::PointerMoved(t.pos));
                    self.mode = Mode::Drag(button);
                }
            }
            Mode::Drag(_) => {
                if let Some(t) = self.only_finger() {
                    out.push(Event::PointerMoved(t.pos));
                }
            }
            Mode::Pinch {
                centre,
                spread,
                moved,
                most,
            } => {
                let (c, s) = (self.centre(), self.spread());
                if c.distance(centre) > 0.0 {
                    out.push(Event::PointerMoved(c));
                }
                if spread > 1.0 && s > 1.0 && (s - spread).abs() > 0.0 {
                    out.push(Event::Zoom(s / spread));
                }
                let gone = self
                    .tracks
                    .values()
                    .any(|t| (t.pos - t.start).length() > SLOP);
                self.mode = Mode::Pinch {
                    centre: c,
                    spread: s,
                    moved: moved || gone,
                    most: most.max(self.tracks.len()),
                };
            }
            _ => {}
        }
    }

    /// A touch was lifted.
    fn lifted(
        &mut self,
        id: TouchId,
        area: Option<Area>,
        finger_draws: bool,
        out: &mut Vec<Event>,
        actions: &mut Vec<Action>,
        now: f64,
    ) {
        let Some(t) = self.tracks.remove(&id) else {
            return;
        };
        match self.mode {
            Mode::Waiting => {
                // A tap: a click where it was.
                out.push(Event::PointerMoved(t.pos));
                out.push(press(PointerButton::Primary, t.pos));
                out.push(release(PointerButton::Primary, t.pos));
                self.mode = Mode::Idle;
            }
            Mode::Drag(button) => {
                out.push(release(button, t.pos));
                self.mode = Mode::Idle;
            }
            Mode::Pinch { moved, most, .. } => {
                if self.tracks.is_empty() {
                    out.push(release(PointerButton::Middle, t.pos));
                    if !moved {
                        self.tapped(most, now, actions);
                    }
                    self.mode = Mode::Idle;
                } else if self.tracks.len() == 1 {
                    // From two fingers to one: the view stops; the finger
                    // left does nothing until it is lifted.
                    out.push(release(PointerButton::Middle, t.pos));
                    self.mode = Mode::Spent {
                        tap: (!moved).then_some(most),
                    };
                } else {
                    // Still two or more: grip again at their new middle.
                    out.push(release(PointerButton::Middle, t.pos));
                    let keep = (moved, most);
                    self.pinch(out);
                    if let Mode::Pinch { moved, most, .. } = &mut self.mode {
                        (*moved, *most) = keep;
                    }
                }
            }
            Mode::Spent { tap } => {
                if self.tracks.is_empty() {
                    let still = (t.pos - t.start).length() <= SLOP;
                    if let (Some(most), true) = (tap, still) {
                        self.tapped(most, now, actions);
                    }
                    self.mode = Mode::Idle;
                }
            }
            Mode::Through => {
                if self.tracks.is_empty() {
                    self.mode = Mode::Idle;
                }
            }
            Mode::Idle => {}
        }
        let _ = (area, finger_draws);
    }

    /// A quick tap of `fingers` fingers: undo (two) or redo (three).
    fn tapped(&self, fingers: usize, now: f64, actions: &mut Vec<Action>) {
        if now - self.began < MULTI_TAP {
            match fingers {
                2 => actions.push(Action::Undo),
                n if n >= 3 => actions.push(Action::Redo),
                _ => {}
            }
        }
    }

    /// A finger held still long enough: the menu (right button).
    fn held(&mut self, _area: Option<Area>, out: &mut Vec<Event>, now: f64) {
        if self.mode != Mode::Waiting {
            return;
        }
        if let Some(t) = self.only_finger() {
            if now - t.started >= LONG_PRESS {
                out.push(Event::PointerMoved(t.pos));
                out.push(press(PointerButton::Secondary, t.pos));
                out.push(release(PointerButton::Secondary, t.pos));
                self.mode = Mode::Spent { tap: None };
            }
        }
    }

    /// Starts moving the view with the fingers down.
    fn pinch(&mut self, out: &mut Vec<Event>) {
        let (c, s) = (self.centre(), self.spread());
        out.push(Event::PointerMoved(c));
        out.push(press(PointerButton::Middle, c));
        self.mode = Mode::Pinch {
            centre: c,
            spread: s,
            moved: false,
            most: self.tracks.len(),
        };
    }

    fn fingers(&self) -> impl Iterator<Item = &Track> {
        self.tracks.values().filter(|t| t.kind == Kind::Finger)
    }

    fn only_finger(&self) -> Option<Track> {
        let mut f = self.fingers();
        let first = f.next().copied();
        if f.next().is_some() {
            None
        } else {
            first
        }
    }

    fn last_finger(&self) -> Pos2 {
        self.fingers().last().map_or(Pos2::ZERO, |t| t.pos)
    }

    /// The middle of the fingers.
    fn centre(&self) -> Pos2 {
        let n = self.fingers().count().max(1) as f32;
        let sum = self
            .fingers()
            .fold(Vec2::ZERO, |acc, t| acc + t.pos.to_vec2());
        (sum / n).to_pos2()
    }

    /// How far apart the fingers are (their mean distance to the middle).
    fn spread(&self) -> f32 {
        let c = self.centre();
        let n = self.fingers().count().max(1) as f32;
        self.fingers().map(|t| t.pos.distance(c)).sum::<f32>() / n
    }
}

fn press(button: PointerButton, pos: Pos2) -> Event {
    Event::PointerButton {
        pos,
        button,
        pressed: true,
        modifiers: Modifiers::NONE,
    }
}

fn release(button: PointerButton, pos: Pos2) -> Event {
    Event::PointerButton {
        pos,
        button,
        pressed: false,
        modifiers: Modifiers::NONE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(three_d: bool) -> Option<Area> {
        Some(Area {
            rect: Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(800.0, 600.0)),
            three_d,
        })
    }

    fn touch(id: u64, phase: TouchPhase, x: f32, y: f32) -> Event {
        Event::Touch {
            device_id: egui::TouchDeviceId(0),
            id: TouchId(id),
            phase,
            pos: Pos2::new(x, y),
            force: None,
        }
    }

    /// One frame: the touches given, with what the browser adds for the
    /// first one as a mouse; returns the events egui gets and the actions.
    fn frame(
        t: &mut Touches,
        time: f64,
        events: Vec<Event>,
        three_d: bool,
        pen: bool,
        finger_draws: bool,
    ) -> (Vec<Event>, Vec<Action>) {
        let mut all = Vec::new();
        for e in &events {
            if let Event::Touch { phase, pos, .. } = e {
                all.push(match phase {
                    TouchPhase::Start => press(PointerButton::Primary, *pos),
                    TouchPhase::Move => Event::PointerMoved(*pos),
                    _ => release(PointerButton::Primary, *pos),
                });
            }
            all.push(e.clone());
        }
        let mut raw = egui::RawInput {
            time: Some(time),
            events: all,
            ..Default::default()
        };
        let actions = t.translate(&mut raw, area(three_d), |_| true, |_| pen, finger_draws);
        let events = raw
            .events
            .into_iter()
            .filter(|e| !matches!(e, Event::Touch { .. }))
            .collect();
        (events, actions)
    }

    fn pressed(events: &[Event]) -> Vec<(PointerButton, bool)> {
        events
            .iter()
            .filter_map(|e| match e {
                Event::PointerButton {
                    button, pressed, ..
                } => Some((*button, *pressed)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_pencil_draws_as_the_mouse() {
        let mut t = Touches::default();
        let (e, _) = frame(
            &mut t,
            0.0,
            vec![touch(1, TouchPhase::Start, 300.0, 300.0)],
            false,
            true,
            false,
        );
        assert_eq!(pressed(&e), vec![(PointerButton::Primary, true)]);
        let (e, _) = frame(
            &mut t,
            0.1,
            vec![touch(1, TouchPhase::Move, 350.0, 300.0)],
            false,
            true,
            false,
        );
        assert!(e.contains(&Event::PointerMoved(Pos2::new(350.0, 300.0))));
        let (e, _) = frame(
            &mut t,
            0.2,
            vec![touch(1, TouchPhase::End, 350.0, 300.0)],
            false,
            true,
            false,
        );
        assert_eq!(pressed(&e), vec![(PointerButton::Primary, false)]);
        assert!(t.pen_seen);
    }

    #[test]
    fn a_finger_pans_the_plan_and_turns_the_3d() {
        for (three_d, button) in [
            (false, PointerButton::Middle),
            (true, PointerButton::Secondary),
        ] {
            let mut t = Touches::default();
            let (e, _) = frame(
                &mut t,
                0.0,
                vec![touch(1, TouchPhase::Start, 300.0, 300.0)],
                three_d,
                false,
                false,
            );
            assert!(pressed(&e).is_empty(), "nothing until it moves");
            let (e, _) = frame(
                &mut t,
                0.1,
                vec![touch(1, TouchPhase::Move, 340.0, 300.0)],
                three_d,
                false,
                false,
            );
            assert_eq!(pressed(&e), vec![(button, true)]);
            let (e, _) = frame(
                &mut t,
                0.2,
                vec![touch(1, TouchPhase::End, 380.0, 300.0)],
                three_d,
                false,
                false,
            );
            assert_eq!(pressed(&e), vec![(button, false)]);
        }
    }

    #[test]
    fn a_tap_clicks_and_a_long_press_opens_the_menu() {
        let mut t = Touches::default();
        frame(
            &mut t,
            0.0,
            vec![touch(1, TouchPhase::Start, 300.0, 300.0)],
            false,
            false,
            false,
        );
        let (e, _) = frame(
            &mut t,
            0.1,
            vec![touch(1, TouchPhase::End, 302.0, 300.0)],
            false,
            false,
            false,
        );
        assert_eq!(
            pressed(&e),
            vec![
                (PointerButton::Primary, true),
                (PointerButton::Primary, false)
            ]
        );

        frame(
            &mut t,
            1.0,
            vec![touch(2, TouchPhase::Start, 300.0, 300.0)],
            false,
            false,
            false,
        );
        let (e, _) = frame(&mut t, 1.6, vec![], false, false, false);
        assert_eq!(
            pressed(&e),
            vec![
                (PointerButton::Secondary, true),
                (PointerButton::Secondary, false)
            ]
        );
        let (e, _) = frame(
            &mut t,
            1.8,
            vec![touch(2, TouchPhase::End, 300.0, 300.0)],
            false,
            false,
            false,
        );
        assert!(
            pressed(&e).is_empty(),
            "lifting after the menu does nothing"
        );
    }

    #[test]
    fn two_fingers_pinch_to_zoom_and_tap_to_undo() {
        let mut t = Touches::default();
        frame(
            &mut t,
            0.0,
            vec![touch(1, TouchPhase::Start, 300.0, 300.0)],
            false,
            false,
            false,
        );
        let (e, _) = frame(
            &mut t,
            0.02,
            vec![touch(2, TouchPhase::Start, 400.0, 300.0)],
            false,
            false,
            false,
        );
        assert_eq!(pressed(&e), vec![(PointerButton::Middle, true)]);
        let (e, _) = frame(
            &mut t,
            0.1,
            vec![
                touch(1, TouchPhase::Move, 250.0, 300.0),
                touch(2, TouchPhase::Move, 450.0, 300.0),
            ],
            false,
            false,
            false,
        );
        let zoom: f32 = e
            .iter()
            .filter_map(|e| match e {
                Event::Zoom(z) => Some(*z),
                _ => None,
            })
            .product();
        assert!((zoom - 2.0).abs() < 1e-3, "spread doubled: {zoom}");
        frame(
            &mut t,
            0.2,
            vec![touch(1, TouchPhase::End, 250.0, 300.0)],
            false,
            false,
            false,
        );
        let (e, a) = frame(
            &mut t,
            0.25,
            vec![touch(2, TouchPhase::End, 450.0, 300.0)],
            false,
            false,
            false,
        );
        assert!(a.is_empty(), "a pinch is not an undo");
        assert!(pressed(&e).is_empty());

        // A quick tap of two fingers: undo; of three: redo.
        let mut t = Touches::default();
        frame(
            &mut t,
            0.0,
            vec![
                touch(1, TouchPhase::Start, 300.0, 300.0),
                touch(2, TouchPhase::Start, 380.0, 300.0),
            ],
            false,
            false,
            false,
        );
        let (_, a) = frame(
            &mut t,
            0.15,
            vec![
                touch(1, TouchPhase::End, 300.0, 300.0),
                touch(2, TouchPhase::End, 380.0, 300.0),
            ],
            false,
            false,
            false,
        );
        assert_eq!(a, vec![Action::Undo]);
        let mut t = Touches::default();
        frame(
            &mut t,
            0.0,
            vec![
                touch(1, TouchPhase::Start, 300.0, 300.0),
                touch(2, TouchPhase::Start, 380.0, 300.0),
                touch(3, TouchPhase::Start, 340.0, 360.0),
            ],
            false,
            false,
            false,
        );
        let (_, a) = frame(
            &mut t,
            0.2,
            vec![
                touch(1, TouchPhase::End, 300.0, 300.0),
                touch(2, TouchPhase::End, 380.0, 300.0),
                touch(3, TouchPhase::End, 340.0, 360.0),
            ],
            false,
            false,
            false,
        );
        assert_eq!(a, vec![Action::Redo]);
    }

    #[test]
    fn with_finger_drawing_one_finger_drags_as_the_mouse() {
        let mut t = Touches::default();
        frame(
            &mut t,
            0.0,
            vec![touch(1, TouchPhase::Start, 300.0, 300.0)],
            false,
            false,
            true,
        );
        let (e, _) = frame(
            &mut t,
            0.1,
            vec![touch(1, TouchPhase::Move, 360.0, 300.0)],
            false,
            false,
            true,
        );
        assert_eq!(pressed(&e), vec![(PointerButton::Primary, true)]);
        assert_eq!(
            e.first(),
            Some(&Event::PointerMoved(Pos2::new(300.0, 300.0))),
            "it starts where the finger did"
        );
    }

    #[test]
    fn touches_on_the_panels_are_left_alone() {
        let mut t = Touches::default();
        let (e, _) = frame(
            &mut t,
            0.0,
            vec![touch(1, TouchPhase::Start, 50.0, 50.0)],
            false,
            false,
            false,
        );
        assert_eq!(pressed(&e), vec![(PointerButton::Primary, true)]);
        let (e, _) = frame(
            &mut t,
            0.1,
            vec![touch(1, TouchPhase::End, 50.0, 50.0)],
            false,
            false,
            false,
        );
        assert_eq!(pressed(&e), vec![(PointerButton::Primary, false)]);
    }
}
