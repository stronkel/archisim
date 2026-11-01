//! What an import brought in and what it left out, in words for the
//! architect (Spanish, as the rest of the interface).

use std::collections::BTreeMap;
use std::fmt;

/// Summary of an IFC import.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportReport {
    /// `IFC4`, `IFC2X3`…
    pub schema: String,
    /// Application that wrote the file, from its header.
    pub source: String,
    pub levels: usize,
    pub walls: usize,
    pub slabs: usize,
    pub doors: usize,
    pub windows: usize,
    /// Rooms (IfcSpace).
    pub rooms: usize,
    /// Openings in walls with no door or window in them.
    pub voids: usize,
    pub columns: usize,
    pub beams: usize,
    /// Pitched roofs written by ArchiSim (`IfcRoof`).
    pub roofs: usize,
    /// Stairs written by ArchiSim (`IfcStair`).
    pub stairs: usize,
    /// Terrains written by ArchiSim (`IfcGeographicElement`).
    pub terrains: usize,
    /// Footings and tie beams (`IfcFooting`).
    pub footings: usize,
    /// Setting-out grid lines (axes of `IfcGrid`).
    pub grids: usize,
    pub plots: usize,
    pub roads: usize,
    pub roundabouts: usize,
    /// Pools written by ArchiSim (`IfcElementAssembly` «Piscina»).
    pub pools: usize,
    /// Lifts written by ArchiSim (`IfcTransportElement` with `ArchiSim_Elevator`).
    pub elevators: usize,
    pub services: usize,
    /// Curtain walls written by ArchiSim (`IfcCurtainWall`).
    pub curtain_walls: usize,
    /// Railings written by ArchiSim (`IfcRailing`).
    pub railings: usize,
    /// Furniture and sanitary ware written by ArchiSim.
    pub objects: usize,
    pub wall_types: usize,
    pub slab_types: usize,
    pub opening_types: usize,
    pub member_types: usize,
    pub materials: usize,
    /// Parameters of the project written by ArchiSim, and how many values
    /// of them the elements had.
    pub parameters: usize,
    pub parameter_values: usize,
    /// Elements not imported: (what, why) → how many.
    pub skipped: BTreeMap<(String, String), usize>,
    /// Things imported with approximations: message → how many times.
    pub warnings: BTreeMap<String, usize>,
}

impl ImportReport {
    pub fn skip(&mut self, what: impl Into<String>, why: impl Into<String>) {
        *self.skipped.entry((what.into(), why.into())).or_default() += 1;
    }

    pub fn warn(&mut self, message: impl Into<String>) {
        *self.warnings.entry(message.into()).or_default() += 1;
    }

    /// Number of elements left out.
    pub fn skipped_count(&self) -> usize {
        self.skipped.values().sum()
    }

    /// One line for a status bar.
    pub fn summary(&self) -> String {
        let mut s = format!(
            "{} niveles, {} muros, {} forjados, {} puertas, {} ventanas",
            self.levels, self.walls, self.slabs, self.doors, self.windows
        );
        if self.voids > 0 {
            s.push_str(&format!(", {} huecos", self.voids));
        }
        if self.columns + self.beams > 0 {
            s.push_str(&format!(", {} pilares, {} vigas", self.columns, self.beams));
        }
        if self.rooms > 0 {
            s.push_str(&format!(", {} locales", self.rooms));
        }
        let n = self.skipped_count();
        if n > 0 {
            s.push_str(&format!("; {n} elementos sin importar"));
        }
        s
    }
}

fn times(n: usize) -> String {
    if n > 1 {
        format!(" ({n})")
    } else {
        String::new()
    }
}

