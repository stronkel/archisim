//! Light framing (steel frame and timber frame): what a wall type or a
//! slab type is framed with, kept apart from the type, by its id, as the
//! reinforcement is (so that the types stay as they are). The members
//! themselves are not stored: `archisim_geom::framing` lays them from the
//! walls, slabs and roofs of those types, and the cut list comes from
//! them. Generic sections (no maker's catalogue): cold-formed steel C
//! studs and U tracks, and C24 timber.

use super::*;

/// The name of the material of steel members.
pub const FRAME_STEEL: &str = "Acero galvanizado (entramado)";
/// The name of the material of timber members.
pub const FRAME_TIMBER: &str = "Madera C24 (entramado)";

/// What the frame is made of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameMaterial {
    /// Cold-formed galvanised steel (light steel frame).
    Steel,
    /// Sawn timber, C24 (timber frame).
    Timber,
}

impl FrameMaterial {
    pub const ALL: [FrameMaterial; 2] = [FrameMaterial::Steel, FrameMaterial::Timber];

    pub fn name(self) -> &'static str {
        match self {
            FrameMaterial::Steel => "Steel frame (acero galvanizado)",
            FrameMaterial::Timber => "Entramado de madera (C24)",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            FrameMaterial::Steel => "steel",
            FrameMaterial::Timber => "timber",
        }
    }

    pub fn from_code(s: &str) -> Option<Self> {
        match s {
            "steel" => Some(FrameMaterial::Steel),
            "timber" => Some(FrameMaterial::Timber),
            _ => None,
        }
    }

    /// Density, kg/m³.
    pub fn density(self) -> f64 {
        match self {
            FrameMaterial::Steel => 7850.0,
            FrameMaterial::Timber => 420.0,
        }
    }
}

/// A section of the frame: a steel C (with lips) or U (`lip` 0), or a
/// timber rectangle (`thickness` 0). `depth` is across the wall (the
/// web), `width` along it (the flange).
#[derive(Debug, Clone, PartialEq)]
pub struct FrameSection {
    pub name: String,
    pub depth: f64,
    pub width: f64,
    pub lip: f64,
    pub thickness: f64,
}

impl FrameSection {
    /// A steel C (`lip` > 0) or U.
    pub fn steel(depth_mm: f64, width_mm: f64, lip_mm: f64, t_mm: f64) -> Self {
        let kind = if lip_mm > 0.0 { "C" } else { "U" };
        let name = if lip_mm > 0.0 {
            format!("{kind} {depth_mm:.0}×{width_mm:.0}×{lip_mm:.0}×{t_mm}")
        } else {
            format!("{kind} {depth_mm:.0}×{width_mm:.0}×{t_mm}")
        };
        FrameSection {
            name: name.replace('.', ","),
            depth: depth_mm / 1000.0,
            width: width_mm / 1000.0,
            lip: lip_mm / 1000.0,
            thickness: t_mm / 1000.0,
        }
    }

    /// A timber section.
    pub fn timber(width_mm: f64, depth_mm: f64) -> Self {
        FrameSection {
            name: format!("{width_mm:.0}×{depth_mm:.0} C24"),
            depth: depth_mm / 1000.0,
            width: width_mm / 1000.0,
            lip: 0.0,
            thickness: 0.0,
        }
    }

    pub fn is_steel(&self) -> bool {
        self.thickness > 0.0
    }

    /// The area of the section, m².
    pub fn area(&self) -> f64 {
        if self.is_steel() {
            (self.depth + 2.0 * self.width + 2.0 * self.lip) * self.thickness
        } else {
            self.depth * self.width
        }
    }

    /// Its mass per metre, kg.
    pub fn kg_per_m(&self) -> f64 {
        self.area()
            * if self.is_steel() {
                FrameMaterial::Steel.density()
            } else {
                FrameMaterial::Timber.density()
            }
    }

    pub fn is_valid(&self) -> bool {
        self.depth > 0.0 && self.width > 0.0 && self.lip >= 0.0 && self.thickness >= 0.0
    }
}

