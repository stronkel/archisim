//! The street types (U3): every section a street usually has here, by
//! group, as cards to choose from (peatonal, de coexistencia, residencial
//! de uno o dos sentidos con o sin aparcamiento en línea, batería o
//! espiga, arbolada, con mediana, bulevar, con carril bici, bus o tranvía,
//! travesía, calle de servicio, carretera con arcén y cuneta, camino
//! rural), and the tree pits of the sidewalks that have them.

use super::*;

/// The group a street type is shown in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StreetGroup {
    Pedestrian,
    Residential,
    TreeLined,
    Avenue,
    Transport,
    Rural,
}

impl StreetGroup {
    pub const ALL: [StreetGroup; 6] = [
        StreetGroup::Pedestrian,
        StreetGroup::Residential,
        StreetGroup::TreeLined,
        StreetGroup::Avenue,
        StreetGroup::Transport,
        StreetGroup::Rural,
    ];

    pub fn name(self) -> &'static str {
        match self {
            StreetGroup::Pedestrian => "Peatonales y de coexistencia",
            StreetGroup::Residential => "Residenciales",
            StreetGroup::TreeLined => "Arboladas",
            StreetGroup::Avenue => "Avenidas y bulevares",
            StreetGroup::Transport => "Bici, bus y tranvía",
            StreetGroup::Rural => "Travesías, carreteras y caminos",
        }
    }
}

/// A street type: its name, its group and its section.
#[derive(Debug, Clone, PartialEq)]
pub struct StreetType {
    pub name: &'static str,
    pub group: StreetGroup,
    pub section: RoadSection,
}

