//! Drawing pipes quickly (instalaciones, segunda vuelta): what a pipe's
//! ends join (another pipe of its system, a device that serves it, a
//! sanitary appliance), how it fits them (the height of the pipe it
//! starts from, a drain falling towards what it drains into), how it goes
//! up or down a floor (a riser or a downpipe where it changes level), and
//! the branches drawn by themselves from each appliance to the nearest
//! pipe, riser or downpipe.
//!
//! The sanitary appliances are objects: [`Appliance::of`] tells them by
//! the model ArchiSim makes for them or by their type's name.

use super::*;

fn lerp(a: Point2, b: Point2, t: f64) -> Point2 {
    a + (b - a) * t
}

/// A sanitary appliance, as the tables of DB-HS name them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Appliance {
    Washbasin,
    Bidet,
    Shower,
    Bath,
    Wc,
    Sink,
    Dishwasher,
    Washer,
}

impl Appliance {
    pub const ALL: [Appliance; 8] = [
        Appliance::Washbasin,
        Appliance::Bidet,
        Appliance::Shower,
        Appliance::Bath,
        Appliance::Wc,
        Appliance::Sink,
        Appliance::Dishwasher,
        Appliance::Washer,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Appliance::Washbasin => "Lavabo",
            Appliance::Bidet => "Bidé",
            Appliance::Shower => "Ducha",
            Appliance::Bath => "Bañera",
            Appliance::Wc => "Inodoro con cisterna",
            Appliance::Sink => "Fregadero",
            Appliance::Dishwasher => "Lavavajillas",
            Appliance::Washer => "Lavadora",
        }
    }

    /// HS 5, tabla 4.1: units of discharge and the minimum diameter of its
    /// trap and its branch, private use (mm).
    pub fn drain(self) -> (u32, f64) {
        match self {
            Appliance::Washbasin => (1, 32.0),
            Appliance::Bidet => (2, 32.0),
            Appliance::Shower => (2, 40.0),
            Appliance::Bath => (3, 40.0),
            Appliance::Wc => (4, 100.0),
            Appliance::Sink | Appliance::Dishwasher | Appliance::Washer => (3, 40.0),
        }
    }

    /// HS 4, tabla 4.2: the minimum diameter of its supply, copper or
    /// plastic (mm).
    pub fn supply(self) -> f64 {
        match self {
            Appliance::Bath | Appliance::Washer => 20.0,
            _ => 12.0,
        }
    }

    /// HS 4, tabla 2.1: the minimum flows of cold and hot water, l/s.
    pub fn flows(self) -> (f64, f64) {
        match self {
            Appliance::Washbasin => (0.10, 0.065),
            Appliance::Bidet => (0.10, 0.065),
            Appliance::Shower => (0.20, 0.10),
            Appliance::Bath => (0.30, 0.20),
            Appliance::Wc => (0.10, 0.0),
            Appliance::Sink => (0.20, 0.10),
            Appliance::Dishwasher => (0.15, 0.10),
            Appliance::Washer => (0.20, 0.15),
        }
    }

    /// Whether it takes hot water.
    pub fn hot(self) -> bool {
        !matches!(self, Appliance::Wc | Appliance::Washer)
    }

    /// The outer diameter of the pipe drawn for its branch, metres: the
    /// usual plastic pipe that meets the code's minimum.
    pub fn branch_pipe(self, system: ServiceSystem) -> f64 {
        if system == ServiceSystem::Drainage {
            match self.drain().1 as u32 {
                100 => 0.110,
                40 => 0.040,
                _ => 0.032,
            }
        } else if self.supply() >= 20.0 {
            0.020
        } else {
            0.016
        }
    }

    /// What an object is, by the model ArchiSim makes for it or by the
    /// name of its type.
    pub fn of(model: &Model, id: ObjectId) -> Option<Appliance> {
        let o = model.objects().find(|o| o.id == id)?;
        let t = model.object_type(o.object_type)?;
        if t.mesh.is_none() {
            if let Some(kind) = model.family_prop(t.family) {
                let a = match kind.as_str() {
                    "wc" => Some(Appliance::Wc),
                    "bidet" => Some(Appliance::Bidet),
                    "washbasin" => Some(Appliance::Washbasin),
                    "bath" => Some(Appliance::Bath),
                    "shower" => Some(Appliance::Shower),
                    "counter" => Some(Appliance::Sink),
                    "washer" => Some(Appliance::Washer),
                    _ => None,
                };
                if a.is_some() {
                    return a;
                }
            }
        }
        let name = t.name.to_lowercase();
        [
            ("inodoro", Appliance::Wc),
            ("bidé", Appliance::Bidet),
            ("bide", Appliance::Bidet),
            ("lavabo", Appliance::Washbasin),
            ("lavamanos", Appliance::Washbasin),
            ("bañera", Appliance::Bath),
            ("ducha", Appliance::Shower),
            ("fregadero", Appliance::Sink),
            ("lavavajillas", Appliance::Dishwasher),
            ("lavadora", Appliance::Washer),
        ]
        .into_iter()
        .find(|(k, _)| name.contains(k))
        .map(|(_, a)| a)
    }
}

