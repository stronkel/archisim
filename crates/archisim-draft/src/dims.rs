//! Drafted dimensions (angles, radii, diameters and arc lengths): what
//! they draw, their text, and how they are moved and edited.

use archisim_core::{Measure, Point2, Text, TextAlign};

use crate::seg::{dir, Seg};
use crate::transform::Transform;

/// Height of a dimension's text, paper millimetres (the graphic standard
/// may draw it otherwise; picking uses this).
pub const DIM_TEXT: f64 = 2.5;
/// Length and half width of the arrowheads, paper millimetres.
const ARROW: (f64, f64) = (2.5, 0.45);
/// Gap between the drawing and the text, paper millimetres.
const GAP: f64 = 1.0;

/// What a dimension draws: lines and arcs, filled arrowheads, its text.
#[derive(Debug, Clone, PartialEq)]
pub struct DimDrawing {
    pub segs: Vec<Seg>,
    pub arrows: Vec<[Point2; 3]>,
    pub text: Text,
}

/// A length as dimensions write it: `3,45`.
fn length(v: f64) -> String {
    let v = (v * 100.0).round() / 100.0;
    let v = if v == 0.0 { 0.0 } else { v };
    format!("{v:.2}").replace('.', ",")
}

/// The text of a dimension: `37,5°`, `R 1,25`, `Ø 2,50`, `⌒ 3,14`.
pub fn label(m: &Measure) -> String {
    match m {
        Measure::Angle { .. } => {
            let d = (m.value().to_degrees() * 100.0).round() / 100.0;
            let s = format!("{d:.2}");
            let s = s.trim_end_matches('0').trim_end_matches('.');
            format!("{}°", s.replace('.', ","))
        }
        Measure::Radius { .. } => format!("R {}", length(m.value())),
        Measure::Diameter { .. } => format!("Ø {}", length(m.value())),
        Measure::ArcLength { .. } => format!("⌒ {}", length(m.value())),
    }
}

fn arrow(tip: Point2, towards_tip: Point2, k: f64) -> [Point2; 3] {
    let d = towards_tip.normalized().unwrap_or(Point2::new(1.0, 0.0));
    let n = d.perp_left();
    let base = tip - d * (ARROW.0 * k);
    [tip, base + n * (ARROW.1 * k), base - n * (ARROW.1 * k)]
}

/// A text beside a point, on the side `u` points to, read horizontally.
fn text_at(content: String, p: Point2, u: Point2, k: f64, h: f64) -> Text {
    let align = if u.x > 0.5 {
        TextAlign::MiddleLeft
    } else if u.x < -0.5 {
        TextAlign::MiddleRight
    } else if u.y >= 0.0 {
        TextAlign::BottomCentre
    } else {
        TextAlign::TopCentre
    };
    Text {
        at: p + u * (GAP * k),
        content,
        height: h,
        angle: 0.0,
        align,
    }
}

/// The main lines and arcs of a dimension (no arrows, no text).
pub fn main_segs(m: &Measure) -> Vec<Seg> {
    match *m {
        Measure::Angle {
            centre,
            start,
            sweep,
            radius,
        } => vec![Seg::Arc {
            centre,
            radius,
            start,
            sweep,
        }],
        Measure::Radius {
            centre,
            radius,
            angle,
            reach,
        } => {
            let u = dir(angle);
            vec![Seg::Line {
                a: centre,
                b: centre + u * radius.max(reach),
            }]
        }
        Measure::Diameter {
            centre,
            radius,
            angle,
            reach,
        } => {
            let u = dir(angle);
            vec![Seg::Line {
                a: centre - u * radius,
                b: centre + u * radius.max(reach),
            }]
        }
        Measure::ArcLength {
            centre,
            radius,
            start,
            sweep,
            offset,
        } => vec![Seg::Arc {
            centre,
            radius: (radius + offset).max(1e-6),
            start,
            sweep,
        }],
    }
}

/// Everything a dimension draws, `k` drawing units per paper millimetre,
/// its text `text_mm` high.
pub fn drawing(m: &Measure, k: f64, text_mm: f64) -> DimDrawing {
    let mut segs = main_segs(m);
    let mut arrows = Vec::new();
    let text = match *m {
        Measure::Angle {
            centre,
            start,
            sweep,
            radius,
        } => {
            let s = segs[0];
            arrows.push(arrow(s.start(), -s.tangent(0.0), k));
            arrows.push(arrow(s.end(), s.tangent(1.0), k));
            let u = dir(start + sweep / 2.0);
            text_at(label(m), centre + u * radius, u, k, text_mm)
        }
        Measure::Radius {
            centre,
            radius,
            angle,
            reach,
        } => {
            let u = dir(angle);
            let p = centre + u * radius;
            // Inside the circle the arrow points out to it; outside, in.
            arrows.push(arrow(p, if reach > radius { -u } else { u }, k));
            let at = centre + u * reach.max(radius);
            text_at(label(m), at, u, k, text_mm)
        }
        Measure::Diameter {
            centre,
            radius,
            angle,
            reach,
        } => {
            let u = dir(angle);
            let outside = reach > radius;
            arrows.push(arrow(centre + u * radius, if outside { -u } else { u }, k));
            arrows.push(arrow(centre - u * radius, -u, k));
            let at = centre + u * reach.max(radius);
            text_at(label(m), at, u, k, text_mm)
        }
        Measure::ArcLength {
            centre,
            radius,
            start,
            sweep,
            offset,
        } => {
            let s = segs[0];
            arrows.push(arrow(s.start(), -s.tangent(0.0), k));
            arrows.push(arrow(s.end(), s.tangent(1.0), k));
            // Extension lines from the arc to past the dimension.
            let r = radius + offset;
            let gap = GAP * k;
            if (r - radius).abs() > gap {
                let sign = if r > radius { 1.0 } else { -1.0 };
                for a in [start, start + sweep] {
                    let u = dir(a);
                    segs.push(Seg::Line {
                        a: centre + u * (radius + sign * gap),
                        b: centre + u * (r + sign * 1.5 * gap),
                    });
                }
            }
            let u = dir(start + sweep / 2.0);
            text_at(label(m), centre + u * r, u, k, text_mm)
        }
    };
    DimDrawing { segs, arrows, text }
}

