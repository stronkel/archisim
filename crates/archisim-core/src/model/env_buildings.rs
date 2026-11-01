//! Buildings of the surroundings (U6): a mass by floors "dressed" in a
//! style — the white house between party walls with its window grilles,
//! the block of the «ensanche» with balconies and a shop on the ground
//! floor, the 1960s block with terraces, the office building in glass… —
//! so that the streets around a project look like streets. A dressed mass
//! keeps its contour, its floors and its floor height; the dress says how
//! its façades, its roof and its finishes are drawn, and nothing of it is
//! stored but its parameters: the windows, balconies and roof are made
//! each time from them (in `archisim-geom::env_buildings`).
//!
//! The sides of the contour that lie against another mass are party walls
//! («medianeras»): blind, without windows nor balconies, and the roof does
//! not fall on them.

use super::*;

/// The style of a building of the surroundings: what it looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BuildingStyle {
    /// Casa andaluza entre medianeras: white, grilles, plinth, tiled roof.
    Andalusian,
    /// Casa patio: one or two floors round a courtyard, flat roof.
    Courtyard,
    /// Adosada: a terraced house, tiled roof falling to front and back.
    Terraced,
    /// Pareada: a semi-detached house, hipped roof.
    SemiDetached,
    /// Bloque de ensanche: balconies, shop on the ground floor, cornice.
    Ensanche,
    /// Bloque de los años 60–70: brick, continuous terraces.
    Block1960s,
    /// Chalet: a detached house, hipped roof.
    Detached,
    /// Casco histórico: coloured render, iron balconies, big cornice.
    HistoricCentre,
    /// Oficinas: curtain wall.
    Offices,
    /// Comercio con viviendas encima.
    ShopAndFlats,
    /// Nave industrial: metal sheet, strip windows, a big door.
    Warehouse,
    /// Equipamiento (colegio, polideportivo): long windows, flat roof.
    Facility,
    /// Hotel de costa: white, terraces with glass balustrades.
    CoastalHotel,
}

impl BuildingStyle {
    pub const ALL: [BuildingStyle; 13] = [
        BuildingStyle::Andalusian,
        BuildingStyle::Courtyard,
        BuildingStyle::Terraced,
        BuildingStyle::SemiDetached,
        BuildingStyle::Ensanche,
        BuildingStyle::Block1960s,
        BuildingStyle::Detached,
        BuildingStyle::HistoricCentre,
        BuildingStyle::Offices,
        BuildingStyle::ShopAndFlats,
        BuildingStyle::Warehouse,
        BuildingStyle::Facility,
        BuildingStyle::CoastalHotel,
    ];

