//! Urbanismo (`docs/DISCIPLINAS.md` §3): plots and roads, which belong to
//! the site and not to the building.
//!
//! A [`Plot`] is a parcel by its boundary, with its cadastral reference and
//! the setback the building keeps from its boundary: its area is what the
//! masses' plot ratio and coverage are worked out with. A [`Road`] runs
//! along an axis with a cross-section: the carriageway in the middle and,
//! on each side, parking, a kerb and a sidewalk. IFC: a plot is an
//! `IfcSite` inside the project's site; a road an `IfcRoad` with its
//! `IfcPavement`s and `IfcKerb`s (in IFC4, proxies on the site).

use super::*;

/// A parcel.
#[derive(Debug, Clone, PartialEq)]
pub struct Plot {
    pub id: PlotId,
    pub guid: GlobalId,
    pub name: String,
    pub level: LevelId,
    /// Its boundary, counter-clockwise.
    pub outline: Vec<Point2>,
    /// What the building keeps from the boundary, metres.
    pub setback: f64,
    /// Cadastral reference.
    pub reference: String,
}

/// Data needed to make a plot.
#[derive(Debug, Clone, PartialEq)]
pub struct NewPlot {
    pub name: String,
    pub level: LevelId,
    pub outline: Vec<Point2>,
    pub setback: f64,
    pub reference: String,
}

impl Plot {
    pub fn area(&self) -> f64 {
        signed_area(&self.outline).abs()
    }

    pub fn perimeter(&self) -> f64 {
        let n = self.outline.len();
        (0..n)
            .map(|i| self.outline[i].distance(self.outline[(i + 1) % n]))
            .sum()
    }

    /// Where the building may go: the boundary `setback` inwards (none if
    /// it does not fit).
    pub fn buildable(&self) -> Option<Vec<Point2>> {
        if self.setback <= EPSILON {
            Some(self.outline.clone())
        } else {
            offset_contour(&self.outline, -self.setback)
        }
    }
}

/// The cross-section of a road, metres: the carriageway's whole width and,
/// on each side, parking (zero: none), kerb and sidewalk.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoadSection {
    pub carriageway: f64,
    pub parking: f64,
    pub kerb: f64,
    pub sidewalk: f64,
    /// How far the kerb and sidewalk stand over the carriageway.
    pub kerb_height: f64,
    /// Depth of the pavement under its top.
    pub depth: f64,
}

impl Default for RoadSection {
    /// Two lanes of 3,00 m, parking of 2,20 m, kerb of 0,15 m and
    /// sidewalks of 2,50 m.
    fn default() -> Self {
        RoadSection {
            carriageway: 6.0,
            parking: 2.2,
            kerb: 0.15,
            sidewalk: 2.5,
            kerb_height: 0.15,
            depth: 0.4,
        }
    }
}

impl RoadSection {
    pub fn width(&self) -> f64 {
        self.carriageway + 2.0 * (self.parking + self.kerb + self.sidewalk)
    }

    /// Each band with how far its two edges are to the left of the axis
    /// (negative: to its right), carriageway first, then each side from
    /// the carriageway outwards.
    pub fn band_offsets(&self) -> Vec<(RoadBand, f64, f64)> {
        let c = self.carriageway / 2.0;
        let mut out = vec![(RoadBand::Carriageway, -c, c)];
        for side in [1.0, -1.0] {
            let mut d = c;
            for (band, w) in [
                (RoadBand::Parking, self.parking),
                (RoadBand::Kerb, self.kerb),
                (RoadBand::Sidewalk, self.sidewalk),
            ] {
                if w > EPSILON {
                    let (a, b) = if side > 0.0 { (d, d + w) } else { (-d - w, -d) };
                    out.push((band, a, b));
                }
                d += w;
            }
        }
        out
    }
}

/// A band of a road's section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoadBand {
    Carriageway,
    Parking,
    Kerb,
    Sidewalk,
}

impl RoadBand {
    pub fn name(self) -> &'static str {
        match self {
            RoadBand::Carriageway => "Calzada",
            RoadBand::Parking => "Aparcamiento",
            RoadBand::Kerb => "Bordillo",
            RoadBand::Sidewalk => "Acera",
        }
    }
}