/// A sanitary appliance placed in the model, with the point its pipes go
/// to: the middle of its back (against the wall), or its centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacedAppliance {
    pub object: ObjectId,
    pub level: LevelId,
    pub kind: Appliance,
    pub point: Point2,
}

impl DeviceKind {
    /// Whether a pipe or duct of `system` ends at this device: a riser of
    /// the water and heating, a downpipe of the drainage, a heater or a
    /// meter of the water, a radiator of the heating, a grille of the air.
    pub fn serves(self, system: ServiceSystem) -> bool {
        use DeviceKind as D;
        use ServiceSystem as S;
        match self {
            D::Equipment => !system.is_urban(),
            D::Riser => matches!(
                system,
                S::ColdWater | S::HotWater | S::Heating | S::FireProtection
            ),
            D::Downpipe => matches!(system, S::Drainage | S::Rainwater),
            D::WaterHeater | D::Valve | D::WaterMeter | D::Tank | D::Pump => {
                matches!(system, S::ColdWater | S::HotWater)
            }
            D::Radiator => system == S::Heating,
            D::Grille | D::Diffuser => system.is_duct(),
            D::Sprinkler | D::HoseReel => system == S::FireProtection,
            D::SolarCollector => matches!(system, S::HotWater | S::Heating),
            _ => false,
        }
    }
}

/// What a pipe's end joins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JoinTo {
    /// Another run of its system, at that distance along it.
    Run(RunId, f64),
    Device(DeviceId),
    Appliance(ObjectId),
}

/// A point a pipe being drawn takes: where it joins, and at what height
/// over the level when that is known (another pipe's axis there).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Join {
    pub point: Point2,
    pub to: JoinTo,
    pub z: Option<f64>,
}

/// How near a pipe's end must be to what it joins, metres.
pub const JOIN_REACH: f64 = 0.05;

/// The usual fall of a drain (HS 5): branches to the appliances 2,5 %,
/// collectors 2 %.
pub fn usual_drain_slope(diameter: f64) -> f64 {
    if diameter < 0.075 {
        0.025
    } else {
        0.02
    }
}

fn crosses(a: Point2, b: Point2, c: Point2, d: Point2) -> bool {
    let r = b - a;
    let s = d - c;
    let den = r.cross(s);
    if den.abs() < 1e-12 {
        return false;
    }
    let t = (c - a).cross(s) / den;
    let u = (c - a).cross(r) / den;
    (1e-6..1.0 - 1e-6).contains(&t) && (1e-6..1.0 - 1e-6).contains(&u)
}

