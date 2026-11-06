//! The structural actions of the project (CTE DB-SE-AE and NCSE-02): the
//! site data that the snow, the wind and the earthquake come from, and the
//! loads placed by hand (a machine, a water tank, a planter). The weights
//! of the elements and the live loads of the rooms are not stored: they
//! come from the model (`archisim-struct`).

use super::*;

/// The wind zone of figure D.1 of DB-SE-AE.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindZone {
    #[default]
    A,
    B,
    C,
}

impl WindZone {
    pub const ALL: [WindZone; 3] = [WindZone::A, WindZone::B, WindZone::C];

    /// The basic dynamic pressure qb (kN/m², DB-SE-AE D.1).
    pub fn basic_pressure(self) -> f64 {
        match self {
            WindZone::A => 0.42,
            WindZone::B => 0.45,
            WindZone::C => 0.52,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            WindZone::A => "A",
            WindZone::B => "B",
            WindZone::C => "C",
        }
    }

    pub fn from_code(s: &str) -> Option<WindZone> {
        WindZone::ALL.into_iter().find(|z| z.code() == s)
    }
}

/// The importance of the building for the earthquake (NCSE-02, 1.2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Importance {
    /// Of little importance (sheds): the norm does not apply.
    Moderate,
    /// Dwellings, offices, schools… (ρ 1,0).
    #[default]
    Normal,
    /// Hospitals, fire stations, buildings that must keep working (ρ 1,3).
    Special,
}

impl Importance {
    pub const ALL: [Importance; 3] = [
        Importance::Moderate,
        Importance::Normal,
        Importance::Special,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Importance::Moderate => "moderate",
            Importance::Normal => "normal",
            Importance::Special => "special",
        }
    }

    pub fn from_code(s: &str) -> Option<Importance> {
        Importance::ALL.into_iter().find(|i| i.code() == s)
    }

    pub fn name(self) -> &'static str {
        match self {
            Importance::Moderate => "Moderada",
            Importance::Normal => "Normal",
            Importance::Special => "Especial",
        }
    }
}

/// Strength classes of timber (EN 338, EN 14080).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimberClass {
    C18,
    #[default]
    C24,
    C30,
    GL24h,
    GL28h,
    GL32h,
}

impl TimberClass {
    pub const ALL: [TimberClass; 6] = [
        TimberClass::C18,
        TimberClass::C24,
        TimberClass::C30,
        TimberClass::GL24h,
        TimberClass::GL28h,
        TimberClass::GL32h,
    ];

    pub fn code(self) -> &'static str {
        match self {
            TimberClass::C18 => "C18",
            TimberClass::C24 => "C24",
            TimberClass::C30 => "C30",
            TimberClass::GL24h => "GL24h",
            TimberClass::GL28h => "GL28h",
            TimberClass::GL32h => "GL32h",
        }
    }

    pub fn from_code(s: &str) -> Option<TimberClass> {
        TimberClass::ALL.into_iter().find(|t| t.code() == s)
    }

    /// Glued laminated (γM 1,25 and kh up to 1,1).
    pub fn glulam(self) -> bool {
        matches!(
            self,
            TimberClass::GL24h | TimberClass::GL28h | TimberClass::GL32h
        )
    }
}

/// The materials of the structure, for the checks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StructuralMaterials {
    /// Concrete fck, N/mm² (HA-25 … HA-50).
    pub concrete_fck: f64,
    /// Reinforcing steel fyk, N/mm² (B400S, B500S).
    pub rebar_fyk: f64,
    /// Structural steel fy, N/mm² (S235, S275, S355).
    pub steel_fy: f64,
    pub timber: TimberClass,
    /// Masonry compressive strength fk, N/mm² (DB-SE-F, table 4.4).
    pub masonry_fk: f64,
    /// The ends of the steel beams are rigid (end plates bolted to the
    /// columns) rather than hinged (fin plates).
    pub steel_rigid: bool,
}

impl Default for StructuralMaterials {
    fn default() -> Self {
        StructuralMaterials {
            concrete_fck: 25.0,
            rebar_fyk: 500.0,
            steel_fy: 275.0,
            timber: TimberClass::C24,
            masonry_fk: 4.0,
            steel_rigid: false,
        }
    }
}

impl StructuralMaterials {
    pub const CONCRETES: [f64; 6] = [25.0, 30.0, 35.0, 40.0, 45.0, 50.0];
    pub const REBARS: [f64; 2] = [400.0, 500.0];
    pub const STEELS: [f64; 3] = [235.0, 275.0, 355.0];

    pub fn is_valid(&self) -> bool {
        (20.0..=90.0).contains(&self.concrete_fck)
            && (400.0..=600.0).contains(&self.rebar_fyk)
            && (200.0..=460.0).contains(&self.steel_fy)
            && self.masonry_fk.is_finite()
            && (0.5..=20.0).contains(&self.masonry_fk)
    }
}

