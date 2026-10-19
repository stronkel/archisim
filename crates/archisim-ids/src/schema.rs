//! What the IFC schemas say about each entity: its attributes in file
//! order (inherited first) and its supertypes.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::schema_data::{Row, IFC2X3, IFC4, IFC4X3};

pub struct Schema {
    rows: HashMap<&'static str, &'static Row>,
}

impl Schema {
    fn new(rows: &'static [Row]) -> Schema {
        Schema {
            rows: rows.iter().map(|r| (r.0, r)).collect(),
        }
    }

    /// The schema a file header names; IFC4X3 and its addenda share one
    /// table, other or unknown names fall back to IFC4.
    pub fn for_file(name: &str) -> &'static Schema {
        static S2X3: OnceLock<Schema> = OnceLock::new();
        static S4: OnceLock<Schema> = OnceLock::new();
        static S4X3: OnceLock<Schema> = OnceLock::new();
        let name = name.to_uppercase();
        if name.starts_with("IFC2X3") {
            S2X3.get_or_init(|| Schema::new(IFC2X3))
        } else if name.starts_with("IFC4X3") {
            S4X3.get_or_init(|| Schema::new(IFC4X3))
        } else {
            S4.get_or_init(|| Schema::new(IFC4))
        }
    }

    /// The name as the schema writes it (`IFCWALLSTANDARDCASE` →
    /// `IfcWallStandardCase`).
    pub fn spelling(&self, entity: &str) -> Option<&'static str> {
        self.rows.get(entity).map(|r| r.4)
    }

    pub fn has(&self, entity: &str) -> bool {
        self.rows.contains_key(entity)
    }

    pub fn supertype(&self, entity: &str) -> Option<&'static str> {
        self.rows.get(entity).map(|r| r.1).filter(|s| !s.is_empty())
    }

    /// Whether `entity` is `ancestor` or one of its subtypes.
    pub fn is_a(&self, entity: &str, ancestor: &str) -> bool {
        let mut e = Some(entity);
        while let Some(x) = e {
            if x == ancestor {
                return true;
            }
            e = self.supertype(x);
        }
        false
    }

    /// Every explicit attribute of `entity`, inherited ones first: the
    /// order of the arguments in a STEP file.
    pub fn attributes(&self, entity: &str) -> Vec<&'static str> {
        let mut chain = Vec::new();
        let mut e = self.rows.get(entity).copied();
        while let Some(r) = e {
            chain.push(r);
            e = self.rows.get(r.1).copied();
        }
        chain
            .iter()
            .rev()
            .flat_map(|r| r.2.iter().copied())
            .collect()
    }

    /// Position of a named attribute in the file arguments.
    pub fn index_of(&self, entity: &str, attribute: &str) -> Option<usize> {
        self.attributes(entity).iter().position(|a| *a == attribute)
    }
}