    pub fn name(self) -> &'static str {
        match self {
            BuildingStyle::Andalusian => "Casa andaluza entre medianeras",
            BuildingStyle::Courtyard => "Casa patio",
            BuildingStyle::Terraced => "Adosada",
            BuildingStyle::SemiDetached => "Pareada",
            BuildingStyle::Ensanche => "Bloque de ensanche",
            BuildingStyle::Block1960s => "Bloque de los años 60–70",
            BuildingStyle::Detached => "Vivienda aislada (chalet)",
            BuildingStyle::HistoricCentre => "Casco histórico",
            BuildingStyle::Offices => "Oficinas de muro cortina",
            BuildingStyle::ShopAndFlats => "Comercio con viviendas",
            BuildingStyle::Warehouse => "Nave industrial",
            BuildingStyle::Facility => "Equipamiento",
            BuildingStyle::CoastalHotel => "Hotel de costa",
        }
    }

    /// Its code in files.
    pub fn code(self) -> &'static str {
        match self {
            BuildingStyle::Andalusian => "andalusian",
            BuildingStyle::Courtyard => "courtyard",
            BuildingStyle::Terraced => "terraced",
            BuildingStyle::SemiDetached => "semi_detached",
            BuildingStyle::Ensanche => "ensanche",
            BuildingStyle::Block1960s => "block_1960s",
            BuildingStyle::Detached => "detached",
            BuildingStyle::HistoricCentre => "historic_centre",
            BuildingStyle::Offices => "offices",
            BuildingStyle::ShopAndFlats => "shop_and_flats",
            BuildingStyle::Warehouse => "warehouse",
            BuildingStyle::Facility => "facility",
            BuildingStyle::CoastalHotel => "coastal_hotel",
        }
    }

    pub fn from_code(code: &str) -> Option<BuildingStyle> {
        Self::ALL.into_iter().find(|s| s.code() == code)
    }

    /// The floors and floor height it usually has.
    pub fn usual_floors(self) -> (u32, f64) {
        match self {
            BuildingStyle::Andalusian => (2, 3.0),
            BuildingStyle::Courtyard => (2, 3.0),
            BuildingStyle::Terraced => (2, 2.9),
            BuildingStyle::SemiDetached => (2, 2.9),
            BuildingStyle::Ensanche => (5, 3.2),
            BuildingStyle::Block1960s => (7, 2.85),
            BuildingStyle::Detached => (2, 3.0),
            BuildingStyle::HistoricCentre => (3, 3.3),
            BuildingStyle::Offices => (6, 3.6),
            BuildingStyle::ShopAndFlats => (4, 3.0),
            BuildingStyle::Warehouse => (1, 7.0),
            BuildingStyle::Facility => (2, 3.8),
            BuildingStyle::CoastalHotel => (9, 3.0),
        }
    }

    /// What it is drawn with: name, category and colour of each finish.
    pub fn finishes(self) -> [(&'static str, &'static str, [u8; 3]); 6] {
        let glass = ("Vidrio del entorno", "glass", [120, 150, 165]);
        let iron = ("Hierro forjado negro", "metal", [40, 40, 42]);
        let tile = ("Teja cerámica curva", "roof", [172, 88, 58]);
        let white = ("Cal blanca", "render", [246, 244, 236]);
        match self {
            BuildingStyle::Andalusian => [
                white,
                ("Zócalo albero", "render", [205, 160, 80]),
                tile,
                ("Carpintería verde carruaje", "wood", [38, 74, 52]),
                glass,
                iron,
            ],
            BuildingStyle::Courtyard => [
                white,
                ("Zócalo gris piedra", "stone", [150, 146, 138]),
                ("Cubierta plana gris", "roof", [168, 166, 160]),
                ("Carpintería madera", "wood", [120, 80, 48]),
                glass,
                iron,
            ],
            BuildingStyle::Terraced | BuildingStyle::SemiDetached => [
                ("Revoco ocre claro", "render", [230, 212, 176]),
                ("Zócalo gris piedra", "stone", [150, 146, 138]),
                tile,
                ("Carpintería blanca", "metal", [236, 236, 232]),
                glass,
                ("Barandilla gris", "metal", [90, 92, 96]),
            ],
            BuildingStyle::Ensanche => [
                ("Revoco crema", "render", [226, 210, 178]),
                ("Piedra artificial", "stone", [200, 190, 168]),
                ("Cubierta plana gris", "roof", [168, 166, 160]),
                ("Carpintería madera", "wood", [120, 80, 48]),
                glass,
                iron,
            ],
            BuildingStyle::Block1960s => [
                ("Ladrillo visto rojizo", "masonry", [168, 92, 64]),
                ("Hormigón visto", "concrete", [178, 176, 168]),
                ("Cubierta plana gris", "roof", [168, 166, 160]),
                ("Carpintería aluminio", "metal", [190, 192, 194]),
                glass,
                ("Barandilla gris", "metal", [90, 92, 96]),
            ],
            BuildingStyle::Detached => [
                white,
                ("Zócalo gris piedra", "stone", [150, 146, 138]),
                ("Teja mixta roja", "roof", [160, 70, 50]),
                ("Carpintería blanca", "metal", [236, 236, 232]),
                glass,
                ("Barandilla gris", "metal", [90, 92, 96]),
            ],
            BuildingStyle::HistoricCentre => [
                ("Revoco almagra", "render", [196, 128, 92]),
                ("Piedra artificial", "stone", [214, 204, 182]),
                tile,
                ("Carpintería verde carruaje", "wood", [38, 74, 52]),
                glass,
                iron,
            ],
            BuildingStyle::Offices => [
                ("Panel de aluminio gris", "metal", [92, 98, 106]),
                ("Panel de aluminio gris", "metal", [92, 98, 106]),
                ("Cubierta plana gris", "roof", [168, 166, 160]),
                ("Montante de aluminio", "metal", [176, 180, 184]),
                ("Vidrio de control solar", "glass", [92, 128, 150]),
                ("Montante de aluminio", "metal", [176, 180, 184]),
            ],
            BuildingStyle::ShopAndFlats => [
                ("Revoco blanco roto", "render", [236, 230, 218]),
                ("Piedra artificial", "stone", [200, 190, 168]),
                ("Cubierta plana gris", "roof", [168, 166, 160]),
                ("Carpintería aluminio", "metal", [190, 192, 194]),
                glass,
                ("Barandilla gris", "metal", [90, 92, 96]),
            ],
            BuildingStyle::Warehouse => [
                ("Chapa grecada gris", "metal", [186, 190, 192]),
                ("Hormigón visto", "concrete", [178, 176, 168]),
                ("Chapa grecada gris", "metal", [186, 190, 192]),
                ("Chapa lacada azul", "metal", [52, 84, 128]),
                glass,
                ("Chapa lacada azul", "metal", [52, 84, 128]),
            ],
            BuildingStyle::Facility => [
                ("Revoco blanco roto", "render", [236, 230, 218]),
                ("Ladrillo visto rojizo", "masonry", [168, 92, 64]),
                ("Cubierta plana gris", "roof", [168, 166, 160]),
                ("Carpintería aluminio", "metal", [190, 192, 194]),
                glass,
                ("Barandilla gris", "metal", [90, 92, 96]),
            ],
            BuildingStyle::CoastalHotel => [
                white,
                ("Zócalo gris piedra", "stone", [150, 146, 138]),
                ("Cubierta plana gris", "roof", [168, 166, 160]),
                ("Carpintería aluminio", "metal", [190, 192, 194]),
                ("Vidrio de control solar", "glass", [92, 128, 150]),
                ("Vidrio de barandilla", "glass", [150, 178, 186]),
            ],
        }
    }
}

