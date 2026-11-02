//! `archisim ortofoto <proyecto.archisim> [salida.archisim]`: lays on a
//! georeferenced project a test orthophoto (V2) made here from its own
//! model —a picture from above of its roofs, streets and ground, with a
//! little grain— so that the draping can be seen without downloading the
//! PNOA's. It also writes the picture with its world file (`.png` and
//! `.pgw`), to drop on «Traer el entorno».

use std::path::Path;

use archisim_core::Model;
use archisim_geom::solid::{model_solids, solid_color, SolidOwner};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Metres a pixel of the test picture.
const PIXEL: f64 = 0.5;

/// A grain that repeats nowhere near (hash of the pixel).
fn grain(x: i64, y: i64) -> f64 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 29;
    (h % 1000) as f64 / 1000.0
}

/// The picture from above of the model in the map rectangle `bbox`
/// (west, south, east, north), by its georeference: RGB rows from the
/// top, and its size.
pub fn aerial(m: &Model, bbox: [f64; 4]) -> (Vec<u8>, u32, u32) {
    let g = m.georeference().cloned().expect("georeferenced");
    let w = ((bbox[2] - bbox[0]) / PIXEL).round() as usize;
    let h = ((bbox[3] - bbox[1]) / PIXEL).round() as usize;
    let mut z = vec![f64::MIN; w * h];
    let mut rgb = vec![0u8; w * h * 3];
    // Plan → map, as the importer's frame.
    let a = m.true_north() + g.convergence();
    let (c, s) = (a.cos(), a.sin());
    let to_px = |p: [f64; 3]| -> (f64, f64) {
        let e = g.eastings + p[0] * c - p[1] * s;
        let n = g.northings + p[0] * s + p[1] * c;
        ((e - bbox[0]) / PIXEL, (bbox[3] - n) / PIXEL)
    };
    for sol in model_solids(m) {
        let base = match sol.owner {
            SolidOwner::Terrain(_) | SolidOwner::Growth(_) => None,
            SolidOwner::Orthophoto | SolidOwner::Decal(_) => continue,
            _ => Some(solid_color(m, &sol)),
        };
        for t in &sol.triangles {
            let ab = [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]];
            let ac = [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]];
            let n = [
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            ];
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if len < 1e-12 || n[2] / len < 0.05 {
                continue;
            }
            // Lit from the south-west, as a morning flight.
            let light = (0.75 + 0.25 * (-n[0] - n[1] + n[2]) / len / 1.5).clamp(0.4, 1.1);
            let p = t.map(to_px);
            let x0 = p
                .iter()
                .map(|q| q.0)
                .fold(f64::MAX, f64::min)
                .floor()
                .max(0.0) as usize;
            let x1 = (p.iter().map(|q| q.0).fold(f64::MIN, f64::max).ceil() as usize).min(w);
            let y0 = p
                .iter()
                .map(|q| q.1)
                .fold(f64::MAX, f64::min)
                .floor()
                .max(0.0) as usize;
            let y1 = (p.iter().map(|q| q.1).fold(f64::MIN, f64::max).ceil() as usize).min(h);
            let d = (p[1].1 - p[2].1) * (p[0].0 - p[2].0) + (p[2].0 - p[1].0) * (p[0].1 - p[2].1);
            if d.abs() < 1e-12 {
                continue;
            }
            for y in y0..y1 {
                for x in x0..x1 {
                    let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                    let l0 =
                        ((p[1].1 - p[2].1) * (px - p[2].0) + (p[2].0 - p[1].0) * (py - p[2].1)) / d;
                    let l1 =
                        ((p[2].1 - p[0].1) * (px - p[2].0) + (p[0].0 - p[2].0) * (py - p[2].1)) / d;
                    let l2 = 1.0 - l0 - l1;
                    if l0 < 0.0 || l1 < 0.0 || l2 < 0.0 {
                        continue;
                    }
                    let zz = l0 * t[0][2] + l1 * t[1][2] + l2 * t[2][2];
                    let i = y * w + x;
                    if zz <= z[i] {
                        continue;
                    }
                    z[i] = zz;
                    let gr = grain(x as i64, y as i64);
                    let col = match base {
                        Some(c) => c.map(|v| v as f64 * (0.92 + 0.16 * gr)),
                        // Ground: dry grass and earth in patches.
                        None => {
                            let patch = grain(x as i64 / 9, y as i64 / 9);
                            let k = 0.6 * patch + 0.4 * gr;
                            [118.0 + 60.0 * k, 128.0 + 30.0 * k, 72.0 + 20.0 * k]
                        }
                    };
                    for (j, v) in col.iter().enumerate() {
                        rgb[i * 3 + j] = (v * light).clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }
    }
    // Beyond the model, bare ground.
    for (i, zz) in z.iter().enumerate() {
        if *zz == f64::MIN {
            let (x, y) = ((i % w) as i64, (i / w) as i64);
            let k = 0.6 * grain(x / 9, y / 9) + 0.4 * grain(x, y);
            let c = [150.0 + 50.0 * k, 140.0 + 30.0 * k, 96.0 + 20.0 * k];
            for (j, v) in c.iter().enumerate() {
                rgb[i * 3 + j] = *v as u8;
            }
        }
    }
    (rgb, w as u32, h as u32)
}

pub fn run(project: &Path, out: &Path) -> Res {
    let mut m = archisim_file::open(project)?;
    let Some(g) = m.georeference().cloned() else {
        return Err("el proyecto no está georreferenciado".into());
    };
    m.set_orthophoto(None)?;
    // The rectangle round the project's origin that holds its terrain.
    let r = 160.0;
    let bbox = [
        g.eastings - r,
        g.northings - r,
        g.eastings + r,
        g.northings + r,
    ];
    let (rgb, w, h) = aerial(&m, bbox);
    let png = archisim_export::png::encode_rgb(w, h, &rgb, 96.0);
    archisim_library::carto::place_orthophoto(&mut m, "ortofoto-ensayo.png", png.clone(), bbox)?;
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir)?;
    }
    archisim_file::save(&m, out)?;
    let pic = out.with_file_name("ortofoto-ensayo.png");
    std::fs::write(&pic, &png)?;
    std::fs::write(
        pic.with_extension("pgw"),
        format!(
            "{PIXEL}\n0\n0\n-{PIXEL}\n{}\n{}\n",
            bbox[0] + PIXEL / 2.0,
            bbox[3] - PIXEL / 2.0
        ),
    )?;
    println!(
        "{} con ortofoto de ensayo de {w} × {h} píxeles ({} y su .pgw)",
        out.display(),
        pic.display()
    );
    Ok(())
}
