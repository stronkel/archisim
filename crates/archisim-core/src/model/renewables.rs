//! The data of the renewable energy checks besides the drawing (DB-HE 4
//! and HE 5, format version 93): where the building is (its province, for
//! the cold water of the Anejo G), what makes its hot water renewable and
//! how much, the panels' power, and whether the sections apply.

use super::{Model, ModelError};

/// What covers the renewable part of the hot water (HE 4, 3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AcsSource {
    /// A heat pump (aerothermal, geothermal): its renewable part is
    /// 1 − 1/SCOP of the heat it gives.
    #[default]
    HeatPump,
    /// Solar thermal collectors: the solar fraction of the annual demand,
    /// from their calculation.
    Solar,
    /// Another source (biomass, a district network, waste heat): its
    /// renewable fraction, given.
    Other,
    /// Nothing renewable.
    None,
}

impl AcsSource {
    pub const ALL: [AcsSource; 4] = [
        AcsSource::HeatPump,
        AcsSource::Solar,
        AcsSource::Other,
        AcsSource::None,
    ];

    pub fn code(self) -> &'static str {
        match self {
            AcsSource::HeatPump => "heat_pump",
            AcsSource::Solar => "solar",
            AcsSource::Other => "other",
            AcsSource::None => "none",
        }
    }

    pub fn from_code(s: &str) -> Option<AcsSource> {
        AcsSource::ALL.into_iter().find(|a| a.code() == s)
    }

    pub fn name(self) -> &'static str {
        match self {
            AcsSource::HeatPump => "Bomba de calor (aerotermia)",
            AcsSource::Solar => "Solar térmica",
            AcsSource::Other => "Otra renovable",
            AcsSource::None => "Ninguna",
        }
    }
}

/// The renewables' data.
#[derive(Debug, Clone, PartialEq)]
pub struct RenewablesSite {
    /// A new building, or one renovated whole (the sections apply).
    pub new_building: bool,
    /// Province, by its name in the Anejo G (its capital's cold water).
    pub province: String,
    /// The site above its provincial capital, metres (Anejo G correction).
    pub above_capital: f64,
    pub acs_source: AcsSource,
    /// The heat pump's seasonal performance for hot water.
    pub scop: f64,
    /// The solar or other source's renewable fraction (0 to 1).
    pub fraction: f64,
    /// Temperature of the hot water store, °C.
    pub storage_temp: f64,
    /// Peak power of the panels per m² of panel, kW (a panel of 400 W
    /// and 2 m²: 0,2).
    pub panel_kwp_m2: f64,
    /// The built area, m², where the one worked out of the levels is not
    /// right.
    pub built_area: Option<f64>,
    /// The daily hot water at 60 °C, litres, where the one worked out of
    /// the dwellings is not right (other uses, Anejo F table c).
    pub acs_demand: Option<f64>,
    /// The hot water store, litres, where it is not one day's demand.
    pub tank_volume: Option<f64>,
}

impl Default for RenewablesSite {
    fn default() -> Self {
        RenewablesSite {
            new_building: true,
            province: "Huelva".into(),
            above_capital: 0.0,
            acs_source: AcsSource::HeatPump,
            scop: 3.0,
            fraction: 0.6,
            storage_temp: 60.0,
            panel_kwp_m2: 0.2,
            built_area: None,
            acs_demand: None,
            tank_volume: None,
        }
    }
}

impl RenewablesSite {
    pub fn is_valid(&self) -> bool {
        let fin = [
            self.above_capital,
            self.scop,
            self.fraction,
            self.storage_temp,
            self.panel_kwp_m2,
        ]
        .iter()
        .all(|v| v.is_finite());
        fin && self.scop > 0.0
            && (0.0..=1.0).contains(&self.fraction)
            && self.storage_temp > 20.0
            && self.panel_kwp_m2 > 0.0
            && self.built_area.is_none_or(|a| a.is_finite() && a >= 0.0)
            && self.acs_demand.is_none_or(|a| a.is_finite() && a >= 0.0)
            && self.tank_volume.is_none_or(|a| a.is_finite() && a >= 0.0)
    }
}

