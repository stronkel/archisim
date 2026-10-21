//! Presentation styles on the drawing (see
//! [`archisim_core::PresentationStyle`]): the walls and columns cut solid
//! in a colour while the view is drawn, and then, on the finished SVG, the
//! paper, the palette and the line weights. Done on the SVG, the styled
//! view prints, goes to PDF and DXF and shows on screen the same.

use archisim_core::{Model, PresentationStyle, ViewStyle};

use crate::num;

/// The presentation style a view uses, if it still exists.
pub fn style_of<'a>(model: &'a Model, style: &ViewStyle) -> Option<&'a PresentationStyle> {
    style
        .presentation
        .and_then(|id| model.presentation_style(id))
}

/// Sets the solid fill of cut walls and columns for the drawing that
/// follows; the guard clears it.
pub(crate) fn begin(p: Option<&PresentationStyle>) -> Guard {
    crate::POCHE.with(|c| c.set(p.and_then(|p| p.poche)));
    crate::PRESENTING.with(|c| c.set(p.is_some()));
    Guard
}

pub(crate) struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        crate::POCHE.with(|c| c.set(None));
        crate::PRESENTING.with(|c| c.set(false));
    }
}

/// How dark the shadows are: their opacity.
pub const SHADOW_OPACITY: f64 = 0.28;

/// The colour of shadows in a style: its darkest tone, or a soft black.
pub fn shadow_colour(p: &PresentationStyle) -> archisim_core::Rgb {
    let lum = |c: &archisim_core::Rgb| 0.3 * c[0] as f64 + 0.59 * c[1] as f64 + 0.11 * c[2] as f64;
    p.palette
        .iter()
        .min_by(|a, b| lum(a).total_cmp(&lum(b)))
        .copied()
        .unwrap_or([40, 40, 46])
}

/// The shadow areas, see-through in the style's shadow colour.
pub(crate) fn draw_shadows(
    s: &mut String,
    shapes: &[archisim_geom::junctions::Shape],
    p: &PresentationStyle,
) {
    use std::fmt::Write;
    let mut d = String::new();
    for (outer, holes) in shapes {
        for ring in std::iter::once(outer).chain(holes) {
            for (i, q) in ring.iter().enumerate() {
                d.push(if i == 0 { 'M' } else { 'L' });
                d.push_str(&crate::pt(*q));
                d.push(' ');
            }
            d.push_str("Z ");
        }
    }
    let [r, g, b] = shadow_colour(p);
    let _ = writeln!(
        s,
        r#"<g id="shadows"><path d="{}" fill="rgb({r},{g},{b})" fill-opacity="{}" fill-rule="evenodd" stroke="none"/></g>"#,
        d.trim_end(),
        num(SHADOW_OPACITY)
    );
}

/// The SVG of a view in a presentation style: lines `line_scale` times
/// as heavy, every colour turned to the palette (the paper white to the
/// style's paper, the solid fill kept), and the paper's colour behind
/// the whole view.
pub fn apply(svg: &str, p: &PresentationStyle) -> String {
    let scaled = if (p.line_scale - 1.0).abs() > 1e-9 {
        scale_strokes(svg, p.line_scale)
    } else {
        svg.to_string()
    };
    let mut out = crate::dim::map_colours(&scaled, |c| {
        if c == [255, 255, 255] {
            p.background
        } else if Some(c) == p.poche {
            c
        } else {
            p.tone(c)
        }
    });
    if p.background != [255, 255, 255] {
        paper(&mut out, p.background);
    }
    out
}

/// A rectangle of the paper's colour over the whole view box, behind
/// everything (after the opening `<svg …>`).
fn paper(svg: &mut String, c: archisim_core::Rgb) {
    let Some(start) = svg.find("<svg") else {
        return;
    };
    let Some(end) = svg[start..].find('>').map(|e| start + e + 1) else {
        return;
    };
    let head = &svg[start..end];
    let Some(vb) = head
        .find("viewBox=\"")
        .map(|i| &head[i + 9..])
        .and_then(|r| r.split('"').next())
    else {
        return;
    };
    let v: Vec<f64> = vb
        .split_whitespace()
        .filter_map(|x| x.parse().ok())
        .collect();
    if v.len() != 4 {
        return;
    }
    let rect = format!(
        "\n<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#{:02x}{:02x}{:02x}\"/>",
        num(v[0]),
        num(v[1]),
        num(v[2]),
        num(v[3]),
        c[0],
        c[1],
        c[2]
    );
    svg.insert_str(end, &rect);
}

