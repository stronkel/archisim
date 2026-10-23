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
    /// Fall along the path, from its first point to its last (0,02: 2 %):
    /// a drain's slope. `elevation` is the height at the first point.
    pub slope: f64,
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
    pub slope: f64,
}

impl ServiceRun {
    /// Its length in plan.
    pub fn length(&self) -> f64 {
        self.path.windows(2).map(|w| w[0].distance(w[1])).sum()
    }

    /// Its length along its slope.
    pub fn true_length(&self) -> f64 {
        self.length() * (1.0 + self.slope * self.slope).sqrt()
    }

    /// Height of its axis over the level at a distance along its path.
    pub fn z_at(&self, distance: f64) -> f64 {
        self.elevation - self.slope * distance
    }

    /// Height of its axis over the level at each point of its path.
    pub fn point_elevations(&self) -> Vec<f64> {
        let mut s = 0.0;
        let mut out = Vec::with_capacity(self.path.len());
        for (i, p) in self.path.iter().enumerate() {
            if i > 0 {
                s += self.path[i - 1].distance(*p);
            }
            out.push(self.z_at(s));
        }
        out
    }

    /// Height of its axis over the level at its last point.
    pub fn end_elevation(&self) -> f64 {
        self.z_at(self.length())
    }

    /// The point of its axis nearest to `p`: (distance along the path,
    /// distance from `p`).
    pub fn locate(&self, p: Point2) -> (f64, f64) {
        let mut best = (0.0, f64::INFINITY);
        let mut s = 0.0;
        for w in self.path.windows(2) {
            let d = w[1] - w[0];
            let len = d.length();
            let t = if len > EPSILON {
                ((p - w[0]).dot(d) / (len * len)).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let q = w[0] + d * t;
            let off = q.distance(p);
            if off < best.1 {
                best = (s + t * len, off);
            }
            s += len;
        }
        best
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

/// What a fitting does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FittingKind {
    /// Where a run turns, or two runs meet end to end at an angle.
    Elbow,
    /// Where a run ends on another one of its system.
    Tee,
}

impl FittingKind {
    pub fn name(self) -> &'static str {
        match self {
            FittingKind::Elbow => "Codo",
            FittingKind::Tee => "Te",
        }
    }
}

/// A fitting of the pipes and ducts: not drawn by anyone, it follows from
/// their paths (see [`Model::run_fittings`]).
#[derive(Debug, Clone, PartialEq)]
pub struct RunFitting {
    pub kind: FittingKind,
    /// The run it belongs to (a tee, to the run that goes through).
    pub run: RunId,
    /// The other run, when it joins two.
    pub other: Option<RunId>,
    pub position: Point2,
    /// Height of its centre over the level.
    pub elevation: f64,
    /// The direction the run comes from and the one it leaves by (a tee:
    /// along the main run, and the branch).
    pub from: Point2,
    pub to: Point2,
    /// The same in every export: worked out from the runs' identifiers.
    pub guid: GlobalId,
}

/// Turns of less than this are not elbows, radians (5°).
const ELBOW_MIN_TURN: f64 = 0.087;

fn fitting_guid(a: &GlobalId, tag: &str) -> GlobalId {
    let mut h: u128 = 0x6c62272e07bb014262b821756295c58d;
    for x in a.as_str().bytes().chain(*b"/fitting/").chain(tag.bytes()) {
        h ^= x as u128;
        h = h.wrapping_mul(0x0000000001000000000000000000013B);
    }
    GlobalId::from_u128(h)
}

fn turn(a: Point2, b: Point2) -> f64 {
    a.cross(b).atan2(a.dot(b)).abs()
}

/// What a device is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeviceKind {
    /// A boiler, heat pump, air handler…: a box.
    Equipment,
    Light,
    Socket,
    Switch,
    /// A radiator of the heating, on a wall.
    Radiator,
    /// A grille of the air, on a wall.
    Grille,
    /// A diffuser of the air, in the ceiling.
    Diffuser,
}

impl DeviceKind {
    pub const ALL: [DeviceKind; 7] = [
        DeviceKind::Equipment,
        DeviceKind::Light,
        DeviceKind::Socket,
        DeviceKind::Switch,
        DeviceKind::Radiator,
        DeviceKind::Grille,
        DeviceKind::Diffuser,
    ];

