//! The renewable energy of the building (DB-HE 4 and HE 5, 2022).
//!
//! - HE 4, the hot water: the daily demand at 60 °C of the dwellings by
//!   their bedrooms (Anejo F, tables a and b: 28 l a person, the persons
//!   by bedrooms and the centralisation factor by the number of
//!   dwellings) or as given, the energy of each month with the cold water
//!   of the province (Anejo G), and the part a heat pump, solar
//!   collectors or another source cover against the 70 % (60 % below
//!   5 000 l a day) the section asks.
//! - HE 5, the electricity: the minimum peak power P = min(Fpr,el·S;
//!   0,1·(0,5·Sc − Soc)) by the built area S and the roof area Sc not
//!   walked on, against the solar panels drawn.
//!
//! The demand is increased by the losses of distribution, storage and
//! recirculation (HE 4, 4.1), worked out of the drawn hot water network
//! (`Losses`).
//!
//! HE 0 (primary energy) is not worked out: it needs the official tool.

use archisim_core::{cold_water, AcsSource, BuildingUse, DeviceKind, Model, RenewablesSite};
use archisim_geom::junctions::{difference, union, Shape};

/// Litres a day at 60 °C a person in a dwelling (Anejo F).
pub const LITRES_A_PERSON: f64 = 28.0;
/// Days of each month.
pub const DAYS: [f64; 12] = [
    31.0, 28.0, 31.0, 30.0, 31.0, 30.0, 31.0, 31.0, 30.0, 31.0, 30.0, 31.0,
];
/// Wh to heat a litre a degree.
const WH_PER_LITRE_K: f64 = 1.162;
/// Below this, HE 4 does not apply (litres a day).
pub const HE4_THRESHOLD: f64 = 100.0;
/// Below this, the renewable part asked is 60 %, above it 70 %.
pub const HE4_LARGE: f64 = 5000.0;
/// HE 5 applies above this built area (m²).
pub const HE5_THRESHOLD: f64 = 1000.0;
/// The room around the pipes and the store, °C.
pub const AMBIENT: f64 = 20.0;
/// Heat lost by a metre of insulated pipe a degree, W/mK (insulation of
/// the RITE, IT 1.2.4.2.1, on the usual sizes).
pub const PIPE_PSI: f64 = 0.25;
/// Times a day the water left in a pipe without return cools down.
pub const COOLINGS: f64 = 3.0;
/// The water arrives this much cooler at the end of the loop, °C.
pub const LOOP_DROP: f64 = 5.0;

/// Heat pumps below this seasonal performance count as no renewable
/// (Directive 2018/2001, Annex VII, with the Spanish grid).
pub const MIN_SCOP: f64 = 2.5;

/// Anejo F table a: persons of a dwelling by its bedrooms.
pub fn persons(bedrooms: usize) -> f64 {
    match bedrooms {
        0 | 1 => 1.5,
        2 => 3.0,
        3 => 4.0,
        4 => 5.0,
        5 | 6 => 6.0,
        _ => 7.0,
    }
}

/// Anejo F table b: the centralisation factor by the number of dwellings.
pub fn centralisation(dwellings: usize) -> f64 {
    match dwellings {
        0..=3 => 1.0,
        4..=10 => 0.95,
        11..=20 => 0.90,
        21..=50 => 0.85,
        51..=75 => 0.80,
        76..=100 => 0.75,
        _ => 0.70,
    }
}

/// A dwelling's share of the hot water.
#[derive(Debug, Clone, PartialEq)]
pub struct DwellingDemand {
    pub name: String,
    pub bedrooms: usize,
    pub persons: f64,
}

/// The losses of the hot water (HE 4, 4.1), kWh a year each.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Losses {
    /// Hot pipe drawn, metres; the part in the loop, kept hot by the
    /// return (and the return itself), and its water, litres.
    pub pipe: f64,
    pub looped: f64,
    pub dead_water: f64,
    /// The store: litres, whether given, and its standing loss, W.
    pub tank: f64,
    pub tank_given: bool,
    pub standing: f64,
    pub distribution: f64,
    pub storage: f64,
    pub recirculation: f64,
}

