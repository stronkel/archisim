//! Decals (U8): each an `IfcBuildingElementProxy` (`DECAL`) with its
//! squares as a triangulated face set coloured by an
//! `IfcIndexedColourMap`, and the conjunto `ArchiSim_Decal` with the
//! picture (a PNG in base64) and where it is, so that ArchiSim reads it
//! back.

use std::collections::BTreeMap;

use archisim_core::base64_encode;
use archisim_geom::decals::decal_cells;

use crate::step::{real, string, Ref};
use crate::Exporter;

impl Exporter<'_> {
    pub(crate) fn write_decals(&mut self, site_pl: Ref, on_site: &mut Vec<Ref>) {
        let model = self.model;
        for d in model.decals() {
            let base = model.level(d.level).map_or(0.0, |l| l.elevation);
            let cells = decal_cells(model, d);
            if cells.is_empty() {
                continue;
            }
            // Its squares, welded, each triangle with its colour.
            let mut pts: Vec<String> = Vec::new();
            let mut welded = std::collections::HashMap::new();
            let mut faces = Vec::new();
            let mut colours: BTreeMap<[u8; 3], usize> = BTreeMap::new();
            let mut colour_list: Vec<[u8; 3]> = Vec::new();
            let mut index = Vec::new();
            for (tris, c) in &cells {
                let n = colour_list.len();
                let ci = *colours.entry(*c).or_insert_with(|| {
                    colour_list.push(*c);
                    n
                });
                for t in tris {
                    let f = t.map(|p| {
                        crate::step::weld(
                            &mut pts,
                            &mut welded,
                            format!("({},{},{})", real(p[0]), real(p[1]), real(p[2] - base)),
                        )
                    });
                    faces.push(format!("({},{},{})", f[0], f[1], f[2]));
                    index.push((ci + 1).to_string());
                }
            }
            let list3 = self
                .w
                .add(format!("IFCCARTESIANPOINTLIST3D(({}),$)", pts.join(",")));
            let body = self.w.add(format!(
                "IFCTRIANGULATEDFACESET({list3},$,.F.,({}),$)",
                faces.join(",")
            ));
            let rgb: Vec<String> = colour_list
                .iter()
                .map(|c| {
                    format!(
                        "({},{},{})",
                        real(c[0] as f64 / 255.0),
                        real(c[1] as f64 / 255.0),
                        real(c[2] as f64 / 255.0)
                    )
                })
                .collect();
            let list = self.w.add(format!("IFCCOLOURRGBLIST(({}))", rgb.join(",")));
            self.w.add(format!(
                "IFCINDEXEDCOLOURMAP({body},$,{list},({}))",
                index.join(",")
            ));
            let rep = self.w.add(format!(
                "IFCSHAPEREPRESENTATION({},'Body','Tessellation',({body}))",
                self.body_ctx
            ));
            let shape = self
                .w
                .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"));
            let loc = self.point3(0.0, 0.0, base);
            let pl = self.placement(Some(site_pl), loc, None);
            let el = self.w.add(format!(
                "IFCBUILDINGELEMENTPROXY({},$,{},{},'DECAL',{pl},{shape},$,.USERDEFINED.)",
                string(d.guid.as_str()),
                string(&d.name),
                string(&d.image.source)
            ));
            let triple = |v: [f64; 3]| format!("{},{},{}", real(v[0]), real(v[1]), real(v[2]));
            let level = model
                .level(d.level)
                .map(|l| l.guid.to_string())
                .unwrap_or_default();
            let mut props = vec![
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Centre',$,IFCTEXT({}),$)",
                    string(&triple(d.centre))
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Normal',$,IFCTEXT({}),$)",
                    string(&triple(d.normal))
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Rotation',$,IFCPLANEANGLEMEASURE({}),$)",
                    real(d.rotation)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Width',$,IFCPOSITIVELENGTHMEASURE({}),$)",
                    real(d.width)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Source',$,IFCLABEL({}),$)",
                    string(&d.image.source)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Image',$,IFCTEXT({}),$)",
                    string(&base64_encode(&d.image.png))
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Level',$,IFCIDENTIFIER({}),$)",
                    string(&level)
                )),
            ];
            if let Some(h) = d.host.and_then(|h| self.guid_of_element(h)) {
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Host',$,IFCIDENTIFIER({}),$)",
                    string(&h)
                )));
            }
            self.pset(el, &d.guid, "ArchiSim_Decal", &props);
            on_site.push(el);
        }
    }

    /// The GlobalId of a model element by its id, when it has one.
    fn guid_of_element(&self, id: u64) -> Option<String> {
        let m = self.model;
        m.wall(archisim_core::WallId(id))
            .map(|x| x.guid.to_string())
            .or_else(|| {
                m.slab(archisim_core::SlabId(id))
                    .map(|x| x.guid.to_string())
            })
            .or_else(|| {
                m.mass(archisim_core::MassId(id))
                    .map(|x| x.guid.to_string())
            })
            .or_else(|| {
                m.road(archisim_core::RoadId(id))
                    .map(|x| x.guid.to_string())
            })
    }
}
