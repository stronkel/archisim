//! The interface's icons, drawn as lines (no image files, no fonts): they
//! stay sharp at any screen resolution and take the colour of the text
//! around them. Each one is drawn on a 24 × 24 grid scaled to its rect.

use eframe::egui::{Color32, Painter, Pos2, Rect, Shape, Stroke};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Arrow,
    Wall,
    Slab,
    Door,
    Window,
    Column,
    Beam,
    Roof,
    Stair,
    Object,
    Room,
    Dimension,
    Text,
    Tag,
    Hatch,
    Section,
    Elevation,
    Plan,
    ThreeD,
    Sheet,
    Undo,
    Redo,
    Save,
    Export,
    Sidebar,
    Close,
    ChevronLeft,
    ChevronRight,
    ChevronDown,
    Theme,
    Plus,
    Minus,
    Fit,
}

/// Draws `icon` in `rect` with lines of `color`.
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let s = rect.width().min(rect.height()) / 24.0;
    let o = rect.center() - eframe::egui::Vec2::splat(12.0 * s);
    let p = |x: f32, y: f32| Pos2::new(o.x + x * s, o.y + y * s);
    let w = (1.6 * s).max(1.0);
    let thin = (1.0 * s).max(0.8);
    let st = Stroke::new(w, color);
    let st_thin = Stroke::new(thin, color);
    let line = |pts: &[(f32, f32)], stroke: Stroke| {
        let v: Vec<Pos2> = pts.iter().map(|&(x, y)| p(x, y)).collect();
        painter.add(Shape::line(v, stroke));
    };
    let closed = |pts: &[(f32, f32)], stroke: Stroke| {
        let v: Vec<Pos2> = pts.iter().map(|&(x, y)| p(x, y)).collect();
        painter.add(Shape::closed_line(v, stroke));
    };
    let filled = |pts: &[(f32, f32)]| {
        let v: Vec<Pos2> = pts.iter().map(|&(x, y)| p(x, y)).collect();
        painter.add(Shape::convex_polygon(v, color, Stroke::NONE));
    };
    let rect_ = |x0: f32, y0: f32, x1: f32, y1: f32, stroke: Stroke| {
        closed(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)], stroke)
    };
    let arc = |cx: f32, cy: f32, r: f32, a0: f32, a1: f32, stroke: Stroke| {
        let n = 16;
        let v: Vec<Pos2> = (0..=n)
            .map(|i| {
                let a = (a0 + (a1 - a0) * i as f32 / n as f32).to_radians();
                p(cx + r * a.cos(), cy + r * a.sin())
            })
            .collect();
        painter.add(Shape::line(v, stroke));
    };
    let circle = |cx: f32, cy: f32, r: f32, stroke: Stroke| {
        painter.circle_stroke(p(cx, cy), r * s, stroke);
    };
    let dashed = |a: (f32, f32), b: (f32, f32), stroke: Stroke| {
        painter.extend(Shape::dashed_line(
            &[p(a.0, a.1), p(b.0, b.1)],
            stroke,
            3.0 * s,
            2.0 * s,
        ));
    };

    match icon {
        Icon::Arrow => closed(&[(6.0, 3.0), (19.0, 11.0), (13.0, 13.0), (10.0, 19.5)], st),
        Icon::Wall => {
            rect_(2.5, 8.0, 21.5, 16.0, st);
            line(&[(2.5, 11.0), (21.5, 11.0)], st_thin);
            line(&[(2.5, 13.0), (21.5, 13.0)], st_thin);
        }
        Icon::Slab => {
            closed(&[(2.0, 13.0), (9.0, 8.0), (22.0, 8.0), (15.0, 13.0)], st);
            line(
                &[
                    (2.0, 13.0),
                    (2.0, 16.0),
                    (15.0, 16.0),
                    (22.0, 11.0),
                    (22.0, 8.0),
                ],
                st,
            );
            line(&[(15.0, 13.0), (15.0, 16.0)], st);
        }
        Icon::Door => {
            line(&[(2.0, 20.0), (7.0, 20.0)], st);
            line(&[(17.0, 20.0), (22.0, 20.0)], st);
            line(&[(7.0, 20.0), (7.0, 6.0)], st);
            arc(7.0, 20.0, 14.0, -90.0, 0.0, st_thin);
        }
        Icon::Window => {
            rect_(4.0, 4.0, 20.0, 20.0, st);
            line(&[(12.0, 4.0), (12.0, 20.0)], st_thin);
            line(&[(4.0, 12.0), (20.0, 12.0)], st_thin);
        }
        Icon::Column => {
            rect_(7.0, 3.0, 17.0, 21.0, st);
            line(&[(7.0, 3.0), (17.0, 21.0)], st_thin);
            line(&[(17.0, 3.0), (7.0, 21.0)], st_thin);
        }
        Icon::Beam => {
            line(&[(2.0, 8.0), (22.0, 8.0)], st);
            line(&[(2.0, 16.0), (22.0, 16.0)], st);
            line(&[(5.0, 8.0), (5.0, 16.0)], st);
            line(&[(19.0, 8.0), (19.0, 16.0)], st);
        }
        Icon::Roof => {
            line(&[(2.0, 14.0), (12.0, 5.0), (22.0, 14.0)], st);
            line(&[(5.0, 12.0), (5.0, 19.0), (19.0, 19.0), (19.0, 12.0)], st);
        }
        Icon::Stair => line(
            &[
                (3.0, 20.0),
                (7.0, 20.0),
                (7.0, 16.0),
                (11.0, 16.0),
                (11.0, 12.0),
                (15.0, 12.0),
                (15.0, 8.0),
                (19.0, 8.0),
                (19.0, 4.0),
                (21.0, 4.0),
            ],
            st,
        ),
        Icon::Object => {
            rect_(4.0, 10.0, 20.0, 15.0, st);
            line(&[(6.0, 15.0), (6.0, 19.0)], st);
            line(&[(18.0, 15.0), (18.0, 19.0)], st);
            line(&[(6.0, 10.0), (6.0, 6.0), (18.0, 6.0), (18.0, 10.0)], st);
        }
        Icon::Room => {
            for (a, b) in [
                ((3.0, 4.0), (21.0, 4.0)),
                ((21.0, 4.0), (21.0, 20.0)),
                ((21.0, 20.0), (3.0, 20.0)),
                ((3.0, 20.0), (3.0, 4.0)),
            ] {
                dashed(a, b, st);
            }
            line(&[(8.0, 11.0), (16.0, 11.0)], st_thin);
            line(&[(9.0, 14.0), (15.0, 14.0)], st_thin);
        }
        Icon::Dimension => {
            line(&[(3.0, 14.0), (21.0, 14.0)], st);
            line(&[(5.0, 9.0), (5.0, 18.0)], st_thin);
            line(&[(19.0, 9.0), (19.0, 18.0)], st_thin);
            line(&[(3.5, 15.5), (6.5, 12.5)], st);
            line(&[(17.5, 15.5), (20.5, 12.5)], st);
        }
        Icon::Text => {
            line(&[(5.0, 7.0), (5.0, 4.0), (19.0, 4.0), (19.0, 7.0)], st);
            line(&[(12.0, 4.0), (12.0, 20.0)], st);
            line(&[(9.0, 20.0), (15.0, 20.0)], st);
        }
        Icon::Tag => {
            closed(
                &[
                    (3.0, 8.0),
                    (15.0, 8.0),
                    (20.0, 12.0),
                    (15.0, 16.0),
                    (3.0, 16.0),
                ],
                st,
            );
            circle(7.0, 12.0, 1.2, st_thin);
        }
        Icon::Hatch => {
            rect_(4.0, 4.0, 20.0, 20.0, st);
            line(&[(4.0, 12.0), (12.0, 4.0)], st_thin);
            line(&[(4.0, 20.0), (20.0, 4.0)], st_thin);
            line(&[(12.0, 20.0), (20.0, 12.0)], st_thin);
        }
        Icon::Section => {
            dashed((12.0, 2.0), (12.0, 22.0), st_thin);
            line(&[(12.0, 4.0), (18.0, 4.0), (15.0, 7.0)], st);
            line(&[(12.0, 20.0), (18.0, 20.0), (15.0, 17.0)], st);
        }
        Icon::Elevation => {
            circle(12.0, 12.0, 5.0, st);
            line(&[(12.0, 4.0), (12.0, 7.0)], st);
            line(&[(12.0, 17.0), (12.0, 20.0)], st);
            filled(&[(12.0, 7.0), (7.0, 12.0), (12.0, 17.0)]);
        }
        Icon::Plan => {
            rect_(3.0, 5.0, 21.0, 19.0, st);
            line(&[(3.0, 12.0), (12.0, 12.0), (12.0, 19.0)], st_thin);
            line(&[(12.0, 9.0), (21.0, 9.0)], st_thin);
        }
        Icon::ThreeD => {
            closed(
                &[
                    (12.0, 3.0),
                    (20.0, 7.5),
                    (20.0, 16.5),
                    (12.0, 21.0),
                    (4.0, 16.5),
                    (4.0, 7.5),
                ],
                st,
            );
            line(&[(4.0, 7.5), (12.0, 12.0), (20.0, 7.5)], st);
            line(&[(12.0, 12.0), (12.0, 21.0)], st);
        }
        Icon::Sheet => {
            rect_(3.0, 5.0, 21.0, 19.0, st);
            rect_(14.0, 14.0, 19.5, 17.5, st_thin);
            for (a, b) in [
                ((5.0, 7.0), (12.0, 7.0)),
                ((12.0, 7.0), (12.0, 13.0)),
                ((12.0, 13.0), (5.0, 13.0)),
                ((5.0, 13.0), (5.0, 7.0)),
            ] {
                dashed(a, b, st_thin);
            }
        }
        Icon::Undo => {
            line(&[(9.0, 14.0), (4.0, 9.0), (9.0, 4.0)], st);
            line(&[(4.0, 9.0), (15.0, 9.0)], st);
            arc(15.0, 14.0, 5.0, -90.0, 90.0, st);
            line(&[(15.0, 19.0), (11.0, 19.0)], st);
        }
        Icon::Redo => {
            line(&[(15.0, 14.0), (20.0, 9.0), (15.0, 4.0)], st);
            line(&[(20.0, 9.0), (9.0, 9.0)], st);
            arc(9.0, 14.0, 5.0, 90.0, 270.0, st);
            line(&[(9.0, 19.0), (13.0, 19.0)], st);
        }
        Icon::Save => {
            closed(
                &[
                    (5.0, 3.0),
                    (16.0, 3.0),
                    (19.0, 6.0),
                    (19.0, 21.0),
                    (5.0, 21.0),
                ],
                st,
            );
            line(&[(8.0, 3.0), (8.0, 9.0), (16.0, 9.0), (16.0, 3.0)], st);
        }
        Icon::Export => {
            line(&[(12.0, 3.0), (12.0, 15.0)], st);
            line(&[(7.0, 10.0), (12.0, 15.0), (17.0, 10.0)], st);
            line(&[(4.0, 20.0), (20.0, 20.0)], st);
        }
        Icon::Sidebar => {
            rect_(3.0, 4.0, 21.0, 20.0, st);
            line(&[(9.0, 4.0), (9.0, 20.0)], st);
        }
        Icon::Close => {
            line(&[(6.0, 6.0), (18.0, 18.0)], st);
            line(&[(18.0, 6.0), (6.0, 18.0)], st);
        }
        Icon::ChevronLeft => line(&[(15.0, 6.0), (9.0, 12.0), (15.0, 18.0)], st),
        Icon::ChevronRight => line(&[(9.0, 6.0), (15.0, 12.0), (9.0, 18.0)], st),
        Icon::ChevronDown => line(&[(6.0, 9.0), (12.0, 15.0), (18.0, 9.0)], st),
        Icon::Theme => {
            circle(12.0, 12.0, 8.0, st);
            let v: Vec<Pos2> = (0..=16)
                .map(|i| {
                    let a = (-90.0 + 180.0 * i as f32 / 16.0f32).to_radians();
                    p(12.0 + 8.0 * a.cos(), 12.0 + 8.0 * a.sin())
                })
                .collect();
            painter.add(Shape::convex_polygon(v, color, Stroke::NONE));
        }
        Icon::Plus => {
            line(&[(12.0, 5.0), (12.0, 19.0)], st);
            line(&[(5.0, 12.0), (19.0, 12.0)], st);
        }
        Icon::Minus => line(&[(5.0, 12.0), (19.0, 12.0)], st),
        Icon::Fit => {
            line(&[(4.0, 9.0), (4.0, 4.0), (9.0, 4.0)], st);
            line(&[(15.0, 4.0), (20.0, 4.0), (20.0, 9.0)], st);
            line(&[(20.0, 15.0), (20.0, 20.0), (15.0, 20.0)], st);
            line(&[(9.0, 20.0), (4.0, 20.0), (4.0, 15.0)], st);
        }
    }
}
