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
    }
}
