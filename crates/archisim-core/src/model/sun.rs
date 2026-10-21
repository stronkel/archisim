//! The sun of the project, for the shadows of presentation drawings: a
//! day of the year and a solar hour, at the project's latitude (from its
//! georeference; Madrid's without one), turned by its true north.

use super::*;

/// When the shadows are cast.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sun {
    /// Day of the year, 1 (1 January) to 365.
    pub day: u16,
    /// Solar time, hours (12: noon, the sun to the south).
    pub hour: f64,
}

impl Default for Sun {
    /// The spring equinox at eleven in the morning: long shadows that do
    /// not hide the building.
    fn default() -> Self {
        Sun {
            day: 80,
            hour: 11.0,
        }
    }
}

/// Latitude used without a georeference (Madrid), degrees.
pub const DEFAULT_LATITUDE: f64 = 40.42;

impl Sun {
    /// Height above the horizon and bearing (from north, clockwise), in
    /// radians, at a latitude in degrees.
    pub fn position(&self, latitude: f64) -> (f64, f64) {
        let d = 23.44_f64.to_radians()
            * (std::f64::consts::TAU / 365.0 * (284.0 + self.day as f64)).sin();
        let phi = latitude.to_radians();
        let h = (15.0 * (self.hour - 12.0)).to_radians();
        let alt = (phi.sin() * d.sin() + phi.cos() * d.cos() * h.cos()).asin();
        // Bearing from north, clockwise (the sun to the east in the
        // morning, to the west in the afternoon).
        let az = h.sin().atan2(h.cos() * phi.sin() - d.tan() * phi.cos()) + std::f64::consts::PI;
        (alt, az)
    }

    pub fn is_valid(&self) -> bool {
        (1..=365).contains(&self.day) && self.hour.is_finite() && (0.0..=24.0).contains(&self.hour)
    }
}

impl Model {
    pub fn sun(&self) -> Sun {
        self.sun
    }

    pub fn set_sun(&mut self, sun: Sun) -> Result<(), ModelError> {
        if !sun.is_valid() {
            return Err(ModelError::NotFinite("sun"));
        }
        self.sun = sun;
        Ok(())
    }

    /// The project's latitude, degrees: its georeference's, or Madrid's.
    pub fn latitude(&self) -> f64 {
        self.georeference()
            .and_then(|g| g.lat_lon())
            .map_or(DEFAULT_LATITUDE, |(lat, _)| lat)
    }

    /// Unit vector towards the sun in the model's axes (x, y of the plan,
    /// z up); `None` with the sun below the horizon.
    pub fn sun_direction(&self) -> Option<[f64; 3]> {
        let (alt, az) = self.sun.position(self.latitude());
        if alt <= 0.02 {
            return None;
        }
        // North is `true_north` clockwise from the plan's up: a bearing
        // `az` from north is `az + true_north` from the plan's up.
        let b = az + self.true_north();
        Some([b.sin() * alt.cos(), b.cos() * alt.cos(), alt.sin()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sun_goes_round_from_east_to_west() {
        let noon = Sun {
            day: 172,
            hour: 12.0,
        }
        .position(40.42);
        // Midsummer noon in Madrid: about 73° high, to the south.
        assert!((noon.0.to_degrees() - 73.0).abs() < 1.0, "{noon:?}");
        assert!((noon.1.to_degrees() - 180.0).abs() < 1.0);
        let morning = Sun { day: 80, hour: 9.0 }.position(40.42);
        assert!(morning.1.to_degrees() > 90.0 && morning.1.to_degrees() < 150.0);
        let evening = Sun {
            day: 80,
            hour: 16.0,
        }
        .position(40.42);
        assert!(evening.1.to_degrees() > 210.0);
        let mut m = Model::new("Sol");
        let d = m.sun_direction().unwrap();
        // In the morning the sun is to the south-east: x > 0, y < 0.
        assert!(d[0] > 0.0 && d[1] < 0.0 && d[2] > 0.0, "{d:?}");
        m.set_sun(Sun {
            day: 355,
            hour: 6.0,
        })
        .unwrap();
        assert!(m.sun_direction().is_none(), "before dawn in winter");
        assert!(m.set_sun(Sun { day: 0, hour: 12.0 }).is_err());
    }
}
