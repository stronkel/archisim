//! The door and window plan symbols as ArchiSim drew them before
//! families, kept to check that the system families draw the same.

use std::fmt::Write as _;

use archisim_core::{DoorOperation, OpeningKind, Point2, WindowOperation, EPSILON};
use archisim_geom::OpeningGeometry;

use super::{num, path, pt, FRAME_LINE, INK, LAYER_LINE};

const LEAF_THICKNESS: f64 = 0.04;
const SILL_PROJECTION: f64 = 0.03;

/// A closed rectangle from four corners, filled white.
fn quad(s: &mut String, pts: [Point2; 4], weight: f64, k: f64) {
    let _ = writeln!(
        s,
        r#"<path d="{}" fill="white" stroke="{INK}" stroke-width="{}"/>"#,
        path(&pts),
        num(weight * k)
    );
}

fn line(s: &mut String, p: Point2, q: Point2, weight: f64, k: f64) {
    let _ = writeln!(
        s,
        r#"<path d="M{} L{}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
        pt(p),
        pt(q),
        num(weight * k)
    );
}

/// A rectangle in the opening's own coordinates: `t` along the wall and
/// offsets across it.
fn rect(o: &OpeningGeometry, t0: f64, t1: f64, y0: f64, y1: f64) -> [Point2; 4] {
    [
        o.point(t0, y0),
        o.point(t1, y0),
        o.point(t1, y1),
        o.point(t0, y1),
    ]
}

/// A door or window crossed by the cut plane.
pub(super) fn draw_opening(s: &mut String, o: &OpeningGeometry, k: f64) {
    let (fe, fi, fw) = (o.frame_exterior, o.frame_interior, o.frame_width);
    match o.kind {
        OpeningKind::Door(op) => {
            // Jambs of the frame ("cerco").
            quad(s, rect(o, o.t0, o.t0 + fw, fi, fe), FRAME_LINE, k);
            quad(s, rect(o, o.t1 - fw, o.t1, fi, fe), FRAME_LINE, k);
            let (c0, c1) = (o.t0 + fw, o.t1 - fw);
            match op {
                DoorOperation::SingleSwing => {
                    let (hinge, dir) = if o.hinge_at_start() {
                        (c0, 1.0)
                    } else {
                        (c1, -1.0)
                    };
                    draw_leaf(s, o, hinge, dir, c1 - c0, Leaf::Door, k);
                }
                DoorOperation::DoubleSwing => {
                    let w = (c1 - c0) / 2.0;
                    draw_leaf(s, o, c0, 1.0, w, Leaf::Door, k);
                    draw_leaf(s, o, c1, -1.0, w, Leaf::Door, k);
                }
                DoorOperation::Sliding => draw_sliding_leaf(s, o, c0, c1, k),
            }
        }
        OpeningKind::Window(op) => {
            let mid = (fe + fi) / 2.0;
            // Sill wall seen below the cut: sill board outside, edge inside.
            // The board runs from the frame to past the façade, and only
            // beyond the façade does it reach past the jambs (behind them
            // it is hidden by the cut wall).
            if o.sill > EPSILON {
                let (ext, out, lap) =
                    (o.exterior_offset, o.exterior_offset + SILL_PROJECTION, 0.02);
                let board = [
                    o.point(o.t0, fe),
                    o.point(o.t1, fe),
                    o.point(o.t1, ext),
                    o.point(o.t1 + lap, ext),
                    o.point(o.t1 + lap, out),
                    o.point(o.t0 - lap, out),
                    o.point(o.t0 - lap, ext),
                    o.point(o.t0, ext),
                ];
                let _ = writeln!(
                    s,
                    r#"<path d="{}" fill="white" stroke="{INK}" stroke-width="{}"/>"#,
                    path(&board),
                    num(LAYER_LINE * k)
                );
                line(
                    s,
                    o.point(o.t0, o.interior_offset),
                    o.point(o.t1, o.interior_offset),
                    LAYER_LINE,
                    k,
                );
            }
            quad(s, rect(o, o.t0, o.t1, fi, fe), FRAME_LINE, k);
            let (c0, c1) = (o.t0 + fw, o.t1 - fw);
            match op {
                WindowOperation::Fixed => {
                    line(s, o.point(c0, mid), o.point(c1, mid), LAYER_LINE, k);
                }
                WindowOperation::SingleCasement => {
                    line(s, o.point(c0, mid), o.point(c1, mid), LAYER_LINE, k);
                    let (hinge, dir) = if o.hinge_at_start() {
                        (c0, 1.0)
                    } else {
                        (c1, -1.0)
                    };
                    draw_leaf(s, o, hinge, dir, c1 - c0, Leaf::Window, k);
                }
                WindowOperation::DoubleCasement => {
                    let tm = (o.t0 + o.t1) / 2.0;
                    line(s, o.point(c0, mid), o.point(c1, mid), LAYER_LINE, k);
                    line(s, o.point(tm, fi), o.point(tm, fe), FRAME_LINE, k);
                    // Each leaf reaches the centre of the mullion.
                    draw_leaf(s, o, c0, 1.0, tm - c0, Leaf::Window, k);
                    draw_leaf(s, o, c1, -1.0, c1 - tm, Leaf::Window, k);
                }
                WindowOperation::Sliding => {
                    // Two leaves on separate tracks, overlapping at the centre.
                    let tm = (o.t0 + o.t1) / 2.0;
                    let q = (fe - fi) / 4.0;
                    let lap = fw / 2.0;
                    quad(s, rect(o, c0, tm + lap, mid, mid + q), LAYER_LINE, k);
                    quad(s, rect(o, tm - lap, c1, mid - q, mid), LAYER_LINE, k);
                }
            }
        }
    }
}