impl Losses {
    pub fn total(&self) -> f64 {
        self.distribution + self.storage + self.recirculation
    }

    /// A day's losses, kWh.
    fn daily(&self) -> f64 {
        self.total() / 365.0
    }
}

/// The losses of the hot water drawn.
pub fn losses(model: &Model, site: &RenewablesSite, daily_stored: f64) -> Losses {
    let w = crate::water::water(model);
    let hot: Vec<&crate::water::Stretch> = w
        .networks
        .iter()
        .filter(|n| n.system == archisim_core::ServiceSystem::HotWater)
        .flat_map(|n| n.stretches.iter())
        .collect();
    let pipe: f64 = hot.iter().map(|s| s.length).sum();
    let hot_t = site.storage_temp;
    // The loop: from the heater to the farthest tap and back.
    let looped = w.hot_return.as_ref().map_or(0.0, |r| r.length.min(pipe));
    let loop_t = hot_t - LOOP_DROP / 2.0;
    let recirculation = PIPE_PSI * 2.0 * looped * (loop_t - AMBIENT).max(0.0) * 8.76;
    // The rest: the water left in it cools down after each use.
    let share = if pipe > 0.0 {
        ((pipe - looped) / pipe).max(0.0)
    } else {
        0.0
    };
    let dead_water: f64 = hot
        .iter()
        .map(|s| std::f64::consts::PI / 4.0 * s.drawn.1 * s.drawn.1 * s.length * 1000.0)
        .sum::<f64>()
        * share;
    let distribution =
        dead_water * WH_PER_LITRE_K * (hot_t - AMBIENT).max(0.0) * COOLINGS * 365.0 / 1000.0;
    // The store: the standing loss of a class C store (Regulation (EU)
    // 812/2013: 16,66 + 8,33·V^0,4 W at 45 K), at its own temperature.
    let (tank, tank_given) = match site.tank_volume {
        Some(v) => (v, true),
        None => (daily_stored, false),
    };
    let standing = if tank > 0.0 {
        (16.66 + 8.33 * tank.powf(0.4)) * (hot_t - AMBIENT).max(0.0) / 45.0
    } else {
        0.0
    };
    Losses {
        pipe,
        looped,
        dead_water,
        tank,
        tank_given,
        standing,
        distribution,
        storage: standing * 8.76,
        recirculation,
    }
}

/// HE 4, the hot water.
#[derive(Debug, Clone, PartialEq)]
pub struct HotWater {
    /// Whether the section applies (a new building asking more than
    /// 100 l a day).
    pub applies: bool,
    /// The demand comes from the site (given) and not the dwellings.
    pub given: bool,
    pub dwellings: Vec<DwellingDemand>,
    pub persons: f64,
    pub factor: f64,
    /// Litres a day at 60 °C, and at the store's temperature.
    pub daily: f64,
    pub daily_stored: f64,
    pub cold: [f64; 12],
    /// kWh of each month and of the year: the water heated, the losses,
    /// and both (`annual`, what the renewable part is of).
    pub useful: [f64; 12],
    pub lost: [f64; 12],
    pub monthly: [f64; 12],
    pub annual: f64,
    pub losses: Losses,
    /// The renewable part asked (0,6 or 0,7) and the one given.
    pub required: f64,
    pub provided: f64,
    pub source: AcsSource,
    /// A heat pump too poor to count.
    pub weak_pump: bool,
}

impl HotWater {
    pub fn renewable_kwh(&self) -> f64 {
        self.annual * self.provided
    }

    pub fn passes(&self) -> bool {
        !self.applies || self.provided + 1e-9 >= self.required
    }
}