impl Model {
    /// The sanitary appliances of a level, with the point their pipes go
    /// to.
    pub fn appliances_on(&self, level: LevelId) -> Vec<PlacedAppliance> {
        let mut out: Vec<PlacedAppliance> = self
            .objects()
            .filter(|o| o.level == level)
            .filter_map(|o| {
                let kind = Appliance::of(self, o.id)?;
                let outline = self.object_outline(o.id);
                let point = if outline.len() >= 4 {
                    // The family's back is its −y side: the middle of the
                    // edge from its first corner to its second.
                    let back = lerp(outline[0], outline[1], 0.5);
                    let centre = crate::rooms::centroid(&outline);
                    lerp(back, centre, 0.2)
                } else {
                    o.position
                };
                Some(PlacedAppliance {
                    object: o.id,
                    level,
                    kind,
                    point,
                })
            })
            .collect();
        out.sort_by_key(|a| a.object);
        out
    }

    /// What a pipe of `system` drawn on `level` would join at `p`, within
    /// `reach`: a device that serves it (by its centre), a sanitary
    /// appliance (water and drainage), the end of another run of the
    /// system, or the nearest point of its axis (a tee). Ends and devices
    /// win over the middle of a run.
    pub fn pipe_join(
        &self,
        level: LevelId,
        system: ServiceSystem,
        p: Point2,
        reach: f64,
    ) -> Option<Join> {
        let mut best: Option<(f64, Join)> = None;
        let take = |best: &mut Option<(f64, Join)>, d: f64, j: Join| {
            if d <= reach && best.as_ref().is_none_or(|(bd, _)| d < *bd) {
                *best = Some((d, j));
            }
        };
        for d in self.devices().filter(|d| d.kind.serves(system)) {
            let on = d.level == level
                || (d.kind.is_vertical()
                    && self
                        .level(d.level)
                        .zip(self.level(level))
                        .is_some_and(|(a, b)| {
                            b.elevation >= a.elevation - EPSILON
                                && b.elevation <= a.elevation + d.elevation + d.height + EPSILON
                        }));
            if on {
                take(
                    &mut best,
                    d.position.distance(p),
                    Join {
                        point: d.position,
                        to: JoinTo::Device(d.id),
                        z: None,
                    },
                );
            }
        }
        if matches!(
            system,
            ServiceSystem::ColdWater | ServiceSystem::HotWater | ServiceSystem::Drainage
        ) {
            for a in self.appliances_on(level) {
                if system == ServiceSystem::HotWater && !a.kind.hot() {
                    continue;
                }
                take(
                    &mut best,
                    a.point.distance(p),
                    Join {
                        point: a.point,
                        to: JoinTo::Appliance(a.object),
                        z: None,
                    },
                );
            }
        }
        for r in self.runs_on_level(level).filter(|r| r.system == system) {
            let len = r.length();
            let zs = r.point_elevations();
            for (i, q) in [
                (0usize, r.path[0]),
                (r.path.len() - 1, *r.path.last().unwrap()),
            ] {
                let s = if i == 0 { 0.0 } else { len };
                take(
                    &mut best,
                    q.distance(p),
                    Join {
                        point: q,
                        to: JoinTo::Run(r.id, s),
                        z: Some(zs[i]),
                    },
                );
            }
        }
        if best.is_some() {
            return best.map(|b| b.1);
        }
        for r in self.runs_on_level(level).filter(|r| r.system == system) {
            let (s, off) = r.locate(p);
            let mut acc = 0.0;
            let mut q = r.path[0];
            for w in r.path.windows(2) {
                let l = w[0].distance(w[1]);
                if acc + l >= s - EPSILON {
                    q = lerp(w[0], w[1], ((s - acc) / l.max(EPSILON)).clamp(0.0, 1.0));
                    break;
                }
                acc += l;
            }
            take(
                &mut best,
                off,
                Join {
                    point: q,
                    to: JoinTo::Run(r.id, s),
                    z: Some(r.z_at(s)),
                },
            );
        }
        best.map(|b| b.1)
    }