/// The usual studs (C) and tracks (U) of light steel frame, mm.
pub fn steel_studs() -> Vec<FrameSection> {
    let mut v = Vec::new();
    for (d, w, l) in [
        (90.0, 40.0, 12.0),
        (100.0, 40.0, 12.0),
        (150.0, 40.0, 15.0),
        (200.0, 40.0, 15.0),
        (250.0, 50.0, 15.0),
    ] {
        for t in [0.75, 1.0, 1.2, 1.5, 2.0] {
            v.push(FrameSection::steel(d, w, l, t));
        }
    }
    v
}

pub fn steel_tracks() -> Vec<FrameSection> {
    let mut v = Vec::new();
    for (d, w) in [
        (92.0, 35.0),
        (102.0, 35.0),
        (152.0, 35.0),
        (202.0, 35.0),
        (252.0, 40.0),
    ] {
        for t in [0.75, 1.0, 1.2, 1.5, 2.0] {
            v.push(FrameSection::steel(d, w, 0.0, t));
        }
    }
    v
}

/// The usual timber sections, mm.
pub fn timber_sections() -> Vec<FrameSection> {
    [
        (45.0, 70.0),
        (45.0, 95.0),
        (45.0, 120.0),
        (45.0, 145.0),
        (45.0, 170.0),
        (45.0, 195.0),
        (45.0, 220.0),
        (45.0, 245.0),
        (70.0, 220.0),
    ]
    .into_iter()
    .map(|(w, d)| FrameSection::timber(w, d))
    .collect()
}

/// Reads a catalogue of sections, one a line, fields separated by `;` (or
/// tabs or commas): name; material (`acero`/`steel` or `madera`/`timber`);
/// depth; flange width; lip (0 for a U); thickness — all in millimetres
/// (lip and thickness are left out, or 0, for timber). Lines starting
/// with `#` and a header line that is not a section are skipped. This is
/// how a supplier's catalogue enters the project: as a file of the
/// studio's library, never inside the program.
pub fn parse_section_catalogue(text: &str) -> Vec<(FrameMaterial, FrameSection)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let sep = if line.contains(';') {
            ';'
        } else if line.contains('\t') {
            '\t'
        } else {
            ','
        };
        let f: Vec<&str> = line.split(sep).map(|x| x.trim()).collect();
        if f.len() < 4 {
            continue;
        }
        let material = match f[1].to_lowercase().as_str() {
            "acero" | "steel" | "lsf" => FrameMaterial::Steel,
            "madera" | "timber" | "wood" => FrameMaterial::Timber,
            _ => continue,
        };
        let num = |i: usize| {
            f.get(i)
                .and_then(|x| x.replace(',', ".").parse::<f64>().ok())
                .unwrap_or(0.0)
        };
        let (d, w) = (num(2), num(3));
        if d <= 0.0 || w <= 0.0 || f[0].is_empty() {
            continue;
        }
        let mut section = match material {
            FrameMaterial::Steel => {
                let t = num(5);
                if t <= 0.0 {
                    continue;
                }
                FrameSection::steel(d, w, num(4), t)
            }
            FrameMaterial::Timber => FrameSection::timber(w, d),
        };
        section.name = f[0].to_string();
        out.push((material, section));
    }
    out
}

/// The boards over the frame (OSB, plywood, gypsum).
#[derive(Debug, Clone, PartialEq)]
pub struct Sheathing {
    pub name: String,
    pub thickness: f64,
    /// The size of a board, m.
    pub board: (f64, f64),
}

impl Sheathing {
    pub fn osb() -> Self {
        Sheathing {
            name: "OSB/3 12 mm".into(),
            thickness: 0.012,
            board: (2.44, 1.22),
        }
    }
}

/// How a wall type or a slab type is framed.
#[derive(Debug, Clone, PartialEq)]
pub struct Framing {
    pub material: FrameMaterial,
    /// The studs of a wall, the joists of a floor, the rafters of a roof.
    pub stud: FrameSection,
    /// The tracks or plates of a wall, the rim joists of a floor.
    pub track: FrameSection,
    /// Centre to centre, m.
    pub spacing: f64,
    /// Rows of blocking between the studs (noggins) or the joists.
    pub noggins: bool,
    pub sheathing: Option<Sheathing>,
}

