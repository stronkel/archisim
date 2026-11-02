//! The networks under the street (U5): what is worked out from a run of an
//! urban network — the connections («acometidas») to the plots beside it,
//! each with its chamber at the front of the plot, the manholes along a
//! sewer, and its longitudinal profile (the street above it, its pipe and
//! the manholes, stretch by stretch).

use super::*;

/// How far apart the manholes of a sewer go, at most.
pub const MANHOLE_SPACING: f64 = 40.0;
/// How far from a run the plots it serves can be.
pub const CONNECTION_REACH: f64 = 25.0;
/// The slope of a sewer's connection, down to the sewer.
pub const CONNECTION_SLOPE: f64 = 0.02;

/// A point of a run's longitudinal profile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfileStation {
    /// Distance along the run.
    pub at: f64,
    /// Height of the street (or the ground) over the level there.
    pub ground: f64,
    /// Height of the pipe's bottom inside («cota de solera») over the level.
    pub invert: f64,
    /// Whether a manhole (or a chamber) is there.
    pub manhole: bool,
}

/// The nearest point of a path to `p`: the distance along it and the
/// point.
fn nearest_along(path: &[Point2], p: Point2) -> (f64, Point2, f64) {
    let mut best = (0.0, path[0], f64::INFINITY);
    let mut s = 0.0;
    for w in path.windows(2) {
        let (a, b) = (w[0], w[1]);
        let l = a.distance(b);
        let t = if l > EPSILON {
            ((p - a).dot(b - a) / (l * l)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let q = a + (b - a) * t;
        let d = q.distance(p);
        if d < best.2 {
            best = (s + l * t, q, d);
        }
        s += l;
    }
    best
}

impl Model {
    /// The height over the level of what is at `p`: the street's pavement
    /// or, off the streets, the ground of the level's terrain (or 0).
    pub fn surface_at(&self, level: LevelId, p: Point2) -> f64 {
        let on_street = self
            .roads_on_level(level)
            .any(|r| point_in_polygon(p, &r.outline()))
            || self
                .roundabouts_on_level(level)
                .any(|c| c.centre.distance(p) <= c.radius);
        if on_street {
            return self.pavement_top_at(level, p);
        }
        let base = self.level(level).map_or(0.0, |l| l.elevation);
        self.terrains_on_level(level)
            .find_map(|t| t.height_at(p))
            .map_or_else(|| self.pavement_top_at(level, p), |z| z - base)
    }

    /// The connections from a run of an urban network to the plots of its
    /// level beside it (within [`CONNECTION_REACH`]): for each, a chamber
    /// at the front of the plot (its boundary's nearest point to the run,
    /// 0,5 m out into the street) and a pipe from the chamber to the run,
    /// square to it; a sewer's falls [`CONNECTION_SLOPE`] to it. Plots
    /// that already have a chamber of this run's system there are left.
    pub fn connect_plots(&mut self, run: RunId) -> Result<Vec<(DeviceId, RunId)>, ModelError> {
        let r = self
            .run(run)
            .cloned()
            .ok_or(ModelError::UnknownElement(run.0))?;
        let plots: Vec<Plot> = self.plots_on_level(r.level).cloned().collect();
        let mut out = Vec::new();
        for plot in plots {
            // The point of the plot nearest to the run.
            let mut best: Option<(f64, Point2, Point2, f64)> = None;
            let o = &plot.outline;
            for i in 0..o.len() {
                let (a, b) = (o[i], o[(i + 1) % o.len()]);
                for k in 0..=8 {
                    let q = a + (b - a) * (k as f64 / 8.0);
                    let (s, on, d) = nearest_along(&r.path, q);
                    if best.is_none_or(|x| d < x.3) {
                        best = Some((s, q, on, d));
                    }
                }
            }
            let Some((s, front, on, d)) = best else {
                continue;
            };
            if !(0.6..=CONNECTION_REACH).contains(&d) {
                continue;
            }
            let Some(u) = (on - front).normalized() else {
                continue;
            };
            let at = front + u * 0.5;
            let taken = self
                .devices_on_level(r.level)
                .any(|x| x.kind == DeviceKind::Chamber && x.position.distance(at) < 0.3);
            if taken {
                continue;
            }
            let main_z = r.z_at(s);
            let len = at.distance(on);
            let (elevation, slope) = if r.system.falls() {
                (main_z + CONNECTION_SLOPE * len, CONNECTION_SLOPE)
            } else {
                (main_z, 0.0)
            };
            let size = if r.system.falls() {
                0.16
            } else {
                (r.width * 0.5).max(0.04)
            };
            let top = self.surface_at(r.level, at);
            let depth = (top - elevation + size / 2.0 + 0.15).max(0.5);
            let chamber = self.add_device(NewDevice {
                level: r.level,
                kind: DeviceKind::Chamber,
                name: format!("Acometida de {} · {}", r.system.name(), plot.name),
                position: at,
                rotation: u.y.atan2(u.x),
                elevation: top - depth,
                width: 0.5,
                depth: 0.5,
                height: depth,
            })?;
            let pipe = self.add_run(NewRun {
                level: r.level,
                system: r.system,
                path: vec![at, on],
                elevation,
                width: size,
                height: size,
                slope,
            })?;
            out.push((chamber, pipe));
        }
        Ok(out)
    }

    /// What a network's connection is joined to inside a building: the
    /// kinds of device and the building's own systems.
    fn building_ends(system: ServiceSystem) -> (&'static [DeviceKind], &'static [ServiceSystem]) {
        match system {
            ServiceSystem::Sewer => (&[DeviceKind::Downpipe], &[ServiceSystem::Drainage]),
            ServiceSystem::Stormwater => (&[DeviceKind::Downpipe], &[]),
            ServiceSystem::WaterMain => (&[DeviceKind::WaterMeter], &[ServiceSystem::ColdWater]),
            ServiceSystem::PowerLow => (&[DeviceKind::MainFuseBox, DeviceKind::Panel], &[]),
            _ => (&[], &[]),
        }
    }

    /// Joins the connections of an urban network (the chambers
    /// [`Model::connect_plots`] made) to each building's installation in
    /// its plot: the sewer to the nearest downpipe (or the building's
    /// drainage), stormwater to a downpipe, the water main to the meter
    /// (or the cold water), low voltage to the general protection box (or
    /// the panel). A pipe from the chamber to it, on the chamber's level;
    /// a sewer's falls [`CONNECTION_SLOPE`] to the chamber. Those joined
    /// already, or with nothing to join to, are left. The pipes made.
    pub fn connect_buildings(&mut self, run: RunId) -> Result<Vec<RunId>, ModelError> {
        let r = self
            .run(run)
            .cloned()
            .ok_or(ModelError::UnknownElement(run.0))?;
        let (kinds, systems) = Self::building_ends(r.system);
        if kinds.is_empty() && systems.is_empty() {
            return Ok(Vec::new());
        }
        let prefix = format!("Acometida de {}", r.system.name());
        let chambers: Vec<ServiceDevice> = self
            .devices_on_level(r.level)
            .filter(|d| d.kind == DeviceKind::Chamber && d.name.starts_with(&prefix))
            .cloned()
            .collect();
        let plots: Vec<Plot> = self.plots_on_level(r.level).cloned().collect();
        let near = |a: Point2, b: Point2| a.distance(b) < 0.05;
        let mut out = Vec::new();
        for ch in chambers {
            let at = ch.position;
            // Its plot: the one whose boundary is nearest (it stands half a
            // metre out of it).
            let Some(plot) = plots
                .iter()
                .map(|p| {
                    let o = &p.outline;
                    let d = (0..o.len())
                        .map(|i| nearest_along(&[o[i], o[(i + 1) % o.len()]], at).2)
                        .fold(f64::INFINITY, f64::min);
                    (d, p)
                })
                .filter(|(d, _)| *d < 1.0)
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map(|x| x.1)
            else {
                continue;
            };
            let inside = |q: Point2| point_in_polygon(q, &plot.outline);
            let mine = |x: &ServiceRun| x.system == r.system;
            // Joined already: a pipe of the network from the chamber into
            // the plot.
            let joined = self.runs().filter(|x| mine(x)).any(|x| {
                let (a, b) = (x.path[0], x.path[x.path.len() - 1]);
                (near(a, at) && inside(b)) || (near(b, at) && inside(a))
            });
            if joined {
                continue;
            }
            // The chamber's connection pipe, for its depth there.
            let Some(z_in) = self.runs().filter(|x| mine(x)).find_map(|x| {
                let n = x.path.len();
                if near(x.path[0], at) {
                    Some(x.z_at(0.0))
                } else if near(x.path[n - 1], at) {
                    Some(x.z_at(x.length()))
                } else {
                    None
                }
            }) else {
                continue;
            };
            // The nearest end inside the plot: a device, or a point of the
            // building's own pipes.
            let devices = self
                .devices()
                .filter(|d| kinds.contains(&d.kind) && inside(d.position))
                .map(|d| d.position);
            let pipes = self
                .runs()
                .filter(|x| systems.contains(&x.system))
                .flat_map(|x| x.path.iter().copied())
                .filter(|q| inside(*q));
            let Some(target) = devices
                .chain(pipes)
                .min_by(|a, b| a.distance(at).total_cmp(&b.distance(at)))
            else {
                continue;
            };
            let len = target.distance(at);
            if len < 0.1 {
                continue;
            }
            let (elevation, slope) = if r.system.falls() {
                (z_in + CONNECTION_SLOPE * len, CONNECTION_SLOPE)
            } else {
                (z_in, 0.0)
            };
            let size = if r.system.falls() {
                0.16
            } else {
                (r.width * 0.5).max(0.04)
            };
            out.push(self.add_run(NewRun {
                level: r.level,
                system: r.system,
                path: vec![target, at],
                elevation,
                width: size,
                height: size,
                slope,
            })?);
        }
        Ok(out)
    }

    /// Manholes along a sewer (or a stormwater pipe): one at each end, at
    /// each corner and no more than [`MANHOLE_SPACING`] apart, from its
    /// bottom up to the street, where there is not one already.
    pub fn add_manholes(&mut self, run: RunId) -> Result<Vec<DeviceId>, ModelError> {
        let r = self
            .run(run)
            .cloned()
            .ok_or(ModelError::UnknownElement(run.0))?;
        let mut stations = vec![0.0];
        let mut s = 0.0;
        for w in r.path.windows(2) {
            let l = w[0].distance(w[1]);
            let n = (l / MANHOLE_SPACING).ceil().max(1.0) as usize;
            for k in 1..=n {
                stations.push(s + l * k as f64 / n as f64);
            }
            s += l;
        }
        let mut out = Vec::new();
        for st in stations {
            let (p, _) = super::urban::axis_point(&r.path, st);
            if self
                .devices_on_level(r.level)
                .any(|x| x.kind == DeviceKind::Manhole && x.position.distance(p) < 1.0)
            {
                continue;
            }
            let top = self.surface_at(r.level, p);
            let bottom = r.z_at(st) - r.width / 2.0 - 0.2;
            let id = self.add_device(NewDevice {
                level: r.level,
                kind: DeviceKind::Manhole,
                name: format!("Pozo {}", out.len() + 1),
                position: p,
                rotation: 0.0,
                elevation: bottom,
                width: 1.2,
                depth: 1.2,
                height: (top - bottom).max(0.8),
            })?;
            out.push(id);
        }
        Ok(out)
    }

    /// A run's longitudinal profile: a station at each point of its path,
    /// every 5 m between them, and at each manhole or chamber on it.
    pub fn run_profile(&self, run: RunId) -> Vec<ProfileStation> {
        let Some(r) = self.run(run) else {
            return Vec::new();
        };
        let len = r.length();
        let mut at: Vec<f64> = Vec::new();
        let mut s = 0.0;
        at.push(0.0);
        for w in r.path.windows(2) {
            let l = w[0].distance(w[1]);
            let n = (l / 5.0).ceil().max(1.0) as usize;
            for k in 1..=n {
                at.push(s + l * k as f64 / n as f64);
            }
            s += l;
        }
        let holes: Vec<f64> = self
            .devices_on_level(r.level)
            .filter(|d| matches!(d.kind, DeviceKind::Manhole | DeviceKind::Chamber))
            .filter_map(|d| {
                let (s, _, dist) = nearest_along(&r.path, d.position);
                (dist < 1.0).then_some(s)
            })
            .collect();
        at.extend(holes.iter().copied());
        at.sort_by(f64::total_cmp);
        at.dedup_by(|a, b| (*a - *b).abs() < 0.05);
        at.into_iter()
            .filter(|x| *x <= len + 1e-6)
            .map(|x| {
                let (p, _) = super::urban::axis_point(&r.path, x);
                ProfileStation {
                    at: x,
                    ground: self.surface_at(r.level, p),
                    invert: r.z_at(x) - r.width / 2.0,
                    manhole: holes.iter().any(|h| (h - x).abs() < 0.05),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sewer_with_its_manholes_connections_and_profile() {
        let mut m = Model::new("P");
        let lv = m.add_level("PB", 0.0);
        let sewer = m
            .add_run(NewRun {
                level: lv,
                system: ServiceSystem::Sewer,
                path: vec![Point2::new(0.0, 0.0), Point2::new(100.0, 0.0)],
                elevation: -1.8,
                width: 0.315,
                height: 0.315,
                slope: 0.01,
            })
            .unwrap();
        for (i, x) in [10.0, 40.0, 70.0].into_iter().enumerate() {
            m.add_plot(NewPlot {
                name: format!("P{i}"),
                level: lv,
                outline: vec![
                    Point2::new(x, 8.0),
                    Point2::new(x + 20.0, 8.0),
                    Point2::new(x + 20.0, 30.0),
                    Point2::new(x, 30.0),
                ],
                setback: 0.0,
                reference: String::new(),
            })
            .unwrap();
        }
        // Manholes: at both ends and no more than 40 m apart (0, 33, 67,
        // 100).
        let holes = m.add_manholes(sewer).unwrap();
        assert_eq!(holes.len(), 4);
        assert!(m.add_manholes(sewer).unwrap().is_empty(), "no twice");
        // A connection per plot, falling to the sewer.
        let links = m.connect_plots(sewer).unwrap();
        assert_eq!(links.len(), 3);
        for (c, p) in &links {
            let pipe = m.run(*p).unwrap();
            assert!((pipe.slope - CONNECTION_SLOPE).abs() < 1e-12);
            assert!(pipe.z_at(pipe.length()) > -1.8 - 1.0);
            let ch = m.device(*c).unwrap();
            assert!((ch.position.y - 7.5).abs() < 1e-6, "{:?}", ch.position);
            assert!(ch.elevation < 0.0 && ch.elevation + ch.height > -1e-6);
        }
        assert!(m.connect_plots(sewer).unwrap().is_empty(), "no twice");
        // Joined to the buildings: the second plot's house has a downpipe.
        m.add_device(NewDevice {
            level: lv,
            kind: DeviceKind::Downpipe,
            name: "Bajante".into(),
            position: Point2::new(50.0, 12.0),
            rotation: 0.0,
            elevation: 0.0,
            width: 0.11,
            depth: 0.11,
            height: 6.0,
        })
        .unwrap();
        let joined = m.connect_buildings(sewer).unwrap();
        assert_eq!(joined.len(), 1);
        let pipe = m.run(joined[0]).unwrap();
        assert!(pipe.path[0].distance(Point2::new(50.0, 12.0)) < 1e-9);
        assert!((pipe.path[1].y - 7.5).abs() < 1e-6);
        // It falls into the chamber's connection.
        assert!(pipe.z_at(pipe.length()) < pipe.elevation);
        assert!(m.connect_buildings(sewer).unwrap().is_empty(), "no twice");
        // Its profile: the ground over it, the pipe falling and the
        // manholes.
        let prof = m.run_profile(sewer);
        assert!(prof.len() >= 21);
        assert_eq!(prof.iter().filter(|s| s.manhole).count(), 4);
        let (a, b) = (prof[0], prof[prof.len() - 1]);
        assert!((a.invert - b.invert - 1.0).abs() < 1e-9);
        assert!(prof.iter().all(|s| s.ground > s.invert));
    }
}
