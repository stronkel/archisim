//! Textures (PNG and JPEG), sampled where each vertex sits on them: the 3D
//! view and the IFC get one colour per vertex, not the image.

use crate::Rgb;

/// A decoded image, rows from the top.
pub struct Texture {
    pub width: usize,
    pub height: usize,
    /// RGB, row by row.
    pub pixels: Vec<u8>,
}

impl Texture {
    pub fn decode(bytes: &[u8]) -> Option<Texture> {
        if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
            decode_png(bytes)
        } else if bytes.starts_with(&[0xFF, 0xD8]) {
            decode_jpeg(bytes)
        } else {
            None
        }
    }

    /// The colour at `(u, v)`, repeating outside 0–1; `v` from the bottom
    /// when `flip` (OBJ, COLLADA), from the top otherwise (glTF). The
    /// average of a few pixels around, so that a vertex does not take the
    /// colour of one stray pixel.
    pub fn sample(&self, u: f32, v: f32, flip: bool) -> Rgb {
        let wrap = |t: f32| t - t.floor();
        let u = wrap(u);
        let v = if flip { 1.0 - wrap(v) } else { wrap(v) };
        let x = ((u * self.width as f32) as isize).clamp(0, self.width as isize - 1);
        let y = ((v * self.height as f32) as isize).clamp(0, self.height as isize - 1);
        let r = (self.width.min(self.height) / 64).max(1) as isize;
        let mut sum = [0u32; 3];
        let mut n = 0;
        for dy in [-r, 0, r] {
            for dx in [-r, 0, r] {
                let px = (x + dx).rem_euclid(self.width as isize) as usize;
                let py = (y + dy).rem_euclid(self.height as isize) as usize;
                let i = (py * self.width + px) * 3;
                for (s, p) in sum.iter_mut().zip(&self.pixels[i..i + 3]) {
                    *s += *p as u32;
                }
                n += 1;
            }
        }
        sum.map(|s| (s / n) as u8)
    }

    /// The average colour of the whole image.
    pub fn average(&self) -> Rgb {
        let mut sum = [0u64; 3];
        let n = (self.width * self.height).max(1) as u64;
        for p in self.pixels.chunks_exact(3) {
            for k in 0..3 {
                sum[k] += p[k] as u64;
            }
        }
        sum.map(|s| (s / n) as u8)
    }
}

fn decode_png(bytes: &[u8]) -> Option<Texture> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width as usize, info.height as usize);
    let channels = match info.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Indexed => return None,
    };
    let mut pixels = Vec::with_capacity(w * h * 3);
    for p in buf[..w * h * channels].chunks_exact(channels) {
        match channels {
            1 | 2 => pixels.extend([p[0], p[0], p[0]]),
            _ => pixels.extend([p[0], p[1], p[2]]),
        }
    }
    (w > 0 && h > 0).then_some(Texture {
        width: w,
        height: h,
        pixels,
    })
}

fn decode_jpeg(bytes: &[u8]) -> Option<Texture> {
    let mut d = zune_jpeg::JpegDecoder::new(std::io::Cursor::new(bytes));
    let data = d.decode().ok()?;
    let info = d.info()?;
    let (w, h) = (info.width as usize, info.height as usize);
    let channels = data.len() / (w * h).max(1);
    let pixels = match channels {
        3 => data,
        1 => data.iter().flat_map(|&g| [g, g, g]).collect(),
        c if c >= 4 => data
            .chunks_exact(c)
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect(),
        _ => return None,
    };
    (w > 0 && h > 0).then_some(Texture {
        width: w,
        height: h,
        pixels,
    })
}

/// A colour tinted by another (texture × material factor).
pub fn tint(a: Rgb, b: Rgb) -> Rgb {
    [0, 1, 2].map(|k| ((a[k] as u32 * b[k] as u32) / 255) as u8)
}

/// From linear 0–1 (glTF's and COLLADA's factors) to 0–255.
pub fn from_unit(c: [f64; 3]) -> Rgb {
    c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// From glTF's linear colour factors (0–1) to sRGB 0–255.
pub fn from_linear(c: [f64; 3]) -> Rgb {
    c.map(|v| {
        let v = v.clamp(0.0, 1.0);
        let s = if v <= 0.003_130_8 {
            v * 12.92
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        };
        (s * 255.0).round() as u8
    })
}
