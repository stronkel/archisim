//! The orthophoto (V2): an aerial picture of the site (the PNOA's, or any
//! georeferenced PNG) laid over the terrain and, if asked, over the
//! streets, for the 3D view, the render, the glTF and the site plan. It is
//! kept as the picture and the plan rectangle it covers (turned when the
//! project's north is).

use super::*;

/// An aerial picture laid on the ground.
#[derive(Debug, Clone, PartialEq)]
pub struct Orthophoto {
    pub image: DecalImage,
    /// The plan point of the picture's bottom-left corner.
    pub origin: [f64; 2],
    /// The plan vectors of its bottom side (its width) and its left side
    /// (its height).
    pub u: [f64; 2],
    pub v: [f64; 2],
    /// Laid over the streets too (else only the terrain).
    pub on_roads: bool,
}

impl Orthophoto {
    /// Where a plan point falls on the picture: `(u, v)` from 0 to 1, `v`
    /// from the bottom; `None` outside it.
    pub fn uv(&self, p: Point2) -> Option<(f64, f64)> {
        let (dx, dy) = (p.x - self.origin[0], p.y - self.origin[1]);
        let det = self.u[0] * self.v[1] - self.u[1] * self.v[0];
        if det.abs() < 1e-12 {
            return None;
        }
        let a = (dx * self.v[1] - dy * self.v[0]) / det;
        let b = (self.u[0] * dy - self.u[1] * dx) / det;
        ((0.0..=1.0).contains(&a) && (0.0..=1.0).contains(&b)).then_some((a, b))
    }

    /// Its four corners in plan, anticlockwise from the bottom-left.
    pub fn corners(&self) -> [Point2; 4] {
        let o = self.origin;
        let (u, v) = (self.u, self.v);
        [
            Point2::new(o[0], o[1]),
            Point2::new(o[0] + u[0], o[1] + u[1]),
            Point2::new(o[0] + u[0] + v[0], o[1] + u[1] + v[1]),
            Point2::new(o[0] + v[0], o[1] + v[1]),
        ]
    }

    /// Its width and height, metres.
    pub fn size(&self) -> (f64, f64) {
        (self.u[0].hypot(self.u[1]), self.v[0].hypot(self.v[1]))
    }
}

impl Model {
    pub fn orthophoto(&self) -> Option<&Orthophoto> {
        self.orthophoto.as_ref()
    }

    /// Lays (or takes away) the orthophoto.
    pub fn set_orthophoto(&mut self, o: Option<Orthophoto>) -> Result<(), ModelError> {
        if let Some(o) = &o {
            if o.origin
                .iter()
                .chain(&o.u)
                .chain(&o.v)
                .any(|x| !x.is_finite())
            {
                return Err(ModelError::NotFinite("orthophoto"));
            }
            let (w, h) = o.size();
            let det = o.u[0] * o.v[1] - o.u[1] * o.v[0];
            if w < 1.0 || h < 1.0 || det.abs() < 1e-6 {
                return Err(ModelError::InvalidShape(
                    "an orthophoto covers at least a metre each way",
                ));
            }
            if png_size(&o.image.png).is_none() {
                return Err(ModelError::InvalidShape("the orthophoto is not a PNG"));
            }
        }
        self.orthophoto = o;
        Ok(())
    }

    /// Sets whether it is laid over the streets too.
    pub fn set_orthophoto_on_roads(&mut self, on: bool) -> Result<(), ModelError> {
        match &mut self.orthophoto {
            Some(o) => {
                o.on_roads = on;
                Ok(())
            }
            None => Err(ModelError::InvalidShape("there is no orthophoto")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32) -> Vec<u8> {
        // Only its header is read here.
        let mut b = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        b.extend_from_slice(&13u32.to_be_bytes());
        b.extend_from_slice(b"IHDR");
        b.extend_from_slice(&w.to_be_bytes());
        b.extend_from_slice(&h.to_be_bytes());
        b.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
        b
    }

    #[test]
    fn a_turned_picture_finds_its_points() {
        let mut m = Model::new("T");
        // 200 m by 100 m, turned 90°: its bottom runs north.
        let o = Orthophoto {
            image: decal_image("o.png", png(4, 2)).unwrap(),
            origin: [10.0, 0.0],
            u: [0.0, 200.0],
            v: [-100.0, 0.0],
            on_roads: true,
        };
        m.set_orthophoto(Some(o.clone())).unwrap();
        let o = m.orthophoto().unwrap();
        let (a, b) = o.uv(Point2::new(-40.0, 50.0)).unwrap();
        assert!((a - 0.25).abs() < 1e-9 && (b - 0.5).abs() < 1e-9);
        assert!(o.uv(Point2::new(20.0, 50.0)).is_none());
        assert_eq!(o.corners()[2], Point2::new(-90.0, 200.0));
        let mut bad = o.clone();
        bad.u = [0.0, 0.0];
        assert!(m.set_orthophoto(Some(bad)).is_err());
        m.set_orthophoto(None).unwrap();
        assert!(m.orthophoto().is_none());
    }
}