/// What closes the windows on the outside.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Closure {
    /// Nothing: windows in the wall.
    #[default]
    None,
    /// Window grilles («rejas»), standing out of the wall.
    Grilles,
    /// A balcony with an iron railing in front of each window (balcony
    /// doors), from the first floor up.
    Balconies,
    /// Terraces along the whole façade, with a parapet, from the first
    /// floor up.
    Terraces,
    /// The whole façade in glass, floor by floor, with mullions.
    CurtainWall,
}

impl Closure {
    pub const ALL: [Closure; 5] = [
        Closure::None,
        Closure::Grilles,
        Closure::Balconies,
        Closure::Terraces,
        Closure::CurtainWall,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Closure::None => "Ventanas",
            Closure::Grilles => "Rejas",
            Closure::Balconies => "Balcones",
            Closure::Terraces => "Terrazas corridas",
            Closure::CurtainWall => "Muro cortina",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Closure::None => "none",
            Closure::Grilles => "grilles",
            Closure::Balconies => "balconies",
            Closure::Terraces => "terraces",
            Closure::CurtainWall => "curtain_wall",
        }
    }

    pub fn from_code(code: &str) -> Option<Closure> {
        Self::ALL.into_iter().find(|s| s.code() == code)
    }
}

/// The roof of a building of the surroundings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnvRoof {
    /// Flat, with a parapet.
    #[default]
    Flat,
    /// Tiled, falling to the long sides, with gables on the short ones
    /// (and on the party walls).
    Gable,
    /// Tiled, falling to every free side.
    Hip,
}

impl EnvRoof {
    pub const ALL: [EnvRoof; 3] = [EnvRoof::Flat, EnvRoof::Gable, EnvRoof::Hip];

    pub fn name(self) -> &'static str {
        match self {
            EnvRoof::Flat => "Plana",
            EnvRoof::Gable => "A dos aguas",
            EnvRoof::Hip => "A cuatro aguas",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            EnvRoof::Flat => "flat",
            EnvRoof::Gable => "gable",
            EnvRoof::Hip => "hip",
        }
    }

    pub fn from_code(code: &str) -> Option<EnvRoof> {
        Self::ALL.into_iter().find(|s| s.code() == code)
    }
}

/// The finishes of a dressed building, by what they are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DressMaterials {
    /// The walls.
    pub wall: MaterialId,
    /// The plinth, the cornice, the sills and the shop's fascia.
    pub trim: MaterialId,
    pub roof: MaterialId,
    /// The window frames and the shop front.
    pub joinery: MaterialId,
    pub glass: MaterialId,
    /// Grilles and railings.
    pub metal: MaterialId,
}

