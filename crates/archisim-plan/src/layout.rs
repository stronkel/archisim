//! Presentation sheets (P4): a sheet laid out from a template as
//! competition and presentation boards are, with the project's title, the
//! plan of a level, the 3D view and a section at the scales that fit,
//! the rooms' areas and a diagram of the sun's path with the north; the
//! sheet and its views in a presentation style. Everything placed is an
//! ordinary view, table or drafting of the sheet: it can be moved,
//! resized or deleted afterwards.

use archisim_core::{
    Axo, DrawLayer, LevelId, LineStyle, Model, ModelError, NewDraft, NewSheet, NewViewport, Point2,
    PresentationStyleId, ScheduleKind, Shape, SheetId, Text, TextAlign, ViewRef,
};

/// Paper metres per millimetre.
const MM: f64 = 0.001;

/// The scales a view of a presentation sheet is tried at, closest first.
const SCALES: [f64; 12] = [
    50.0, 75.0, 100.0, 150.0, 200.0, 250.0, 300.0, 400.0, 500.0, 750.0, 1000.0, 2000.0,
];

/// What a presentation sheet is made of.
#[derive(Debug, Clone, PartialEq)]
pub struct PresentationSheet {
    pub number: String,
    pub name: String,
    /// Paper, millimetres (landscape).
    pub width: f64,
    pub height: f64,
    /// The level whose plan it shows.
    pub level: LevelId,
    /// The style of the sheet and its views; `None`: the plain look.
    pub style: Option<PresentationStyleId>,
}

/// A box on the paper, millimetres: lower-left and upper-right corners.
#[derive(Debug, Clone, Copy)]
struct Area {
    lo: (f64, f64),
    hi: (f64, f64),
}

impl Area {
    fn size(&self) -> (f64, f64) {
        (self.hi.0 - self.lo.0, self.hi.1 - self.lo.1)
    }

    fn centre(&self) -> Point2 {
        Point2::new(
            (self.lo.0 + self.hi.0) / 2.0 * MM,
            (self.lo.1 + self.hi.1) / 2.0 * MM,
        )
    }
}

/// The largest of [`SCALES`] (the most detailed) at which something of
/// `extent` metres (at 1:1, as `extent(scale)` gives it) fits `area`.
fn fitting_scale(area: Area, extent: impl Fn(f64) -> Option<(Point2, Point2)>) -> Option<f64> {
    let (w, h) = area.size();
    SCALES.iter().copied().find(|&s| {
        extent(s).is_some_and(|(lo, hi)| {
            (hi.x - lo.x) / s * 1000.0 <= w && (hi.y - lo.y) / s * 1000.0 <= h
        })
    })
}

