//! Preferences kept between sessions on this computer (`preferencias.json`
//! in the program's settings folder): the size of the text, the theme,
//! AutoCAD's keys and the onboarding missions done. Never read or written
//! by the tests.

use std::path::PathBuf;

/// How big the whole window is drawn: text, tiles, bars and Properties
/// grow together (`docs/INTERFAZ.md` §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextSize {
    #[default]
    Normal,
    Large,
    Larger,
}

impl TextSize {
    pub const ALL: [TextSize; 3] = [TextSize::Normal, TextSize::Large, TextSize::Larger];

    pub fn name(self) -> &'static str {
        match self {
            TextSize::Normal => "Normal",
            TextSize::Large => "Grande",
            TextSize::Larger => "Muy grande",
        }
    }

    /// egui's zoom factor.
    pub fn zoom(self) -> f32 {
        match self {
            TextSize::Normal => 1.0,
            TextSize::Large => 1.2,
            TextSize::Larger => 1.4,
        }
    }

    fn key(self) -> &'static str {
        match self {
            TextSize::Normal => "normal",
            TextSize::Large => "grande",
            TextSize::Larger => "muy grande",
        }
    }

    fn from_key(s: &str) -> Self {
        TextSize::ALL
            .into_iter()
            .find(|t| t.key() == s)
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Prefs {
    pub text_size: TextSize,
    pub dark: bool,
    pub keys_autocad: bool,
    /// The unit lengths are shown and typed in.
    pub units: crate::units::LengthUnit,
    /// Ids of the missions done.
    pub missions_done: Vec<String>,
    /// The welcome card offering the missions was answered.
    pub welcomed: bool,
    /// Projects opened or saved lately, the last first.
    pub recent: Vec<String>,
    /// The price bank (BC3) opened last, opened again when needed.
    pub price_bank: Option<String>,
    /// The quality of the 3D picture (none: as the computer goes).
    pub quality_3d: Option<crate::view3d::look::Quality>,
    /// How the 3D view draws the materials' textures: `completas`,
    /// `ligeras` or `no` (none: as the computer goes).
    pub textures_3d: Option<String>,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            text_size: TextSize::Normal,
            dark: false,
            keys_autocad: true,
            units: crate::units::LengthUnit::Metre,
            missions_done: Vec::new(),
            welcomed: false,
            recent: Vec::new(),
            price_bank: None,
            textures_3d: None,
            quality_3d: None,
        }
    }
}

/// The program's settings folder (the same as the library's and the
/// team's).
pub fn settings_dir() -> Option<PathBuf> {
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    }?;
    Some(base.join("ArchiSim"))
}

fn file() -> Option<PathBuf> {
    settings_dir().map(|d| d.join("preferencias.json"))
}

impl Prefs {
    pub fn to_json(&self) -> String {
        let v = serde_json::json!({
            "tamaño_letra": self.text_size.key(),
            "tema_oscuro": self.dark,
            "teclas_autocad": self.keys_autocad,
            "unidades": self.units.symbol(),
            "misiones_hechas": self.missions_done,
            "bienvenida": self.welcomed,
            "recientes": self.recent,
            "banco_de_precios": self.price_bank,
            "texturas_3d": self.textures_3d,
            "calidad_3d": self.quality_3d.map(|q| q.key()),
        });
        serde_json::to_string_pretty(&v).unwrap_or_default()
    }

    /// Reads what it knows and keeps the default for the rest (a file of
    /// a newer version, or touched by hand, still opens).
    pub fn from_json(text: &str) -> Self {
        let d = Prefs::default();
        let Ok(v) = serde_json::from_str::<serde_json::Value>(text) else {
            return d;
        };
        let flag = |k: &str, def: bool| v.get(k).and_then(|x| x.as_bool()).unwrap_or(def);
        Prefs {
            text_size: v
                .get("tamaño_letra")
                .and_then(|x| x.as_str())
                .map_or(d.text_size, TextSize::from_key),
            dark: flag("tema_oscuro", d.dark),
            keys_autocad: flag("teclas_autocad", d.keys_autocad),
            units: v
                .get("unidades")
                .and_then(|x| x.as_str())
                .and_then(crate::units::LengthUnit::from_symbol)
                .unwrap_or(d.units),
            missions_done: v
                .get("misiones_hechas")
                .and_then(|x| x.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
            welcomed: flag("bienvenida", d.welcomed),
            recent: v
                .get("recientes")
                .and_then(|x| x.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
            price_bank: v
                .get("banco_de_precios")
                .and_then(|x| x.as_str())
                .map(str::to_string),
            quality_3d: v
                .get("calidad_3d")
                .and_then(|x| x.as_str())
                .and_then(crate::view3d::look::Quality::from_key),
            // Version 0.47.2 kept on or off (on hung some Macs): that
            // choice is forgotten, and the computer's default taken.
            textures_3d: v
                .get("texturas_3d")
                .and_then(|x| x.as_str())
                .map(str::to_string),
        }
    }

    /// The preferences of this computer; in the tests, the defaults with
    /// the welcome already answered.
    pub fn load() -> Self {
        if cfg!(test) {
            return Prefs {
                welcomed: true,
                units: crate::units::LengthUnit::Millimetre,
                ..Default::default()
            };
        }
        file()
            .and_then(|f| std::fs::read_to_string(f).ok())
            .map(|t| Prefs::from_json(&t))
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if cfg!(test) {
            return;
        }
        if let Some(f) = file() {
            if let Some(dir) = f.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let tmp = f.with_extension("json.tmp");
            if std::fs::write(&tmp, self.to_json()).is_ok() {
                let _ = std::fs::rename(&tmp, &f);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_round_trip() {
        let p = Prefs {
            text_size: TextSize::Larger,
            dark: true,
            keys_autocad: false,
            units: crate::units::LengthUnit::Centimetre,
            missions_done: vec!["muro".into(), "plano".into()],
            welcomed: true,
            recent: vec!["/proyectos/casa.archisim".into()],
            price_bank: Some("/bancos/bcca.bc3".into()),
            quality_3d: Some(crate::view3d::look::Quality::Pretty),
            textures_3d: Some("ligeras".into()),
        };
        assert_eq!(Prefs::from_json(&p.to_json()), p);
        // Unknown or broken files give the defaults.
        assert_eq!(Prefs::from_json("no es json"), Prefs::default());
        let partial = Prefs::from_json(r#"{"tamaño_letra": "grande", "otra": 3}"#);
        assert_eq!(partial.text_size, TextSize::Large);
        assert!(partial.keys_autocad);
    }
}