impl DressMaterials {
    pub fn all(&self) -> [MaterialId; 6] {
        [
            self.wall,
            self.trim,
            self.roof,
            self.joinery,
            self.glass,
            self.metal,
        ]
    }

    /// Their names, in the order of `all`.
    pub const ROLES: [&'static str; 6] = [
        "Fachada",
        "Zócalo y cornisa",
        "Cubierta",
        "Carpintería",
        "Vidrio",
        "Cerrajería",
    ];
}

/// How a mass by floors is dressed as a building of the surroundings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dress {
    pub style: BuildingStyle,
    /// The distance between window axes along a façade (m).
    pub bay: f64,
    pub window_width: f64,
    pub window_height: f64,
    /// The windows' sill over each floor (m).
    pub sill: f64,
    pub closure: Closure,
    /// How far balconies and terraces stand out (m).
    pub balcony_depth: f64,
    /// The ground floor is a shop (wide shop windows, a fascia above).
    pub shop: bool,
    pub roof: EnvRoof,
    /// The pitch of a tiled roof (rise per unit of run).
    pub pitch: f64,
    /// The height of the plinth (0: none).
    pub plinth: f64,
    /// A cornice at the top of the façade.
    pub cornice: bool,
    /// Round a courtyard (casa patio): the depth of the wings round it (0:
    /// none).
    pub patio: f64,
    pub materials: DressMaterials,
}

/// Shortest and longest bays.
pub const MIN_BAY: f64 = 1.2;
pub const MAX_BAY: f64 = 12.0;

impl Dress {
    /// The usual dress of a style, with its materials (as `materials`
    /// gives them).
    pub fn usual(style: BuildingStyle, materials: DressMaterials) -> Dress {
        let base = Dress {
            style,
            bay: 3.2,
            window_width: 1.2,
            window_height: 1.4,
            sill: 0.9,
            closure: Closure::None,
            balcony_depth: 0.7,
            shop: false,
            roof: EnvRoof::Flat,
            pitch: 0.35,
            plinth: 0.0,
            cornice: false,
            patio: 0.0,
            materials,
        };
        match style {
            BuildingStyle::Andalusian => Dress {
                bay: 3.4,
                window_width: 1.0,
                window_height: 1.5,
                closure: Closure::Grilles,
                roof: EnvRoof::Gable,
                pitch: 0.4,
                plinth: 0.8,
                cornice: true,
                ..base
            },
            BuildingStyle::Courtyard => Dress {
                bay: 3.6,
                window_width: 1.0,
                closure: Closure::Grilles,
                plinth: 0.6,
                patio: 4.0,
                ..base
            },
            BuildingStyle::Terraced => Dress {
                bay: 2.8,
                roof: EnvRoof::Gable,
                plinth: 0.5,
                ..base
            },
            BuildingStyle::SemiDetached => Dress {
                bay: 3.0,
                roof: EnvRoof::Hip,
                plinth: 0.5,
                ..base
            },
            BuildingStyle::Ensanche => Dress {
                bay: 3.0,
                window_width: 1.1,
                window_height: 2.3,
                sill: 0.0,
                closure: Closure::Balconies,
                balcony_depth: 0.6,
                shop: true,
                plinth: 0.0,
                cornice: true,
                ..base
            },
            BuildingStyle::Block1960s => Dress {
                bay: 3.6,
                window_width: 2.2,
                window_height: 2.2,
                sill: 0.0,
                closure: Closure::Terraces,
                balcony_depth: 1.2,
                shop: true,
                ..base
            },
            BuildingStyle::Detached => Dress {
                bay: 3.4,
                window_width: 1.4,
                window_height: 1.3,
                roof: EnvRoof::Hip,
                pitch: 0.3,
                plinth: 0.4,
                ..base
            },
            BuildingStyle::HistoricCentre => Dress {
                bay: 2.8,
                window_width: 1.0,
                window_height: 2.4,
                sill: 0.0,
                closure: Closure::Balconies,
                balcony_depth: 0.45,
                roof: EnvRoof::Gable,
                pitch: 0.45,
                plinth: 1.0,
                cornice: true,
                ..base
            },
            BuildingStyle::Offices => Dress {
                bay: 1.5,
                window_width: 1.5,
                window_height: 2.6,
                sill: 0.4,
                closure: Closure::CurtainWall,
                ..base
            },
            BuildingStyle::ShopAndFlats => Dress {
                bay: 3.2,
                shop: true,
                cornice: true,
                ..base
            },
            BuildingStyle::Warehouse => Dress {
                bay: 6.0,
                window_width: 4.0,
                window_height: 0.9,
                sill: 5.2,
                roof: EnvRoof::Gable,
                pitch: 0.12,
                plinth: 1.2,
                shop: true,
                ..base
            },
            BuildingStyle::Facility => Dress {
                bay: 4.2,
                window_width: 3.6,
                window_height: 1.6,
                sill: 1.0,
                plinth: 1.0,
                ..base
            },
            BuildingStyle::CoastalHotel => Dress {
                bay: 3.8,
                window_width: 2.6,
                window_height: 2.3,
                sill: 0.0,
                closure: Closure::Terraces,
                balcony_depth: 1.6,
                shop: true,
                ..base
            },
        }
    }