/// Table a-Anejo G: the cold water of each provincial capital, °C, January
/// to December.
pub const COLD_WATER: [(&str, [f64; 12]); 52] = [
    (
        "A Coruña",
        [
            10.0, 10.0, 11.0, 12.0, 13.0, 14.0, 16.0, 16.0, 15.0, 14.0, 12.0, 11.0,
        ],
    ),
    (
        "Albacete",
        [
            7.0, 8.0, 9.0, 11.0, 14.0, 17.0, 19.0, 19.0, 17.0, 13.0, 9.0, 7.0,
        ],
    ),
    (
        "Alicante",
        [
            11.0, 12.0, 13.0, 14.0, 16.0, 18.0, 20.0, 20.0, 19.0, 16.0, 13.0, 12.0,
        ],
    ),
    (
        "Almería",
        [
            12.0, 12.0, 13.0, 14.0, 16.0, 18.0, 20.0, 21.0, 19.0, 17.0, 14.0, 12.0,
        ],
    ),
    (
        "Ávila",
        [
            6.0, 6.0, 7.0, 9.0, 11.0, 14.0, 17.0, 16.0, 14.0, 11.0, 8.0, 6.0,
        ],
    ),
    (
        "Badajoz",
        [
            9.0, 10.0, 11.0, 13.0, 15.0, 18.0, 20.0, 20.0, 18.0, 15.0, 12.0, 9.0,
        ],
    ),
    (
        "Barcelona",
        [
            9.0, 10.0, 11.0, 12.0, 14.0, 17.0, 19.0, 19.0, 17.0, 15.0, 12.0, 10.0,
        ],
    ),
    (
        "Vizcaya",
        [
            9.0, 10.0, 10.0, 11.0, 13.0, 15.0, 17.0, 17.0, 16.0, 14.0, 11.0, 10.0,
        ],
    ),
    (
        "Burgos",
        [
            5.0, 6.0, 7.0, 9.0, 11.0, 13.0, 16.0, 16.0, 14.0, 11.0, 7.0, 6.0,
        ],
    ),
    (
        "Cáceres",
        [
            9.0, 10.0, 11.0, 12.0, 14.0, 18.0, 21.0, 20.0, 19.0, 15.0, 11.0, 9.0,
        ],
    ),
    (
        "Cádiz",
        [
            12.0, 12.0, 13.0, 14.0, 16.0, 18.0, 19.0, 20.0, 19.0, 17.0, 14.0, 12.0,
        ],
    ),
    (
        "Castellón",
        [
            10.0, 11.0, 12.0, 13.0, 15.0, 18.0, 19.0, 20.0, 18.0, 16.0, 12.0, 11.0,
        ],
    ),
    (
        "Ceuta",
        [
            11.0, 11.0, 12.0, 13.0, 14.0, 16.0, 18.0, 18.0, 17.0, 15.0, 13.0, 12.0,
        ],
    ),
    (
        "Ciudad Real",
        [
            7.0, 8.0, 10.0, 11.0, 14.0, 17.0, 20.0, 20.0, 17.0, 13.0, 10.0, 7.0,
        ],
    ),
    (
        "Córdoba",
        [
            10.0, 11.0, 12.0, 14.0, 16.0, 19.0, 21.0, 21.0, 19.0, 16.0, 12.0, 10.0,
        ],
    ),
    (
        "Cuenca",
        [
            6.0, 7.0, 8.0, 10.0, 13.0, 16.0, 18.0, 18.0, 16.0, 12.0, 9.0, 7.0,
        ],
    ),
    (
        "Girona",
        [
            8.0, 9.0, 10.0, 11.0, 14.0, 16.0, 19.0, 18.0, 17.0, 14.0, 10.0, 9.0,
        ],
    ),
    (
        "Granada",
        [
            8.0, 9.0, 10.0, 12.0, 14.0, 17.0, 20.0, 19.0, 17.0, 14.0, 11.0, 8.0,
        ],
    ),
    (
        "Guadalajara",
        [
            7.0, 8.0, 9.0, 11.0, 14.0, 17.0, 19.0, 19.0, 16.0, 13.0, 9.0, 7.0,
        ],
    ),
    (
        "Huelva",
        [
            12.0, 12.0, 13.0, 14.0, 16.0, 18.0, 20.0, 20.0, 19.0, 17.0, 14.0, 12.0,
        ],
    ),
    (
        "Huesca",
        [
            7.0, 8.0, 10.0, 11.0, 14.0, 16.0, 19.0, 18.0, 17.0, 13.0, 9.0, 7.0,
        ],
    ),
    (
        "Jaén",
        [
            9.0, 10.0, 11.0, 13.0, 16.0, 19.0, 21.0, 21.0, 19.0, 15.0, 12.0, 9.0,
        ],
    ),
    (
        "Las Palmas",
        [
            15.0, 15.0, 16.0, 16.0, 17.0, 18.0, 19.0, 19.0, 19.0, 18.0, 17.0, 16.0,
        ],
    ),
    (
        "León",
        [
            6.0, 6.0, 8.0, 9.0, 12.0, 14.0, 16.0, 16.0, 15.0, 11.0, 8.0, 6.0,
        ],
    ),
    (
        "Lleida",
        [
            7.0, 9.0, 10.0, 12.0, 15.0, 17.0, 20.0, 19.0, 17.0, 14.0, 10.0, 7.0,
        ],
    ),
    (
        "La Rioja",
        [
            7.0, 8.0, 10.0, 11.0, 13.0, 16.0, 18.0, 18.0, 16.0, 13.0, 10.0, 8.0,
        ],
    ),
    (
        "Lugo",
        [
            7.0, 8.0, 9.0, 10.0, 11.0, 13.0, 15.0, 15.0, 14.0, 12.0, 9.0, 8.0,
        ],
    ),
    (
        "Madrid",
        [
            8.0, 8.0, 10.0, 12.0, 14.0, 17.0, 20.0, 19.0, 17.0, 13.0, 10.0, 8.0,
        ],
    ),
    (
        "Málaga",
        [
            12.0, 12.0, 13.0, 14.0, 16.0, 18.0, 20.0, 20.0, 19.0, 16.0, 14.0, 12.0,
        ],
    ),
    (
        "Melilla",
        [
            12.0, 13.0, 13.0, 14.0, 16.0, 18.0, 20.0, 20.0, 19.0, 17.0, 14.0, 13.0,
        ],
    ),
    (
        "Murcia",
        [
            11.0, 11.0, 12.0, 13.0, 15.0, 17.0, 19.0, 20.0, 18.0, 16.0, 13.0, 11.0,
        ],
    ),
    (
        "Ourense",
        [
            8.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 18.0, 17.0, 13.0, 11.0, 9.0,
        ],
    ),
    (
        "Asturias",
        [
            9.0, 9.0, 10.0, 10.0, 12.0, 14.0, 15.0, 16.0, 15.0, 13.0, 10.0, 9.0,
        ],
    ),
    (
        "Palencia",
        [
            6.0, 7.0, 8.0, 10.0, 12.0, 15.0, 17.0, 17.0, 15.0, 12.0, 9.0, 6.0,
        ],
    ),
    (
        "Baleares",
        [
            11.0, 11.0, 12.0, 13.0, 15.0, 18.0, 20.0, 20.0, 19.0, 17.0, 14.0, 12.0,
        ],
    ),
    (
        "Navarra",
        [
            7.0, 8.0, 9.0, 10.0, 12.0, 15.0, 17.0, 17.0, 16.0, 13.0, 9.0, 7.0,
        ],
    ),
    (
        "Pontevedra",
        [
            10.0, 11.0, 11.0, 13.0, 14.0, 16.0, 17.0, 17.0, 16.0, 14.0, 12.0, 10.0,
        ],
    ),
    (
        "Salamanca",
        [
            6.0, 7.0, 8.0, 10.0, 12.0, 15.0, 17.0, 17.0, 15.0, 12.0, 8.0, 6.0,
        ],
    ),
    (
        "Guipúzcoa",
        [
            9.0, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0, 16.0, 15.0, 14.0, 11.0, 9.0,
        ],
    ),
    (
        "Santa Cruz de Tenerife",
        [
            15.0, 15.0, 16.0, 16.0, 17.0, 18.0, 20.0, 20.0, 20.0, 18.0, 17.0, 16.0,
        ],
    ),
    (
        "Cantabria",
        [
            10.0, 10.0, 11.0, 11.0, 13.0, 15.0, 16.0, 16.0, 16.0, 14.0, 12.0, 10.0,
        ],
    ),
    (
        "Segovia",
        [
            6.0, 7.0, 8.0, 10.0, 12.0, 15.0, 18.0, 18.0, 15.0, 12.0, 8.0, 6.0,
        ],
    ),
    (
        "Sevilla",
        [
            11.0, 11.0, 13.0, 14.0, 16.0, 19.0, 21.0, 21.0, 20.0, 16.0, 13.0, 11.0,
        ],
    ),
    (
        "Soria",
        [
            5.0, 6.0, 7.0, 9.0, 11.0, 14.0, 17.0, 16.0, 14.0, 11.0, 8.0, 6.0,
        ],
    ),
    (
        "Tarragona",
        [
            10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 20.0, 19.0, 16.0, 12.0, 11.0,
        ],
    ),
    (
        "Teruel",
        [
            6.0, 7.0, 8.0, 10.0, 12.0, 15.0, 18.0, 17.0, 15.0, 12.0, 8.0, 6.0,
        ],
    ),
    (
        "Toledo",
        [
            8.0, 9.0, 11.0, 12.0, 15.0, 18.0, 21.0, 20.0, 18.0, 14.0, 11.0, 8.0,
        ],
    ),
    (
        "Valencia",
        [
            10.0, 11.0, 12.0, 13.0, 15.0, 17.0, 19.0, 20.0, 18.0, 16.0, 13.0, 11.0,
        ],
    ),
    (
        "Valladolid",
        [
            6.0, 8.0, 9.0, 10.0, 12.0, 15.0, 18.0, 18.0, 16.0, 12.0, 9.0, 7.0,
        ],
    ),
    (
        "Álava",
        [
            7.0, 7.0, 8.0, 10.0, 12.0, 14.0, 16.0, 16.0, 14.0, 12.0, 8.0, 7.0,
        ],
    ),
    (
        "Zamora",
        [
            6.0, 8.0, 9.0, 10.0, 13.0, 16.0, 18.0, 18.0, 16.0, 12.0, 9.0, 7.0,
        ],
    ),
    (
        "Zaragoza",
        [
            8.0, 9.0, 10.0, 12.0, 15.0, 17.0, 20.0, 19.0, 17.0, 14.0, 10.0, 8.0,
        ],
    ),
];