/// Where the building is, for its actions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StructuralSite {
    /// Winter climate zone of figure E.2 (1 to 7).
    pub snow_zone: u8,
    /// Altitude of the site, m; `None`: the height of the georeference.
    pub altitude: Option<f64>,
    pub wind_zone: WindZone,
    /// Roughness of the surroundings (table D.2): 1 sea shore … 5 city
    /// centre with tall buildings.
    pub roughness: u8,
    /// Basic seismic acceleration ab / g (NCSE-02, annex 1); 0: none.
    pub seismic_ab: f64,
    /// Contribution coefficient K (NCSE-02, annex 1).
    pub seismic_k: f64,
    /// Ground coefficient C (NCSE-02, table 2.1): 1,0 rock … 2,0 soft
    /// soil.
    pub soil: f64,
    pub importance: Importance,
    /// Admissible bearing pressure of the ground, kN/m² (DB-SE-C), for
    /// the footings.
    pub bearing: f64,
    /// Angle of internal friction of the ground, degrees, and its weight,
    /// kN/m³: the earth pressure on basement walls.
    pub soil_friction: f64,
    pub soil_weight: f64,
    pub materials: StructuralMaterials,
    /// A single-family house (vivienda unifamiliar): the fire resistance
    /// of its structure is R 30 (DB-SI 6, table 3.1).
    pub single_house: bool,
}

impl Default for StructuralSite {
    fn default() -> Self {
        StructuralSite {
            snow_zone: 6,
            altitude: None,
            wind_zone: WindZone::A,
            roughness: 4,
            seismic_ab: 0.0,
            seismic_k: 1.0,
            soil: 1.3,
            importance: Importance::Normal,
            bearing: 200.0,
            soil_friction: 30.0,
            soil_weight: 18.0,
            materials: StructuralMaterials::default(),
            single_house: false,
        }
    }
}

impl StructuralSite {
    pub fn is_valid(&self) -> bool {
        (1..=7).contains(&self.snow_zone)
            && (1..=5).contains(&self.roughness)
            && self.altitude.is_none_or(|a| a.is_finite() && a > -500.0)
            && self.seismic_ab.is_finite()
            && (0.0..=0.5).contains(&self.seismic_ab)
            && self.seismic_k.is_finite()
            && (1.0..=1.5).contains(&self.seismic_k)
            && self.soil.is_finite()
            && (1.0..=2.0).contains(&self.soil)
            && self.bearing.is_finite()
            && self.bearing > 0.0
            && (15.0..=45.0).contains(&self.soil_friction)
            && (12.0..=24.0).contains(&self.soil_weight)
            && self.materials.is_valid()
    }
}

/// Which action a load placed by hand belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadCase {
    /// Permanent (G): weights that are always there.
    Dead,
    /// Live load of use (Q).
    Live,
    Snow,
    Wind,
}

impl LoadCase {
    pub const ALL: [LoadCase; 4] = [
        LoadCase::Dead,
        LoadCase::Live,
        LoadCase::Snow,
        LoadCase::Wind,
    ];

    pub fn code(self) -> &'static str {
        match self {
            LoadCase::Dead => "dead",
            LoadCase::Live => "live",
            LoadCase::Snow => "snow",
            LoadCase::Wind => "wind",
        }
    }

    pub fn from_code(s: &str) -> Option<LoadCase> {
        LoadCase::ALL.into_iter().find(|c| c.code() == s)
    }

    pub fn name(self) -> &'static str {
        match self {
            LoadCase::Dead => "Permanente",
            LoadCase::Live => "Sobrecarga de uso",
            LoadCase::Snow => "Nieve",
            LoadCase::Wind => "Viento",
        }
    }
}

/// Where a load placed by hand acts, in plan.
#[derive(Debug, Clone, PartialEq)]
pub enum LoadShape {
    /// A point load, kN.
    Point { at: Point2 },
    /// A line load, kN/m.
    Line { start: Point2, end: Point2 },
    /// A load spread over an area, kN/m²; counter-clockwise outline.
    Area { outline: Vec<Point2> },
}

impl LoadShape {
    /// The units of its value.
    pub fn unit(&self) -> &'static str {
        match self {
            LoadShape::Point { .. } => "kN",
            LoadShape::Line { .. } => "kN/m",
            LoadShape::Area { .. } => "kN/m²",
        }
    }

    pub fn points(&self) -> Vec<Point2> {
        match self {
            LoadShape::Point { at } => vec![*at],
            LoadShape::Line { start, end } => vec![*start, *end],
            LoadShape::Area { outline } => outline.clone(),
        }
    }
}

/// A load placed by hand: vertical, downwards when positive; a wind load
/// pushes horizontally along `direction`.
#[derive(Debug, Clone, PartialEq)]
pub struct StructuralLoad {
    pub id: u64,
    pub name: String,
    pub level: LevelId,
    /// Height of its point of application above the level, m.
    pub height: f64,
    pub shape: LoadShape,
    pub case: LoadCase,
    /// kN, kN/m or kN/m², by its shape.
    pub value: f64,
    /// For a horizontal load (wind, a thrust): its direction in plan;
    /// `None`: vertical.
    pub direction: Option<Vec2>,
}

