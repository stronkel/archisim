//! Lifts in plan (H4), on every level they cross: the shaft's walls cut
//! (when the lift has them), the void of the shaft crossed by its two
//! diagonals, the car with its counterweight and rails, the landing door
//! in front, and its name with its load.

use std::fmt::Write as _;

use archisim_core::{Elevator, LevelId, Model, Point2};

use crate::lettering::{text_path, Align};
use crate::{num, path, pens, pt, INK};

/// The points that mark a lift, for the snaps.
pub fn elevator_points(e: &Elevator) -> Vec<Point2> {
    let mut v = e.inside();
    v.extend(e.outside());
    v.push(e.position);
    v
}

fn rect(e: &Elevator, x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point2> {
    [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
        .into_iter()
        .map(|(x, y)| e.point_at(x, y))
        .collect()
}

/// One lift on a level: the car drawn where it is (dashed elsewhere) and
/// the landing door where it stops.
pub fn draw_elevator(s: &mut String, model: &Model, e: &Elevator, level: LevelId, k: f64) {
    let (hw, hd) = (e.shaft_w / 2.0, e.shaft_d / 2.0);
    let line = |s: &mut String, a: Point2, b: Point2, w: f64| {
        let _ = writeln!(
            s,
            r#"<path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
            pt(a),
            pt(b),
            num(w * k)
        );
    };
    let poly = |s: &mut String, p: &[Point2], fill: &str, w: f64| {
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="{fill}" stroke="{INK}" stroke-width="{}"/>"#,
            path(p),
            num(w * k)
        );
    };
    // The walls, cut: the outside less the inside, but for the door.
    if e.wall > 0.0 {
        let t = e.wall;
        let dw = (e.door_w / 2.0 + 0.06).min(hw);
        let c = e
            .material
            .map_or([189, 186, 180], |m| model.surface_color(m));
        let fill = format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
        for (x0, y0, x1, y1) in [
            (-hw - t, -hd - t, hw + t, -hd),
            (-hw - t, -hd, -hw, hd),
            (hw, -hd, hw + t, hd),
            (-hw - t, hd, -dw, hd + t),
            (dw, hd, hw + t, hd + t),
        ] {
            poly(s, &rect(e, x0, y0, x1, y1), &fill, pens().cut);
        }
    }
    let inside = e.inside();
    poly(s, &inside, "none", pens().view);
    // The void, crossed.
    line(s, inside[0], inside[2], pens().thin);
    line(s, inside[1], inside[3], pens().thin);
    // The car, its counterweight and rails.
    let stops = model.elevator_stops(e.id);
    let car_level = e.car_at.unwrap_or(e.level);
    let cy = e.car_centre_y();
    let car = rect(
        e,
        -e.car_w / 2.0,
        cy - e.car_d / 2.0,
        e.car_w / 2.0,
        cy + e.car_d / 2.0,
    );
    if car_level == level {
        poly(s, &car, "white", pens().view);
        line(s, car[0], car[2], pens().thin);
        line(s, car[1], car[3], pens().thin);
    } else {
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="none" stroke="{INK}" stroke-width="{}" stroke-dasharray="{} {}"/>"#,
            path(&car),
            num(pens().thin * k),
            num(1.5 * k),
            num(1.0 * k)
        );
    }
    let rx = (e.car_w / 2.0 + 0.09).min(hw - 0.03);
    for sx in [-1.0, 1.0] {
        poly(
            s,
            &rect(e, sx * rx - 0.008, cy - 0.045, sx * rx + 0.008, cy + 0.045),
            INK,
            pens().thin,
        );
    }
    if e.shaft_w - e.car_w > 0.3 {
        let cx = -hw + 0.09;
        poly(
            s,
            &rect(e, cx - 0.06, -0.35, cx + 0.06, 0.35),
            "white",
            pens().view,
        );
    }
    // The landing door at the levels it stops at.
    if stops.contains(&level) {
        let face = hd + e.wall;
        let dw = e.door_w / 2.0;
        poly(
            s,
            &rect(e, -dw, face - 0.05, -0.003, face - 0.025),
            "white",
            pens().thin,
        );
        poly(
            s,
            &rect(e, 0.003, face - 0.03, dw, face - 0.005),
            "white",
            pens().thin,
        );
        line(
            s,
            e.point_at(-dw - 0.05, face),
            e.point_at(-dw - 0.05, face + 0.04),
            pens().view,
        );
        line(
            s,
            e.point_at(dw + 0.05, face),
            e.point_at(dw + 0.05, face + 0.04),
            pens().view,
        );
    }
    // Its name and load.
    let h = 2.0 * k;
    for (i, text) in [
        e.name.clone(),
        format!("{} kg · {} pers.", e.load, e.persons()),
    ]
    .iter()
    .enumerate()
    {
        let at = e.point_at(0.0, -hd * 0.5) - Point2::new(0.0, (i as f64 * 1.6 + 0.5) * h);
        let _ = writeln!(
            s,
            r#"<path d="{}" {}/>"#,
            text_path(text, at, Point2::new(1.0, 0.0), h, Align::Centre),
            crate::lettering::paint(h, INK)
        );
    }
}

/// The lifts of a level.
pub fn draw_elevators(
    s: &mut String,
    model: &Model,
    level: LevelId,
    k: f64,
    visible: &dyn Fn(&[Point2]) -> bool,
) {
    if model.elevators_on_level(level).next().is_none() {
        return;
    }
    s.push_str("<g id=\"elevators\">\n");
    for e in model.elevators_on_level(level) {
        if !visible(&e.outside()) {
            continue;
        }
        crate::dim::element(s, e.id.0, |s| draw_elevator(s, model, e, level, k));
    }
    s.push_str("</g>\n");
}
