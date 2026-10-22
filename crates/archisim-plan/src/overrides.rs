//! How each category is drawn in a view (Revit's visibility and graphic
//! overrides, [`archisim_core::GraphicOverride`]): every category is
//! drawn inside its own `<g data-cat="…">`, and on the finished SVG the
//! group of each category with an override gets its line colour, its cut
//! and surface fills, halftone, line weight, transparency and, without
//! patterns, no hatches (paths marked `class="pat"`). The lines between
//! the layers of walls are left out while drawing (see [`layer_lines`]).
//! Done on the SVG, a view looks the same on screen, in PDF and in DXF.

use std::cell::Cell;
use std::fmt::Write as _;

use archisim_core::{Category, GraphicOverride, LayerLines, ViewStyle};

thread_local! {
    static WALL_LINES: Cell<LayerLines> = const { Cell::new(LayerLines::All) };
}

/// Which lines between the layers of cut walls the view draws.
pub(crate) fn layer_lines() -> LayerLines {
    WALL_LINES.with(|c| c.get())
}

/// Sets what is decided while drawing (the lines between wall layers);
/// the guard puts it back.
pub(crate) fn begin(style: &ViewStyle) -> Guard {
    let previous = WALL_LINES.with(|c| c.replace(style.graphics(Category::Walls).layer_lines));
    Guard(previous)
}

pub(crate) struct Guard(LayerLines);

impl Drop for Guard {
    fn drop(&mut self) {
        WALL_LINES.with(|c| c.set(self.0));
    }
}

