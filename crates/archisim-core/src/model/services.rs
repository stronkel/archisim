//! Instalaciones: pipes and ducts by their path, and the devices they
//! serve (equipment, lights, sockets and switches).
//!
//! A [`ServiceRun`] is a pipe or duct along a path of straight stretches,
//! at a height over its level, of a size and of a system (cold or hot
//! water, drainage, heating; supply, return or exhaust air). A
//! [`ServiceDevice`] stands at a point. IFC: `IfcPipeSegment` and
//! `IfcDuctSegment` (one per stretch), `IfcUnitaryEquipment`,
//! `IfcLightFixture`, `IfcOutlet` and `IfcSwitchingDevice`, grouped in an
//! `IfcDistributionSystem` per system.

use super::*;

/// What a run carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ServiceSystem {
    ColdWater,
    HotWater,
    Drainage,
    Heating,
    SupplyAir,
    ReturnAir,
    ExhaustAir,
}

impl ServiceSystem {
    pub const PIPES: [ServiceSystem; 4] = [
        ServiceSystem::ColdWater,
        ServiceSystem::HotWater,
        ServiceSystem::Drainage,
        ServiceSystem::Heating,
    ];
    pub const DUCTS: [ServiceSystem; 3] = [
        ServiceSystem::SupplyAir,
        ServiceSystem::ReturnAir,
        ServiceSystem::ExhaustAir,
    ];

    pub fn is_duct(self) -> bool {
        Self::DUCTS.contains(&self)
    }

    pub fn code(self) -> &'static str {
        match self {
            ServiceSystem::ColdWater => "cold_water",
            ServiceSystem::HotWater => "hot_water",
            ServiceSystem::Drainage => "drainage",
            ServiceSystem::Heating => "heating",
            ServiceSystem::SupplyAir => "supply_air",
            ServiceSystem::ReturnAir => "return_air",
            ServiceSystem::ExhaustAir => "exhaust_air",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        Self::PIPES
            .into_iter()
            .chain(Self::DUCTS)
            .find(|s| s.code() == c)
    }

    pub fn name(self) -> &'static str {
        match self {
            ServiceSystem::ColdWater => "Agua fría",
            ServiceSystem::HotWater => "Agua caliente",
            ServiceSystem::Drainage => "Saneamiento",
            ServiceSystem::Heating => "Calefacción",
            ServiceSystem::SupplyAir => "Impulsión",
            ServiceSystem::ReturnAir => "Retorno",
            ServiceSystem::ExhaustAir => "Extracción",
        }
    }

    /// Its colour in plan and 3D, as the usual schemes draw them.
    pub fn color(self) -> [u8; 3] {
        match self {
            ServiceSystem::ColdWater => [30, 110, 220],
            ServiceSystem::HotWater => [215, 45, 45],
            ServiceSystem::Drainage => [125, 85, 50],
            ServiceSystem::Heating => [235, 130, 20],
            ServiceSystem::SupplyAir => [70, 170, 210],
            ServiceSystem::ReturnAir => [80, 160, 90],
            ServiceSystem::ExhaustAir => [130, 130, 130],
        }
    }

    /// The `IfcDistributionSystem` predefined type.
    pub fn ifc_system(self) -> &'static str {
        match self {
            ServiceSystem::ColdWater => "DOMESTICCOLDWATER",
            ServiceSystem::HotWater => "DOMESTICHOTWATER",
            ServiceSystem::Drainage => "DRAINAGE",
            ServiceSystem::Heating => "HEATING",
            ServiceSystem::SupplyAir | ServiceSystem::ReturnAir => "VENTILATION",
            ServiceSystem::ExhaustAir => "EXHAUST",
        }
    }
}

/// A pipe or duct.
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceRun {
    pub id: RunId,
    pub guid: GlobalId,
    pub level: LevelId,
    pub system: ServiceSystem,
    /// Its axis in plan, two points at least.
    pub path: Vec<Point2>,
    /// Height of its axis over the level.
    pub elevation: f64,
    /// A pipe's diameter, a duct's width.
    pub width: f64,
    /// A duct's height (a pipe's is its diameter).
    pub height: f64,
}

