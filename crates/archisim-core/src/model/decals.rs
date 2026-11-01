//! Decals for the render (U8): a picture (a PNG, with its transparency)
//! stuck on any face — a graffiti, a poster, a shop sign, a damp stain, a
//! worn zebra crossing. It is placed by its centre on the face, the
//! face's outward normal and a turn on it, at a width (its height keeps
//! the picture's proportion); it shows in the 3D view, the render and the
//! glTF. It does not follow the face it was stuck on: it remembers which
//! element that was, to say so.

use std::sync::Arc;

use super::*;

/// The picture of a decal: the PNG as it came, and its width over its
/// height.
#[derive(Debug, Clone, PartialEq)]
pub struct DecalImage {
    /// Where it came from (the file's name), to show.
    pub source: String,
    pub png: Arc<Vec<u8>>,
    pub aspect: f64,
}

/// A picture stuck on a face.
#[derive(Debug, Clone, PartialEq)]
pub struct Decal {
    pub id: DecalId,
    pub guid: GlobalId,
    pub name: String,
    pub level: LevelId,
    /// Its centre: x and y in plan, z over the level.
    pub centre: [f64; 3],
    /// The outward normal of the face it is on (of length 1).
    pub normal: [f64; 3],
    /// Its turn on the face, radians, anticlockwise seen from the front
    /// (0: upright on a wall; on a floor, its top to the north).
    pub rotation: f64,
    pub width: f64,
    pub image: DecalImage,
    /// The element it was stuck on, if any.
    pub host: Option<u64>,
}

/// Data to stick a decal.
#[derive(Debug, Clone, PartialEq)]
pub struct NewDecal {
    pub name: String,
    pub level: LevelId,
    pub centre: [f64; 3],
    pub normal: [f64; 3],
    pub rotation: f64,
    pub width: f64,
    pub image: DecalImage,
    pub host: Option<u64>,
}

/// Smallest and largest decals (m).
pub const MIN_DECAL: f64 = 0.02;
pub const MAX_DECAL: f64 = 200.0;
/// How far a decal stands off its face, so that it is drawn over it.
pub const DECAL_OFFSET: f64 = 0.004;

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(a: [f64; 3]) -> Option<[f64; 3]> {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    (l > 1e-9 && l.is_finite()).then(|| [a[0] / l, a[1] / l, a[2] / l])
}

impl Decal {
    pub fn height(&self) -> f64 {
        self.width / self.image.aspect.max(1e-6)
    }

    /// Whether it lies on a floor or a ceiling (not on a wall).
    pub fn lying(&self) -> bool {
        self.normal[2].abs() > 0.9
    }

    /// Its axes on the face: to the right and up of the picture, seen
    /// from the front.
    pub fn axes(&self) -> ([f64; 3], [f64; 3]) {
        let n = unit(self.normal).unwrap_or([0.0, 0.0, 1.0]);
        let up0 = if n[2].abs() > 0.9 {
            // On a floor its top to the north; on a ceiling, seen from
            // below, too.
            [0.0, 1.0, 0.0]
        } else {
            [0.0, 0.0, 1.0]
        };
        // Square to the normal.
        let right = unit(cross(up0, n)).unwrap_or([1.0, 0.0, 0.0]);
        let up = cross(n, right);
        let (s, c) = self.rotation.sin_cos();
        let r = [
            right[0] * c + up[0] * s,
            right[1] * c + up[1] * s,
            right[2] * c + up[2] * s,
        ];
        let u = [
            up[0] * c - right[0] * s,
            up[1] * c - right[1] * s,
            up[2] * c - right[2] * s,
        ];
        (r, u)
    }