    /// Fits a pipe being made to what its ends join: a level pipe takes
    /// the height of the pipe it starts from (or ends on); a drain falls
    /// towards what it drains into (a pipe of its system, a downpipe),
    /// turning its path round if it was drawn from there, and reaches the
    /// pipe it ends on at its height. Says what each end joins.
    pub fn fit_run(&self, spec: &mut NewRun) -> (Option<Join>, Option<Join>) {
        let first = *spec.path.first().unwrap_or(&Point2::new(0.0, 0.0));
        let last = *spec.path.last().unwrap_or(&first);
        let mut a = self.pipe_join(spec.level, spec.system, first, JOIN_REACH);
        let mut b = self.pipe_join(spec.level, spec.system, last, JOIN_REACH);
        if spec.system.falls() {
            let sink = |j: &Option<Join>| {
                matches!(
                    j,
                    Some(Join {
                        to: JoinTo::Run(..),
                        ..
                    })
                ) || matches!(j, Some(Join { to: JoinTo::Device(id), .. })
                    if self.device(*id).is_some_and(|d| d.kind == DeviceKind::Downpipe))
            };
            if sink(&a) && !sink(&b) {
                spec.path.reverse();
                std::mem::swap(&mut a, &mut b);
            }
            let len: f64 = spec.path.windows(2).map(|w| w[0].distance(w[1])).sum();
            if let Some(z) = b.and_then(|j| j.z) {
                spec.elevation = z + spec.slope * len;
            }
        } else if let Some(z) = a.and_then(|j| j.z).or(b.and_then(|j| j.z)) {
            spec.elevation = z;
        }
        (a, b)
    }

    /// Takes a pipe of `system` up (or down) from `level` at `p` to the
    /// next level: a riser (a downpipe for the drainage) between the two
    /// floors, standing on the lower one. Gives the device and the level
    /// the pipe goes on from.
    pub fn change_pipe_level(
        &mut self,
        level: LevelId,
        system: ServiceSystem,
        p: Point2,
        up: bool,
    ) -> Result<(DeviceId, LevelId), ModelError> {
        let levels = self.levels();
        let i = levels
            .iter()
            .position(|l| l.id == level)
            .ok_or(ModelError::UnknownLevel(level))?;
        let j = if up { i + 1 } else { i.wrapping_sub(1) };
        let Some(other) = levels.get(j) else {
            return Err(ModelError::InvalidShape(if up {
                "there is no level above"
            } else {
                "there is no level below"
            }));
        };
        let (low, high) = if up {
            (levels[i], *other)
        } else {
            (*other, levels[i])
        };
        let (low, other_id, rise) = (low.id, other.id, high.elevation - low.elevation);
        let kind = if system.falls() {
            DeviceKind::Downpipe
        } else {
            DeviceKind::Riser
        };
        let (w, d, _, _) = kind.defaults();
        let id = self.add_device(NewDevice {
            level: low,
            kind,
            name: format!("{} · {}", kind.name(), system.name().to_lowercase()),
            position: p,
            rotation: 0.0,
            elevation: 0.0,
            width: w,
            depth: d,
            height: rise.max(0.1),
        })?;
        Ok((id, other_id))
    }

