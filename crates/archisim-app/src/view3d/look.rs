//! How the 3D view looks (bloque V1, `docs/MAQUETA.md` §4): an
//! illustration's light (a warm sun, a cool sky) with the sun's shadows,
//! and three levels of quality so an old laptop stays fluid.
//!
//! The shadows come from a depth picture of the scene seen from the sun
//! (a shadow map), drawn again only when the scene, the sun or the cut
//! away walls change: turning the camera costs nothing more.

/// The quality of the 3D picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Quality {
    /// No shadows and no supersampling: for old computers.
    Light,
    /// The sun's shadows, softened.
    #[default]
    Normal,
    /// Finer, softer shadows and the shade of inner corners (ambient
    /// occlusion).
    Pretty,
}

impl Quality {
    pub const ALL: [Quality; 3] = [Quality::Light, Quality::Normal, Quality::Pretty];

    pub fn name(self) -> &'static str {
        match self {
            Quality::Light => "Ligera",
            Quality::Normal => "Normal",
            Quality::Pretty => "Bonita",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Quality::Light => "Sin sombras y sin suavizado: para ordenadores antiguos",
            Quality::Normal => "Con las sombras del sol",
            Quality::Pretty => {
                "Sombras más finas y suaves y la penumbra de los rincones: para capturas"
            }
        }
    }

    /// As written in the preferences.
    pub fn key(self) -> &'static str {
        match self {
            Quality::Light => "ligera",
            Quality::Normal => "normal",
            Quality::Pretty => "bonita",
        }
    }

    pub fn from_key(s: &str) -> Option<Self> {
        Quality::ALL.into_iter().find(|q| q.key() == s)
    }

    /// The side of the shadow map in pixels (none: no shadows).
    pub fn shadow_size(self) -> Option<u32> {
        match self {
            Quality::Light => None,
            Quality::Normal => Some(2048),
            Quality::Pretty => Some(4096),
        }
    }

    /// Samples of the shadow map a point takes, for its soft edge.
    pub fn shadow_taps(self) -> f32 {
        match self {
            Quality::Light => 1.0,
            Quality::Normal => 5.0,
            Quality::Pretty => 9.0,
        }
    }

    /// The shade of inner corners.
    pub fn occlusion(self) -> bool {
        self == Quality::Pretty
    }

    /// The picture may be drawn larger than the screen and scaled down.
    pub fn supersample(self) -> bool {
        self != Quality::Light
    }
}

/// The light of the picture (in the Maqueta): the project's sun, the
/// golden light of the evening, or the night with the lamps on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Light {
    #[default]
    Day,
    Evening,
    Night,
}

impl Light {
    pub const ALL: [Light; 3] = [Light::Day, Light::Evening, Light::Night];

    pub fn name(self) -> &'static str {
        match self {
            Light::Day => "Día",
            Light::Evening => "Atardecer",
            Light::Night => "Noche",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Light::Day => "El sol del proyecto (su fecha, hora y norte)",
            Light::Evening => "Sol bajo y dorado del oeste, sombras largas",
            Light::Night => {
                "Cielo de noche: las lámparas encienden sus pantallas y alumbran alrededor"
            }
        }
    }

    /// The direction towards the sun for this light, from the project's
    /// (none: no sun, at night).
    pub fn sun(self, project: Option<glam::Vec3>) -> Option<glam::Vec3> {
        match self {
            Light::Day => project,
            // Low in the west-south-west.
            Light::Evening => Some(glam::Vec3::new(-0.86, -0.32, 0.26).normalize()),
            Light::Night => None,
        }
    }

    /// The colours of the light: the sun's (and its strength), the sky's
    /// above and at the horizon (and their strength), the haze, and the
    /// ground far away. Linear RGB.
    pub fn colours(self) -> LightColours {
        match self {
            Light::Day => LightColours {
                sun: [1.0, 0.94, 0.84, 0.58],
                sky_hi: [0.84, 0.89, 1.0, 1.0],
                sky_lo: [0.60, 0.60, 0.64, 1.0],
                zenith: [0.42, 0.62, 0.88, 1.0],
                haze: [0.80, 0.86, 0.92, 1.0],
                ground: [0.72, 0.73, 0.70, 1.0],
                night: 0.0,
            },
            Light::Evening => LightColours {
                sun: [1.0, 0.70, 0.42, 0.78],
                sky_hi: [0.62, 0.68, 0.90, 0.85],
                sky_lo: [0.78, 0.62, 0.52, 0.85],
                zenith: [0.30, 0.42, 0.72, 1.0],
                haze: [0.96, 0.74, 0.56, 1.0],
                ground: [0.60, 0.52, 0.44, 1.0],
                night: 0.25,
            },
            Light::Night => LightColours {
                sun: [0.0, 0.0, 0.0, 0.0],
                sky_hi: [0.20, 0.26, 0.46, 0.26],
                sky_lo: [0.12, 0.13, 0.20, 0.26],
                zenith: [0.012, 0.018, 0.05, 1.0],
                haze: [0.06, 0.08, 0.16, 1.0],
                ground: [0.05, 0.055, 0.07, 1.0],
                night: 1.0,
            },
        }
    }
}

/// See [`Light::colours`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightColours {
    pub sun: [f32; 4],
    pub sky_hi: [f32; 4],
    pub sky_lo: [f32; 4],
    pub zenith: [f32; 4],
    pub haze: [f32; 4],
    pub ground: [f32; 4],
    /// 0 by day, 1 at night: how much the lamps glow and light.
    pub night: f32,
}

/// What surrounds the model in the Maqueta: the sky and the ground fading
/// into the haze, the grid on the ground, the light and the lamps.
#[derive(Debug, Clone, PartialEq)]
pub struct Atmosphere {
    pub light: Light,
    /// The height of the ground (the level's).
    pub ground_z: f32,
    /// The grid on the ground: its centre, how far it reaches and its
    /// step (metres).
    pub grid: Option<(glam::Vec2, f32, f32)>,
}

/// The quality a computer starts with: light on Intel Macs, whose AMD
/// graphics drivers already hung on the textures.
pub fn default_quality() -> Quality {
    if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Quality::Light
    } else {
        Quality::Normal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_read_back() {
        for q in Quality::ALL {
            assert_eq!(Quality::from_key(q.key()), Some(q));
        }
        assert!(Quality::Light.shadow_size().is_none());
        assert!(Quality::Pretty.shadow_size() > Quality::Normal.shadow_size());
        assert!(Light::Night.sun(Some(glam::Vec3::Z)).is_none());
        assert_eq!(Light::Day.sun(Some(glam::Vec3::Z)), Some(glam::Vec3::Z));
    }
}