/// Every street type, by group.
pub fn street_types() -> Vec<StreetType> {
    use RoadBand::*;
    use StreetGroup::*;
    let s = |b, w| strip_of(b, w);
    let k = || strip_of(Kerb, 0.15);
    let of = |strips: Vec<RoadStrip>| RoadSection::of(strips);
    let single = |strips: Vec<RoadStrip>| {
        let mut x = RoadSection::of(strips);
        x.single_level = true;
        x
    };
    let parked = |strips: Vec<RoadStrip>, mode: ParkingMode| {
        let mut x = RoadSection::of(strips);
        x.parking = mode;
        x
    };
    let t = |name, group, section| StreetType {
        name,
        group,
        section,
    };
    vec![
        // Pedestrian and shared.
        t("Calle peatonal", Pedestrian, single(vec![s(Sidewalk, 7.0)])),
        t(
            "Calle peatonal arbolada",
            Pedestrian,
            single(vec![s(Sidewalk, 3.0), s(TreePits, 1.5), s(Sidewalk, 3.0)]),
        ),
        t(
            "Calle de plataforma única",
            Pedestrian,
            single(vec![
                s(Sidewalk, 1.8),
                s(Carriageway, 3.5),
                s(Sidewalk, 1.8),
            ]),
        ),
        t(
            "Calle de coexistencia con alcorques",
            Pedestrian,
            single(vec![
                s(Sidewalk, 1.8),
                s(Parking, 2.0),
                s(Carriageway, 3.0),
                s(TreePits, 1.2),
                s(Sidewalk, 1.8),
            ]),
        ),
        t(
            "Calle de casco histórico",
            Pedestrian,
            of(vec![
                s(Sidewalk, 1.0),
                k(),
                s(Carriageway, 3.0),
                k(),
                s(Sidewalk, 1.0),
            ]),
        ),
        t(
            "Paseo marítimo",
            Pedestrian,
            of(vec![
                s(Sidewalk, 6.0),
                s(BikePath, 2.5),
                k(),
                s(Carriageway, 3.25),
                s(Carriageway, 3.25),
                s(Parking, 2.2),
                k(),
                s(Sidewalk, 3.0),
            ]),
        ),
        // Residential.
        t(
            "Calle de doble sentido con aparcamiento",
            Residential,
            RoadSection::default(),
        ),
        t(
            "Calle de doble sentido sin aparcamiento",
            Residential,
            RoadSection::classic(6.0, 0.0, 0.15, 2.5),
        ),
        t(
            "Calle de doble sentido con aparcamiento a un lado",
            Residential,
            of(vec![
                s(Sidewalk, 2.5),
                k(),
                s(Parking, 2.2),
                s(Carriageway, 3.0),
                s(Carriageway, 3.0),
                k(),
                s(Sidewalk, 2.5),
            ]),
        ),
        t(
            "Calle de un sentido con aparcamiento",
            Residential,
            of(vec![
                s(Sidewalk, 2.0),
                k(),
                s(Parking, 2.2),
                s(Carriageway, 3.5),
                k(),
                s(Sidewalk, 2.0),
            ]),
        ),
        t(
            "Calle de un sentido sin aparcamiento",
            Residential,
            of(vec![
                s(Sidewalk, 2.0),
                k(),
                s(Carriageway, 3.5),
                k(),
                s(Sidewalk, 2.0),
            ]),
        ),
        t(
            "Calle de un sentido con aparcamiento a los dos lados",
            Residential,
            of(vec![
                s(Sidewalk, 2.0),
                k(),
                s(Parking, 2.2),
                s(Carriageway, 3.5),
                s(Parking, 2.2),
                k(),
                s(Sidewalk, 2.0),
            ]),
        ),
        t(
            "Calle con aparcamiento en batería",
            Residential,
            parked(
                vec![
                    s(Sidewalk, 2.5),
                    k(),
                    s(Parking, 5.0),
                    s(Carriageway, 3.5),
                    k(),
                    s(Sidewalk, 2.5),
                ],
                ParkingMode::Battery,
            ),
        ),
        t(
            "Calle con aparcamiento en espiga",
            Residential,
            parked(
                vec![
                    s(Sidewalk, 2.5),
                    k(),
                    s(Parking, 4.5),
                    s(Carriageway, 3.5),
                    k(),
                    s(Sidewalk, 2.5),
                ],
                ParkingMode::Angled,
            ),
        ),
        // Tree-lined.
        t(
            "Calle arbolada a los dos lados",
            TreeLined,
            of(vec![
                s(TreePits, 3.5),
                k(),
                s(Parking, 2.2),
                s(Carriageway, 3.0),
                s(Carriageway, 3.0),
                s(Parking, 2.2),
                k(),
                s(TreePits, 3.5),
            ]),
        ),
        t(
            "Calle arbolada a un lado",
            TreeLined,
            of(vec![
                s(TreePits, 3.5),
                k(),
                s(Parking, 2.2),
                s(Carriageway, 3.0),
                s(Carriageway, 3.0),
                k(),
                s(Sidewalk, 2.5),
            ]),
        ),
        t(
            "Calle con franja verde continua",
            TreeLined,
            of(vec![
                s(Sidewalk, 2.0),
                s(Planting, 1.5),
                k(),
                s(Parking, 2.2),
                s(Carriageway, 3.0),
                s(Carriageway, 3.0),
                s(Parking, 2.2),
                k(),
                s(Planting, 1.5),
                s(Sidewalk, 2.0),
            ]),
        ),
        // Avenues.
        t(
            "Avenida con mediana arbolada",
            Avenue,
            of(vec![
                s(Sidewalk, 2.5),
                s(Planting, 1.2),
                k(),
                s(Parking, 2.2),
                s(Carriageway, 3.25),
                s(Carriageway, 3.25),
                k(),
                s(Median, 2.0),
                k(),
                s(Carriageway, 3.25),
                s(Carriageway, 3.25),
                s(Parking, 2.2),
                k(),
                s(Planting, 1.2),
                s(Sidewalk, 2.5),
            ]),
        ),
        t(
            "Avenida con mediana ajardinada",
            Avenue,
            of(vec![
                s(Sidewalk, 3.0),
                k(),
                s(Carriageway, 3.25),
                s(Carriageway, 3.25),
                k(),
                s(Planting, 4.0),
                k(),
                s(Carriageway, 3.25),
                s(Carriageway, 3.25),
                k(),
                s(Sidewalk, 3.0),
            ]),
        ),
        t(
            "Avenida de dos calzadas con aparcamiento",
            Avenue,
            of(vec![
                s(TreePits, 4.0),
                k(),
                s(Parking, 2.2),
                s(Carriageway, 3.25),
                s(Carriageway, 3.25),
                k(),
                s(Median, 1.5),
                k(),
                s(Carriageway, 3.25),
                s(Carriageway, 3.25),
                s(Parking, 2.2),
                k(),
                s(TreePits, 4.0),
            ]),
        ),
        t(
            "Bulevar con paseo central",
            Avenue,
            of(vec![
                s(Sidewalk, 3.0),
                k(),
                s(Carriageway, 3.5),
                k(),
                s(Planting, 1.5),
                s(Median, 8.0),
                s(Planting, 1.5),
                k(),
                s(Carriageway, 3.5),
                k(),
                s(Sidewalk, 3.0),
            ]),
        ),
        // Bikes, buses and trams.
        t(
            "Calle con carril bici",
            Transport,
            of(vec![
                s(Sidewalk, 2.5),
                k(),
                s(BikeLane, 2.0),
                k(),
                s(Parking, 2.2),
                s(Carriageway, 3.0),
                s(Carriageway, 3.0),
                k(),
                s(Sidewalk, 2.5),
            ]),
        ),
        t(
            "Calle con carril bici bidireccional",
            Transport,
            of(vec![
                s(Sidewalk, 2.5),
                k(),
                s(BikeLane, 2.5),
                k(),
                s(Median, 0.6),
                k(),
                s(Carriageway, 3.0),
                s(Carriageway, 3.0),
                s(Parking, 2.2),
                k(),
                s(Sidewalk, 2.5),
            ]),
        ),
        t(
            "Calle con carril bici en acera",
            Transport,
            of(vec![
                s(Sidewalk, 2.5),
                s(BikePath, 2.5),
                k(),
                s(Parking, 2.2),
                s(Carriageway, 3.0),
                s(Carriageway, 3.0),
                s(Parking, 2.2),
                k(),
                s(Sidewalk, 3.0),
            ]),
        ),
        t(
            "Avenida con carril bus",
            Transport,
            of(vec![
                s(Sidewalk, 3.0),
                k(),
                s(BusLane, 3.5),
                s(Carriageway, 3.25),
                s(Carriageway, 3.25),
                k(),
                s(Median, 1.5),
                k(),
                s(Carriageway, 3.25),
                s(Carriageway, 3.25),
                s(BusLane, 3.5),
                k(),
                s(Sidewalk, 3.0),
            ]),
        ),
        t(
            "Avenida con tranvía",
            Transport,
            of(vec![
                s(Sidewalk, 3.0),
                k(),
                s(Carriageway, 3.25),
                s(Carriageway, 3.25),
                s(Tram, 6.5),
                s(Carriageway, 3.25),
                s(Carriageway, 3.25),
                k(),
                s(Sidewalk, 3.0),
            ]),
        ),
        // Through roads, roads and paths.
        t(
            "Travesía",
            Rural,
            of(vec![
                s(Sidewalk, 2.0),
                k(),
                s(Shoulder, 1.0),
                s(Carriageway, 3.5),
                s(Carriageway, 3.5),
                s(Shoulder, 1.0),
                k(),
                s(Sidewalk, 2.0),
            ]),
        ),
        t(
            "Calle de servicio",
            Rural,
            of(vec![
                s(Sidewalk, 2.0),
                k(),
                s(Parking, 2.2),
                s(Carriageway, 3.5),
                k(),
                s(Median, 2.0),
            ]),
        ),
        t(
            "Carretera con arcén y cuneta",
            Rural,
            single(vec![
                s(Ditch, 1.0),
                s(Shoulder, 1.5),
                s(Carriageway, 3.5),
                s(Carriageway, 3.5),
                s(Shoulder, 1.5),
                s(Ditch, 1.0),
            ]),
        ),
        t(
            "Camino rural",
            Rural,
            single(vec![
                s(Planting, 1.0),
                s(Carriageway, 2.75),
                s(Carriageway, 2.75),
                s(Planting, 1.0),
            ]),
        ),
    ]
}