impl fmt::Display for ImportReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Fichero {}{}",
            self.schema,
            if self.source.is_empty() {
                String::new()
            } else {
                format!(" escrito por {}", self.source)
            }
        )?;
        writeln!(f, "Importado: {}.", self.summary_imported())?;
        writeln!(
            f,
            "Tipos: {} de muro, {} de forjado, {} de puerta o ventana{}; {} materiales.",
            self.wall_types,
            self.slab_types,
            self.opening_types,
            if self.member_types > 0 {
                format!(", {} de pilar o viga", self.member_types)
            } else {
                String::new()
            },
            self.materials
        )?;
        if self.parameters > 0 {
            writeln!(
                f,
                "Parámetros del proyecto: {}, con {} valores.",
                self.parameters, self.parameter_values
            )?;
        }
        if !self.skipped.is_empty() {
            writeln!(f, "\nNo importado:")?;
            for ((what, why), n) in &self.skipped {
                writeln!(f, "  · {n} × {what}: {why}")?;
            }
        }
        if !self.warnings.is_empty() {
            writeln!(f, "\nAvisos:")?;
            for (w, n) in &self.warnings {
                writeln!(f, "  · {w}{}", times(*n))?;
            }
        }
        Ok(())
    }
}

impl ImportReport {
    fn summary_imported(&self) -> String {
        let voids = if self.voids > 0 {
            format!(", {} huecos sin carpintería", self.voids)
        } else {
            String::new()
        };
        let mut members = if self.columns + self.beams > 0 {
            format!(", {} pilares y {} vigas", self.columns, self.beams)
        } else {
            String::new()
        };
        if self.rooms > 0 {
            members.push_str(&format!(", {} locales", self.rooms));
        }
        if self.roofs > 0 {
            members.push_str(&format!(", {} cubiertas", self.roofs));
        }
        if self.stairs > 0 {
            members.push_str(&format!(", {} escaleras", self.stairs));
        }
        if self.curtain_walls > 0 {
            members.push_str(&format!(", {} muros cortina", self.curtain_walls));
        }
        if self.objects > 0 {
            members.push_str(&format!(", {} objetos", self.objects));
        }
        if self.railings > 0 {
            members.push_str(&format!(", {} barandillas", self.railings));
        }
        if self.footings > 0 {
            members.push_str(&format!(", {} cimentaciones", self.footings));
        }
        if self.plots > 0 {
            members.push_str(&format!(", {} parcelas", self.plots));
        }
        if self.roads > 0 {
            members.push_str(&format!(", {} viales", self.roads));
        }
        if self.roundabouts > 0 {
            members.push_str(&format!(", {} glorietas o plazas", self.roundabouts));
        }
        if self.elevators > 0 {
            members.push_str(&format!(", {} ascensores", self.elevators));
        }
        if self.pools > 0 {
            members.push_str(&format!(", {} piscinas", self.pools));
        }
        if self.services > 0 {
            members.push_str(&format!(", {} instalaciones", self.services));
        }
        if self.grids > 0 {
            members.push_str(&format!(", {} ejes", self.grids));
        }
        if self.terrains > 0 {
            members.push_str(&format!(", {} terrenos", self.terrains));
        }
        format!(
            "{} niveles, {} muros, {} forjados, {} puertas y {} ventanas{voids}{members}",
            self.levels, self.walls, self.slabs, self.doors, self.windows
        )
    }
}

