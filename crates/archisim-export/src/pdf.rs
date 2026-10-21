//! Vector PDF of drawings: each SVG a page of its paper size, lines and
//! fills as PDF paths (no fonts: ArchiSim letters with strokes), so the
//! PDF prints at true scale and true line weights.

use std::fmt::Write;

use crate::paths::{self, Item};

/// Points (1/72 inch) per CSS pixel (1/96 inch), usvg's unit.
const PT_PER_PX: f64 = 0.75;

/// One page: a drawing and its title in the PDF's outline.
pub struct Page<'a> {
    pub svg: &'a str,
    pub title: &'a str,
}

/// A PDF with one page per drawing.
pub fn pdf(pages: &[Page<'_>], title: &str) -> Result<Vec<u8>, String> {
    let mut w = Writer::default();
    w.out.extend_from_slice(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n");
    // 1: catalog, 2: pages; then per page: page, contents.
    let n = pages.len();
    let page_ids: Vec<usize> = (0..n).map(|i| 3 + 2 * i).collect();
    let info_id = 3 + 2 * n;
    w.object(1, "<< /Type /Catalog /Pages 2 0 R >>".into());
    let kids: Vec<String> = page_ids.iter().map(|i| format!("{i} 0 R")).collect();
    w.object(
        2,
        format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")),
    );
    for (i, page) in pages.iter().enumerate() {
        let tree = paths::parse(page.svg)?;
        let size = tree.size();
        let (wpt, hpt) = match paths::paper_mm(page.svg) {
            Some((w, h)) => (w * 72.0 / 25.4, h * 72.0 / 25.4),
            None => (
                size.width() as f64 * PT_PER_PX,
                size.height() as f64 * PT_PER_PX,
            ),
        };
        let px = wpt / size.width() as f64;
        let (content, alphas) = content(&tree, hpt, px);
        let mut gs = String::new();
        for (k, a) in alphas.iter().enumerate() {
            let _ = write!(gs, "/A{k} << /ca {a} /CA {a} >> ");
        }
        let resources = if alphas.is_empty() {
            "<< >>".to_string()
        } else {
            format!("<< /ExtGState << {gs}>> >>")
        };
        let pid = page_ids[i];
        w.object(
            pid,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources {resources} /Contents {} 0 R >>",
                num(wpt),
                num(hpt),
                pid + 1
            ),
        );
        let packed = miniz_oxide::deflate::compress_to_vec_zlib(content.as_bytes(), 6);
        w.stream(pid + 1, "/Filter /FlateDecode", &packed);
        let _ = page.title;
    }
    w.object(
        info_id,
        format!(
            "<< /Title {} /Producer (ArchiSim) /Creator (ArchiSim) >>",
            text_string(title)
        ),
    );
    Ok(w.finish(info_id))
}

#[derive(Default)]
struct Writer {
    out: Vec<u8>,
    offsets: Vec<(usize, usize)>,
}

impl Writer {
    fn object(&mut self, id: usize, body: String) {
        self.offsets.push((id, self.out.len()));
        self.out
            .extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    }

    fn stream(&mut self, id: usize, dict: &str, data: &[u8]) {
        self.offsets.push((id, self.out.len()));
        self.out.extend_from_slice(
            format!("{id} 0 obj\n<< {dict} /Length {} >>\nstream\n", data.len()).as_bytes(),
        );
        self.out.extend_from_slice(data);
        self.out.extend_from_slice(b"\nendstream\nendobj\n");
    }

    fn finish(mut self, info: usize) -> Vec<u8> {
        self.offsets.sort();
        let start = self.out.len();
        let size = self.offsets.len() + 1;
        let mut x = format!("xref\n0 {size}\n0000000000 65535 f \n");
        for (_, off) in &self.offsets {
            let _ = writeln!(x, "{off:010} 00000 n ");
        }
        let _ = write!(
            x,
            "trailer\n<< /Size {size} /Root 1 0 R /Info {info} 0 R >>\nstartxref\n{start}\n%%EOF\n"
        );
        self.out.extend_from_slice(x.as_bytes());
        self.out
    }
}

/// A PDF text string (UTF-16 with a byte order mark, as hex).
fn text_string(s: &str) -> String {
    let mut h = String::from("<FEFF");
    for u in s.encode_utf16() {
        let _ = write!(h, "{u:04X}");
    }
    h.push('>');
    h
}

fn num(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".into()
    } else {
        s.to_string()
    }
}