/// The street type a section is (its strips, level and parking the same).
pub fn street_type_of(section: &RoadSection) -> Option<StreetType> {
    street_types().into_iter().find(|t| {
        t.section.strips == section.strips
            && t.section.single_level == section.single_level
            && t.section.parking == section.parking
    })
}

/// The gauge of a tram's track (between the rails' middles).
pub const TRAM_GAUGE: f64 = 1.435;

impl Road {
    /// The rails of its tram strips: a track (two rails) per 3 m of strip,
    /// two at most, along the whole road.
    pub fn rail_lines(&self) -> Vec<Vec<Point2>> {
        let mut out = Vec::new();
        for (band, a, b) in self.section.band_offsets() {
            if band != RoadBand::Tram {
                continue;
            }
            let w = b - a;
            let n = ((w / 3.0).floor() as usize).clamp(1, 2);
            for i in 0..n {
                let c = a + w * (i as f64 + 0.5) / n as f64;
                for d in [-TRAM_GAUGE / 2.0, TRAM_GAUGE / 2.0] {
                    if let Some(line) = offset_path(&self.axis, c + d) {
                        out.push(line);
                    }
                }
            }
        }
        out
    }

    /// Where along the axis its tree pits are: every
    /// [`TREE_PIT_SPACING`] from half of it, so the first and last are
    /// half a spacing from the ends.
    pub fn tree_pit_stations(&self) -> Vec<f64> {
        let has = self
            .section
            .strips
            .iter()
            .any(|x| x.band == RoadBand::TreePits)
            || self
                .changes
                .iter()
                .any(|c| c.strips.iter().any(|x| x.band == RoadBand::TreePits));
        if !has {
            return Vec::new();
        }
        let len = self.length();
        let n = ((len - TREE_PIT_SPACING) / TREE_PIT_SPACING)
            .round()
            .max(0.0) as usize
            + 1;
        let step = if n > 1 {
            (len - TREE_PIT_SPACING) / (n - 1) as f64
        } else {
            0.0
        };
        (0..n)
            .map(|i| {
                if n == 1 {
                    len / 2.0
                } else {
                    TREE_PIT_SPACING / 2.0 + step * i as f64
                }
            })
            .collect()
    }