/// Lays out a presentation sheet; returns it.
pub fn add_presentation_sheet(
    m: &mut Model,
    spec: &PresentationSheet,
) -> Result<SheetId, ModelError> {
    let (w, h) = (spec.width, spec.height);
    let sheet = m.add_sheet(NewSheet {
        number: spec.number.clone(),
        name: spec.name.clone(),
        width: w,
        height: h,
    })?;
    if let Some(mut s) = m.sheet(sheet).cloned() {
        s.presentation = spec.style;
        m.set_sheet(s)?;
    }
    let frame = crate::sheet::FRAME;
    let left = crate::sheet::FRAME_LEFT;
    let title_top = frame + crate::sheet::TITLE_H;
    let header = 34.0;
    let gap = 8.0;
    // The plan on the left, above the title block's band; the 3D view and
    // a section on the right; areas and the sun below the plan.
    let split = left + (w - left - frame) * 0.6;
    let plan = Area {
        lo: (left + gap, title_top + gap),
        hi: (split - gap / 2.0, h - frame - header),
    };
    let right_mid = title_top + (h - frame - header - title_top) * 0.45;
    let axo_area = Area {
        lo: (split + gap / 2.0, right_mid + gap / 2.0),
        hi: (w - frame - gap, h - frame - header),
    };
    let section_area = Area {
        lo: (split + gap / 2.0, title_top + gap),
        hi: (w - frame - gap, right_mid - gap / 2.0),
    };
    let level = spec.level;
    let style = spec.style;
    let mut views = Vec::new();
    if let Some(scale) = fitting_scale(plan, |s| crate::sheet::level_extent(m, level, s)) {
        views.push(m.add_viewport(NewViewport {
            sheet,
            level,
            section: None,
            axo: None,
            scale,
            cut_plane: 1.0,
            crop: None,
            centre: plan.centre(),
        })?);
    }
    let axo = Axo::default();
    // Each drawing made once, its extent at each scale tried from it.
    let picture = crate::axo::axo_drawing(m, &axo, Default::default());
    let axo_extent = |s: f64| {
        let k = 5.0 * s / 1000.0;
        (
            picture.lo - Point2::new(k, k),
            picture.hi + Point2::new(k, k),
        )
    };
    if let Some(scale) = fitting_scale(axo_area, |s| Some(axo_extent(s))) {
        let crop = axo_extent(scale);
        views.push(m.add_viewport(NewViewport {
            sheet,
            level,
            section: None,
            axo: Some(axo),
            scale,
            cut_plane: 1.0,
            crop: Some(crop),
            centre: axo_area.centre(),
        })?);
    }
    // The first cross-section (not an elevation), else the first elevation.
    let sections: Vec<_> = m.sections().map(|s| (s.id, s.title())).collect();
    let section = sections
        .iter()
        .find(|(_, t)| t.starts_with("Sección"))
        .or(sections.first())
        .map(|(id, _)| *id);
    if let Some(id) = section {
        let drawing = m
            .section(id)
            .map(|sec| crate::section::section_drawing(m, sec));
        let extent = |s: f64| drawing.as_ref().map(|d| crate::sheet::drawing_extent(d, s));
        if let Some(scale) = fitting_scale(section_area, extent) {
            let crop = extent(scale);
            views.push(m.add_viewport(NewViewport {
                sheet,
                level,
                section: Some(id),
                axo: None,
                scale,
                cut_plane: 1.0,
                crop,
                centre: section_area.centre(),
            })?);
        }
    }
    for v in views {
        if let Some(mut vp) = m.viewport(v).cloned() {
            vp.style.presentation = style;
            m.set_viewport(vp)?;
        }
    }
    // The title of the board, and a line under it.
    let layer = presentation_layer(m)?;
    let view = ViewRef::Sheet(sheet);
    let info = m.info().clone();
    let at = |x: f64, y: f64| Point2::new(x * MM, y * MM);
    m.add_draft(NewDraft::new(
        view,
        layer,
        Shape::Text(Text {
            at: at(left + gap, h - frame - 14.0),
            content: m.project_name.to_uppercase(),
            height: 9.0,
            angle: 0.0,
            align: TextAlign::BottomLeft,
        }),
    ))?;
    let subtitle: Vec<&str> = [info.site.as_str(), info.phase.as_str(), info.date.as_str()]
        .into_iter()
        .filter(|t| !t.trim().is_empty())
        .collect();
    if !subtitle.is_empty() {
        m.add_draft(NewDraft::new(
            view,
            layer,
            Shape::Text(Text {
                at: at(left + gap, h - frame - 25.0),
                content: subtitle.join(" · "),
                height: 3.5,
                angle: 0.0,
                align: TextAlign::BottomLeft,
            }),
        ))?;
    }
    m.add_draft(NewDraft::new(
        view,
        layer,
        Shape::Line {
            a: at(left + gap, h - frame - header + 1.0),
            b: at(w - frame - gap, h - frame - header + 1.0),
        },
    ))?;
    // Under the plan: the sun's path, and the rooms' areas next to it.
    let band = Area {
        lo: (left + gap, frame + 4.0),
        hi: (w - frame - crate::sheet::TITLE_W - gap, title_top),
    };
    let r = ((band.size().1 - 8.0) / 2.0).clamp(10.0, 24.0);
    let sun_c = Point2::new(
        (band.lo.0 + r + 4.0) * MM,
        (band.lo.1 + band.size().1 / 2.0) * MM,
    );
    for (shape, dashed) in sun_path(m, sun_c, r * MM) {
        let mut d = NewDraft::new(view, layer, shape);
        if dashed {
            d.line = Some(LineStyle::Dashed);
            d.weight = Some(0.13);
        }
        m.add_draft(d)?;
    }
    let table = crate::schedules::table(m, ScheduleKind::RoomAreas, Some(level));
    let (tw, th) = table.size();
    let tx = band.lo.0 + 2.0 * r + 12.0;
    if !table.rows.is_empty() && tx + tw <= band.hi.0 + gap && th <= band.size().1 + 4.0 {
        m.add_table(
            sheet,
            ScheduleKind::RoomAreas,
            Some(level),
            at(tx, band.hi.1),
        )?;
    }
    Ok(sheet)
}

/// The «Presentación» drawing layer of sheets (made if missing).
fn presentation_layer(m: &mut Model) -> Result<archisim_core::DrawLayerId, ModelError> {
    if let Some(l) = m.draw_layer_by_name("Presentación") {
        return Ok(l.id);
    }
    m.add_draw_layer(DrawLayer::new(
        "Presentación",
        [30, 30, 30],
        0.25,
        LineStyle::Continuous,
    ))
}