/// How an open leaf is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Leaf {
    /// A door leaf: a rectangle as thick as the leaf, medium line.
    Door,
    /// A window leaf: a single thin line.
    Window,
}

/// A hinged leaf drawn open at 90°, with its swing. `dir` is +1 when the
/// leaf closes towards growing `t` from its hinge, −1 otherwise.
fn draw_leaf(
    s: &mut String,
    o: &OpeningGeometry,
    hinge_t: f64,
    dir: f64,
    width: f64,
    leaf: Leaf,
    k: f64,
) {
    let h = o.point(hinge_t, o.swing_frame_face());
    let out = o.swing_dir();
    let along = o.u * dir;
    let tip_open = h + out * width;
    let tip_closed = h + along * width;
    match leaf {
        Leaf::Door => {
            let lt = LEAF_THICKNESS.min(width / 4.0);
            quad(
                s,
                [h, tip_open, tip_open + along * lt, h + along * lt],
                FRAME_LINE,
                k,
            );
        }
        Leaf::Window => line(s, h, tip_open, LAYER_LINE, k),
    }
    // Model counter-clockwise is screen counter-clockwise (sweep 0).
    let sweep = if out.cross(along) > 0.0 { 0 } else { 1 };
    let _ = writeln!(
        s,
        r#"<path d="M{} A{r},{r} 0 0,{sweep} {}" fill="none" stroke="{INK}" stroke-width="{}"/>"#,
        pt(tip_open),
        pt(tip_closed),
        num(LAYER_LINE * k),
        r = num(width)
    );
}

/// A sliding leaf hung on the face of the wall, drawn half open towards the
/// side it parks on, with an arrow.
fn draw_sliding_leaf(s: &mut String, o: &OpeningGeometry, c0: f64, c1: f64, k: f64) {
    let clear = c1 - c0;
    let lap = 0.05;
    let shift = if o.hinge_at_start() {
        -clear / 2.0
    } else {
        clear / 2.0
    };
    let face = o.swing_face();
    let sign = if o.swing_dir().dot(o.n) > 0.0 {
        1.0
    } else {
        -1.0
    };
    let (y0, y1) = (face + sign * 0.01, face + sign * (0.01 + LEAF_THICKNESS));
    quad(
        s,
        rect(o, c0 - lap + shift, c1 + lap + shift, y0, y1),
        FRAME_LINE,
        k,
    );
    // Arrow pointing where the leaf slides to open.
    let ya = face + sign * (0.01 + LEAF_THICKNESS + 0.08);
    let tm = (c0 + c1) / 2.0;
    let tip = tm + shift.signum() * clear * 0.3;
    let head = 0.06 * shift.signum();
    line(s, o.point(tm, ya), o.point(tip, ya), LAYER_LINE, k);
    line(
        s,
        o.point(tip, ya),
        o.point(tip - head, ya + 0.03),
        LAYER_LINE,
        k,
    );
    line(
        s,
        o.point(tip, ya),
        o.point(tip - head, ya - 0.03),
        LAYER_LINE,
        k,
    );
}
