//! `archisim tramas`: the presentation textures of T1 (25/10/2026) side by
//! side, wood, metal, concrete and stone, each in a square with its name,
//! at 1:20 and at 1:50, as a project, SVG, PDF and DXF.

use std::path::Path;

use archisim_core::{
    DrawLayer, Hatch, HatchPattern, LineStyle, Model, NewDraft, Point2, Polyline, Shape, Text,
    TextAlign, ViewRef,
};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// The textures by family, in the order of the sheet.
pub fn families() -> Vec<(&'static str, Vec<HatchPattern>)> {
    let mut out: Vec<(&'static str, Vec<HatchPattern>)> = Vec::new();
    for p in HatchPattern::ALL {
        if let Some(f) = p.texture() {
            match out.iter_mut().find(|(n, _)| *n == f) {
                Some((_, list)) => list.push(p),
                None => out.push((f, vec![p])),
            }
        }
    }
    out
}

/// A project with every texture in a square `side` metres across, on the
/// ground floor's plan, in grey.
pub fn build(side: f64) -> Res<Model> {
    let mut m = archisim_library::new_project("Tramas de presentación");
    let level = m.levels()[0].id;
    let view = ViewRef::Level(level);
    let layer = m.add_draw_layer(DrawLayer::new(
        "Tramas",
        [96, 96, 96],
        0.09,
        LineStyle::Continuous,
    ))?;
    let edges = m.add_draw_layer(DrawLayer::new(
        "Contornos",
        [60, 60, 60],
        0.18,
        LineStyle::Continuous,
    ))?;
    let gap = side * 0.35;
    let mut y = 0.0;
    for (family, list) in families() {
        m.add_draft(NewDraft::new(
            view,
            edges,
            Shape::Text(Text {
                at: Point2::new(0.0, y + side + gap * 0.5),
                content: family.to_uppercase(),
                height: 3.5,
                angle: 0.0,
                align: TextAlign::BottomLeft,
            }),
        ))?;
        for (i, p) in list.iter().enumerate() {
            let x = i as f64 * (side + gap);
            let square = Polyline::straight(
                vec![
                    Point2::new(x, y),
                    Point2::new(x + side, y),
                    Point2::new(x + side, y + side),
                    Point2::new(x, y + side),
                ],
                true,
            );
            m.add_draft(NewDraft::new(
                view,
                layer,
                Shape::Hatch(Hatch {
                    loops: vec![square.clone()],
                    pattern: *p,
                    scale: 1.0,
                    angle: 0.0,
                    background: None,
                    opacity: 1.0,
                }),
            ))?;
            m.add_draft(NewDraft::new(view, edges, Shape::Polyline(square)))?;
            m.add_draft(NewDraft::new(
                view,
                edges,
                Shape::Text(Text {
                    at: Point2::new(x, y - gap * 0.4),
                    content: p.label().to_owned(),
                    height: 2.5,
                    angle: 0.0,
                    align: TextAlign::BottomLeft,
                }),
            ))?;
        }
        y -= side + gap * 1.8;
    }
    Ok(m)
}

pub fn write(dir: &Path) -> Res {
    std::fs::create_dir_all(dir)?;
    let mut pages = Vec::new();
    for (suffix, side, scale) in [("1-20", 1.2, 20.0), ("1-50", 3.0, 50.0)] {
        let m = build(side)?;
        if suffix == "1-50" {
            let project = dir.join(format!("tramas.{}", archisim_file::EXTENSION));
            archisim_file::save(&m, &project)?;
        }
        let options = archisim_plan::PlanOptions {
            scale,
            ..Default::default()
        };
        let svg = archisim_plan::render_plan(&m, m.levels()[0].id, &options);
        std::fs::write(dir.join(format!("tramas-{suffix}.svg")), &svg)?;
        pages.push(svg);
        if suffix == "1-50" {
            let dxf = archisim_export::dxf::plan_dxf(&m, m.levels()[0].id, &options)?;
            std::fs::write(dir.join("tramas.dxf"), dxf)?;
        }
    }
    let titles = ["Tramas a 1:20", "Tramas a 1:50"];
    let pages: Vec<archisim_export::pdf::Page> = pages
        .iter()
        .zip(titles)
        .map(|(svg, title)| archisim_export::pdf::Page { svg, title })
        .collect();
    std::fs::write(
        dir.join("tramas.pdf"),
        archisim_export::pdf::pdf(&pages, "Tramas de presentación")?,
    )?;
    let n: usize = families().iter().map(|(_, l)| l.len()).sum();
    println!("Tramas: {n} tramas de presentación (madera, metal, hormigón y piedra).");
    println!("  Proyecto: tramas.archisim (cuadros de 3 m)");
    println!("  Láminas: tramas-1-20.svg, tramas-1-50.svg, tramas.pdf; DXF: tramas.dxf");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every texture draws piece by piece in its square (none falls back to
    /// its plain lines), stays inside it and is not too heavy to draw.
    #[test]
    fn every_texture_draws_in_its_square() {
        let side = 3.0;
        let square = Polyline::straight(
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(side, 0.0),
                Point2::new(side, side),
                Point2::new(0.0, side),
            ],
            true,
        );
        let fams = families();
        assert_eq!(fams.len(), 4);
        assert_eq!(fams.iter().map(|(_, l)| l.len()).sum::<usize>(), 28);
        for (_, list) in fams {
            for p in list {
                let h = Hatch {
                    loops: vec![square.clone()],
                    pattern: p,
                    scale: 1.0,
                    angle: 0.3,
                    background: None,
                    opacity: 1.0,
                };
                let lines = archisim_draft::hatch::hatch_lines(&h, 0.05).unwrap();
                assert!(lines.len() > 40, "{p:?}: {}", lines.len());
                assert!(lines.len() < 40_000, "{p:?}: {}", lines.len());
                for (a, b) in &lines {
                    for q in [a, b] {
                        assert!(
                            q.x > -1e-6 && q.x < side + 1e-6 && q.y > -1e-6 && q.y < side + 1e-6,
                            "{p:?} {q:?}"
                        );
                    }
                }
                // The same hatch draws the same lines.
                assert_eq!(archisim_draft::hatch::hatch_lines(&h, 0.05).unwrap(), lines);
            }
        }
    }
}