    pub fn code(self) -> &'static str {
        match self {
            DeviceKind::Equipment => "equipment",
            DeviceKind::Light => "light",
            DeviceKind::Socket => "socket",
            DeviceKind::Switch => "switch",
            DeviceKind::Radiator => "radiator",
            DeviceKind::Grille => "grille",
            DeviceKind::Diffuser => "diffuser",
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
            DeviceKind::Radiator => "Radiador",
            DeviceKind::Grille => "Rejilla",
            DeviceKind::Diffuser => "Difusor",
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
            DeviceKind::Radiator => (0.8, 0.1, 0.6, 0.15),
            DeviceKind::Grille => (0.4, 0.05, 0.2, 2.2),
            DeviceKind::Diffuser => (0.6, 0.6, 0.05, 2.6),
        }
    }

    /// The system it is usually fed by, if any.
    pub fn system(self) -> Option<ServiceSystem> {
        match self {
            DeviceKind::Radiator => Some(ServiceSystem::Heating),
            DeviceKind::Grille => Some(ServiceSystem::ReturnAir),
            DeviceKind::Diffuser => Some(ServiceSystem::SupplyAir),
            _ => None,
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
        finite(r.slope, "run slope")?;
        if !(0.0..=1.0).contains(&r.slope) {
            return Err(ModelError::InvalidShape(
                "a pipe's slope goes from 0 to 100 %",
            ));
        }
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
                slope: spec.slope,
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
            slope: r.slope,
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

    /// The elbows and tees of the pipes and ducts, worked out from their
    /// paths: an elbow where a run turns (or two of the same system meet
    /// end to end at an angle), a tee where a run ends on another.
    pub fn run_fittings(&self) -> Vec<RunFitting> {
        let mut out = Vec::new();
        let runs: Vec<&ServiceRun> = self.runs.values().collect();
        for r in &runs {
            let z = r.point_elevations();
            for (k, w) in r.path.windows(3).enumerate() {
                let i = k + 1;
                let a = (w[1] - w[0]).normalized();
                let b = (w[2] - w[1]).normalized();
                if let (Some(a), Some(b)) = (a, b) {
                    if turn(a, b) > ELBOW_MIN_TURN {
                        out.push(RunFitting {
                            kind: FittingKind::Elbow,
                            run: r.id,
                            other: None,
                            position: w[1],
                            elevation: z[i],
                            from: a,
                            to: b,
                            guid: fitting_guid(&r.guid, &format!("elbow/{i}")),
                        });
                    }
                }
            }
        }
        // The ends of each run: point, height, direction away from the run.
        let ends = |r: &ServiceRun| -> [(Point2, f64, Option<Point2>); 2] {
            let n = r.path.len();
            [
                (r.path[0], r.elevation, (r.path[0] - r.path[1]).normalized()),
                (
                    r.path[n - 1],
                    r.end_elevation(),
                    (r.path[n - 1] - r.path[n - 2]).normalized(),
                ),
            ]
        };
        for a in &runs {
            for (e, (p, za, dir_a)) in ends(a).into_iter().enumerate() {
                for b in &runs {
                    if a.id == b.id || a.level != b.level || a.system != b.system {
                        continue;
                    }
                    let reach = (a.width.max(b.width) / 2.0).max(0.02) + 0.01;
                    let (s, off) = b.locate(p);
                    if off > reach {
                        continue;
                    }
                    if (b.z_at(s) - za).abs() > a.height.max(b.height) + 0.05 {
                        continue;
                    }
                    let at_end = s < reach || s > b.length() - reach;
                    if !at_end {
                        // A tee on `b`.
                        let main = segment_dir(b, s);
                        if let (Some(main), Some(d)) = (main, dir_a) {
                            out.push(RunFitting {
                                kind: FittingKind::Tee,
                                run: b.id,
                                other: Some(a.id),
                                position: p,
                                elevation: b.z_at(s),
                                from: main,
                                to: -d,
                                guid: fitting_guid(
                                    &b.guid,
                                    &format!("tee/{}/{e}", a.guid.as_str()),
                                ),
                            });
                        }
                    } else if a.id < b.id {
                        // End to end: an elbow if they turn.
                        let eb = if s < reach { 0 } else { 1 };
                        let (_, _, dir_b) = ends(b)[eb];
                        if let (Some(da), Some(db)) = (dir_a, dir_b) {
                            // Coming along `a` towards its end, leaving
                            // along `b` away from its own.
                            if turn(da, -db) > ELBOW_MIN_TURN {
                                out.push(RunFitting {
                                    kind: FittingKind::Elbow,
                                    run: a.id,
                                    other: Some(b.id),
                                    position: p,
                                    elevation: za,
                                    from: da,
                                    to: -db,
                                    guid: fitting_guid(
                                        &a.guid,
                                        &format!("join/{}/{e}{eb}", b.guid.as_str()),
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// The fittings of each system, kind and size, for the measurements.
    pub fn fitting_counts(&self) -> Vec<(ServiceSystem, FittingKind, String, usize)> {
        let mut by: std::collections::BTreeMap<(ServiceSystem, FittingKind, String), usize> =
            std::collections::BTreeMap::new();
        for f in self.run_fittings() {
            if let Some(r) = self.runs.get(&f.run) {
                *by.entry((r.system, f.kind, r.size_text())).or_default() += 1;
            }
        }
        by.into_iter().map(|((s, k, t), n)| (s, k, t, n)).collect()
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
                slope: r.slope,
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

/// The direction of a run's path at a distance along it.
fn segment_dir(r: &ServiceRun, s: f64) -> Option<Point2> {
    let mut acc = 0.0;
    for w in r.path.windows(2) {
        let len = w[0].distance(w[1]);
        if s <= acc + len + EPSILON {
            return (w[1] - w[0]).normalized();
        }
        acc += len;
    }
    let n = r.path.len();
    (r.path[n - 1] - r.path[n - 2]).normalized()
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
                slope: 0.0,
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
            slope: 0.0,
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

    #[test]
    fn drains_fall_and_fittings_follow_from_the_paths() {
        let mut m = Model::new("Saneamiento");
        let l = m.add_level("Planta baja", 0.0);
        let main = m
            .add_run(NewRun {
                level: l,
                system: ServiceSystem::Drainage,
                path: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(5.0, 0.0),
                    Point2::new(5.0, 3.0),
                ],
                elevation: 0.3,
                width: 0.11,
                height: 0.11,
                slope: 0.02,
            })
            .unwrap();
        let r = m.run(main).unwrap().clone();
        assert!((r.end_elevation() - (0.3 - 0.02 * 8.0)).abs() < 1e-9);
        assert_eq!(r.point_elevations().len(), 3);
        assert!(r.true_length() > r.length());
        // A branch ending on the first stretch: a tee.
        let branch = m
            .add_run(NewRun {
                level: l,
                system: ServiceSystem::Drainage,
                path: vec![Point2::new(2.0, 2.0), Point2::new(2.0, 0.0)],
                elevation: 0.3,
                width: 0.04,
                height: 0.04,
                slope: 0.02,
            })
            .unwrap();
        // A cold water pipe crossing does not join it.
        m.add_run(NewRun {
            level: l,
            system: ServiceSystem::ColdWater,
            path: vec![Point2::new(1.0, 0.0), Point2::new(1.0, 3.0)],
            elevation: 0.3,
            width: 0.02,
            height: 0.02,
            slope: 0.0,
        })
        .unwrap();
        // One continuing the main one at an angle: an elbow.
        m.add_run(NewRun {
            level: l,
            system: ServiceSystem::Drainage,
            path: vec![Point2::new(5.0, 3.0), Point2::new(8.0, 3.0)],
            elevation: r.end_elevation(),
            width: 0.11,
            height: 0.11,
            slope: 0.02,
        })
        .unwrap();
        let f = m.run_fittings();
        let elbows = f.iter().filter(|f| f.kind == FittingKind::Elbow).count();
        let tees: Vec<_> = f.iter().filter(|f| f.kind == FittingKind::Tee).collect();
        assert_eq!(elbows, 2, "{f:?}");
        assert_eq!(tees.len(), 1);
        assert_eq!(tees[0].run, main);
        assert_eq!(tees[0].other, Some(branch));
        assert!(tees[0].position.approx_eq(Point2::new(2.0, 0.0)));
        // The same identifiers every time.
        assert_eq!(m.run_fittings(), f);
        assert_eq!(m.fitting_counts().len(), 2);
        assert!(m.edit_run(main, |r| r.slope = -0.01).is_err());
    }
}