    /// Its tree pits, squares in the middle of its strips with tree pits
    /// (but in the stretches `gaps`).
    pub fn tree_pits_except(&self, gaps: &[(f64, f64)]) -> Vec<Vec<Point2>> {
        let mut out = Vec::new();
        for t in self.tree_pit_stations() {
            if gaps.iter().any(|&(a, b)| t > a - 1.0 && t < b + 1.0) {
                continue;
            }
            for (band, a, b) in self.offsets_at(t) {
                if band != RoadBand::TreePits {
                    continue;
                }
                let side = TREE_PIT_SIZE.min(b - a - 0.3);
                if side < 0.4 {
                    continue;
                }
                let m = (a + b) / 2.0;
                out.push(super::urban::quad(
                    &self.axis,
                    t - side / 2.0,
                    t + side / 2.0,
                    m - side / 2.0,
                    m + side / 2.0,
                ));
            }
        }
        out
    }

    /// The lines trees are planted along: the middle of its planting
    /// strips and, for its strips with tree pits, from the first pit to
    /// the last; and how many trees each takes at `spacing` (one per pit).
    pub fn tree_lines(&self, spacing: f64) -> Vec<(Vec<Point2>, usize)> {
        let mut out = Vec::new();
        let len = self.length();
        for line in self.strip_lines(RoadBand::Planting) {
            let l: f64 = line.windows(2).map(|w| w[0].distance(w[1])).sum();
            out.push((line, ((l / spacing).floor() as usize).max(1)));
        }
        let stations = self.tree_pit_stations();
        if let (Some(&first), Some(&last)) = (stations.first(), stations.last()) {
            for (band, a, b) in self.section.band_offsets() {
                if band != RoadBand::TreePits {
                    continue;
                }
                let m = (a + b) / 2.0;
                let path = super::urban::sub_path_of(&self.axis, first, last.min(len));
                if let Some(line) = offset_path(&path, m) {
                    out.push((line, stations.len()));
                }
            }
        }
        out
    }
}

/// The roles of a road's materials, for a street type kept in the
/// library: (role, its name).
pub const STREET_MATERIAL_ROLES: [(&str, &str); 6] = [
    ("pavement", "Calzada"),
    ("kerb", "Bordillo"),
    ("sidewalk", "Acera"),
    ("planting", "Franja verde y alcorques"),
    ("bike", "Carril bici"),
    ("markings", "Marcas viales"),
];

/// A material of a street type kept in the library: its name, category
/// and colour (found by name in a project, or added to it).
#[derive(Debug, Clone, PartialEq)]
pub struct SavedMaterial {
    pub role: String,
    pub name: String,
    pub category: String,
    pub rgb: [u8; 3],
}

/// A street type kept in the library (U3): its name, its section (without
/// build-outs or crossings, which belong to a street) and its materials.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedStreet {
    pub name: String,
    pub section: RoadSection,
    pub materials: Vec<SavedMaterial>,
}