    /// Its four corners in the project (z absolute, from the level's
    /// elevation `base`), anticlockwise from the bottom left, off the face
    /// by `DECAL_OFFSET`.
    pub fn corners(&self, base: f64) -> [[f64; 3]; 4] {
        let (r, u) = self.axes();
        let n = unit(self.normal).unwrap_or([0.0, 0.0, 1.0]);
        let (hw, hh) = (self.width / 2.0, self.height() / 2.0);
        let c = [
            self.centre[0] + n[0] * DECAL_OFFSET,
            self.centre[1] + n[1] * DECAL_OFFSET,
            self.centre[2] + base + n[2] * DECAL_OFFSET,
        ];
        let at = |a: f64, b: f64| {
            [
                c[0] + r[0] * a + u[0] * b,
                c[1] + r[1] * a + u[1] * b,
                c[2] + r[2] * a + u[2] * b,
            ]
        };
        [at(-hw, -hh), at(hw, -hh), at(hw, hh), at(-hw, hh)]
    }

    pub fn spec(&self) -> NewDecal {
        NewDecal {
            name: self.name.clone(),
            level: self.level,
            centre: self.centre,
            normal: self.normal,
            rotation: self.rotation,
            width: self.width,
            image: self.image.clone(),
            host: self.host,
        }
    }
}

impl Model {
    fn validate_decal(&self, d: &NewDecal) -> Result<(), ModelError> {
        if self.level(d.level).is_none() {
            return Err(ModelError::UnknownLevel(d.level));
        }
        let ok = d.centre.iter().all(|v| v.is_finite())
            && unit(d.normal).is_some()
            && d.rotation.is_finite()
            && (MIN_DECAL..=MAX_DECAL).contains(&d.width)
            && d.image.aspect.is_finite()
            && (0.01..=100.0).contains(&d.image.aspect)
            && (MIN_DECAL..=MAX_DECAL).contains(&(d.width / d.image.aspect))
            && d.image.png.starts_with(b"\x89PNG");
        if ok {
            Ok(())
        } else {
            Err(ModelError::InvalidShape(
                "a decal needs a PNG picture, a face and a size of 2 cm to 200 m",
            ))
        }
    }

    pub fn add_decal(&mut self, spec: NewDecal) -> Result<DecalId, ModelError> {
        let mut spec = spec;
        spec.normal = unit(spec.normal).unwrap_or(spec.normal);
        self.validate_decal(&spec)?;
        let id = DecalId(self.take_id());
        self.decals.insert(
            id,
            Decal {
                id,
                guid: GlobalId::new(),
                name: spec.name,
                level: spec.level,
                centre: spec.centre,
                normal: spec.normal,
                rotation: spec.rotation,
                width: spec.width,
                image: spec.image,
                host: spec.host,
            },
        );
        Ok(id)
    }

    pub fn edit_decal(
        &mut self,
        id: DecalId,
        edit: impl FnOnce(&mut Decal),
    ) -> Result<(), ModelError> {
        let mut d = self
            .decals
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        edit(&mut d);
        d.normal = unit(d.normal).unwrap_or(d.normal);
        self.validate_decal(&d.spec())?;
        self.decals.insert(id, d);
        Ok(())
    }

    pub fn remove_decal(&mut self, id: DecalId) -> Result<Decal, ModelError> {
        self.decals
            .remove(&id)
            .ok_or(ModelError::UnknownElement(id.0))
    }

    pub fn decal(&self, id: DecalId) -> Option<&Decal> {
        self.decals.get(&id)
    }

    pub fn decals(&self) -> impl Iterator<Item = &Decal> {
        self.decals.values()
    }

    pub fn decals_on_level(&self, level: LevelId) -> impl Iterator<Item = &Decal> {
        self.decals.values().filter(move |d| d.level == level)
    }

    /// «Pegatina N», the next free.
    pub fn next_decal_name(&self) -> String {
        (1..)
            .map(|i| format!("Pegatina {i}"))
            .find(|n| !self.decals.values().any(|d| &d.name == n))
            .unwrap_or_default()
    }

    pub(super) fn restore_decals(
        &mut self,
        list: Vec<Decal>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for d in list {
            claim(d.id.0, Some(&d.guid))
                .and_then(|_| self.validate_decal(&d.spec()))
                .map_err(|error| RestoreError {
                    element: format!("decal {}", d.id.0),
                    error,
                })?;
            self.decals.insert(d.id, d);
        }
        Ok(())
    }
}

