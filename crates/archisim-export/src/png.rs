//! Drawings as PNG images (P4): a sheet or a plan rasterised from its SVG
//! at a resolution in dots per inch, on its paper colour, for boards,
//! presentations and the web. The PNG is written here (zlib from
//! miniz_oxide), without an image library.

use resvg::tiny_skia;
use resvg::usvg;

/// Biggest image side, pixels (an A0 at 300 dpi is 14 043 wide).
pub const MAX_SIDE: u32 = 20_000;

/// The SVG drawing as a PNG at `dpi`; its size on paper is the SVG's
/// width and height in millimetres (or, without them, its own pixels at
/// 96 dpi).
pub fn png(svg: &str, dpi: f64) -> Result<Vec<u8>, String> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(|e| e.to_string())?;
    let size = tree.size();
    let (w_mm, h_mm) = crate::paths::paper_mm(svg).unwrap_or((
        size.width() as f64 * 25.4 / 96.0,
        size.height() as f64 * 25.4 / 96.0,
    ));
    let px = |mm: f64| (mm / 25.4 * dpi).round().clamp(1.0, MAX_SIDE as f64) as u32;
    let (w, h) = (px(w_mm), px(h_mm));
    let mut pixmap = tiny_skia::Pixmap::new(w, h).ok_or("la imagen sería demasiado grande")?;
    pixmap.fill(tiny_skia::Color::WHITE);
    let t = tiny_skia::Transform::from_scale(w as f32 / size.width(), h as f32 / size.height());
    resvg::render(&tree, t, &mut pixmap.as_mut());
    // Opaque on white: the premultiplied colours are the colours.
    let rgb: Vec<u8> = pixmap
        .data()
        .chunks_exact(4)
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    Ok(encode_rgb(w, h, &rgb, dpi))
}

/// An RGB image as PNG, with its resolution written in it.
pub fn encode_rgb(w: u32, h: u32, rgb: &[u8], dpi: f64) -> Vec<u8> {
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8 bits, RGB
    chunk(&mut out, b"IHDR", &ihdr);
    // Pixels per metre.
    let ppm = ((dpi / 0.0254).round() as u32).to_be_bytes();
    let mut phys = Vec::with_capacity(9);
    phys.extend_from_slice(&ppm);
    phys.extend_from_slice(&ppm);
    phys.push(1);
    chunk(&mut out, b"pHYs", &phys);
    let row = w as usize * 3;
    let mut raw = Vec::with_capacity((row + 1) * h as usize);
    for y in 0..h as usize {
        raw.push(0); // no filter
        raw.extend_from_slice(&rgb[y * row..(y + 1) * row]);
    }
    let packed = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6);
    chunk(&mut out, b"IDAT", &packed);
    chunk(&mut out, b"IEND", &[]);
    out
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut c = crc32(kind, 0xFFFF_FFFF);
    c = crc32(data, c);
    out.extend_from_slice(&(c ^ 0xFFFF_FFFF).to_be_bytes());
}

fn crc32(bytes: &[u8], mut c: u32) -> u32 {
    for b in bytes {
        c ^= *b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drawing_comes_out_as_a_png_of_its_paper_size() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="25.4mm" height="12.7mm" viewBox="0 0 2 1"><rect x="0" y="0" width="1" height="1" fill="#ff0000"/></svg>"##;
        let png = png(svg, 100.0).unwrap();
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        // 1 × 0,5 inches at 100 dpi.
        assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 100);
        assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), 50);
        // The checksum of IHDR, as any reader checks it.
        assert_eq!(crc32(b"IEND", 0xFFFF_FFFF) ^ 0xFFFF_FFFF, 0xAE42_6082);
    }
}