/// Every `stroke-width` of an SVG multiplied by `f`.
fn scale_strokes(svg: &str, f: f64) -> String {
    let key = "stroke-width=\"";
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(i) = rest.find(key) {
        out.push_str(&rest[..i + key.len()]);
        rest = &rest[i + key.len()..];
        let Some(j) = rest.find('"') else { break };
        match rest[..j].parse::<f64>() {
            Ok(w) => out.push_str(&num(w * f)),
            Err(_) => out.push_str(&rest[..j]),
        }
        rest = &rest[j..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_turns_colours_lines_and_paper() {
        let mut p = PresentationStyle::new("Bosque");
        p.background = [245, 240, 230];
        p.palette = vec![[30, 50, 40], [245, 240, 230]];
        p.poche = Some([47, 79, 62]);
        p.line_scale = 0.5;
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 -10 20 10">
<rect x="0" y="-10" width="20" height="10" fill="white"/>
<path d="M0 0 L1 1" stroke="#1a1a1a" stroke-width="0.4"/>
<path d="M0 0 L1 1 L1 0Z" fill="rgb(47,79,62)"/>
</svg>"##;
        let out = apply(svg, &p);
        assert!(out.contains("stroke-width=\"0.2\""), "{out}");
        assert!(!out.contains("#1a1a1a"));
        assert!(out.contains("fill=\"#2f4f3e\""), "the poche stays");
        assert!(out.contains("fill=\"#f5f0e6\""), "the paper");
        // The paper goes first, over the whole view box.
        let rect = out.find("<rect x=\"0\" y=\"-10\"").unwrap();
        assert!(rect < out.find("<path").unwrap());
    }

    #[test]
    fn walls_are_cut_solid_and_floors_filled() {
        use archisim_core::{
            CutPattern, FloorFill, HatchPattern, Layer, LayerFunction, NewRoom, NewWall, Point2,
            ReferenceLine, WallFunction, WallTop,
        };
        let mut m = Model::new("Presentación");
        let l = m.add_level("PB", 0.0);
        let brick = m.add_material("Ladrillo", "brick", [230, 120, 90], CutPattern::Diagonal);
        let t = m
            .add_wall_type(
                "Muro",
                WallFunction::Exterior,
                vec![Layer {
                    material: brick,
                    thickness: 0.25,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        m.add_wall(NewWall {
            wall_type: t,
            level: l,
            start: Point2::new(0.0, 0.0),
            end: Point2::new(6.0, 0.0),
            reference: ReferenceLine::Center,
            height: 3.0,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })
        .unwrap();
        let room = m
            .add_room(NewRoom {
                level: l,
                number: "1".into(),
                name: "Terraza".into(),
                outline: vec![
                    Point2::new(0.0, 1.0),
                    Point2::new(6.0, 1.0),
                    Point2::new(6.0, 4.0),
                    Point2::new(0.0, 4.0),
                ],
                holes: Vec::new(),
                label: Point2::new(3.0, 2.5),
                bounded: false,
            })
            .unwrap();
        let mut r = m.room(room).unwrap().clone();
        r.floor_fill = Some(FloorFill {
            pattern: HatchPattern::Decking,
            color: [120, 80, 40],
            background: Some([230, 210, 180]),
            scale: 1.0,
            angle: 0.0,
        });
        m.set_room(r).unwrap();
        let plain = crate::render_plan(&m, l, &crate::PlanOptions::default());
        assert!(plain.contains("rgb(230,120,90)"), "the brick's own colour");
        assert!(plain.contains("#e6d2b4"), "the decking's background");
        let mut look = PresentationStyle::new("Bosque");
        look.poche = Some([47, 79, 62]);
        let id = m.add_presentation_style(look).unwrap();
        let mut o = crate::PlanOptions::default();
        o.style.presentation = Some(id);
        let styled = crate::render_plan(&m, l, &o);
        assert!(!styled.contains("rgb(230,120,90)"), "no layers");
        assert!(styled.contains("#2f4f3e"), "solid green walls");
        assert!(styled.contains("#e6d2b4"), "floors still filled");
        assert!(!styled.contains("id=\"shadows\""));
        // With shadows: the wall's, on the ground.
        let mut look = m.presentation_style(id).unwrap().clone();
        look.shadows = true;
        m.set_presentation_style(look).unwrap();
        let shaded = crate::render_plan(&m, l, &o);
        assert!(shaded.contains("id=\"shadows\""));
        assert!(shaded.contains("fill-opacity=\"0.28\""));
        // And in an axonometric of a sheet.
        let axo = archisim_core::Axo::default();
        let d = crate::axo::axo_drawing(&m, &axo, archisim_core::DisciplineFocus::None)
            .with_shadows(&m, &axo, [40, 40, 46]);
        let svg = crate::axo::render_axo(&d, (d.lo, d.hi), 100.0);
        assert!(svg.contains("fill-opacity"));
    }
}