/// Data needed to make a run.
#[derive(Debug, Clone, PartialEq)]
pub struct NewRun {
    pub level: LevelId,
    pub system: ServiceSystem,
    pub path: Vec<Point2>,
    pub elevation: f64,
    pub width: f64,
    pub height: f64,
}

impl ServiceRun {
    pub fn length(&self) -> f64 {
        self.path.windows(2).map(|w| w[0].distance(w[1])).sum()
    }

    pub fn is_duct(&self) -> bool {
        self.system.is_duct()
    }

    /// «Ø 25 mm», «400 × 250 mm».
    pub fn size_text(&self) -> String {
        let mm = |v: f64| format!("{:.0}", v * 1000.0);
        if self.is_duct() {
            format!("{} × {} mm", mm(self.width), mm(self.height))
        } else {
            format!("Ø {} mm", mm(self.width))
        }
    }

    /// Its outline in plan: the width along the path.
    pub fn outline(&self) -> Vec<Point2> {
        strip(&self.path, -self.width / 2.0, self.width / 2.0)
    }
}

/// What a device is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeviceKind {
    /// A boiler, heat pump, air handler…: a box.
    Equipment,
    Light,
    Socket,
    Switch,
}

impl DeviceKind {
    pub const ALL: [DeviceKind; 4] = [
        DeviceKind::Equipment,
        DeviceKind::Light,
        DeviceKind::Socket,
        DeviceKind::Switch,
    ];

    pub fn code(self) -> &'static str {
        match self {
            DeviceKind::Equipment => "equipment",
            DeviceKind::Light => "light",
            DeviceKind::Socket => "socket",
            DeviceKind::Switch => "switch",
        }
    }

    pub fn from_code(c: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.code() == c)
    }

    pub fn name(self) -> &'static str {
        match self {
            DeviceKind::Equipment => "Aparato",
            DeviceKind::Light => "Luminaria",
            DeviceKind::Socket => "Enchufe",
            DeviceKind::Switch => "Interruptor",
        }
    }

    /// Its usual size (width, depth, height) and mounting height over the
    /// floor, metres.
    pub fn defaults(self) -> (f64, f64, f64, f64) {
        match self {
            DeviceKind::Equipment => (0.6, 0.4, 0.8, 1.2),
            DeviceKind::Light => (0.3, 0.3, 0.1, 2.6),
            DeviceKind::Socket => (0.08, 0.04, 0.08, 0.3),
            DeviceKind::Switch => (0.08, 0.04, 0.08, 1.1),
        }
    }
}

/// A device at a point.
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceDevice {
    pub id: DeviceId,
    pub guid: GlobalId,
    pub level: LevelId,
    pub kind: DeviceKind,
    pub name: String,
    pub position: Point2,
    /// Turn, radians.
    pub rotation: f64,
    /// Height of its bottom over the level.
    pub elevation: f64,
    pub width: f64,
    pub depth: f64,
    pub height: f64,
}

/// Data needed to place a device.
#[derive(Debug, Clone, PartialEq)]
pub struct NewDevice {
    pub level: LevelId,
    pub kind: DeviceKind,
    pub name: String,
    pub position: Point2,
    pub rotation: f64,
    pub elevation: f64,
    pub width: f64,
    pub depth: f64,
    pub height: f64,
}

impl ServiceDevice {
    /// Its box in plan.
    pub fn outline(&self) -> Vec<Point2> {
        let (c, s) = (self.rotation.cos(), self.rotation.sin());
        let (hx, hy) = (self.width / 2.0, self.depth / 2.0);
        [(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)]
            .into_iter()
            .map(|(x, y)| {
                Point2::new(
                    self.position.x + x * c - y * s,
                    self.position.y + x * s + y * c,
                )
            })
            .collect()
    }
}

impl Model {
    // ---- Runs ---------------------------------------------------------------

