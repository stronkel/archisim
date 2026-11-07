//! The solar fraction of the hot water by the f-Chart method (Klein,
//! Beckman and Duffie, 1976), as the Spanish practice applies it to the
//! DB-HE 4: month by month, the energy the collectors absorb (D1) and lose
//! (D2) over the month's load, and f from the correlation of the method.
//!
//! The collectors are the «Captadores solares» drawn: their area along
//! their slope, their tilt (from their box: the height they rise over
//! their depth) and the way they face. The sun on them comes from the
//! mean daily irradiation on the horizontal of each month (the site's,
//! given, or estimated from the latitude) by the isotropic sky of Liu and
//! Jordan, with the diffuse part of Erbs and the beam factor worked out
//! hour by hour, so any tilt and orientation counts.

use archisim_core::{DeviceKind, Model, RenewablesSite};

/// Days of each month.
const DAYS: [f64; 12] = [
    31.0, 28.0, 31.0, 30.0, 31.0, 30.0, 31.0, 31.0, 30.0, 31.0, 30.0, 31.0,
];

/// The mean day of each month (its number in the year, Klein 1977).
const MEAN_DAY: [f64; 12] = [
    17.0, 47.0, 75.0, 105.0, 135.0, 162.0, 198.0, 228.0, 258.0, 288.0, 318.0, 344.0,
];

/// The solar constant, W/m².
const SOLAR_CONSTANT: f64 = 1367.0;

/// The clearness of the sky of each month where the site's irradiation is
/// not given: the share of the sun above the atmosphere that reaches the
/// ground, a mean of the Iberian peninsula. It is an estimate: the site's
/// own data (PVGIS, AEMET) should replace it.
pub const ESTIMATED_CLEARNESS: [f64; 12] = [
    0.50, 0.53, 0.56, 0.57, 0.60, 0.64, 0.68, 0.67, 0.62, 0.55, 0.50, 0.48,
];

/// How much warmer than the cold water the air is by day, each month,
/// where the site's temperatures are not given (an estimate).
const AIR_OVER_COLD_WATER: [f64; 12] = [2.0, 2.0, 3.0, 4.0, 5.0, 6.0, 6.0, 6.0, 5.0, 4.0, 3.0, 2.0];

/// The reflectance of the ground in front of the collectors.
const GROUND_REFLECTANCE: f64 = 0.2;

/// The lowest temperature of the hot water at the tap, °C (the Tac of the
/// method).
pub const TAC: f64 = 45.0;

/// The factors the method takes for the incidence angle (the mean
/// τα over the normal one) and for the heat exchanger (F'R/FR).
const INCIDENCE: f64 = 0.96;
const EXCHANGER: f64 = 0.95;

/// The thickness of a collector over its box's height (the rise is the
/// rest): as `archisim_props::fine::services` draws them.
const COLLECTOR_THICK: f64 = 0.08;

/// One collector box as the method needs it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Collector {
    /// Its area along its slope, m².
    pub area: f64,
    /// Its tilt from the horizontal, radians.
    pub tilt: f64,
    /// Which way it faces: from the south, radians, positive to the west.
    pub azimuth: f64,
}

/// A month of the method.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Month {
    /// Mean daily irradiation on the horizontal and on the collectors,
    /// kWh/m².
    pub horizontal: f64,
    pub on_collectors: f64,
    /// Air by day, °C.
    pub ambient: f64,
    /// The load (the water heated and the losses), kWh.
    pub load: f64,
    pub d1: f64,
    pub d2: f64,
    /// The solar fraction of the month (0 to 1) and the energy it gives,
    /// kWh.
    pub f: f64,
    pub solar: f64,
}

/// The f-Chart of a building's collectors.
#[derive(Debug, Clone, PartialEq)]
pub struct SolarFraction {
    pub collectors: Vec<Collector>,
    pub area: f64,
    /// Area-weighted tilt and azimuth, degrees (to tell).
    pub tilt_deg: f64,
    pub azimuth_deg: f64,
    pub latitude: f64,
    /// The store, litres, and per m² of collector.
    pub volume: f64,
    pub litres_m2: f64,
    pub eta0: f64,
    pub k1: f64,
    /// The irradiation or the air are estimates (no site data given).
    pub estimated_sun: bool,
    pub estimated_air: bool,
    pub months: [Month; 12],
    /// Σ solar / Σ load.
    pub annual: f64,
    /// Months the collectors would give more than the load (the f of the
    /// correlation over 1): a sign of a field too big for summer.
    pub over: Vec<usize>,
}