/// Opens the group of a category.
pub(crate) fn open(s: &mut String, c: Category) {
    let _ = writeln!(s, r#"<g data-cat="{}">"#, c.code());
}

pub(crate) fn close(s: &mut String) {
    s.push_str("</g>\n");
}

/// Draws a category inside its group.
pub(crate) fn group(s: &mut String, c: Category, draw: impl FnOnce(&mut String)) {
    open(s, c);
    draw(s);
    close(s);
}

/// The class of hatch paths, taken away when a category has no patterns.
pub(crate) const PATTERN: &str = r#"class="pat" "#;

/// Categories whose fills in plan are what is cut.
fn fills_are_cut(c: Category) -> bool {
    matches!(
        c,
        Category::Walls | Category::Structure | Category::CurtainWalls | Category::Foundations
    )
}

/// The SVG with each category's overrides applied.
pub fn apply(svg: &str, style: &ViewStyle) -> String {
    if style.overrides.is_empty() {
        return svg.to_string();
    }
    let key = "<g data-cat=\"";
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(i) = rest.find(key) {
        out.push_str(&rest[..i]);
        let after = &rest[i + key.len()..];
        let Some(q) = after.find('"') else {
            out.push_str(&rest[i..]);
            return out;
        };
        let code = &after[..q];
        let tag_end = after.find('>').map_or(after.len(), |e| e + 1);
        let open_tag = &rest[i..i + key.len() + tag_end];
        let body_start = i + key.len() + tag_end;
        let body_len = group_len(&rest[body_start..]);
        let body = &rest[body_start..body_start + body_len];
        let close_len = "</g>".len().min(rest.len() - body_start - body_len);
        let g = Category::from_code(code).map(|c| (c, style.graphics(c)));
        match g.filter(|(_, g)| !g.is_default()) {
            Some((c, g)) => {
                if g.transparency > 0 {
                    let opacity = (100 - g.transparency.min(100)) as f64 / 100.0;
                    out.push_str(&open_tag[..open_tag.len() - 1]);
                    let _ = write!(out, r#" opacity="{opacity}">"#);
                } else {
                    out.push_str(open_tag);
                }
                out.push_str(&restyle(body, c, &g));
            }
            None => {
                out.push_str(open_tag);
                out.push_str(body);
            }
        }
        out.push_str(&rest[body_start + body_len..body_start + body_len + close_len]);
        rest = &rest[body_start + body_len + close_len..];
    }
    out.push_str(rest);
    out
}

/// Length of a group's content, up to its closing `</g>` (nested groups
/// counted).
fn group_len(body: &str) -> usize {
    let mut depth = 0usize;
    let mut i = 0;
    let b = body.as_bytes();
    while i < b.len() {
        if body[i..].starts_with("</g>") {
            if depth == 0 {
                return i;
            }
            depth -= 1;
            i += 4;
        } else if body[i..].starts_with("<g") && matches!(b.get(i + 2), Some(b' ' | b'>')) {
            // A self-closed group opens nothing.
            let end = body[i..].find('>').map_or(b.len(), |e| i + e);
            if !body[..end].ends_with('/') {
                depth += 1;
            }
            i = end + 1;
        } else {
            i += 1;
        }
    }
    b.len()
}

/// A category's drawing restyled by its override.
fn restyle(body: &str, c: Category, g: &GraphicOverride) -> String {
    let mut body = if g.patterns {
        body.to_string()
    } else {
        without_patterns(body)
    };
    if g.weight != 100 {
        body = crate::presentation::scale_strokes(&body, g.weight as f64 / 100.0);
    }
    let fill_over = if fills_are_cut(c) {
        g.cut_fill
    } else {
        g.surface_fill
    };
    map_paint(&body, |stroke, colour| {
        let lum = 0.3 * colour[0] as f64 + 0.59 * colour[1] as f64 + 0.11 * colour[2] as f64;
        let ink = stroke || lum < 64.0;
        let to = if ink {
            g.line.unwrap_or(colour)
        } else {
            fill_over.unwrap_or(colour)
        };
        g.tint(to)
    })
}

/// Takes away the hatch paths.
fn without_patterns(body: &str) -> String {
    let mark = format!("<path {PATTERN}");
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(i) = rest.find(&mark) {
        out.push_str(&rest[..i]);
        let after = &rest[i..];
        let end = after.find("/>").map_or(after.len(), |e| e + 2);
        rest = &after[end..];
        if rest.starts_with('\n') {
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    out
}

/// Every `stroke="…"` and `fill="…"` colour turned by `f(is_stroke,
/// colour)`; `none` and unknown values stay. A stroke of the same colour
/// as its element's fill (the hairline that hides seams between fills)
/// goes with the fill.
fn map_paint(svg: &str, f: impl Fn(bool, [u8; 3]) -> [u8; 3]) -> String {
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(i) = rest.find('<') {
        out.push_str(&rest[..i]);
        let end = rest[i..].find('>').map_or(rest.len(), |e| i + e + 1);
        out.push_str(&map_tag(&rest[i..end], &f));
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<(usize, &'a str)> {
    let key = format!(" {name}=\"");
    let i = tag.find(&key)? + key.len();
    let j = tag[i..].find('"')?;
    Some((i, &tag[i..i + j]))
}

fn map_tag(tag: &str, f: &impl Fn(bool, [u8; 3]) -> [u8; 3]) -> String {
    let fill = attr(tag, "fill");
    let stroke = attr(tag, "stroke");
    let seam = matches!((fill, stroke), (Some((_, a)), Some((_, b))) if a == b);
    // Replacements from the end so earlier positions stay right.
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    let mut put = |at: Option<(usize, &str)>, is_stroke: bool| {
        if let Some((i, v)) = at {
            if let Some(c) = parse_colour(v) {
                let [r, g, b] = f(is_stroke, c);
                edits.push((i, i + v.len(), format!("#{r:02x}{g:02x}{b:02x}")));
            }
        }
    };
    put(fill, false);
    put(stroke, !seam);
    edits.sort_by_key(|e| std::cmp::Reverse(e.0));
    let mut out = tag.to_string();
    for (a, b, v) in edits {
        out.replace_range(a..b, &v);
    }
    out
}

fn parse_colour(v: &str) -> Option<[u8; 3]> {
    let v = v.trim();
    match v {
        "white" => return Some([255, 255, 255]),
        "black" => return Some([0, 0, 0]),
        _ => {}
    }
    if let Some(h) = v.strip_prefix('#') {
        let d = |s: &str| u8::from_str_radix(s, 16).ok();
        return match h.len() {
            6 => Some([d(&h[0..2])?, d(&h[2..4])?, d(&h[4..6])?]),
            3 => Some([d(&h[0..1])? * 17, d(&h[1..2])? * 17, d(&h[2..3])? * 17]),
            _ => None,
        };
    }
    let inside = v.strip_prefix("rgb(")?.strip_suffix(')')?;
    let p: Vec<u8> = inside
        .split(',')
        .filter_map(|x| x.trim().parse::<f64>().ok())
        .map(|x| x.clamp(0.0, 255.0) as u8)
        .collect();
    (p.len() == 3).then(|| [p[0], p[1], p[2]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overrides_restyle_only_their_category() {
        let svg = concat!(
            r##"<svg><g data-cat="walls"><path class="pat" d="M0 0" fill="none" stroke="#1a1a1a"/>"##,
            "\n",
            r##"<path id="cut-1" d="M0 0" fill="rgb(230,120,90)" stroke="#1a1a1a" stroke-width="0.5"/><g id="w"><path d="M1 1" fill="#1a1a1a"/></g></g>"##,
            r##"<g data-cat="slabs"><path d="M2 2" fill="#f1f1ee" stroke="#1a1a1a"/></g></svg>"##
        );
        let mut style = ViewStyle::default();
        style.set_graphics(
            Category::Walls,
            GraphicOverride {
                cut_fill: Some([255, 255, 255]),
                line: Some([0, 0, 255]),
                patterns: false,
                weight: 200,
                transparency: 50,
                ..Default::default()
            },
        );
        let out = apply(svg, &style);
        assert!(!out.contains("class=\"pat\""), "{out}");
        assert!(
            out.contains(
                r##"id="cut-1" d="M0 0" fill="#ffffff" stroke="#0000ff" stroke-width="1""##
            ),
            "{out}"
        );
        // Dark fills are ink: they take the line colour.
        assert!(
            out.contains(r##"<path d="M1 1" fill="#0000ff"/>"##),
            "{out}"
        );
        assert!(
            out.contains(r#"<g data-cat="walls" opacity="0.5">"#),
            "{out}"
        );
        // The slabs are untouched.
        assert!(out.contains(r##"<path d="M2 2" fill="#f1f1ee" stroke="#1a1a1a"/>"##));
        // Halftone: halfway to white.
        let mut half = ViewStyle::default();
        half.set_graphics(
            Category::Slabs,
            GraphicOverride {
                halftone: true,
                ..Default::default()
            },
        );
        assert!(apply(svg, &half).contains(r##"fill="#f8f8f6" stroke="#8c8c8c""##));
    }
}