/// The diagram of the sun's path over the project, centred at `c` with
/// the horizon at radius `r` (paper metres), north as the plan's: the
/// horizon and the 30° and 60° heights (dashed: the second of each pair),
/// the paths of the solstices and the equinox, hours and where the sun of
/// the shadows is.
pub fn sun_path(m: &Model, c: Point2, r: f64) -> Vec<(Shape, bool)> {
    use archisim_core::{Hatch, HatchPattern, Polyline, Sun};
    let lat = m.latitude();
    let north = m.true_north();
    let place = |alt: f64, az: f64| {
        let rr = r * (1.0 - alt / std::f64::consts::FRAC_PI_2);
        let b = az + north;
        c + Point2::new(b.sin(), b.cos()) * rr
    };
    let mut out: Vec<(Shape, bool)> = vec![(
        Shape::Circle {
            centre: c,
            radius: r,
        },
        false,
    )];
    for deg in [30.0_f64, 60.0] {
        out.push((
            Shape::Circle {
                centre: c,
                radius: r * (1.0 - deg / 90.0),
            },
            true,
        ));
    }
    // North: a tick out of the horizon and an N.
    let n_dir = Point2::new(north.sin(), north.cos());
    out.push((
        Shape::Line {
            a: c + n_dir * r,
            b: c + n_dir * (r * 1.12),
        },
        false,
    ));
    out.push((
        Shape::Text(Text {
            at: c + n_dir * (r * 1.22),
            content: "N".into(),
            height: 3.0,
            angle: 0.0,
            align: TextAlign::Middle,
        }),
        false,
    ));
    let height_mm = 1.8;
    for (day, label) in [(172u16, "21 jun"), (80, "21 mar · 21 sep"), (355, "21 dic")] {
        let pts: Vec<Point2> = (0..=96)
            .map(|i| Sun {
                day,
                hour: i as f64 / 4.0,
            })
            .map(|s| s.position(lat))
            .filter(|(alt, _)| *alt > 0.0)
            .map(|(alt, az)| place(alt, az))
            .collect();
        if pts.len() >= 2 {
            out.push((Shape::Polyline(Polyline::straight(pts, false)), false));
        }
        let (alt, az) = Sun { day, hour: 12.0 }.position(lat);
        if alt > 0.0 {
            out.push((
                Shape::Text(Text {
                    at: place(alt, az) + Point2::new(0.0, -0.0025),
                    content: label.into(),
                    height: height_mm,
                    angle: 0.0,
                    align: TextAlign::TopCentre,
                }),
                false,
            ));
        }
        // Hours, on the summer path: 8 to 18.
        if day == 172 {
            for hour in [8.0, 10.0, 12.0, 14.0, 16.0, 18.0] {
                let (alt, az) = Sun { day, hour }.position(lat);
                if alt > 0.0 {
                    out.push((
                        Shape::Text(Text {
                            at: place(alt, az) + Point2::new(0.0, 0.0015),
                            content: format!("{hour:.0}h"),
                            height: 1.5,
                            angle: 0.0,
                            align: TextAlign::BottomCentre,
                        }),
                        false,
                    ));
                }
            }
        }
    }
    // The sun of the shadows.
    let (alt, az) = m.sun().position(lat);
    if alt > 0.0 {
        let p = place(alt, az);
        let dot = (0..12)
            .map(|i| {
                let a = i as f64 / 12.0 * std::f64::consts::TAU;
                p + Point2::new(a.cos(), a.sin()) * (r * 0.06)
            })
            .collect();
        out.push((
            Shape::Hatch(Hatch {
                loops: vec![Polyline::straight(dot, true)],
                pattern: HatchPattern::Solid,
                scale: 1.0,
                angle: 0.0,
                background: None,
                opacity: 1.0,
            }),
            false,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sun_path_goes_round_the_south() {
        let m = Model::new("Sol");
        let c = Point2::new(0.1, 0.1);
        let shapes = sun_path(&m, c, 0.02);
        // The summer path: from the north-east to the north-west by the
        // south, all inside the horizon.
        let paths: Vec<&archisim_core::Polyline> = shapes
            .iter()
            .filter_map(|(s, _)| match s {
                Shape::Polyline(p) => Some(p),
                _ => None,
            })
            .collect();
        assert_eq!(paths.len(), 3);
        for p in &paths {
            for q in &p.points {
                assert!(q.distance(c) <= 0.02 + 1e-9);
            }
        }
        let summer = paths[0];
        let lowest = summer
            .points
            .iter()
            .map(|q| q.y)
            .fold(f64::INFINITY, f64::min);
        assert!(lowest < c.y, "noon is to the south");
        assert!(summer.points[0].y > c.y, "sunrise to the north-east");
        assert!(shapes.iter().any(|(s, _)| matches!(s, Shape::Hatch(_))));
    }
}