/// Spanish name of an IFC class, in plural, for the report.
pub fn class_name(kind: &str) -> &'static str {
    match kind {
        "IFCWALL" | "IFCWALLSTANDARDCASE" | "IFCWALLELEMENTEDCASE" => "muros",
        "IFCSLAB" | "IFCSLABSTANDARDCASE" | "IFCSLABELEMENTEDCASE" => "forjados",
        "IFCDOOR" | "IFCDOORSTANDARDCASE" => "puertas",
        "IFCWINDOW" | "IFCWINDOWSTANDARDCASE" => "ventanas",
        "IFCOPENINGELEMENT" | "IFCOPENINGSTANDARDCASE" => "huecos",
        "IFCCOLUMN" | "IFCCOLUMNSTANDARDCASE" => "pilares",
        "IFCBEAM" | "IFCBEAMSTANDARDCASE" => "vigas",
        "IFCSTAIR" | "IFCSTAIRFLIGHT" => "escaleras",
        "IFCRAMP" | "IFCRAMPFLIGHT" => "rampas",
        "IFCRAILING" => "barandillas",
        "IFCROOF" => "cubiertas (IfcRoof)",
        "IFCCOVERING" => "revestimientos",
        "IFCCURTAINWALL" => "muros cortina",
        "IFCMEMBER" | "IFCMEMBERSTANDARDCASE" => "perfiles",
        "IFCPLATE" | "IFCPLATESTANDARDCASE" => "placas y paneles",
        "IFCFOOTING" => "cimentaciones",
        "IFCPILE" => "pilotes",
        "IFCCHIMNEY" => "chimeneas",
        "IFCSHADINGDEVICE" => "protecciones solares",
        "IFCBUILDINGELEMENTPROXY" => "objetos genéricos",
        "IFCVEHICLE" => "vehículos",
        "IFCPAVEMENT" => "firmes y pavimentos exteriores",
        "IFCKERB" => "bordillos",
        "IFCCOURSE" => "capas de firme",
        "IFCEARTHWORKSFILL" => "rellenos de tierras",
        "IFCFURNISHINGELEMENT" | "IFCFURNITURE" => "mobiliario",
        "IFCSPACE" => "espacios (locales)",
        "IFCSANITARYTERMINAL"
        | "IFCFLOWTERMINAL"
        | "IFCFLOWSEGMENT"
        | "IFCFLOWFITTING"
        | "IFCDISTRIBUTIONELEMENT"
        | "IFCDISTRIBUTIONFLOWELEMENT"
        | "IFCLIGHTFIXTURE"
        | "IFCDUCTSEGMENT"
        | "IFCPIPESEGMENT"
        | "IFCDUCTFITTING"
        | "IFCPIPEFITTING"
        | "IFCAIRTERMINAL"
        | "IFCELECTRICAPPLIANCE"
        | "IFCOUTLET"
        | "IFCSWITCHINGDEVICE"
        | "IFCFIRESUPPRESSIONTERMINAL"
        | "IFCSENSOR"
        | "IFCALARM"
        | "IFCVALVE"
        | "IFCFLOWMETER"
        | "IFCTANK"
        | "IFCPUMP"
        | "IFCBOILER"
        | "IFCSOLARDEVICE" => "elementos de instalaciones",
        "IFCTRANSPORTELEMENT" => "ascensores y transporte",
        "IFCGEOGRAPHICELEMENT" => "elementos del terreno",
        _ => "otros elementos",
    }
}

/// IFC classes of built elements the importer does not bring in (yet),
/// counted in the report.
pub const UNSUPPORTED_CLASSES: &[&str] = &[
    "IFCSTAIR",
    "IFCRAMP",
    "IFCRAILING",
    "IFCCOVERING",
    "IFCCURTAINWALL",
    "IFCPILE",
    "IFCCHIMNEY",
    "IFCSHADINGDEVICE",
    "IFCBUILDINGELEMENTPROXY",
    "IFCFURNISHINGELEMENT",
    "IFCFURNITURE",
    "IFCSANITARYTERMINAL",
    "IFCFLOWTERMINAL",
    "IFCFLOWSEGMENT",
    "IFCFLOWFITTING",
    "IFCDISTRIBUTIONFLOWELEMENT",
    "IFCLIGHTFIXTURE",
    "IFCDUCTSEGMENT",
    "IFCPIPESEGMENT",
    "IFCDUCTFITTING",
    "IFCPIPEFITTING",
    "IFCAIRTERMINAL",
    "IFCTRANSPORTELEMENT",
    "IFCVEHICLE",
    "IFCGEOGRAPHICELEMENT",
    "IFCPAVEMENT",
    "IFCKERB",
    "IFCCOURSE",
    "IFCEARTHWORKSFILL",
];

/// Classes that are only parts of something else already counted: stair
/// flights of a stair, members and plates of a curtain wall.
pub const PART_CLASSES: &[&str] = &[
    "IFCSTAIRFLIGHT",
    "IFCRAMPFLIGHT",
    "IFCMEMBER",
    "IFCMEMBERSTANDARDCASE",
    "IFCPLATE",
    "IFCPLATESTANDARDCASE",
];