    /// Checks its measurements.
    pub fn validate(&self) -> Result<(), ModelError> {
        let ok = (MIN_BAY..=MAX_BAY).contains(&self.bay)
            && self.window_width > 0.2
            && self.window_width <= self.bay + 1e-9
            && self.window_height > 0.2
            && self.sill >= 0.0
            && (0.0..=3.0).contains(&self.balcony_depth)
            && (0.0..=2.0).contains(&self.pitch)
            && (0.0..=3.0).contains(&self.plinth)
            && (0.0..=20.0).contains(&self.patio)
            && [
                self.bay,
                self.window_width,
                self.window_height,
                self.sill,
                self.balcony_depth,
                self.pitch,
                self.plinth,
                self.patio,
            ]
            .iter()
            .all(|v| v.is_finite());
        if ok {
            Ok(())
        } else {
            Err(ModelError::InvalidDress)
        }
    }

    /// Its parameters as text, for the IFC (`key=value;…`).
    pub fn encode(&self) -> String {
        format!(
            "style={};bay={};window={}x{};sill={};closure={};balcony={};shop={};roof={};pitch={};plinth={};cornice={};patio={}",
            self.style.code(),
            self.bay,
            self.window_width,
            self.window_height,
            self.sill,
            self.closure.code(),
            self.balcony_depth,
            self.shop as u8,
            self.roof.code(),
            self.pitch,
            self.plinth,
            self.cornice as u8,
            self.patio,
        )
    }

    /// Reads what `encode` wrote, with these materials.
    pub fn decode(text: &str, materials: DressMaterials) -> Option<Dress> {
        let mut style = None;
        let mut d = Dress::usual(BuildingStyle::ShopAndFlats, materials);
        for part in text.split(';') {
            let (k, v) = part.split_once('=')?;
            let num = || v.parse::<f64>().ok();
            match k {
                "style" => {
                    let s = BuildingStyle::from_code(v)?;
                    style = Some(s);
                    d.style = s;
                }
                "bay" => d.bay = num()?,
                "window" => {
                    let (w, h) = v.split_once('x')?;
                    d.window_width = w.parse().ok()?;
                    d.window_height = h.parse().ok()?;
                }
                "sill" => d.sill = num()?,
                "closure" => d.closure = Closure::from_code(v)?,
                "balcony" => d.balcony_depth = num()?,
                "shop" => d.shop = v == "1",
                "roof" => d.roof = EnvRoof::from_code(v)?,
                "pitch" => d.pitch = num()?,
                "plinth" => d.plinth = num()?,
                "cornice" => d.cornice = v == "1",
                "patio" => d.patio = num()?,
                _ => {}
            }
        }
        style?;
        d.validate().ok()?;
        Some(d)
    }
}

/// Data to put a building of the surroundings.
#[derive(Debug, Clone, PartialEq)]
pub struct NewEnvBuilding {
    pub name: String,
    pub level: LevelId,
    /// Its contour (any turn).
    pub outline: Vec<Point2>,
    pub style: BuildingStyle,
    /// None: the style's usual.
    pub floors: Option<u32>,
    pub floor_height: Option<f64>,
    pub base_offset: f64,
}