/// The collectors drawn in a model.
pub fn collectors(model: &Model) -> Vec<Collector> {
    let north = model.true_north();
    model
        .devices()
        .filter(|d| d.kind == DeviceKind::SolarCollector)
        .filter_map(|d| {
            let rise = if d.height <= 0.05 {
                0.0
            } else {
                (d.height - COLLECTOR_THICK).max(0.0)
            };
            let slope = (rise * rise + d.depth * d.depth).sqrt();
            let area = d.width * slope;
            if area <= 1e-6 {
                return None;
            }
            let tilt = rise.atan2(d.depth);
            // They face their box's +y (the low front), turned with it.
            let (dx, dy) = (-d.rotation.sin(), d.rotation.cos());
            // Bearing from north, clockwise; from the south, to the west.
            let bearing = dx.atan2(dy) - north;
            let mut azimuth = bearing - std::f64::consts::PI;
            while azimuth > std::f64::consts::PI {
                azimuth -= std::f64::consts::TAU;
            }
            while azimuth < -std::f64::consts::PI {
                azimuth += std::f64::consts::TAU;
            }
            Some(Collector {
                area,
                tilt,
                azimuth,
            })
        })
        .collect()
}

/// The latitude of the f-Chart: the project's georeference, or its
/// province's capital.
pub fn latitude(model: &Model, site: &RenewablesSite) -> f64 {
    model.georeference().and_then(|g| g.lat_lon()).map_or_else(
        || archisim_core::capital_latitude(&site.province),
        |(lat, _)| lat,
    )
}

/// The estimate of the site's climate where it is not given: the mean
/// daily irradiation on the horizontal (kWh/m²) by the latitude and a
/// mean clearness, and the air by day (°C) by the cold water.
pub fn estimated_climate(model: &Model, site: &RenewablesSite) -> ([f64; 12], [f64; 12]) {
    let phi = latitude(model, site).to_radians();
    let cold = archisim_core::cold_water(&site.province, site.above_capital);
    let sun = std::array::from_fn(|i| ESTIMATED_CLEARNESS[i] * extraterrestrial(phi, MEAN_DAY[i]));
    let air = std::array::from_fn(|i| cold[i] + AIR_OVER_COLD_WATER[i]);
    (sun, air)
}

/// Declination of the sun on a day of the year, radians.
fn declination(n: f64) -> f64 {
    (23.45_f64).to_radians() * (std::f64::consts::TAU * (284.0 + n) / 365.0).sin()
}

/// Mean daily irradiation above the atmosphere on the horizontal, kWh/m²,
/// on a day `n` at latitude `phi` (radians).
pub fn extraterrestrial(phi: f64, n: f64) -> f64 {
    let d = declination(n);
    let ws = (-phi.tan() * d.tan()).clamp(-1.0, 1.0).acos();
    let e = 1.0 + 0.033 * (std::f64::consts::TAU * n / 365.0).cos();
    24.0 / std::f64::consts::PI * SOLAR_CONSTANT / 1000.0
        * e
        * (phi.cos() * d.cos() * ws.sin() + ws * phi.sin() * d.sin())
}

/// The irradiation on a plane over that on the horizontal, for a month's
/// mean day: Liu–Jordan's isotropic sky with Erbs's diffuse part, and the
/// beam factor summed over the day.
fn tilt_factor(phi: f64, n: f64, kt: f64, tilt: f64, azimuth: f64) -> f64 {
    let d = declination(n);
    let ws = (-phi.tan() * d.tan()).clamp(-1.0, 1.0).acos();
    let kt = kt.clamp(0.3, 0.8);
    let diffuse = if ws.to_degrees() <= 81.4 {
        1.391 - 3.560 * kt + 4.189 * kt * kt - 2.137 * kt * kt * kt
    } else {
        1.311 - 3.022 * kt + 3.427 * kt * kt - 1.821 * kt * kt * kt
    }
    .clamp(0.0, 1.0);
    let (sb, cb) = tilt.sin_cos();
    let (sg, cg) = azimuth.sin_cos();
    let (sd, cd) = d.sin_cos();
    let (sp, cp) = phi.sin_cos();
    let steps = 240;
    let (mut on_plane, mut on_ground) = (0.0, 0.0);
    for i in 0..steps {
        let w = -ws + (i as f64 + 0.5) * 2.0 * ws / steps as f64;
        let (sw, cw) = w.sin_cos();
        let zenith = sd * sp + cd * cp * cw;
        if zenith <= 0.0 {
            continue;
        }
        let incidence = sd * sp * cb - sd * cp * sb * cg
            + cd * cp * cb * cw
            + cd * sp * sb * cg * cw
            + cd * sb * sg * sw;
        on_plane += incidence.max(0.0);
        on_ground += zenith;
    }
    let rb = if on_ground > 0.0 {
        on_plane / on_ground
    } else {
        0.0
    };
    (1.0 - diffuse) * rb + diffuse * (1.0 + cb) / 2.0 + GROUND_REFLECTANCE * (1.0 - cb) / 2.0
}

