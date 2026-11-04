//! The skins of the places: what each builds with and how. A skin is data
//! only; the typologies read it (see `types`).

/// The form of a roof.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Roof {
    /// A flat roof behind a parapet (azotea).
    Flat,
    /// Two slopes, the ridge along the street.
    Gable,
    /// Two slopes, the gable to the street (the Low Countries).
    GableToStreet,
    /// Four slopes.
    Hip,
    /// A mansard: a steep slope with dormers and a low one over it.
    Mansard,
    /// Four slopes of dark tiles with deep eaves and raised ridge ends.
    Asian,
    /// One slope, against a party wall (only for parts of a plan).
    Pent,
}

/// What covers a pitched roof.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cover {
    /// Curved tiles, course by course.
    Curved,
    /// Slates, plain tiles or shingles: courses across the slope.
    Courses,
    /// Sheet metal: seams down the slope.
    Seam,
    /// Nothing over the slab (seen from afar).
    Plain,
}

/// The outline of a gable to the street.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gable {
    Plain,
    Stepped,
    Bell,
    /// One of the three, by the building.
    Varied,
}

/// How the windows open.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sash {
    /// Leaves on hinges.
    Casement,
    /// Two sashes sliding up and down (guillotina).
    Sash,
    /// Leaves sliding sideways.
    Sliding,
}

/// What closes the windows from outside.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shut {
    None,
    /// Roller blinds with their box.
    Roller,
    /// Louvred shutters (persianas mallorquinas, volets, persiane).
    Louvred,
    /// Shutters of boards.
    Board,
}

/// What frames the head of a window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Head {
    Plain,
    /// A band round the hole (recercado).
    Band,
    /// A band and a small cornice over it.
    Hood,
    /// A band, a cornice and a pediment on the main floor.
    Pediment,
    /// A round-headed hole.
    Arch,
    /// A band with a keystone.
    Keystone,
}

/// The balconies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Balc {
    None,
    /// Iron balconies, one to each balcony door.
    Iron,
    /// A balcony along the whole front on the main floor and the top one,
    /// French windows with their rail on the others.
    Continuous,
    /// A wooden balcony with its roof (Latin America, the Canaries).
    Covered,
    /// A glazed gallery over the whole front (Galicia, the Cantabrian).
    Gallery,
    /// A bay window (the United Kingdom, San Francisco).
    Bay,
    /// Iron fire escapes (New York).
    FireEscape,
    /// A verandah of two floors with cast iron lace (Sydney, New Orleans).
    Lace,
    /// A closed bay of wooden lattice (the Maghreb, the Middle East).
    Mashrabiya,
    /// Cages of bars round the windows (China, Korea).
    Cage,
    /// A balcony to each flat, some glazed in (the blocks of the East).
    Glazed,
    /// Balconies along each floor with solid fronts (Japan).
    Slab,
}

/// How the ground floor meets the street.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ground {
    /// A door and windows.
    Door,
    /// A shop and the portal.
    Shop,
    /// Raised over a basement, steps up to the door.
    Stoop,
    /// A covered walk under the floors (soportales, portici, qilou).
    Arcade,
    /// A garage door and the house's door.
    Garage,
    /// Wooden lattice over the whole ground floor (machiya).
    Lattice,
    /// Open on columns, cars under the flats (Korea).
    Pilotis,
}

/// How the walls end at the top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Top {
    None,
    Band,
    /// A deep cornice on brackets (New York, San Francisco).
    Brackets,
    /// A heavy moulded cornice (Paris).
    Heavy,
    /// Two rows of tiles (the south of Spain).
    Tiles,
    /// A balustrade over the cornice.
    Balustrade,
}

/// The porch of a house.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Porch {
    None,
    /// A small roof on two columns over the door.
    Portico,
    /// A verandah along the front.
    Veranda,
    /// A verandah all round (the Queenslander).
    Wraparound,
    /// Arches along the front (loggia, porche).
    Arcade,
}

/// What closes the plot of a house.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fence {
    None,
    Picket,
    Wall,
    Hedge,
    Iron,
}

// What a place adds to its buildings.
pub const QUOINS: u32 = 1;
pub const BELT: u32 = 1 << 1;
pub const FLOWERS: u32 = 1 << 2;
pub const AC: u32 = 1 << 3;
pub const CHIMNEYS: u32 = 1 << 4;
pub const DORMERS: u32 = 1 << 5;
pub const SIGNS: u32 = 1 << 6;
pub const HOIST: u32 = 1 << 7;
pub const TANK: u32 = 1 << 8;
pub const LAUNDRY: u32 = 1 << 9;
pub const PENT: u32 = 1 << 10;
pub const WATER_TOWER: u32 = 1 << 11;
pub const PALM: u32 = 1 << 12;
pub const RUSTIC: u32 = 1 << 13;
pub const GRILLES: u32 = 1 << 14;
pub const AWNINGS: u32 = 1 << 15;
pub const PLANTS: u32 = 1 << 16;
pub const SOLAR: u32 = 1 << 17;
pub const ALGARVE: u32 = 1 << 18;
pub const CORNER_BOARDS: u32 = 1 << 19;
pub const TREE: u32 = 1 << 20;