/// How far two sides may be apart to make a party wall, and how much of
/// a side must touch.
pub const PARTY_GAP: f64 = 0.3;
pub const PARTY_SHARE: f64 = 0.5;

/// How far a street's axis may be in front of a façade for it to face
/// the street (m).
pub const STREET_REACH: f64 = 30.0;

/// Most floors of a building of the surroundings.
pub const MAX_ENV_FLOORS: u32 = 60;

impl Model {
    /// The material called `name`, or a new one with that category and
    /// colour (its texture by its category).
    pub fn env_material(&mut self, name: &str, category: &str, rgb: [u8; 3]) -> MaterialId {
        if let Some(m) = self.materials().find(|m| m.name == name) {
            return m.id;
        }
        let id = self.add_material(name, category, rgb, CutPattern::Solid);
        let texture = match category {
            "masonry" => Some(TextureKind::Brick),
            "stone" => Some(TextureKind::Stone),
            "concrete" => Some(TextureKind::Concrete),
            "render" => Some(TextureKind::Plaster),
            "roof" if name.starts_with("Teja") => Some(TextureKind::Tile),
            "metal" if name.starts_with("Chapa") => Some(TextureKind::Metal),
            _ => None,
        };
        if texture.is_some() {
            let _ = self.set_appearance(
                id,
                Appearance {
                    texture,
                    ..Appearance::default()
                },
            );
        }
        id
    }

    /// The finishes of a style, found in the project by their names or
    /// made.
    pub fn style_materials(&mut self, style: BuildingStyle) -> DressMaterials {
        let f = style.finishes();
        let ids = f.map(|(n, c, rgb)| self.env_material(n, c, rgb));
        DressMaterials {
            wall: ids[0],
            trim: ids[1],
            roof: ids[2],
            joinery: ids[3],
            glass: ids[4],
            metal: ids[5],
        }
    }

    /// The usual dress of a style, with its materials.
    pub fn usual_dress(&mut self, style: BuildingStyle) -> Dress {
        let m = self.style_materials(style);
        Dress::usual(style, m)
    }

    /// Puts a building of the surroundings: a mass by floors with its
    /// style's dress.
    pub fn add_env_building(&mut self, spec: NewEnvBuilding) -> Result<MassId, ModelError> {
        let (floors, fh) = spec.style.usual_floors();
        let floors = spec.floors.unwrap_or(floors);
        let fh = spec.floor_height.unwrap_or(fh);
        if floors == 0 || floors > MAX_ENV_FLOORS || !(2.0..=12.0).contains(&fh) {
            return Err(ModelError::InvalidDress);
        }
        let dress = self.usual_dress(spec.style);
        let id = self.add_mass(NewMass {
            name: spec.name,
            level: spec.level,
            shape: MassShape::Prism(spec.outline),
            base_offset: spec.base_offset,
            height: fh * floors as f64,
            floor_height: Some(fh),
            material: dress.materials.wall,
        })?;
        self.set_mass_dress(id, Some(dress))?;
        Ok(id)
    }

    /// Dresses a mass (or undresses it with `None`). Only a contour mass
    /// by floors that is not a building can be dressed.
    pub fn set_mass_dress(&mut self, id: MassId, dress: Option<Dress>) -> Result<(), ModelError> {
        let m = self
            .masses
            .get(&id)
            .ok_or(ModelError::UnknownElement(id.0))?;
        if let Some(d) = &dress {
            d.validate()?;
            let ok = matches!(m.shape, MassShape::Prism(_))
                && m.floor_height.is_some()
                && m.building.is_none();
            if !ok {
                return Err(ModelError::InvalidDress);
            }
            if d.materials
                .all()
                .iter()
                .any(|x| !self.materials.contains_key(x))
            {
                return Err(ModelError::InvalidDress);
            }
        }
        let wall = dress.map(|d| d.materials.wall);
        let m = self.masses.get_mut(&id).expect("checked");
        m.dress = dress;
        if let Some(w) = wall {
            m.material = w;
        }
        Ok(())
    }