/// A dimension after a transform: it keeps measuring the moved geometry.
pub fn transform(m: &Measure, t: &Transform) -> Measure {
    let f = t.factor();
    let arc_start = |start: f64, sweep: f64| {
        if t.is_mirror() {
            t.angle(start + sweep)
        } else {
            t.angle(start)
        }
    };
    match *m {
        Measure::Angle {
            centre,
            start,
            sweep,
            radius,
        } => Measure::Angle {
            centre: t.apply(centre),
            start: arc_start(start, sweep),
            sweep,
            radius: radius * f,
        },
        Measure::Radius {
            centre,
            radius,
            angle,
            reach,
        } => Measure::Radius {
            centre: t.apply(centre),
            radius: radius * f,
            angle: t.angle(angle),
            reach: reach * f,
        },
        Measure::Diameter {
            centre,
            radius,
            angle,
            reach,
        } => Measure::Diameter {
            centre: t.apply(centre),
            radius: radius * f,
            angle: t.angle(angle),
            reach: reach * f,
        },
        Measure::ArcLength {
            centre,
            radius,
            start,
            sweep,
            offset,
        } => Measure::ArcLength {
            centre: t.apply(centre),
            radius: radius * f,
            start: arc_start(start, sweep),
            sweep,
            offset: offset * f,
        },
    }
}

/// Where a dimension is held: its centre (moves it) and the point that
/// places its line or text.
pub fn handles(m: &Measure) -> [Point2; 2] {
    let c = m.centre();
    let second = match *m {
        Measure::Angle {
            centre,
            start,
            sweep,
            radius,
        } => centre + dir(start + sweep / 2.0) * radius,
        Measure::Radius {
            centre,
            radius,
            angle,
            reach,
        }
        | Measure::Diameter {
            centre,
            radius,
            angle,
            reach,
        } => centre + dir(angle) * reach.max(radius),
        Measure::ArcLength {
            centre,
            radius,
            start,
            sweep,
            offset,
        } => centre + dir(start + sweep / 2.0) * (radius + offset),
    };
    [c, second]
}

/// The dimension with its line or text placed through `to` (its centre
/// and what it measures stay).
pub fn place(m: &Measure, to: Point2) -> Measure {
    let c = m.centre();
    let d = to - c;
    let r = d.length();
    let mut out = *m;
    match &mut out {
        Measure::Angle { radius, .. } => *radius = r.max(1e-6),
        Measure::Radius { angle, reach, .. } | Measure::Diameter { angle, reach, .. } => {
            if r > 1e-9 {
                *angle = d.y.atan2(d.x);
            }
            *reach = r;
        }
        Measure::ArcLength { radius, offset, .. } => *offset = r - *radius,
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    #[test]
    fn values_texts_and_transforms() {
        let a = Measure::Angle {
            centre: Point2::new(0.0, 0.0),
            start: 0.0,
            sweep: FRAC_PI_2 * 0.75,
            radius: 1.0,
        };
        assert_eq!(label(&a), "67,5°");
        let d = drawing(&a, 0.05, 2.5);
        assert_eq!(d.arrows.len(), 2);
        // Arrow tips on the arc ends.
        assert!(d.arrows[0][0].distance(Point2::new(1.0, 0.0)) < 1e-9);
        let r = Measure::Diameter {
            centre: Point2::new(1.0, 1.0),
            radius: 1.25,
            angle: 0.0,
            reach: 2.0,
        };
        assert_eq!(label(&r), "Ø 2,50");
        // Scaled ×2 about the origin: twice the diameter.
        let s = transform(&r, &Transform::scale(Point2::new(0.0, 0.0), 2.0));
        assert_eq!(label(&s), "Ø 5,00");
        // Mirrored, an angle keeps its value.
        let m = transform(
            &a,
            &Transform::mirror(Point2::new(0.0, 0.0), Point2::new(0.0, 1.0)),
        );
        assert_eq!(label(&m), "67,5°");
        let arc = Measure::ArcLength {
            centre: Point2::new(0.0, 0.0),
            radius: 2.0,
            start: 0.0,
            sweep: std::f64::consts::PI,
            offset: 0.5,
        };
        assert_eq!(label(&arc), "⌒ 6,28");
        assert_eq!(drawing(&arc, 0.05, 2.5).segs.len(), 3);
        // Placing moves the line, not the value.
        let moved = place(&arc, Point2::new(0.0, 3.0));
        assert_eq!(label(&moved), "⌒ 6,28");
        assert!(handles(&moved)[1].distance(Point2::new(0.0, 3.0)) < 1e-9);
    }
}