    /// The branches from each sanitary appliance of `level` to the nearest
    /// pipe of `system` (a tee), or to the nearest riser (water) or
    /// downpipe (drainage): in two straight stretches, the way that crosses
    /// fewer walls, of the appliance's pipe; a drain falls 2,5 % towards
    /// where it ends (`elevation` is the height where a branch ends on a
    /// riser or a downpipe; on a pipe, that pipe's). Appliances that have
    /// a pipe of the system within 30 cm are left out. Gives the runs to
    /// make and the appliances left without anywhere to go.
    pub fn route_appliances(
        &self,
        level: LevelId,
        system: ServiceSystem,
        elevation: f64,
    ) -> (Vec<NewRun>, Vec<ObjectId>) {
        let walls: Vec<(Point2, Point2)> = self
            .walls_on_level(level)
            .map(|w| (w.start, w.end))
            .collect();
        let crossings = |path: &[Point2]| -> usize {
            path.windows(2)
                .map(|s| {
                    walls
                        .iter()
                        .filter(|w| crosses(s[0], s[1], w.0, w.1))
                        .count()
                })
                .sum()
        };
        let vertical = |d: &ServiceDevice| {
            d.kind.is_vertical() && d.kind.serves(system) && {
                d.level == level
                    || self
                        .level(d.level)
                        .zip(self.level(level))
                        .is_some_and(|(a, b)| {
                            b.elevation >= a.elevation - EPSILON
                                && b.elevation <= a.elevation + d.height + EPSILON
                        })
            }
        };
        let mut out = Vec::new();
        let mut alone = Vec::new();
        for a in self.appliances_on(level) {
            if system == ServiceSystem::HotWater && !a.kind.hot() {
                continue;
            }
            let served = self
                .runs_on_level(level)
                .any(|r| r.system == system && r.locate(a.point).1 <= 0.3);
            if served {
                continue;
            }
            // Where to: the nearest point of a pipe of the system, or the
            // nearest riser or downpipe.
            let mut to: Option<(f64, Point2, Option<f64>)> = None;
            for r in self.runs_on_level(level).filter(|r| r.system == system) {
                let (s, off) = r.locate(a.point);
                let mut acc = 0.0;
                let mut q = r.path[0];
                for w in r.path.windows(2) {
                    let l = w[0].distance(w[1]);
                    if acc + l >= s - EPSILON {
                        q = lerp(w[0], w[1], ((s - acc) / l.max(EPSILON)).clamp(0.0, 1.0));
                        break;
                    }
                    acc += l;
                }
                if to.is_none_or(|t| off < t.0) {
                    to = Some((off, q, Some(r.z_at(s))));
                }
            }
            for d in self.devices().filter(|d| vertical(d)) {
                let off = d.position.distance(a.point);
                if to.is_none_or(|t| off < t.0) {
                    to = Some((off, d.position, None));
                }
            }
            let Some((_, t, z_end)) = to else {
                alone.push(a.object);
                continue;
            };
            let c = a.point;
            let options = [
                vec![c, Point2::new(t.x, c.y), t],
                vec![c, Point2::new(c.x, t.y), t],
            ];
            let mut path = options
                .into_iter()
                .min_by_key(|p| crossings(p))
                .unwrap_or_default();
            path.dedup_by(|x, y| x.distance(*y) < 0.01);
            if path.len() < 2 {
                continue;
            }
            let width = a.kind.branch_pipe(system);
            let len: f64 = path.windows(2).map(|w| w[0].distance(w[1])).sum();
            let end_z = z_end.unwrap_or(elevation);
            let (slope, start_z) = if system.falls() {
                let s = usual_drain_slope(width);
                (s, end_z + s * len)
            } else {
                (0.0, end_z)
            };
            out.push(NewRun {
                level,
                system,
                path,
                elevation: start_z,
                width,
                height: width,
                slope,
            });
        }
        (out, alone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bathroom() -> (Model, LevelId) {
        let mut m = Model::new("Baño");
        let l0 = m.add_level("Planta baja", 0.0);
        m.add_level("Planta 1", 3.0);
        let fam = m.system_family("Inodoro").unwrap();
        let white = m.add_material("Porcelana", "ceramic", [240, 240, 240], CutPattern::Solid);
        let mut values = m.family_defaults(fam, ParamScope::Type).unwrap();
        for v in values.values_mut() {
            if let Value::Material(id) = v {
                *id = white.0;
            }
        }
        let wc = m
            .add_object_type("Inodoro suspendido", fam, values)
            .unwrap();
        m.add_object(NewObject {
            level: l0,
            object_type: wc,
            position: Point2::new(2.0, 1.0),
            rotation: 0.0,
            base_offset: 0.0,
        })
        .unwrap();
        (m, l0)
    }

    #[test]
    fn a_branch_from_the_wc_to_the_downpipe() {
        let (mut m, l0) = bathroom();
        let (w, d, h, _) = DeviceKind::Downpipe.defaults();
        m.add_device(NewDevice {
            level: l0,
            kind: DeviceKind::Downpipe,
            name: "Bajante".into(),
            position: Point2::new(0.0, 0.0),
            rotation: 0.0,
            elevation: 0.0,
            width: w,
            depth: d,
            height: h,
        })
        .unwrap();
        let apps = m.appliances_on(l0);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].kind, Appliance::Wc);
        let (runs, alone) = m.route_appliances(l0, ServiceSystem::Drainage, -0.2);
        assert!(alone.is_empty());
        assert_eq!(runs.len(), 1);
        let r = &runs[0];
        assert_eq!(r.width, 0.110);
        assert!(r.path.last().unwrap().approx_eq(Point2::new(0.0, 0.0)));
        // It falls towards the downpipe and ends at the height asked.
        let len: f64 = r.path.windows(2).map(|w| w[0].distance(w[1])).sum();
        assert!((r.elevation - r.slope * len - -0.2).abs() < 1e-9);
        for r in runs {
            m.add_run(r).unwrap();
        }
        // Done once: the second time there is nothing to draw.
        assert!(m
            .route_appliances(l0, ServiceSystem::Drainage, -0.2)
            .0
            .is_empty());
    }