    /// Gives these masses a style (keeping their floors), each with the
    /// style's usual dress.
    pub fn restyle_masses(
        &mut self,
        ids: &[MassId],
        style: BuildingStyle,
    ) -> Result<usize, ModelError> {
        let dress = self.usual_dress(style);
        let mut n = 0;
        for id in ids {
            let Some(m) = self.masses.get(id) else {
                continue;
            };
            if !matches!(m.shape, MassShape::Prism(_)) || m.building.is_some() {
                continue;
            }
            if m.floor_height.is_none() {
                let (floors, fh) = style.usual_floors();
                let floors = ((m.height / fh).round() as u32).clamp(1, floors.max(1) * 4);
                self.edit_mass(*id, |x| {
                    x.floor_height = Some(fh);
                    x.height = fh * floors as f64;
                })?;
            }
            self.set_mass_dress(*id, Some(dress))?;
            n += 1;
        }
        Ok(n)
    }

    /// The dressed masses.
    pub fn env_buildings(&self) -> impl Iterator<Item = &Mass> {
        self.masses.values().filter(|m| m.dress.is_some())
    }

    /// Which sides of a mass's contour are party walls: against another
    /// mass's side, facing it, over at least half of its length.
    pub fn party_sides(&self, id: MassId) -> Vec<bool> {
        let Some(m) = self.masses.get(&id) else {
            return Vec::new();
        };
        let MassShape::Prism(p) = &m.shape else {
            return Vec::new();
        };
        let others: Vec<(Point2, Point2)> = self
            .masses
            .values()
            .filter(|o| o.id != id && o.level == m.level)
            .filter_map(|o| match &o.shape {
                MassShape::Prism(q) => Some(q),
                _ => None,
            })
            .flat_map(|q| (0..q.len()).map(move |i| (q[i], q[(i + 1) % q.len()])))
            .collect();
        let n = p.len();
        (0..n)
            .map(|i| {
                let (a, b) = (p[i], p[(i + 1) % n]);
                let len = a.distance(b);
                let Some(u) = (b - a).normalized() else {
                    return false;
                };
                let nrm = u.perp_left();
                let touched: f64 = others
                    .iter()
                    .filter_map(|(c, d)| {
                        let v = (*d - *c).normalized()?;
                        // Facing: running the other way, in line.
                        if v.dot(u) > -0.98 {
                            return None;
                        }
                        let off = |q: Point2| (q - a).dot(nrm).abs();
                        if off(*c) > PARTY_GAP || off(*d) > PARTY_GAP {
                            return None;
                        }
                        let (s0, s1) = ((*c - a).dot(u), (*d - a).dot(u));
                        let (lo, hi) = (s0.min(s1).max(0.0), s0.max(s1).min(len));
                        (hi > lo).then_some(hi - lo)
                    })
                    .sum();
                len > 1e-9 && touched >= PARTY_SHARE * len
            })
            .collect()
    }

    /// Which sides of a mass's contour face a street: free sides with a
    /// road's axis in front of them within `STREET_REACH`. With no road
    /// near, its longest free side. They take the shop fronts and the
    /// big door.
    pub fn street_sides(&self, id: MassId) -> Vec<bool> {
        let Some(m) = self.masses.get(&id) else {
            return Vec::new();
        };
        let MassShape::Prism(p) = &m.shape else {
            return Vec::new();
        };
        let mut p = p.clone();
        if signed_area(&p) < 0.0 {
            p.reverse();
        }
        let party = self.party_sides(id);
        let n = p.len();
        let roads: Vec<&Road> = self.roads_on_level(m.level).collect();
        let mut out: Vec<bool> = (0..n)
            .map(|i| {
                if party.get(i).copied().unwrap_or(false) {
                    return false;
                }
                let (a, b) = (p[i], p[(i + 1) % n]);
                let Some(u) = (b - a).normalized() else {
                    return false;
                };
                let out = -u.perp_left();
                let mid = (a + b) * 0.5;
                roads.iter().any(|r| {
                    let q = r.axis_at(r.station_of(mid)).0;
                    let d = q - mid;
                    d.length() <= STREET_REACH && d.dot(out) > 0.5 * d.length()
                })
            })
            .collect();
        if !out.iter().any(|x| *x) {
            let longest = (0..n)
                .filter(|i| !party.get(*i).copied().unwrap_or(false))
                .max_by(|x, y| p[*x].distance(p[(*x + 1) % n]).total_cmp(&p[*y].distance(p[(*y + 1) % n])));
            if let Some(i) = longest {
                out[i] = true;
            }
        }
        out
    }