impl Framing {
    /// Light steel frame: C 90 studs and U 92 tracks of 1 mm at 600.
    pub fn steel() -> Self {
        Framing {
            material: FrameMaterial::Steel,
            stud: FrameSection::steel(90.0, 40.0, 12.0, 1.0),
            track: FrameSection::steel(92.0, 35.0, 0.0, 1.0),
            spacing: 0.6,
            noggins: true,
            sheathing: Some(Sheathing::osb()),
        }
    }

    /// Timber frame: 45 × 145 C24 at 600.
    pub fn timber() -> Self {
        Framing {
            material: FrameMaterial::Timber,
            stud: FrameSection::timber(45.0, 145.0),
            track: FrameSection::timber(45.0, 145.0),
            spacing: 0.6,
            noggins: true,
            sheathing: Some(Sheathing::osb()),
        }
    }

    /// The usual framing of `material` for a structure layer `depth` deep
    /// (a floor's joists deeper than a wall's studs).
    pub fn for_depth(material: FrameMaterial, depth: f64) -> Self {
        let mut f = match material {
            FrameMaterial::Steel => Framing::steel(),
            FrameMaterial::Timber => Framing::timber(),
        };
        let fit = |list: Vec<FrameSection>| {
            list.into_iter()
                .filter(|s| s.depth <= depth + 0.006)
                .max_by(|a, b| a.depth.total_cmp(&b.depth))
        };
        match material {
            FrameMaterial::Steel => {
                if let Some(s) = fit(steel_studs()
                    .into_iter()
                    .filter(|s| (s.thickness - 0.001).abs() < 1e-6 || s.depth > 0.14)
                    .collect())
                {
                    let t = if s.depth > 0.14 { 0.0015 } else { 0.001 };
                    f.stud = steel_studs()
                        .into_iter()
                        .find(|x| x.depth == s.depth && (x.thickness - t).abs() < 1e-6)
                        .unwrap_or(s);
                    let stud = f.stud.clone();
                    f.track = steel_tracks()
                        .into_iter()
                        .filter(|x| {
                            x.depth >= stud.depth && (x.thickness - stud.thickness).abs() < 1e-6
                        })
                        .min_by(|a, b| a.depth.total_cmp(&b.depth))
                        .unwrap_or(f.track);
                }
            }
            FrameMaterial::Timber => {
                if let Some(s) = fit(timber_sections()) {
                    f.stud = s.clone();
                    f.track = s;
                }
            }
        }
        f
    }

    pub fn is_valid(&self) -> bool {
        self.stud.is_valid()
            && self.track.is_valid()
            && self.spacing >= 0.2
            && self.spacing <= 1.5
            && self
                .sheathing
                .as_ref()
                .is_none_or(|s| s.thickness > 0.0 && s.board.0 > 0.0 && s.board.1 > 0.0)
    }
}

impl Model {
    /// Whether `id` is a wall type or a slab type (what can be framed).
    fn framable(&self, id: u64) -> bool {
        self.wall_type(WallTypeId(id)).is_some() || self.slab_type(SlabTypeId(id)).is_some()
    }

    /// The framing of a wall type or slab type, if it has one.
    pub fn type_framing(&self, id: u64) -> Option<&Framing> {
        self.framings.get(&id).filter(|_| self.framable(id))
    }

    /// The framed types and their framing.
    pub fn framings(&self) -> impl Iterator<Item = (u64, &Framing)> {
        self.framings
            .iter()
            .filter(|(id, _)| self.framable(**id))
            .map(|(id, f)| (*id, f))
    }

    /// Frames a wall type or slab type (or takes its framing away, `None`).
    pub fn set_type_framing(&mut self, id: u64, f: Option<Framing>) -> Result<(), ModelError> {
        match f {
            None => {
                self.framings.remove(&id);
            }
            Some(f) => {
                if !self.framable(id) {
                    return Err(ModelError::UnknownElement(id));
                }
                if !f.is_valid() {
                    return Err(ModelError::NonPositive("framing"));
                }
                self.framings.insert(id, f);
            }
        }
        Ok(())
    }

