//! Mechanical properties of the sections of columns and beams, and of the
//! materials they are made of.

use archisim_core::{Material, Point2, Profile};

use crate::frame::Section;

/// What a member is made of, for its stiffness and its checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Concrete,
    Steel,
    Timber,
    Aluminium,
    Masonry,
    Other,
}

impl Kind {
    pub fn of(m: Option<&Material>) -> Kind {
        let Some(m) = m else {
            return Kind::Other;
        };
        let n = m.name.to_lowercase();
        let has = |w: &[&str]| w.iter().any(|x| n.contains(x));
        if has(&["hormigón", "hormigon", "ha-", "ha "]) {
            Kind::Concrete
        } else if has(&["acero", "steel", "ipe", "heb", "hea", "upn"]) {
            Kind::Steel
        } else if has(&[
            "madera", "glulam", "laminada", "clt", "pino", "roble", "abeto",
        ]) {
            Kind::Timber
        } else if has(&["aluminio"]) {
            Kind::Aluminium
        } else if has(&["ladrillo", "bloque", "fábrica", "fabrica", "piedra"]) {
            Kind::Masonry
        } else {
            match m.category.as_str() {
                "concrete" => Kind::Concrete,
                "steel" | "metal" => Kind::Steel,
                "wood" | "timber" => Kind::Timber,
                "aluminium" => Kind::Aluminium,
                "brick" | "masonry" | "stone" => Kind::Masonry,
                _ => Kind::Other,
            }
        }
    }

    /// Young's and shear modulus, kN/m² (HA-25: Ecm of the Código
    /// Estructural; S275; C24 mean).
    pub fn moduli(self) -> (f64, f64) {
        match self {
            Kind::Concrete => (27.3e6, 27.3e6 / 2.4),
            Kind::Steel => (210e6, 81e6),
            Kind::Timber => (11e6, 0.69e6),
            Kind::Aluminium => (70e6, 27e6),
            Kind::Masonry => (4e6, 1.6e6),
            Kind::Other => (27.3e6, 11e6),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::Concrete => "hormigón armado",
            Kind::Steel => "acero",
            Kind::Timber => "madera",
            Kind::Aluminium => "aluminio",
            Kind::Masonry => "fábrica",
            Kind::Other => "otro",
        }
    }
}

/// Area and second moments of a polygon about its centroid: (A, cx, cy,
/// Ixx = ∫y², Iyy = ∫x²).
fn polygon_props(p: &[Point2]) -> (f64, f64, f64, f64, f64) {
    let n = p.len();
    let (mut a, mut cx, mut cy, mut ixx, mut iyy) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for i in 0..n {
        let (p0, p1) = (p[i], p[(i + 1) % n]);
        let c = p0.x * p1.y - p1.x * p0.y;
        a += c;
        cx += (p0.x + p1.x) * c;
        cy += (p0.y + p1.y) * c;
        ixx += (p0.y * p0.y + p0.y * p1.y + p1.y * p1.y) * c;
        iyy += (p0.x * p0.x + p0.x * p1.x + p1.x * p1.x) * c;
    }
    a /= 2.0;
    if a.abs() < 1e-18 {
        return (0.0, 0.0, 0.0, 0.0, 0.0);
    }
    cx /= 6.0 * a;
    cy /= 6.0 * a;
    ixx /= 12.0;
    iyy /= 12.0;
    // About the centroid.
    let ixx = ixx - a * cy * cy;
    let iyy = iyy - a * cx * cx;
    (a.abs(), cx, cy, ixx.abs(), iyy.abs())
}

/// The geometric properties of a profile: area, I about the section's x
/// axis (∫y², the strong axis when the depth is along y), I about its y
/// axis, torsion constant; m², m⁴.
pub fn profile_props(p: Profile) -> (f64, f64, f64, f64) {
    let out = p.outline(64);
    let (mut a, _, _, mut ix, mut iy) = polygon_props(&out);
    if let Some(h) = p.hole(64) {
        let (ha, _, _, hx, hy) = polygon_props(&h);
        a -= ha;
        ix -= hx;
        iy -= hy;
    }
    let j = match p {
        Profile::Rectangle { width, depth } => {
            let (l, s) = if width > depth {
                (width, depth)
            } else {
                (depth, width)
            };
            l * s.powi(3) * (1.0 / 3.0 - 0.21 * (s / l) * (1.0 - s.powi(4) / (12.0 * l.powi(4))))
        }
        Profile::Circle { diameter } => std::f64::consts::PI * diameter.powi(4) / 32.0,
        Profile::CircleHollow { diameter, wall } => {
            let (r, i) = (diameter / 2.0, diameter / 2.0 - wall);
            std::f64::consts::PI / 2.0 * (r.powi(4) - i.powi(4))
        }
        Profile::RectHollow { width, depth, wall } => {
            let (bm, hm) = (width - wall, depth - wall);
            4.0 * (bm * hm).powi(2) * wall / (2.0 * (bm + hm))
        }
        Profile::IShape {
            width,
            depth,
            web,
            flange,
        }
        | Profile::UShape {
            width,
            depth,
            web,
            flange,
        } => (2.0 * width * flange.powi(3) + (depth - 2.0 * flange) * web.powi(3)) / 3.0,
        Profile::TShape {
            width,
            depth,
            web,
            flange,
        } => (width * flange.powi(3) + (depth - flange) * web.powi(3)) / 3.0,
        Profile::LShape {
            width,
            depth,
            thickness,
        } => (width + depth - thickness) * thickness.powi(3) / 3.0,
    };
    (a, ix, iy, j)
}

/// The frame section of a profile in a material. The bar's local `z` is
/// along the profile's depth.
pub fn section_of(p: Profile, kind: Kind) -> Section {
    let (a, ix, iy, j) = profile_props(p);
    let (e, g) = kind.moduli();
    Section {
        a,
        iy: ix,
        iz: iy,
        j,
        e,
        g,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn properties_match_the_tables() {
        // IPE 200: A 28,5 cm², Iy 1 943 cm⁴, Iz 142 cm⁴ (without roots:
        // a little less).
        let ipe = Profile::IShape {
            width: 0.1,
            depth: 0.2,
            web: 0.0056,
            flange: 0.0085,
        };
        let (a, ix, iy, j) = profile_props(ipe);
        assert!((a * 1e4 - 27.2).abs() < 0.5, "{}", a * 1e4);
        assert!((ix * 1e8 - 1943.0).abs() / 1943.0 < 0.08, "{}", ix * 1e8);
        assert!((iy * 1e8 - 142.0).abs() / 142.0 < 0.08, "{}", iy * 1e8);
        assert!(j > 0.0);
        // A 30 × 50 rectangle.
        let (a, ix, iy, _) = profile_props(Profile::Rectangle {
            width: 0.3,
            depth: 0.5,
        });
        assert!((a - 0.15).abs() < 1e-9);
        assert!((ix - 0.3 * 0.125 / 12.0).abs() < 1e-9);
        assert!((iy - 0.5 * 0.027 / 12.0).abs() < 1e-9);
    }
}