/// The block of flats of a place, where it is not the street's house
/// made bigger.
#[derive(Clone, Copy, Debug)]
pub struct Block {
    /// Its walls (empty: the skin's).
    pub walls: &'static [&'static str],
    pub balc: Balc,
    pub roof: Roof,
    pub ground: Ground,
    pub top: Top,
    pub floors: usize,
    /// Added to the skin's extras.
    pub extras: u32,
}

/// The detached house of a place.
#[derive(Clone, Copy, Debug)]
pub struct House {
    /// Its walls (empty: the skin's).
    pub walls: &'static [&'static str],
    pub roof: Roof,
    pub cover: Cover,
    /// Its roof's material (empty: the skin's).
    pub roof_mat: &'static str,
    pub pitch: f64,
    pub porch: Porch,
    pub fence: Fence,
    pub shut: Shut,
    pub balc: Balc,
    /// Its floor over the ground (on stilts).
    pub raised: f64,
    pub floors: usize,
    pub extras: u32,
}

/// The skin of a place.
#[derive(Clone, Copy, Debug)]
pub struct Skin {
    /// Its name in files and parameters.
    pub key: &'static str,
    /// Its name for Antonio.
    pub name: &'static str,
    /// The part of the world it belongs to.
    pub region: &'static str,
    pub walls: &'static [&'static str],
    /// The ground floor or the plinth (empty: the wall's).
    pub base: &'static [&'static str],
    pub trims: &'static [&'static str],
    /// A second colour of the trims (bands, doors' surrounds).
    pub accents: &'static [&'static str],
    pub frames: &'static [&'static str],
    pub shutters: &'static [&'static str],
    pub doors: &'static [&'static str],
    pub roof_mat: &'static str,
    pub roof: Roof,
    pub cover: Cover,
    pub pitch: f64,
    pub eaves: f64,
    pub gable: Gable,
    pub sash: Sash,
    pub leaves: usize,
    /// Panes of each leaf: across and up (of each sash for a sash window).
    pub panes: (usize, usize),
    /// The windows of the floors: width and height.
    pub win: (f64, f64),
    /// From axis to axis of the windows.
    pub bay: f64,
    pub shut: Shut,
    pub head: Head,
    pub balc: Balc,
    pub ground: Ground,
    /// The ground floor over the street (the stoop's height).
    pub lift: f64,
    pub top: Top,
    /// The heights of the ground floor and of the others.
    pub storey: (f64, f64),
    /// The usual floors of the street's house.
    pub floors: usize,
    pub extras: u32,
    pub block: Block,
    pub house: House,
}

impl Skin {
    pub fn has(&self, flag: u32) -> bool {
        self.extras & flag != 0
    }
}

const NO_BLOCK: Block = Block {
    walls: &[],
    balc: Balc::Iron,
    roof: Roof::Flat,
    ground: Ground::Shop,
    top: Top::Band,
    floors: 6,
    extras: 0,
};

const NO_HOUSE: House = House {
    walls: &[],
    roof: Roof::Gable,
    cover: Cover::Curved,
    roof_mat: "",
    pitch: 0.45,
    porch: Porch::None,
    fence: Fence::Wall,
    shut: Shut::None,
    balc: Balc::None,
    raised: 0.0,
    floors: 2,
    extras: 0,
};

const BASE: Skin = Skin {
    key: "",
    name: "",
    region: "",
    walls: &["w_white"],
    base: &[],
    trims: &["w_white"],
    accents: &[],
    frames: &["f_white"],
    shutters: &["f_green"],
    doors: &["f_brown"],
    roof_mat: "r_curved",
    roof: Roof::Gable,
    cover: Cover::Curved,
    pitch: 0.45,
    eaves: 0.3,
    gable: Gable::Plain,
    sash: Sash::Casement,
    leaves: 2,
    panes: (1, 1),
    win: (1.0, 1.6),
    bay: 2.4,
    shut: Shut::None,
    head: Head::Band,
    balc: Balc::None,
    ground: Ground::Door,
    lift: 0.0,
    top: Top::Band,
    storey: (3.4, 3.0),
    floors: 3,
    extras: 0,
    block: NO_BLOCK,
    house: NO_HOUSE,
};