    /// The material its members are drawn with: «Acero galvanizado» or
    /// «Madera C24» when the project has them (the library adds them with
    /// `archisim_library::frame_material`), or else any steel or timber.
    pub fn frame_member_material(&self, kind: FrameMaterial) -> Option<MaterialId> {
        let (name, cats): (&str, &[&str]) = match kind {
            FrameMaterial::Steel => (FRAME_STEEL, &["steel", "metal"]),
            FrameMaterial::Timber => (FRAME_TIMBER, &["timber", "wood"]),
        };
        self.materials()
            .find(|m| m.name == name)
            .or_else(|| {
                self.materials()
                    .find(|m| cats.contains(&m.category.as_str()))
            })
            .map(|m| m.id)
    }

    /// The index of the layer that the frame fills in a type's layers: its
    /// first structure layer, or else its thickest.
    pub fn frame_layer(layers: &[Layer]) -> Option<usize> {
        layers
            .iter()
            .position(|l| l.function == LayerFunction::Structure)
            .or_else(|| {
                layers
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.thickness.total_cmp(&b.1.thickness))
                    .map(|(i, _)| i)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_catalogue_is_read_from_text() {
        let text = "# perfiles del proveedor\nnombre;material;canto;ala;pestaña;espesor\nC-90-10;acero;90;40;12;1,0\nU-92;steel;92;35;0;1\nPino 45x145;madera;145;45\nmal;acero;90;40\n";
        let list = parse_section_catalogue(text);
        assert_eq!(list.len(), 3, "{list:?}");
        assert_eq!(list[0].1.name, "C-90-10");
        assert!((list[0].1.thickness - 0.001).abs() < 1e-9 && list[0].1.lip > 0.0);
        assert_eq!(list[1].1.lip, 0.0);
        assert_eq!(list[2].0, FrameMaterial::Timber);
        assert!((list[2].1.depth - 0.145).abs() < 1e-9 && (list[2].1.width - 0.045).abs() < 1e-9);
    }

    #[test]
    fn sections_weigh_and_framings_are_kept() {
        let c = FrameSection::steel(90.0, 40.0, 12.0, 1.0);
        assert_eq!(c.name, "C 90×40×12×1");
        // (90 + 80 + 24) mm × 1 mm × 7850 kg/m³ ≈ 1,52 kg/m.
        assert!((c.kg_per_m() - 1.523).abs() < 0.01, "{}", c.kg_per_m());
        let t = FrameSection::timber(45.0, 145.0);
        assert!((t.area() - 0.006525).abs() < 1e-9);
        let f = Framing::for_depth(FrameMaterial::Steel, 0.15);
        assert!((f.stud.depth - 0.15).abs() < 1e-9 && f.track.depth >= f.stud.depth);
        let w = Framing::for_depth(FrameMaterial::Timber, 0.2);
        assert!((w.stud.depth - 0.195).abs() < 1e-9);
        let mut m = Model::new("T");
        let wt = m.wall_types().next().map(|t| t.id);
        let wt = wt.unwrap_or_else(|| {
            let mat = m.add_material("Hormigón", "concrete", [180, 180, 180], CutPattern::Solid);
            m.add_wall_type(
                "Muro",
                WallFunction::Exterior,
                vec![Layer {
                    material: mat,
                    thickness: 0.15,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap()
        });
        m.set_type_framing(wt.0, Some(Framing::steel())).unwrap();
        assert!(m.type_framing(wt.0).is_some());
        assert!(m.set_type_framing(999_999, Some(Framing::steel())).is_err());
        let mut bad = Framing::steel();
        bad.spacing = 0.0;
        assert!(m.set_type_framing(wt.0, Some(bad)).is_err());
        m.set_type_framing(wt.0, None).unwrap();
        assert_eq!(m.framings().count(), 0);
    }
}