/// The page's drawing operators, and the opacities they use.
fn content(tree: &resvg::usvg::Tree, hpt: f64, pt_per_px: f64) -> (String, Vec<f64>) {
    let mut s = String::new();
    // PDF y goes up; the drawing is in CSS pixels.
    let _ = writeln!(
        s,
        "{} 0 0 {} 0 {} cm",
        num(pt_per_px),
        num(-pt_per_px),
        num(hpt)
    );
    // Opacities as graphics states: /A0 is solid, the others as the
    // drawing asks (rounded to a hundredth).
    let mut alphas: Vec<f64> = Vec::new();
    let items = paths::items(tree);
    let alpha = |it: &Item| {
        if it.fill.is_some() {
            it.fill_opacity
        } else {
            it.stroke_opacity
        }
    };
    let see_through = items.iter().any(|it| alpha(it) < 0.995);
    if see_through {
        alphas.push(1.0);
    }
    let mut current = 0;
    for it in items {
        if see_through {
            let a = (alpha(&it) * 100.0).round() / 100.0;
            let k = match alphas.iter().position(|x| (x - a).abs() < 1e-9) {
                Some(k) => k,
                None => {
                    alphas.push(a);
                    alphas.len() - 1
                }
            };
            if k != current {
                let _ = writeln!(s, "/A{k} gs");
                current = k;
            }
        }
        draw(&mut s, &it);
    }
    (s, alphas)
}

fn path_ops(s: &mut String, it: &Item) {
    for (pts, closed) in &it.subpaths {
        for (i, p) in pts.iter().enumerate() {
            let op = if i == 0 { "m" } else { "l" };
            let _ = write!(s, "{} {} {op} ", num(p.0), num(p.1));
        }
        if pts.len() == 1 {
            // A dot: a zero-length line drawn with round caps.
            let _ = write!(s, "{} {} l ", num(pts[0].0), num(pts[0].1));
        }
        if *closed {
            s.push_str("h ");
        }
    }
    s.push('\n');
}

fn rgb(c: [u8; 3]) -> String {
    format!(
        "{} {} {}",
        num(c[0] as f64 / 255.0),
        num(c[1] as f64 / 255.0),
        num(c[2] as f64 / 255.0)
    )
}

fn draw(s: &mut String, it: &Item) {
    if let Some(f) = it.fill {
        let _ = writeln!(s, "{} rg", rgb(f));
        path_ops(s, it);
        s.push_str(if it.even_odd { "f*\n" } else { "f\n" });
    }
    if let Some((c, w)) = it.stroke {
        let _ = writeln!(s, "{} RG {} w 1 J 1 j", rgb(c), num(w));
        if it.dashes.is_empty() {
            s.push_str("[] 0 d\n");
        } else {
            let d: Vec<String> = it.dashes.iter().map(|v| num(*v)).collect();
            let _ = writeln!(s, "[{}] 0 d", d.join(" "));
        }
        path_ops(s, it);
        s.push_str("S\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_of_the_paper_size() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="210mm" height="297mm" viewBox="0 0 0.21 0.297">
<path d="M0.01,0.01 L0.2,0.01 L0.2,0.287 Z" fill="#c0c0c0" stroke="#000000" stroke-width="0.0005"/>
</svg>"##;
        let bytes = pdf(&[Page { svg, title: "A-01" }], "Prueba").unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.starts_with("%PDF-1.4"));
        // A4 in points.
        assert!(text.contains("/MediaBox [0 0 595.2756 841.8898]"), "{text}");
        assert!(text.trim_end().ends_with("%%EOF"));
        // The cross-reference table points at each object.
        let xref = bytes.windows(4).rposition(|w| w == b"xref").unwrap();
        let tail = String::from_utf8_lossy(&bytes[xref..]).to_string();
        for (i, line) in tail.lines().skip(3).take(4).enumerate() {
            let off: usize = line[..10].parse().unwrap();
            let want = format!("{} 0 obj", i + 1);
            assert!(bytes[off..].starts_with(want.as_bytes()), "{line}");
        }
    }
}