/// A road.
#[derive(Debug, Clone, PartialEq)]
pub struct Road {
    pub id: RoadId,
    pub guid: GlobalId,
    pub name: String,
    pub level: LevelId,
    /// Its axis, two points at least.
    pub axis: Vec<Point2>,
    pub section: RoadSection,
    /// Height of the carriageway over the level.
    pub top_offset: f64,
    /// Of the carriageway and parking, of the kerbs and of the sidewalks.
    pub pavement: MaterialId,
    pub kerb_material: MaterialId,
    pub sidewalk_material: MaterialId,
}

/// Data needed to make a road.
#[derive(Debug, Clone, PartialEq)]
pub struct NewRoad {
    pub name: String,
    pub level: LevelId,
    pub axis: Vec<Point2>,
    pub section: RoadSection,
    pub top_offset: f64,
    pub pavement: MaterialId,
    pub kerb_material: MaterialId,
    pub sidewalk_material: MaterialId,
}

impl Road {
    /// Each band of the section with its outline, from the left side to
    /// the right.
    pub fn bands(&self) -> Vec<(RoadBand, Vec<Point2>)> {
        let mut out: Vec<_> = self
            .section
            .band_offsets()
            .into_iter()
            .map(|(band, a, b)| (band, strip(&self.axis, a, b)))
            .collect();
        out.retain(|(_, o)| o.len() >= 3);
        out
    }

    /// Its outline: the whole width along the axis.
    pub fn outline(&self) -> Vec<Point2> {
        let w = self.section.width() / 2.0;
        strip(&self.axis, -w, w)
    }

    pub fn length(&self) -> f64 {
        self.axis.windows(2).map(|w| w[0].distance(w[1])).sum()
    }

    /// The area of each kind of band, m².
    pub fn area_of(&self, band: RoadBand) -> f64 {
        self.bands()
            .iter()
            .filter(|(b, _)| *b == band)
            .map(|(_, o)| signed_area(o).abs())
            .sum()
    }
}

impl Model {
    // ---- Plots --------------------------------------------------------------

    fn validate_plot(&self, p: &NewPlot) -> Result<(), ModelError> {
        if !self.levels.contains_key(&p.level) {
            return Err(ModelError::UnknownLevel(p.level));
        }
        for q in &p.outline {
            finite_point(*q, "plot boundary")?;
        }
        if p.outline.len() < 3 || signed_area(&p.outline).abs() <= EPSILON {
            return Err(ModelError::InvalidShape(
                "a plot needs a boundary of three points or more",
            ));
        }
        finite(p.setback, "plot setback")?;
        if p.setback < 0.0 {
            return Err(ModelError::NonPositive("plot setback"));
        }
        Ok(())
    }

    pub fn add_plot(&mut self, spec: NewPlot) -> Result<PlotId, ModelError> {
        let mut spec = spec;
        if signed_area(&spec.outline) < 0.0 {
            spec.outline.reverse();
        }
        self.validate_plot(&spec)?;
        let id = PlotId(self.take_id());
        self.plots.insert(
            id,
            Plot {
                id,
                guid: GlobalId::new(),
                name: spec.name,
                level: spec.level,
                outline: spec.outline,
                setback: spec.setback,
                reference: spec.reference,
            },
        );
        Ok(id)
    }

    pub fn edit_plot(
        &mut self,
        id: PlotId,
        edit: impl FnOnce(&mut Plot),
    ) -> Result<(), ModelError> {
        let mut p = self
            .plots
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        edit(&mut p);
        if signed_area(&p.outline) < 0.0 {
            p.outline.reverse();
        }
        self.validate_plot(&NewPlot {
            name: p.name.clone(),
            level: p.level,
            outline: p.outline.clone(),
            setback: p.setback,
            reference: p.reference.clone(),
        })?;
        self.plots.insert(id, p);
        Ok(())
    }

    pub fn remove_plot(&mut self, id: PlotId) -> Result<Plot, ModelError> {
        self.plots
            .remove(&id)
            .ok_or(ModelError::UnknownElement(id.0))
    }

    pub fn plot(&self, id: PlotId) -> Option<&Plot> {
        self.plots.get(&id)
    }

