//! Moving, turning, scaling and mirroring shapes (AutoCAD's DESPLAZA, GIRA,
//! ESCALA and SIMETRIA).

use std::f64::consts::PI;

use archisim_core::{Hatch, Point2, Polyline, Shape, Text, Vec2};

use crate::seg::{angle_of, dir};

/// A similarity of the plane: `p' = M·p + t`, where `M` is a rotation times
/// a uniform scale, possibly with a reflection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    m: [[f64; 2]; 2],
    t: Vec2,
}

impl Transform {
    pub fn identity() -> Self {
        Self {
            m: [[1.0, 0.0], [0.0, 1.0]],
            t: Vec2::new(0.0, 0.0),
        }
    }

    pub fn translate(d: Vec2) -> Self {
        Self {
            t: d,
            ..Self::identity()
        }
    }

    /// Turning by `angle` (counter-clockwise) about `centre`.
    pub fn rotate(centre: Point2, angle: f64) -> Self {
        let (s, c) = angle.sin_cos();
        let m = [[c, -s], [s, c]];
        Self::about(centre, m)
    }

    /// Scaling by `factor` about `centre`.
    pub fn scale(centre: Point2, factor: f64) -> Self {
        Self::about(centre, [[factor, 0.0], [0.0, factor]])
    }

    /// Reflection in the line through `a` and `b`.
    pub fn mirror(a: Point2, b: Point2) -> Self {
        let th = angle_of(b - a);
        let (s, c) = (2.0 * th).sin_cos();
        Self::about(a, [[c, s], [s, -c]])
    }

    fn about(centre: Point2, m: [[f64; 2]; 2]) -> Self {
        let mc = Vec2::new(
            m[0][0] * centre.x + m[0][1] * centre.y,
            m[1][0] * centre.x + m[1][1] * centre.y,
        );
        Self { m, t: centre - mc }
    }

    /// First `self`, then `other`.
    pub fn then(&self, other: &Transform) -> Transform {
        let a = &other.m;
        let b = &self.m;
        let m = [
            [
                a[0][0] * b[0][0] + a[0][1] * b[1][0],
                a[0][0] * b[0][1] + a[0][1] * b[1][1],
            ],
            [
                a[1][0] * b[0][0] + a[1][1] * b[1][0],
                a[1][0] * b[0][1] + a[1][1] * b[1][1],
            ],
        ];
        Transform {
            m,
            t: other.apply(self.t),
        }
    }

    pub fn apply(&self, p: Point2) -> Point2 {
        self.linear(p) + self.t
    }

    /// The transform of a direction or displacement (no translation).
    pub fn linear(&self, v: Vec2) -> Vec2 {
        Vec2::new(
            self.m[0][0] * v.x + self.m[0][1] * v.y,
            self.m[1][0] * v.x + self.m[1][1] * v.y,
        )
    }

    /// Whether it mirrors (reverses the sense of turning).
    pub fn is_mirror(&self) -> bool {
        self.m[0][0] * self.m[1][1] - self.m[0][1] * self.m[1][0] < 0.0
    }

    /// The uniform scale factor.
    pub fn factor(&self) -> f64 {
        (self.m[0][0] * self.m[1][1] - self.m[0][1] * self.m[1][0])
            .abs()
            .sqrt()
    }

    /// The direction an angle has after the transform.
    pub fn angle(&self, a: f64) -> f64 {
        angle_of(self.linear(dir(a)))
    }
}

fn polyline(p: &Polyline, t: &Transform) -> Polyline {
    let flip = if t.is_mirror() { -1.0 } else { 1.0 };
    Polyline {
        points: p.points.iter().map(|q| t.apply(*q)).collect(),
        bulges: p.bulges.iter().map(|b| b * flip).collect(),
        closed: p.closed,
        width: p.width * t.factor(),
    }
}

/// The angle of a text's baseline turned so it reads from below or from
/// the right (AutoCAD's mirrored text, MIRRTEXT = 0).
pub fn readable(a: f64) -> f64 {
    let mut a = (a + PI).rem_euclid(2.0 * PI) - PI;
    if a <= -PI / 2.0 - 1e-9 {
        a += PI;
    } else if a > PI / 2.0 + 1e-9 {
        a -= PI;
    }
    a
}