/// HE 5, the electricity.
#[derive(Debug, Clone, PartialEq)]
pub struct Electricity {
    pub applies: bool,
    /// The built area, m², and whether it was given.
    pub built: f64,
    pub built_given: bool,
    /// The roof area not walked on (Sc) and the one the solar collectors
    /// take (Soc), m².
    pub roof: f64,
    pub collectors: f64,
    pub factor: f64,
    pub p1: f64,
    pub p2: f64,
    /// The minimum, kWp.
    pub minimum: f64,
    /// The panels drawn: how many rows, their area and power.
    pub panels: usize,
    pub panel_area: f64,
    pub installed: f64,
}

impl Electricity {
    pub fn passes(&self) -> bool {
        !self.applies || self.installed + 1e-9 >= self.minimum
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Renewables {
    pub site: RenewablesSite,
    pub hot_water: HotWater,
    pub electricity: Electricity,
}

impl Renewables {
    /// The checks that fail.
    pub fn fails(&self) -> usize {
        usize::from(!self.hot_water.passes()) + usize::from(!self.electricity.passes())
    }
}

/// The renewables of the model as drawn.
pub fn renewables(model: &Model) -> Renewables {
    let site = model.renewables_site().clone();
    Renewables {
        hot_water: hot_water(model, &site),
        electricity: electricity(model, &site),
        site,
    }
}

fn hot_water(model: &Model, site: &RenewablesSite) -> HotWater {
    let dwellings: Vec<DwellingDemand> =
        if model.code_settings().building_use == BuildingUse::Dwelling {
            crate::ventilation::ventilation(model)
                .dwellings
                .iter()
                .map(|d| DwellingDemand {
                    name: d.name.clone(),
                    bedrooms: d.bedrooms,
                    persons: persons(d.bedrooms),
                })
                .collect()
        } else {
            Vec::new()
        };
    let persons_all: f64 = dwellings.iter().map(|d| d.persons).sum();
    let factor = centralisation(dwellings.len());
    let (daily, given) = match site.acs_demand {
        Some(l) => (l, true),
        None => (persons_all * LITRES_A_PERSON * factor, false),
    };
    let cold = cold_water(&site.province, site.above_capital);
    let mean_cold = cold.iter().sum::<f64>() / 12.0;
    let daily_stored = if site.storage_temp > mean_cold + 1.0 {
        daily * (60.0 - mean_cold) / (site.storage_temp - mean_cold)
    } else {
        daily
    };
    let losses = if daily > 0.0 {
        losses(model, site, daily_stored)
    } else {
        Losses::default()
    };
    let (mut useful, mut lost, mut monthly) = ([0.0; 12], [0.0; 12], [0.0; 12]);
    for i in 0..12 {
        useful[i] = daily * DAYS[i] * WH_PER_LITRE_K * (60.0 - cold[i]).max(0.0) / 1000.0;
        lost[i] = losses.daily() * DAYS[i];
        monthly[i] = useful[i] + lost[i];
    }
    let annual: f64 = monthly.iter().sum();
    let weak_pump = site.acs_source == AcsSource::HeatPump && site.scop <= MIN_SCOP;
    let provided = match site.acs_source {
        AcsSource::HeatPump if !weak_pump => 1.0 - 1.0 / site.scop,
        AcsSource::Solar | AcsSource::Other => site.fraction,
        _ => 0.0,
    };
    HotWater {
        applies: site.new_building && daily > HE4_THRESHOLD,
        given,
        dwellings,
        persons: persons_all,
        factor,
        daily,
        daily_stored,
        cold,
        useful,
        lost,
        monthly,
        annual,
        losses,
        required: if daily < HE4_LARGE { 0.6 } else { 0.7 },
        provided,
        source: site.acs_source,
        weak_pump,
    }
}

fn shape(outline: &[archisim_core::Point2]) -> Shape {
    (outline.to_vec(), Vec::new())
}

fn area_of(shapes: &[Shape]) -> f64 {
    shapes
        .iter()
        .map(|(o, hs)| {
            archisim_core::signed_area(o).abs()
                - hs.iter()
                    .map(|h| archisim_core::signed_area(h).abs())
                    .sum::<f64>()
        })
        .sum()
}

/// The built area: rooms and walls of every level together (as the
/// surfaces table of the CTE check).
pub fn built_area(model: &Model) -> f64 {
    model
        .levels()
        .into_iter()
        .map(|l| {
            let mut shapes: Vec<Shape> = model
                .rooms_on_level(l.id)
                .map(|r| shape(&r.outline))
                .collect();
            shapes.extend(
                archisim_geom::wall_geometries(model, l.id)
                    .iter()
                    .map(|g| shape(&g.outline)),
            );
            if shapes.is_empty() {
                0.0
            } else {
                area_of(&union(&shapes, &[]))
            }
        })
        .sum()
}

/// The roof not walked on: the pitched roofs' footprints and the slabs of
/// every level but the lowest one where no room stands on them (a room
/// on a slab, a terrace among them, is walked on).
pub fn roof_area(model: &Model) -> f64 {
    let lowest = model
        .levels()
        .into_iter()
        .min_by(|a, b| a.elevation.total_cmp(&b.elevation))
        .map(|l| l.id);
    let mut total = 0.0;
    for l in model.levels() {
        let mut roofs: Vec<Shape> = model
            .roofs_on_level(l.id)
            .map(|r| shape(&r.outline))
            .collect();
        if Some(l.id) != lowest {
            let slabs: Vec<Shape> = model
                .slabs()
                .filter(|s| s.level == l.id)
                .map(|s| (s.outline.clone(), s.holes.clone()))
                .collect();
            let rooms: Vec<Shape> = model
                .rooms_on_level(l.id)
                .map(|r| shape(&r.outline))
                .collect();
            if !slabs.is_empty() {
                let open = if rooms.is_empty() {
                    union(&slabs, &[])
                } else {
                    difference(&union(&slabs, &[]), &union(&rooms, &[]))
                };
                roofs.extend(
                    open.into_iter()
                        .filter(|s| area_of(std::slice::from_ref(s)) > 1.0),
                );
            }
        }
        if !roofs.is_empty() {
            total += area_of(&union(&roofs, &[]));
        }
    }
    total
}

fn electricity(model: &Model, site: &RenewablesSite) -> Electricity {
    let (built, built_given) = match site.built_area {
        Some(a) => (a, true),
        None => (built_area(model), false),
    };
    let roof = roof_area(model);
    let collectors: f64 = model
        .devices()
        .filter(|d| d.kind == DeviceKind::SolarCollector)
        .map(|d| d.width * d.depth)
        .sum();
    let factor = if model.code_settings().building_use == BuildingUse::Dwelling {
        0.005
    } else {
        0.010
    };
    let p1 = factor * built;
    let p2 = (0.1 * (0.5 * roof - collectors)).max(0.0);
    let panels: Vec<_> = model
        .devices()
        .filter(|d| d.kind == DeviceKind::SolarPanel)
        .collect();
    let panel_area: f64 = panels.iter().map(|d| d.width * d.depth).sum();
    Electricity {
        applies: site.new_building && built > HE5_THRESHOLD,
        built,
        built_given,
        roof,
        collectors,
        factor,
        p1,
        p2,
        minimum: p1.min(p2),
        panels: panels.len(),
        panel_area,
        installed: panel_area * site.panel_kwp_m2,
    }
}

fn dec(v: f64, n: usize) -> String {
    // An empty sum is −0.
    let v = if v == 0.0 { 0.0 } else { v };
    format!("{v:.n$}").replace('.', ",")
}

/// The calculation in text.
pub fn report(r: &Renewables) -> String {
    let h = &r.hot_water;
    let e = &r.electricity;
    let mut s = String::new();
    s.push_str("ENERGÍAS RENOVABLES (DB-HE 4 y HE 5)\n\n");
    s.push_str(&format!(
        "HE 4 — Agua caliente sanitaria ({}, {} m sobre la capital)\n",
        r.site.province,
        dec(r.site.above_capital, 0)
    ));
    if h.given {
        s.push_str(&format!(
            "  Demanda dada: {} l/día a 60 °C\n",
            dec(h.daily, 0)
        ));
    } else {
        s.push_str(&format!(
            "  {} viviendas, {} personas × 28 l × {} = {} l/día a 60 °C\n",
            h.dwellings.len(),
            dec(h.persons, 1),
            dec(h.factor, 2),
            dec(h.daily, 0)
        ));
    }
    let l = &h.losses;
    s.push_str(&format!(
        "  Pérdidas: distribución {} kWh, acumulación {} kWh ({} l), recirculación {} kWh\n",
        dec(l.distribution, 0),
        dec(l.storage, 0),
        dec(l.tank, 0),
        dec(l.recirculation, 0)
    ));
    s.push_str(&format!(
        "  Energía anual {} kWh; renovable {} % ({}), exigido {} %{}\n",
        dec(h.annual, 0),
        dec(h.provided * 100.0, 0),
        h.source.name(),
        dec(h.required * 100.0, 0),
        if !h.applies {
            " — no se aplica"
        } else if h.passes() {
            " — CUMPLE"
        } else {
            " — NO CUMPLE"
        }
    ));
    s.push_str(&format!(
        "\nHE 5 — Electricidad\n  S = {} m², Sc = {} m², Soc = {} m²\n  P1 = {} kWp, P2 = {} kWp, mínima {} kWp; dibujada {} kWp ({} m² de placas){}\n",
        dec(e.built, 0),
        dec(e.roof, 0),
        dec(e.collectors, 0),
        dec(e.p1, 2),
        dec(e.p2, 2),
        dec(e.minimum, 2),
        dec(e.installed, 2),
        dec(e.panel_area, 1),
        if !e.applies {
            " — no se aplica"
        } else if e.passes() {
            " — CUMPLE"
        } else {
            " — NO CUMPLE"
        }
    ));
    s.push_str(
        "\nHE 0 (consumo de energía primaria) no se calcula aquí: se justifica con la herramienta oficial.\n",
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tables() {
        assert_eq!(persons(3), 4.0);
        assert_eq!(persons(9), 7.0);
        assert_eq!(centralisation(12), 0.90);
        assert_eq!(centralisation(200), 0.70);
    }

    #[test]
    fn a_given_demand_in_huelva() {
        let mut m = archisim_library::new_project("Prueba");
        m.set_renewables_site(RenewablesSite {
            acs_demand: Some(1000.0),
            acs_source: AcsSource::HeatPump,
            scop: 3.0,
            ..Default::default()
        })
        .unwrap();
        let r = renewables(&m);
        let h = &r.hot_water;
        assert!(h.applies && h.given);
        // January in Huelva: 1000 l × 31 d × 1,162 Wh × (60 − 12) / 1000.
        assert!((h.useful[0] - 1000.0 * 31.0 * 1.162 * 48.0 / 1000.0).abs() < 1e-6);
        // No pipes drawn: only the store's loss, a day's litres at 60 °C.
        let l = &h.losses;
        assert_eq!((l.distribution, l.recirculation), (0.0, 0.0));
        let s = (16.66 + 8.33 * l.tank.powf(0.4)) * 40.0 / 45.0;
        assert!((l.storage - s * 8.76).abs() < 1e-6 && l.storage > 0.0);
        assert!((h.annual - h.useful.iter().sum::<f64>() - l.storage).abs() < 1e-6);
        assert_eq!(h.required, 0.6);
        assert!((h.provided - 2.0 / 3.0).abs() < 1e-9);
        assert!(h.passes());
        // A poor pump does not count.
        let mut site = m.renewables_site().clone();
        site.scop = 2.2;
        m.set_renewables_site(site).unwrap();
        let h = renewables(&m).hot_water;
        assert!(h.weak_pump && !h.passes());
        // An empty project: nothing to build, HE 5 does not apply.
        assert!(!renewables(&m).electricity.applies);
    }
}