    pub fn plots(&self) -> impl Iterator<Item = &Plot> {
        self.plots.values()
    }

    pub fn plots_on_level(&self, level: LevelId) -> impl Iterator<Item = &Plot> {
        self.plots.values().filter(move |p| p.level == level)
    }

    /// A name for a new plot: «Parcela 1».
    pub fn next_plot_name(&self) -> String {
        (1..)
            .map(|n: usize| format!("Parcela {n}"))
            .find(|n| !self.plots.values().any(|p| p.name == *n))
            .unwrap_or_default()
    }

    /// The plot an outline is on: the one holding its centre.
    pub fn plot_under(&self, outline: &[Point2]) -> Option<&Plot> {
        if outline.is_empty() {
            return None;
        }
        let c = outline.iter().fold(Point2::new(0.0, 0.0), |a, p| a + *p)
            * (1.0 / outline.len() as f64);
        self.plots.values().find(|p| point_in(&p.outline, c))
    }

    // ---- Roads --------------------------------------------------------------

    fn validate_road(&self, r: &NewRoad) -> Result<(), ModelError> {
        if !self.levels.contains_key(&r.level) {
            return Err(ModelError::UnknownLevel(r.level));
        }
        for m in [r.pavement, r.kerb_material, r.sidewalk_material] {
            if !self.materials.contains_key(&m) {
                return Err(ModelError::UnknownMaterial(m));
            }
        }
        for q in &r.axis {
            finite_point(*q, "road axis")?;
        }
        if r.axis.len() < 2 || r.axis.windows(2).any(|w| w[0].approx_eq(w[1])) {
            return Err(ModelError::InvalidShape(
                "a road needs an axis of two different points or more",
            ));
        }
        let s = r.section;
        for (v, what) in [
            (s.carriageway, "carriageway width"),
            (s.depth, "pavement depth"),
        ] {
            finite(v, what)?;
            if v <= EPSILON {
                return Err(ModelError::NonPositive(what));
            }
        }
        for (v, what) in [
            (s.parking, "parking width"),
            (s.kerb, "kerb width"),
            (s.sidewalk, "sidewalk width"),
            (s.kerb_height, "kerb height"),
        ] {
            finite(v, what)?;
            if v < 0.0 {
                return Err(ModelError::NonPositive(what));
            }
        }
        finite(r.top_offset, "road top offset")?;
        Ok(())
    }

    pub fn add_road(&mut self, spec: NewRoad) -> Result<RoadId, ModelError> {
        self.validate_road(&spec)?;
        let id = RoadId(self.take_id());
        self.roads.insert(
            id,
            Road {
                id,
                guid: GlobalId::new(),
                name: spec.name,
                level: spec.level,
                axis: spec.axis,
                section: spec.section,
                top_offset: spec.top_offset,
                pavement: spec.pavement,
                kerb_material: spec.kerb_material,
                sidewalk_material: spec.sidewalk_material,
            },
        );
        Ok(id)
    }

    pub fn edit_road(
        &mut self,
        id: RoadId,
        edit: impl FnOnce(&mut Road),
    ) -> Result<(), ModelError> {
        let mut r = self
            .roads
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        edit(&mut r);
        self.validate_road(&NewRoad {
            name: r.name.clone(),
            level: r.level,
            axis: r.axis.clone(),
            section: r.section,
            top_offset: r.top_offset,
            pavement: r.pavement,
            kerb_material: r.kerb_material,
            sidewalk_material: r.sidewalk_material,
        })?;
        self.roads.insert(id, r);
        Ok(())
    }

    pub fn remove_road(&mut self, id: RoadId) -> Result<Road, ModelError> {
        self.roads
            .remove(&id)
            .ok_or(ModelError::UnknownElement(id.0))
    }

    pub fn road(&self, id: RoadId) -> Option<&Road> {
        self.roads.get(&id)
    }

    pub fn roads(&self) -> impl Iterator<Item = &Road> {
        self.roads.values()
    }

    pub fn roads_on_level(&self, level: LevelId) -> impl Iterator<Item = &Road> {
        self.roads.values().filter(move |r| r.level == level)
    }

