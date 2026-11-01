//! Lifts (H4): an `IfcTransportElement` (`ELEVATOR`) in the storey of its
//! lowest stop, its body the car, the rails, the counterweight and the
//! landing doors, with `Pset_TransportElementElevator`-like figures in
//! `ArchiSim_Elevator` (which reads it back); its shaft, when the lift
//! builds it, an `IfcBuildingElementProxy` «Hueco de ascensor» with
//! `ArchiSim_ElevatorPart`
//! (which the import leaves out).

use std::collections::BTreeMap;

use archisim_core::{LevelId, MaterialId};
use archisim_geom::solid::P3;

use crate::step::{real, string, Ref};
use crate::{derived_guid, Exporter};

impl Exporter<'_> {
    pub(crate) fn write_elevators(
        &mut self,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
        contained: &mut BTreeMap<LevelId, Vec<Ref>>,
    ) {
        let model = self.model;
        for e in model.elevators() {
            let Some(&(_, storey_pl)) = storeys.get(&e.level) else {
                continue;
            };
            let storey_z = model.level(e.level).map_or(0.0, |l| l.elevation);
            let solids = archisim_geom::elevators::elevator_solids(model, e.id);
            let (shaft, rest): (Vec<_>, Vec<_>) = solids
                .into_iter()
                .partition(|s| archisim_geom::elevators::is_shaft(model, s));
            let tris: Vec<[P3; 3]> = rest.into_iter().flat_map(|s| s.triangles).collect();
            let shape = self.triangulated_shape(&tris, storey_z);
            let pl = self.placement(Some(storey_pl), self.origin, None);
            let el = self.w.add(format!(
                "IFCTRANSPORTELEMENT({},$,{},$,$,{pl},{shape},$,.ELEVATOR.)",
                string(e.guid.as_str()),
                string(&e.name)
            ));
            let level_name =
                |l: LevelId| model.level(l).map(|x| x.name.clone()).unwrap_or_default();
            let mut props = Vec::new();
            for (k, v) in [
                ("Top", level_name(e.top)),
                (
                    "Position",
                    format!("{},{}", real(e.position.x), real(e.position.y)),
                ),
            ] {
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('{k}',$,IFCTEXT({}),$)",
                    string(&v)
                )));
            }
            if let Some(l) = e.car_at {
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('CarAt',$,IFCTEXT({}),$)",
                    string(&level_name(l))
                )));
            }
            props.push(self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('Rotation',$,IFCPLANEANGLEMEASURE({}),$)",
                real(e.rotation)
            )));
            for (k, v) in [
                ("ShaftWidth", e.shaft_w),
                ("ShaftDepth", e.shaft_d),
                ("Wall", e.wall),
                ("Pit", e.pit),
                ("Overrun", e.overrun),
                ("CarWidth", e.car_w),
                ("CarDepth", e.car_d),
                ("CarHeight", e.car_h),
                ("DoorWidth", e.door_w),
                ("DoorHeight", e.door_h),
            ] {
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('{k}',$,IFCLENGTHMEASURE({}),$)",
                    real(v)
                )));
            }
            props.push(self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('Load',$,IFCMASSMEASURE({}),$)",
                real(e.load as f64)
            )));
            props.push(self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('Persons',$,IFCCOUNTMEASURE({}),$)",
                e.persons()
            )));
            props.push(self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('CutSlabs',$,IFCBOOLEAN(.{}.),$)",
                if e.cut_slabs { "T" } else { "F" }
            )));
            if let Some(m) = e.material.and_then(|m| model.material(m)) {
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Material',$,IFCLABEL({}),$)",
                    string(&m.name)
                )));
            }
            self.pset(el, &e.guid, "ArchiSim_Elevator", &props);
            contained.entry(e.level).or_default().push(el);
            // Its shaft.
            let pieces: Vec<(MaterialId, Vec<[P3; 3]>)> = shaft
                .into_iter()
                .map(|s| (s.material, s.triangles))
                .collect();
            if pieces.iter().any(|(_, t)| !t.is_empty()) {
                let guid = derived_guid(&e.guid, "shaft");
                let shape = self.styled_shape(&pieces, storey_z);
                let pl = self.placement(Some(storey_pl), self.origin, None);
                let shaft_el = self.w.add(format!(
                    "IFCBUILDINGELEMENTPROXY({},$,{},$,'Hueco de ascensor',{pl},{shape},$,.ELEMENT.)",
                    string(guid.as_str()),
                    string(&format!("Hueco de {}", e.name))
                ));
                let p = [self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Elevator',$,IFCIDENTIFIER({}),$)",
                    string(e.guid.as_str())
                ))];
                self.pset(shaft_el, &guid, "ArchiSim_ElevatorPart", &p);
                contained.entry(e.level).or_default().push(shaft_el);
            }
        }
    }
}