/// The cold water of a province's capital, by month (Huelva's if the name
/// is not in the table), lowered for a site above it (Anejo G: 0,0066 °C a
/// metre from October to March, 0,0033 from April to September).
pub fn cold_water(province: &str, above_capital: f64) -> [f64; 12] {
    let base = COLD_WATER
        .iter()
        .find(|(p, _)| p.eq_ignore_ascii_case(province))
        .map_or(COLD_WATER[19].1, |(_, t)| *t);
    let mut out = base;
    for (i, t) in out.iter_mut().enumerate() {
        let b = if (3..=8).contains(&i) { 0.0033 } else { 0.0066 };
        *t -= b * above_capital;
    }
    out
}

impl Model {
    pub fn renewables_site(&self) -> &RenewablesSite {
        &self.renewables_site
    }

    pub fn set_renewables_site(&mut self, site: RenewablesSite) -> Result<(), ModelError> {
        if !site.is_valid() {
            return Err(ModelError::NotFinite("renewables site"));
        }
        self.renewables_site = site;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cold_water() {
        assert_eq!(cold_water("Huelva", 0.0)[0], 12.0);
        assert_eq!(cold_water("madrid", 0.0)[6], 20.0);
        // 100 m above the capital: 0,66 °C less in January, 0,33 in July.
        let t = cold_water("Sevilla", 100.0);
        assert!((t[0] - 10.34).abs() < 1e-9 && (t[6] - 20.67).abs() < 1e-9);
        assert!(RenewablesSite::default().is_valid());
        assert_eq!(AcsSource::from_code("solar"), Some(AcsSource::Solar));
    }
}