/// IFC2X3 had no class for many occurrences (an air terminal was an
/// `IfcFlowTerminal` typed by an `IfcAirTerminalType`). IDS checks those
/// with the IFC4 name: (name in the IDS, IFC2X3 occurrence, IFC2X3 type).
pub static IFC2X3_TYPED_CLASSES: &[(&str, &str, &str)] = &[
    ("IFCFURNITURE", "IFCFURNISHINGELEMENT", "IFCFURNITURETYPE"),
    (
        "IFCSYSTEMFURNITUREELEMENT",
        "IFCFURNISHINGELEMENT",
        "IFCSYSTEMFURNITUREELEMENTTYPE",
    ),
    (
        "IFCACTUATOR",
        "IFCDISTRIBUTIONCONTROLELEMENT",
        "IFCACTUATORTYPE",
    ),
    ("IFCALARM", "IFCDISTRIBUTIONCONTROLELEMENT", "IFCALARMTYPE"),
    (
        "IFCCONTROLLER",
        "IFCDISTRIBUTIONCONTROLELEMENT",
        "IFCCONTROLLERTYPE",
    ),
    (
        "IFCFLOWINSTRUMENT",
        "IFCDISTRIBUTIONCONTROLELEMENT",
        "IFCFLOWINSTRUMENTTYPE",
    ),
    (
        "IFCSENSOR",
        "IFCDISTRIBUTIONCONTROLELEMENT",
        "IFCSENSORTYPE",
    ),
    (
        "IFCAIRTOAIRHEATRECOVERY",
        "IFCENERGYCONVERSIONDEVICE",
        "IFCAIRTOAIRHEATRECOVERYTYPE",
    ),
    ("IFCBOILER", "IFCENERGYCONVERSIONDEVICE", "IFCBOILERTYPE"),
    ("IFCCHILLER", "IFCENERGYCONVERSIONDEVICE", "IFCCHILLERTYPE"),
    ("IFCCOIL", "IFCENERGYCONVERSIONDEVICE", "IFCCOILTYPE"),
    (
        "IFCCONDENSER",
        "IFCENERGYCONVERSIONDEVICE",
        "IFCCONDENSERTYPE",
    ),
    (
        "IFCCOOLEDBEAM",
        "IFCENERGYCONVERSIONDEVICE",
        "IFCCOOLEDBEAMTYPE",
    ),
    (
        "IFCCOOLINGTOWER",
        "IFCENERGYCONVERSIONDEVICE",
        "IFCCOOLINGTOWERTYPE",
    ),
    (
        "IFCELECTRICGENERATOR",
        "IFCENERGYCONVERSIONDEVICE",
        "IFCELECTRICGENERATORTYPE",
    ),
    (
        "IFCELECTRICMOTOR",
        "IFCENERGYCONVERSIONDEVICE",
        "IFCELECTRICMOTORTYPE",
    ),
    (
        "IFCEVAPORATIVECOOLER",
        "IFCENERGYCONVERSIONDEVICE",
        "IFCEVAPORATIVECOOLERTYPE",
    ),
    (
        "IFCEVAPORATOR",
        "IFCENERGYCONVERSIONDEVICE",
        "IFCEVAPORATORTYPE",
    ),
    (
        "IFCHEATEXCHANGER",
        "IFCENERGYCONVERSIONDEVICE",
        "IFCHEATEXCHANGERTYPE",
    ),
    (
        "IFCHUMIDIFIER",
        "IFCENERGYCONVERSIONDEVICE",
        "IFCHUMIDIFIERTYPE",
    ),
    (
        "IFCMOTORCONNECTION",
        "IFCENERGYCONVERSIONDEVICE",
        "IFCMOTORCONNECTIONTYPE",
    ),
    (
        "IFCSPACEHEATER",
        "IFCENERGYCONVERSIONDEVICE",
        "IFCSPACEHEATERTYPE",
    ),
    (
        "IFCTRANSFORMER",
        "IFCENERGYCONVERSIONDEVICE",
        "IFCTRANSFORMERTYPE",
    ),
    (
        "IFCTUBEBUNDLE",
        "IFCENERGYCONVERSIONDEVICE",
        "IFCTUBEBUNDLETYPE",
    ),
    (
        "IFCUNITARYEQUIPMENT",
        "IFCENERGYCONVERSIONDEVICE",
        "IFCUNITARYEQUIPMENTTYPE",
    ),
    (
        "IFCAIRTERMINALBOX",
        "IFCFLOWCONTROLLER",
        "IFCAIRTERMINALBOXTYPE",
    ),
    ("IFCDAMPER", "IFCFLOWCONTROLLER", "IFCDAMPERTYPE"),
    (
        "IFCELECTRICTIMECONTROL",
        "IFCFLOWCONTROLLER",
        "IFCELECTRICTIMECONTROLTYPE",
    ),
    ("IFCFLOWMETER", "IFCFLOWCONTROLLER", "IFCFLOWMETERTYPE"),
    (
        "IFCPROTECTIVEDEVICE",
        "IFCFLOWCONTROLLER",
        "IFCPROTECTIVEDEVICETYPE",
    ),
    (
        "IFCSWITCHINGDEVICE",
        "IFCFLOWCONTROLLER",
        "IFCSWITCHINGDEVICETYPE",
    ),
    ("IFCVALVE", "IFCFLOWCONTROLLER", "IFCVALVETYPE"),
    (
        "IFCCABLECARRIERFITTING",
        "IFCFLOWFITTING",
        "IFCCABLECARRIERFITTINGTYPE",
    ),
    ("IFCDUCTFITTING", "IFCFLOWFITTING", "IFCDUCTFITTINGTYPE"),
    ("IFCJUNCTIONBOX", "IFCFLOWFITTING", "IFCJUNCTIONBOXTYPE"),
    ("IFCPIPEFITTING", "IFCFLOWFITTING", "IFCPIPEFITTINGTYPE"),
    ("IFCCOMPRESSOR", "IFCFLOWMOVINGDEVICE", "IFCCOMPRESSORTYPE"),
    ("IFCFAN", "IFCFLOWMOVINGDEVICE", "IFCFANTYPE"),
    ("IFCPUMP", "IFCFLOWMOVINGDEVICE", "IFCPUMPTYPE"),
    (
        "IFCCABLECARRIERSEGMENT",
        "IFCFLOWSEGMENT",
        "IFCCABLECARRIERSEGMENTTYPE",
    ),
    ("IFCCABLESEGMENT", "IFCFLOWSEGMENT", "IFCCABLESEGMENTTYPE"),
    ("IFCDUCTSEGMENT", "IFCFLOWSEGMENT", "IFCDUCTSEGMENTTYPE"),
    ("IFCPIPESEGMENT", "IFCFLOWSEGMENT", "IFCPIPESEGMENTTYPE"),
    (
        "IFCELECTRICFLOWSTORAGEDEVICE",
        "IFCFLOWSTORAGEDEVICE",
        "IFCELECTRICFLOWSTORAGEDEVICETYPE",
    ),
    ("IFCTANK", "IFCFLOWSTORAGEDEVICE", "IFCTANKTYPE"),
    ("IFCAIRTERMINAL", "IFCFLOWTERMINAL", "IFCAIRTERMINALTYPE"),
    (
        "IFCELECTRICAPPLIANCE",
        "IFCFLOWTERMINAL",
        "IFCELECTRICAPPLIANCETYPE",
    ),
    (
        "IFCFIRESUPPRESSIONTERMINAL",
        "IFCFLOWTERMINAL",
        "IFCFIRESUPPRESSIONTERMINALTYPE",
    ),
    ("IFCLAMP", "IFCFLOWTERMINAL", "IFCLAMPTYPE"),
    ("IFCLIGHTFIXTURE", "IFCFLOWTERMINAL", "IFCLIGHTFIXTURETYPE"),
    ("IFCOUTLET", "IFCFLOWTERMINAL", "IFCOUTLETTYPE"),
    (
        "IFCSANITARYTERMINAL",
        "IFCFLOWTERMINAL",
        "IFCSANITARYTERMINALTYPE",
    ),
    (
        "IFCSTACKTERMINAL",
        "IFCFLOWTERMINAL",
        "IFCSTACKTERMINALTYPE",
    ),
    (
        "IFCWASTETERMINAL",
        "IFCFLOWTERMINAL",
        "IFCWASTETERMINALTYPE",
    ),
    (
        "IFCDUCTSILENCER",
        "IFCFLOWTREATMENTDEVICE",
        "IFCDUCTSILENCERTYPE",
    ),
    ("IFCFILTER", "IFCFLOWTREATMENTDEVICE", "IFCFILTERTYPE"),
    (
        "IFCVIBRATIONISOLATOR",
        "IFCEQUIPMENTELEMENT",
        "IFCVIBRATIONISOLATORTYPE",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wall_attributes_in_file_order() {
        let s = Schema::for_file("IFC4");
        assert_eq!(
            s.attributes("IFCWALL"),
            [
                "GlobalId",
                "OwnerHistory",
                "Name",
                "Description",
                "ObjectType",
                "ObjectPlacement",
                "Representation",
                "Tag",
                "PredefinedType"
            ]
        );
        assert!(s.is_a("IFCWALLSTANDARDCASE", "IFCWALL"));
        assert!(!s.is_a("IFCWALL", "IFCSLAB"));
        // IFC2X3 walls have no predefined type.
        assert_eq!(
            Schema::for_file("IFC2X3").index_of("IFCWALL", "PredefinedType"),
            None
        );
        assert!(Schema::for_file("IFC4X3_ADD2").has("IFCROAD"));
    }
}