impl Model {
    pub fn structural_site(&self) -> &StructuralSite {
        &self.structural_site
    }

    pub fn set_structural_site(&mut self, site: StructuralSite) -> Result<(), ModelError> {
        if !site.is_valid() {
            return Err(ModelError::NotFinite("structural site"));
        }
        self.structural_site = site;
        Ok(())
    }

    /// The altitude of the site: its own, or the georeference's height.
    pub fn site_altitude(&self) -> f64 {
        self.structural_site
            .altitude
            .or_else(|| self.georeference.as_ref().map(|g| g.height))
            .unwrap_or(0.0)
    }

    pub fn structural_loads(&self) -> impl Iterator<Item = &StructuralLoad> {
        self.structural_loads.values()
    }

    pub fn structural_load(&self, id: u64) -> Option<&StructuralLoad> {
        self.structural_loads.get(&id)
    }

    fn check_load(&self, l: &StructuralLoad) -> Result<(), ModelError> {
        if self.level(l.level).is_none() {
            return Err(ModelError::UnknownElement(l.level.0));
        }
        if !l.value.is_finite() || !l.height.is_finite() {
            return Err(ModelError::NotFinite("structural load"));
        }
        let pts = l.shape.points();
        if pts.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
            return Err(ModelError::NotFinite("structural load"));
        }
        match &l.shape {
            LoadShape::Line { start, end } if start.distance(*end) < 1e-6 => {
                Err(ModelError::NonPositive("load length"))
            }
            LoadShape::Area { outline } if outline.len() < 3 => {
                Err(ModelError::NonPositive("load area"))
            }
            _ => Ok(()),
        }?;
        if l.direction
            .is_some_and(|d| !d.x.is_finite() || !d.y.is_finite() || d.length() < 1e-9)
        {
            return Err(ModelError::NotFinite("load direction"));
        }
        Ok(())
    }

    /// Adds a load; an empty name becomes «Carga n».
    pub fn add_structural_load(&mut self, mut l: StructuralLoad) -> Result<u64, ModelError> {
        if l.name.trim().is_empty() {
            let taken = |n: &str| self.structural_loads.values().any(|k| k.name == n);
            l.name = (1..)
                .map(|i| format!("Carga {i}"))
                .find(|n| !taken(n))
                .unwrap_or_default();
        }
        self.check_load(&l)?;
        l.id = self.take_id();
        let id = l.id;
        self.structural_loads.insert(id, l);
        Ok(id)
    }

    pub fn edit_structural_load(
        &mut self,
        id: u64,
        change: impl FnOnce(&mut StructuralLoad),
    ) -> Result<(), ModelError> {
        let mut l = self
            .structural_loads
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id))?;
        change(&mut l);
        l.id = id;
        self.check_load(&l)?;
        self.structural_loads.insert(id, l);
        Ok(())
    }

    pub fn remove_structural_load(&mut self, id: u64) -> Result<StructuralLoad, ModelError> {
        self.structural_loads
            .remove(&id)
            .ok_or(ModelError::UnknownElement(id))
    }

    pub(super) fn restore_structural_loads(
        &mut self,
        site: StructuralSite,
        loads: Vec<StructuralLoad>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        self.set_structural_site(site)
            .map_err(|error| RestoreError {
                element: "structural site".into(),
                error,
            })?;
        for l in loads {
            claim(l.id, None)
                .and_then(|_| self.check_load(&l))
                .map_err(|error| RestoreError {
                    element: format!("structural load {}", l.id),
                    error,
                })?;
            self.structural_loads.insert(l.id, l);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_and_site_are_kept() {
        let mut m = Model::new("Cargas");
        let level = m.add_level("Planta baja", 0.0);
        let id = m
            .add_structural_load(StructuralLoad {
                id: 0,
                name: String::new(),
                level,
                height: 0.0,
                shape: LoadShape::Point {
                    at: Point2::new(1.0, 2.0),
                },
                case: LoadCase::Live,
                value: 5.0,
                direction: None,
            })
            .unwrap();
        assert_eq!(m.structural_load(id).unwrap().name, "Carga 1");
        assert!(m.edit_structural_load(id, |l| l.value = f64::NAN).is_err());
        let mut site = *m.structural_site();
        site.snow_zone = 9;
        assert!(m.set_structural_site(site).is_err());
        site.snow_zone = 3;
        site.altitude = Some(650.0);
        m.set_structural_site(site).unwrap();
        assert_eq!(m.site_altitude(), 650.0);
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back.structural_site(), m.structural_site());
        assert_eq!(back.structural_load(id), m.structural_load(id));
    }
}