/// The skins, in the order they are offered.
pub const SKINS: &[Skin] = &[
    Skin {
        key: "andalucia",
        name: "Andalucía",
        region: "España",
        walls: &["w_white"],
        base: &["w_almagra", "w_grey", "w_yellow"],
        trims: &["w_yellow", "w_almagra", "w_white"],
        frames: &["f_green", "f_brown", "f_white"],
        shutters: &["f_green"],
        doors: &["f_brown", "f_green"],
        pitch: 0.4,
        ..ANDALUSIA
    },
    Skin {
        key: "castilla",
        name: "Castilla y Madrid",
        region: "España",
        walls: &["k_red", "w_ochre", "w_cream", "w_almagra"],
        base: &["s_granite"],
        trims: &["w_white", "s_lime"],
        frames: &["f_white", "f_green", "f_brown"],
        doors: &["f_brown", "f_green"],
        panes: (1, 2),
        win: (1.0, 1.9),
        bay: 2.6,
        shut: Shut::Roller,
        balc: Balc::Iron,
        ground: Ground::Shop,
        storey: (3.6, 3.0),
        floors: 5,
        extras: BELT | AC | PLANTS,
        block: Block {
            walls: &["k_red", "w_cream"],
            balc: Balc::Glazed,
            floors: 8,
            extras: TANK | AC,
            ..NO_BLOCK
        },
        house: House {
            walls: &["k_red", "w_cream"],
            roof: Roof::Hip,
            fence: Fence::Iron,
            shut: Shut::Roller,
            balc: Balc::Iron,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "norte",
        name: "Galicia y el Cantábrico",
        region: "España",
        walls: &["w_white", "w_cream", "s_granite"],
        base: &["s_granite"],
        trims: &["s_granite"],
        frames: &["f_white"],
        doors: &["f_brown", "f_green", "f_red"],
        roof_mat: "r_curved",
        pitch: 0.5,
        panes: (1, 3),
        win: (1.0, 1.6),
        bay: 2.3,
        head: Head::Band,
        balc: Balc::Gallery,
        ground: Ground::Shop,
        storey: (3.3, 2.9),
        floors: 4,
        extras: QUOINS,
        block: Block {
            walls: &["w_white", "w_cream"],
            balc: Balc::Gallery,
            roof: Roof::Hip,
            floors: 6,
            ..NO_BLOCK
        },
        house: House {
            walls: &["w_white", "s_rubble"],
            roof: Roof::Hip,
            fence: Fence::Wall,
            extras: TREE,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "mediterraneo",
        name: "Cataluña y Levante",
        region: "España",
        walls: &["w_ochre", "w_cream", "w_pink", "w_yellow", "w_salmon"],
        base: &["s_lime", "w_grey"],
        trims: &["w_white", "s_lime"],
        frames: &["f_brown", "f_green"],
        shutters: &["f_green", "f_brown"],
        doors: &["f_brown"],
        roof_mat: "r_gravel",
        roof: Roof::Flat,
        win: (1.1, 2.3),
        bay: 2.5,
        shut: Shut::Louvred,
        head: Head::Hood,
        balc: Balc::Iron,
        ground: Ground::Shop,
        top: Top::Balustrade,
        storey: (4.0, 3.1),
        floors: 6,
        extras: BELT | PLANTS | TANK | AC | AWNINGS,
        block: Block {
            walls: &["w_cream", "w_pink"],
            balc: Balc::Glazed,
            floors: 9,
            extras: TANK,
            ..NO_BLOCK
        },
        house: House {
            walls: &["w_white", "w_cream"],
            porch: Porch::Portico,
            shut: Shut::Louvred,
            extras: TREE,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "lisboa",
        name: "Lisboa y Portugal",
        region: "Península",
        walls: &["c_azulejo", "w_yellow", "w_pink", "w_white", "w_blue"],
        base: &["s_lime"],
        trims: &["s_lime", "w_white"],
        frames: &["f_white", "f_green"],
        doors: &["f_green", "f_brown", "f_red"],
        pitch: 0.5,
        panes: (1, 3),
        win: (1.0, 2.2),
        bay: 2.1,
        balc: Balc::Iron,
        ground: Ground::Shop,
        storey: (3.6, 3.0),
        floors: 4,
        extras: DORMERS | LAUNDRY | PLANTS | QUOINS,
        block: Block {
            walls: &["w_pink", "w_yellow", "w_cream"],
            balc: Balc::Glazed,
            floors: 7,
            ..NO_BLOCK
        },
        house: House {
            walls: &["w_white", "w_yellow"],
            roof: Roof::Hip,
            floors: 2,
            extras: TREE,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "algarve",
        name: "Algarve",
        region: "Península",
        walls: &["w_white"],
        base: &["w_blue", "w_yellow", "w_ochre"],
        trims: &["w_yellow", "w_blue", "w_ochre"],
        frames: &["f_green", "f_brown", "f_blue"],
        doors: &["f_green", "f_blue", "f_brown"],
        roof_mat: "r_gravel",
        roof: Roof::Flat,
        panes: (1, 2),
        win: (0.9, 1.4),
        bay: 2.2,
        ground: Ground::Door,
        storey: (3.0, 2.8),
        floors: 2,
        extras: ALGARVE | PLANTS | TANK,
        block: Block {
            balc: Balc::Glazed,
            floors: 5,
            ..NO_BLOCK
        },
        house: House {
            roof: Roof::Flat,
            floors: 1,
            extras: ALGARVE | PALM,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "paris",
        name: "París",
        region: "Europa",
        walls: &["s_lime"],
        base: &["s_lime"],
        trims: &["s_lime", "w_cream"],
        frames: &["f_white", "f_greyblue"],
        shutters: &["f_greyblue", "f_white"],
        doors: &["f_green", "f_blue", "f_black", "f_brown"],
        roof_mat: "r_zinc",
        roof: Roof::Mansard,
        cover: Cover::Seam,
        panes: (1, 3),
        win: (1.1, 2.5),
        bay: 2.3,
        head: Head::Pediment,
        balc: Balc::Continuous,
        ground: Ground::Shop,
        top: Top::Heavy,
        storey: (4.2, 3.0),
        floors: 6,
        extras: RUSTIC | CHIMNEYS | DORMERS | BELT,
        block: Block {
            balc: Balc::Continuous,
            roof: Roof::Mansard,
            top: Top::Heavy,
            floors: 7,
            ..NO_BLOCK
        },
        house: House {
            walls: &["w_cream", "w_white", "s_rubble"],
            roof: Roof::Hip,
            cover: Cover::Courses,
            roof_mat: "r_slate",
            pitch: 0.9,
            fence: Fence::Iron,
            shut: Shut::Louvred,
            extras: TREE | CHIMNEYS,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "italia",
        name: "Italia",
        region: "Europa",
        walls: &["w_ochre", "w_terracotta", "w_yellow", "w_pink", "w_salmon"],
        base: &["s_lime", "w_grey"],
        trims: &["w_white", "s_lime", "w_cream"],
        frames: &["f_brown", "f_white"],
        shutters: &["f_green", "f_brown"],
        doors: &["f_brown", "f_green"],
        roof: Roof::Hip,
        pitch: 0.35,
        eaves: 0.7,
        win: (1.0, 1.8),
        bay: 2.6,
        shut: Shut::Louvred,
        balc: Balc::Iron,
        ground: Ground::Shop,
        storey: (3.8, 3.2),
        floors: 4,
        extras: FLOWERS | LAUNDRY | PLANTS | BELT,
        block: Block {
            balc: Balc::Glazed,
            roof: Roof::Hip,
            floors: 7,
            ..NO_BLOCK
        },
        house: House {
            walls: &["w_ochre", "s_rubble", "w_cream"],
            roof: Roof::Hip,
            porch: Porch::Arcade,
            fence: Fence::Hedge,
            shut: Shut::Louvred,
            extras: TREE,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "reino_unido",
        name: "Reino Unido",
        region: "Europa",
        walls: &["k_yellow", "k_red"],
        base: &["w_white", "w_cream"],
        trims: &["w_white", "s_lime"],
        frames: &["f_white"],
        doors: &["f_black", "f_blue", "f_red", "f_green", "f_yellow"],
        roof_mat: "r_slate",
        cover: Cover::Courses,
        pitch: 0.7,
        eaves: 0.15,
        sash: Sash::Sash,
        leaves: 1,
        panes: (3, 2),
        win: (1.0, 1.9),
        bay: 2.0,
        head: Head::Hood,
        balc: Balc::Bay,
        ground: Ground::Stoop,
        lift: 0.6,
        top: Top::Band,
        storey: (3.0, 2.9),
        floors: 3,
        extras: CHIMNEYS,
        block: Block {
            walls: &["k_red", "k_dark"],
            balc: Balc::None,
            ground: Ground::Door,
            floors: 5,
            ..NO_BLOCK
        },
        house: House {
            walls: &["k_red", "w_white", "w_cream"],
            cover: Cover::Courses,
            roof_mat: "r_tile",
            pitch: 0.8,
            porch: Porch::Portico,
            fence: Fence::Hedge,
            balc: Balc::Bay,
            extras: CHIMNEYS | TREE,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "centroeuropa",
        name: "Centroeuropa",
        region: "Europa",
        walls: &[
            "w_yellow", "w_pink", "w_green", "w_cream", "w_blue", "w_pearl",
        ],
        base: &["w_grey", "s_ashlar"],
        trims: &["w_white", "w_cream"],
        frames: &["f_white", "f_brown"],
        shutters: &["f_green", "f_brown"],
        doors: &["f_brown", "f_green"],
        roof_mat: "r_tile",
        cover: Cover::Courses,
        pitch: 1.0,
        eaves: 0.4,
        panes: (1, 2),
        win: (1.0, 1.6),
        bay: 2.2,
        ground: Ground::Shop,
        storey: (3.6, 3.0),
        floors: 4,
        extras: DORMERS | FLOWERS | BELT,
        block: Block {
            balc: Balc::Glazed,
            roof: Roof::Hip,
            floors: 6,
            ..NO_BLOCK
        },
        house: House {
            walls: &["w_white", "w_cream"],
            cover: Cover::Courses,
            pitch: 0.55,
            shut: Shut::Louvred,
            balc: Balc::Covered,
            fence: Fence::Picket,
            extras: FLOWERS | TREE,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "paises_bajos",
        name: "Países Bajos y Bélgica",
        region: "Europa",
        walls: &["k_dark", "k_red"],
        base: &["k_dark"],
        trims: &["w_white", "s_lime"],
        frames: &["f_white"],
        shutters: &["f_green", "f_red"],
        doors: &["f_green", "f_black", "f_red", "f_blue"],
        roof_mat: "r_grey",
        roof: Roof::GableToStreet,
        cover: Cover::Courses,
        pitch: 1.4,
        eaves: 0.1,
        gable: Gable::Varied,
        sash: Sash::Sash,
        leaves: 1,
        panes: (2, 2),
        win: (1.1, 2.0),
        bay: 1.8,
        head: Head::Band,
        ground: Ground::Stoop,
        lift: 0.45,
        top: Top::None,
        storey: (3.4, 2.9),
        floors: 4,
        extras: HOIST,
        block: Block {
            balc: Balc::Glazed,
            roof: Roof::Flat,
            ground: Ground::Door,
            floors: 5,
            ..NO_BLOCK
        },
        house: House {
            walls: &["k_red", "k_dark"],
            cover: Cover::Courses,
            roof_mat: "r_tile",
            pitch: 1.0,
            fence: Fence::Hedge,
            extras: TREE,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "nordico",
        name: "Países nórdicos",
        region: "Europa",
        walls: &[
            "w_yellow", "w_ochre", "w_pink", "w_green", "w_blue", "w_cream",
        ],
        base: &["w_grey"],
        trims: &["w_white"],
        frames: &["f_white"],
        doors: &["f_green", "f_blue", "f_brown"],
        roof_mat: "r_tile",
        cover: Cover::Courses,
        pitch: 0.9,
        eaves: 0.3,
        panes: (1, 3),
        win: (0.9, 1.5),
        bay: 2.0,
        ground: Ground::Shop,
        storey: (3.2, 2.8),
        floors: 4,
        extras: DORMERS | CHIMNEYS | BELT,
        block: Block {
            walls: &["k_red", "w_yellow"],
            balc: Balc::Glazed,
            roof: Roof::Gable,
            ground: Ground::Door,
            floors: 5,
            ..NO_BLOCK
        },
        house: House {
            walls: &["b_falu", "b_yellow", "b_white"],
            cover: Cover::Courses,
            roof_mat: "r_tile",
            pitch: 0.9,
            porch: Porch::Portico,
            fence: Fence::Picket,
            extras: CORNER_BOARDS | TREE,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "nueva_york",
        name: "Nueva York y Chicago",
        region: "América",
        walls: &["s_brown", "s_brown", "k_dark"],
        base: &["s_brown"],
        trims: &["s_brown", "w_pearl"],
        frames: &["f_black", "f_white", "f_brown"],
        doors: &["f_brown", "f_black", "f_green"],
        roof_mat: "r_gravel",
        roof: Roof::Flat,
        sash: Sash::Sash,
        leaves: 1,
        win: (1.0, 2.0),
        bay: 2.0,
        head: Head::Hood,
        ground: Ground::Stoop,
        lift: 1.6,
        top: Top::Brackets,
        storey: (3.4, 3.1),
        floors: 4,
        extras: AC,
        block: Block {
            walls: &["k_red", "k_dark", "k_yellow"],
            balc: Balc::FireEscape,
            ground: Ground::Shop,
            top: Top::Brackets,
            floors: 6,
            extras: WATER_TOWER,
            roof: Roof::Flat,
        },
        house: House {
            walls: &["b_white", "b_grey", "k_red"],
            cover: Cover::Courses,
            roof_mat: "r_shingle",
            pitch: 0.7,
            porch: Porch::Veranda,
            fence: Fence::Iron,
            extras: TREE,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "eeuu",
        name: "Estados Unidos rural",
        region: "América",
        walls: &["k_red", "k_dark", "w_cream"],
        trims: &["w_white", "w_cream"],
        frames: &["f_white", "f_black"],
        shutters: &["f_black", "f_green", "f_blue"],
        doors: &["f_black", "f_green", "f_red"],
        roof_mat: "r_gravel",
        roof: Roof::Flat,
        sash: Sash::Sash,
        leaves: 1,
        panes: (1, 2),
        win: (1.0, 1.9),
        bay: 2.4,
        head: Head::Arch,
        ground: Ground::Shop,
        top: Top::Brackets,
        storey: (4.0, 3.4),
        floors: 2,
        extras: AWNINGS,
        block: Block {
            balc: Balc::None,
            ground: Ground::Shop,
            floors: 4,
            ..NO_BLOCK
        },
        house: House {
            walls: &["b_white", "b_yellow", "b_blue", "b_grey"],
            cover: Cover::Courses,
            roof_mat: "r_shingle",
            pitch: 0.7,
            porch: Porch::Veranda,
            fence: Fence::Picket,
            shut: Shut::Louvred,
            extras: CORNER_BOARDS | CHIMNEYS | TREE,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "california",
        name: "California",
        region: "América",
        walls: &["w_white", "w_cream", "w_salmon"],
        trims: &["w_cream", "w_white"],
        frames: &["f_black", "f_brown", "f_white"],
        shutters: &["f_brown", "f_green"],
        doors: &["f_brown"],
        pitch: 0.35,
        eaves: 0.4,
        panes: (2, 3),
        win: (1.0, 1.8),
        bay: 2.8,
        head: Head::Arch,
        balc: Balc::Iron,
        ground: Ground::Arcade,
        top: Top::None,
        storey: (3.6, 3.0),
        floors: 2,
        extras: PALM | PLANTS,
        block: Block {
            balc: Balc::Iron,
            roof: Roof::Hip,
            ground: Ground::Shop,
            floors: 4,
            ..NO_BLOCK
        },
        house: House {
            roof: Roof::Hip,
            pitch: 0.35,
            porch: Porch::Arcade,
            fence: Fence::Hedge,
            floors: 1,
            extras: PALM,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "san_francisco",
        name: "San Francisco",
        region: "América",
        walls: &["b_blue", "b_green", "b_yellow", "b_white", "b_grey"],
        trims: &["b_white", "w_cream"],
        accents: &["f_red", "f_yellow", "f_greyblue", "f_blue"],
        frames: &["f_white"],
        doors: &["f_red", "f_green", "f_brown", "f_black"],
        roof_mat: "r_shingle",
        roof: Roof::Flat,
        sash: Sash::Sash,
        leaves: 1,
        win: (0.9, 2.2),
        bay: 1.9,
        head: Head::Pediment,
        balc: Balc::Bay,
        ground: Ground::Garage,
        top: Top::Brackets,
        storey: (3.0, 3.2),
        floors: 3,
        extras: CORNER_BOARDS,
        block: Block {
            balc: Balc::Bay,
            ground: Ground::Shop,
            floors: 4,
            ..NO_BLOCK
        },
        house: House {
            cover: Cover::Courses,
            roof_mat: "r_shingle",
            pitch: 0.9,
            porch: Porch::Portico,
            fence: Fence::Iron,
            balc: Balc::Bay,
            extras: CORNER_BOARDS,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "latam",
        name: "Latinoamérica colonial",
        region: "América",
        walls: &[
            "w_yellow",
            "w_blue",
            "w_pink",
            "w_green",
            "w_terracotta",
            "w_ochre",
        ],
        base: &["w_white", "w_grey"],
        trims: &["w_white"],
        frames: &["f_brown", "f_green", "f_blue"],
        shutters: &["f_brown", "f_green"],
        doors: &["f_brown", "f_blue", "f_green"],
        pitch: 0.35,
        eaves: 0.5,
        win: (1.2, 2.4),
        bay: 3.0,
        shut: Shut::Board,
        balc: Balc::Covered,
        ground: Ground::Door,
        top: Top::Tiles,
        storey: (4.0, 3.4),
        floors: 2,
        extras: GRILLES | PLANTS,
        block: Block {
            balc: Balc::Iron,
            roof: Roof::Flat,
            floors: 5,
            extras: TANK | AC,
            ..NO_BLOCK
        },
        house: House {
            porch: Porch::Veranda,
            fence: Fence::Wall,
            shut: Shut::Board,
            floors: 1,
            extras: PALM | PLANTS,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "china",
        name: "China",
        region: "Asia",
        walls: &["w_cream", "w_yellow", "w_pink", "w_blue", "w_pearl"],
        trims: &["w_white"],
        frames: &["f_brown", "f_green", "f_white"],
        shutters: &["f_green", "f_brown"],
        doors: &["f_brown", "f_red"],
        roof_mat: "r_gravel",
        roof: Roof::Flat,
        panes: (1, 2),
        win: (1.0, 2.0),
        bay: 3.0,
        shut: Shut::Louvred,
        ground: Ground::Arcade,
        top: Top::Balustrade,
        storey: (4.2, 3.4),
        floors: 3,
        extras: SIGNS | AC | LAUNDRY | PLANTS,
        block: Block {
            walls: &["c_tile_white", "w_pearl", "w_pink"],
            balc: Balc::Cage,
            ground: Ground::Shop,
            top: Top::Band,
            floors: 7,
            extras: SOLAR,
            roof: Roof::Flat,
        },
        house: House {
            walls: &["k_grey"],
            roof: Roof::Asian,
            roof_mat: "r_grey",
            pitch: 0.5,
            fence: Fence::Wall,
            floors: 1,
            extras: TREE,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "japon",
        name: "Japón",
        region: "Asia",
        walls: &["b_dark"],
        base: &["b_dark"],
        trims: &["w_cream", "w_white"],
        frames: &["b_dark"],
        doors: &["b_dark"],
        roof_mat: "r_black",
        pitch: 0.45,
        eaves: 0.6,
        sash: Sash::Sliding,
        panes: (2, 2),
        win: (1.6, 1.0),
        bay: 3.0,
        head: Head::Plain,
        ground: Ground::Lattice,
        top: Top::None,
        storey: (3.2, 2.6),
        floors: 2,
        extras: PENT,
        block: Block {
            walls: &["c_tile_white", "c_concrete", "w_pearl"],
            balc: Balc::Slab,
            ground: Ground::Door,
            floors: 7,
            extras: AC | LAUNDRY,
            ..NO_BLOCK
        },
        house: House {
            walls: &["b_grey", "w_cream", "b_dark"],
            roof: Roof::Hip,
            pitch: 0.45,
            fence: Fence::Hedge,
            extras: AC | TREE,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "corea",
        name: "Corea",
        region: "Asia",
        walls: &["k_red", "k_dark"],
        trims: &["w_white", "c_concrete"],
        frames: &["f_alu", "f_white"],
        doors: &["f_black", "f_alu"],
        roof_mat: "r_gravel",
        roof: Roof::Flat,
        sash: Sash::Sliding,
        win: (1.4, 1.3),
        bay: 3.0,
        head: Head::Plain,
        balc: Balc::Cage,
        ground: Ground::Pilotis,
        storey: (2.9, 2.8),
        floors: 5,
        extras: AC | SIGNS,
        block: Block {
            walls: &["c_tile_white", "w_pearl"],
            balc: Balc::Glazed,
            ground: Ground::Door,
            floors: 12,
            extras: 0,
            ..NO_BLOCK
        },
        house: House {
            walls: &["w_white"],
            roof: Roof::Asian,
            roof_mat: "r_black",
            pitch: 0.5,
            porch: Porch::Veranda,
            fence: Fence::Wall,
            floors: 1,
            extras: TREE,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "grecia",
        name: "Grecia y el Mediterráneo oriental",
        region: "Otros",
        walls: &["w_white"],
        base: &["w_white"],
        trims: &["w_white"],
        accents: &["w_blue"],
        frames: &["f_blue", "f_sky", "f_green"],
        shutters: &["f_blue", "f_sky", "f_green"],
        doors: &["f_blue", "f_sky", "f_green", "f_red"],
        roof_mat: "w_white",
        roof: Roof::Flat,
        panes: (1, 2),
        win: (0.8, 1.2),
        bay: 2.4,
        shut: Shut::Louvred,
        head: Head::Plain,
        ground: Ground::Door,
        storey: (3.0, 2.8),
        floors: 2,
        extras: PLANTS,
        block: Block {
            balc: Balc::Glazed,
            floors: 5,
            ..NO_BLOCK
        },
        house: House {
            roof: Roof::Flat,
            porch: Porch::Portico,
            fence: Fence::Wall,
            shut: Shut::Louvred,
            extras: PLANTS | TREE,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "magreb",
        name: "Magreb y Oriente Medio",
        region: "Otros",
        walls: &["w_ochre", "w_terracotta", "w_pink", "w_cream"],
        base: &["w_terracotta"],
        trims: &["w_white", "w_cream"],
        frames: &["f_brown", "f_green", "f_blue"],
        shutters: &["f_green", "f_brown"],
        doors: &["f_brown", "f_green", "f_blue"],
        roof_mat: "r_gravel",
        roof: Roof::Flat,
        win: (0.8, 1.3),
        bay: 2.6,
        head: Head::Arch,
        balc: Balc::Mashrabiya,
        ground: Ground::Door,
        storey: (3.2, 2.9),
        floors: 3,
        extras: GRILLES | AC | TANK,
        block: Block {
            balc: Balc::Glazed,
            floors: 6,
            extras: TANK,
            ..NO_BLOCK
        },
        house: House {
            roof: Roof::Flat,
            porch: Porch::Arcade,
            fence: Fence::Wall,
            extras: PALM,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "este",
        name: "Europa del Este",
        region: "Otros",
        walls: &["w_yellow", "w_pink", "w_green", "w_cream", "w_ochre"],
        base: &["w_grey"],
        trims: &["w_white"],
        frames: &["f_white", "f_brown"],
        doors: &["f_brown"],
        roof_mat: "r_tile",
        cover: Cover::Courses,
        pitch: 0.9,
        panes: (1, 2),
        win: (1.0, 1.7),
        bay: 2.2,
        head: Head::Pediment,
        ground: Ground::Shop,
        storey: (3.8, 3.2),
        floors: 4,
        extras: DORMERS | BELT,
        block: Block {
            walls: &["c_panel"],
            balc: Balc::Glazed,
            ground: Ground::Door,
            floors: 9,
            ..NO_BLOCK
        },
        house: House {
            walls: &["w_cream", "w_pearl", "w_yellow"],
            roof: Roof::Hip,
            cover: Cover::Courses,
            roof_mat: "r_tile",
            pitch: 0.8,
            fence: Fence::Picket,
            floors: 1,
            extras: TREE,
            ..NO_HOUSE
        },
        ..BASE
    },
    Skin {
        key: "tropico",
        name: "Australia y el trópico",
        region: "Otros",
        walls: &["w_cream", "w_white", "k_red", "w_pearl"],
        trims: &["w_white"],
        frames: &["f_white", "f_green"],
        shutters: &["f_green"],
        doors: &["f_green", "f_red", "f_blue"],
        roof_mat: "r_galv",
        cover: Cover::Seam,
        pitch: 0.5,
        sash: Sash::Sash,
        leaves: 1,
        win: (1.0, 2.0),
        bay: 2.2,
        balc: Balc::Lace,
        ground: Ground::Door,
        storey: (3.4, 3.2),
        floors: 2,
        extras: PLANTS,
        block: Block {
            balc: Balc::Glazed,
            floors: 5,
            ..NO_BLOCK
        },
        house: House {
            walls: &["b_white", "b_yellow", "b_green"],
            roof: Roof::Hip,
            cover: Cover::Seam,
            roof_mat: "r_red_metal",
            pitch: 0.55,
            porch: Porch::Wraparound,
            fence: Fence::Picket,
            raised: 2.2,
            floors: 1,
            extras: PALM,
            ..NO_HOUSE
        },
        ..BASE
    },
];

const ANDALUSIA: Skin = Skin {
    key: "andalucia",
    name: "Andalucía",
    region: "España",
    roof_mat: "r_curved",
    roof: Roof::Gable,
    cover: Cover::Curved,
    eaves: 0.3,
    panes: (1, 2),
    win: (1.0, 1.5),
    bay: 2.4,
    shut: Shut::Roller,
    head: Head::Band,
    balc: Balc::Iron,
    ground: Ground::Door,
    top: Top::Tiles,
    storey: (3.1, 2.9),
    floors: 2,
    extras: GRILLES | PLANTS,
    block: Block {
        walls: &["w_white", "w_cream"],
        balc: Balc::Iron,
        floors: 6,
        extras: TANK | AC,
        ..NO_BLOCK
    },
    house: House {
        porch: Porch::Portico,
        fence: Fence::Wall,
        shut: Shut::Roller,
        floors: 1,
        extras: PLANTS | TREE,
        ..NO_HOUSE
    },
    ..BASE
};

/// The skin of a number (as the parameter «skin» of the recipes), or of a
/// key.
pub fn skin_of(n: f64) -> &'static Skin {
    &SKINS[(n.round().max(0.0) as usize).min(SKINS.len() - 1)]
}

/// The skin of its key.
pub fn skin_by_key(key: &str) -> Option<&'static Skin> {
    SKINS.iter().find(|s| s.key == key)
}