    fn validate_run(&self, r: &NewRun) -> Result<(), ModelError> {
        if !self.levels.contains_key(&r.level) {
            return Err(ModelError::UnknownLevel(r.level));
        }
        for q in &r.path {
            finite_point(*q, "run path")?;
        }
        if r.path.len() < 2 || r.path.windows(2).any(|w| w[0].approx_eq(w[1])) {
            return Err(ModelError::InvalidShape(
                "a pipe or duct needs a path of two different points or more",
            ));
        }
        for (v, what) in [(r.width, "run width"), (r.height, "run height")] {
            finite(v, what)?;
            if v <= EPSILON {
                return Err(ModelError::NonPositive(what));
            }
        }
        finite(r.elevation, "run elevation")?;
        Ok(())
    }

    pub fn add_run(&mut self, spec: NewRun) -> Result<RunId, ModelError> {
        let mut spec = spec;
        if !spec.system.is_duct() {
            spec.height = spec.width;
        }
        self.validate_run(&spec)?;
        let id = RunId(self.take_id());
        self.runs.insert(
            id,
            ServiceRun {
                id,
                guid: GlobalId::new(),
                level: spec.level,
                system: spec.system,
                path: spec.path,
                elevation: spec.elevation,
                width: spec.width,
                height: spec.height,
            },
        );
        Ok(id)
    }

    pub fn edit_run(
        &mut self,
        id: RunId,
        edit: impl FnOnce(&mut ServiceRun),
    ) -> Result<(), ModelError> {
        let mut r = self
            .runs
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        edit(&mut r);
        if !r.system.is_duct() {
            r.height = r.width;
        }
        self.validate_run(&NewRun {
            level: r.level,
            system: r.system,
            path: r.path.clone(),
            elevation: r.elevation,
            width: r.width,
            height: r.height,
        })?;
        self.runs.insert(id, r);
        Ok(())
    }

    pub fn remove_run(&mut self, id: RunId) -> Result<ServiceRun, ModelError> {
        self.runs
            .remove(&id)
            .ok_or(ModelError::UnknownElement(id.0))
    }

    pub fn run(&self, id: RunId) -> Option<&ServiceRun> {
        self.runs.get(&id)
    }

    pub fn runs(&self) -> impl Iterator<Item = &ServiceRun> {
        self.runs.values()
    }

    pub fn runs_on_level(&self, level: LevelId) -> impl Iterator<Item = &ServiceRun> {
        self.runs.values().filter(move |r| r.level == level)
    }

    // ---- Devices ------------------------------------------------------------

    fn validate_device(&self, d: &NewDevice) -> Result<(), ModelError> {
        if !self.levels.contains_key(&d.level) {
            return Err(ModelError::UnknownLevel(d.level));
        }
        finite_point(d.position, "device position")?;
        finite(d.rotation, "device rotation")?;
        finite(d.elevation, "device elevation")?;
        for (v, what) in [
            (d.width, "device width"),
            (d.depth, "device depth"),
            (d.height, "device height"),
        ] {
            finite(v, what)?;
            if v <= EPSILON {
                return Err(ModelError::NonPositive(what));
            }
        }
        Ok(())
    }

    pub fn add_device(&mut self, spec: NewDevice) -> Result<DeviceId, ModelError> {
        self.validate_device(&spec)?;
        let id = DeviceId(self.take_id());
        self.devices.insert(
            id,
            ServiceDevice {
                id,
                guid: GlobalId::new(),
                level: spec.level,
                kind: spec.kind,
                name: spec.name,
                position: spec.position,
                rotation: spec.rotation,
                elevation: spec.elevation,
                width: spec.width,
                depth: spec.depth,
                height: spec.height,
            },
        );
        Ok(id)
    }

    pub fn edit_device(
        &mut self,
        id: DeviceId,
        edit: impl FnOnce(&mut ServiceDevice),
    ) -> Result<(), ModelError> {
        let mut d = self
            .devices
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        edit(&mut d);
        self.validate_device(&NewDevice {
            level: d.level,
            kind: d.kind,
            name: d.name.clone(),
            position: d.position,
            rotation: d.rotation,
            elevation: d.elevation,
            width: d.width,
            depth: d.depth,
            height: d.height,
        })?;
        self.devices.insert(id, d);
        Ok(())
    }