/// The f-Chart of the collectors drawn, for the monthly loads (kWh), the
/// store (litres) and the cold water (°C); `None` without collectors.
pub fn f_chart(
    model: &Model,
    site: &RenewablesSite,
    load: [f64; 12],
    volume: f64,
    cold: [f64; 12],
) -> Option<SolarFraction> {
    let collectors = collectors(model);
    let area: f64 = collectors.iter().map(|c| c.area).sum();
    if collectors.is_empty() || area <= 0.0 {
        return None;
    }
    let latitude = latitude(model, site);
    let phi = latitude.to_radians();
    let eta0 = site.collector_eta0;
    let k1 = site.collector_k1;
    // F'R(τα) and F'R·UL.
    let gain = eta0 * INCIDENCE * EXCHANGER;
    let loss = k1 * EXCHANGER;
    // The store against the method's 75 l/m² (37,5 to 300 l/m²).
    let litres_m2 = volume / area;
    let k_store = (litres_m2 / 75.0).clamp(0.5, 4.0).powf(-0.25);
    let mut months = [Month::default(); 12];
    for i in 0..12 {
        let h0 = extraterrestrial(phi, MEAN_DAY[i]);
        let horizontal = match site.irradiation {
            Some(m) => m[i],
            None => ESTIMATED_CLEARNESS[i] * h0,
        };
        let kt = if h0 > 0.0 { horizontal / h0 } else { 0.5 };
        let on_collectors = collectors
            .iter()
            .map(|c| c.area * horizontal * tilt_factor(phi, MEAN_DAY[i], kt, c.tilt, c.azimuth))
            .sum::<f64>()
            / area;
        let ambient = match site.ambient {
            Some(m) => m[i],
            None => cold[i] + AIR_OVER_COLD_WATER[i],
        };
        let l = load[i];
        let mut m = Month {
            horizontal,
            on_collectors,
            ambient,
            load: l,
            ..Month::default()
        };
        if l > 0.0 {
            let absorbed = area * gain * on_collectors * DAYS[i];
            let k_water = (11.6 + 1.18 * TAC + 3.86 * cold[i] - 2.32 * ambient) / (100.0 - ambient);
            let lost =
                area * loss * (100.0 - ambient) * DAYS[i] * 24.0 / 1000.0 * k_store * k_water;
            let (d1, d2) = (absorbed / l, lost / l);
            let f = 1.029 * d1 - 0.065 * d2 - 0.245 * d1 * d1
                + 0.0018 * d2 * d2
                + 0.0215 * d1 * d1 * d1;
            m.d1 = d1;
            m.d2 = d2;
            m.f = f;
            m.solar = f.clamp(0.0, 1.0) * l;
        }
        months[i] = m;
    }
    let total_load: f64 = months.iter().map(|m| m.load).sum();
    let total_solar: f64 = months.iter().map(|m| m.solar).sum();
    let mean = |f: &dyn Fn(&Collector) -> f64| {
        collectors.iter().map(|c| c.area * f(c)).sum::<f64>() / area
    };
    Some(SolarFraction {
        tilt_deg: mean(&|c| c.tilt).to_degrees(),
        azimuth_deg: mean(&|c| c.azimuth).to_degrees(),
        collectors,
        area,
        latitude,
        volume,
        litres_m2,
        eta0,
        k1,
        estimated_sun: site.irradiation.is_none(),
        estimated_air: site.ambient.is_none(),
        annual: if total_load > 0.0 {
            total_solar / total_load
        } else {
            0.0
        },
        over: (0..12).filter(|&i| months[i].f > 1.0).collect(),
        months,
    })
}

/// The monthly irradiation (kWh/m² a day) and air (°C) of a monthly file
/// of PVGIS (its «Monthly radiation» tool, CSV or text): the mean of each
/// month over the years it holds. The air is `None` when the file has no
/// temperature column.
pub fn read_pvgis(text: &str) -> Option<([f64; 12], Option<[f64; 12]>)> {
    let mut columns: Option<(usize, usize, Option<usize>)> = None;
    let mut sums = [(0.0_f64, 0.0_f64, 0usize); 12];
    for line in text.lines() {
        let cells: Vec<&str> = line
            .split(['\t', ',', ';'])
            .flat_map(|c| c.split_whitespace())
            .collect();
        if cells.is_empty() {
            continue;
        }
        if columns.is_none() {
            let at = |name: &str| cells.iter().position(|c| c.eq_ignore_ascii_case(name));
            if let (Some(m), Some(h)) = (at("month"), at("H(h)_m")) {
                columns = Some((m, h, at("T2m")));
            }
            continue;
        }
        let (m, h, t) = columns.unwrap_or((1, 2, None));
        let month = cells.get(m).and_then(|v| month_number(v));
        let value = cells.get(h).and_then(|v| v.parse::<f64>().ok());
        let (Some(month), Some(value)) = (month, value) else {
            continue;
        };
        let air = t
            .and_then(|t| cells.get(t))
            .and_then(|v| v.parse::<f64>().ok());
        let s = &mut sums[month];
        s.0 += value / DAYS[month];
        s.1 += air.unwrap_or(f64::NAN);
        s.2 += 1;
    }
    columns?;
    if sums.iter().any(|s| s.2 == 0) {
        return None;
    }
    let sun = sums.map(|s| s.0 / s.2 as f64);
    let air = sums.map(|s| s.1 / s.2 as f64);
    let air = air.iter().all(|v| v.is_finite()).then_some(air);
    Some((sun, air))
}