/// A shape after a transform. Texts move, turn and grow with it but are
/// never written backwards: a mirrored text keeps reading normally, as in
/// AutoCAD.
pub fn transform_shape(shape: &Shape, t: &Transform) -> Shape {
    let k = t.factor();
    match shape {
        Shape::Line { a, b } => Shape::Line {
            a: t.apply(*a),
            b: t.apply(*b),
        },
        Shape::Polyline(p) => Shape::Polyline(polyline(p, t)),
        Shape::Circle { centre, radius } => Shape::Circle {
            centre: t.apply(*centre),
            radius: radius * k,
        },
        Shape::Arc {
            centre,
            radius,
            start,
            sweep,
        } => {
            let s = if t.is_mirror() {
                t.angle(start + sweep)
            } else {
                t.angle(*start)
            };
            Shape::Arc {
                centre: t.apply(*centre),
                radius: radius * k,
                start: s,
                sweep: *sweep,
            }
        }
        Shape::Ellipse {
            centre,
            axis,
            ratio,
            start,
            sweep,
        } => Shape::Ellipse {
            centre: t.apply(*centre),
            axis: t.linear(*axis),
            ratio: *ratio,
            start: if t.is_mirror() {
                -(start + sweep)
            } else {
                *start
            },
            sweep: *sweep,
        },
        Shape::Point { at } => Shape::Point { at: t.apply(*at) },
        Shape::Text(x) => {
            let angle = t.angle(x.angle);
            Shape::Text(Text {
                at: t.apply(x.at),
                angle: if t.is_mirror() {
                    readable(angle)
                } else {
                    angle
                },
                height: x.height * k,
                ..x.clone()
            })
        }
        Shape::Hatch(h) => Shape::Hatch(Hatch {
            loops: h.loops.iter().map(|l| polyline(l, t)).collect(),
            angle: t.angle(h.angle),
            scale: if h.pattern.is_model_scale() {
                h.scale * k
            } else {
                h.scale
            },
            ..h.clone()
        }),
        Shape::Dim(m) => Shape::Dim(crate::dims::transform(m, t)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    #[test]
    fn rotate_scale_mirror_points() {
        let r = Transform::rotate(p(1.0, 1.0), FRAC_PI_2);
        assert!(r.apply(p(2.0, 1.0)).approx_eq(p(1.0, 2.0)));
        let s = Transform::scale(p(1.0, 1.0), 2.0);
        assert!(s.apply(p(2.0, 1.0)).approx_eq(p(3.0, 1.0)));
        assert!((s.factor() - 2.0).abs() < 1e-12);
        let m = Transform::mirror(p(0.0, 0.0), p(1.0, 1.0));
        assert!(m.apply(p(2.0, 0.0)).approx_eq(p(0.0, 2.0)));
        assert!(m.is_mirror());
        let both = r.then(&s);
        assert!(both.apply(p(2.0, 1.0)).approx_eq(p(1.0, 3.0)));
    }

    #[test]
    fn mirrored_arc_and_polyline_keep_their_points() {
        let arc = Shape::Arc {
            centre: p(0.0, 0.0),
            radius: 1.0,
            start: 0.0,
            sweep: FRAC_PI_2,
        };
        let m = Transform::mirror(p(0.0, 0.0), p(0.0, 1.0));
        let Shape::Arc { start, sweep, .. } = transform_shape(&arc, &m) else {
            panic!()
        };
        // The quarter from +x to +y becomes the quarter from +y to −x.
        assert!((start - FRAC_PI_2).abs() < 1e-12);
        assert!((sweep - FRAC_PI_2).abs() < 1e-12);
        let pl = Shape::Polyline(Polyline {
            points: vec![p(0.0, 0.0), p(2.0, 0.0)],
            bulges: vec![1.0, 0.0],
            closed: false,
            width: 0.0,
        });
        let Shape::Polyline(q) = transform_shape(&pl, &m) else {
            panic!()
        };
        // The half circle bowed down (−y); mirrored in the y axis it still
        // bows down, now from (0,0) to (−2,0).
        let s = crate::seg::Seg::from_bulge(q.points[0], q.points[1], q.bulges[0]);
        assert!(s.midpoint().approx_eq(p(-1.0, -1.0)));
    }

    #[test]
    fn mirrored_text_reads_normally() {
        let t = Shape::Text(Text {
            at: p(1.0, 0.0),
            content: "A".into(),
            height: 2.5,
            angle: 0.0,
            align: Default::default(),
        });
        let m = Transform::mirror(p(0.0, 0.0), p(0.0, 1.0));
        let Shape::Text(x) = transform_shape(&t, &m) else {
            panic!()
        };
        assert!(x.at.approx_eq(p(-1.0, 0.0)));
        assert!(x.angle.abs() < 1e-12);
    }
}