impl SavedStreet {
    /// One line of a library file: `name⇥section⇥role:name:category:r,g,b;…`.
    pub fn encode(&self) -> String {
        let mut section = self.section.clone();
        section.buildouts.clear();
        section.crossings.clear();
        let clean = |t: &str| t.replace(['\t', '\n', ';', ':'], " ");
        let mats: Vec<String> = self
            .materials
            .iter()
            .map(|m| {
                format!(
                    "{}:{}:{}:{},{},{}",
                    m.role,
                    clean(&m.name),
                    clean(&m.category),
                    m.rgb[0],
                    m.rgb[1],
                    m.rgb[2]
                )
            })
            .collect();
        format!(
            "{}\t{}\t{}",
            clean(&self.name),
            section.encode(),
            mats.join(";")
        )
    }

    /// The street types of a library file (lines that cannot be read are
    /// left out).
    pub fn decode_all(text: &str) -> Vec<SavedStreet> {
        text.lines().filter_map(SavedStreet::decode).collect()
    }

    pub fn decode(line: &str) -> Option<SavedStreet> {
        let mut it = line.split('\t');
        let name = it.next()?.trim().to_string();
        let section = RoadSection::decode(it.next()?)?;
        if name.is_empty() {
            return None;
        }
        let mut materials = Vec::new();
        for m in it
            .next()
            .unwrap_or("")
            .split(';')
            .filter(|x| !x.trim().is_empty())
        {
            let v: Vec<&str> = m.split(':').collect();
            if v.len() != 4 {
                continue;
            }
            let c: Vec<u8> = v[3]
                .split(',')
                .filter_map(|x| x.trim().parse().ok())
                .collect();
            if c.len() != 3 {
                continue;
            }
            materials.push(SavedMaterial {
                role: v[0].to_string(),
                name: v[1].to_string(),
                category: v[2].to_string(),
                rgb: [c[0], c[1], c[2]],
            });
        }
        Some(SavedStreet {
            name,
            section,
            materials,
        })
    }

    /// The library file with this type in it, replacing one of the same
    /// name.
    pub fn into_file(&self, text: &str) -> String {
        let mut out: Vec<String> = text
            .lines()
            .filter(|l| SavedStreet::decode(l).is_none_or(|s| s.name != self.name))
            .filter(|l| !l.trim().is_empty())
            .map(str::to_string)
            .collect();
        out.push(self.encode());
        out.join("\n") + "\n"
    }
}

impl Model {
    /// A road's section and materials as a street type for the library.
    pub fn saved_street(&self, id: RoadId, name: &str) -> Option<SavedStreet> {
        let r = self.road(id)?;
        let s = &r.section;
        let mut materials = Vec::new();
        for (role, mat) in [
            ("pavement", Some(r.pavement)),
            ("kerb", Some(r.kerb_material)),
            ("sidewalk", Some(r.sidewalk_material)),
            ("planting", s.planting),
            ("bike", s.bike),
            ("markings", s.markings),
        ] {
            if let Some(m) = mat.and_then(|m| self.material(m)) {
                materials.push(SavedMaterial {
                    role: role.into(),
                    name: m.name.clone(),
                    category: m.category.clone(),
                    rgb: m.cut_color,
                });
            }
        }
        let mut section = s.clone();
        section.buildouts.clear();
        section.crossings.clear();
        Some(SavedStreet {
            name: name.trim().to_string(),
            section,
            materials,
        })
    }