/// A month, by its number or its English short name, from 0.
fn month_number(s: &str) -> Option<usize> {
    if let Ok(n) = s.parse::<usize>() {
        return (1..=12).contains(&n).then(|| n - 1);
    }
    const NAMES: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let low = s.to_ascii_lowercase();
    NAMES.iter().position(|n| low.starts_with(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Above the atmosphere at 40° N: 15,2 MJ/m² in January (4,22 kWh)
    /// and 41,5 in June (11,5 kWh), as Duffie and Beckman give them.
    #[test]
    fn the_sun_above_the_atmosphere() {
        let phi = 40f64.to_radians();
        let jan = extraterrestrial(phi, 17.0);
        let jun = extraterrestrial(phi, 162.0);
        assert!((jan - 4.22).abs() < 0.05, "{jan}");
        assert!((jun - 11.5).abs() < 0.15, "{jun}");
    }

    /// Facing south at the latitude, a collector catches more than the
    /// horizontal in winter and less in summer; facing north, less always.
    #[test]
    fn the_tilt_factor() {
        let phi = 37f64.to_radians();
        let tilt = 37f64.to_radians();
        let winter = tilt_factor(phi, 17.0, 0.5, tilt, 0.0);
        let summer = tilt_factor(phi, 162.0, 0.65, tilt, 0.0);
        assert!(winter > 1.4 && winter < 2.0, "{winter}");
        assert!(summer > 0.8 && summer < 1.0, "{summer}");
        let north = tilt_factor(phi, 17.0, 0.5, tilt, std::f64::consts::PI);
        assert!(north < 0.6, "{north}");
        // Flat, it is the horizontal.
        let flat = tilt_factor(phi, 105.0, 0.57, 0.0, 0.0);
        assert!((flat - 1.0).abs() < 0.01, "{flat}");
    }

    /// The estimate gives Madrid and Huelva a year of 1.650 to 1.950
    /// kWh/m² on the horizontal, as the atlases do.
    #[test]
    fn the_estimate_is_of_the_order_of_the_atlas() {
        for lat in [40.42_f64, 37.26] {
            let phi = lat.to_radians();
            let year: f64 = (0..12)
                .map(|i| ESTIMATED_CLEARNESS[i] * extraterrestrial(phi, MEAN_DAY[i]) * DAYS[i])
                .sum();
            assert!((1650.0..1950.0).contains(&year), "{lat}: {year}");
        }
    }

    #[test]
    fn reads_a_monthly_file_of_pvgis() {
        let text = "Latitude (decimal degrees):\t37.260\n\
                    Radiation database:\tPVGIS-SARAH3\n\n\
                    year\t\tmonth\t\tH(h)_m\t\tT2m\n\
                    2020\t\tJan\t\t83.7\t\t11.9\n\
                    2020\t\tFeb\t\t106.4\t\t13.5\n\
                    2020\t\tMar\t\t150.2\t\t14.8\n\
                    2020\t\tApr\t\t180.0\t\t16.9\n\
                    2020\t\tMay\t\t225.3\t\t20.1\n\
                    2020\t\tJun\t\t240.0\t\t23.8\n\
                    2020\t\tJul\t\t257.0\t\t26.9\n\
                    2020\t\tAug\t\t228.0\t\t27.1\n\
                    2020\t\tSep\t\t170.0\t\t24.3\n\
                    2020\t\tOct\t\t125.0\t\t20.4\n\
                    2020\t\tNov\t\t89.0\t\t15.6\n\
                    2020\t\tDec\t\t74.4\t\t13.0\n\
                    2021\t\tJan\t\t89.9\t\t12.1\n";
        let (sun, air) = read_pvgis(text).unwrap();
        // January: the two years, by day.
        assert!((sun[0] - (83.7 + 89.9) / 2.0 / 31.0).abs() < 1e-9);
        assert!((sun[5] - 8.0).abs() < 1e-9);
        let air = air.unwrap();
        assert!((air[0] - 12.0).abs() < 1e-9);
        // Without the months, nothing.
        assert!(read_pvgis("year\tmonth\tH(h)_m\n2020\t1\t80\n").is_none());
    }
}