    #[test]
    fn a_drain_drawn_from_the_collector_is_turned_round() {
        let (mut m, l0) = bathroom();
        m.add_run(NewRun {
            level: l0,
            system: ServiceSystem::Drainage,
            path: vec![Point2::new(0.0, -2.0), Point2::new(6.0, -2.0)],
            elevation: -0.3,
            width: 0.11,
            height: 0.11,
            slope: 0.02,
        })
        .unwrap();
        // From a point of the collector to the WC.
        let mut spec = NewRun {
            level: l0,
            system: ServiceSystem::Drainage,
            path: vec![Point2::new(3.0, -2.0), m.appliances_on(l0)[0].point],
            elevation: 0.0,
            width: 0.04,
            height: 0.04,
            slope: 0.025,
        };
        let (a, b) = m.fit_run(&mut spec);
        assert!(matches!(a.unwrap().to, JoinTo::Appliance(_)));
        assert!(matches!(b.unwrap().to, JoinTo::Run(..)));
        assert!(spec.path[1].approx_eq(Point2::new(3.0, -2.0)));
        let collector_z = -0.3 - 0.02 * 3.0;
        let len = spec.path[0].distance(spec.path[1]);
        assert!((spec.elevation - (collector_z + 0.025 * len)).abs() < 1e-9);
    }

    #[test]
    fn a_pipe_goes_up_a_floor_by_a_riser() {
        let (mut m, l0) = bathroom();
        let (dev, up) = m
            .change_pipe_level(l0, ServiceSystem::ColdWater, Point2::new(1.0, 1.0), true)
            .unwrap();
        let d = m.device(dev).unwrap();
        assert_eq!(d.kind, DeviceKind::Riser);
        assert!((d.height - 3.0).abs() < 1e-9);
        assert_ne!(up, l0);
        // A pipe on the floor above joins the riser.
        let j = m
            .pipe_join(up, ServiceSystem::ColdWater, Point2::new(1.02, 1.0), 0.05)
            .unwrap();
        assert_eq!(j.to, JoinTo::Device(dev));
        assert!(m
            .change_pipe_level(up, ServiceSystem::ColdWater, Point2::new(1.0, 1.0), true)
            .is_err());
    }
}