    /// The building of the surroundings for a plot: its buildable area.
    pub fn plot_outline_for_building(&self, plot: PlotId) -> Option<Vec<Point2>> {
        self.plot(plot)?.buildable()
    }
}

/// The floor count in Roman numerals, as plans write it (I to LX).
pub fn roman(n: u32) -> String {
    let table = [
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut n = n.min(MAX_ENV_FLOORS);
    let mut s = String::new();
    for (v, r) in table {
        while n >= v {
            s.push_str(r);
            n -= v;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point2> {
        vec![
            Point2::new(x0, y0),
            Point2::new(x1, y0),
            Point2::new(x1, y1),
            Point2::new(x0, y1),
        ]
    }

    #[test]
    fn houses_in_a_row_share_party_walls() {
        let mut m = Model::new("T");
        let l = m.add_level("Planta baja", 0.0);
        let mut ids = Vec::new();
        for i in 0..3 {
            let x = i as f64 * 6.0;
            ids.push(
                m.add_env_building(NewEnvBuilding {
                    name: format!("Casa {i}"),
                    level: l,
                    outline: rect(x, 0.0, x + 6.0, 12.0),
                    style: BuildingStyle::Andalusian,
                    floors: None,
                    floor_height: None,
                    base_offset: 0.0,
                })
                .unwrap(),
            );
        }
        // Sides: front (y = 0), right, back, left.
        assert_eq!(m.party_sides(ids[0]), vec![false, true, false, false]);
        assert_eq!(m.party_sides(ids[1]), vec![false, true, false, true]);
        assert_eq!(m.party_sides(ids[2]), vec![false, false, false, true]);
        let b = m.mass(ids[1]).unwrap();
        assert_eq!(b.floors(), 2);
        let d = b.dress.unwrap();
        assert_eq!(d.closure, Closure::Grilles);
        assert_eq!(m.material(d.materials.wall).unwrap().name, "Cal blanca");
        // The same finishes for all three.
        assert_eq!(
            m.mass(ids[0]).unwrap().dress.unwrap().materials,
            d.materials
        );
        // A whole row restyled at once keeps its floors.
        assert_eq!(m.restyle_masses(&ids, BuildingStyle::Ensanche).unwrap(), 3);
        let b = m.mass(ids[1]).unwrap();
        assert_eq!(b.dress.unwrap().style, BuildingStyle::Ensanche);
        assert_eq!(b.floors(), 2);
        // Encoded and read back.
        let text = d.encode();
        assert_eq!(Dress::decode(&text, d.materials), Some(d));
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back.mass(ids[1]).unwrap().dress, b.dress);
    }

    #[test]
    fn the_shops_face_the_street() {
        let mut m = Model::new("T");
        let l = m.add_level("Planta baja", 0.0);
        let mat = m.add_material("Asfalto", "asphalt", [60, 60, 60], CutPattern::Solid);
        // A corner block: a street in front (south) and one to the east.
        let block = m
            .add_env_building(NewEnvBuilding {
                name: "Esquina".into(),
                level: l,
                outline: rect(0.0, 0.0, 20.0, 15.0),
                style: BuildingStyle::Ensanche,
                floors: None,
                floor_height: None,
                base_offset: 0.0,
            })
            .unwrap();
        // With no street, its longest free side.
        assert_eq!(m.street_sides(block), vec![false, false, true, false]);
        for axis in [
            vec![Point2::new(-10.0, -6.0), Point2::new(40.0, -6.0)],
            vec![Point2::new(26.0, -20.0), Point2::new(26.0, 40.0)],
        ] {
            m.add_road(NewRoad {
                name: "Calle".into(),
                level: l,
                axis,
                section: RoadSection::default(),
                top_offset: 0.0,
                profile: Vec::new(),
                changes: Vec::new(),
                pavement: mat,
                kerb_material: mat,
                sidewalk_material: mat,
            })
            .unwrap();
        }
        assert_eq!(m.street_sides(block), vec![true, true, false, false]);
    }

    #[test]
    fn roman_numerals() {
        assert_eq!(roman(1), "I");
        assert_eq!(roman(4), "IV");
        assert_eq!(roman(9), "IX");
        assert_eq!(roman(14), "XIV");
        assert_eq!(roman(48), "XLVIII");
    }
}
