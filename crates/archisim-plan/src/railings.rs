//! Loose railings in plan: the handrail seen from above (two fine lines as
//! wide as it) and the posts as small squares, filled where the cut plane
//! goes through them.

use std::fmt::Write as _;

use archisim_core::{Point2, Railing, RailingPieceKind, HANDRAIL_SIZE};

use crate::{num, path, pens, INK};

/// Every point of the railing the plan must show.
pub fn railing_points(r: &Railing) -> Vec<Point2> {
    r.path.clone()
}

pub fn draw_railing(s: &mut String, r: &Railing, cut_plane: f64, k: f64) {
    let _ = writeln!(s, r#"<g class="railing">"#);
    let half = HANDRAIL_SIZE / 2.0;
    for w in r.path.windows(2) {
        let Some(u) = (w[1] - w[0]).normalized() else {
            continue;
        };
        let n = u.perp_left() * half;
        let quad = [w[0] - n, w[1] - n, w[1] + n, w[0] + n];
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="white" stroke="{INK}" stroke-width="{}"/>"#,
            path(&quad),
            num(pens().view * k)
        );
    }
    let cut = cut_plane > r.base_offset && cut_plane < r.base_offset + r.height;
    for p in r
        .pieces()
        .iter()
        .filter(|p| p.kind == RailingPieceKind::Post)
    {
        let c = p.a.0;
        let h = p.size / 2.0;
        let sq = [
            Point2::new(c.x - h, c.y - h),
            Point2::new(c.x + h, c.y - h),
            Point2::new(c.x + h, c.y + h),
            Point2::new(c.x - h, c.y + h),
        ];
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="{}" stroke="{INK}" stroke-width="{}"/>"#,
            path(&sq),
            if cut { INK } else { "white" },
            num(pens().view * k)
        );
    }
    s.push_str("</g>\n");
}