/// The size of a PNG (width, height) from its header, without decoding
/// it.
pub fn png_size(png: &[u8]) -> Option<(u32, u32)> {
    if png.len() < 24 || !png.starts_with(b"\x89PNG\r\n\x1a\n") || &png[12..16] != b"IHDR" {
        return None;
    }
    let be = |i: usize| u32::from_be_bytes([png[i], png[i + 1], png[i + 2], png[i + 3]]);
    let (w, h) = (be(16), be(20));
    (w > 0 && h > 0).then_some((w, h))
}

/// A decal's picture from a PNG file's bytes.
pub fn decal_image(source: &str, png: Vec<u8>) -> Option<DecalImage> {
    let (w, h) = png_size(&png)?;
    Some(DecalImage {
        source: source.to_string(),
        png: Arc::new(png),
        aspect: w as f64 / h as f64,
    })
}

/// Bytes as base64 (for the pictures in the IFC).
pub fn base64_encode(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        out.push(A[(n >> 18) as usize & 63] as char);
        out.push(A[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 {
            A[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if c.len() > 2 {
            A[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// What `base64_encode` wrote; `None` if it is not base64.
pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => continue,
            _ => return None,
        } as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 2 × 1 PNG header (enough for the size; the pixels are not read
    /// here).
    pub(crate) fn tiny_png() -> Vec<u8> {
        let mut v = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        v.extend(2u32.to_be_bytes());
        v.extend(1u32.to_be_bytes());
        v.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]);
        v
    }

    #[test]
    fn a_decal_on_a_wall_and_on_the_floor() {
        let mut m = Model::new("T");
        let l = m.add_level("Planta baja", 0.0);
        let image = decal_image("cartel.png", tiny_png()).unwrap();
        assert_eq!(image.aspect, 2.0);
        // On a wall facing south, 2 m wide: 1 m high, its right to the
        // east.
        let id = m
            .add_decal(NewDecal {
                name: m.next_decal_name(),
                level: l,
                centre: [5.0, 0.0, 1.5],
                normal: [0.0, -2.0, 0.0],
                rotation: 0.0,
                width: 2.0,
                image: image.clone(),
                host: None,
            })
            .unwrap();
        let d = m.decal(id).unwrap();
        assert_eq!(d.name, "Pegatina 1");
        assert_eq!(d.normal, [0.0, -1.0, 0.0]);
        assert!((d.height() - 1.0).abs() < 1e-12);
        let c = d.corners(0.0);
        assert!((c[0][0] - 4.0).abs() < 1e-9 && (c[0][2] - 1.0).abs() < 1e-9);
        assert!((c[2][0] - 6.0).abs() < 1e-9 && (c[2][2] - 2.0).abs() < 1e-9);
        assert!((c[0][1] + DECAL_OFFSET).abs() < 1e-9);
        // On the floor, turned a quarter: its width along y.
        let f = m
            .add_decal(NewDecal {
                name: m.next_decal_name(),
                level: l,
                centre: [0.0, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                rotation: std::f64::consts::FRAC_PI_2,
                width: 2.0,
                image,
                host: None,
            })
            .unwrap();
        let d = m.decal(f).unwrap();
        assert!(d.lying());
        let c = d.corners(0.0);
        assert!((c[1][1] - c[0][1] - 2.0).abs() < 1e-9, "{c:?}");
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back.decals().count(), 2);
        // Not a PNG: refused.
        let bad = DecalImage {
            source: String::new(),
            png: Arc::new(vec![1, 2, 3]),
            aspect: 1.0,
        };
        assert!(m.edit_decal(id, |x| x.image = bad).is_err());
        let bytes = tiny_png();
        assert_eq!(base64_decode(&base64_encode(&bytes)), Some(bytes));
    }
}