    /// Gives a road a street type of the library: its section (keeping
    /// the road's build-outs and crossings) and its materials, found by
    /// name in the project or added to it.
    pub fn apply_saved_street(&mut self, id: RoadId, t: &SavedStreet) -> Result<(), ModelError> {
        let mut ids = Vec::new();
        for m in &t.materials {
            let found = self.materials().find(|x| x.name == m.name).map(|x| x.id);
            let id = match found {
                Some(id) => id,
                None => {
                    self.add_material(m.name.clone(), m.category.clone(), m.rgb, CutPattern::Solid)
                }
            };
            ids.push((m.role.clone(), id));
        }
        let of = |role: &str| ids.iter().find(|(r, _)| r == role).map(|(_, id)| *id);
        let section = t.section.clone();
        self.edit_road(id, |r| {
            let mut s = section;
            s.buildouts = r.section.buildouts.clone();
            s.crossings = r.section.crossings.clone();
            s.planting = of("planting").or(r.section.planting);
            s.bike = of("bike").or(r.section.bike);
            s.markings = of("markings").or(r.section.markings);
            r.section = s;
            if let Some(m) = of("pavement") {
                r.pavement = m;
            }
            if let Some(m) = of("kerb") {
                r.kerb_material = m;
            }
            if let Some(m) = of("sidewalk") {
                r.sidewalk_material = m;
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_type_has_a_name_and_a_section() {
        let all = street_types();
        assert!(all.len() >= 30, "{}", all.len());
        let mut names: Vec<&str> = all.iter().map(|t| t.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), all.len(), "names repeated");
        for g in StreetGroup::ALL {
            assert!(all.iter().any(|t| t.group == g), "{}", g.name());
        }
        for t in &all {
            assert!(t.section.width() > 2.0, "{}", t.name);
            assert_eq!(street_type_of(&t.section).map(|x| x.name), Some(t.name));
            // Its text comes back the same.
            let back = RoadSection::decode(&t.section.encode()).unwrap();
            assert_eq!(back.strips, t.section.strips, "{}", t.name);
        }
    }

    #[test]
    fn tree_pits_every_six_metres() {
        let section = street_types()
            .into_iter()
            .find(|t| t.name == "Calle arbolada a los dos lados")
            .unwrap()
            .section;
        let r = Road {
            id: RoadId(1),
            guid: GlobalId::new(),
            name: "R".into(),
            level: LevelId(0),
            axis: vec![Point2::new(0.0, 0.0), Point2::new(60.0, 0.0)],
            section,
            top_offset: 0.0,
            profile: Vec::new(),
            changes: Vec::new(),
            pavement: MaterialId(0),
            kerb_material: MaterialId(0),
            sidewalk_material: MaterialId(0),
        };
        let st = r.tree_pit_stations();
        assert_eq!(st.len(), 10);
        assert!((st[0] - 3.0).abs() < 1e-9 && (st[9] - 57.0).abs() < 1e-9);
        // Two sides.
        assert_eq!(r.tree_pits_except(&[]).len(), 20);
        assert_eq!(r.tree_pits_except(&[(0.0, 10.0)]).len(), 16);
        assert!(r.rail_lines().is_empty());
        let lines = r.tree_lines(6.0);
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|(_, n)| *n == 10));
    }

    #[test]
    fn a_street_type_goes_to_the_library_and_back() {
        let mut m = Model::new("P");
        let lv = m.add_level("PB", 0.0);
        let asphalt = m.add_material("Asfalto rojo", "asphalt", [150, 60, 50], CutPattern::Solid);
        let stone = m.add_material("Granito", "stone", [200, 200, 200], CutPattern::Solid);
        let section = street_types()
            .into_iter()
            .find(|t| t.name == "Avenida con carril bus")
            .unwrap()
            .section;
        let id = m
            .add_road(NewRoad {
                name: "R".into(),
                level: lv,
                axis: vec![Point2::new(0.0, 0.0), Point2::new(50.0, 0.0)],
                section,
                top_offset: 0.0,
                profile: Vec::new(),
                changes: Vec::new(),
                pavement: asphalt,
                kerb_material: stone,
                sidewalk_material: stone,
            })
            .unwrap();
        let saved = m.saved_street(id, "Mi avenida: con bus").unwrap();
        let file = saved.into_file("");
        let back = SavedStreet::decode_all(&file);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].name, "Mi avenida  con bus");
        assert_eq!(back[0].section.strips, saved.section.strips);
        assert_eq!(back[0].materials.len(), 3);
        // Saved again with the same name: replaced, not repeated.
        let mut again = back[0].clone();
        again.section.strips.pop();
        let file = again.into_file(&file);
        assert_eq!(SavedStreet::decode_all(&file).len(), 1);
        // Into another project: the materials come with it.
        let mut n = Model::new("Q");
        let lv = n.add_level("PB", 0.0);
        let any = n.add_material("Hormigón", "concrete", [180, 180, 180], CutPattern::Solid);
        let other = n
            .add_road(NewRoad {
                name: "S".into(),
                level: lv,
                axis: vec![Point2::new(0.0, 0.0), Point2::new(50.0, 0.0)],
                section: RoadSection::default(),
                top_offset: 0.0,
                profile: Vec::new(),
                changes: Vec::new(),
                pavement: any,
                kerb_material: any,
                sidewalk_material: any,
            })
            .unwrap();
        n.apply_saved_street(other, &back[0]).unwrap();
        let r = n.road(other).unwrap();
        assert_eq!(r.section.strips, back[0].section.strips);
        assert_eq!(n.material(r.pavement).unwrap().name, "Asfalto rojo");
    }
}