    pub fn remove_device(&mut self, id: DeviceId) -> Result<ServiceDevice, ModelError> {
        self.devices
            .remove(&id)
            .ok_or(ModelError::UnknownElement(id.0))
    }

    pub fn device(&self, id: DeviceId) -> Option<&ServiceDevice> {
        self.devices.get(&id)
    }

    pub fn devices(&self) -> impl Iterator<Item = &ServiceDevice> {
        self.devices.values()
    }

    pub fn devices_on_level(&self, level: LevelId) -> impl Iterator<Item = &ServiceDevice> {
        self.devices.values().filter(move |d| d.level == level)
    }

    /// The metres of each system and size, for the measurements:
    /// (system, size text, metres), by system and size.
    pub fn run_lengths(&self) -> Vec<(ServiceSystem, String, f64)> {
        let mut by: std::collections::BTreeMap<(ServiceSystem, String), f64> =
            std::collections::BTreeMap::new();
        for r in self.runs.values() {
            *by.entry((r.system, r.size_text())).or_default() += r.length();
        }
        by.into_iter().map(|((s, t), l)| (s, t, l)).collect()
    }

    pub(super) fn restore_services(
        &mut self,
        runs: Vec<ServiceRun>,
        devices: Vec<ServiceDevice>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for r in runs {
            let spec = NewRun {
                level: r.level,
                system: r.system,
                path: r.path.clone(),
                elevation: r.elevation,
                width: r.width,
                height: r.height,
            };
            claim(r.id.0, Some(&r.guid))
                .and_then(|_| self.validate_run(&spec))
                .map_err(|error| RestoreError {
                    element: format!("run {}", r.id.0),
                    error,
                })?;
            self.runs.insert(r.id, r);
        }
        for d in devices {
            let spec = NewDevice {
                level: d.level,
                kind: d.kind,
                name: d.name.clone(),
                position: d.position,
                rotation: d.rotation,
                elevation: d.elevation,
                width: d.width,
                depth: d.depth,
                height: d.height,
            };
            claim(d.id.0, Some(&d.guid))
                .and_then(|_| self.validate_device(&spec))
                .map_err(|error| RestoreError {
                    element: format!("device {} «{}»", d.id.0, d.name),
                    error,
                })?;
            self.devices.insert(d.id, d);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_and_devices() {
        let mut m = Model::new("Instalaciones");
        let l = m.add_level("Planta baja", 0.0);
        let pipe = m
            .add_run(NewRun {
                level: l,
                system: ServiceSystem::ColdWater,
                path: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(4.0, 0.0),
                    Point2::new(4.0, 3.0),
                ],
                elevation: 0.5,
                width: 0.025,
                height: 1.0,
            })
            .unwrap();
        let r = m.run(pipe).unwrap();
        // A pipe is round: its height is its diameter.
        assert_eq!(r.height, 0.025);
        assert!((r.length() - 7.0).abs() < 1e-9);
        assert_eq!(r.size_text(), "Ø 25 mm");
        m.add_run(NewRun {
            level: l,
            system: ServiceSystem::SupplyAir,
            path: vec![Point2::new(0.0, 5.0), Point2::new(6.0, 5.0)],
            elevation: 2.7,
            width: 0.4,
            height: 0.25,
        })
        .unwrap();
        assert_eq!(m.run_lengths().len(), 2);
        let (w, d, h, z) = DeviceKind::Light.defaults();
        let dev = m
            .add_device(NewDevice {
                level: l,
                kind: DeviceKind::Light,
                name: "Downlight".into(),
                position: Point2::new(2.0, 2.0),
                rotation: 0.0,
                elevation: z,
                width: w,
                depth: d,
                height: h,
            })
            .unwrap();
        assert_eq!(m.device(dev).unwrap().outline().len(), 4);
        assert!(m
            .edit_run(pipe, |r| r.path = vec![Point2::new(0.0, 0.0)])
            .is_err());
        assert_eq!(
            ServiceSystem::from_code("hot_water"),
            Some(ServiceSystem::HotWater)
        );
    }
}