    /// A name for a new road: «Vial 1».
    pub fn next_road_name(&self) -> String {
        (1..)
            .map(|n: usize| format!("Vial {n}"))
            .find(|n| !self.roads.values().any(|r| r.name == *n))
            .unwrap_or_default()
    }

    pub(super) fn restore_urban(
        &mut self,
        plots: Vec<Plot>,
        roads: Vec<Road>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for p in plots {
            let spec = NewPlot {
                name: p.name.clone(),
                level: p.level,
                outline: p.outline.clone(),
                setback: p.setback,
                reference: p.reference.clone(),
            };
            claim(p.id.0, Some(&p.guid))
                .and_then(|_| self.validate_plot(&spec))
                .map_err(|error| RestoreError {
                    element: format!("plot {} «{}»", p.id.0, p.name),
                    error,
                })?;
            self.plots.insert(p.id, p);
        }
        for r in roads {
            let spec = NewRoad {
                name: r.name.clone(),
                level: r.level,
                axis: r.axis.clone(),
                section: r.section,
                top_offset: r.top_offset,
                pavement: r.pavement,
                kerb_material: r.kerb_material,
                sidewalk_material: r.sidewalk_material,
            };
            claim(r.id.0, Some(&r.guid))
                .and_then(|_| self.validate_road(&spec))
                .map_err(|error| RestoreError {
                    element: format!("road {} «{}»", r.id.0, r.name),
                    error,
                })?;
            self.roads.insert(r.id, r);
        }
        Ok(())
    }
}

/// Whether a closed polygon contains a point (ray casting).
fn point_in(poly: &[Point2], p: Point2) -> bool {
    let n = poly.len();
    let mut inside = false;
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plots_give_their_area_to_the_masses() {
        let mut m = Model::new("Urbanismo");
        let l = m.add_level("Rasante", 0.0);
        let id = m
            .add_plot(NewPlot {
                name: m.next_plot_name(),
                level: l,
                outline: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(0.0, 30.0),
                    Point2::new(40.0, 30.0),
                    Point2::new(40.0, 0.0),
                ],
                setback: 3.0,
                reference: "1234567VK4713S0001AB".into(),
            })
            .unwrap();
        let p = m.plot(id).unwrap();
        assert_eq!(p.name, "Parcela 1");
        assert!(signed_area(&p.outline) > 0.0);
        assert!((p.area() - 1200.0).abs() < 1e-9);
        assert!((p.perimeter() - 140.0).abs() < 1e-9);
        assert!((signed_area(&p.buildable().unwrap()) - 34.0 * 24.0).abs() < 1e-9);
        // Without an area written by hand, the masses take the plots'.
        assert_eq!(m.plot_area(), Some(1200.0));
        m.set_plot_area(Some(1000.0)).unwrap();
        assert_eq!(m.plot_area(), Some(1000.0));
        assert!(m.plot_under(&[Point2::new(10.0, 10.0)]).is_some());
    }

    #[test]
    fn roads_have_their_bands() {
        let mut m = Model::new("Urbanismo");
        let l = m.add_level("Rasante", 0.0);
        let mat = m.add_material("Asfalto", "asphalt", [90, 90, 90], CutPattern::Solid);
        let id = m
            .add_road(NewRoad {
                name: m.next_road_name(),
                level: l,
                axis: vec![Point2::new(0.0, 0.0), Point2::new(100.0, 0.0)],
                section: RoadSection::default(),
                top_offset: 0.0,
                pavement: mat,
                kerb_material: mat,
                sidewalk_material: mat,
            })
            .unwrap();
        let r = m.road(id).unwrap();
        assert!((r.section.width() - 15.7).abs() < 1e-9);
        // Carriageway, and parking, kerb and sidewalk on each side.
        assert_eq!(r.bands().len(), 7);
        assert!((r.area_of(RoadBand::Carriageway) - 600.0).abs() < 1e-9);
        assert!((r.area_of(RoadBand::Sidewalk) - 500.0).abs() < 1e-9);
        assert!((signed_area(&r.outline()) - 1570.0).abs() < 1e-9);
        assert!(m
            .edit_road(id, |r| r.axis = vec![Point2::new(0.0, 0.0)])
            .is_err());
    }
}
